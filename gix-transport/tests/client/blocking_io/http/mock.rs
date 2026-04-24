use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr},
    time::Duration,
};

use bstr::ByteVec;
use gix_transport::{
    client::{blocking_io::http, TransportWithoutIO},
    Protocol,
};

use crate::fixture_bytes;

#[cfg(feature = "http-client-curl")]
type Remote = http::curl::Curl;
#[cfg(all(feature = "http-client-reqwest", not(feature = "http-client-curl")))]
type Remote = http::reqwest::Remote;

enum Command {
    ReadAndRespond(Vec<u8>),
}

enum CommandResult {
    ReadAndRespond(Vec<u8>),
}

pub struct Server {
    pub addr: SocketAddr,
    send_command: std::sync::mpsc::SyncSender<Command>,
    recv_result: std::sync::mpsc::Receiver<CommandResult>,
}

impl Server {
    fn normalize_http_response(mut response: Vec<u8>) -> Vec<u8> {
        let split = response
            .windows(2)
            .position(|window| window == b"\n\n")
            .or_else(|| response.windows(4).position(|window| window == b"\r\n\r\n"))
            .expect("HTTP fixture with header/body separator");
        let separator_len = if response[split..].starts_with(b"\r\n\r\n") {
            4
        } else {
            2
        };
        let body = response.split_off(split + 2);
        let header = String::from_utf8(response[..split].to_vec()).expect("HTTP header fixtures are valid UTF-8");
        let mut body = body[(separator_len - 2)..].to_vec();

        let has_packetline_content = header.lines().any(|line| {
            line.to_ascii_lowercase()
                .starts_with("content-type: application/x-git-")
        });
        if has_packetline_content && body.ends_with(b"0000\n") {
            body.pop();
        }

        let mut out = Vec::with_capacity(header.len() + body.len() + 32);
        for line in header.lines() {
            if line.to_ascii_lowercase().starts_with("content-length:") {
                out.extend_from_slice(format!("Content-Length: {}", body.len()).as_bytes());
            } else {
                out.extend_from_slice(line.as_bytes());
            }
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&body);
        out
    }

    pub fn new(fixture: Vec<u8>) -> Self {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .expect("an ephemeral local port to be free");
        let addr = listener.local_addr().expect("a local address");
        let (send_result, recv_result) = std::sync::mpsc::sync_channel(0);
        let (send_command, recv_commands) = std::sync::mpsc::sync_channel(0);
        std::thread::spawn(move || {
            for command in recv_commands {
                match command {
                    Command::ReadAndRespond(response) => {
                        let (mut stream, _) = listener.accept().expect("accept to always work");
                        stream
                            .set_read_timeout(Some(Duration::from_millis(50)))
                            .expect("timeout to always work");
                        stream
                            .set_write_timeout(Some(Duration::from_millis(50)))
                            .expect("timeout to always work");
                        let mut out = Vec::new();
                        stream.read_to_end(&mut out).ok();
                        stream
                            .write_all(&Self::normalize_http_response(response))
                            .expect("write to always work");
                        stream.flush().expect("flush to work");
                        stream.shutdown(Shutdown::Write).ok();
                        if send_result.send(CommandResult::ReadAndRespond(out)).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        send_command
            .send(Command::ReadAndRespond(fixture))
            .expect("send to go through when thread is up");
        Server {
            addr,
            send_command,
            recv_result,
        }
    }

    pub fn next_read_and_respond_with(&self, fixture: Vec<u8>) {
        self.send_command
            .send(Command::ReadAndRespond(Self::normalize_http_response(fixture)))
            .expect("thread to be waiting");
    }

    pub fn received(&self) -> Vec<u8> {
        match self.recv_result.recv().expect("thread to be up") {
            CommandResult::ReadAndRespond(received) => received,
        }
    }

    pub fn received_as_string(&self) -> String {
        self.received().into_string().expect("utf8 only")
    }
}

pub fn serve_once(name: &str) -> Server {
    Server::new(fixture_bytes(name))
}

pub fn serve_and_connect(
    name: &str,
    path: &str,
    version: Protocol,
) -> Result<(Server, http::Transport<Remote>), crate::Error> {
    let server = serve_once(name);
    let url_str = format!(
        "http://{}:{}/{}",
        &server.addr.ip().to_string(),
        &server.addr.port(),
        path
    );
    let client =
        gix_transport::client::blocking_io::http::connect::<Remote>(url_str.as_str().try_into()?, version, false);
    assert_eq!(url_str, client.to_url().as_ref());
    Ok((server, client))
}
