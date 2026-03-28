mod application;
mod config;
mod ui;

use self::application::LiftApplication;
use self::ui::window::LiftWindow;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, glib};
use gtk::prelude::*;

fn main() -> glib::ExitCode {
    let env = env_logger::Env::default().filter_or("LIFT_LOG", "info");
    env_logger::init_from_env(env);
    
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/lift.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let app = LiftApplication::new("com.github.XtremeTHN.Lift", &gio::ApplicationFlags::empty());
    app.run()
}
