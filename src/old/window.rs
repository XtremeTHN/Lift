use glib::{Object, subclass::InitializingObject};
use gtk4::glib::property::PropertySet;
use gtk4::{
    CompositeTemplate,TemplateChild, gio, glib, prelude::*, subclass::prelude::*};
use libadwaita::{Application, ApplicationWindow, subclass::prelude::*};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::{default::Default, os::fd::AsRawFd};

use std::sync::OnceLock;
use tokio::runtime::Runtime;

use futures_util::StreamExt;
use ashpd::desktop::usb::{AcquireDevicesOptions, Device, DeviceID, UsbEventAction, UsbProxy};
use ashpd::{WindowIdentifier, AppID};


#[derive(Default)]
pub struct State {
    switch_id: RefCell<String>
}

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/XtremeTHN/Lift/window.ui")]
    pub struct LiftWindow {
        #[template_child]
        pub btt: TemplateChild<gtk4::Button>,
        #[template_child]
        pub ovr: TemplateChild<libadwaita::ToastOverlay>,

        pub state: RefCell<State>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LiftWindow {
        const NAME: &'static str = "LiftWindow";

        type Type = super::LiftWindow;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action_async("win.find-switch", None, async |window, _, _| {
                window.find_switch().await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    // Trait shared by all GObjects
    impl ObjectImpl for LiftWindow {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            let obj = self.obj();
            obj.setup_finder(&self.ovr);

            // // Connect to "clicked" signal of `button`
            // self.button.connect_clicked(move |button| {
            //     // Set the label to "Hello World!" after the button has been clicked on
            //     button.set_label("Hello World!");
            // });
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

#[gtk4::template_callbacks]
impl LiftWindow {
    pub fn new(app: &Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    fn setup_finder(&self, ovr: &libadwaita::ToastOverlay) {
        let _ovr = RefCell::new(ovr.clone());
        let obj = self.clone();

        glib::MainContext::default().spawn_local(
            async move {
            let proxy = UsbProxy::new().await.expect("err");
            let _ = proxy.create_session(Default::default()).await.expect("err");
            
            let mut stream = proxy.receive_device_events().await.expect("ERRS");

            while let Some(event) = stream.next().await {
                let events = event.events();

                for x in events {
                    println!("{}", x.device_id());
                    let imp = obj.imp();
                    let state = imp.state.borrow();
                    let id = &state.switch_id;

                    
                    let toast = libadwaita::Toast::new("");
                    match x.action() {
                        UsbEventAction::Add => {
                            if x.device().vendor().unwrap_or(String::new()) != "Nintendo Co., Ltd" {
                                continue;
                            }

                            id.set(x.device_id().to_string());
                            toast.set_title("Switch connected");
                        },
                        UsbEventAction::Remove => { 
                            if x.device_id().as_str() != id.borrow().as_str() {
                                continue;
                            }
                            toast.set_title("Switch disconnected");
                        },
                        _ => {
                            continue;
                        }
                    }
                    
                    _ovr.borrow().add_toast(toast);
                };
            }
        });
    }

    async fn acquire_switch(&self, dev_id: &DeviceID) {
        let proxy = UsbProxy::new().await.expect("err");
        let root = self.native().unwrap();
        let handle = WindowIdentifier::from_native(&root).await;
        let device = Device::new(dev_id.clone(), true);

        // let options = 
        let r = proxy.acquire_devices(handle.as_ref(), &[device], Default::default()).await.expect("couldn't acquire device");

        if r.len() == 0 {
            println!("couldn't acquire device");
            return;
        }
        let (id, fd) = &r[0];
        
        if let Err(e) = fd {
            println!("couldnt get correct permissions: {}", e.to_string());
            return;
        }

        let mut ctx = crate::usb::protocol::SwitchProtocol::new().expect("err");
        ctx.open_switch_from_fd(fd.as_ref().unwrap().as_raw_fd()).expect("err");

        ctx.send_roms(vec!["ori.xci".to_string()]).expect("err");
    }

    async fn find_switch(&self) {
        let proxy = UsbProxy::new().await.expect("err");
        let devices = proxy.enumerate_devices(Default::default()).await.expect("err");

        for (id, dev) in devices.iter() {
            if dev.vendor().unwrap() != "Nintendo Co., Ltd" {
                continue;
            }
            
            println!("{} {}", dev.is_readable(), dev.is_writable());
            self.acquire_switch(id).await;
            break;
        }
    }

    // fn _find_switch(&self) {
    //     let (p_sender, p_reciever) = async_channel::bounded::<UsbProxy>(1);
    //     let (sender, reciever) = async_channel::bounded::<DeviceID>(1);
    //     runtime().spawn( async move {
    //         let proxy = UsbProxy::new().await.expect("err");

    //         let devices = proxy.enumerate_devices(Default::default()).await.expect("err");

    //         for (x, v) in devices.iter() {
    //             if v.vendor().unwrap() == "Nintendo Co., Ltd" {
    //                 p_sender.send(proxy).await.expect("failed to send proxy");
    //                 sender.send(x.clone()).await.expect("failed to send device id");
    //                 break;
    //             }
    //         }
    //     });

    //     let native = self.native().unwrap();

    //     glib::MainContext::default().spawn_local(glib::clone!(
    //         #[weak]
    //         native,
    //         async move {
    //             let handle = WindowIdentifier::from_native(&native).await;
    //             while let Ok(proxy) = p_reciever.recv().await {
    //                 let dev_id = reciever.recv().await.unwrap();
    //                 let device = Device::new(dev_id, true);

    //                 spawn_tokio(async move {
    //                     let proxy = UsbProxy::new().await.expect("failed to get usb proxy");
    //                     // proxy.acquire_devices(handle.as_ref(), &[device], Default::default()).await;
    //                 });


    //             }
    //         })
    //     );
    // }
}
