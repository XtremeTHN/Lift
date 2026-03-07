use log::{info, warn};
use rusb::Error;
use rusb::{ConfigDescriptor, Context, Device, DeviceHandle, UsbContext};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::string::FromUtf8Error;
use std::time::Duration;
use binrw::BinRead;

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),
    #[error("Error from ctx")]
    CtxError(#[from] Error),
    #[error("Invalid magic")]
    DecodingError(#[from] FromUtf8Error),
    #[error("Invalid magic: {0}")]
    InvalidMagic(String),
    #[error("Switch not found")]
    SwitchNotFound(),
    #[error("Error while recieving file header")]
    ReadError(#[from] binrw::Error)
}

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

/// Protocol for transfering nintendo switch roms (.nsp, .xci) through usb
/// Only tested for AwooInstaller
///
/// Before polling commands, you must send the roms that will be transfered
///
/// ```
/// let mut protocol = SwitchProtocol::new();
/// protocol.find_switch();
/// protocol.send_roms(vec!["./ori.xci"]);
/// protocol.poll_commands();
/// ```
pub struct SwitchProtocol {
    pub ctx: Context,
    handle: Option<DeviceHandle<Context>>,

    interface_num: Option<u8>,
    in_endpoint: Option<u8>,
    out_endpoint: Option<u8>,
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

type ProtocolResult = Result<(), ProtocolError>;

const BUFFER_SEGMENT_DATA_SIZE: u64 = 0x100000;
const PADDING_SIZE: u64 = 0x1000;

const PAD_3: [u8; 3] = [0; 3];
const PAD_8: [u8; 8] = [0; 8];
const PAD_C: [u8; 0xC] = [0; 0xC];
const TUL0: &[u8; 4] = b"TUL0";

const SWITCH_VENDOR_ID: u16 = 0x057E;
const SWITCH_PRODUCT_ID: u16 = 0x3000;

const FILE_HEADER_SIZE: usize = 0x20;

impl SwitchProtocol {
    pub fn new() -> Result<SwitchProtocol, Error> {
        let ctx = Context::new()?;

        Ok(Self {
            ctx,
            handle: None,
            interface_num: None,
            in_endpoint: None,
            out_endpoint: None,
        })
    }

    fn find_endpoints(&mut self, conf_desc: ConfigDescriptor) -> ProtocolResult {
        for interface in conf_desc.interfaces() {
            for altsetting in interface.descriptors() {
                for endpoint in altsetting.endpoint_descriptors() {
                    let address = endpoint.address();
                    if address == 0x81 {
                        info!("Found in endpoint: {}", address);
                        self.in_endpoint = Some(address);
                        self.interface_num = Some(altsetting.interface_number());
                    } else {
                        info!("Found out endpoint: {}", address);
                        self.out_endpoint = Some(address);
                        self.interface_num = Some(altsetting.interface_number());
                    }
                }
            }
        }

        if self.in_endpoint.is_none() {
            return Err(ProtocolError::EndpointNotFound(String::from("IN")));
        }
        if self.out_endpoint.is_none() {
            return Err(ProtocolError::EndpointNotFound(String::from("OUT")));
        }

        Ok(())
    }

    fn write(&self, buf: &[u8]) -> ProtocolResult {
        let handle = self.handle.as_ref().unwrap();
        handle.write_bulk(self.out_endpoint.unwrap(), buf, Duration::from_secs(1))?;

        Ok(())
    }

    fn read(&self, buf: &mut [u8]) -> ProtocolResult {
        self.read_with_timeout(buf, Duration::from_secs(1))
    }

    fn read_with_timeout(&self, buf: &mut [u8], timeout: Duration) -> ProtocolResult {
        let handle = self.handle.as_ref().unwrap();
        handle.read_bulk(self.in_endpoint.unwrap(), buf, timeout)?;

        Ok(())
    }

    fn open_from_fd(&self, fd: i32) -> Result<DeviceHandle<Context>, ProtocolError> {
        unsafe {
            let r = self.ctx.open_device_with_fd(fd)?;
            Ok(r)
        }
    }

    pub fn open_switch_from_fd(&mut self, fd: i32) -> ProtocolResult {
        let handle = self.open_from_fd(fd)?;
        
        let dev = handle.device();
        self.find_endpoints(dev.active_config_descriptor()?)?;

        self.handle = Some(handle);

        Ok(())
    }

    /// Finds the usb device where the switch is connected and sets the switch and handle fields of Self
    pub fn find_switch(&mut self) -> ProtocolResult {
        let devs = self.ctx.devices()?;

        let mut switch: Option<Device<Context>> = None;
        for dev in devs.iter() {
            let descriptor = dev.device_descriptor().unwrap();

            if descriptor.vendor_id() == SWITCH_VENDOR_ID && descriptor.product_id() == SWITCH_PRODUCT_ID {
                info!("Found switch on bus {:03}", dev.bus_number());
                switch = Some(dev);
            }
        }

        if switch.is_none() {
            return Err(ProtocolError::SwitchNotFound());
        }

        let dev = switch.unwrap();
        self.find_endpoints(dev.active_config_descriptor()?)?;

        let handle = dev.open()?;
        handle.claim_interface(self.interface_num.unwrap())?;

        self.handle = Some(handle);

        Ok(())
    }

    fn send_list_header(&self, length: u32) -> ProtocolResult {
        info!("Sending rom list with length of {}", length);
        self.write(TUL0)?;
        self.write(&length.to_le_bytes())?;
        self.write(&PAD_8)?; // padding

        Ok(())
    }

    /// Sends the roms that will be transferred
    /// ```
    /// protocol.send_roms(vec!["ori.xci", "undertale.nsp"]);
    /// ```
    pub fn send_roms(&self, roms: Vec<String>) -> ProtocolResult {
        let mut new_vec: Vec<String> = Vec::new();
        let mut length = 0;

        for file in roms {
            new_vec.push(file.clone() + "\n");
            length += file.len() + 1;
        }

        self.send_list_header(length as u32)?;

        for file in new_vec {
            self.write(file.as_bytes())?;
        }

        Ok(())
    }

    fn recieve_file(&self) -> Result<FileHeader, ProtocolError> {
        let mut header = [0u8; FILE_HEADER_SIZE];
        self.read(&mut header)?;

        let mut cur = Cursor::new(header);
        let mut file_header = FileHeader::read(&mut cur)?;
        
        let mut raw_name = vec![0u8; file_header.rom_name_length as usize];
        self.read(&mut raw_name)?;
        file_header.name = String::from_utf8(raw_name)?;

        Ok(file_header)
    }

    fn send_file_response_header(&self, cmd_id: ProtocolCommand, data_size: u64) -> ProtocolResult {
        self.write("TUC0".as_bytes())?;
        self.write(&[1])?;
        self.write(&PAD_3)?; // padding 1
        self.write(&(cmd_id as u32).to_le_bytes())?;
        self.write(&data_size.to_le_bytes())?;
        self.write(&PAD_C)?; // padding 2

        Ok(())
    }

    fn send_file(&self, padded: bool) -> ProtocolResult {
        let cmd = if padded {
            ProtocolCommand::FileRange
        } else {
            ProtocolCommand::FileRangePadded
        };

        let header = self.recieve_file()?;

        info!("Requested file: {}", header.name);

        self.send_file_response_header(cmd, header.range_size)?;
        let mut file = File::open(header.name.clone()).expect("Couldn't find rom");

        file.seek(SeekFrom::Start(header.range_offset))
            .expect("couldn't seek");

        let mut current_offset: u64 = 0x0;
        let mut read_size = BUFFER_SEGMENT_DATA_SIZE;
        let mut buffer = vec![0u8; (BUFFER_SEGMENT_DATA_SIZE + PADDING_SIZE) as usize];

        while current_offset < header.range_size {
            if current_offset + read_size >= header.range_size {
                read_size = header.range_size - current_offset;
            }

            let data_start = if padded { PADDING_SIZE as usize } else { 0 };
            let slice = &mut buffer[data_start..data_start + read_size as usize];
            file.read_exact(slice).expect("couldn't read");

            self.write(&buffer[..data_start + read_size as usize])?;
            current_offset += read_size;
        }

        Ok(())
    }

    /// Handles the commands sent by the switch
    /// Call find_switch() before using this function
    /// Send the roms before using this function
    pub fn poll_commands(&self) -> ProtocolResult {
        loop {
            let mut header = vec![0u8; 0x20];
            self.read_with_timeout(&mut header, Duration::from_secs(10))?;

            let magic = String::from_utf8(header[0..4].to_vec())?;
            if magic != "TUC0" {
                return Err(ProtocolError::InvalidMagic(magic));
            }

            let raw_cmd = u32::from_le_bytes(header[8..12].try_into().unwrap());
            let cmd = ProtocolCommand::try_from(raw_cmd);
            if cmd.is_err() {
                warn!("Invalid command: {}", raw_cmd);
                continue;
            }

            let unwrapped_cmd = cmd.unwrap();
            match unwrapped_cmd {
                ProtocolCommand::Exit => {
                    info!("Exit recieved");
                    break;
                }
                ProtocolCommand::FileRange => {
                    info!("Recieved FileRange command");
                    self.send_file(false)?;
                }
                ProtocolCommand::FileRangePadded => {
                    info!("Recieved FileRangePadded command");
                    self.send_file(true)?;
                }
            }
        }

        Ok(())
    }
}
