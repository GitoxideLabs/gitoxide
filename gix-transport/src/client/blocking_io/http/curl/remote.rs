use std::{
    io,
    io::{Read, Write},
    sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
    thread,
    time::Duration,
};

use curl::easy::{Auth, Easy2};
use gix_features::io::pipe;

use crate::client::{
    blocking_io::http::{self, curl::Error, redirect},
    http::{
        curl::curl_is_spurious,
        options::{FollowRedirects, HttpVersion, ProxyAuthMethod, SslVersion},
        traits::PostBodyDataKind,
    },
};

enum StreamOrBuffer {
    Stream(pipe::Reader),
    Buffer(std::io::Cursor<Vec<u8>>),
}

#[derive(Default)]
struct Handler {
    send_header: Option<pipe::Writer>,
    send_data: Option<pipe::Writer>,
    receive_body: Option<StreamOrBuffer>,
    checked_status: bool,
    last_status: usize,
    follow: FollowRedirects,
}

impl Handler {
    fn reset(&mut self) {
        self.checked_status = false;
        self.last_status = 0;
        self.follow = FollowRedirects::default();
    }
    fn parse_status_inner(data: &[u8]) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let code = data
            .split(|b| *b == b' ')
            .nth(1)
            .ok_or("Expected HTTP/<VERSION> STATUS")?;
        let code = std::str::from_utf8(code)?;
        code.parse().map_err(Into::into)
    }
    fn parse_status(data: &[u8], follow: FollowRedirects) -> Option<(usize, Box<dyn std::error::Error + Send + Sync>)> {
        let valid_end = match follow {
            FollowRedirects::Initial | FollowRedirects::All => 308,
            FollowRedirects::None => 299,
        };
        match Self::parse_status_inner(data) {
            Ok(status) if !(200..=valid_end).contains(&status) => {
                Some((status, format!("Received HTTP status {status}").into()))
            }
            Ok(_) => None,
            Err(err) => Some((500, err)),
        }
    }
}

impl curl::easy::Handler for Handler {
    fn write(&mut self, data: &[u8]) -> Result<usize, curl::easy::WriteError> {
        drop(self.send_header.take()); // signal header readers to stop trying
        match self.send_data.as_mut() {
            Some(writer) => writer.write_all(data).map(|_| data.len()).or(Ok(0)),
            None => Ok(0), // nothing more to receive, reader is done
        }
    }
    fn read(&mut self, data: &mut [u8]) -> Result<usize, curl::easy::ReadError> {
        match self.receive_body.as_mut() {
            Some(StreamOrBuffer::Stream(reader)) => reader.read(data).map_err(|_err| curl::easy::ReadError::Abort),
            Some(StreamOrBuffer::Buffer(cursor)) => cursor.read(data).map_err(|_err| curl::easy::ReadError::Abort),
            None => Ok(0), // nothing more to read/writer depleted
        }
    }

    fn header(&mut self, data: &[u8]) -> bool {
        if let Some(writer) = self.send_header.as_mut() {
            if self.checked_status {
                writer.write_all(data).ok();
            } else {
                self.checked_status = true;
                self.last_status = 200;
                if let Some((status, err)) = Handler::parse_status(data, self.follow) {
                    self.last_status = status;
                    writer
                        .channel
                        .send(Err(io::Error::new(
                            if status == 401 {
                                io::ErrorKind::PermissionDenied
                            } else if (500..600).contains(&status) {
                                io::ErrorKind::ConnectionAborted
                            } else {
                                io::ErrorKind::Other
                            },
                            err,
                        )))
                        .ok();
                }
            }
        }
        true
    }
}

pub struct Request {
    pub url: String,
    pub base_url: String,
    pub headers: curl::easy::List,
    pub upload_body_kind: Option<PostBodyDataKind>,
    pub config: http::Options,
}

pub struct Response {
    pub headers: pipe::Reader,
    pub body: pipe::Reader,
    pub upload_body: pipe::Writer,
}

pub fn new() -> (
    thread::JoinHandle<Result<(), Error>>,
    SyncSender<Request>,
    Receiver<Response>,
) {
    let (req_send, req_recv) = sync_channel(0);
    let (res_send, res_recv) = sync_channel(0);
    let handle = std::thread::spawn(move || -> Result<(), Error> {
        let mut handle = Easy2::new(Handler::default());
        // We don't wait for the possibility for pipelining to become clear, and curl tries to reuse connections by default anyway.
        handle.pipewait(false)?;
        handle.tcp_keepalive(true)?;

        let mut follow = None;
        let mut redirected_base_url = None::<String>;

        for Request {
            url,
            base_url,
            mut headers,
            upload_body_kind,
            config:
                http::Options {
                    extra_headers,
                    follow_redirects,
                    low_speed_limit_bytes_per_second,
                    low_speed_time_seconds,
                    connect_timeout,
                    proxy,
                    no_proxy,
                    proxy_auth_method,
                    user_agent,
                    proxy_authenticate,
                    verbose,
                    ssl_ca_info,
                    ssl_version,
                    ssl_verify,
                    http_version,
                    backend,
                },
        } in req_recv
        {
            let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
            handle.url(&effective_url)?;

            handle.post(upload_body_kind.is_some())?;
            for header in extra_headers {
                headers.append(&header)?;
            }
            // needed to avoid sending Expect: 100-continue, which adds another response and only CURL wants that
            headers.append("Expect:")?;
            handle.verbose(verbose)?;

            if let Some(ca_info) = ssl_ca_info {
                handle.cainfo(ca_info)?;
            }

            if let Some(ref mut curl_options) = backend.as_ref().and_then(|backend| backend.lock().ok()) {
                if let Some(opts) = curl_options.downcast_mut::<super::Options>() {
                    if let Some(enabled) = opts.schannel_check_revoke {
                        handle.ssl_options(curl::easy::SslOpt::new().no_revoke(!enabled))?;
                    }
                }
            }

            if let Some(ssl_version) = ssl_version {
                let (min, max) = ssl_version.min_max();
                if min == max {
                    handle.ssl_version(to_curl_ssl_version(min))?;
                } else {
                    handle.ssl_min_max_version(to_curl_ssl_version(min), to_curl_ssl_version(max))?;
                }
            }

            handle.ssl_verify_peer(ssl_verify)?;
            handle.ssl_verify_host(ssl_verify)?;

            if let Some(http_version) = http_version {
                let version = match http_version {
                    HttpVersion::V1_1 => curl::easy::HttpVersion::V11,
                    HttpVersion::V2 => curl::easy::HttpVersion::V2,
                };
                // Failing to set the version isn't critical, and may indeed fail depending on the version
                // of libcurl we are built against.
                // Furthermore, `git` itself doesn't actually check for errors when configuring curl at all,
                // treating all or most flags as non-critical.
                handle.http_version(version).ok();
            }

            let mut proxy_auth_action = None;
            if let Some(proxy) = proxy {
                handle.proxy(&proxy)?;
                let proxy_type = if proxy.starts_with("socks5h") {
                    curl::easy::ProxyType::Socks5Hostname
                } else if proxy.starts_with("socks5") {
                    curl::easy::ProxyType::Socks5
                } else if proxy.starts_with("socks4a") {
                    curl::easy::ProxyType::Socks4a
                } else if proxy.starts_with("socks") {
                    curl::easy::ProxyType::Socks4
                } else {
                    curl::easy::ProxyType::Http
                };
                handle.proxy_type(proxy_type)?;

                if let Some((obtain_creds_action, authenticate)) = proxy_authenticate {
                    let creds = authenticate.lock().expect("no panics in other threads")(obtain_creds_action)?
                        .expect("action to fetch credentials");
                    handle.proxy_username(&creds.identity.username)?;
                    handle.proxy_password(&creds.identity.password)?;
                    proxy_auth_action = Some((creds.next, authenticate));
                }
            }
            if let Some(no_proxy) = no_proxy {
                handle.noproxy(&no_proxy)?;
            }
            if let Some(user_agent) = user_agent {
                handle.useragent(&user_agent)?;
            }
            handle.transfer_encoding(false)?;
            if let Some(timeout) = connect_timeout {
                handle.connect_timeout(timeout)?;
            }
            {
                let mut auth = Auth::new();
                match proxy_auth_method {
                    ProxyAuthMethod::AnyAuth => auth
                        .basic(true)
                        .digest(true)
                        .digest_ie(true)
                        .gssnegotiate(true)
                        .ntlm(true)
                        .aws_sigv4(true),
                    ProxyAuthMethod::Basic => auth.basic(true),
                    ProxyAuthMethod::Digest => auth.digest(true),
                    ProxyAuthMethod::Negotiate => auth.digest_ie(true),
                    ProxyAuthMethod::Ntlm => auth.ntlm(true),
                };
                handle.proxy_auth(&auth)?;
            }
            handle.tcp_keepalive(true)?;

            if low_speed_time_seconds > 0 && low_speed_limit_bytes_per_second > 0 {
                handle.low_speed_limit(low_speed_limit_bytes_per_second)?;
                handle.low_speed_time(Duration::from_secs(low_speed_time_seconds))?;
            }
            let (receive_data, receive_headers, send_body, mut receive_body) = {
                let handler = handle.get_mut();
                let (send, receive_data) = pipe::unidirectional(1);
                handler.send_data = Some(send);
                let (send, receive_headers) = pipe::unidirectional(1);
                handler.send_header = Some(send);
                let (send_body, receive_body) = pipe::unidirectional(0);
                (receive_data, receive_headers, send_body, receive_body)
            };

            let follow = follow.get_or_insert(follow_redirects);
            handle.get_mut().follow = *follow;
            handle.follow_location(matches!(*follow, FollowRedirects::Initial | FollowRedirects::All))?;

            if *follow == FollowRedirects::Initial {
                *follow = FollowRedirects::None;
            }

            if res_send
                .send(Response {
                    headers: receive_headers,
                    body: receive_data,
                    upload_body: send_body,
                })
                .is_err()
            {
                break;
            }

            handle.get_mut().receive_body = Some(match upload_body_kind {
                Some(PostBodyDataKind::Unbounded) | None => StreamOrBuffer::Stream(receive_body),
                Some(PostBodyDataKind::BoundedAndFitsIntoMemory) => {
                    let mut buf = Vec::<u8>::with_capacity(512);
                    receive_body.read_to_end(&mut buf)?;
                    handle.post_field_size(buf.len() as u64)?;
                    drop(receive_body);
                    StreamOrBuffer::Buffer(std::io::Cursor::new(buf))
                }
            });
            handle.http_headers(headers)?;

            if let Err(err) = handle.perform() {
                let handler = handle.get_mut();
                handler.reset();

                if let Some((action, authenticate)) = proxy_auth_action {
                    authenticate.lock().expect("no panics in other threads")(action.erase()).ok();
                }
                let err = Err(io::Error::new(
                    if curl_is_spurious(&err) {
                        std::io::ErrorKind::ConnectionReset
                    } else {
                        std::io::ErrorKind::Other
                    },
                    err,
                ));
                handler.receive_body.take();
                match (handler.send_header.take(), handler.send_data.take()) {
                    (Some(header), mut data) => {
                        if let Err(TrySendError::Disconnected(err) | TrySendError::Full(err)) =
                            header.channel.try_send(err)
                        {
                            if let Some(body) = data.take() {
                                body.channel.try_send(err).ok();
                            }
                        }
                    }
                    (None, Some(body)) => {
                        body.channel.try_send(err).ok();
                    }
                    (None, None) => {}
                }
            } else {
                let handler = handle.get_mut();
                if let Some((action, authenticate)) = proxy_auth_action {
                    authenticate.lock().expect("no panics in other threads")(if handler.last_status == 200 {
                        action.store()
                    } else {
                        action.erase()
                    })?;
                }
                handler.reset();
                handler.receive_body.take();
                handler.send_header.take();
                handler.send_data.take();
                let actual_url = handle
                    .effective_url()?
                    .expect("effective url is present and valid UTF-8");
                if actual_url != effective_url {
                    redirected_base_url = redirect::base_url(actual_url, &base_url, url)?.into();
                }
            }
        }
        Ok(())
    });
    (handle, req_send, res_recv)
}

fn to_curl_ssl_version(vers: SslVersion) -> curl::easy::SslVersion {
    use curl::easy::SslVersion::*;
    match vers {
        SslVersion::Default => Default,
        SslVersion::TlsV1 => Tlsv1,
        SslVersion::SslV2 => Sslv2,
        SslVersion::SslV3 => Sslv3,
        SslVersion::TlsV1_0 => Tlsv10,
        SslVersion::TlsV1_1 => Tlsv11,
        SslVersion::TlsV1_2 => Tlsv12,
        SslVersion::TlsV1_3 => Tlsv13,
    }
}

impl From<Error> for http::Error {
    fn from(err: Error) -> Self {
        http::Error::Detail {
            description: err.to_string(),
        }
    }
}

impl From<curl::Error> for http::Error {
    fn from(err: curl::Error) -> Self {
        http::Error::Detail {
            description: err.to_string(),
        }
    }
}
