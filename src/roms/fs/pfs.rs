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
pub struct PartitionFsEntry {
    _offset: u64,
    _size: u64,
    #[br(pad_after = 4)]
    _string_offset: u32,
}

impl PFSEntry for PartitionFsEntry {
    fn string_offset(&self) -> u32 {
        self._string_offset
    }

    fn size(&self) -> u64 {
        self._size
    }

    fn offset(&self) -> u64 {
        self._offset
    }
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"PFS0")]
pub struct PartitionFsHeader {
    entry_count: u32,
    #[br(pad_after = 4)]
    string_table_size: u32,

    #[br(count = entry_count)]
    pub entry_table: Vec<PartitionFsEntry>,

    #[br(count = string_table_size)]
    _string_table: Vec<u8>,

    #[br(calc = entry_count as u64 * size_of_val(&entry_table) as u64 + string_table_size as u64 + 0x10)]
    pub raw_data_pos: u64,
}

pub trait PFSHeader {
    type Entry: PFSEntry;

    fn raw_data_pos(&self) -> u64;
    fn string_table(&self) -> &[u8];
    fn entry_table(&self) -> &Vec<Self::Entry>;
}

pub trait PFSEntry {
    fn string_offset(&self) -> u32;
    fn size(&self) -> u64;
    fn offset(&self) -> u64;
}

impl PFSHeader for PartitionFsHeader {
    type Entry = PartitionFsEntry;

    fn raw_data_pos(&self) -> u64 {
        self.raw_data_pos
    }

    fn string_table(&self) -> &[u8] {
        &self._string_table
    }

    fn entry_table(&self) -> &Vec<Self::Entry> {
        &self.entry_table
    }
}

#[derive(Debug)]
pub struct PartitionFs<T: BinRead + PFSHeader, R: ReadAt> {
    pub header: T,
    pub stream: R
}

impl<T: BinRead + PFSHeader, R: ReadAt + Read + Seek> PartitionFs<T, R> {
    pub fn new(header: T, stream: R) -> Result<Self, binrw::Error> {
        Ok(Self { header, stream })
    }

    pub fn get_name_for_entry<E: PFSEntry>(&self, entry: &E) -> Result<String, PartitionFsErrors> {
        let slice = &self.header.string_table()[entry.string_offset() as usize..];

        match slice.iter().position(|&b| b == 0) {
            Some(pos) => Ok(String::from_utf8(slice[..pos].to_vec())?),
            None => Err(PartitionFsErrors::NullTerminatorError),
        }
    }

    pub fn open_entry<E: PFSEntry>(&self, entry: &E) -> FileRegion<&R> {
        return FileRegion::new(
            &self.stream,
            entry.offset() as u64 + self.header.raw_data_pos(),
            entry.size(),
        );
    }
}

impl<R: ReadAt + Read + Seek> PartitionFs<PartitionFsHeader, R> {
    pub fn new_default_header(
        mut stream: R,
    ) -> Result<PartitionFs<PartitionFsHeader, R>, binrw::Error> {
        let h = PartitionFsHeader::read(&mut stream)?;

        return PartitionFs::<PartitionFsHeader, R>::new(h, stream);
    }
}
