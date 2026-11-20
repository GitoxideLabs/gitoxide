use std::{
    any::Any,
    error::Error as _,
    io::{Read, Write},
    str::FromStr,
    sync::Arc,
};

use gix_error::{Class, Error, ErrorExt, Result, ResultExt, message};
use gix_reqwest as reqwest;
use gix_utils::io::pipe;
use parking_lot::Mutex;
use reqwest::{Method, StatusCode, header};

use crate::client::blocking_io::http::{
    self,
    options::{FollowRedirects, ProxyAuthMethod},
    redirect::{self, Action as RedirectAction},
    reqwest::Remote,
    traits::PostBodyDataKind,
};

fn classify_reqwest(err: reqwest::Error) -> Error {
    let class = match err.status() {
        Some(reqwest::StatusCode::FORBIDDEN) => Some(Class::PermissionDenied),
        Some(reqwest::StatusCode::CONFLICT) => Some(Class::Conflict),
        Some(reqwest::StatusCode::NOT_IMPLEMENTED) => Some(Class::Unsupported),
        _ if err.is_timeout() || err.is_connect() || err.status().is_some_and(|status| status.is_server_error()) => {
            Some(Class::Retryable)
        }
        _ => None,
    };
    match class {
        Some(class) => Error::from_error(gix_error::ClassificationMarker::with_source(class, err)),
        None => Error::from_error(err),
    }
}

fn authority_changed(curr_url: &reqwest::Url, prev_url: &reqwest::Url) -> bool {
    curr_url.scheme() != prev_url.scheme()
        || curr_url.host_str() != prev_url.host_str()
        || curr_url.port_or_known_default() != prev_url.port_or_known_default()
}

fn basic_auth(username: &str, password: &str) -> header::HeaderValue {
    use base64::Engine;

    let value = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
    );
    let mut value = header::HeaderValue::from_str(&value).expect("base64 is a valid header");
    value.set_sensitive(true);
    value
}

#[derive(Default)]
struct RedirectState {
    action: RedirectAction,
    tail: String,
    request: ProxyRequest,
}

#[derive(Clone, Default, Eq, PartialEq)]
struct ProxyRouting {
    proxy: Option<String>,
    no_proxy: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    all_proxy: Option<String>,
    environment_no_proxy: Option<String>,
}

impl ProxyRouting {
    fn capture(config: &http::Options) -> Self {
        Self {
            proxy: config.proxy.clone(),
            no_proxy: config.no_proxy.clone(),
            // Ignore uppercase HTTP_PROXY, which can originate in a CGI request.
            http_proxy: std::env::var("http_proxy").ok().filter(|value| !value.is_empty()),
            https_proxy: proxy_env("https_proxy", "HTTPS_PROXY"),
            all_proxy: proxy_env("all_proxy", "ALL_PROXY"),
            environment_no_proxy: proxy_env("no_proxy", "NO_PROXY"),
        }
    }
}

#[derive(Default)]
struct ProxyRequest {
    config: http::Options,
    routing: ProxyRouting,
    proxy: Option<gix_url::Url>,
    proxy_changed: bool,
    crossed_authority: bool,
    credentials: Vec<(gix_url::Url, gix_credentials::protocol::Outcome)>,
    selected_credentials: Option<usize>,
}

impl ProxyRequest {
    fn prepare(&mut self, url: &reqwest::Url) -> Result {
        self.proxy = None;
        self.selected_credentials = None;
        let no_proxy = self
            .routing
            .no_proxy
            .as_deref()
            .or(self.routing.environment_no_proxy.as_deref());
        let mut proxy = if bypasses_proxy(no_proxy, url) {
            None
        } else {
            proxy_url(&self.config, &self.routing, url.scheme())?
        };
        if let Some(proxy) = proxy.as_mut()
            && let Some((action, authenticate)) = &self.config.proxy_authenticate
            && (self.config.proxy.is_some() || proxy.user.is_some())
        {
            let index = match self.credentials.iter().position(|(declared, _)| declared == proxy) {
                Some(index) => index,
                None => {
                    let action = if self.config.proxy.is_some() {
                        action.clone()
                    } else {
                        gix_credentials::helper::Action::get_for_url(proxy.to_bstring())
                    };
                    let credentials = authenticate.lock().expect("no panics in other threads")(action)
                        .or_raise(|| message("Could not obtain proxy credentials"))?
                        .ok_or_else(|| message("The proxy credential helper returned no credentials").raise())?;
                    self.credentials.push((proxy.clone(), credentials));
                    self.credentials.len() - 1
                }
            };
            match &self.credentials[index].1.credentials {
                gix_sec::identity::Credentials::UsernamePassword { username, password, .. } => {
                    proxy.user = Some(username.clone());
                    proxy.password = Some(password.clone());
                }
                gix_sec::identity::Credentials::Encoded { .. } => {
                    return Err(
                        message("Pre-encoded credentials are not supported for proxy authentication")
                            .unsupported_error(),
                    );
                }
            }
            self.selected_credentials = Some(index);
        }
        if let Some(proxy) = &proxy {
            // Validate the authenticated URL too: custom proxy callbacks cannot report parse errors.
            reqwest::Proxy::all(proxy.to_bstring().to_string()).or_raise(|| message("Invalid proxy URL"))?;
        }
        self.proxy = proxy;
        Ok(())
    }

    fn finish_authentication(&self, failed: bool) -> Result {
        if let Some(index) = self.selected_credentials
            && let Some((_, authenticate)) = &self.config.proxy_authenticate
        {
            let next = self.credentials[index].1.next.clone();
            authenticate.lock().expect("no panics in other threads")(if failed { next.erase() } else { next.store() })?;
        }
        Ok(())
    }
}

impl Default for Remote {
    fn default() -> Self {
        let (req_send, req_recv) = std::sync::mpsc::sync_channel(0);
        let (res_send, res_recv) = std::sync::mpsc::sync_channel(0);
        let redirected_base_url_shared = Arc::new(Mutex::new(None));
        let redirected_base_url_shared_for_field = redirected_base_url_shared.clone();
        let handle = std::thread::spawn(gix_trace::in_thread(move || -> Result {
            let mut follow = None;
            let mut suppress_client_defaults = false;
            let redirects = Arc::new(Mutex::new(RedirectState::default()));

            // Reuse connections until the effective proxy configuration (including credentials) changes.
            let mut client: Option<(ProxyRouting, Option<gix_url::Url>, (reqwest::blocking::Client, bool))> = None;
            let create_builder = |config: &http::Options| -> Result<_> {
                let mut customized = false;
                let mut builder = reqwest::blocking::ClientBuilder::new()
                    .connect_timeout(std::time::Duration::from_secs(20))
                    .http1_title_case_headers();
                if let Some(backend) = config.backend.as_ref() {
                    let mut backend = backend
                        .lock()
                        .map_err(|_| message("HTTP backend configuration lock is poisoned").raise())?;
                    if let Some(options) = backend.downcast_mut::<super::Options>()
                        && let Some(configure_client) = options.configure_client.as_mut()
                    {
                        customized = true;
                        builder = configure_client(builder).or_raise(|| message("HTTP client configuration failed"))?;
                    }
                }
                Ok((builder, customized))
            };
            let create_client = |(builder, customized): (reqwest::blocking::ClientBuilder, bool)| -> Result<_> {
                // Keep Git's proxy and redirect policies authoritative after client customization.
                builder
                    .no_proxy()
                    .proxy(reqwest::Proxy::custom({
                        let redirects = redirects.clone();
                        move |_| redirects.lock().request.proxy.as_ref().map(|proxy| proxy.to_bstring().to_string())
                    }))
                    .redirect(reqwest::redirect::Policy::custom({
                        let redirects = redirects.clone();
                        move |attempt| {
                            let mut redirects = redirects.lock();
                            match redirects.action {
                                RedirectAction::Follow => {
                                    let curr_url = attempt.url();
                                    let prev_urls = attempt.previous();
                                    // Emulate Git's curl redirect limit.
                                    const CURL_DEFAULT_REDIRS: usize = 50;
                                    if prev_urls.len() >= CURL_DEFAULT_REDIRS {
                                        return attempt.error("too many redirects");
                                    }
                                    if let Some(prev_url) = prev_urls.last()
                                        && !redirect::scheme_is_safe(curr_url.as_str(), prev_url.as_str())
                                    {
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
                                    let previous_proxy = redirects.request.proxy.clone();
                                    if let Err(err) = redirects.request.finish_authentication(false)
                                        .and_then(|()| redirects.request.prepare(curr_url))
                                    {
                                        return attempt.error(std::io::Error::other(err));
                                    }
                                    redirects.request.proxy_changed |= redirects.request.proxy != previous_proxy;
                                    redirects.request.crossed_authority |= prev_urls
                                        .last()
                                        .is_some_and(|prev_url| authority_changed(curr_url, prev_url));
                                    attempt.follow()
                                }
                                RedirectAction::RejectConfiguredHeaders => {
                                    attempt.error("refusing to follow redirect after request headers were configured")
                                }
                                RedirectAction::Stop => attempt.stop(),
                            }
                        }
                    }))
                    .build()
                    .map(|client| (client, customized))
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
                let mut client_builder = client.is_none().then(|| create_builder(&config)).transpose()?;
                let redirected_base_url = redirected_base_url_shared.lock().clone();
                let effective_url = redirect::swap_tails(redirected_base_url.as_deref(), &base_url, url.clone());
                let routing = ProxyRouting::capture(&config);
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
                    let origin =
                        gix_url::parse(req.url().as_str()).or_raise(|| message("Request configuration failed"))?;
                    req.headers_mut().entry(header::AUTHORIZATION).or_insert_with(|| {
                        basic_auth(origin.user().unwrap_or_default(), origin.password().unwrap_or_default())
                    });
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
                    request: ProxyRequest {
                        config: config.clone(),
                        routing: routing.clone(),
                        ..Default::default()
                    },
                };

                if *follow == FollowRedirects::Initial {
                    *follow = FollowRedirects::None;
                }

                if let Err(err) = redirects.lock().request.prepare(req.url()) {
                    drop(req); // Release a streamed upload before sending its error to the header reader.
                    headers_tx.channel.send(Err(std::io::Error::other(err))).ok();
                    continue 'requests;
                }
                let proxy = redirects.lock().request.proxy.clone();
                if client.as_ref().is_none_or(|(previous_routing, previous_proxy, _)| {
                    previous_routing != &routing || previous_proxy != &proxy
                }) {
                    let new_client = match client_builder
                        .take()
                        .map_or_else(|| create_builder(&config), Ok)
                        .and_then(create_client)
                    {
                        Ok(client) => client,
                        Err(err) => {
                            drop(req);
                            headers_tx.channel.send(Err(std::io::Error::other(err))).ok();
                            continue 'requests;
                        }
                    };
                    client = Some((routing, proxy, new_client));
                }
                let (_, proxy, (http_client, customized)) = client.as_ref().expect("client was initialized above");
                if suppress_client_defaults && *customized {
                    let inserted_proxy_auth = !req.headers().contains_key(header::PROXY_AUTHORIZATION);
                    for name in [
                        header::AUTHORIZATION,
                        header::COOKIE,
                        header::WWW_AUTHENTICATE,
                        header::PROXY_AUTHORIZATION,
                        header::HeaderName::from_static("cookie2"),
                    ] {
                        req.headers_mut()
                            .entry(name)
                            .or_insert(header::HeaderValue::from_static(""));
                    }
                    if inserted_proxy_auth
                        && req.url().scheme() == "http"
                        && let Some(proxy) = proxy
                        && matches!(proxy.scheme, gix_url::Scheme::Http | gix_url::Scheme::Https)
                        && let Some(username) = proxy.user()
                    {
                        req.headers_mut().insert(
                            header::PROXY_AUTHORIZATION,
                            basic_auth(username, proxy.password().unwrap_or_default()),
                        );
                    }
                }
                let response = http_client.execute(req);
                let execute_failed = response.is_err();
                if !execute_failed {
                    suppress_client_defaults |= redirects.lock().request.crossed_authority;
                }
                // Retire stale cores after response delivery and before the next request replaces routing state.
                let _retired_client = if execute_failed || redirects.lock().request.proxy_changed {
                    client.take()
                } else {
                    None
                };
                let proxy_auth_failed = match &response {
                    Ok(res) => res.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED,
                    Err(err) => {
                        // shortcut: reqwest hides CONNECT's status and hyper-util's error type is private; use a typed check when exposed.
                        err.is_connect()
                            && std::iter::successors(err.source(), |&err| err.source())
                                .any(|err| err.to_string() == "tunnel error: proxy authorization required")
                    }
                };
                let authentication = if response.is_ok() || proxy_auth_failed {
                    redirects.lock().request.finish_authentication(proxy_auth_failed)
                } else {
                    Ok(())
                };
                if let Err(err) = authentication {
                    headers_tx.channel.send(Err(std::io::Error::other(err))).ok();
                    if execute_failed {
                        suppress_client_defaults |= redirects.lock().request.crossed_authority;
                    }
                    continue 'requests;
                }

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
                                let kind = if status.is_server_error() && status != reqwest::StatusCode::NOT_IMPLEMENTED
                                {
                                    std::io::ErrorKind::ConnectionAborted
                                } else {
                                    std::io::ErrorKind::Other
                                };
                                std::io::Error::new(
                                    kind,
                                    classify_reqwest(err)
                                        .and_raise(message!("Received HTTP status {}", status.as_str())),
                                )
                            }
                            // Preserve the `reqwest::Error` as the source so the underlying cause -- e.g. a
                            // connection or TLS failure -- isn't lost. It was previously stringified, which
                            // dead-ended `source()` and hid the real reason a request failed. See #2140.
                            None => std::io::Error::other(classify_reqwest(err)),
                        };
                        headers_tx.channel.send(Err(err)).ok();
                        if execute_failed {
                            suppress_client_defaults |= redirects.lock().request.crossed_authority;
                        }
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
        }));

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
    fn restore_thread_after_failure(&mut self) -> Error {
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
        err_that_brought_thread_down.and_raise(message("Could not initialize the http client"))
    }

    fn make_request(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        upload_body_kind: Option<PostBodyDataKind>,
    ) -> Result<http::PostResponse<pipe::Reader, pipe::Reader, pipe::Writer>> {
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

fn proxy_url(config: &http::Options, routing: &ProxyRouting, scheme: &str) -> Result<Option<gix_url::Url>> {
    let proxy = routing.proxy.clone().or_else(|| {
        if scheme == "https" {
            routing.https_proxy.clone()
        } else {
            routing.http_proxy.clone()
        }
        .or_else(|| routing.all_proxy.clone())
    });
    let Some(mut proxy) = proxy.filter(|proxy| !proxy.is_empty()) else {
        return Ok(None);
    };
    if !proxy.contains("://") {
        proxy.insert_str(0, "http://");
    }
    let mut proxy = gix_url::parse(proxy.as_str()).or_raise(|| message("Invalid proxy URL"))?;
    if proxy.scheme.as_str() == "socks" {
        proxy.scheme = "socks4".into();
    }
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
    ) -> Result<http::GetResponse<Self::Headers, Self::ResponseBody>> {
        self.make_request(url, base_url, headers, None).map(Into::into)
    }

    fn post(
        &mut self,
        url: &str,
        base_url: &str,
        headers: impl IntoIterator<Item = impl AsRef<str>>,
        post_body_kind: PostBodyDataKind,
    ) -> Result<http::PostResponse<Self::Headers, Self::ResponseBody, Self::PostBody>> {
        self.make_request(url, base_url, headers, Some(post_body_kind))
    }

    fn configure(&mut self, config: &dyn Any) -> Result {
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
                &super::ProxyRouting {
                    proxy: Some(input.into()),
                    ..Default::default()
                },
                "http",
            )?
            .expect("the proxy is configured");
            assert_eq!(proxy.port, port, "curl-style proxy port for {input}");
        }
        Ok(())
    }
}
