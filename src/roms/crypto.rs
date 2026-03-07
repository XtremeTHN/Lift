pub fn get_tweak(sector: u128) -> [u8; 0x10] {
    sector.to_be_bytes()
}