use crate::usb::{
    async_protocol::{ProtocolError, SwitchProtocol},
    manager::UsbBackendErrors,
};

use super::{DeviceAction, UsbBackend};
use async_channel::Sender;
use gtk4::glib::MainContext;
use gudev::{
    Client,
    prelude::{ClientExt, DeviceExt},
};
use rusb::{Context, Device, DeviceHandle, UsbContext};

pub struct GUdevBackend {
    client: Client,
    ctx: Context,
    sender: Sender<DeviceAction>,
}

fn find_switch(ctx: &Context) -> Result<DeviceHandle<Context>, UsbBackendErrors> {
    let devs = ctx.devices()?;

    let mut switch: Option<Device<Context>> = None;
    for dev in devs.iter() {
        let descriptor = dev.device_descriptor().unwrap();

        if descriptor.vendor_id() == 0x057E && descriptor.product_id() == 0x3000 {
            switch = Some(dev);
        }
    }

    if switch.is_none() {
        return Err(UsbBackendErrors::Error(String::from("Switch not found")));
    }

    let dev = switch.unwrap();
    let handle = dev.open()?;
    Ok(handle)
}

impl GUdevBackend {
    pub async fn new(sender: Sender<DeviceAction>) -> Result<Self, UsbBackendErrors> {
        let client = Client::new(&["usb/usb_interface"]);
        let ctx = Context::new()?;
        Ok(Self {
            client,
            ctx,
            sender,
        })
    }
}

impl UsbBackend for GUdevBackend {
    type Error = UsbBackendErrors;

    async fn start(&self) -> Result<(), Self::Error> {
        let sender = self.sender.clone();
        self.client.connect_uevent(move |_, action, dev| {
            if let Some(vendor) = dev.property("ID_VENDOR_FROM_DATABASE")
                && vendor != "Nintendo Co., Ltd"
            {
                return;
            }

            if let Some(product) = dev.property("PRODUCT")
                && product != "57e/3000/100"
            {
                return;
            }

            let _sender = sender.clone();

            if action == "add" {
                MainContext::default().spawn_local(async move {
                    let _ = _sender.send(DeviceAction::Add).await;
                });
            } else if action == "remove" {
                MainContext::default()
                    .spawn_local(async move { _sender.send(DeviceAction::Remove).await });
            } else {
                log::warn!("Unknown action: {}", action);
            }
        });

        Ok(())
    }

    fn set_native(&self, _: gtk4::Native) {}

    async fn device(&self) -> Result<SwitchProtocol, UsbBackendErrors> {
        let handle = find_switch(&self.ctx)?;
        let mut protocol = SwitchProtocol::new()?;
        protocol.spawn_usb(handle).await?;

        Ok(protocol)
    }
}
