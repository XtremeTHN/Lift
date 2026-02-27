use binrw::BinRead;
use positioned_io::ReadAt;
// use crate::roms::readers::FileRegion;

#[derive(BinRead, Debug, Clone, Copy)]
pub struct FsEntry {
    start_offset: u32,
    end_offset: u32,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum FsType {
    RomFS = 0,
    PartitionFs = 1,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u8)]
pub enum HashType {
    Auto = 0,
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

    #[br(count = max_layers, pad_after = 0x4)]
    levels: Vec<InfoLevelHash>,

    #[br(count = 0x20)]
    salt: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"IVFC")]
pub struct HierarchicalIntegrity {
    version: u32,
    master_hash_size: u32,
    info_level_hash: InfoLevelHash, // check this if error
}

#[derive(BinRead, Debug)]
#[br(import { _type: HashType })]
pub enum HashData {
    #[br(pre_assert(matches!(_type, HashType::HierarchicalIntegrityHash)))]
    HierarchicalIntegrity(HierarchicalIntegrity),
    #[br(pre_assert(matches!(_type, HashType::HierarchicalSha256Hash)))]
    HierarchicalSha256(HierarchicalSha256Data),
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
    #[br(args { _type: hash_type })]
    hash_data: HashData,
    meta_hash_data_info: MetadataHashInfo,

    #[br(seek_before = std::io::SeekFrom::Start(0x140))]
    ctr: u64,
}
