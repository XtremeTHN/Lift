use crate::roms::{crypto::get_tweak, keyring::Keyring};
use aes::cipher::BlockDecryptMut;
use aes::{Aes128, cipher::KeyInit, cipher::generic_array::GenericArray};
use binrw::BinRead;
use ecb::Decryptor;
use std::io::Cursor;
use std::io::{Read, Seek};
use std::string::FromUtf8Error;
use xts_mode::Xts128;

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum DistributionType {
    Download = 0x00,
    GameCard = 0x01,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum ContentType {
    Program = 0x00,
    Meta = 0x01,
    Control = 0x02,
    Manual = 0x03,
    Data = 0x04,
    PublicData = 0x05,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
enum KeyGenOld {
    _1_0_0 = 0x00,
    Unusued = 0x01,
    _3_0_0 = 0x02,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
enum KeyAreaEncryptionKeyIndex {
    Application = 0x00,
    Ocean = 0x01,
    System = 0x02,
}

#[derive(BinRead, Debug, Default)]
#[br(little)]
pub struct KeyArea {
    #[br(count = 0x20)]
    pub aes_xts_key: Vec<u8>,
    #[br(count = 0x10)]
    pub aes_ctr_key: Vec<u8>,
    #[br(count = 0x10)]
    pub unk_key: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct NcaHeader {
    // #[br(count = 0x200)]
    // signature: Vec<u8>,
    #[br(seek_before = std::io::SeekFrom::Start(0x200), count = 4)]
    pub magic: Vec<u8>,
    pub distribution_type: DistributionType,
    pub content_type: ContentType,
    pub key_generation_old: KeyGenOld,
    pub key_area_encryption_key_index: KeyAreaEncryptionKeyIndex,

    pub content_size: u64,
    pub program_id: u64,
    pub content_index: u32,

    #[br(count = 4)]
    pub sdk_addon_version: Vec<u8>,

    pub key_generation: u8,

    #[br(count = 10)]
    pub rights_id: Vec<u8>,
}

#[derive(Debug)]
pub struct Nca {
    pub header: NcaHeader,
    pub key_area: KeyArea,
    keyring: Keyring,
}

#[derive(thiserror::Error, Debug)]
pub enum NcaErrors {
    #[error("Nca data is corrupted")]
    CorruptNca(#[from] binrw::Error),
    #[error("Decoding error")]
    DecodingError(#[from] FromUtf8Error),
    #[error("Invalid or unsupported magic: {0}")]
    InvalidMagic(String),
    #[error("Invalid keys: {0}")]
    InvalidKeys(String),
    #[error("Couldn't read header: {0}")]
    ReadError(#[from] std::io::Error),
}

impl Nca {
    pub fn new<T: Read + Seek>(keyring: Keyring, stream: &mut T) -> Result<Self, NcaErrors> {
        let mut encrypted_header = vec![0u8; 0xC00];
        stream.read_exact(&mut encrypted_header)?;

        let _cipher_1 = Aes128::new_from_slice(&keyring.header_key[..0x10]);
        let _cipher_2 = Aes128::new_from_slice(&keyring.header_key[0x10..]);

        if let Err(e) = _cipher_1 {
            return Err(NcaErrors::InvalidKeys(e.to_string()));
        }

        if let Err(e) = _cipher_2 {
            return Err(NcaErrors::InvalidKeys(e.to_string()));
        }

        let cipher_1 = _cipher_1.unwrap();
        let cipher_2 = _cipher_2.unwrap();

        let xts = Xts128::new(cipher_1, cipher_2);

        xts.decrypt_area(&mut encrypted_header, 0x200, 0, get_tweak);

        let mut cur = Cursor::new(encrypted_header);
        let header = NcaHeader::read(&mut cur)?;

        // that array is NCA3 in u8
        if header.magic != [78, 67, 65, 51] {
            let magic = String::from_utf8(header.magic)?;
            return Err(NcaErrors::InvalidMagic(magic));
        }

        let mut r = Self {
            header,
            keyring: keyring.clone(),
            key_area: Default::default(),
        };

        r.decrypt_key_area(stream);

        Ok(r)
    }

    fn get_key_generation(&self) -> u8 {
        let old = self.header.key_generation_old as u8;
        let new = self.header.key_generation as u8;

        let key = if old < new { new } else { old };

        if key > 0 { key - 1 } else { key }
    }

    fn get_key_area_key(&self) -> Vec<u8> {
        let _gen = self.get_key_generation();

        match self.header.key_area_encryption_key_index {
            KeyAreaEncryptionKeyIndex::Application => {
                return self.keyring.key_area_application[_gen as usize].clone();
            }
            KeyAreaEncryptionKeyIndex::Ocean => {
                return self.keyring.key_area_ocean[_gen as usize].clone();
            }
            KeyAreaEncryptionKeyIndex::System => {
                return self.keyring.key_area_system[_gen as usize].clone();
            }
        }
    }

    fn decrypt_key_area<T: Read + Seek>(&mut self, stream: &mut T) {
        let mut buf = vec![0u8; 0x40];

        stream.seek(std::io::SeekFrom::Start(0x300));
        stream.read_exact(&mut buf);

        if self.header.rights_id.iter().all(|&b| b == 0) {
            let key: [u8; 16] = self
                .get_key_area_key()
                .try_into()
                .expect("Key must be 16 bytes");

            let mut decryptor = Decryptor::<Aes128>::new(&key.into());

            let mut _buf = buf.clone();
            for block in _buf.chunks_exact_mut(16) {
                decryptor.decrypt_block_mut(block.into());
                buf.append(block.to_vec().as_mut());
            }

            let mut cursor = std::io::Cursor::new(_buf);
            self.key_area = KeyArea::read(&mut cursor).expect("");
        }
    }

    fn populate_fs_entries(&self) {}
    fn populate_fs_headers(&self) {}
}
