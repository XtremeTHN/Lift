use super::page::{RomsPage, RomsPageImpl};
use adw::subclass::prelude::*;
use gtk::glib::{self, Object}; // your base

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct UsbRomsPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for UsbRomsPage {
        const NAME: &'static str = "UsbRomsPage";
        type Type = super::UsbRomsPage;
        type ParentType = RomsPage;
    }

    impl ObjectImpl for UsbRomsPage {}
    impl WidgetImpl for UsbRomsPage {}
    impl NavigationPageImpl for UsbRomsPage {}
    impl RomsPageImpl for UsbRomsPage {}
}

glib::wrapper! {
    pub struct UsbRomsPage(ObjectSubclass<imp::UsbRomsPage>)
        @extends RomsPage, gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UsbRomsPage {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
