use gtk4::{
    glib,
    gio,
    CompositeTemplate,
    TemplateChild,
    prelude::*,
    subclass::prelude::*
};
use libadwaita::{Application, ApplicationWindow, subclass::prelude::*};
use glib::{Object, subclass::InitializingObject};

// use std::{
//     borrow::Borrow,
//     cell::RefCell,
//     default::Default,
//     os::fd::AsRawFd,
// };

mod imp {
    use gtk4::glib::VariantTy;

    use super::*;
    use crate::ui::{not_found_page::NotFoundPage, roms_page::RomsPage};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/window.ui")]
    pub struct LiftWindow {
        #[template_child]
        pub toast: TemplateChild<libadwaita::ToastOverlay>,
        #[template_child]
        pub navigation: TemplateChild<libadwaita::NavigationView>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LiftWindow {
        const NAME: &'static str = "LiftWindow";

        type Type = super::LiftWindow;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            NotFoundPage::ensure_type();
            RomsPage::ensure_type();
            klass.bind_template();

            klass.install_action("win.toast", Some(VariantTy::STRING), |win, _, arg| {
                let msg = arg.unwrap().str();
                if let Some(_msg) = msg {
                    win.add_toast(_msg);
                } else {
                    log::warn!("Toast: Argument was not a string");
                }
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftWindow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for LiftWindow {}

    impl WindowImpl for LiftWindow {}

    impl ApplicationWindowImpl for LiftWindow {}

    impl AdwApplicationWindowImpl for LiftWindow {}
}

glib::wrapper! {
    pub struct LiftWindow(ObjectSubclass<imp::LiftWindow>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager,
                    libadwaita::ApplicationWindow;
}

impl LiftWindow {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn add_toast(&self, message: &str) {
        let obj = self.imp();

        let toast = libadwaita::Toast::new(message);
        obj.toast.add_toast(toast);
    }
}