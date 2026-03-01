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
    fn raw_data_pos(&self) -> u64;
    fn string_table(&self) -> &[u8];
}

pub trait PFSEntry {
    fn string_offset(&self) -> u32;
    fn size(&self) -> u64;
    fn offset(&self) -> u64;
}

impl PFSHeader for PartitionFsHeader {
    fn raw_data_pos(&self) -> u64 {
        return self.raw_data_pos;
    }

    fn string_table(&self) -> &[u8] {
        return &self._string_table;
    }
}

#[derive(Debug)]
pub struct PartitionFs<T: BinRead + PFSHeader> {
    pub header: T,
}

impl<T: BinRead + PFSHeader> PartitionFs<T> {
    pub fn new(header: T) -> Result<Self, binrw::Error> {
        Ok(Self { header })
    }

    pub fn new_default_header<R: Read + Seek>(
        stream: &mut R,
    ) -> Result<PartitionFs<PartitionFsHeader>, binrw::Error> {
        let h = PartitionFsHeader::read(stream)?;

        return PartitionFs::<PartitionFsHeader>::new(h);
    }

    pub fn get_name_for_entry<E: PFSEntry>(&self, entry: &E) -> Result<String, PartitionFsErrors> {
        let slice = &self.header.string_table()[entry.string_offset() as usize..];

        match slice.iter().position(|&b| b == 0) {
            Some(pos) => Ok(String::from_utf8(slice[..pos].to_vec())?),
            None => Err(PartitionFsErrors::NullTerminatorError),
        }
    }

    pub fn open_entry<R: ReadAt, E: PFSEntry>(&self, entry: &E, stream: R) -> FileRegion<R> {
        return FileRegion::new(
            stream,
            entry.string_offset() as u64 + self.header.raw_data_pos(),
            entry.size(),
        );
    }
}
