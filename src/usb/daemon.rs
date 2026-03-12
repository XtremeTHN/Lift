use async_channel::{Sender, unbounded};
use gtk4::gio;
use rusb::{Context, DeviceHandle, Result};
use std::time::Duration;

pub enum UsbCommand {
    Write(Vec<u8>, Sender<Result<usize>>, u64),
    Read(usize, Sender<Result<Vec<u8>>>, u64),
    Exit(Sender<Result<()>>),
}

pub fn spawn_daemon(
    handle: DeviceHandle<Context>,
    in_ep: u8,
    out_ep: u8,
    iface: u8,
) -> Sender<UsbCommand> {
    let (sender, reciever) = unbounded();
    gio::spawn_blocking(move || {
        log::info!("spawned daemon");
        while let Some(op) = reciever.recv_blocking().iter().next() {
            match op {
                UsbCommand::Write(buf, send, timeout) => {
                    let res = handle.write_bulk(out_ep, buf, Duration::from_secs(*timeout));
                    send.send_blocking(res)
                        .expect("failed to send write result");
                }
                UsbCommand::Read(size, send, timeout) => {
                    let mut buf = vec![0u8; *size];

                    let res = handle
                        .read_bulk(in_ep, &mut buf, Duration::from_secs(*timeout))
                        .map(|_| buf);
                    send.send_blocking(res).expect("failed to send read result");
                }
                UsbCommand::Exit(sender) => {
                    sender
                        .send_blocking(handle.reset())
                        .expect("failed to send reset result");
                    sender
                        .send_blocking(handle.release_interface(iface))
                        .expect("failed to send release result");
                    break;
                }
            }
        }
        log::info!("daemon exit");
    });

    sender
}
