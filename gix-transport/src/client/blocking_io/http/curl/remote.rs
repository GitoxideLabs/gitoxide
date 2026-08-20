use std::{
    io,
    io::{Read, Write},
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread,
    time::Duration,
};

use bstr::ByteSlice;
use curl::easy::{Auth, Easy2};
use gix_features::io::pipe;
use parking_lot::Mutex;

use crate::client::blocking_io::http::{
    self,
    curl::Error,
    curl::curl_is_spurious,
    options::{FollowRedirects, HttpVersion, ProxyAuthMethod, SslVersion},
    redirect::{self, Action as RedirectAction},
    traits::PostBodyDataKind,
};

enum StreamOrBuffer {
    Stream(pipe::Reader),
    Buffer(std::io::Cursor<Vec<u8>>),
}

/// Shared output for the redirected base URL of the active request.
///
/// Before request setup calls `track_redirects()`, this points to a private `None` value created by `Default`.
/// During a request it points to the worker-shared output, allowing redirect headers observed by curl callbacks to
/// update the transport-visible base URL before `perform()` returns.
type SharedRedirectedBaseUrl = Arc<Mutex<Option<String>>>;

#[derive(Default)]
struct Handler {
    /// Sends response headers to the consumer until the request finishes or an error is reported.
    send_header: Option<pipe::Writer>,
    /// Sends response body chunks to the consumer until the request finishes or an error is reported.
    send_data: Option<pipe::Writer>,
    /// Provides the optional upload body to curl, either streamed from the caller or buffered for known-size uploads.
    receive_body: Option<StreamOrBuffer>,
    /// `true` once the status line of the current response header block was parsed.
    checked_status: bool,
    /// Status code of the current response header block, used to associate following headers with redirects.
    current_status: Option<usize>,
    /// Last non-success status reported to the caller, or `200` for a successful transfer.
    last_status: usize,
    /// Redirect policy configured for the current request sequence.
    follow: FollowRedirects,
    /// Per-request redirect behavior.
    redirect_action: RedirectAction,
    /// URL curl is currently requested to fetch, including any previously published redirect target.
    request_url: String,
    /// Caller-provided base URL used to derive the transport-visible base URL after redirects.
    base_url: String,
    redirected_base_url: SharedRedirectedBaseUrl,
}

impl Handler {
    fn reset(&mut self) {
        self.checked_status = false;
        self.current_status = None;
        self.last_status = 0;
        self.follow = FollowRedirects::default();
        self.redirect_action = RedirectAction::Stop;
    }

    fn track_redirects(
        &mut self,
        request_url: String,
        base_url: String,
        redirected_base_url: SharedRedirectedBaseUrl,
        redirect_action: RedirectAction,
    ) {
        self.request_url = request_url;
        self.base_url = base_url;
        self.redirected_base_url = redirected_base_url;
        self.redirect_action = redirect_action;
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

    /// Record a redirected base URL while curl is still reporting response headers.
    ///
    /// Curl provides the normalized final URL through `effective_url()` only after `perform()` finishes
    /// successfully. Redirected authentication failures need the new base before returning the 401 error so the retry
    /// targets the redirected URL, which means we have to inspect and resolve the raw `Location` header here.
    fn publish_redirect_location(&self, data: &[u8]) {
        if self.redirect_action != RedirectAction::Follow || !self.current_status.is_some_and(is_redirect_status) {
            return;
        }

        let Some((name, value)) = std::str::from_utf8(data).ok().and_then(|line| line.split_once(':')) else {
            return;
        };
        if !name.eq_ignore_ascii_case("location") {
            return;
        }

        let Some(location) = absolute_location(&self.request_url, value.trim()) else {
            return;
        };

        let Ok(new_base_url) = redirect::base_url(&location, &self.base_url, self.request_url.clone()) else {
            return;
        };
        *self.redirected_base_url.lock() = Some(new_base_url);
    }
}

/// Convert a raw `Location` header into the absolute URL curl would eventually follow.
///
/// Unlike reqwest's error path, the curl header callback only gives us the server-provided value. Absolute and
/// scheme-relative locations can be completed directly, while path-only locations must be resolved against the current
/// request URL.
fn absolute_location(request_url: &str, location: &str) -> Option<String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Some(location.to_owned());
    }

    let request_url = gix_url::parse(request_url).ok()?;
    let mut out = format!("{}://", request_url.scheme.as_str());
    if location.starts_with("//") {
        out.push_str(location.trim_start_matches('/'));
        return Some(out);
    }
    out.push_str(request_url.host()?);
    if let Some(port) = request_url.port {
        out.push(':');
        out.push_str(&port.to_string());
    }
    let request_path = request_url.original_path().to_str().ok()?;
    out.push_str(&resolve_location_path(request_path, location));
    Some(out)
}

/// Resolve a path-only `Location` value in `location` against the current `request_path`.
///
/// This mirrors URL redirect resolution for the subset curl does not expose during header processing: root-relative
/// paths, relative paths, and query/fragment-only updates, including normalization of `.` and `..` path segments.
fn resolve_location_path(request_path: &str, location: &str) -> String {
    let (path, suffix) = split_url_path_suffix(location);
    if path.starts_with('/') {
        let mut out = normalize_url_path(path);
        out.push_str(suffix);
        return out;
    }

    let (request_path, _) = split_url_path_suffix(request_path);
    if path.is_empty() {
        let mut out = request_path.to_owned();
        out.push_str(suffix);
        return out;
    }

    let base_dir = request_path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut merged = String::new();
    merged.push_str(base_dir);
    merged.push('/');
    merged.push_str(path);
    let mut out = normalize_url_path(&merged);
    out.push_str(suffix);
    out
}

fn split_url_path_suffix(input: &str) -> (&str, &str) {
    input
        .find(['?', '#'])
        .map_or((input, ""), |suffix| input.split_at(suffix))
}

/// Normalize URL path segments while preserving absolute and trailing-slash shape.
fn normalize_url_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let trailing_slash = path.ends_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }

    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&segments.join("/"));
    if trailing_slash && !out.ends_with('/') {
        out.push('/');
    }
    if out.is_empty() {
        out.push('/');
    }
    out
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
        if self.send_header.is_none() {
            return true;
        }
        let header_block_is_done = matches!(data, b"\r\n" | b"\n");
        if header_block_is_done {
            if let Some(writer) = self.send_header.as_mut() {
                writer.write_all(data).ok();
            }
            self.checked_status = false;
            self.current_status = None;
            return true;
        }
        if self.checked_status {
            self.publish_redirect_location(data);
            if let Some(writer) = self.send_header.as_mut() {
                writer.write_all(data).ok();
            }
        } else {
            self.checked_status = true;
            self.last_status = 200;
            self.current_status = Handler::parse_status_inner(data).ok();
            let err = self.current_status.and_then(|status| {
                if self.redirect_action == RedirectAction::RejectConfiguredHeaders && is_redirect_status(status) {
                    Some((
                        status,
                        "refusing to follow redirect after request headers were configured".into(),
                    ))
                } else {
                    Handler::parse_status(data, self.follow)
                }
            });
            if let Some((status, err)) = err {
                self.last_status = status;
                if let Some(writer) = self.send_header.as_mut() {
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

type Worker = (
    thread::JoinHandle<Result<(), Error>>,
    SyncSender<Request>,
    Receiver<Response>,
    SharedRedirectedBaseUrl,
);

pub fn new() -> Worker {
    let redirected_base_url_shared = Arc::new(Mutex::new(None));
    let redirected_base_url_shared_out = redirected_base_url_shared.clone();
    let (req_send, req_recv) = sync_channel(0);
    let (res_send, res_recv) = sync_channel(0);
    let handle = std::thread::spawn(move || -> Result<(), Error> {
        let mut handle = Easy2::new(Handler::default());
        // We don't wait for the possibility for pipelining to become clear, and curl tries to reuse connections by default anyway.
        handle.pipewait(false)?;
        handle.tcp_keepalive(true)?;

        let mut follow = None;

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
            let redirected_base_url = redirected_base_url_shared.lock().clone();
            let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
            handle.url(&effective_url)?;

            handle.post(upload_body_kind.is_some())?;
            let has_extra_headers = !extra_headers.is_empty();
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
                    let creds = authenticate.lock().expect("no panics in other threads")(obtain_creds_action)
                        .map_err(|err| Error::Authenticate(err.into_error()))?
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
            let may_follow_redirects = matches!(*follow, FollowRedirects::Initial | FollowRedirects::All);
            let redirect_action = RedirectAction::from_request(may_follow_redirects, has_extra_headers);
            {
                let handler = handle.get_mut();
                handler.follow = *follow;
                handler.track_redirects(
                    effective_url.clone(),
                    base_url.clone(),
                    redirected_base_url_shared.clone(),
                    redirect_action,
                );
            }
            handle.follow_location(redirect_action == RedirectAction::Follow)?;

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
                let actual_url = handle
                    .effective_url()?
                    .expect("effective url is present and valid UTF-8");
                if actual_url != effective_url {
                    let new_base_url = redirect::base_url(actual_url, &base_url, url)?;
                    *redirected_base_url_shared.lock() = Some(new_base_url);
                }

                let handler = handle.get_mut();
                if let Some((action, authenticate)) = proxy_auth_action {
                    authenticate.lock().expect("no panics in other threads")(if handler.last_status == 200 {
                        action.store()
                    } else {
                        action.erase()
                    })
                    .map_err(|err| Error::Authenticate(err.into_error()))?;
                }
                handler.reset();
                handler.receive_body.take();
                handler.send_header.take();
                handler.send_data.take();
            }
        }
        Ok(())
    });
    (handle, req_send, res_recv, redirected_base_url_shared_out)
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

fn is_redirect_status(status: usize) -> bool {
    (300..=308).contains(&status)
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

#[cfg(test)]
mod absolute_location_tests {
    use super::absolute_location;

    const REQUEST_URL: &str = "http://example.com:8080/original/repo/info/refs?service=git-upload-pack";

    #[test]
    fn keeps_absolute_locations() {
        assert_eq!(
            absolute_location(
                REQUEST_URL,
                "https://redirected.example/repo/info/refs?service=git-upload-pack"
            ),
            Some("https://redirected.example/repo/info/refs?service=git-upload-pack".to_owned()),
            "absolute Location values already contain the authority curl would follow"
        );
    }

    #[test]
    fn uses_request_scheme_for_scheme_relative_locations() {
        assert_eq!(
            absolute_location(
                REQUEST_URL,
                "//redirected.example/repo/info/refs?service=git-upload-pack"
            ),
            Some("http://redirected.example/repo/info/refs?service=git-upload-pack".to_owned()),
            "scheme-relative Location values inherit the scheme of the request URL"
        );
    }

    #[test]
    fn resolves_root_relative_locations() {
        assert_eq!(
            absolute_location(
                REQUEST_URL,
                "/redirected/./repo/../repo/info/refs?service=git-upload-pack"
            ),
            Some("http://example.com:8080/redirected/repo/info/refs?service=git-upload-pack".to_owned()),
            "root-relative Location values keep the request authority and normalize path segments"
        );
    }

    #[test]
    fn resolves_relative_locations_from_the_request_directory() {
        assert_eq!(
            absolute_location(
                REQUEST_URL,
                "../../../redirected/repo/info/refs?service=git-upload-pack"
            ),
            Some("http://example.com:8080/redirected/repo/info/refs?service=git-upload-pack".to_owned()),
            "relative Location values are resolved from the current request directory before normalization"
        );
    }

    #[test]
    fn resolves_query_only_locations_against_the_request_path() {
        assert_eq!(
            absolute_location(REQUEST_URL, "?service=git-receive-pack"),
            Some("http://example.com:8080/original/repo/info/refs?service=git-receive-pack".to_owned()),
            "query-only Location values replace the query while preserving the current request path"
        );
    }

    #[test]
    fn keeps_percent_encoded_separators_in_relative_locations() {
        assert_eq!(
            absolute_location(
                "http://example.com/a%2Fb/info/refs?service=git-upload-pack",
                "../objects/info/packs"
            ),
            Some("http://example.com/a%2Fb/objects/info/packs".to_owned()),
            "relative redirects resolve against the encoded request path"
        );
    }

    #[test]
    fn keeps_percent_encoded_non_separators_in_relative_locations() {
        assert_eq!(
            absolute_location(
                "http://example.com/a%20b/info/refs?service=git-upload-pack",
                "../objects/info/packs"
            ),
            Some("http://example.com/a%20b/objects/info/packs".to_owned()),
            "relative redirects use the original encoded request path"
        );
    }
}

#[cfg(test)]
mod resolve_location_path_tests {
    use super::resolve_location_path;

    const REQUEST_PATH: &str = "/original/repo/info/refs?service=git-upload-pack";

    #[test]
    fn keeps_root_relative_locations_rooted_and_normalized() {
        assert_eq!(
            resolve_location_path(
                REQUEST_PATH,
                "/redirected/./repo/../repo/info/refs?service=git-upload-pack"
            ),
            "/redirected/repo/info/refs?service=git-upload-pack",
            "root-relative Location paths should ignore the request path and normalize their own segments"
        );
    }

    #[test]
    fn resolves_relative_locations_from_the_request_directory() {
        assert_eq!(
            resolve_location_path(
                REQUEST_PATH,
                "../../../redirected/repo/info/refs?service=git-upload-pack"
            ),
            "/redirected/repo/info/refs?service=git-upload-pack",
            "relative Location paths should resolve from the directory containing the current request path"
        );
    }

    #[test]
    fn replaces_query_only_locations_on_the_request_path() {
        assert_eq!(
            resolve_location_path(REQUEST_PATH, "?service=git-receive-pack"),
            "/original/repo/info/refs?service=git-receive-pack",
            "query-only Location values should preserve the current path and replace only the query"
        );
    }

    #[test]
    fn replaces_fragment_only_locations_on_the_request_path() {
        assert_eq!(
            resolve_location_path(REQUEST_PATH, "#advertisement"),
            "/original/repo/info/refs#advertisement",
            "fragment-only Location values should preserve the current path and replace the query with the fragment"
        );
    }

    #[test]
    fn strips_request_query_before_resolving_relative_locations() {
        assert_eq!(
            resolve_location_path(REQUEST_PATH, "../objects/info/packs"),
            "/original/repo/objects/info/packs",
            "relative Location paths should resolve against the request path without its existing query"
        );
    }
}
