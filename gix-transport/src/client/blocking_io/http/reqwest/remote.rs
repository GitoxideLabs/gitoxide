use std::{
    any::Any,
    io::{Read, Write},
    str::FromStr,
    sync::{atomic, Arc},
};

use gix_features::io::pipe;

use crate::client::blocking_io::http::{
    self, options::FollowRedirects, redirect, reqwest::Remote, traits::PostBodyDataKind,
};

/// The error returned by the 'remote' helper, a purely internal construct to perform http requests.
pub type Error = crate::Error;

impl Default for Remote {
    fn default() -> Self {
        let (req_send, req_recv) = std::sync::mpsc::sync_channel(0);
        let (res_send, res_recv) = std::sync::mpsc::sync_channel(0);
        let handle = std::thread::spawn(move || -> Result<(), Error> {
            let mut follow = None;
            let mut redirected_base_url = None::<String>;
            let allow_redirects = Arc::new(atomic::AtomicBool::new(false));

            // We may error while configuring, which is expected as part of the internal protocol. The error will be
            // received and the sender of the request might restart us.
            let client = reqwest::blocking::ClientBuilder::new()
                .connect_timeout(std::time::Duration::from_secs(20))
                .http1_title_case_headers()
                .redirect(reqwest::redirect::Policy::custom({
                    let allow_redirects = allow_redirects.clone();
                    move |attempt| {
                        if allow_redirects.load(atomic::Ordering::Relaxed) {
                            let curr_url = attempt.url();
                            let prev_urls = attempt.previous();

                            match prev_urls.first() {
                                Some(prev_url) if prev_url.host_str() != curr_url.host_str() => {
                                    // git does not want to be redirected to a different host.
                                    attempt.stop()
                                }
                                _ => {
                                    // emulate default git behaviour which relies on curl default behaviour apparently.
                                    const CURL_DEFAULT_REDIRS: usize = 50;
                                    if prev_urls.len() >= CURL_DEFAULT_REDIRS {
                                        attempt.error("too many redirects")
                                    } else {
                                        attempt.follow()
                                    }
                                }
                            }
                        } else {
                            attempt.stop()
                        }
                    }
                }))
                .build()?;

            for Request {
                url,
                base_url,
                headers,
                upload_body_kind,
                config,
            } in req_recv
            {
                let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
                let mut req_builder = if upload_body_kind.is_some() {
                    client.post(&effective_url)
                } else {
                    client.get(&effective_url)
                }
                .headers(headers);
                let (post_body_tx, mut post_body_rx) = pipe::unidirectional(0);
                let (mut response_body_tx, response_body_rx) = pipe::unidirectional(0);
                let (mut headers_tx, headers_rx) = pipe::unidirectional(0);
                if res_send
                    .send(Response {
                        headers: headers_rx,
                        body: response_body_rx,
                        upload_body: post_body_tx,
                    })
                    .is_err()
                {
                    // This means our internal protocol is violated as the one who sent the request isn't listening anymore.
                    // Shut down as something is off.
                    break;
                }
                req_builder = match upload_body_kind {
                    Some(PostBodyDataKind::BoundedAndFitsIntoMemory) => {
                        let mut buf = Vec::<u8>::with_capacity(512);
                        post_body_rx.read_to_end(&mut buf)?;
                        req_builder.body(buf)
                    }
                    Some(PostBodyDataKind::Unbounded) => req_builder.body(reqwest::blocking::Body::new(post_body_rx)),
                    None => req_builder,
                };
                let mut req = req_builder.build()?;
                if let Some(ref mut request_options) = config.backend.as_ref().and_then(|backend| backend.lock().ok()) {
                    if let Some(options) = request_options.downcast_mut::<super::Options>() {
                        if let Some(configure_request) = &mut options.configure_request {
                            configure_request(&mut req).map_err(|e| Error::ConfigureRequest(e.to_string()))?;
                        }
                    }
                }

                let follow = follow.get_or_insert(config.follow_redirects);
                allow_redirects.store(
                    matches!(follow, FollowRedirects::Initial | FollowRedirects::All),
                    atomic::Ordering::Relaxed,
                );

                if *follow == FollowRedirects::Initial {
                    *follow = FollowRedirects::None;
                }

                let mut res = match client
                    .execute(req)
                    .and_then(reqwest::blocking::Response::error_for_status)
                {
                    Ok(res) => res,
                    Err(err) => {
                        let (kind, err) = match err.status() {
                            Some(status) => {
                                let kind = if status == reqwest::StatusCode::UNAUTHORIZED {
                                    std::io::ErrorKind::PermissionDenied
                                } else if status.is_server_error() {
                                    std::io::ErrorKind::ConnectionAborted
                                } else {
                                    std::io::ErrorKind::Other
                                };
                                (kind, format!("Received HTTP status {}", status.as_str()))
                            }
                            None => (std::io::ErrorKind::Other, err.to_string()),
                        };
                        let err = Err(std::io::Error::new(kind, err));
                        headers_tx.channel.send(err).ok();
                        continue;
                    }
                };

                let actual_url = res.url().as_str();
                if actual_url != effective_url.as_str() {
                    redirected_base_url = redirect::base_url(actual_url, &base_url, url)?.into();
                }

                let send_headers = {
                    let headers = res.headers();
                    move || -> std::io::Result<()> {
                        for (name, value) in headers {
                            headers_tx.write_all(name.as_str().as_bytes())?;
                            headers_tx.write_all(b":")?;
                            headers_tx.write_all(value.as_bytes())?;
                            headers_tx.write_all(b"\n")?;
                        }
                        // Make sure this is an FnOnce closure to signal the remote reader we are done.
                        drop(headers_tx);
                        Ok(())
                    }
                };

                // We don't have to care if anybody is receiving the header, as a matter of fact we cannot fail sending them.
                // Thus an error means the receiver failed somehow, but might also have decided not to read headers at all. Fine with us.
                send_headers().ok();

                // reading the response body is streaming and may fail for many reasons. If so, we send the error over the response
                // body channel and that's all we can do.
                if let Err(err) = std::io::copy(&mut res, &mut response_body_tx) {
                    response_body_tx.channel.send(Err(err)).ok();
                }
            }
            Ok(())
        });

        Remote {
            handle: Some(handle),
            request: req_send,
            response: res_recv,
            config: http::Options::default(),
        }
    }
}

/// utilities
impl Remote {
    fn restore_thread_after_failure(&mut self) -> http::Error {
        let err_that_brought_thread_down = self
            .handle
            .take()
            .expect("thread handle present")
            .join()
            .expect("handler thread should never panic")
            .expect_err("something should have gone wrong with curl (we join on error only)");
        *self = Remote::default();
        http::Error::InitHttpClient(err_that_brought_thread_down.to_string())
    }

    fn make_request(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        upload_body_kind: Option<PostBodyDataKind>,
    ) -> Result<http::PostResponse<pipe::Reader, pipe::Reader, pipe::Writer>, http::Error> {
        let mut header_map = reqwest::header::HeaderMap::new();
        for header_line in headers {
            let header_line = header_line.as_ref();
            let colon_pos = header_line
                .find(':')
                .expect("header line must contain a colon to separate key and value");
            let header_name = &header_line[..colon_pos];
            let value = &header_line[colon_pos + 1..];

            match reqwest::header::HeaderName::from_str(header_name)
                .ok()
                .zip(reqwest::header::HeaderValue::try_from(value.trim()).ok())
            {
                Some((key, val)) => header_map.insert(key, val),
                None => continue,
            };
        }
        if self
            .request
            .send(Request {
                url: url.to_owned(),
                base_url: base_url.to_owned(),
                headers: header_map,
                upload_body_kind,
                config: self.config.clone(),
            })
            .is_err()
        {
            return Err(self.restore_thread_after_failure());
        }

        let Response {
            headers,
            body,
            upload_body,
        } = match self.response.recv() {
            Ok(res) => res,
            Err(_) => {
                return Err(self.restore_thread_after_failure());
            }
        };

        Ok(http::PostResponse {
            post_body: upload_body,
            headers,
            body,
        })
    }
}

impl http::Http for Remote {
    type Headers = pipe::Reader;
    type ResponseBody = pipe::Reader;
    type PostBody = pipe::Writer;

    fn get(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<http::GetResponse<Self::Headers, Self::ResponseBody>, http::Error> {
        self.make_request(url, base_url, headers, None).map(Into::into)
    }

    fn post(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        post_body_kind: PostBodyDataKind,
    ) -> Result<http::PostResponse<Self::Headers, Self::ResponseBody, Self::PostBody>, http::Error> {
        self.make_request(url, base_url, headers, Some(post_body_kind))
    }

    fn configure(&mut self, config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if let Some(config) = config.downcast_ref::<http::Options>() {
            self.config = config.clone();
        }
        Ok(())
    }
}

pub(crate) struct Request {
    pub url: String,
    pub base_url: String,
    pub headers: reqwest::header::HeaderMap,
    pub upload_body_kind: Option<PostBodyDataKind>,
    pub config: http::Options,
}

/// A link to a thread who provides data for the contained readers.
/// The expected order is:
/// - write `upload_body`
/// - read `headers` to end
/// - read `body` to hend
pub(crate) struct Response {
    pub headers: pipe::Reader,
    pub body: pipe::Reader,
    pub upload_body: pipe::Writer,
}
