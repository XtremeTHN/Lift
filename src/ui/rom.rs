use gtk::{glib, subclass::prelude::ListBoxRowImpl, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Rom {}

    #[glib::object_subclass]
    impl ObjectSubclass for Rom {
        const NAME: &'static str = "Rom";
        type Type = super::Rom;
        type ParentType = gtk::ListBoxRow;
    }

    impl ObjectImpl for Rom {}
    impl WidgetImpl for Rom {}
    impl ListBoxRowImpl for Rom {}
}

glib::wrapper! {
    pub struct Rom(ObjectSubclass<imp::Rom>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}
