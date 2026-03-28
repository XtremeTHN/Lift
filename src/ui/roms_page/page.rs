use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/XtremeTHN/Lift/roms_page.ui")]
    pub struct RomsPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for RomsPage {
        const NAME: &'static str = "RomsPage";
        type Type = super::RomsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RomsPage {}
    impl WidgetImpl for RomsPage {}
    impl NavigationPageImpl for RomsPage {}
}

glib::wrapper! {
    pub struct RomsPage(ObjectSubclass<imp::RomsPage>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub trait RomsPageImpl: NavigationPageImpl {}

unsafe impl<T: RomsPageImpl> IsSubclassable<T> for RomsPage {}
