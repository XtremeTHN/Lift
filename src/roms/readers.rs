use positioned_io::ReadAt;
use std::cmp::min;
use std::io;
use std::io::Read;

pub struct FileRegion<T: ReadAt> {
    pub offset: u64,
    pub size: u64,
    pub pos: u64,
    file: T,
}

impl<T: ReadAt> FileRegion<T> {
    pub fn new(file: T, offset: u64, size: u64) -> Self {
        Self {
            offset,
            size,
            pos: 0,
            file,
        }
    }
}

impl<T: ReadAt> Read for FileRegion<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.size {
            return Ok(0);
        }

        let size = min(buf.len() as u64, self.size - self.pos) as usize;
        let n = self
            .file
            .read_at(self.pos + self.offset, &mut buf[..size])?;

        self.pos += size as u64;
        Ok(n)
    }
}

impl<T: ReadAt> ReadAt for FileRegion<T> {
    fn read_at(&self, pos: u64, buf: &mut [u8]) -> io::Result<usize> {
        if pos >= self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "position out of bounds",
            ));
        }

        let size = min(buf.len() as u64, self.size - pos) as usize;
        let n = self.file.read_at(pos + self.offset, &mut buf[..size])?;
        Ok(n)
    }
}
