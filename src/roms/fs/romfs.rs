use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    string::FromUtf8Error,
};

use binrw::BinRead;
use positioned_io::ReadAt;

#[derive(BinRead, Debug)]
#[br(little)]
pub struct RomFsHeader {
    #[br(assert(header_size == 80))]
    pub header_size: u64,

    pub dir_hash_table_offset: u64,
    pub dir_hash_table_size: u64,

    pub dir_meta_table_offset: u64,
    pub dir_meta_table_size: u64,

    pub file_hash_table_offset: u64,
    pub file_hash_table_size: u64,

    pub file_meta_table_offset: u64,
    pub file_meta_table_size: u64,

    pub data_offset: u64,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct RomFsFileEntry {
    pub parent: u32,
    pub sibling: u32,
    pub offset: u64,
    pub size: u64,
    pub hash: u32,
    pub name_size: u32,

    #[br(count = name_size)]
    pub name: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct RomFsDirectoryEntry {
    pub parent: u32,
    pub sibling: u32,
    pub child: u32,
    pub file: u32,
    pub hash: u32,
    pub name_size: u32,

    #[br(count = name_size)]
    pub name: Vec<u8>,
}

#[derive(thiserror::Error, Debug)]
pub enum RomFsErrors {
    #[error("The romfs is invalid/corrupted")]
    CorruptRomFs(#[from] binrw::Error),
    #[error("Failed to read: {0:?}")]
    ReadError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct RomFs {
    pub header: RomFsHeader,
    pub files: Vec<RomFsFileEntry>,
}

impl RomFs {
    pub fn new<T: Read + Seek>(stream: &mut T) -> Result<Self, RomFsErrors> {
        let mut r = RomFs {
            header: RomFsHeader::read(stream)?,
            files: vec![],
        };

        r.populate_files(stream)?;

        Ok(r)
    }

    fn populate_files<T: Read + Seek>(&mut self, stream: &mut T) -> Result<(), RomFsErrors> {
        let mut sibling: u64 = 0;
        let old = stream.stream_position()?;

        loop {
            let offset = self.header.file_meta_table_offset + sibling;
            let size = self.header.file_meta_table_size - sibling;
            let mut buffer = vec![0u8; size as usize];

            stream.seek(SeekFrom::Start(offset))?;
            stream.read(&mut buffer)?;

            let mut cur = Cursor::new(buffer);
            let f = RomFsFileEntry::read(&mut cur)?;

            sibling = f.sibling as u64;
            self.files.push(f);

            if sibling == 4294967295 {
                stream.seek(SeekFrom::Start(old))?;
                return Ok(());
            }
        }
    }

    pub fn get_name_for_entry(&self, entry: &RomFsFileEntry) -> Result<String, FromUtf8Error> {
        return String::from_utf8(entry.name.clone());
    }
}
