mod roms;
mod usb;

use binrw::BinRead;
use env_logger::Env;
use log::info;
use positioned_io::ReadAt;
use roms::fs::pfs::PartitionFs;
use std::{
    fs::File,
    io::{Read, Write},
};

use crate::roms::{
    formats::{nca, xci::Xci},
    fs::{hfs::HFSEntry, romfs::RomFs},
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

fn nxroms() {
    let mut file = File::open("ori.xci").expect("er");
    let mut xci = Xci::new(&mut file).expect("err");
    info!("{}", size_of_val(&xci.root_hfs.header.entry_table[0]));

    let pfs = xci
        .open_partition_fs("secure".to_string(), &file)
        .expect("");

    info!("Listing pfs files...");
    for (index, entry) in pfs.header.entry_table.iter().enumerate() {
        let name = pfs.get_name_for_entry(entry).expect("failed to get name:");

        info!("Index {}: {}", index, name);
    }

    // let entry = pfs.header.entry_table[6];
    // let mut range = pfs.open_entry(&entry, file);

    // let mut keyring = Keyring::new();
    // keyring.parse().expect("coulnd't parse");

    // let mut nca = nca::Nca::new(keyring, &mut range).expect("Nca error");

    // info!("dumping");

    // let mut fs = nca.open_fs(0, &range).expect("fail");
    // let rom_fs = RomFs::new(&mut fs).expect("err");
    // info!("{:?}", rom_fs);

    // let mut f = rom_fs.get_file(&rom_fs.files[0], &fs);

    // let mut buf = vec![0u8; enc_r.inner.size as usize];
    // let mut f = File::create("out.bin").expect("fata");
    // enc_r.read(&mut buf).expect("");

    // f.write_all(&buf).expect("fatal");

    // info!("Has rights id?: {:?}", nca.header.rights_id);
    // info!(
    //     "Nca {} have a content type of {:?}",
    //     pfs.header.get_name_for_entry(&entry).expect(""),
    //     nca.header.content_type
    // );

    // info!("Key Area: {:?}", hex::encode(nca.key_area.aes_ctr_key));
    // info!("Entries: {:?}", nca.header.fs_entries);
    // info!("Headers: {:?}", nca.fs_headers);
}

fn main() {
    let env = Env::default().filter_or("LIFT_LOG", "trace");

    env_logger::init_from_env(env);

    nxroms();
}
