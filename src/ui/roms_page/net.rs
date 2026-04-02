use super::page::{RomsPage, RomsPageImpl};
use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib::{self, Object},
}; // your base

mod imp {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    use async_std::channel::{self, Receiver};
    use gtk::glib::object::{Cast, ObjectExt};

    use crate::{
        remote::server::Server,
        ui::rom::Rom,
        usb::async_protocol::ProtocolOperation,
        utils::{self, CancellableAsyncTasks},
    };

    use super::*;

    #[derive(Default)]
    pub struct NetRomsPage {
        pub ip: RefCell<String>,
        pub tasks: RefCell<CancellableAsyncTasks<()>>,
        pub server: RefCell<Rc<Server>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetRomsPage {
        const NAME: &'static str = "NetRomsPage";
        type Type = super::NetRomsPage;
        type ParentType = RomsPage;
    }

    impl ObjectImpl for NetRomsPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.connect_local(
                "upload-clicked",
                true,
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    #[upgrade_or]
                    None,
                    move |_| {
                        glib::spawn_future_local(async move {
                            imp.upload().await;
                        });
                        None
                    }
                ),
            );

            obj.connect_local(
                "cancel-clicked",
                true,
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    #[upgrade_or]
                    None,
                    move |_| {
                        glib::spawn_future_local(async move {
                            imp.cancel_upload().await;
                        });
                        None
                    }
                ),
            );
        }
    }

    impl WidgetImpl for NetRomsPage {}
    impl NavigationPageImpl for NetRomsPage {}
    impl RomsPageImpl for NetRomsPage {}

    impl NetRomsPage {
        async fn receive_events(
            &self,
            receiver: Receiver<ProtocolOperation>,
            page: RomsPage,
            total_size: i64,
        ) {
            let hash: HashMap<String, Rom> = page.roms_hash();
            let imp = page.imp();

            loop {
                let msg = match glib::future_with_timeout(
                    std::time::Duration::from_secs(5),
                    receiver.recv(),
                )
                .await
                {
                    Ok(Ok(msg)) => msg,
                    Ok(Err(_)) => break, // channel closed
                    Err(_) => {
                        utils::send_error(&*self.obj(), "Timeout");
                        self.cancel_upload().await;
                        break;
                    }
                };

                match msg {
                    ProtocolOperation::File(name, chunk_read) => {
                        page.set_pulse(false);
                        imp.info_label.set_label(&format!("Sending {}...", name));
                        page.add_progress(chunk_read as i64, total_size);
                        if let Some(rom) = hash.get(&*name) {
                            rom.set_progress_visible(true);
                            rom.add_progress(chunk_read as i64);
                        } else {
                            utils::send_error(
                                &*self.obj(),
                                &format!("Row not found for rom: {}", name),
                            );
                        }
                    }
                    ProtocolOperation::Wait(message) => {
                        imp.info_label.set_label(&message);
                        page.set_pulse(true);
                    }
                    ProtocolOperation::Exit => {
                        page.reset_state();
                        break;
                    }
                }
            }
        }

        async fn upload(&self) -> Option<()> {
            let mut srv = Server::new();

            // TODO: move this to page.rs
            let obj = self.obj();
            let page = obj.upcast_ref::<RomsPage>();

            // TODO: optimize this
            // maybe use only one for loop?
            let rows = page.all_rows()?;
            let files = rows
                .iter()
                .map(|r| r.file().clone())
                .collect::<Vec<gio::File>>();

            let total_size = page.total_size(&rows);

            let mut tasks = self.tasks.borrow_mut();

            page.set_info_reveal(true);
            page.set_pulse(true);
            page.set_info("Connecting to switch...");

            if let Err(e) = srv.connect_to_switch(&self.ip.borrow()).await {
                utils::send_error(&*obj, &format!("Failed to serve: {}", e.to_string()));
                page.reset_state();
                return None;
            };

            let (sender, receiver) = channel::bounded(1);

            page.set_info("Sending roms...");
            if let Err(e) = srv.send_roms(files).await {
                utils::send_error(&*obj, &format!("Failed to send roms: {}", e.to_string()));
                page.reset_state();
                return None;
            };

            page.set_info("Waiting for switch to connect...");

            page.set_cancel_visible(true);

            let rc = Rc::new(srv);

            tasks.spawn_task(glib::clone!(
                #[weak]
                obj,
                #[weak]
                rc,
                #[weak(rename_to = imp)]
                self,
                async move {
                    if let Err(e) = rc.serve(sender).await {
                        utils::send_error(&obj, &format!("Error while serving: {}", e.to_string()));
                        imp.cancel_upload().await;
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

            self.server.replace(rc);

            None
        }

        // TODO: move this to page.rs
        async fn cancel_upload(&self) {
            let mut t = self.tasks.borrow_mut();
            self.server.take().cancel().await;
            t.cancel_all();
            self.obj().upcast_ref::<RomsPage>().reset_state();
        }
    }
}

glib::wrapper! {
    pub struct NetRomsPage(ObjectSubclass<imp::NetRomsPage>)
        @extends RomsPage, gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NetRomsPage {
    pub fn new(ip: &str) -> Self {
        let obj: NetRomsPage = Object::builder().build();

        obj.imp().ip.replace(ip.to_string());

        obj
    }
}
