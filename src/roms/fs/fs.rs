use binrw::BinRead;
use positioned_io::ReadAt;
// use crate::roms::readers::FileRegion;

pub fn media_to_bytes(media: u32) -> u32 {
    return media * 0x200;
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(little)]
pub struct FsEntry {
    #[br(map(|x| media_to_bytes(x)))]
    start_offset: u32,
    #[br(pad_after = 0x8, map(|x| media_to_bytes(x)))]
    end_offset: u32,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8, little)]
pub enum FsType {
    RomFS = 0,
    PartitionFs = 1,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum HashType {
    Auto = 0,
    None = 1,
    HierarchicalSha256Hash = 2,
    HierarchicalIntegrityHash = 3,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum EncryptionType {
    Auto = 0,
    None = 1,
    AesXts = 2,
    AesCtr = 3,
    AesCtrEx = 4,
    AesCtrSkipLayerHash = 5,
    AesCtrExSkipLayerHash = 6,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum MetadataHashType {
    None = 0,
    HierarchicalIntegrity = 1,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct MetadataHashInfo {
    table_offset: u64,
    table_size: u64,

    #[br(count = 0x10)]
    table_hash: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct LayerRegion {
    offset: u64,
    size: u64,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct HierarchicalSha256Data {
    #[br(count = 0x20)]
    master_hash: Vec<u8>,
    block_size: u32,
    #[br(pad_after = 0x4)]
    layer_count: u32,

    #[br(count = layer_count)]
    layer_regions: Vec<LayerRegion>,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct HierarchicalIntegrityLevel {
    logical_offset: u64,
    hash_data_size: u64,
    #[br(pad_after = 0x4)]
    block_size: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct InfoLevelHash {
    max_layers: u32,

    #[br(count = 6)]
    levels: Vec<HierarchicalIntegrityLevel>,

    #[br(count = 0x20)]
    salt: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"IVFC")]
pub struct HierarchicalIntegrity {
    version: u32,
    master_hash_size: u32,
    info_level_hash: InfoLevelHash,

    #[br(count = 0x20, pad_after = 0x18)]
    master_hash: Vec<u8>,
}

#[derive(BinRead, Debug)]
pub enum HashData {
    HierarchicalIntegrity(HierarchicalIntegrity),
    HierarchicalSha256(HierarchicalSha256Data),
    // Unknown,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct FsHeader {
    version: u16,
    fs_type: FsType,
    hash_type: HashType,
    encryption_type: EncryptionType,
    #[br(pad_after = 2)]
    meta_hash_type: MetadataHashType,
    hash_data: HashData,
    meta_hash_data_info: MetadataHashInfo,

    #[br(seek_before = std::io::SeekFrom::Start(0x140))]
    ctr: u64,
}
