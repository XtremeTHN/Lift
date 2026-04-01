use std::net::IpAddr;

use async_std::{channel::Sender, io::WriteExt, net::TcpStream};
use gtk::glib::JoinHandle;

use gtk::gio;

// TODO: find a place for this enum
use crate::usb::async_protocol::ProtocolOperation;
use crate::utils::FileVecBuilder;

struct Server {
    switch_sock: Option<TcpStream>,
    host_ip: Option<IpAddr>,
    server_task: Option<JoinHandle<()>>,
    server_ip: Option<String>,
}

#[derive(thiserror::Error, Debug)]
enum ServeErrors {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Server {
    pub fn new() -> Self {
        Self {
            switch_sock: None,
            host_ip: None,
            server_task: None,
            server_ip: None,
        }
    }

    pub async fn connect_to_switch(&mut self, switch_ip: &str) -> Result<(), ServeErrors> {
        let switch_sock = TcpStream::connect(switch_ip).await?;
        self.host_ip = Some(switch_sock.local_addr()?.ip());
        self.switch_sock = Some(switch_sock);

        Ok(())
    }

    async fn run_server(&self, sender: Sender<ProtocolOperation>) {}

    pub async fn serve(
        &self,
        roms: Vec<gio::File>,
        sender: Sender<ProtocolOperation>,
    ) -> Result<(), ServeErrors> {
        let Some(server_ip) = self.server_ip.as_ref() else {
            return Ok(());
        };

        let Some(mut switch_sock) = self.switch_sock.as_ref() else {
            return Ok(());
        };

        let payload = FileVecBuilder::new()
            .prefix(&server_ip)
            .gfiles(roms)
            .build_net();

        switch_sock.write_all(&payload);

        Ok(())
    }

    pub fn cancel(&mut self) {
        if let Some(t) = self.server_task.take() {
            t.abort();
        }
    }
}
