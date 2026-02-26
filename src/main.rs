mod roms;
mod usb;

use env_logger::Env;
use log::info;
use std::fs::File;
use std::io::{Cursor, Write};

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
    let region = roms::readers::FileRegion::new(0, 4);

    let mut buf = Cursor::new([0u8; 20]);
    region.copy_to(&mut file, &mut buf).expect("couldn't");

    info!(
        "Magic: {}",
        String::from_utf8(buf.into_inner().to_vec()).expect("decoding")
    );
}

fn main() {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    nxroms();
}
