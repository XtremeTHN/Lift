mod roms;
mod usb;

use env_logger::Env;
use log::info;

fn protocol() -> Result<(), Box<dyn std::error::Error>> {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    let mut ctx = usb::protocol::SwitchProtocol::new()?;

    ctx.find_switch()?;
    ctx.send_roms(vec!["/home/axel/undertale.nsp".to_string()])?;
    ctx.poll_commands()?;

    Ok(())
}

fn main() {
    let env = Env::default().filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);
}
