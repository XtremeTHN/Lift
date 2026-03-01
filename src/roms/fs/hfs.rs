use binrw::BinRead;

use crate::roms::fs::pfs::PFSHeader;

#[derive(BinRead, Debug, Clone, Copy)]
#[br(little)]
pub struct HFSEntry {
    offset: u64,
    size: u64,
    #[br(pad_after = 0x18)]
    string_offset: u32,
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

    #[br(calc = entry_count as u64 * size_of_val(&entry_table) as u64 + string_table_size as u64 + 0x10)]
    pub raw_data_pos: u64,
}

impl PFSHeader for HashPartitionFsHeader {
    fn raw_data_pos(&self) -> u64 {
        return self.raw_data_pos;
    }
}
