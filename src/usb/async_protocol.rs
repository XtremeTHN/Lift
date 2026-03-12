#[cfg(feature = "portal")]
use ashpd::zvariant::OwnedFd;
use binrw::BinRead;
use gtk4::gio::prelude::{CancellableExt, FileExt, InputStreamExt, SeekableExt};
use log::{info, warn};
use rusb::Error;
use rusb::{ConfigDescriptor, Context, DeviceHandle, UsbContext};
use std::io::Cursor;
#[cfg(feature = "portal")]
use std::os::fd::AsRawFd;
use std::string::FromUtf8Error;

use gtk4::{gio, glib};
use std::sync::Arc;

use super::daemon::{UsbCommand, spawn_daemon};
use async_channel::Sender;

#[repr(u32)]
enum ProtocolCommand {
    Exit = 1,
    FileRange = 2,
    FileRangePadded = 3,
}

impl TryFrom<u32> for ProtocolCommand {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ProtocolCommand::Exit),
            1 => Ok(ProtocolCommand::FileRange),
            2 => Ok(ProtocolCommand::FileRangePadded),
            _ => Err(()),
        }
    }
}

#[derive(BinRead)]
#[br(little)]
struct FileHeader {
    range_size: u64,
    range_offset: u64,
    rom_name_length: u64,
    #[br(ignore)]
    name: String,
}

pub enum UsbOperation {
    File(Arc<str>, u64),
    Exit,
    Wait,
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),
    #[error("Command send failed: {0}")]
    Send(#[from] async_channel::SendError<UsbCommand>),
    #[error("Command recv failed: {0}")]
    Recv(#[from] async_channel::RecvError),
    #[error("libusb error: {0}")]
    Rusb(#[from] Error),
    #[error("Binary parse failed: {0}")]
    BinRead(#[from] binrw::Error),
    #[error("Decoding error: {0}")]
    Decoding(#[from] FromUtf8Error),
    #[error("gio::File error: {0}")]
    File(#[from] glib::Error),
    #[error("Invalid magic: {0}")]
    InvalidMagic(String),
}

type ProtocolResult<T> = Result<T, ProtocolError>;

const BUFFER_SEGMENT_DATA_SIZE: u64 = 0x100000;
const PADDING_SIZE: u64 = 0x1000;

const PAD_3: [u8; 3] = [0; 3];
const PAD_8: [u8; 8] = [0; 8];
const PAD_C: [u8; 0xC] = [0; 0xC];
const TUL0: &[u8; 4] = b"TUL0";

const FILE_HEADER_SIZE: usize = 0x20;

pub struct SwitchProtocol {
    pub ctx: Context,
    daemon_sender: Option<Sender<UsbCommand>>,
    #[cfg(feature = "portal")]
    fd: Option<OwnedFd>,
}

impl SwitchProtocol {
    pub fn new() -> Result<SwitchProtocol, Error> {
        let ctx = Context::new()?;

        Ok(Self {
            ctx,
            daemon_sender: None,
            #[cfg(feature = "portal")]
            fd: None,
        })
    }

    async fn open_fd(&mut self, fd: i32) -> ProtocolResult<DeviceHandle<Context>> {
        Ok(unsafe { self.ctx.open_device_with_fd(fd) }?)
    }

    #[cfg(feature = "portal")]
    pub async fn open_switch_from_fd(&mut self, fd: OwnedFd) -> ProtocolResult<()> {
        let handle = self.open_fd(fd.as_raw_fd()).await?;
        self.fd = Some(fd);
        self.spawn_usb(handle).await?;

        Ok(())
    }

    #[cfg(feature = "gudev")]
    pub async fn open_switch_from_fd(&mut self, fd: i32) -> ProtocolResult<()> {
        let handle = self.open_fd(fd).await?;
        self.spawn_usb(handle).await?;

        Ok(())
    }

    /// Sends the roms that will be transferred
    /// ```
    /// protocol.send_roms(vec!["ori.xci", "undertale.nsp"]);
    /// ```
    pub async fn send_roms(&self, roms: Vec<String>) -> ProtocolResult<()> {
        let mut new_vec: Vec<String> = Vec::new();
        let mut length = 0;

        for file in roms {
            new_vec.push(file.clone() + "\n");
            length += file.len() + 1;
        }

        self.send_list_header(length as u32).await?;

        for file in new_vec {
            self.write(file.as_bytes()).await?;
        }

        Ok(())
    }
    async fn send_exit(&mut self) -> ProtocolResult<()> {
        if let Some(old) = &self.daemon_sender {
            let (sender, reciever) = async_channel::bounded(1);
            old.send(UsbCommand::Exit(sender)).await?;
            reciever.recv().await??;
        }

        self.daemon_sender = None;
        Ok(())
    }

    /// Handles the commands sent by the switch
    /// Call find_switch() before using this function
    /// Send the roms before using this function
    pub async fn poll_commands(
        &mut self,
        cancellable: Option<gio::Cancellable>,
        sender: Sender<UsbOperation>,
    ) -> ProtocolResult<()> {
        let _cancellable = if let Some(c) = cancellable {
            c
        } else {
            gio::Cancellable::new()
        };

        loop {
            if _cancellable.is_cancelled() {
                info!("Cancelled");
                break;
            }

            let _ = sender.send(UsbOperation::Wait).await;
            let header = self.read_with_timeout(0x20, 0).await?;

            let magic = String::from_utf8(header[0..4].to_vec())?;
            if magic != "TUC0" {
                return Err(ProtocolError::InvalidMagic(magic));
            }

            let raw_cmd = u32::from_le_bytes(header[8..12].try_into().unwrap());
            match ProtocolCommand::try_from(raw_cmd) {
                Ok(ProtocolCommand::Exit) => {
                    info!("Exit recieved");
                    break;
                }
                Ok(ProtocolCommand::FileRange) => {
                    info!("Recieved FileRange command");
                    self.send_file(false, &sender, &_cancellable).await?;
                }
                Ok(ProtocolCommand::FileRangePadded) => {
                    info!("Recieved FileRangePadded command");
                    self.send_file(true, &sender, &_cancellable).await?;
                }
                Err(_) => {
                    warn!("Invalid command id");
                    continue;
                }
            }
        }

        let _ = sender.send(UsbOperation::Exit).await;
        self.send_exit().await?;

        Ok(())
    }

    pub async fn spawn_usb(&mut self, handle: DeviceHandle<Context>) -> ProtocolResult<()> {
        self.send_exit().await?;

        let dev = handle.device();
        let (in_endpoint, out_endpoint, interface) =
            self.find_endpoints(dev.active_config_descriptor()?)?;
        handle.claim_interface(interface)?;
        let sender = spawn_daemon(handle, in_endpoint, out_endpoint, interface);
        self.daemon_sender = Some(sender);

        Ok(())
    }

    fn find_endpoints(&mut self, conf_desc: ConfigDescriptor) -> ProtocolResult<(u8, u8, u8)> {
        let mut in_endpoint: Option<u8> = None;
        let mut out_endpoint: Option<u8> = None;

        let mut interface_num: Option<u8> = None;

        for interface in conf_desc.interfaces() {
            for altsetting in interface.descriptors() {
                for endpoint in altsetting.endpoint_descriptors() {
                    let address = endpoint.address();
                    if address == 0x81 {
                        info!("Found in endpoint: {}", address);
                        in_endpoint = Some(address);
                        interface_num = Some(altsetting.interface_number());
                    } else {
                        info!("Found out endpoint: {}", address);
                        out_endpoint = Some(address);
                        interface_num = Some(altsetting.interface_number());
                    }
                }
            }
        }

        if in_endpoint.is_none() {
            return Err(ProtocolError::EndpointNotFound(String::from("IN")));
        }
        if out_endpoint.is_none() {
            return Err(ProtocolError::EndpointNotFound(String::from("OUT")));
        }

        Ok((
            in_endpoint.unwrap(),
            out_endpoint.unwrap(),
            interface_num.unwrap(),
        ))
    }

    async fn write(&self, buf: &[u8]) -> ProtocolResult<()> {
        let sender = self.daemon_sender.as_ref().unwrap();

        let (_send, rec) = async_channel::bounded(1);

        // i think `buf.to_vec()` is going to be a problem
        sender
            .send(UsbCommand::Write(buf.to_vec(), _send, 1))
            .await?;
        rec.recv().await??;

        Ok(())
    }

    async fn read_with_timeout(&self, size: usize, timeout: u64) -> ProtocolResult<Vec<u8>> {
        let sender = self.daemon_sender.as_ref().unwrap();

        let (_send, rec) = async_channel::bounded(1);
        sender.send(UsbCommand::Read(size, _send, timeout)).await?;

        Ok(rec.recv().await??)
    }

    async fn read(&self, size: usize) -> ProtocolResult<Vec<u8>> {
        self.read_with_timeout(size, 1).await
    }

    async fn send_list_header(&self, length: u32) -> ProtocolResult<()> {
        info!("Sending rom list with length of {}", length);
        self.write(TUL0).await?;
        self.write(&length.to_le_bytes()).await?;
        self.write(&PAD_8).await?; // padding

        Ok(())
    }

    async fn recieve_file(&self) -> ProtocolResult<FileHeader> {
        let header = self.read(FILE_HEADER_SIZE).await?;

        let mut cur = Cursor::new(header);
        let mut file_header = FileHeader::read(&mut cur)?;

        let raw_name = self.read(file_header.rom_name_length as usize).await?;
        file_header.name = String::from_utf8(raw_name)?;

        Ok(file_header)
    }

    async fn send_file_response_header(
        &self,
        cmd_id: ProtocolCommand,
        data_size: u64,
    ) -> ProtocolResult<()> {
        self.write("TUC0".as_bytes()).await?;
        self.write(&[1]).await?;
        self.write(&PAD_3).await?; // padding 1
        self.write(&(cmd_id as u32).to_le_bytes()).await?;
        self.write(&data_size.to_le_bytes()).await?;
        self.write(&PAD_C).await?; // padding 2

        Ok(())
    }

    async fn send_file(
        &self,
        padded: bool,
        sender: &Sender<UsbOperation>,
        cancellable: &gio::Cancellable,
    ) -> ProtocolResult<()> {
        let cmd = if padded {
            ProtocolCommand::FileRangePadded
        } else {
            ProtocolCommand::FileRange
        };

        let header = self.recieve_file().await?;
        info!("Requested file: {}", header.name);

        self.send_file_response_header(cmd, header.range_size)
            .await?;

        let file = gio::File::for_path(&header.name);
        let stream = file.read(None::<&gio::Cancellable>)?;

        stream.seek(
            header.range_offset as i64,
            glib::SeekType::Set,
            None::<&gio::Cancellable>,
        )?;

        let mut current_offset: u64 = 0x0;
        let mut read_size = BUFFER_SEGMENT_DATA_SIZE;
        let mut buffer = vec![0u8; (BUFFER_SEGMENT_DATA_SIZE + PADDING_SIZE) as usize];

        let data_start = if padded { PADDING_SIZE as usize } else { 0 };

        let name: Arc<str> = header.name.into();
        while current_offset < header.range_size && !cancellable.is_cancelled() {
            if current_offset + read_size >= header.range_size {
                read_size = header.range_size - current_offset;
            }

            // let slice = &mut buffer[data_start..data_start + read_size as usize];
            let bytes = stream
                .read_bytes_future(read_size as usize, glib::Priority::DEFAULT)
                .await?;
            let slice = bytes.as_ref();

            buffer[data_start..data_start + slice.len()].copy_from_slice(slice);

            self.write(&buffer[..data_start + read_size as usize])
                .await?;
            sender
                .send(UsbOperation::File(name.clone(), read_size))
                .await;
            current_offset += read_size;
        }

        Ok(())
    }
}

impl Drop for SwitchProtocol {
    fn drop(&mut self) {
        if let Some(e) = &self.daemon_sender {
            let (sender, reciever) = async_channel::bounded(1);
            if let Err(e) = e.send_blocking(UsbCommand::Exit(sender)) {
                log::error!("usb daemon couldn't exit: {:?}", e);
                return;
            }

            let _ = reciever.recv_blocking().expect("failed to exit the daemon");
        }
    }
}
