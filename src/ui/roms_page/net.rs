use super::page::{RomsPage, RomsPageImpl};
use adw::subclass::prelude::*;
use gtk::glib::{self, Object}; // your base

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use async_std::channel;
    use gtk::glib::object::Cast;

    use crate::{
        remote::server::Server,
        ui::roms_page::page::{FileVec, RomVec},
        utils,
    };

    use super::*;

    #[derive(Default)]
    pub struct NetRomsPage {
        pub ip: RefCell<String>,
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
            let page = obj.upcast_ref::<RomsPage>();

            page.connect_upload_clicked(glib::clone!(
                #[weak]
                obj,
                move |_, roms, files, total_size| {
                    glib::spawn_future_local(async move {
                        obj.imp().upload(roms, files, total_size).await;
                    });
                }
            ));

            page.connect_cancel_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    glib::spawn_future_local(async move {
                        obj.imp().cancel_upload().await;
                    });
                }
            ));
        }
    }

    impl WidgetImpl for NetRomsPage {}
    impl NavigationPageImpl for NetRomsPage {}
    impl RomsPageImpl for NetRomsPage {}

    impl NetRomsPage {
        async fn upload(&self, _: RomVec, files: FileVec, total_size: i64) -> Option<()> {
            let mut srv = Server::new();

            let obj = self.obj();
            let page = obj.upcast_ref::<RomsPage>();

            let mut tasks = page.imp().tasks.borrow_mut();

            page.set_no_roms(true);
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
            if let Err(e) = srv.send_roms(files.0).await {
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
                #[weak]
                page,
                async move {
                    page.receive_events(true, receiver, total_size).await;
                }
            ));

            self.server.replace(rc);

            None
        }

        async fn cancel_upload(&self) {
            self.server.take().cancel().await;
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
