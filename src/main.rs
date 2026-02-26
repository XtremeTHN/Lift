mod usb;
use env_logger::Env;
use log::{info, error};
use rusb::{Context, Device, Error, UsbContext};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Env::default()
        .filter_or("LIFT_LOG", "info");

    env_logger::init_from_env(env);

    let mut ctx = usb::protocol::SwitchProtocol::new()?;

    let devs = ctx.ctx.devices()?;

    let mut switch: Option<Device<Context>> = None;
    for dev in devs.iter() {
        let descriptor = dev.device_descriptor().unwrap();

        if descriptor.vendor_id() == 0x057E && descriptor.product_id() == 0x3000 {
            info!("Found switch on bus {:03}", dev.bus_number());
            switch = Some(dev);
        }
    }

    if switch.is_none() {
        return Err("Switch not found".into());
    }

    ctx.set_switch(switch.unwrap())?;

    ctx.send_roms(vec!["/home/axel/undertale.nsp".to_string()]);
    ctx.poll_commands()?;
    Ok(())
}
