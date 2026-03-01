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

use crate::roms::{formats::nca, keyring::Keyring};

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
    let mut file = File::open("undertale.nsp").expect("er");
    let pfs = PartitionFs::new(&mut file).expect("pfs");

    info!("Listing pfs files...");
    for (index, entry) in pfs.header.entry_table.iter().enumerate() {
        let name = pfs
            .header
            .get_name_for_entry(&entry)
            .expect("failed to get name:");

        info!("Index {}: {}", index, name);
    }

    let entry = pfs.header.entry_table[6];
    let mut range = pfs.open_entry(&entry, file);

    let mut keyring = Keyring::new();
    keyring.parse().expect("coulnd't parse");

    let mut nca = nca::Nca::new(keyring, &mut range).expect("Nca error");

    info!("dumping");

    let mut enc_r = nca.open_fs(0, &range).expect("fatal");

    let mut buf = vec![0u8; enc_r.inner.size as usize];
    let mut f = File::create("out.bin").expect("fata");
    enc_r.read(&mut buf).expect("");

    f.write_all(&buf).expect("fatal");

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
