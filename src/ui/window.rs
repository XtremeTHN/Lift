use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::usb::manager::UsbBackend;

use super::home_page::HomePage;
use super::roms_page::usb::UsbRomsPage;
use super::settings::LiftSettings;

mod imp {
    use adw::prelude::AdwDialogExt;
    use gtk::glib::VariantTy;

    use crate::{finder::Finder, ui::roms_page::net::NetRomsPage};

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/window.ui")]
    pub struct LiftWindow {
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub navigation: TemplateChild<adw::NavigationView>,

        pub finder: Finder,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LiftWindow {
        const NAME: &'static str = "LiftWindow";
        type Type = super::LiftWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            HomePage::ensure_type();

            klass.bind_template();

            klass.install_action("win.start-finder", None, |win, _, _| {
                win.setup_finder();
            });

            klass.install_action("win.stop-finder", None, |win, _, _| {
                win.imp().finder.stop();
            });

            klass.install_action("win.show-net", Some(VariantTy::STRING), |win, _, arg| {
                let ip = arg.unwrap().str().unwrap();
                let net = NetRomsPage::new(ip);
                win.imp().navigation.push(&net);
            });

            klass.install_action("win.settings", None, |win, _, _| {
                let settings = LiftSettings::new();
                settings.present(Some(win));
            });

            klass.install_action("win.toast", Some(VariantTy::STRING), |win, _, arg| {
                let msg = arg.unwrap().str();
                if let Some(_msg) = msg {
                    win.add_toast(_msg);
                } else {
                    log::warn!("Toast: Argument was not a string");
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().setup_finder();

            // let rom =
        }
    }
    impl WidgetImpl for LiftWindow {}
    impl WindowImpl for LiftWindow {}
    impl ApplicationWindowImpl for LiftWindow {}
    impl AdwApplicationWindowImpl for LiftWindow {}

    impl LiftWindow {}
}

glib::wrapper! {
    pub struct LiftWindow(ObjectSubclass<imp::LiftWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager,
                    adw::ApplicationWindow;
}

impl LiftWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    pub fn add_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        self.imp().toast.add_toast(toast);
    }

    fn setup_finder(&self) {
        let imp = self.imp();
        let navigation_on_connect = imp.navigation.clone();
        let navigation_on_disconnect = imp.navigation.clone();

        let _obj = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let obj = _obj.clone();
            let imp = obj.imp();

            let native = obj.native();
            imp.finder
                .start(
                    move |_bc| {
                        log::info!("connected");
                        _bc.set_native(native.clone().unwrap());
                        let page = UsbRomsPage::new(_bc);
                        navigation_on_connect.push(&page);
                    },
                    move || {
                        log::info!("disconnected");
                        navigation_on_disconnect.pop_to_tag("home-page");
                    },
                    _obj,
                )
                .await;
        });
    }
}
