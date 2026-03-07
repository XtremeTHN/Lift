use gtk4::{self, gio, gio::prelude::{ApplicationExt, ApplicationExtManual}, prelude::GtkWindowExt};
use libadwaita::{Application};
use std::path::PathBuf;

mod rom_info;
mod config;
mod utils;
mod roms;
mod usb;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = env_logger::Env::default().filter_or("LIFT_LOG", "info");
    env_logger::init_from_env(env);

    let mut ctx = usb::protocol::SwitchProtocol::new()?;

    ctx.find_switch()?;
    ctx.send_roms(vec!["/home/axel/undertale.nsp".to_string()])?;
    ctx.poll_commands()?;

    Ok(())
}
// fn main() {
//     let env = env_logger::Env::default().filter_or("LIFT_LOG", "info");
//     env_logger::init_from_env(env);
    
//     let mut buf = PathBuf::from(config::PKGDATADIR);
//     buf.push("lift.gresource");

//     let res = gio::Resource::load(buf).expect("failed to load resource");
//     gio::resources_register(&res);

//     let _app = Application::builder()
//         .application_id("com.github.XtremeTHN.Lift")
//         .build();

//     _app.connect_activate(move |app| {
//         let win = ui::window::LiftWindow::new(&app);
//         win.present();
//     });


//     _app.run();
// }
