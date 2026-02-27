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
    let mut pfs = PartitionFs::new(&mut file).expect("pfs");

    info!("Listing pfs files...");
    for (index, entry) in pfs.header.entry_table.iter().enumerate() {
        let name = pfs
            .header
            .get_name_for_entry(&entry)
            .expect("failed to get name:");

        info!("Index {}: {}", index, name);
    }

    let mut range = pfs.open_entry(&pfs.header.entry_table[5], file);

    let mut output = File::create("out.bin").expect("opening:");

    loop {
        let mut buf = vec![0u8; 1024];
        let read = range.read(&mut buf).expect("reading:");

        if read == 0 {
            break;
        }

        output.write_all(&buf).expect("writting:");
    }
}

fn main() {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    nxroms();
}
