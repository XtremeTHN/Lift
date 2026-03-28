use super::page::{RomsPage, RomsPageImpl};
use adw::subclass::prelude::*;
use gtk::glib; // your base

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct NetRomsPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for NetRomsPage {
        const NAME: &'static str = "NetRomsPage";
        type Type = super::NetRomsPage;
        type ParentType = RomsPage;
    }

    impl ObjectImpl for NetRomsPage {}
    impl WidgetImpl for NetRomsPage {}
    impl NavigationPageImpl for NetRomsPage {}
    impl RomsPageImpl for NetRomsPage {}
}

glib::wrapper! {
    pub struct NetRomsPage(ObjectSubclass<imp::NetRomsPage>)
        @extends RomsPage, gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
