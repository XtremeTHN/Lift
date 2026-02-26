use log::{info, warn};
use rusb::{ConfigDescriptor, Context, Device, DeviceHandle};
use rusb::Error;
use std::string::FromUtf8Error;
use std::time::Duration;

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),
    #[error("Error from ctx")]
    CtxError(#[from] Error),
    #[error("Invalid magic")]
    DecodingError(#[from] FromUtf8Error),
    #[error("Invalid magic: {0}")]
    InvalidMagic(String)
}

enum ProtocolCommand {
    Exit,
    FileRange,
    FileRangePadded,
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


pub struct SwitchProtocol {
    pub ctx: Context,
    pub switch: Option<Device<Context>>,
    handle: Option<DeviceHandle<Context>>,

    interface_num: Option<u8>,
    in_endpoint: Option<u8>,
    out_endpoint: Option<u8>
}

impl SwitchProtocol {
    pub fn new() -> Result<SwitchProtocol, Error> {
        let ctx = Context::new ()?;
        
        Ok(Self {
            ctx,
            switch: None,
            handle: None,
            interface_num: None,
            in_endpoint: None,
            out_endpoint: None
        })
    }

    fn find_endpoints(&mut self, conf_desc: ConfigDescriptor) -> Result<(), ProtocolError>{
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
            return Err(
                ProtocolError::EndpointNotFound(String::from("IN"))
            );
        }
        if self.out_endpoint.is_none() {
            return Err(
                ProtocolError::EndpointNotFound(String::from("OUT"))
            );
        }

        Ok(())
    }

    fn write(&self, buf: &[u8]) -> Result<(), ProtocolError> {
        let handle = self.handle.as_ref().unwrap();
        handle.write_bulk(self.out_endpoint.unwrap(), buf, Duration::from_secs(1))?;

        Ok(())
    }

    fn read(&self, buf: &mut [u8]) -> Result<(), ProtocolError> {
        self.read_with_timeout(buf, Duration::from_secs(1))
    }

    fn read_with_timeout(&self, buf: &mut [u8], timeout: Duration) -> Result<(), ProtocolError> {
        let handle = self.handle.as_ref().unwrap();
        handle.read_bulk(self.in_endpoint.unwrap(), buf, timeout)?;

        Ok(())
    }

    pub fn set_switch(&mut self, dev: Device<Context>) -> Result<(), ProtocolError>{
        self.find_endpoints(dev.active_config_descriptor()?)?;

        let handle = dev.open()?;
        handle.claim_interface(self.interface_num.unwrap())?;

        self.handle = Some(handle);
        self.switch = Some(dev);

        Ok(())
    }

    fn send_list_header(&self, length: u32) {
        info!("Sending rom list with length of {}", length);
        self.write("TUL0".as_bytes()).expect("magic");
        self.write(&length.to_le_bytes()).expect("length");
        self.write(&vec![0u8; 0x8]).expect("padding"); // padding
    }

    pub fn send_roms(&self, roms: Vec<String>) {
        let mut new_vec: Vec<String> = Vec::new();
        let mut length = 0;

        for file in roms {
            new_vec.push(file.clone() + "\n");
            length += file.len() + 1;
        }

        self.send_list_header(length.try_into().unwrap());

        for file in new_vec {
            self.write(file.as_bytes()).expect(file.as_str());
        }
    }

    pub fn poll_commands(&self) -> Result<(), ProtocolError> {
        loop {
            let mut header = vec![0u8; 0x20];
            self.read_with_timeout(&mut header, Duration::from_secs(0))?;

            let magic = String::from_utf8(header[0..4].to_vec())?;
            if magic != "TUC0" {
                return Err(ProtocolError::InvalidMagic(magic));
            }

            info!("Magic: {}", magic);

            let raw_cmd = u32::from_le_bytes(header[8..12].try_into().unwrap());
            let cmd = ProtocolCommand::try_from(raw_cmd);
            if let Err(_) = cmd {
                warn!("Invalid command: {}", raw_cmd);
                continue;
            }

            match cmd.unwrap() {
                ProtocolCommand::Exit => {
                    info!("Exit recieved");
                    break;
                },
                ProtocolCommand::FileRange => {
                    todo!();
                },
                ProtocolCommand::FileRangePadded => {
                    todo!();
                }
            }
        }

        Ok(())
    }
}