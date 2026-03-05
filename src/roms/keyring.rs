use hex::{FromHexError, decode};
use log::info;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use thiserror::Error;

use shellexpand::tilde;

#[derive(Error, Debug)]
pub enum KeyringErrors {
    #[error("Couldn't decode value")]
    HexDecodingError(#[from] FromHexError),

    #[error("Couldn't decode key")]
    Utf8DecodingError(#[from] FromUtf8Error),

    #[error("Failed to read")]
    ReadError(#[from] std::io::Error),
}

#[derive(Default, Debug, Clone)]
pub struct Keyring {
    pub key_area_application: Vec<Vec<u8>>,
    pub key_area_ocean: Vec<Vec<u8>>,
    pub key_area_system: Vec<Vec<u8>>,
    pub header_key: Vec<u8>,
}

impl Keyring {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn parse(&mut self) -> Result<(), KeyringErrors> {
        let mut file = File::open(tilde("~/.switch/prod.keys").to_string())?;

        let mut buf = vec![];
        file.read_to_end(&mut buf)?;

        for raw_line in buf.split(|&b| b == b'\n') {
            let unsplitted = String::from_utf8(raw_line.to_vec())?;
            let line = unsplitted.split_once('=');

            if line.is_none() {
                continue;
            }

            let (key, val) = {
                let (_key, _val) = line.unwrap();

                (_key.replace(" ", ""), _val.replace(" ", ""))
            };

            if key.starts_with("key_area_key_application_") {
                self.key_area_application.push(decode(val).expect("err"));
                continue;
            }

            if key.starts_with("key_area_key_ocean_") {
                self.key_area_ocean.push(decode(val).expect("err"));
                continue;
            }

            if key.starts_with("key_area_key_system_") {
                self.key_area_system.push(decode(val).expect("err"));
                continue;
            }

            if key == "header_key" {
                self.header_key = decode(val).expect("err");
            }
        }

        Ok(())
    }
}
