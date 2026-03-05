use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    string::FromUtf8Error,
};

use binrw::BinRead;
use positioned_io::ReadAt;

use crate::roms::readers::FileRegion;

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
pub struct RomFs<T: ReadAt + Read + Seek> {
    pub header: RomFsHeader,
    pub files: Vec<RomFsFileEntry>,
    stream: T
}

impl<T: ReadAt + Read + Seek> RomFs<T> {
    pub fn new(mut stream: T) -> Result<Self, RomFsErrors> {
        let mut r = RomFs {
            header: RomFsHeader::read(&mut stream)?,
            files: vec![],
            stream
        };

        r.populate_files()?;

        Ok(r)
    }

    fn populate_files(&mut self) -> Result<(), RomFsErrors> {
        let mut sibling: u64 = 0;

        loop {
            let offset = self.header.file_meta_table_offset + sibling;
            let size = self.header.file_meta_table_size - sibling;
            let mut buffer = vec![0u8; size as usize];

            self.stream.read_at(offset, &mut buffer)?;

            let mut cur = Cursor::new(buffer);
            let f = RomFsFileEntry::read(&mut cur)?;

            sibling = f.sibling as u64;
            self.files.push(f);

            if sibling == 4294967295 {
                return Ok(());
            }
        }
    }

    pub fn get_name_for_entry(&self, entry: &RomFsFileEntry) -> Result<String, FromUtf8Error> {
        return String::from_utf8(entry.name.clone());
    }

    pub fn get_file(&self, file: &RomFsFileEntry) -> FileRegion<&T> {
        FileRegion::new(&self.stream, self.header.data_offset + file.offset, file.size)
    }
}
