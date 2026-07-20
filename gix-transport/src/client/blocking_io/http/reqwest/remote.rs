use std::{
    any::Any,
    io::{Read, Write},
    str::FromStr,
    sync::Arc,
};

use gix_features::io::pipe;
use parking_lot::Mutex;

use crate::client::blocking_io::http::{
    self,
    options::FollowRedirects,
    redirect::{self, Action as RedirectAction},
    reqwest::Remote,
    traits::PostBodyDataKind,
};

/// The error returned by the 'remote' helper, a purely internal construct to perform http requests.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Reqwest(reqwest::Error),
    ReadPostBody(std::io::Error),
    ConfigureRequest(Box<dyn std::error::Error + Send + Sync + 'static>),
    Redirect(redirect::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Reqwest(err) => std::fmt::Display::fmt(err, f),
            Error::ReadPostBody(_) => f.write_str("Could not finish reading all data to post to the remote"),
            Error::ConfigureRequest(_) => f.write_str("Request configuration failed"),
            Error::Redirect(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Reqwest(err) => err.source(),
            Error::ReadPostBody(err) => Some(err),
            Error::ConfigureRequest(err) => Some(&**err),
            Error::Redirect(err) => err.source(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::ReadPostBody(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Error::ConfigureRequest(err)
    }
}

impl From<redirect::Error> for Error {
    fn from(err: redirect::Error) -> Self {
        Error::Redirect(err)
    }
}

impl crate::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::Reqwest(err) => {
                err.is_timeout() || err.is_connect() || err.status().is_some_and(|status| status.is_server_error())
            }
            _ => false,
        }
    }
}

fn authority_changed(curr_url: &reqwest::Url, prev_url: &reqwest::Url) -> bool {
    curr_url.scheme() != prev_url.scheme()
        || curr_url.host_str() != prev_url.host_str()
        || curr_url.port_or_known_default() != prev_url.port_or_known_default()
}

impl Default for Remote {
    fn default() -> Self {
        let (req_send, req_recv) = std::sync::mpsc::sync_channel(0);
        let (res_send, res_recv) = std::sync::mpsc::sync_channel(0);
        let redirected_base_url_shared = Arc::new(Mutex::new(None));
        let redirected_base_url_shared_for_field = redirected_base_url_shared.clone();
        let handle = std::thread::spawn(move || -> Result<(), Error> {
            let mut follow = None;
            let redirect_action = Arc::new(Mutex::new(RedirectAction::Stop));
            let redirect_tail = Arc::new(Mutex::new(String::new()));

            // We may error while configuring, which is expected as part of the internal protocol. The error will be
            // received and the sender of the request might restart us.
            let client = reqwest::blocking::ClientBuilder::new()
                .connect_timeout(std::time::Duration::from_secs(20))
                .http1_title_case_headers()
                .redirect(reqwest::redirect::Policy::custom({
                    let redirect_action = redirect_action.clone();
                    let redirect_tail = redirect_tail.clone();
                    move |attempt| {
                        match *redirect_action.lock() {
                            RedirectAction::Follow => {
                                let curr_url = attempt.url();
                                let prev_urls = attempt.previous();
                                // emulate default git behaviour which relies on curl default behaviour apparently.
                                const CURL_DEFAULT_REDIRS: usize = 50;
                                if prev_urls.len() >= CURL_DEFAULT_REDIRS {
                                    return attempt.error("too many redirects");
                                }

                                match prev_urls.last() {
                                    Some(prev_url) if !redirect::scheme_is_safe(curr_url.as_str(), prev_url.as_str()) => {
                                        // Don't follow insecure protocol redirects, particularly https-to-http downgrades.
                                        attempt.stop()
                                    }
                                    Some(prev_url) if authority_changed(curr_url, prev_url) => {
                                        // Allowed only if the tail doesn't change.
                                        let redirect_tail = redirect_tail.lock();
                                        if curr_url.as_str().ends_with(redirect_tail.as_str()) {
                                            attempt.follow()
                                        } else {
                                            let curr_url = curr_url.as_str().to_owned();
                                            let redirect_tail = redirect_tail.to_string();
                                            attempt.error(format!(
                                                "redirect url {curr_url:?} does not end with expected request suffix {redirect_tail:?}",
                                            ))
                                        }
                                    }
                                    _ => attempt.follow(),
                                }
                            }
                            RedirectAction::RejectConfiguredHeaders => {
                                attempt.error("refusing to follow redirect after request headers were configured")
                            }
                            RedirectAction::Stop => attempt.stop(),
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
                let redirected_base_url = redirected_base_url_shared.lock().clone();
                let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
                let has_configured_extra_headers = !config.extra_headers.is_empty();
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
                let mut has_configure_request = false;
                if let Some(ref mut request_options) = config.backend.as_ref().and_then(|backend| backend.lock().ok()) {
                    if let Some(options) = request_options.downcast_mut::<super::Options>() {
                        if let Some(configure_request) = &mut options.configure_request {
                            has_configure_request = true;
                            configure_request(&mut req)?;
                        }
                    }
                }

                let follow = follow.get_or_insert(config.follow_redirects);
                let may_follow_redirects = matches!(*follow, FollowRedirects::Initial | FollowRedirects::All);
                let has_configured_request_headers = has_configure_request || has_configured_extra_headers;
                *redirect_action.lock() =
                    RedirectAction::from_request(may_follow_redirects, has_configured_request_headers);
                url.strip_prefix(&base_url)
                    .expect("BUG: caller assures `base_url` is subset of `url`")
                    .clone_into(&mut redirect_tail.lock());

                if *follow == FollowRedirects::Initial {
                    *follow = FollowRedirects::None;
                }

                let mut res = match client
                    .execute(req)
                    .and_then(reqwest::blocking::Response::error_for_status)
                {
                    Ok(res) => res,
                    Err(err) => {
                        // `error_for_status()` preserves the final URL for HTTP error responses. Capture it here so
                        // authentication retries after redirected 401 responses use the redirected base URL.
                        if let Some(actual_url) = err.url().map(reqwest::Url::as_str) {
                            if actual_url != effective_url {
                                let new_base_url = redirect::base_url(actual_url, &base_url, url.clone())?;
                                *redirected_base_url_shared.lock() = Some(new_base_url);
                            }
                        }
                        let err = match err.status() {
                            Some(status) => {
                                let kind = if status == reqwest::StatusCode::UNAUTHORIZED {
                                    std::io::ErrorKind::PermissionDenied
                                } else if status.is_server_error() {
                                    std::io::ErrorKind::ConnectionAborted
                                } else {
                                    std::io::ErrorKind::Other
                                };
                                std::io::Error::new(kind, format!("Received HTTP status {}", status.as_str()))
                            }
                            // Preserve the `reqwest::Error` as the source so the underlying cause -- e.g. a
                            // connection or TLS failure -- isn't lost. It was previously stringified, which
                            // dead-ended `source()` and hid the real reason a request failed. See #2140.
                            None => std::io::Error::other(err),
                        };
                        headers_tx.channel.send(Err(err)).ok();
                        continue;
                    }
                };

                let actual_url = res.url().as_str();
                if actual_url != effective_url.as_str() {
                    let new_base_url = redirect::base_url(actual_url, &base_url, url)?;
                    *redirected_base_url_shared.lock() = Some(new_base_url);
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
            redirected_base_url: redirected_base_url_shared_for_field,
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
        http::Error::InitHttpClient {
            source: Box::new(err_that_brought_thread_down),
        }
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
            insert_header(&mut header_map, header_line.as_ref());
        }
        for header_line in &self.config.extra_headers {
            insert_header(&mut header_map, header_line);
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

/// Add one `name: value` header line to `header_map`, ignoring malformed or unsupported input in `header_line`.
///
/// Git configuration may provide arbitrary extra header lines, so invalid names or values are skipped instead of
/// failing request construction. Multiple entries with the same header name are preserved to match curl behavior.
fn insert_header(header_map: &mut reqwest::header::HeaderMap, header_line: &str) {
    let Some(colon_pos) = header_line.find(':') else {
        return;
    };
    let header_name = &header_line[..colon_pos];
    let value = &header_line[colon_pos + 1..];

    if let Some((key, val)) = reqwest::header::HeaderName::from_str(header_name)
        .ok()
        .zip(reqwest::header::HeaderValue::try_from(value.trim()).ok())
    {
        header_map.append(key, val);
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

    fn redirected_base_url(&self) -> Option<String> {
        self.redirected_base_url.lock().clone()
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
