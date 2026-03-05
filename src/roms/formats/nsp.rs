use crate::roms::fs::pfs::{PFSEntry, PartitionFsHeader};
use std::fs::File;

struct Nsp {
    ncas: Vec<PFSEntry>,
}

// impl Nsp {
//     pub fn from_header() -> Self {
//         // Self {}
//     }
// }
