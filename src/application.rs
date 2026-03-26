use gettextrs::gettext;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::LiftWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct LiftApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for LiftApplication {
        const NAME: &'static str = "LiftApplication";
        type Type = super::LiftApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for LiftApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<control>q"]);
        }
    }

    impl ApplicationImpl for LiftApplication {
        fn activate(&self) {
            let application = self.obj();
            let window = application.active_window().unwrap_or_else(|| {
                let window = LiftWindow::new(&*application);
                window.upcast()
            });

            window.present();
        }
    }

    impl GtkApplicationImpl for LiftApplication {}
    impl AdwApplicationImpl for LiftApplication {}
}

glib::wrapper! {
    pub struct LiftApplication(ObjectSubclass<imp::LiftApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl LiftApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/com/github/XtremeTHN/Lift")
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("lift")
            .application_icon("com.github.XtremeTHN.Lift")
            .developer_name("Axel Andres Valles Gonzalez")
            .version(VERSION)
            .developers(vec!["Axel Andres Valles Gonzalez"])
            // Translators: Replace "translator-credits" with your name/username, and optionally an email or URL.
            .translator_credits(&gettext("translator-credits"))
            .copyright("© 2026 Axel Andres Valles Gonzalez")
            .build();

        about.present(Some(&window));
    }
}
