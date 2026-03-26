use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use super::home_page::HomePage;
use super::roms_page::RomsPage;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/window.ui")]
    pub struct LiftWindow {
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub navigation: TemplateChild<adw::NavigationView>
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
}

glib::wrapper! {
    pub struct LiftWindow(ObjectSubclass<imp::LiftWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl LiftWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
