use crate::roms::{crypto::get_tweak, keyring::Keyring};
use aes::{Aes128, cipher::KeyInit, cipher::generic_array::GenericArray};
use binrw::BinRead;
use std::io::Cursor;
use std::io::{Read, Seek};
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

#[derive(BinRead, Debug)]
#[br(little)]
pub struct NcaHeader {
    // #[br(count = 0x200)]
    // signature: Vec<u8>,
    #[br(seek_before = std::io::SeekFrom::Start(0x200), count = 4)]
    pub magic: Vec<u8>,
    pub distribution_type: DistributionType,
    pub content_type: ContentType,
}

#[derive(Debug)]
pub struct Nca {
    pub header: NcaHeader,
    keyring: Keyring,
}

impl Nca {
    pub fn new<T: Read + Seek>(keyring: Keyring, stream: &mut T) -> Self {
        let mut encrypted_header = vec![0u8; 0xC00];
        stream
            .read_exact(&mut encrypted_header)
            .expect("couldnt read header");

        let cipher_1 = Aes128::new_from_slice(&keyring.header_key[..0x10]).expect("invalid len");
        let cipher_2 = Aes128::new_from_slice(&keyring.header_key[0x10..]).expect("invalid len");

        let xts = Xts128::new(cipher_1, cipher_2);

        xts.decrypt_area(&mut encrypted_header, 0x200, 0, get_tweak);

        let mut cur = Cursor::new(encrypted_header);
        let header = NcaHeader::read(&mut cur).expect("invalid header");

        Self {
            header,
            keyring: keyring.clone(),
        }
    }
}
