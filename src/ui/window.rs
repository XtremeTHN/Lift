use async_channel::Receiver;
use glib::{Object, subclass::InitializingObject};
use gtk4::{CompositeTemplate, TemplateChild, gio, glib, prelude::*, subclass::prelude::*};
use libadwaita::{Application, ApplicationWindow, subclass::prelude::*};
use std::{cell::RefCell, rc::Rc};

use crate::{
    usb::manager::{Backend, DeviceAction, UsbBackend},
    utils::{self},
};

mod imp {
    use gtk4::glib::VariantTy;
    use libadwaita::prelude::AdwDialogExt;

    use super::*;
    use crate::ui::{not_found_page::NotFoundPage, roms_page::RomsPage, settings::LiftSettings};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/window.ui")]
    pub struct LiftWindow {
        #[template_child]
        pub toast: TemplateChild<libadwaita::ToastOverlay>,
        #[template_child]
        pub navigation: TemplateChild<libadwaita::NavigationView>,
        #[template_child]
        pub roms_page: TemplateChild<RomsPage>,
        pub switch_id: RefCell<String>,
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

            klass.install_action("win.settings", None, |win, _, _| {
                let set = LiftSettings::new();
                set.present(Some(win));
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LiftWindow {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_usb_finder();
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

    async fn recieve_events(&self, receiver: Receiver<DeviceAction>, backend: Rc<Backend>) {
        let imp = self.imp();
        imp.roms_page.set_backend(backend);
        while let Ok(e) = receiver.recv().await {
            match e {
                DeviceAction::Add => {
                    imp.navigation.push_by_tag("roms-page");
                }
                DeviceAction::Remove => {
                    imp.navigation.pop_to_tag("switch-not-found");
                }
            }
        }
    }

    fn setup_usb_finder(&self) {
        let obj = self.clone();
        glib::MainContext::default().spawn_local(async move {
            // obj.spawn_finder().await;
            let (sender, receiver) = async_channel::bounded(1);
            match Backend::new(sender).await {
                Ok(backend) => {
                    let native = obj.native().unwrap();
                    backend.set_native(native);

                    let backend = Rc::new(backend);

                    // start backend loop
                    {
                        let obj = obj.clone();
                        let _backend = backend.clone();
                        glib::MainContext::default().spawn_local(async move {
                            if let Err(e) = _backend.clone().start().await {
                                utils::send_error(&obj, &e.to_string());
                            }
                        });
                    }

                    // receive events loop
                    {
                        let obj = obj.clone();
                        glib::MainContext::default().spawn_local(async move {
                            obj.recieve_events(receiver, backend.clone()).await;
                        });
                    }
                }
                Err(e) => {
                    utils::send_error(&obj, &e.to_string());
                }
            }
        });
    }

    fn add_toast(&self, message: &str) {
        let obj = self.imp();

        let toast = libadwaita::Toast::new(message);
        obj.toast.add_toast(toast);
    }
}
