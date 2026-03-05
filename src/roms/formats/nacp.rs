// use crate::roms::readers::

use std::string::FromUtf8Error;

use binrw::BinRead;

fn strip(bytes: &mut Vec<u8>) {
    bytes.retain(|&b| b != 0x00);
}

#[derive(BinRead, PartialEq, Eq, Clone, Copy)]
#[br(repr(u8))]
#[br(little)]
pub enum TitleLanguage {
    AmericanEnglish = 0,
    BritishEnglish = 1,
    Japanese = 2,
    French = 3,
    German = 4,
    LatinAmericanSpanish = 5,
    Spanish = 6,
    Italian = 7,
    Dutch = 8,
    CanadianFrench = 9,
    Portuguese = 10,
    Russian = 11,
    Korean = 12,
    TraditionalChinese = 13,
    SimplifiedChinese = 14,
    BrazilianPortuguese = 15,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Title {
    #[br(count = 0x200)]
    pub raw_name: Vec<u8>,
    #[br(count = 0x100)]
    pub raw_publisher: Vec<u8>,
}

impl Title {
    pub fn name(&self) -> Result<String, FromUtf8Error> {
        let mut n = self.raw_name.clone();
        strip(&mut n);
        String::from_utf8(n)
    }

    pub fn publisher(&self) -> Result<String, FromUtf8Error> {
        let mut n = self.raw_publisher.clone();
        strip(&mut n);
        String::from_utf8(n)
    }
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct Nacp {
    #[br(count = 16)]
    pub titles: Vec<Title>,
    
    #[br(count = 0x10, seek_before = std::io::SeekFrom::Start(0x3060))]
    pub raw_version: Vec<u8>,
}

impl Nacp {
    pub fn version(&self) -> Result<String, FromUtf8Error> {
        let mut n = self.raw_version.clone();
        strip(&mut n);

        String::from_utf8(n)
    }
}