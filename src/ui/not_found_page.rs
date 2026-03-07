use gtk4::{
    glib,
    CompositeTemplate,
    subclass::prelude::*
};
use libadwaita::{NavigationPage, subclass::prelude::*};
use glib::subclass::InitializingObject;

mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/not-found-page.ui")]
    pub struct NotFoundPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for NotFoundPage {
        const NAME: &'static str = "NotFoundPage";

        type Type = super::NotFoundPage;
        type ParentType = NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NotFoundPage {}
    impl WidgetImpl for NotFoundPage {}
    impl NavigationPageImpl for NotFoundPage {}

}

glib::wrapper! {
    pub struct NotFoundPage(ObjectSubclass<imp::NotFoundPage>)
        @extends gtk4::Widget, NavigationPage,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

// impl NotFoundPage {}