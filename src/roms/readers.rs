use aes::cipher::{KeyIvInit, StreamCipher};
use positioned_io::ReadAt;
use std::cmp::min;
use std::io;
use std::io::{Read, Seek, SeekFrom};

use crate::roms::crypto::get_tweak;
use aes::Aes128;
use ctr::Ctr128BE;

pub struct FileRegion<T: ReadAt> {
    pub offset: u64,
    pub size: u64,
    pub pos: u64,
    pub file: T,
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

impl<T: ReadAt> Seek for FileRegion<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Current(v) => {
                let new = (self.pos as i64 + v).clamp(0, self.size as i64) as u64;
                self.pos = new;
            }
            SeekFrom::Start(v) => {
                self.pos = v;
            }
            SeekFrom::End(v) => {
                self.pos -= v.abs() as u64;
            }
        };

        Ok(self.pos)
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

pub struct EncryptedCtrFileRegion<T: ReadAt> {
    pub inner: FileRegion<T>,
    pub key: Vec<u8>,
    pub ctr: u64,
}

impl<T: ReadAt> EncryptedCtrFileRegion<T> {
    pub fn new(inner: FileRegion<T>, key: Vec<u8>, ctr: u64) -> Self {
        Self { inner, key, ctr }
    }
}

// MIT License
//
// Copyright (c) 2021 XorTroll
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

pub const fn align_down(value: u64, align: u64) -> u64 {
    let inv_mask = align - 1;
    value & !inv_mask
}

pub const fn align_up(value: usize, align: usize) -> usize {
    let inv_mask = align - 1;
    (value + inv_mask) & !inv_mask
}

impl<T: ReadAt> Read for EncryptedCtrFileRegion<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.inner.pos >= self.inner.size {
            return Ok(0);
        }

        let offset = self.inner.offset + self.inner.pos;
        let aligned_offset = align_down(offset, 0x10);
        let diff = (offset - aligned_offset) as usize;

        let max_read = min(buf.len() as u64, offset - self.inner.size) as usize;
        let read_buf_size_raw = max_read + diff;
        let read_buf_size = align_up(read_buf_size_raw, 0x10);

        let mut read_buf = vec![0u8; read_buf_size];
        self.inner.file.read_at(aligned_offset, &mut read_buf)?;

        let iv = get_tweak(((aligned_offset as u128) >> 4) | ((self.ctr as u128) << 64));
        let mut cipher = Ctr128BE::<Aes128>::new_from_slices(&self.key, &iv)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key/iv"))?;
        cipher.apply_keystream(&mut read_buf);

        let start = diff;
        let end = start + max_read;
        buf[..max_read].copy_from_slice(&read_buf[start..end]);

        self.inner.pos += max_read as u64;

        Ok(max_read)
    }
}

impl<T: ReadAt> Seek for EncryptedCtrFileRegion<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        return self.inner.seek(pos);
    }
}
