use super::page::{RomsPage, RomsPageImpl};
use crate::usb::manager::Backend;
use adw::subclass::prelude::*;
use gtk::glib::object::Cast;
use gtk::glib::{self, Object};
use std::cell::OnceCell;
use std::rc::Rc;

mod imp {
    use std::cell::Cell;

    use async_std::channel::{Receiver, bounded};
    use gtk::{gio, glib::object::ObjectExt};

    use crate::{
        usb::{async_protocol::UsbOperation, manager::UsbBackend},
        utils::{self, CancellableAsyncTasks},
    };

    use super::*;

    #[derive(Default)]
    pub struct UsbRomsPage {
        pub backend: OnceCell<Rc<Backend>>,
        pub tasks: Cell<Option<CancellableAsyncTasks<()>>>,
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
            self.obj().connect_local(
                "upload-clicked",
                true,
                glib::clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    None,
                    move |_| {
                        glib::spawn_future_local(async move {
                            obj.imp().upload().await;
                        });
                        None
                    }
                ),
            );

            self.obj().connect_local(
                "cancel-clicked",
                true,
                glib::clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    None,
                    move |_| {
                        glib::spawn_future_local(async move {
                            obj.imp().cancel_upload().await;
                        });
                        None
                    }
                ),
            );
        }
    }
    impl WidgetImpl for UsbRomsPage {}
    impl NavigationPageImpl for UsbRomsPage {}

    impl UsbRomsPage {
        async fn receive_events(
            &self,
            receiver: Receiver<UsbOperation>,
            page: RomsPage,
            total_size: i64,
        ) {
            let imp = page.imp();
            while let Some(msg) = receiver.recv().await.iter().next() {
                match msg {
                    UsbOperation::File(name, chunk_read) => {
                        page.set_pulse(false);
                        imp.info_label.set_label(&format!("Sending {}...", name));

                        page.add_progress(*chunk_read as i64, total_size);
                        if let Some(rom) = page.rom(name) {
                            rom.set_progress_visible(true);
                            rom.add_progress(*chunk_read as i64);
                        } else {
                            utils::send_error(
                                &*self.obj(),
                                &format!("Row not found for rom: {}", name),
                            );
                        }
                    }
                    UsbOperation::Wait => {
                        imp.info_label.set_label("Waiting for command...");
                        page.set_pulse(true);
                    }
                    UsbOperation::Exit => {
                        page.reset_state();
                    }
                }
            }
        }

        async fn upload(&self) -> Option<()> {
            let obj = self.obj();
            let page = obj.upcast_ref::<RomsPage>();

            let rows = page.all_rows()?;
            let files = rows
                .iter()
                .map(|r| r.file().clone())
                .collect::<Vec<gio::File>>();

            let total_size = page.total_size(&rows);

            let mut tasks = CancellableAsyncTasks::<()>::new();

            let backend = self.backend.get().unwrap();

            match backend.device().await {
                Ok(mut dev) => {
                    if let Err(e) = dev.send_roms(files).await {
                        utils::send_error(
                            &obj.clone(),
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
                        #[weak(rename_to = imp)]
                        self,
                        #[weak]
                        page,
                        async move {
                            imp.receive_events(receiver, page, total_size).await;
                        }
                    ));

                    self.tasks.set(Some(tasks));
                }
                Err(e) => {
                    utils::send_error(
                        &*self.obj(),
                        &format!("Couldn't open device: {}", e.to_string()),
                    );
                }
            };

            Some(())
        }

        async fn cancel_upload(&self) {
            if let Some(t) = self.tasks.take() {
                t.cancel_all();
                self.obj().upcast_ref::<RomsPage>().reset_state();
            }
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
