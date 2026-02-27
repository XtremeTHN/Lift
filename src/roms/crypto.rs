pub fn get_tweak(sector: u128) -> [u8; 0x10] {
    return sector.to_be_bytes();
}

pub fn decrypt_xts(buf: &mut [u8], key: Vec<u8>) {}
