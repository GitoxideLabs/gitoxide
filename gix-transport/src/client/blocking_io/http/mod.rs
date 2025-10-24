use std::{
    any::Any,
    borrow::Cow,
    io::{BufRead, Read},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use base64::Engine;
use bstr::BStr;
pub use traits::{Error, GetResponse, Http, PostBodyDataKind, PostResponse};

use crate::{
    client::{
        self,
        blocking_io::{
            self,
            bufread_ext::ReadlineBufRead,
            http::options::{HttpVersion, SslVersionRangeInclusive},
            ExtendedBufRead, HandleProgress, RequestWriter, SetServiceResponse,
        },
        capabilities, Capabilities, MessageKind,
    },
    packetline::{blocking_io::StreamingPeekableIter, PacketLineRef},
    Protocol, Service,
};

#[cfg(feature = "http-client-curl")]
///
pub mod curl;

/// The experimental `reqwest` backend.
///
/// It doesn't support any of the shared http options yet, but can be seen as example on how to integrate blocking `http` backends.
/// There is also nothing that would prevent it from becoming a fully-featured HTTP backend except for demand and time.
#[cfg(feature = "http-client-reqwest")]
pub mod reqwest;

mod traits;

///
pub mod options {
    /// A function to authenticate a URL.
    pub type AuthenticateFn =
        dyn FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + Send + Sync;

    /// Possible settings for the `http.followRedirects` configuration option.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub enum FollowRedirects {
        /// Follow only the first redirect request, most suitable for typical git requests.
        #[default]
        Initial,
        /// Follow all redirect requests from the server unconditionally
        All,
        /// Follow no redirect request.
        None,
    }

    /// The way to configure a proxy for authentication if a username is present in the configured proxy.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub enum ProxyAuthMethod {
        /// Automatically pick a suitable authentication method.
        #[default]
        AnyAuth,
        ///HTTP basic authentication.
        Basic,
        /// Http digest authentication to prevent a password to be passed in clear text.
        Digest,
        /// GSS negotiate authentication.
        Negotiate,
        /// NTLM authentication
        Ntlm,
    }

    /// Available SSL version numbers.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
    #[allow(missing_docs)]
    pub enum SslVersion {
        /// The implementation default, which is unknown to this layer of abstraction.
        Default,
        TlsV1,
        SslV2,
        SslV3,
        TlsV1_0,
        TlsV1_1,
        TlsV1_2,
        TlsV1_3,
    }

    /// Available HTTP version numbers.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
    #[allow(missing_docs)]
    pub enum HttpVersion {
        /// Equivalent to HTTP/1.1
        V1_1,
        /// Equivalent to HTTP/2
        V2,
    }

    /// The desired range of acceptable SSL versions, or the single version to allow if both are set to the same value.
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SslVersionRangeInclusive {
        /// The smallest allowed ssl version to use.
        pub min: SslVersion,
        /// The highest allowed ssl version to use.
        pub max: SslVersion,
    }

    impl SslVersionRangeInclusive {
        /// Return `min` and `max` fields in the right order so `min` is smaller or equal to `max`.
        pub fn min_max(&self) -> (SslVersion, SslVersion) {
            if self.min > self.max {
                (self.max, self.min)
            } else {
                (self.min, self.max)
            }
        }
    }
}

/// Options to configure http requests.
// TODO: testing most of these fields requires a lot of effort, unless special flags to introspect ongoing requests are added.
#[derive(Clone)]
pub struct Options {
    /// Headers to be added to every request.
    /// They are applied unconditionally and are expected to be valid as they occur in an HTTP request, like `header: value`, without newlines.
    ///
    /// Refers to `http.extraHeader` multi-var.
    pub extra_headers: Vec<String>,
    /// How to handle redirects.
    ///
    /// Refers to `http.followRedirects`.
    pub follow_redirects: options::FollowRedirects,
    /// Used in conjunction with `low_speed_time_seconds`, any non-0 value signals the amount of bytes per second at least to avoid
    /// aborting the connection.
    ///
    /// Refers to `http.lowSpeedLimit`.
    pub low_speed_limit_bytes_per_second: u32,
    /// Used in conjunction with `low_speed_bytes_per_second`, any non-0 value signals the amount seconds the minimal amount
    /// of bytes per second isn't reached.
    ///
    /// Refers to `http.lowSpeedTime`.
    pub low_speed_time_seconds: u64,
    /// A curl-style proxy declaration of the form `[protocol://][user[:password]@]proxyhost[:port]`.
    ///
    /// Note that an empty string means the proxy is disabled entirely.
    /// Refers to `http.proxy`.
    pub proxy: Option<String>,
    /// The comma-separated list of hosts to not send through the `proxy`, or `*` to entirely disable all proxying.
    pub no_proxy: Option<String>,
    /// The way to authenticate against the proxy if the `proxy` field contains a username.
    ///
    /// Refers to `http.proxyAuthMethod`.
    pub proxy_auth_method: options::ProxyAuthMethod,
    /// If authentication is needed for the proxy as its URL contains a username, this method must be set to provide a password
    /// for it before making the request, and to store it if the connection succeeds.
    pub proxy_authenticate: Option<(
        gix_credentials::helper::Action,
        Arc<std::sync::Mutex<options::AuthenticateFn>>,
    )>,
    /// The `HTTP` `USER_AGENT` string presented to an `HTTP` server, notably not the user agent present to the `git` server.
    ///
    /// If not overridden, it defaults to the user agent provided by `curl`, which is a deviation from how `git` handles this.
    /// Thus it's expected from the callers to set it to their application, or use higher-level crates which make it easy to do this
    /// more correctly.
    ///
    /// Using the correct user-agent might affect how the server treats the request.
    ///
    /// Refers to `http.userAgent`.
    pub user_agent: Option<String>,
    /// The amount of time we wait until aborting a connection attempt.
    ///
    /// If `None`, this typically defaults to 2 minutes to 5 minutes.
    /// Refers to `gitoxide.http.connectTimeout`.
    pub connect_timeout: Option<std::time::Duration>,
    /// If enabled, emit additional information about connections and possibly the data received or written.
    pub verbose: bool,
    /// If set, use this path to point to a file with CA certificates to verify peers.
    pub ssl_ca_info: Option<PathBuf>,
    /// The SSL version or version range to use, or `None` to let the TLS backend determine which versions are acceptable.
    pub ssl_version: Option<SslVersionRangeInclusive>,
    /// Controls whether to perform SSL identity verification or not. Turning this off is not recommended and can lead to
    /// various security risks. An example where this may be needed is when an internal git server uses a self-signed
    /// certificate and the user accepts the associated security risks.
    pub ssl_verify: bool,
    /// The HTTP version to enforce. If unset, it is implementation defined.
    pub http_version: Option<HttpVersion>,
    /// Backend specific options, if available.
    pub backend: Option<Arc<Mutex<dyn Any + Send + Sync + 'static>>>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            extra_headers: vec![],
            follow_redirects: Default::default(),
            low_speed_limit_bytes_per_second: 0,
            low_speed_time_seconds: 0,
            proxy: None,
            no_proxy: None,
            proxy_auth_method: Default::default(),
            proxy_authenticate: None,
            user_agent: None,
            connect_timeout: None,
            verbose: false,
            ssl_ca_info: None,
            ssl_version: None,
            ssl_verify: true,
            http_version: None,
            backend: None,
        }
    }
}

/// A transport for supporting arbitrary http clients by abstracting interactions with them into the [Http] trait.
pub struct Transport<H: Http> {
    url: String,
    user_agent_header: &'static str,
    desired_version: Protocol,
    actual_version: Protocol,
    http: H,
    service: Option<Service>,
    line_provider: Option<StreamingPeekableIter<H::ResponseBody>>,
    identity: Option<gix_sec::identity::Account>,
    trace: bool,
}

impl<H: Http> Transport<H> {
    /// Create a new instance with `http` as implementation to communicate to `url` using the given `desired_version`.
    /// Note that we will always fallback to other versions as supported by the server.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    pub fn new_http(http: H, url: gix_url::Url, desired_version: Protocol, trace: bool) -> Self {
        let identity = url
            .user()
            .zip(url.password())
            .map(|(user, pass)| gix_sec::identity::Account {
                username: user.to_string(),
                password: pass.to_string(),
                oauth_refresh_token: None,
            });
        Transport {
            url: url.to_bstring().to_string(),
            user_agent_header: concat!("User-Agent: git/oxide-", env!("CARGO_PKG_VERSION")),
            desired_version,
            actual_version: Default::default(),
            service: None,
            http,
            line_provider: None,
            identity,
            trace,
        }
    }
}

impl<H: Http> Transport<H> {
    /// Returns the identity that the transport uses when connecting to the remote.
    pub fn identity(&self) -> Option<&gix_sec::identity::Account> {
        self.identity.as_ref()
    }
}

#[cfg(any(feature = "http-client-curl", feature = "http-client-reqwest"))]
impl<H: Http + Default> Transport<H> {
    /// Create a new instance to communicate to `url` using the given `desired_version` of the `git` protocol.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    ///
    /// Note that the actual implementation depends on feature toggles.
    pub fn new(url: gix_url::Url, desired_version: Protocol, trace: bool) -> Self {
        Self::new_http(H::default(), url, desired_version, trace)
    }
}

impl<H: Http> Transport<H> {
    fn check_content_type(service: Service, kind: &str, headers: <H as Http>::Headers) -> Result<(), client::Error> {
        let wanted_content_type = format!("application/x-{}-{}", service.as_str(), kind);
        if !headers.lines().collect::<Result<Vec<_>, _>>()?.iter().any(|l| {
            let mut tokens = l.split(':');
            tokens.next().zip(tokens.next()).is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("content-type") && value.trim() == wanted_content_type
            })
        }) {
            return Err(client::Error::Http(Error::Detail {
                description: format!(
                    "Didn't find '{wanted_content_type}' header to indicate 'smart' protocol, and 'dumb' protocol is not supported."
                ),
            }));
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps, unknown_lints)]
    fn add_basic_auth_if_present(&self, headers: &mut Vec<Cow<'_, str>>) -> Result<(), client::Error> {
        if let Some(gix_sec::identity::Account {
            username,
            password,
            oauth_refresh_token: _,
        }) = &self.identity
        {
            #[cfg(not(feature = "http-client-insecure-credentials"))]
            if self.url.starts_with("http://") {
                return Err(client::Error::AuthenticationRefused(
                    "Will not send credentials in clear text over http",
                ));
            }
            headers.push(Cow::Owned(format!(
                "Authorization: Basic {}",
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
            )));
        }
        Ok(())
    }
}

fn append_url(base: &str, suffix: &str) -> String {
    let mut buf = base.to_owned();
    if base.as_bytes().last() != Some(&b'/') {
        buf.push('/');
    }
    buf.push_str(suffix);
    buf
}

impl<H: Http> client::TransportWithoutIO for Transport<H> {
    fn set_identity(&mut self, identity: gix_sec::identity::Account) -> Result<(), client::Error> {
        self.identity = Some(identity);
        Ok(())
    }

    fn request(
        &mut self,
        write_mode: client::WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Result<RequestWriter<'_>, client::Error> {
        let service = self.service.ok_or(client::Error::MissingHandshake)?;
        let url = append_url(&self.url, service.as_str());
        let static_headers = &[
            Cow::Borrowed(self.user_agent_header),
            Cow::Owned(format!("Content-Type: application/x-{}-request", service.as_str())),
            format!("Accept: application/x-{}-result", service.as_str()).into(),
        ];
        let mut dynamic_headers = Vec::new();
        self.add_basic_auth_if_present(&mut dynamic_headers)?;
        if self.actual_version != Protocol::V1 {
            dynamic_headers.push(Cow::Owned(format!(
                "Git-Protocol: version={}",
                self.actual_version as usize
            )));
        }

        let PostResponse {
            headers,
            body,
            post_body,
        } = self.http.post(
            &url,
            &self.url,
            static_headers.iter().chain(&dynamic_headers),
            write_mode.into(),
        )?;
        let line_provider = self
            .line_provider
            .as_mut()
            .expect("handshake to have been called first");
        line_provider.replace(body);
        Ok(RequestWriter::new_from_bufread(
            post_body,
            Box::new(HeadersThenBody::<H, _> {
                service,
                headers: Some(headers),
                body: line_provider.as_read_without_sidebands(),
            }),
            write_mode,
            on_into_read,
            trace,
        ))
    }

    fn to_url(&self) -> Cow<'_, BStr> {
        Cow::Borrowed(self.url.as_str().into())
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        false
    }

    fn configure(&mut self, config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.http.configure(config)
    }
}

impl<H: Http> blocking_io::Transport for Transport<H> {
    fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, client::Error> {
        let url = append_url(self.url.as_ref(), &format!("info/refs?service={}", service.as_str()));
        let static_headers = [Cow::Borrowed(self.user_agent_header)];
        let mut dynamic_headers = Vec::<Cow<'_, str>>::new();
        if self.desired_version != Protocol::V1 || !extra_parameters.is_empty() {
            let mut parameters = if self.desired_version != Protocol::V1 {
                let mut p = format!("version={}", self.desired_version as usize);
                if !extra_parameters.is_empty() {
                    p.push(':');
                }
                p
            } else {
                String::new()
            };
            parameters.push_str(
                &extra_parameters
                    .iter()
                    .map(|(key, value)| match value {
                        Some(value) => format!("{key}={value}"),
                        None => key.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(":"),
            );
            dynamic_headers.push(format!("Git-Protocol: {parameters}").into());
        }
        self.add_basic_auth_if_present(&mut dynamic_headers)?;
        let GetResponse { headers, body } =
            self.http
                .get(url.as_ref(), &self.url, static_headers.iter().chain(&dynamic_headers))?;
        <Transport<H>>::check_content_type(service, "advertisement", headers)?;

        let line_reader = self
            .line_provider
            .get_or_insert_with(|| StreamingPeekableIter::new(body, &[PacketLineRef::Flush], self.trace));

        // the service announcement is only sent sometimes depending on the exact server/protocol version/used protocol (http?)
        // eat the announcement when its there to avoid errors later (and check that the correct service was announced).
        // Ignore the announcement otherwise.
        let line_ = line_reader
            .peek_line()
            .ok_or(client::Error::ExpectedLine("capabilities, version or service"))???;
        let line = line_.as_text().ok_or(client::Error::ExpectedLine("text"))?;

        if let Some(announced_service) = line.as_bstr().strip_prefix(b"# service=") {
            if announced_service != service.as_str().as_bytes() {
                return Err(client::Error::Http(Error::Detail {
                    description: format!(
                        "Expected to see service {:?}, but got {:?}",
                        service.as_str(),
                        announced_service
                    ),
                }));
            }

            line_reader.as_read().read_to_end(&mut Vec::new())?;
        }

        let capabilities::recv::Outcome {
            capabilities,
            refs,
            protocol: actual_protocol,
        } = Capabilities::from_lines_with_version_detection(line_reader)?;
        self.actual_version = actual_protocol;
        self.service = Some(service);
        Ok(SetServiceResponse {
            actual_protocol,
            capabilities,
            refs,
        })
    }
}

struct HeadersThenBody<H: Http, B: Unpin> {
    service: Service,
    headers: Option<H::Headers>,
    body: B,
}

impl<H: Http, B: Unpin> HeadersThenBody<H, B> {
    fn handle_headers(&mut self) -> std::io::Result<()> {
        if let Some(headers) = self.headers.take() {
            <Transport<H>>::check_content_type(self.service, "result", headers).map_err(std::io::Error::other)?;
        }
        Ok(())
    }
}

impl<H: Http, B: Read + Unpin> Read for HeadersThenBody<H, B> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle_headers()?;
        self.body.read(buf)
    }
}

impl<H: Http, B: BufRead + Unpin> BufRead for HeadersThenBody<H, B> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.handle_headers()?;
        self.body.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.body.consume(amt);
    }
}

impl<H: Http, B: ReadlineBufRead + Unpin> ReadlineBufRead for HeadersThenBody<H, B> {
    fn readline(&mut self) -> Option<std::io::Result<Result<PacketLineRef<'_>, gix_packetline::decode::Error>>> {
        if let Err(err) = self.handle_headers() {
            return Some(Err(err));
        }
        self.body.readline()
    }

    fn readline_str(&mut self, line: &mut String) -> std::io::Result<usize> {
        self.handle_headers()?;
        self.body.readline_str(line)
    }
}

impl<'a, H: Http, B: ExtendedBufRead<'a> + Unpin> ExtendedBufRead<'a> for HeadersThenBody<H, B> {
    fn set_progress_handler(&mut self, handle_progress: Option<HandleProgress<'a>>) {
        self.body.set_progress_handler(handle_progress);
    }

    fn peek_data_line(&mut self) -> Option<std::io::Result<Result<&[u8], client::Error>>> {
        if let Err(err) = self.handle_headers() {
            return Some(Err(err));
        }
        self.body.peek_data_line()
    }

    fn reset(&mut self, version: Protocol) {
        self.body.reset(version);
    }

    fn stopped_at(&self) -> Option<MessageKind> {
        self.body.stopped_at()
    }
}

/// Connect to the given `url` via HTTP/S using the `desired_version` of the `git` protocol, with `http` as implementation.
/// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
#[cfg(all(feature = "http-client", not(feature = "http-client-curl")))]
pub fn connect_http<H: Http>(http: H, url: gix_url::Url, desired_version: Protocol, trace: bool) -> Transport<H> {
    Transport::new_http(http, url, desired_version, trace)
}

/// Connect to the given `url` via HTTP/S using the `desired_version` of the `git` protocol.
/// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
#[cfg(any(feature = "http-client-curl", feature = "http-client-reqwest"))]
pub fn connect<H: Http + Default>(url: gix_url::Url, desired_version: Protocol, trace: bool) -> Transport<H> {
    Transport::new(url, desired_version, trace)
}

///
#[cfg(any(feature = "http-client-curl", feature = "http-client-reqwest"))]
pub mod redirect;
