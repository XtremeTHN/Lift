// use crate::roms::readers::

use std::string::FromUtf8Error;

use binrw::BinRead;

fn strip(bytes: &mut Vec<u8>) {
    bytes.retain(|&b| b != 0x00);
}

#[derive(BinRead, PartialEq, Debug, Eq, Clone, Copy)]
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
    // remember to remove this if `roms` is separated from Lift
    Automatic = 16,
}

#[derive(thiserror::Error, Debug)]
pub enum TitleLanguageErrors {
    #[error("This language is not supported by Nintendo.")]
    LanguageNotSupported,
}

impl TitleLanguage {
    pub fn from_system_locale() -> Result<Self, TitleLanguageErrors> {
        let languages = gtk4::glib::language_names();
        let language = languages.first().map(|s| s.as_ref()).unwrap_or("en");

        match language {
            "en_US" => Ok(Self::AmericanEnglish),
            "en_GB" => Ok(Self::BritishEnglish),
            "ja" | "ja_JP" => Ok(Self::Japanese),
            "fr" | "fr_FR" => Ok(Self::French),
            "de" | "de_DE" => Ok(Self::German),
            "es_419" => Ok(Self::LatinAmericanSpanish),
            "es_ES" => Ok(Self::Spanish),
            "it" | "it_IT" => Ok(Self::Italian),
            "nl" | "nl_NL" => Ok(Self::Dutch),
            "fr_CA" => Ok(Self::CanadianFrench),
            "pt_PT" => Ok(Self::Portuguese),
            "ru" | "ru_RU" => Ok(Self::Russian),
            "ko" | "ko_KR" => Ok(Self::Korean),
            "zh_TW" | "zh_HK" => Ok(Self::TraditionalChinese),
            "zh_CN" | "zh_SG" => Ok(Self::SimplifiedChinese),
            "pt_BR" => Ok(Self::BrazilianPortuguese),
            _ => Err(TitleLanguageErrors::LanguageNotSupported),
        }
    }
}

impl TryFrom<i32> for TitleLanguage {
    type Error = TitleLanguageErrors;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AmericanEnglish),
            1 => Ok(Self::BritishEnglish),
            2 => Ok(Self::Japanese),
            3 => Ok(Self::French),
            4 => Ok(Self::German),
            5 => Ok(Self::LatinAmericanSpanish),
            6 => Ok(Self::Spanish),
            7 => Ok(Self::Italian),
            8 => Ok(Self::Dutch),
            9 => Ok(Self::CanadianFrench),
            10 => Ok(Self::Portuguese),
            11 => Ok(Self::Russian),
            12 => Ok(Self::Korean),
            13 => Ok(Self::TraditionalChinese),
            14 => Ok(Self::SimplifiedChinese),
            15 => Ok(Self::BrazilianPortuguese),
            16 => Ok(Self::Automatic),
            _ => Err(TitleLanguageErrors::LanguageNotSupported),
        }
    }
}

#[derive(BinRead)]
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

#[derive(BinRead)]
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
