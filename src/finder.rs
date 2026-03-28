use std::{cell::RefCell, rc::Rc};

use crate::{
    usb::manager::{Backend, DeviceAction, UsbBackend, UsbBackendErrors},
    utils,
};
use async_std::channel::{self, Receiver};
use gtk::glib;

#[derive(Default, Debug)]
pub struct Finder {
    protocol_task: RefCell<Option<glib::JoinHandle<()>>>,
    polling_task: RefCell<Option<glib::JoinHandle<()>>>,
}

async fn poll_events<C: Fn(Rc<Backend>), D: Fn()>(
    on_device: C,
    on_disconnect: D,
    receiver: Receiver<DeviceAction>,
    rc_backend: Rc<Backend>,
) {
    log::info!("polling");
    while let Ok(event) = receiver.recv().await {
        match event {
            DeviceAction::Add => {
                log::info!("added");
                on_device(rc_backend.clone());
            }
            DeviceAction::Remove => on_disconnect(),
        }
    }
}

impl Finder {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn start<ConnectFn, DisconnectFn, W>(
        &self,
        on_device: ConnectFn,
        on_disconnect: DisconnectFn,
        widget: W,
    ) where
        ConnectFn: Fn(Rc<Backend>) + 'static,
        DisconnectFn: Fn() + 'static,
        W: glib::object::IsA<gtk::Widget>,
    {
        let (sender, receiver) = channel::bounded(1);
        match Backend::new(sender).await {
            Ok(bc) => {
                let rc_backend = Rc::new(bc);

                let prot_bc = rc_backend.clone();
                self.protocol_task
                    .replace(Some(glib::MainContext::default().spawn_local(async move {
                        if let Err(e) = prot_bc.start().await {
                            utils::send_error(&widget, &e.to_string());
                        }
                    })));

                self.polling_task
                    .replace(Some(glib::MainContext::default().spawn_local(async move {
                        poll_events(on_device, on_disconnect, receiver, rc_backend).await;
                    })));
            }
            Err(err) => {
                utils::send_error(&widget, &err.to_string());
            }
        }
    }

    pub fn stop(&self) {
        if let Some(handle) = self.polling_task.take() {
            handle.abort();
        }

        if let Some(handle) = self.protocol_task.take() {
            handle.abort();
        }
    }
}
