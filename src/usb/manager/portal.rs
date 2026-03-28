use crate::usb::{async_protocol::SwitchProtocol, manager::UsbBackendErrors};

use super::{DeviceAction, UsbBackend};
use ashpd::{
    Error, WindowIdentifier,
    desktop::usb::{Device, DeviceID, UsbEventAction, UsbProxy},
};
use async_channel::Sender;
use futures_util::StreamExt;
use std::{
    cell::{OnceCell, RefCell},
    os::fd::AsRawFd,
};

pub struct PortalBackend {
    proxy: UsbProxy,
    switch_id: RefCell<Option<DeviceID>>,
    native: OnceCell<gtk4::Native>,
    sender: Sender<DeviceAction>,
}

impl PortalBackend {
    pub async fn new(sender: Sender<DeviceAction>) -> Result<Self, Error> {
        let proxy = UsbProxy::new().await?;

        Ok(Self {
            proxy,
            switch_id: RefCell::new(None),
            native: Default::default(),
            sender,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PortalBackendErrors {
    #[error("Couldn't get device events: {0}")]
    Events(#[from] Error),
}

impl UsbBackend for PortalBackend {
    type Error = PortalBackendErrors;
    async fn start(&self) -> Result<(), Self::Error> {
        let _ = self.proxy.create_session(Default::default()).await;
        let mut stream = self.proxy.receive_device_events().await?;

        while let Some(event) = stream.next().await {
            let events = event.events();

            for x in events {
                match x.action() {
                    UsbEventAction::Add => {
                        if x.device().vendor() != Some("Nintendo Co., Ltd".to_string()) {
                            continue;
                        }

                        self.switch_id.replace(Some(x.device_id().clone()));
                        let _ = self.sender.send(DeviceAction::Add).await;
                    }
                    UsbEventAction::Remove => {
                        let switch = {
                            let id = self.switch_id.borrow();
                            id.clone()
                        };

                        if let Some(dev) = switch
                            && x.device_id().as_str() == dev.as_str()
                        {
                            let _ = self.sender.send(DeviceAction::Remove).await;
                        }
                    }
                    UsbEventAction::Change => {
                        log::warn!("Invalid action: Change");
                    }
                }
            }
        }

        Ok(())
    }

    fn set_native(&self, native: gtk4::Native) {
        let _ = self.native.set(native);
    }

    async fn device(&self) -> Result<SwitchProtocol, UsbBackendErrors> {
        let root = self.native.get().unwrap();
        let dev_id = self.switch_id.borrow().clone().unwrap();

        let handle = WindowIdentifier::from_native(root).await;

        let device = Device::new(dev_id, true);

        let devices = match self
            .proxy
            .acquire_devices(handle.as_ref(), &[device], Default::default())
            .await
        {
            Ok(d) => d,
            Err(e) => {
                return Err(UsbBackendErrors::Error(e.to_string()));
            }
        };

        let dev_tuple = match devices.into_iter().next() {
            Some(d) => d,
            None => {
                return Err(UsbBackendErrors::Error(
                    "The request was dismissed".to_string(),
                ));
            }
        };

        let fd = match dev_tuple.1 {
            Ok(fd) => fd,
            Err(e) => {
                return Err(UsbBackendErrors::Error(
                    "Couldn't acquire device.".to_string(),
                ));
            }
        };

        let mut protocol = match SwitchProtocol::new() {
            Ok(p) => p,
            Err(e) => {
                return Err(UsbBackendErrors::Error(e.to_string()));
            }
        };

        if let Err(e) = protocol.open_switch_from_fd(fd).await {
            return Err(UsbBackendErrors::Error(e.to_string()));
        }

        Ok(protocol)
    }
}
