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
    pub start_offset: u32,
    #[br(pad_after = 0x8, map(|x| media_to_bytes(x)))]
    pub end_offset: u32,
}

#[derive(BinRead, Debug, PartialEq, Eq, Clone, Copy)]
#[br(repr = u8, little)]
pub enum FsType {
    RomFS = 0,
    PartitionFs = 1,
}

#[derive(BinRead, Debug, PartialEq, Eq, Clone, Copy)]
#[br(repr = u8)]
pub enum HashType {
    Auto = 0,
    None = 1,
    HierarchicalSha256Hash = 2,
    HierarchicalIntegrityHash = 3,
}

#[derive(BinRead, Debug, PartialEq, Eq, Clone, Copy)]
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

#[derive(BinRead, Debug, PartialEq, Eq, Clone, Copy)]
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
    pub offset: u64,
    pub size: u64,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct HierarchicalSha256Data {
    #[br(count = 0x20)]
    pub master_hash: Vec<u8>,
    pub block_size: u32,
    #[br(pad_after = 0x4)]
    pub layer_count: u32,

    #[br(count = layer_count)]
    pub layer_regions: Vec<LayerRegion>,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct HierarchicalIntegrityLevel {
    pub logical_offset: u64,
    pub hash_data_size: u64,
    #[br(pad_after = 0x4)]
    pub block_size: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct InfoLevelHash {
    pub max_layers: u32,

    #[br(count = 6)]
    pub levels: Vec<HierarchicalIntegrityLevel>,

    #[br(count = 0x20)]
    pub salt: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"IVFC")]
pub struct HierarchicalIntegrity {
    pub version: u32,
    pub master_hash_size: u32,
    pub info_level_hash: InfoLevelHash,

    #[br(count = 0x20, pad_after = 0x18)]
    pub master_hash: Vec<u8>,
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
    pub version: u16,
    pub fs_type: FsType,
    pub hash_type: HashType,
    pub encryption_type: EncryptionType,
    #[br(pad_after = 2)]
    pub meta_hash_type: MetadataHashType,
    pub hash_data: HashData,
    pub meta_hash_data_info: MetadataHashInfo,
    #[br(seek_before = std::io::SeekFrom::Start(0x140))]
    pub ctr: u64,

    #[br(ignore)]
    pub section: u8,
}
