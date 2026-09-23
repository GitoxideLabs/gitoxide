use std::{
    any::Any,
    error::Error as _,
    io::{Read, Write},
    str::FromStr,
    sync::Arc,
};

use gix_error::{ErrorExt, ExnMessageResult, ExnResult, ResultExt, message};
use gix_features::io::pipe;
use parking_lot::Mutex;
use reqwest::{Method, StatusCode, header};

use crate::client::blocking_io::http::{
    self,
    options::{FollowRedirects, ProxyAuthMethod},
    redirect::{self, Action as RedirectAction},
    reqwest::Remote,
    traits::PostBodyDataKind,
};

fn classify_reqwest(err: reqwest::Error) -> gix_error::Error {
    if err.is_timeout() || err.is_connect() || err.status().is_some_and(|status| status.is_server_error()) {
        gix_error::Error::from_error(gix_error::ClassificationMarker::with_source(
            gix_error::Class::Retryable,
            err,
        ))
    } else {
        gix_error::Error::from_error(err)
    }
}

fn authority_changed(curr_url: &reqwest::Url, prev_url: &reqwest::Url) -> bool {
    curr_url.scheme() != prev_url.scheme()
        || curr_url.host_str() != prev_url.host_str()
        || curr_url.port_or_known_default() != prev_url.port_or_known_default()
}

#[derive(Default)]
struct RedirectState {
    action: RedirectAction,
    tail: String,
    next_url: Option<reqwest::Url>,
    count: usize,
}

impl Default for Remote {
    fn default() -> Self {
        let (req_send, req_recv) = std::sync::mpsc::sync_channel(0);
        let (res_send, res_recv) = std::sync::mpsc::sync_channel(0);
        let redirected_base_url_shared = Arc::new(Mutex::new(None));
        let redirected_base_url_shared_for_field = redirected_base_url_shared.clone();
        let handle = std::thread::spawn(move || -> ExnMessageResult {
            let mut follow = None;
            let redirects = Arc::new(Mutex::new(RedirectState::default()));

            // Reuse connections until the effective proxy configuration (including credentials) changes.
            let mut client: Option<(Option<gix_url::Url>, reqwest::blocking::Client)> = None;
            let create_client = |proxy: Option<&gix_url::Url>| -> ExnMessageResult<_> {
                let mut builder = reqwest::blocking::ClientBuilder::new()
                    .no_proxy()
                    .connect_timeout(std::time::Duration::from_secs(20))
                    .http1_title_case_headers()
                    .redirect(reqwest::redirect::Policy::custom({
                        let redirects = redirects.clone();
                        move |attempt| {
                            let mut redirects = redirects.lock();
                            match redirects.action {
                                RedirectAction::Follow => {
                                    let curr_url = attempt.url();
                                    let prev_urls = attempt.previous();
                                    // emulate default git behaviour which relies on curl default behaviour apparently.
                                    const CURL_DEFAULT_REDIRS: usize = 50;
                                    redirects.count += 1;
                                    if redirects.count >= CURL_DEFAULT_REDIRS {
                                        return attempt.error("too many redirects");
                                    }
                                    if let Some(prev_url) = prev_urls.last()
                                        && !redirect::scheme_is_safe(curr_url.as_str(), prev_url.as_str())
                                    {
                                        // Don't follow insecure protocol redirects, particularly https-to-http downgrades.
                                        return attempt.stop();
                                    }
                                    if prev_urls.last().is_some_and(|prev_url| authority_changed(curr_url, prev_url))
                                        && !curr_url.as_str().ends_with(&redirects.tail)
                                    {
                                        let curr_url = curr_url.as_str().to_owned();
                                        let redirect_tail = &redirects.tail;
                                        return attempt.error(format!(
                                            "redirect url {curr_url:?} does not end with expected request suffix {redirect_tail:?}",
                                        ));
                                    }
                                    // Execute each hop ourselves: reqwest otherwise strips proxy credentials on
                                    // authority changes without adding them again for the selected proxy.
                                    redirects.next_url = Some(curr_url.clone());
                                    attempt.stop()
                                }
                                RedirectAction::RejectConfiguredHeaders => {
                                    attempt.error("refusing to follow redirect after request headers were configured")
                                }
                                RedirectAction::Stop => attempt.stop(),
                            }
                        }
                    }));
                if let Some(proxy) = proxy {
                    builder = builder.proxy(
                        reqwest::Proxy::all(proxy.to_bstring().to_string())
                            .map_err(classify_reqwest)
                            .or_raise(|| message("Could not configure HTTP proxy"))?,
                    );
                }
                builder
                    .build()
                    .map_err(classify_reqwest)
                    .or_raise(|| message("Could not initialize HTTP client"))
            };

            'requests: for Request {
                url,
                base_url,
                headers,
                upload_body_kind,
                config,
            } in req_recv
            {
                let redirected_base_url = redirected_base_url_shared.lock().clone();
                let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
                let no_proxy = config.no_proxy.clone().or_else(|| proxy_env("no_proxy", "NO_PROXY"));
                let has_configured_extra_headers = !config.extra_headers.is_empty();
                let mut req = reqwest::blocking::Request::new(
                    if upload_body_kind.is_some() {
                        Method::POST
                    } else {
                        Method::GET
                    },
                    reqwest::Url::parse(&effective_url).or_raise(|| message("Request configuration failed"))?,
                );
                *req.headers_mut() = headers;
                if !req.url().username().is_empty() || req.url().password().is_some() {
                    use base64::Engine;
                    let origin =
                        gix_url::parse(req.url().as_str()).or_raise(|| message("Request configuration failed"))?;
                    let value = format!(
                        "Basic {}",
                        base64::engine::general_purpose::STANDARD.encode(format!(
                            "{}:{}",
                            origin.user().unwrap_or_default(),
                            origin.password().unwrap_or_default()
                        ))
                    );
                    req.headers_mut()
                        .entry(header::AUTHORIZATION)
                        .or_insert(value.parse().expect("base64 is a valid header"));
                    req.url_mut().set_username("").ok();
                    req.url_mut().set_password(None).ok();
                }
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
                *req.body_mut() = match upload_body_kind {
                    Some(PostBodyDataKind::BoundedAndFitsIntoMemory) => {
                        let mut buf = Vec::<u8>::with_capacity(512);
                        post_body_rx
                            .read_to_end(&mut buf)
                            .or_raise(|| message("Could not finish reading all data to post to the remote"))?;
                        Some(buf.into())
                    }
                    Some(PostBodyDataKind::Unbounded) => Some(reqwest::blocking::Body::new(post_body_rx)),
                    None => None,
                };
                let mut has_configure_request = false;
                if let Some(ref mut request_options) = config.backend.as_ref().and_then(|backend| backend.lock().ok())
                    && let Some(options) = request_options.downcast_mut::<super::Options>()
                    && let Some(configure_request) = &mut options.configure_request
                {
                    has_configure_request = true;
                    configure_request(&mut req).or_raise(|| message("Request configuration failed"))?;
                }
                let follow = follow.get_or_insert(config.follow_redirects);
                let may_follow_redirects = matches!(*follow, FollowRedirects::Initial | FollowRedirects::All);
                let has_configured_request_headers = has_configure_request || has_configured_extra_headers;
                *redirects.lock() = RedirectState {
                    action: RedirectAction::from_request(may_follow_redirects, has_configured_request_headers),
                    tail: url
                        .strip_prefix(&base_url)
                        .expect("BUG: caller assures `base_url` is subset of `url`")
                        .into(),
                    ..Default::default()
                };

                if *follow == FollowRedirects::Initial {
                    *follow = FollowRedirects::None;
                }

                let mut proxy_credentials: Option<(gix_url::Url, gix_credentials::protocol::Outcome)> = None;
                let response = loop {
                    let mut proxy = if bypasses_proxy(no_proxy.as_deref(), req.url()) {
                        None
                    } else {
                        match proxy_url(&config, req.url().scheme()) {
                            Ok(proxy) => proxy,
                            Err(err) => {
                                drop(req); // Release a streamed upload before sending its error to the header reader.
                                headers_tx
                                    .channel
                                    .send(Err(std::io::Error::other(err.into_error())))
                                    .ok();
                                continue 'requests;
                            }
                        }
                    };
                    let mut proxy_auth_action = None;
                    if let Some(proxy) = proxy.as_mut()
                        && let Some((action, authenticate)) = &config.proxy_authenticate
                        && (config.proxy.is_some() || proxy.user.is_some())
                    {
                        if proxy_credentials.as_ref().is_none_or(|(previous, _)| previous != proxy) {
                            let action = if config.proxy.is_some() {
                                action.clone()
                            } else {
                                gix_credentials::helper::Action::get_for_url(proxy.to_bstring())
                            };
                            let credentials = match authenticate.lock().expect("no panics in other threads")(action)
                                .or_raise(|| message("Could not obtain proxy credentials"))
                                .and_then(|credentials| {
                                    credentials.ok_or_else(|| {
                                        message("The proxy credential helper returned no credentials").raise()
                                    })
                                }) {
                                Ok(credentials) => credentials,
                                Err(err) => {
                                    drop(req);
                                    headers_tx
                                        .channel
                                        .send(Err(std::io::Error::other(err.into_error())))
                                        .ok();
                                    continue 'requests;
                                }
                            };
                            proxy_credentials = Some((proxy.clone(), credentials));
                        }
                        let (_, credentials) = proxy_credentials.as_ref().expect("credentials were initialized above");
                        proxy.user = Some(credentials.identity.username.clone());
                        proxy.password = Some(credentials.identity.password.clone());
                        proxy_auth_action = Some((credentials.next.clone(), authenticate));
                    }
                    if client
                        .as_ref()
                        .is_none_or(|(previous_proxy, _)| previous_proxy != &proxy)
                    {
                        let new_client = match create_client(proxy.as_ref()) {
                            Ok(client) => client,
                            Err(err) => {
                                drop(req);
                                headers_tx
                                    .channel
                                    .send(Err(std::io::Error::other(err.into_error())))
                                    .ok();
                                continue 'requests;
                            }
                        };
                        client = Some((proxy, new_client));
                    }
                    let (_, client) = client.as_ref().expect("client was initialized above");
                    let mut next_req = req.try_clone().unwrap_or_else(|| {
                        // A streamed POST can still redirect to a GET. Reqwest stops 307/308 redirects
                        // before invoking our policy when it cannot replay the body.
                        let body = req.body_mut().take();
                        let copy = req.try_clone().expect("a request without a body can be cloned");
                        *req.body_mut() = body;
                        copy
                    });
                    let response = client.execute(req);
                    let proxy_auth_failed = match &response {
                        Ok(res) => res.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED,
                        Err(err) => {
                            // ponytail: reqwest hides CONNECT's status and hyper-util's error type is private.
                            // Use a typed check when either dependency exposes one.
                            err.is_connect()
                                && std::iter::successors(err.source(), |&err| err.source())
                                    .any(|err| err.to_string() == "tunnel error: proxy authorization required")
                        }
                    };
                    if (response.is_ok() || proxy_auth_failed)
                        && let Some((action, authenticate)) = proxy_auth_action
                    {
                        let action = if proxy_auth_failed {
                            action.erase()
                        } else {
                            action.store()
                        };
                        if let Err(err) = authenticate.lock().expect("no panics in other threads")(action) {
                            headers_tx
                                .channel
                                .send(Err(std::io::Error::other(err.into_error())))
                                .ok();
                            continue 'requests;
                        }
                    }
                    let Some(next_url) = redirects.lock().next_url.take() else {
                        break response;
                    };
                    let res = match response {
                        Ok(res) => res,
                        Err(err) => break Err(err),
                    };
                    if res.status() == StatusCode::SEE_OTHER
                        || (matches!(res.status(), StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND)
                            && next_req.method() == Method::POST)
                    {
                        if next_req.method() != Method::HEAD {
                            *next_req.method_mut() = Method::GET;
                        }
                        *next_req.body_mut() = None;
                        for name in [
                            header::CONTENT_LENGTH,
                            header::CONTENT_TYPE,
                            header::TRANSFER_ENCODING,
                            header::CONTENT_ENCODING,
                        ] {
                            next_req.headers_mut().remove(name);
                        }
                    }
                    if authority_changed(&next_url, next_req.url()) {
                        for name in [
                            header::AUTHORIZATION,
                            header::COOKIE,
                            header::WWW_AUTHENTICATE,
                            header::PROXY_AUTHORIZATION,
                        ] {
                            next_req.headers_mut().remove(name);
                        }
                        next_req.headers_mut().remove("cookie2");
                    }
                    let mut referer = next_req.url().clone();
                    referer.set_username("").ok();
                    referer.set_password(None).ok();
                    referer.set_fragment(None);
                    if let Ok(value) = header::HeaderValue::from_str(referer.as_str()) {
                        next_req.headers_mut().insert(header::REFERER, value);
                    }
                    *next_req.url_mut() = next_url;
                    req = next_req;
                };

                let mut www_authenticate = Vec::new();
                let mut res = match response.and_then(|res| {
                    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
                        www_authenticate = res
                            .headers()
                            .get_all(reqwest::header::WWW_AUTHENTICATE)
                            .iter()
                            .map(|value| value.as_bytes().into())
                            .collect();
                    }
                    res.error_for_status()
                }) {
                    Ok(res) => res,
                    Err(err) => {
                        // `error_for_status()` preserves the final URL for HTTP error responses. Capture it here so
                        // authentication retries after redirected 401 responses use the redirected base URL.
                        if let Some(actual_url) = err.url().map(reqwest::Url::as_str)
                            && actual_url != effective_url
                        {
                            let new_base_url = redirect::base_url(actual_url, &base_url, url.clone())?;
                            *redirected_base_url_shared.lock() = Some(new_base_url);
                        }
                        let err = match err.status() {
                            Some(reqwest::StatusCode::UNAUTHORIZED) => std::io::Error::new(
                                std::io::ErrorKind::PermissionDenied,
                                crate::client::AuthenticationRequired { www_authenticate },
                            ),
                            Some(status) => {
                                let kind = if status.is_server_error() {
                                    std::io::ErrorKind::ConnectionAborted
                                } else {
                                    std::io::ErrorKind::Other
                                };
                                std::io::Error::new(kind, format!("Received HTTP status {}", status.as_str()))
                            }
                            // Preserve the `reqwest::Error` as the source so the underlying cause -- e.g. a
                            // connection or TLS failure -- isn't lost. It was previously stringified, which
                            // dead-ended `source()` and hid the real reason a request failed. See #2140.
                            None => std::io::Error::other(classify_reqwest(err)),
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
    fn restore_thread_after_failure(&mut self) -> gix_error::Exn<gix_error::Message> {
        let err_that_brought_thread_down = self
            .handle
            .take()
            .expect("thread handle present")
            .join()
            .expect("handler thread should never panic")
            .expect_err("something should have gone wrong with HTTP (we join on error only)");
        *self = Remote {
            config: std::mem::take(&mut self.config),
            ..Remote::default()
        };
        err_that_brought_thread_down.raise(message("Could not initialize the http client"))
    }

    fn make_request(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        upload_body_kind: Option<PostBodyDataKind>,
    ) -> ExnMessageResult<http::PostResponse<pipe::Reader, pipe::Reader, pipe::Writer>> {
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

fn proxy_env(lower: &str, upper: &str) -> Option<String> {
    [lower, upper]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
}

fn bypasses_proxy(no_proxy: Option<&str>, url: &reqwest::Url) -> bool {
    let Some(no_proxy) = no_proxy.filter(|value| !value.is_empty()) else {
        return false;
    };
    // The wildcard in reqwest's matcher currently only covers domain names, not IP addresses.
    no_proxy == "*"
        || url
            .host_str()
            .and_then(|host| format!("http://{host}").parse().ok())
            .is_some_and(|uri| {
                // Use reqwest's own bypass rules before proxy validation and credential lookup.
                // This dummy intercept target is only needed to construct the matcher, and is never contacted.
                hyper_util::client::proxy::matcher::Matcher::builder()
                    .all("http://proxy.invalid")
                    .no(no_proxy)
                    .build()
                    .intercept(&uri)
                    .is_none()
            })
}

fn proxy_url(config: &http::Options, scheme: &str) -> ExnMessageResult<Option<gix_url::Url>> {
    let proxy = config.proxy.clone().or_else(|| {
        // Like Git and curl, ignore uppercase HTTP_PROXY, which can originate in a CGI request.
        if scheme == "https" {
            proxy_env("https_proxy", "HTTPS_PROXY")
        } else {
            std::env::var("http_proxy").ok().filter(|value| !value.is_empty())
        }
        .or_else(|| proxy_env("all_proxy", "ALL_PROXY"))
    });
    let Some(mut proxy) = proxy.filter(|proxy| !proxy.is_empty()) else {
        return Ok(None);
    };
    if !proxy.contains("://") {
        proxy.insert_str(0, "http://");
    }
    let mut proxy = gix_url::parse(proxy.as_str()).or_raise(|| message("Invalid proxy URL"))?;
    if !matches!(
        proxy.scheme.as_str(),
        "http" | "https" | "socks4" | "socks4a" | "socks5" | "socks5h"
    ) {
        return Err(message!("Unsupported proxy scheme '{}'", proxy.scheme.as_str()).raise());
    }
    if !proxy.path.is_empty() && proxy.path.as_slice() != b"/" {
        return Err(message("The reqwest backend does not support Unix socket proxy paths").raise());
    }
    if matches!(proxy.scheme.as_str(), "socks4" | "socks4a")
        && (proxy.user.is_some() || (config.proxy.is_some() && config.proxy_authenticate.is_some()))
    {
        return Err(message("The reqwest backend does not support SOCKS4 proxy user IDs").raise());
    }
    if proxy.scheme == gix_url::Scheme::Http && proxy.port.is_none() {
        // Git/libcurl's default for an HTTP proxy is 1080, unlike reqwest's 80.
        proxy.port = Some(1080);
    }
    if matches!(proxy.scheme, gix_url::Scheme::Http | gix_url::Scheme::Https)
        && (proxy.user.is_some() || (config.proxy.is_some() && config.proxy_authenticate.is_some()))
        && !matches!(
            config.proxy_auth_method,
            ProxyAuthMethod::AnyAuth | ProxyAuthMethod::Basic
        )
    {
        return Err(message("The reqwest backend only supports Basic HTTP proxy authentication").raise());
    }
    // Validate before constructing the client, without checking an unused destination scheme.
    reqwest::Proxy::all(proxy.to_bstring().to_string()).or_raise(|| message("Invalid proxy URL"))?;
    Ok(Some(proxy))
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
    ) -> ExnMessageResult<http::GetResponse<Self::Headers, Self::ResponseBody>> {
        self.make_request(url, base_url, headers, None).map(Into::into)
    }

    fn post(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        post_body_kind: PostBodyDataKind,
    ) -> ExnMessageResult<http::PostResponse<Self::Headers, Self::ResponseBody, Self::PostBody>> {
        self.make_request(url, base_url, headers, Some(post_body_kind))
    }

    fn configure(&mut self, config: &dyn Any) -> ExnResult {
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

#[cfg(test)]
mod tests {
    #[test]
    fn http_proxy_default_port_matches_curl() -> gix_testtools::Result {
        for (input, port) in [
            ("proxy.example", Some(1080)),
            ("http://proxy.example", Some(1080)),
            ("http://proxy.example:80", Some(80)),
            ("https://proxy.example", None),
        ] {
            let proxy = super::proxy_url(
                &super::http::Options {
                    proxy: Some(input.into()),
                    ..Default::default()
                },
                "http",
            )
            .map_err(gix_error::Exn::into_error)?
            .expect("the proxy is configured");
            assert_eq!(proxy.port, port, "curl-style proxy port for {input}");
        }
        Ok(())
    }
}
