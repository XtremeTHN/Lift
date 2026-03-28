use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use super::home_page::HomePage;
use super::roms_page::RomsPage;

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
            RomsPage::ensure_type();

            klass.bind_template();

            klass.install_action_async("win.start-finder", None, async |page, _, _| {
                page.imp().finder.start(
                    |bc| {
                        // log::info!("connected");
                    },
                    || {
                        log::info!("disconnected");
                    },
                    page.clone(),
                );
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftWindow {}
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
}
