use std::path::PathBuf;

use gtk4::{
    self,
    gio::{self, prelude::ApplicationExtManual},
    prelude::{ApplicationExt, GtkWindowExt},
};
use libadwaita::Application;

mod config;
mod rom_info;
mod ui;
mod usb;
mod utils;

fn main() {
    let env = env_logger::Env::default().filter_or("LIFT_LOG", "info");
    env_logger::init_from_env(env);

    let mut buf = PathBuf::from(config::PKGDATADIR);
    buf.push("lift.gresource");

    let res = gio::Resource::load(buf).expect("failed to load resource");
    gio::resources_register(&res);

    let _app = Application::builder()
        .application_id("com.github.XtremeTHN.Lift")
        .build();

    _app.connect_activate(move |app| {
        let win = ui::window::LiftWindow::new(app);
        win.present();
    });

    _app.run();
}
