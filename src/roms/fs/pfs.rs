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

#[derive(BinRead, Debug)]
#[br(little)]
pub struct PFSEntry {
    offset: u64,
    size: u64,
    #[br(pad_after = 4)]
    string_offset: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct PartitionFsHeader {
    #[br(count = 4)]
    magic: Vec<u8>,
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

pub struct PartitionFs {
    pub header: PartitionFsHeader,
}

impl PartitionFs {
    pub fn new<T: Read + Seek>(stream: &mut T) -> Result<PartitionFs, binrw::Error> {
        Ok(Self {
            header: PartitionFsHeader::read(stream)?,
        })
    }

    pub fn open_entry<T: ReadAt>(&self, entry: &PFSEntry, stream: T) -> FileRegion<T> {
        return FileRegion::new(stream, entry.offset + self.header.raw_data_pos, entry.size);
    }
}
