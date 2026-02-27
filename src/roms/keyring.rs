use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use thiserror::Error;

use shellexpand::tilde;

#[derive(Error, Debug)]
pub enum KeyringErrors {
    #[error("Couldn't decode key/value")]
    DecodingError(#[from] FromUtf8Error),

    #[error("Failed to read")]
    ReadError(#[from] std::io::Error),
}

#[derive(Default, Clone)]
pub struct Keyring {
    key_area_application: Vec<String>,
    key_area_ocean: Vec<String>,
    key_area_system: Vec<String>,
    header_key: String,
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
            let line = String::from_utf8(raw_line.to_vec())?;

            let (key, val) = line.split_once('=').unwrap();

            if key.starts_with("key_area_key_application_") {
                self.key_area_application.push(val.to_string());
                continue;
            }

            if key.starts_with("key_area_key_ocean_") {
                self.key_area_ocean.push(val.to_string());
                continue;
            }

            if key.starts_with("key_area_key_system_") {
                self.key_area_system.push(val.to_string());
                continue;
            }

            if key == "header_key" {
                self.header_key = val.to_string();
            }
        }

        Ok(())
    }
}
