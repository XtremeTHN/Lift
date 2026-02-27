use crate::roms::keyring::Keyring;
use binrw::BinRead;
use std::io::{Read, Seek};

#[derive(BinRead, Debug)]
#[br(little)]
struct NcaHeader {}

struct Nca {
    header: NcaHeader,
    keyring: Keyring,
}

impl Nca {
    pub fn new<T: Read + Seek>(keyring: Keyring, stream: T) -> Self {
        let encrypted_header = vec![0u8; 0xC00];

        Self {
            keyring: keyring.clone(),
        }
    }
}
