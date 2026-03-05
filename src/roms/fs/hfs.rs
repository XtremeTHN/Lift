use binrw::BinRead;

use crate::roms::fs::pfs::{PFSEntry, PFSHeader, PartitionFsErrors};

#[derive(BinRead, Debug, Clone, Copy)]
#[br(little)]
pub struct HFSEntry {
    _offset: u64,
    _size: u64,
    #[br(pad_after = 0x2C)]
    _string_offset: u32,
}

impl PFSEntry for HFSEntry {
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
#[br(little, magic = b"HFS0")]
pub struct HashPartitionFsHeader {
    entry_count: u32,
    #[br(pad_after = 4)]
    string_table_size: u32,

    #[br(count = entry_count)]
    pub entry_table: Vec<HFSEntry>,

    #[br(count = string_table_size)]
    string_table: Vec<u8>,

    // HFSEntry is 0x40 bytes, not 0x18 like PFS0
    #[br(calc = entry_count as u64 * 0x40 as u64 + string_table_size as u64 + 0x10)]
    pub raw_data_pos: u64,
}

impl PFSHeader for HashPartitionFsHeader {
    type Entry = HFSEntry;
    fn raw_data_pos(&self) -> u64 {
        self.raw_data_pos
    }

    fn string_table(&self) -> &[u8] {
        &self.string_table
    }

    fn entry_table(&self) -> &Vec<Self::Entry> {
        &self.entry_table
    }
}
