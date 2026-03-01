use binrw::BinRead;
use positioned_io::ReadAt;
use std::io::{Read, Seek};
use std::string::FromUtf8Error;
use thiserror::Error;

use crate::roms::readers::FileRegion;

#[derive(Error, Debug)]
pub enum PartitionFsErrors {
    #[error("Failed to decode from bytes")]
    DecodingError(#[from] FromUtf8Error),
    #[error("Failed to find null terminator in string")]
    NullTerminatorError,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(little)]
pub struct PFSEntry {
    offset: u64,
    size: u64,
    #[br(pad_after = 4)]
    string_offset: u32,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"PFS0")]
pub struct PartitionFsHeader {
    entry_count: u32,
    #[br(pad_after = 4)]
    string_table_size: u32,

    #[br(count = entry_count)]
    pub entry_table: Vec<PFSEntry>,

    #[br(count = string_table_size)]
    string_table: Vec<u8>,

    #[br(calc = entry_count as u64 * size_of_val(&entry_table) as u64 + string_table_size as u64 + 0x10)]
    pub raw_data_pos: u64,
}

impl PartitionFsHeader {
    pub fn get_name_for_entry(&self, entry: &PFSEntry) -> Result<String, PartitionFsErrors> {
        let slice = &self.string_table[entry.string_offset as usize..];

        match slice.iter().position(|&b| b == 0) {
            Some(pos) => Ok(String::from_utf8(slice[..pos].to_vec())?),
            None => Err(PartitionFsErrors::NullTerminatorError),
        }
    }
}

pub trait PFSHeader {
    fn raw_data_pos(&self) -> u64;
}

impl PFSHeader for PartitionFsHeader {
    fn raw_data_pos(&self) -> u64 {
        return self.raw_data_pos;
    }
}

pub struct PartitionFs<T: BinRead + PFSHeader> {
    pub header: T,
}

impl<T: BinRead + PFSHeader> PartitionFs<T> {
    pub fn new<R: Read + Seek>(header: T) -> Result<Self, binrw::Error> {
        Ok(Self { header })
    }

    pub fn new_default_header<R: Read + Seek>(
        stream: &mut R,
    ) -> Result<PartitionFs<PartitionFsHeader>, binrw::Error> {
        let h = PartitionFsHeader::read(stream)?;

        return PartitionFs::<PartitionFsHeader>::new::<R>(h);
    }

    pub fn open_entry<R: ReadAt>(&self, entry: &PFSEntry, stream: R) -> FileRegion<R> {
        return FileRegion::new(
            stream,
            entry.offset + self.header.raw_data_pos(),
            entry.size,
        );
    }
}
