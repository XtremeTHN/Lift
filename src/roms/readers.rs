use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRegion {
    pub offset: u64,
    pub size: u64,
}

impl FileRegion {
    pub fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    pub fn end(&self) -> u64 {
        self.offset + self.size
    }

    pub fn read_from(&self, file: &mut File) -> io::Result<Vec<u8>> {
        file.seek(SeekFrom::Start(self.offset))?;

        let mut buffer = vec![0u8; self.size as usize];
        file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    pub fn copy_to<W: io::Write>(&self, file: &mut File, writer: &mut W) -> io::Result<u64> {
        file.seek(SeekFrom::Start(self.offset))?;

        let mut limited = file.take(self.size);
        io::copy(&mut limited, writer)
    }
}
