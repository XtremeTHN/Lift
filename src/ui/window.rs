use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use super::home_page::HomePage;
use super::roms_page::usb::UsbRomsPage;

mod imp {
    use crate::finder::Finder;

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

            klass.install_action("win.start-finder", None, |page, _, _| {
                page.setup_finder();
            });

            klass.install_action("win.stop-finder", None, |page, _, _| {
                page.imp().finder.stop();
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
            // self.activate_action("win.start-finder", None);
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

    fn setup_finder(&self) {
        let imp = self.imp();
        let navigation_on_connect = imp.navigation.clone();
        let navigation_on_disconnect = imp.navigation.clone();

        let _obj = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let obj = _obj.clone();
            let imp = obj.imp();
            imp.finder
                .start(
                    move |_bc| {
                        log::info!("connected");
                        let page = UsbRomsPage::new();
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
