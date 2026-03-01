use binrw::BinRead;

use crate::roms::fs::fs::media_to_bytes;
use positioned_io::ReadAt;
use std::io::{Read, Seek, SeekFrom};

#[derive(BinRead, Debug)]
#[br(repr(u8))]
pub enum CardSize {
    _1GB = 0xFA,
    _2GB = 0xF8,
    _4GB = 0xF0,
    _8GB = 0xE0,
    _16GB = 0xE1,
    _32GB = 0xE2,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct XciHeader {
    #[br(count = 4, seek_before = SeekFrom::Start(0x100))]
    pub magic: Vec<u8>,

    #[br(seek_before = SeekFrom::Start(0x10C))]
    pub title_key_dec_index: u8,
    pub rom_size: CardSize,
    pub version: u8,

    #[br(seek_before = SeekFrom::Start(0x130))]
    pub hfs_header_offset: u64,
    pub hfs_header_size: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum XciErrors {
    #[error("Invalid/corrupted xci: {0}")]
    CorruptXci(#[from] binrw::Error),
    #[error("Invalid magic: {0:?}")]
    InvalidMagic(Vec<u8>),
}

#[derive(Debug)]
pub struct Xci {
    pub header: XciHeader,
}

impl Xci {
    pub fn new<T: ReadAt + Read + Seek>(stream: &mut T) -> Result<Xci, XciErrors> {
        let h = XciHeader::read(stream)?;

        if h.magic != [72, 69, 65, 68] {
            return Err(XciErrors::InvalidMagic(h.magic));
        }

        Ok(Self { header: h })
    }

    pub fn open_partition(&mut self, partition: String) {}
}
