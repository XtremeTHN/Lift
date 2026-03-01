mod roms;
mod usb;

use binrw::BinRead;
use env_logger::Env;
use log::info;
use positioned_io::ReadAt;
use roms::fs::pfs::PartitionFs;
use std::{
    fs::File,
    io::{Read, Seek, Write},
};

use crate::roms::{
    formats::{
        nca::{self, Nca},
        xci::Xci,
    },
    fs::{
        hfs::HFSEntry,
        pfs::{PFSEntry, PFSHeader, PartitionFsHeader},
        romfs::RomFs,
    },
    keyring::Keyring,
};

fn protocol() -> Result<(), Box<dyn std::error::Error>> {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    let mut ctx = usb::protocol::SwitchProtocol::new()?;

    ctx.find_switch()?;
    ctx.send_roms(vec!["/home/axel/undertale.nsp".to_string()])?;
    ctx.poll_commands()?;

    Ok(())
}

fn list_romfs_files(rom_fs: RomFs) {
    info!("Listing romfs files...");
    for (index, file) in rom_fs.files.iter().enumerate() {
        let name = String::from_utf8(file.name.clone()).expect("error while decoding name");
        info!("{}: {}", index, name);
    }
}

fn get_control_nca_romfs<T: BinRead + PFSHeader, R: ReadAt + Read + Seek>(
    pfs: PartitionFs<T>,
    part: R,
) -> Option<RomFs> {
    let mut keyring = Keyring::new();
    keyring.parse().expect("error while parsing keyring");
    for (index, entry) in pfs.header.entry_table().iter().enumerate() {
        let name = pfs.get_name_for_entry(entry).expect("failed to get name:");

        let mut r = pfs.open_entry(entry, &part);

        if name.split(".").collect::<Vec<&str>>()[1] != "nca" {
            continue;
        }

        let mut nca = Nca::new(&keyring, &mut r).expect("err");

        match nca.header.content_type {
            nca::ContentType::Control => {
                info!("found control nca at index {}: {}", index, name);
                let mut fs = nca.open_fs(0, &mut r).expect("err");
                let rom_fs = RomFs::new(&mut fs).expect("err");

                return Some(rom_fs);
            }
            _ => {}
        }
    }

    None
}

fn xci_test() {
    let mut file = File::open("ori.xci").expect("er");
    let mut xci = Xci::new(&mut file).expect("err");

    let mut part = xci
        .open_partition("secure".to_string(), &file)
        .expect("err");
    let pfs = xci.open_partition_fs(&mut part, &file).expect("");

    info!("Listing pfs files...");

    let romfs = get_control_nca_romfs(pfs, part).unwrap();

    list_romfs_files(romfs);
}

fn nsp_test() {
    let mut file = File::open("celeste.nsp").expect("failed");
    let pfs = PartitionFs::new_default_header(&mut file).expect("failed");

    let romfs = get_control_nca_romfs(pfs, file).unwrap();
    list_romfs_files(romfs);
}

fn nxroms() {
    // xci_test();
    nsp_test();
}

fn main() {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    nxroms();
}
