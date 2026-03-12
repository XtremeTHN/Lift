#[cfg(all(feature = "portal", feature = "gudev"))]
compile_error!("Enable either 'portal' or 'gudev', not both.");

#[cfg(feature = "portal")]
mod portal;
#[cfg(feature = "portal")]
pub use self::portal::PortalBackend as Backend;

#[cfg(feature = "gudev")]
mod gudev;
#[cfg(feature = "gudev")]
pub use self::gudev::GUdevBackend as Backend;

use crate::usb::async_protocol::{ProtocolError, SwitchProtocol};

#[derive(thiserror::Error, Debug)]
pub enum UsbBackendErrors {
    #[error("Error from protocol: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Libusb error: {0}")]
    RUsb(#[from] rusb::Error),

    #[error("Error: {0}")]
    Error(String),
}

pub trait UsbBackend {
    type Error;

    async fn start(&self) -> Result<(), Self::Error>;
    fn set_native(&self, native: gtk4::Native);
    async fn device(&self) -> Result<SwitchProtocol, UsbBackendErrors>;
}

pub enum DeviceAction {
    Add,
    Remove,
}
