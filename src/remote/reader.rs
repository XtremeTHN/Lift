use async_std::{channel::Sender, fs::File};

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// TODO: find a place for this enum
use crate::usb::async_protocol::ProtocolOperation;
pub struct LoggedReader {
    inner: async_std::io::Take<File>,
    name: String,
    sender: Sender<ProtocolOperation>,
    cancelled: Arc<AtomicBool>,
}

impl LoggedReader {
    pub fn new(
        inner: async_std::io::Take<File>,
        name: String,
        sender: Sender<ProtocolOperation>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            inner,
            name,
            sender,
            cancelled,
        }
    }
}

impl async_std::io::Read for LoggedReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if self.cancelled.load(Ordering::Relaxed) {
            return std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            )));
        }

        let result = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        if let std::task::Poll::Ready(Ok(n)) = result {
            self.sender
                .send_blocking(ProtocolOperation::File(self.name.clone().into(), n as u64))
                .map_err(|e| {
                    log::error!("couldn't send chunk: {:?}", e);
                    std::io::Error::other(e)
                })?;
        }
        result
    }
}
