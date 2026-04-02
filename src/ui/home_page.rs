use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use std::net::IpAddr;

mod imp {
    

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/home_page.ui")]
    pub struct HomePage {
        #[template_child]
        pub net_confirm_btt: TemplateChild<gtk::Button>,
        #[template_child]
        pub ip_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        const NAME: &'static str = "HomePage";
        type Type = super::HomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HomePage {}
    impl WidgetImpl for HomePage {}
    impl NavigationPageImpl for HomePage {}

    #[gtk::template_callbacks]
    impl HomePage {
        #[template_callback]
        fn on_child_change(&self, _: glib::ParamSpec, stack: adw::ViewStack) {
            let child = stack.visible_child_name().unwrap();

            let obj = self.obj();
            if child == "usb" {
                let _ = obj
                    .activate_action("win.start-finder", None)
                    .inspect_err(|e| log::warn!("couldn't start finder: {}", e));
            } else {
                let _ = obj
                    .activate_action("win.stop-finder", None)
                    .inspect_err(|e| log::warn!("couldn't stop finder: {}", e));
            }
        }

        #[template_callback]
        fn on_changed(&self, row: adw::EntryRow) {
            let text = row.text();

            let is_valid = text.parse::<IpAddr>().is_ok();
            self.net_confirm_btt.set_sensitive(is_valid);
        }

        #[template_callback]
        fn on_confirm(&self) {
            if !self.net_confirm_btt.get_sensitive() {
                return;
            }

            let ip = self.ip_row.text();

            let _ = self
                .obj()
                .activate_action("win.show-net", Some(&ip.to_variant()));
        }
    }
}

glib::wrapper! {
    pub struct HomePage(ObjectSubclass<imp::HomePage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
