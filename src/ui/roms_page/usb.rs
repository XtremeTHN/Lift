use super::page::{RomsPage, RomsPageImpl};
use crate::usb::manager::Backend;
use adw::subclass::prelude::*;
use gtk::glib::object::Cast;
use gtk::glib::{self, Object};
use std::cell::OnceCell;
use std::rc::Rc;

mod imp {
    use std::cell::RefCell;

    use async_std::channel::bounded;
    use gtk::{gio, glib::object::ObjectExt};

    use crate::{
        ui::roms_page::page::{FileVec, RomVec},
        usb::manager::UsbBackend,
        utils::{self, CancellableAsyncTasks},
    };

    use super::*;

    #[derive(Default)]
    pub struct UsbRomsPage {
        pub backend: OnceCell<Rc<Backend>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UsbRomsPage {
        const NAME: &'static str = "UsbRomsPage";
        type Type = super::UsbRomsPage;
        type ParentType = RomsPage;
    }

    impl RomsPageImpl for UsbRomsPage {}
    impl ObjectImpl for UsbRomsPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.upcast_ref::<RomsPage>()
                .connect_upload_clicked(glib::clone!(
                    #[weak]
                    obj,
                    move |_, roms, files, total_size| {
                        glib::spawn_future_local(async move {
                            obj.imp().upload(roms, files, total_size).await;
                        });
                    }
                ));
        }
    }
    impl WidgetImpl for UsbRomsPage {}
    impl NavigationPageImpl for UsbRomsPage {}

    impl UsbRomsPage {
        async fn upload(&self, _: RomVec, files: FileVec, total_size: i64) -> Option<()> {
            let obj = self.obj();
            let page = obj.upcast_ref::<RomsPage>();

            let backend = self.backend.get().unwrap();
            let mut tasks = page.imp().tasks.borrow_mut();

            match backend.device().await {
                Ok(mut dev) => {
                    if let Err(e) = dev.send_roms(files.0).await {
                        utils::send_error(
                            &*obj,
                            &format!("Couldn't send roms to the switch: {}", e.to_string()),
                        );

                        return None;
                    };

                    page.set_info_reveal(true);
                    page.set_cancel_visible(true);

                    let (sender, receiver) = bounded(1);

                    tasks.spawn_task(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            if let Err(e) = dev.poll_commands(sender).await {
                                utils::send_error(
                                    &obj,
                                    &format!("Error while polling commands: {}", e.to_string()),
                                );
                            };
                        }
                    ));

                    tasks.spawn_task(glib::clone!(
                        #[weak]
                        page,
                        async move {
                            page.receive_events(false, receiver, total_size).await;
                        }
                    ));
                }
                Err(e) => {
                    utils::send_error(&*obj, &format!("Couldn't open device: {}", e.to_string()));
                }
            };

            Some(())
        }
    }
}

glib::wrapper! {
    pub struct UsbRomsPage(ObjectSubclass<imp::UsbRomsPage>)
        @extends RomsPage, gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl UsbRomsPage {
    pub fn new(bc: Rc<Backend>) -> Self {
        let obj: Self = Object::builder().build();
        let _ = obj.imp().backend.set(bc);
        obj
    }
}
