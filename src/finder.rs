use std::{cell::RefCell, rc::Rc};

use crate::{
    usb::manager::{Backend, DeviceAction, UsbBackendErrors},
    utils,
};
use async_std::channel;
use gtk::glib;

#[derive(Default, Debug)]
pub struct Finder {
    task: RefCell<Option<glib::JoinHandle<()>>>,
}

async fn poll_events<C: Fn(Rc<Backend>), D: Fn()>(
    on_device: C,
    on_disconnect: D,
) -> Result<(), UsbBackendErrors> {
    let (sender, reciever) = channel::bounded(1);
    let rc_backend = Rc::new(Backend::new(sender).await?);
    while let Ok(event) = reciever.recv().await {
        match event {
            DeviceAction::Add => {
                on_device(rc_backend.clone());
            }
            DeviceAction::Remove => on_disconnect(),
        }
    }

    Ok(())
}

impl Finder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn start<ConnectFn, DisconnectFn, W>(
        &self,
        on_device: ConnectFn,
        on_disconnect: DisconnectFn,
        widget: W,
    ) where
        ConnectFn: Fn(Rc<Backend>) + 'static,
        DisconnectFn: Fn() + 'static,
        W: glib::object::IsA<gtk::Widget>,
    {
        self.task.replace(Some(glib::MainContext::default().spawn_local(async move {
            if let Err(e) = poll_events(on_device, on_disconnect).await {
                utils::send_error(&widget, &e.to_string());
            };
        })));
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}
