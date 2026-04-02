use std::{io, net::IpAddr, path::PathBuf};

use async_std::{
    channel::Sender,
    fs::File,
    io::{BufReader, ReadExt, SeekExt, WriteExt},
    net::TcpStream,
};
use std::io::SeekFrom;

use super::reader::LoggedReader;
use gtk::gio;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tide::{Request, Response};

// TODO: find a place for this enum
use crate::usb::async_protocol::ProtocolOperation;
use crate::utils::FileVecBuilder;

fn parse_num(num: &str) -> Result<u64, tide::Error> {
    let res: u64 = match num.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(tide::Error::from_str(
                400,
                "Invalid range: Failed to parse value",
            ));
        }
    };

    Ok(res)
}

/*Copyright (c) 2017-2018 Adubbz

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

// ported from https://github.com/nicoboss/OG_Tinfoil/blob/master/tools/remote_install_pc.py
async fn handle_file(
    rq: Request<()>,
    sender: Sender<ProtocolOperation>,
    cancelled: Arc<AtomicBool>,
) -> Result<Response, tide::Error> {
    let path = rq.param("filepath").expect("");

    log::info!("Requested {}", path);
    let pb = PathBuf::from(path);
    let name = pb.to_string_lossy().to_string();

    let mut file = File::open(path).await.expect("");

    let size = file.seek(SeekFrom::End(0)).await.expect("err");
    file.seek(SeekFrom::Start(0)).await.expect("err");

    let (mut start, mut end) = (0, size - 1);
    let head = rq.header("Range");

    if let Some(header) = head {
        // (start, end) = header.trim;
        let header = header.to_string();
        let trimmed = header.replace("[\"bytes=", "").replace("\"]", "");
        let parts = trimmed.split("-").collect::<Vec<&str>>();
        if parts[0] == "" {
            end = parse_num(parts[1])?;
            start = size - end;
        } else {
            start = parse_num(parts[0])?;

            if start >= size {
                return Err(tide::Error::from_str(416, "Invalid range"));
            }

            if parts[1] == "" {
                end = size - 1;
            } else {
                end = parse_num(parts[1])?;
            }
        }
    }

    start = start.max(0);
    end = end.min(size - 1);

    let cnt_len = end - start + 1;
    let status = if head.is_none() { 200 } else { 206 };
    let mut res = tide::Response::new(status);

    res.insert_header("Content-type", "application/octet-stream");
    res.insert_header("Accept-Ranges", "bytes");
    res.insert_header("Content-Range", format!("bytes {}-{}/{}", start, end, size));
    res.insert_header("Content-Length", cnt_len.to_string());

    let time = chrono::Utc::now();
    let modified = time.to_rfc2822();
    res.insert_header("Last-Modified", modified);

    if rq.method().to_string() == "GET" {
        file.seek(std::io::SeekFrom::Start(start)).await?;
        let stream = BufReader::new(LoggedReader::new(
            file.take(cnt_len),
            name,
            sender,
            cancelled,
        ));
        res.set_body(tide::Body::from_reader(stream, Some(cnt_len as usize)));
    }

    Ok(res)
}

pub struct Server {
    switch_sock: Option<TcpStream>,
    host_ip: Option<IpAddr>,
    server_url: Option<String>,
    cancel_flag: Arc<AtomicBool>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            switch_sock: None,
            host_ip: None,
            server_url: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn connect_to_switch(&mut self, switch_ip: &str) -> io::Result<()> {
        let switch_sock = TcpStream::connect(format!("{}:{}", switch_ip, "2000")).await?;
        self.host_ip = Some(switch_sock.local_addr()?.ip());
        self.switch_sock = Some(switch_sock);

        let url = format!("{}:{}", self.host_ip.unwrap(), "8080");
        self.server_url = Some(url.clone());

        Ok(())
    }

    pub async fn serve(&self, sender: Sender<ProtocolOperation>) -> io::Result<()> {
        let mut srv = tide::new();

        let cancelled = Arc::clone(&self.cancel_flag);
        srv.at("/file/*filepath").get(move |r| {
            let send = sender.clone();
            let cancelled = Arc::clone(&cancelled);
            async move {
                let res = handle_file(r, send, cancelled).await;

                res
            }
        });

        srv.listen(self.server_url.as_ref().unwrap()).await?;
        Ok(())
    }

    pub async fn send_roms(&self, roms: Vec<gio::File>) -> io::Result<()> {
        let server_url = self.server_url.as_ref().unwrap();

        let mut switch_sock = self.switch_sock.as_ref().unwrap();

        let payload = FileVecBuilder::new()
            .prefix(&format!("{}/file/", server_url))
            .gfiles(roms)
            .build_net();

        switch_sock.write_all(&payload).await?;

        Ok(())
    }

    pub async fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }
}
