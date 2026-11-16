#[cfg(any(feature = "__native-tls", feature = "__rustls",))]
use std::any::Any;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use std::time::Duration;
use std::{collections::HashMap, convert::TryInto, net::SocketAddr};
use std::{fmt, str};

use super::request::{Request, RequestBuilder};
use super::response::Response;
use super::Body;
#[cfg(feature = "http3")]
use crate::async_impl::h3_client::connect::{H3ClientConfig, H3Connector};
#[cfg(feature = "http3")]
use crate::async_impl::h3_client::H3Client;
use crate::config::{RequestConfig, TotalTimeout};
#[cfg(unix)]
use crate::connect::uds::UnixSocketProvider;
#[cfg(target_os = "windows")]
use crate::connect::windows_named_pipe::WindowsNamedPipeProvider;
use crate::connect::{
    sealed::{Conn, Unnameable},
    BoxedConnectorLayer, BoxedConnectorService, Connector, ConnectorBuilder,
};
#[cfg(feature = "cookies")]
use crate::cookie;
#[cfg(feature = "cookies")]
use crate::cookie::service::CookieService;
#[cfg(feature = "hickory-dns")]
use crate::dns::hickory::HickoryDnsResolver;
use crate::dns::{gai::GaiResolver, DnsResolverWithOverrides, DynResolver, Resolve};
use crate::error::{self, BoxError};
use crate::into_url::try_uri;
use crate::proxy::Matcher as ProxyMatcher;
use crate::redirect::{self, TowerRedirectPolicy};
#[cfg(feature = "__rustls")]
use crate::tls::CertificateRevocationList;
#[cfg(feature = "__tls")]
use crate::tls::{self, TlsBackend};
#[cfg(feature = "__tls")]
use crate::Certificate;
#[cfg(any(feature = "__native-tls", feature = "__rustls"))]
use crate::Identity;
use crate::{IntoUrl, Method, Proxy, Url};

use http::header::{Entry, HeaderMap, HeaderValue, ACCEPT, PROXY_AUTHORIZATION, USER_AGENT};
use http::uri::Scheme;
use http::Uri;
use hyper_util::client::legacy::connect::HttpConnector;
#[cfg(feature = "__native-tls")]
use native_tls_crate::TlsConnector;
use pin_project_lite::pin_project;
#[cfg(feature = "http3")]
use quinn::TransportConfig;
#[cfg(feature = "http3")]
use quinn::VarInt;
use tokio::time::Sleep;
use tower::util::BoxCloneSyncServiceLayer;
use tower::{Layer, Service};
#[cfg(any(
    feature = "gzip",
    feature = "brotli",
    feature = "zstd",
    feature = "deflate"
))]
use tower_http::decompression::Decompression;
use tower_http::follow_redirect::FollowRedirect;

/// An asynchronous `Client` to make Requests with.
///
/// The Client has various configuration values to tweak, but the defaults
/// are set to what is usually the most commonly desired value. To configure a
/// `Client`, use `Client::builder()`.
///
/// The `Client` holds a connection pool internally to improve performance
/// by reusing connections and avoiding setup overhead, so it is advised that
/// you create one and **reuse** it.
///
/// You do **not** have to wrap the `Client` in an [`Rc`] or [`Arc`] to **reuse** it,
/// because it already uses an [`Arc`] internally.
///
/// # Connection Pooling
///
/// The connection pool can be configured using [`ClientBuilder`] methods
/// with the `pool_` prefix, such as [`ClientBuilder::pool_idle_timeout`]
/// and [`ClientBuilder::pool_max_idle_per_host`].
///
/// [`Rc`]: std::rc::Rc
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientRef>,
}

/// A `ClientBuilder` can be used to create a `Client` with custom configuration.
#[must_use]
pub struct ClientBuilder {
    config: Config,
}

enum HttpVersionPref {
    Http1,
    #[cfg(feature = "http2")]
    Http2,
    #[cfg(feature = "http3")]
    Http3,
    All,
}

#[derive(Clone, Copy, Debug)]
struct Accepts {
    #[cfg(feature = "gzip")]
    gzip: bool,
    #[cfg(feature = "brotli")]
    brotli: bool,
    #[cfg(feature = "zstd")]
    zstd: bool,
    #[cfg(feature = "deflate")]
    deflate: bool,
}

impl Default for Accepts {
    fn default() -> Accepts {
        Accepts {
            #[cfg(feature = "gzip")]
            gzip: true,
            #[cfg(feature = "brotli")]
            brotli: true,
            #[cfg(feature = "zstd")]
            zstd: true,
            #[cfg(feature = "deflate")]
            deflate: true,
        }
    }
}

#[derive(Clone)]
struct HyperService {
    hyper: HyperClient,
}

impl Service<hyper::Request<crate::async_impl::body::Body>> for HyperService {
    type Error = crate::Error;
    type Response = http::Response<hyper::body::Incoming>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.hyper.poll_ready(cx).map_err(crate::error::request)
    }

    fn call(&mut self, req: hyper::Request<crate::async_impl::body::Body>) -> Self::Future {
        let clone = self.hyper.clone();
        let mut inner = std::mem::replace(&mut self.hyper, clone);
        Box::pin(async move { inner.call(req).await.map_err(crate::error::request) })
    }
}

struct Config {
    // NOTE: When adding a new field, update `fmt::Debug for ClientBuilder`
    accepts: Accepts,
    headers: HeaderMap,
    #[cfg(feature = "__tls")]
    hostname_verification: bool,
    #[cfg(feature = "__tls")]
    certs_verification: bool,
    #[cfg(feature = "__tls")]
    tls_sni: bool,
    #[cfg(feature = "__rustls")]
    tls_sslkeylogfile: bool,
    connect_timeout: Option<Duration>,
    connection_verbose: bool,
    pool_idle_timeout: Option<Duration>,
    pool_max_idle_per_host: usize,
    tcp_keepalive: Option<Duration>,
    tcp_keepalive_interval: Option<Duration>,
    tcp_keepalive_retries: Option<u32>,
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    tcp_user_timeout: Option<Duration>,
    #[cfg(any(feature = "__native-tls", feature = "__rustls"))]
    identity: Option<Identity>,
    proxies: Vec<ProxyMatcher>,
    auto_sys_proxy: bool,
    redirect_policy: redirect::Policy,
    retry_policy: crate::retry::Builder,
    referer: bool,
    read_timeout: Option<Duration>,
    timeout: Option<Duration>,
    #[cfg(feature = "__tls")]
    root_certs: Vec<Certificate>,
    #[cfg(feature = "__tls")]
    tls_certs_only: bool,
    #[cfg(feature = "__rustls")]
    crls: Vec<CertificateRevocationList>,
    #[cfg(feature = "__tls")]
    min_tls_version: Option<tls::Version>,
    #[cfg(feature = "__tls")]
    max_tls_version: Option<tls::Version>,
    #[cfg(feature = "__tls")]
    tls_info: bool,
    #[cfg(feature = "__tls")]
    tls: TlsBackend,
    connector_layers: Vec<BoxedConnectorLayer>,
    http_version_pref: HttpVersionPref,
    http09_responses: bool,
    http1_title_case_headers: bool,
    http1_allow_obsolete_multiline_headers_in_responses: bool,
    http1_ignore_invalid_headers_in_responses: bool,
    http1_allow_spaces_after_header_name_in_responses: bool,
    http1_max_headers: Option<usize>,
    #[cfg(feature = "http2")]
    http2_initial_stream_window_size: Option<u32>,
    #[cfg(feature = "http2")]
    http2_initial_connection_window_size: Option<u32>,
    #[cfg(feature = "http2")]
    http2_adaptive_window: bool,
    #[cfg(feature = "http2")]
    http2_max_frame_size: Option<u32>,
    #[cfg(feature = "http2")]
    http2_max_header_list_size: Option<u32>,
    #[cfg(feature = "http2")]
    http2_header_table_size: Option<u32>,
    #[cfg(feature = "http2")]
    http2_max_concurrent_streams: Option<u32>,
    #[cfg(feature = "http2")]
    http2_keep_alive_interval: Option<Duration>,
    #[cfg(feature = "http2")]
    http2_keep_alive_timeout: Option<Duration>,
    #[cfg(feature = "http2")]
    http2_keep_alive_while_idle: bool,
    local_address: Option<IpAddr>,
    #[cfg(any(
        target_os = "android",
        target_os = "fuchsia",
        target_os = "illumos",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "solaris",
        target_os = "tvos",
        target_os = "visionos",
        target_os = "watchos",
    ))]
    interface: Option<String>,
    nodelay: bool,
    #[cfg(feature = "cookies")]
    cookie_store: Option<Arc<dyn cookie::CookieStore>>,
    hickory_dns: bool,
    error: Option<crate::Error>,
    https_only: bool,
    #[cfg(feature = "http3")]
    tls_enable_early_data: bool,
    #[cfg(feature = "http3")]
    quic_max_idle_timeout: Option<Duration>,
    #[cfg(feature = "http3")]
    quic_stream_receive_window: Option<VarInt>,
    #[cfg(feature = "http3")]
    quic_receive_window: Option<VarInt>,
    #[cfg(feature = "http3")]
    quic_send_window: Option<u64>,
    #[cfg(feature = "http3")]
    quic_congestion_bbr: bool,
    #[cfg(feature = "http3")]
    h3_max_field_section_size: Option<u64>,
    #[cfg(feature = "http3")]
    h3_send_grease: Option<bool>,
    dns_overrides: HashMap<String, Vec<SocketAddr>>,
    dns_resolver: Option<Arc<dyn Resolve>>,

    #[cfg(unix)]
    unix_socket: Option<Arc<std::path::Path>>,
    #[cfg(target_os = "windows")]
    windows_named_pipe: Option<Arc<std::ffi::OsStr>>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Constructs a new `ClientBuilder`.
    ///
    /// This is the same as `Client::builder()`.
    pub fn new() -> Self {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::with_capacity(2);
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

        ClientBuilder {
            config: Config {
                error: None,
                accepts: Accepts::default(),
                headers,
                #[cfg(feature = "__tls")]
                hostname_verification: true,
                #[cfg(feature = "__tls")]
                certs_verification: true,
                #[cfg(feature = "__tls")]
                tls_sni: true,
                #[cfg(feature = "__rustls")]
                tls_sslkeylogfile: false,
                connect_timeout: None,
                connection_verbose: false,
                pool_idle_timeout: Some(Duration::from_secs(90)),
                pool_max_idle_per_host: usize::MAX,
                tcp_keepalive: Some(Duration::from_secs(15)),
                tcp_keepalive_interval: Some(Duration::from_secs(15)),
                tcp_keepalive_retries: Some(3),
                #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
                tcp_user_timeout: Some(Duration::from_secs(30)),
                proxies: Vec::new(),
                auto_sys_proxy: true,
                redirect_policy: redirect::Policy::default(),
                retry_policy: crate::retry::Builder::default(),
                referer: true,
                read_timeout: None,
                timeout: None,
                #[cfg(feature = "__tls")]
                root_certs: Vec::new(),
                #[cfg(feature = "__tls")]
                tls_certs_only: false,
                #[cfg(any(feature = "__native-tls", feature = "__rustls"))]
                identity: None,
                #[cfg(feature = "__rustls")]
                crls: vec![],
                #[cfg(feature = "__tls")]
                min_tls_version: None,
                #[cfg(feature = "__tls")]
                max_tls_version: None,
                #[cfg(feature = "__tls")]
                tls_info: false,
                #[cfg(feature = "__tls")]
                tls: TlsBackend::default(),
                connector_layers: Vec::new(),
                http_version_pref: HttpVersionPref::All,
                http09_responses: false,
                http1_title_case_headers: false,
                http1_allow_obsolete_multiline_headers_in_responses: false,
                http1_ignore_invalid_headers_in_responses: false,
                http1_allow_spaces_after_header_name_in_responses: false,
                http1_max_headers: None,
                #[cfg(feature = "http2")]
                http2_initial_stream_window_size: None,
                #[cfg(feature = "http2")]
                http2_initial_connection_window_size: None,
                #[cfg(feature = "http2")]
                http2_adaptive_window: false,
                #[cfg(feature = "http2")]
                http2_max_frame_size: None,
                #[cfg(feature = "http2")]
                http2_max_header_list_size: None,
                #[cfg(feature = "http2")]
                http2_header_table_size: None,
                #[cfg(feature = "http2")]
                http2_max_concurrent_streams: None,
                #[cfg(feature = "http2")]
                http2_keep_alive_interval: None,
                #[cfg(feature = "http2")]
                http2_keep_alive_timeout: None,
                #[cfg(feature = "http2")]
                http2_keep_alive_while_idle: false,
                local_address: None,
                #[cfg(any(
                    target_os = "android",
                    target_os = "fuchsia",
                    target_os = "illumos",
                    target_os = "ios",
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "solaris",
                    target_os = "tvos",
                    target_os = "visionos",
                    target_os = "watchos",
                ))]
                interface: None,
                nodelay: true,
                hickory_dns: cfg!(feature = "hickory-dns"),
                #[cfg(feature = "cookies")]
                cookie_store: None,
                https_only: false,
                dns_overrides: HashMap::new(),
                #[cfg(feature = "http3")]
                tls_enable_early_data: false,
                #[cfg(feature = "http3")]
                quic_max_idle_timeout: None,
                #[cfg(feature = "http3")]
                quic_stream_receive_window: None,
                #[cfg(feature = "http3")]
                quic_receive_window: None,
                #[cfg(feature = "http3")]
                quic_send_window: None,
                #[cfg(feature = "http3")]
                quic_congestion_bbr: false,
                #[cfg(feature = "http3")]
                h3_max_field_section_size: None,
                #[cfg(feature = "http3")]
                h3_send_grease: None,
                dns_resolver: None,
                #[cfg(unix)]
                unix_socket: None,
                #[cfg(target_os = "windows")]
                windows_named_pipe: None,
            },
        }
    }
}

impl ClientBuilder {
    /// Returns a `Client` that uses this `ClientBuilder` configuration.
    ///
    /// # Errors
    ///
    /// This method fails if a TLS backend cannot be initialized, or the resolver
    /// cannot load the system configuration.
    pub fn build(self) -> crate::Result<Client> {
        let config = self.config;

        if let Some(err) = config.error {
            return Err(err);
        }

        let mut proxies = config.proxies;
        if config.auto_sys_proxy {
            proxies.push(ProxyMatcher::system());
        }
        let proxies = Arc::new(proxies);

        #[allow(unused)]
        #[cfg(feature = "http3")]
        let mut h3_connector = None;

        let resolver = {
            let mut resolver: Arc<dyn Resolve> = match config.hickory_dns {
                false => Arc::new(GaiResolver::new()),
                #[cfg(feature = "hickory-dns")]
                true => Arc::new(HickoryDnsResolver::default()),
                #[cfg(not(feature = "hickory-dns"))]
                true => unreachable!("hickory-dns shouldn't be enabled unless the feature is"),
            };
            if let Some(dns_resolver) = config.dns_resolver {
                resolver = dns_resolver;
            }
            if !config.dns_overrides.is_empty() {
                resolver = Arc::new(DnsResolverWithOverrides::new(
                    resolver,
                    config.dns_overrides,
                ));
            }
            DynResolver::new(resolver)
        };

        let mut connector_builder = {
            #[cfg(feature = "__tls")]
            fn user_agent(headers: &HeaderMap) -> Option<HeaderValue> {
                headers.get(USER_AGENT).cloned()
            }

            let mut http = HttpConnector::new_with_resolver(resolver.clone());
            http.set_connect_timeout(config.connect_timeout);

            #[cfg(all(feature = "http3", feature = "__rustls"))]
            let build_h3_connector =
                |resolver,
                 tls,
                 quic_max_idle_timeout: Option<Duration>,
                 quic_stream_receive_window,
                 quic_receive_window,
                 quic_send_window,
                 quic_congestion_bbr,
                 h3_max_field_section_size,
                 h3_send_grease,
                 local_address,
                 http_version_pref: &HttpVersionPref| {
                    let mut transport_config = TransportConfig::default();

                    if let Some(max_idle_timeout) = quic_max_idle_timeout {
                        transport_config.max_idle_timeout(Some(
                            max_idle_timeout.try_into().map_err(error::builder)?,
                        ));
                    }

                    if let Some(stream_receive_window) = quic_stream_receive_window {
                        transport_config.stream_receive_window(stream_receive_window);
                    }

                    if let Some(receive_window) = quic_receive_window {
                        transport_config.receive_window(receive_window);
                    }

                    if let Some(send_window) = quic_send_window {
                        transport_config.send_window(send_window);
                    }

                    if quic_congestion_bbr {
                        let factory = Arc::new(quinn::congestion::BbrConfig::default());
                        transport_config.congestion_controller_factory(factory);
                    }

                    let mut h3_client_config = H3ClientConfig::default();

                    if let Some(max_field_section_size) = h3_max_field_section_size {
                        h3_client_config.max_field_section_size = Some(max_field_section_size);
                    }

                    if let Some(send_grease) = h3_send_grease {
                        h3_client_config.send_grease = Some(send_grease);
                    }

                    let res = H3Connector::new(
                        resolver,
                        tls,
                        local_address,
                        transport_config,
                        h3_client_config,
                    );

                    match res {
                        Ok(connector) => Ok(Some(connector)),
                        Err(err) => {
                            if let HttpVersionPref::Http3 = http_version_pref {
                                Err(error::builder(err))
                            } else {
                                Ok(None)
                            }
                        }
                    }
                };

            #[cfg(feature = "__tls")]
            match config.tls {
                #[cfg(feature = "__native-tls")]
                TlsBackend::NativeTls => {
                    let mut tls = TlsConnector::builder();

                    #[cfg(all(feature = "__native-tls-alpn", not(feature = "http3")))]
                    {
                        match config.http_version_pref {
                            HttpVersionPref::Http1 => {
                                tls.request_alpns(&["http/1.1"]);
                            }
                            #[cfg(feature = "http2")]
                            HttpVersionPref::Http2 => {
                                tls.request_alpns(&["h2"]);
                            }
                            HttpVersionPref::All => {
                                tls.request_alpns(&[
                                    #[cfg(feature = "http2")]
                                    "h2",
                                    "http/1.1",
                                ]);
                            }
                        }
                    }

                    tls.danger_accept_invalid_hostnames(!config.hostname_verification);

                    tls.danger_accept_invalid_certs(!config.certs_verification);

                    tls.use_sni(config.tls_sni);

                    tls.disable_built_in_roots(config.tls_certs_only);

                    for cert in config.root_certs {
                        cert.add_to_native_tls(&mut tls);
                    }

                    #[cfg(feature = "__native-tls")]
                    {
                        if let Some(id) = config.identity {
                            id.add_to_native_tls(&mut tls)?;
                        }
                    }
                    #[cfg(all(feature = "__rustls", not(feature = "__native-tls")))]
                    {
                        // Default backend + rustls Identity doesn't work.
                        if let Some(_id) = config.identity {
                            return Err(crate::error::builder("incompatible TLS identity type"));
                        }
                    }

                    if let Some(min_tls_version) = config.min_tls_version {
                        let protocol = min_tls_version.to_native_tls().ok_or_else(|| {
                            // native-tls added support for TLS v1.3 in 0.2.16 🎉
                            // `to_native_tls` could arguably return the value directly
                            // instead of making us check for an impossible None here,
                            // but given that 1.4 does not exist yet, that might get
                            // messy in the future.
                            crate::error::builder("invalid minimum TLS version for backend")
                        })?;
                        tls.min_protocol_version(Some(protocol));
                    }

                    if let Some(max_tls_version) = config.max_tls_version {
                        let protocol = max_tls_version.to_native_tls().ok_or_else(|| {
                            // We could arguably do max_protocol_version(None), given
                            // that 1.4 does not exist yet, but that'd get messy in the
                            // future.
                            crate::error::builder("invalid maximum TLS version for backend")
                        })?;
                        tls.max_protocol_version(Some(protocol));
                    }

                    ConnectorBuilder::new_native_tls(
                        http,
                        tls,
                        proxies.clone(),
                        user_agent(&config.headers),
                        config.local_address,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "illumos",
                            target_os = "ios",
                            target_os = "linux",
                            target_os = "macos",
                            target_os = "solaris",
                            target_os = "tvos",
                            target_os = "visionos",
                            target_os = "watchos",
                        ))]
                        config.interface.as_deref(),
                        config.nodelay,
                        config.tls_info,
                    )?
                }
                #[cfg(feature = "__native-tls")]
                TlsBackend::BuiltNativeTls(conn) => ConnectorBuilder::from_built_native_tls(
                    http,
                    conn,
                    proxies.clone(),
                    user_agent(&config.headers),
                    config.local_address,
                    #[cfg(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "illumos",
                        target_os = "ios",
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "solaris",
                        target_os = "tvos",
                        target_os = "visionos",
                        target_os = "watchos",
                    ))]
                    config.interface.as_deref(),
                    config.nodelay,
                    config.tls_info,
                ),
                #[cfg(feature = "__rustls")]
                TlsBackend::BuiltRustls(conn) => {
                    #[cfg(feature = "http3")]
                    {
                        let mut h3_tls = conn.clone();
                        h3_tls.alpn_protocols = vec!["h3".into()];

                        h3_connector = build_h3_connector(
                            resolver.clone(),
                            h3_tls,
                            config.quic_max_idle_timeout,
                            config.quic_stream_receive_window,
                            config.quic_receive_window,
                            config.quic_send_window,
                            config.quic_congestion_bbr,
                            config.h3_max_field_section_size,
                            config.h3_send_grease,
                            config.local_address,
                            &config.http_version_pref,
                        )?;
                    }

                    ConnectorBuilder::new_rustls_tls(
                        http,
                        conn,
                        proxies.clone(),
                        user_agent(&config.headers),
                        config.local_address,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "illumos",
                            target_os = "ios",
                            target_os = "linux",
                            target_os = "macos",
                            target_os = "solaris",
                            target_os = "tvos",
                            target_os = "visionos",
                            target_os = "watchos",
                        ))]
                        config.interface.as_deref(),
                        config.nodelay,
                        config.tls_info,
                    )
                }
                #[cfg(feature = "__rustls")]
                TlsBackend::Rustls => {
                    use crate::tls::{IgnoreHostname, NoVerifier};

                    // Set TLS versions.
                    let mut versions = rustls::ALL_VERSIONS.to_vec();

                    if let Some(min_tls_version) = config.min_tls_version {
                        versions.retain(|&supported_version| {
                            match tls::Version::from_rustls(supported_version.version) {
                                Some(version) => version >= min_tls_version,
                                // Assume it's so new we don't know about it, allow it
                                // (as of writing this is unreachable)
                                None => true,
                            }
                        });
                    }

                    if let Some(max_tls_version) = config.max_tls_version {
                        versions.retain(|&supported_version| {
                            match tls::Version::from_rustls(supported_version.version) {
                                Some(version) => version <= max_tls_version,
                                None => false,
                            }
                        });
                    }

                    if versions.is_empty() {
                        return Err(crate::error::builder("empty supported tls versions"));
                    }

                    // Allow user to have installed a runtime default.
                    // If not, we ship with _our_ recommended default.
                    let provider = rustls::crypto::CryptoProvider::get_default()
                        .map(|arc| arc.clone())
                        .unwrap_or_else(default_rustls_crypto_provider);

                    // Build TLS config
                    let signature_algorithms = provider.signature_verification_algorithms;
                    let config_builder =
                        rustls::ClientConfig::builder_with_provider(provider.clone())
                            .with_protocol_versions(&versions)
                            .map_err(|_| crate::error::builder("invalid TLS versions"))?;

                    let config_builder = if !config.certs_verification {
                        config_builder
                            .dangerous()
                            .with_custom_certificate_verifier(Arc::new(NoVerifier))
                    } else if !config.hostname_verification {
                        if !config.tls_certs_only {
                            // Should this just warn? Error for now...
                            return Err(crate::error::builder(
                                    "disabling rustls hostname verification only allowed with tls_certs_only()"
                            ));
                        }

                        config_builder
                            .dangerous()
                            .with_custom_certificate_verifier(Arc::new(IgnoreHostname::new(
                                crate::tls::rustls_store(config.root_certs)?,
                                signature_algorithms,
                            )))
                    } else if !config.tls_certs_only {
                        // Check for some misconfigurations and report them.
                        if !config.crls.is_empty() {
                            return Err(crate::error::builder(
                                "CRLs only allowed with tls_certs_only()",
                            ));
                        }

                        let verifier = if config.root_certs.is_empty() {
                            rustls_platform_verifier::Verifier::new(provider)
                                .map_err(crate::error::builder)?
                        } else {
                            #[cfg(any(
                                all(unix, not(target_os = "android")),
                                target_os = "windows"
                            ))]
                            {
                                rustls_platform_verifier::Verifier::new_with_extra_roots(
                                    crate::tls::rustls_der(config.root_certs)?,
                                    provider,
                                )
                                .map_err(crate::error::builder)?
                            }

                            #[cfg(not(any(
                                all(unix, not(target_os = "android")),
                                target_os = "windows"
                            )))]
                            return Err(crate::error::builder(
                                "rustls-platform-verifier could not load extra certs",
                            ));
                        };

                        config_builder
                            .dangerous()
                            .with_custom_certificate_verifier(Arc::new(verifier))
                    } else {
                        if config.crls.is_empty() {
                            config_builder.with_root_certificates(crate::tls::rustls_store(
                                config.root_certs,
                            )?)
                        } else {
                            let crls = config
                                .crls
                                .iter()
                                .map(|e| e.as_rustls_crl())
                                .collect::<Vec<_>>();
                            let verifier =
                                rustls::client::WebPkiServerVerifier::builder_with_provider(
                                    Arc::new(crate::tls::rustls_store(config.root_certs)?),
                                    provider,
                                )
                                .with_crls(crls)
                                .build()
                                .map_err(|_| {
                                    crate::error::builder("invalid TLS verification settings")
                                })?;
                            config_builder.with_webpki_verifier(verifier)
                        }
                    };

                    // Finalize TLS config
                    let mut tls = if let Some(id) = config.identity {
                        id.add_to_rustls(config_builder)?
                    } else {
                        config_builder.with_no_client_auth()
                    };

                    tls.enable_sni = config.tls_sni;

                    if config.tls_sslkeylogfile {
                        tls.key_log = Arc::new(rustls::KeyLogFile::new());
                    }

                    // ALPN protocol
                    match config.http_version_pref {
                        HttpVersionPref::Http1 => {
                            tls.alpn_protocols = vec!["http/1.1".into()];
                        }
                        #[cfg(feature = "http2")]
                        HttpVersionPref::Http2 => {
                            tls.alpn_protocols = vec!["h2".into()];
                        }
                        #[cfg(feature = "http3")]
                        HttpVersionPref::Http3 => {
                            // h3 ALPN is not valid over TCP
                        }
                        HttpVersionPref::All => {
                            tls.alpn_protocols = vec![
                                #[cfg(feature = "http2")]
                                "h2".into(),
                                "http/1.1".into(),
                            ];
                        }
                    }

                    #[cfg(feature = "http3")]
                    {
                        let mut h3_tls = tls.clone();
                        h3_tls.enable_early_data = config.tls_enable_early_data;

                        // h3 ALPN is required over QUIC for HTTP/3
                        h3_tls.alpn_protocols = vec!["h3".into()];

                        h3_connector = build_h3_connector(
                            resolver.clone(),
                            h3_tls,
                            config.quic_max_idle_timeout,
                            config.quic_stream_receive_window,
                            config.quic_receive_window,
                            config.quic_send_window,
                            config.quic_congestion_bbr,
                            config.h3_max_field_section_size,
                            config.h3_send_grease,
                            config.local_address,
                            &config.http_version_pref,
                        )?;
                    }

                    ConnectorBuilder::new_rustls_tls(
                        http,
                        tls,
                        proxies.clone(),
                        user_agent(&config.headers),
                        config.local_address,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "illumos",
                            target_os = "ios",
                            target_os = "linux",
                            target_os = "macos",
                            target_os = "solaris",
                            target_os = "tvos",
                            target_os = "visionos",
                            target_os = "watchos",
                        ))]
                        config.interface.as_deref(),
                        config.nodelay,
                        config.tls_info,
                    )
                }
                #[cfg(any(feature = "__native-tls", feature = "__rustls",))]
                TlsBackend::UnknownPreconfigured => {
                    return Err(crate::error::builder(
                        "Unknown TLS backend passed to `use_preconfigured_tls`",
                    ));
                }
            }

            #[cfg(not(feature = "__tls"))]
            ConnectorBuilder::new(
                http,
                proxies.clone(),
                config.local_address,
                #[cfg(any(
                    target_os = "android",
                    target_os = "fuchsia",
                    target_os = "illumos",
                    target_os = "ios",
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "solaris",
                    target_os = "tvos",
                    target_os = "visionos",
                    target_os = "watchos",
                ))]
                config.interface.as_deref(),
                config.nodelay,
            )
        };

        connector_builder.set_timeout(config.connect_timeout);
        connector_builder.set_verbose(config.connection_verbose);
        connector_builder.set_keepalive(config.tcp_keepalive);
        connector_builder.set_keepalive_interval(config.tcp_keepalive_interval);
        connector_builder.set_keepalive_retries(config.tcp_keepalive_retries);
        #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
        connector_builder.set_tcp_user_timeout(config.tcp_user_timeout);

        #[cfg(feature = "socks")]
        connector_builder.set_socks_resolver(resolver);

        // TODO: It'd be best to refactor this so the HttpConnector is never
        // constructed at all. But there's a lot of code for all the different
        // ways TLS can be configured...
        #[cfg(unix)]
        connector_builder.set_unix_socket(config.unix_socket);
        #[cfg(target_os = "windows")]
        connector_builder.set_windows_named_pipe(config.windows_named_pipe.clone());

        let mut builder =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new());
        #[cfg(feature = "http2")]
        {
            if matches!(config.http_version_pref, HttpVersionPref::Http2) {
                builder.http2_only(true);
            }

            if let Some(http2_initial_stream_window_size) = config.http2_initial_stream_window_size
            {
                builder.http2_initial_stream_window_size(http2_initial_stream_window_size);
            }
            if let Some(http2_initial_connection_window_size) =
                config.http2_initial_connection_window_size
            {
                builder.http2_initial_connection_window_size(http2_initial_connection_window_size);
            }
            if config.http2_adaptive_window {
                builder.http2_adaptive_window(true);
            }
            if let Some(http2_max_frame_size) = config.http2_max_frame_size {
                builder.http2_max_frame_size(http2_max_frame_size);
            }
            if let Some(http2_max_header_list_size) = config.http2_max_header_list_size {
                builder.http2_max_header_list_size(http2_max_header_list_size);
            }
            if let Some(http2_header_table_size) = config.http2_header_table_size {
                builder.http2_header_table_size(http2_header_table_size);
            }
            if let Some(http2_max_concurrent_streams) = config.http2_max_concurrent_streams {
                builder.http2_max_concurrent_streams(http2_max_concurrent_streams);
            }
            if let Some(http2_keep_alive_interval) = config.http2_keep_alive_interval {
                builder.http2_keep_alive_interval(http2_keep_alive_interval);
            }
            if let Some(http2_keep_alive_timeout) = config.http2_keep_alive_timeout {
                builder.http2_keep_alive_timeout(http2_keep_alive_timeout);
            }
            if config.http2_keep_alive_while_idle {
                builder.http2_keep_alive_while_idle(true);
            }
        }

        builder.timer(hyper_util::rt::TokioTimer::new());
        builder.pool_timer(hyper_util::rt::TokioTimer::new());
        builder.pool_idle_timeout(config.pool_idle_timeout);
        builder.pool_max_idle_per_host(config.pool_max_idle_per_host);

        if config.http09_responses {
            builder.http09_responses(true);
        }

        if config.http1_title_case_headers {
            builder.http1_title_case_headers(true);
        }

        if config.http1_allow_obsolete_multiline_headers_in_responses {
            builder.http1_allow_obsolete_multiline_headers_in_responses(true);
        }

        if config.http1_ignore_invalid_headers_in_responses {
            builder.http1_ignore_invalid_headers_in_responses(true);
        }

        if config.http1_allow_spaces_after_header_name_in_responses {
            builder.http1_allow_spaces_after_header_name_in_responses(true);
        }

        if let Some(http1_max_headers) = config.http1_max_headers {
            builder.http1_max_headers(http1_max_headers);
        }

        let proxies_maybe_http_auth = proxies.iter().any(|p| p.maybe_has_http_auth());
        let proxies_maybe_http_custom_headers =
            proxies.iter().any(|p| p.maybe_has_http_custom_headers());

        let redirect_policy_desc = if config.redirect_policy.is_default() {
            None
        } else {
            Some(format!("{:?}", &config.redirect_policy))
        };

        let hyper_client = builder.build(connector_builder.build(config.connector_layers));
        let hyper_service = HyperService {
            hyper: hyper_client,
        };

        let redirect_policy = {
            let mut p = TowerRedirectPolicy::new(config.redirect_policy);
            p.with_referer(config.referer)
                .with_https_only(config.https_only);
            p
        };

        let retry_policy = config.retry_policy.into_policy();

        let svc = tower::retry::Retry::new(retry_policy.clone(), hyper_service);

        #[cfg(feature = "cookies")]
        let svc = CookieService::new(svc, config.cookie_store.clone());
        let hyper = FollowRedirect::with_policy(svc, redirect_policy.clone());
        #[cfg(any(
            feature = "gzip",
            feature = "brotli",
            feature = "zstd",
            feature = "deflate"
        ))]
        let hyper = Decompression::new(hyper)
            // set everything to NO, in case tower-http has it enabled but
            // reqwest does not. then set to config value if cfg allows.
            .no_gzip()
            .no_deflate()
            .no_br()
            .no_zstd();
        #[cfg(feature = "gzip")]
        let hyper = hyper.gzip(config.accepts.gzip);
        #[cfg(feature = "brotli")]
        let hyper = hyper.br(config.accepts.brotli);
        #[cfg(feature = "zstd")]
        let hyper = hyper.zstd(config.accepts.zstd);
        #[cfg(feature = "deflate")]
        let hyper = hyper.deflate(config.accepts.deflate);

        Ok(Client {
            inner: Arc::new(ClientRef {
                accepts: config.accepts,
                #[cfg(feature = "cookies")]
                cookie_store: config.cookie_store.clone(),
                // Use match instead of map since config is partially moved,
                // and it cannot be used in closure
                #[cfg(feature = "http3")]
                h3_client: match h3_connector {
                    Some(h3_connector) => {
                        let h3_service = H3Client::new(h3_connector, config.pool_idle_timeout);
                        let svc = tower::retry::Retry::new(retry_policy, h3_service);
                        #[cfg(feature = "cookies")]
                        let svc = CookieService::new(svc, config.cookie_store);
                        let svc = FollowRedirect::with_policy(svc, redirect_policy);
                        #[cfg(any(
                            feature = "gzip",
                            feature = "brotli",
                            feature = "zstd",
                            feature = "deflate"
                        ))]
                        let svc = Decompression::new(svc)
                            // set everything to NO, in case tower-http has it enabled but
                            // reqwest does not. then set to config value if cfg allows.
                            .no_gzip()
                            .no_deflate()
                            .no_br()
                            .no_zstd();
                        #[cfg(feature = "gzip")]
                        let svc = svc.gzip(config.accepts.gzip);
                        #[cfg(feature = "brotli")]
                        let svc = svc.br(config.accepts.brotli);
                        #[cfg(feature = "zstd")]
                        let svc = svc.zstd(config.accepts.zstd);
                        #[cfg(feature = "deflate")]
                        let svc = svc.deflate(config.accepts.deflate);
                        Some(svc)
                    }
                    None => None,
                },
                headers: config.headers,
                referer: config.referer,
                read_timeout: config.read_timeout,
                total_timeout: RequestConfig::new(config.timeout),
                hyper,
                proxies,
                proxies_maybe_http_auth,
                proxies_maybe_http_custom_headers,
                https_only: config.https_only,
                redirect_policy_desc,
            }),
        })
    }

    // Higher-level options

    /// Sets the `User-Agent` header to be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), reqwest::Error> {
    /// // Name your user agent after your app?
    /// static APP_USER_AGENT: &str = concat!(
    ///     env!("CARGO_PKG_NAME"),
    ///     "/",
    ///     env!("CARGO_PKG_VERSION"),
    /// );
    ///
    /// let client = reqwest::Client::builder()
    ///     .user_agent(APP_USER_AGENT)
    ///     .build()?;
    /// let res = client.get("https://www.rust-lang.org").send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn user_agent<V>(mut self, value: V) -> ClientBuilder
    where
        V: TryInto<HeaderValue>,
        V::Error: Into<http::Error>,
    {
        match value.try_into() {
            Ok(value) => {
                self.config.headers.insert(USER_AGENT, value);
            }
            Err(e) => {
                self.config.error = Some(crate::error::builder(e.into()));
            }
        };
        self
    }
    /// Sets the default headers for every request.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reqwest::header;
    /// # async fn doc() -> Result<(), reqwest::Error> {
    /// let mut headers = header::HeaderMap::new();
    /// headers.insert("X-MY-HEADER", header::HeaderValue::from_static("value"));
    ///
    /// // Consider marking security-sensitive headers with `set_sensitive`.
    /// let mut auth_value = header::HeaderValue::from_static("secret");
    /// auth_value.set_sensitive(true);
    /// headers.insert(header::AUTHORIZATION, auth_value);
    ///
    /// // get a client builder
    /// let client = reqwest::Client::builder()
    ///     .default_headers(headers)
    ///     .build()?;
    /// let res = client.get("https://www.rust-lang.org").send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn default_headers(mut self, headers: HeaderMap) -> ClientBuilder {
        for (key, value) in headers.iter() {
            self.config.headers.insert(key, value.clone());
        }
        self
    }

    /// Enable a persistent cookie store for the client.
    ///
    /// Cookies received in responses will be preserved and included in
    /// additional requests.
    ///
    /// By default, no cookie store is used. Enabling the cookie store
    /// with `cookie_store(true)` will set the store to a default implementation.
    /// It is **not** necessary to call [cookie_store(true)](crate::ClientBuilder::cookie_store) if [cookie_provider(my_cookie_store)](crate::ClientBuilder::cookie_provider)
    /// is used; calling [cookie_store(true)](crate::ClientBuilder::cookie_store) _after_ [cookie_provider(my_cookie_store)](crate::ClientBuilder::cookie_provider) will result
    /// in the provided `my_cookie_store` being **overridden** with a default implementation.
    ///
    /// # Optional
    ///
    /// This requires the optional `cookies` feature to be enabled.
    #[cfg(feature = "cookies")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cookies")))]
    pub fn cookie_store(mut self, enable: bool) -> ClientBuilder {
        if enable {
            self.cookie_provider(Arc::new(cookie::Jar::default()))
        } else {
            self.config.cookie_store = None;
            self
        }
    }

    /// Set the persistent cookie store for the client.
    ///
    /// Cookies received in responses will be passed to this store, and
    /// additional requests will query this store for cookies.
    ///
    /// By default, no cookie store is used. It is **not** necessary to also call
    /// [cookie_store(true)](crate::ClientBuilder::cookie_store) if [cookie_provider(my_cookie_store)](crate::ClientBuilder::cookie_provider) is used; calling
    /// [cookie_store(true)](crate::ClientBuilder::cookie_store) _after_ [cookie_provider(my_cookie_store)](crate::ClientBuilder::cookie_provider) will result
    /// in the provided `my_cookie_store` being **overridden** with a default implementation.
    ///
    /// # Optional
    ///
    /// This requires the optional `cookies` feature to be enabled.
    #[cfg(feature = "cookies")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cookies")))]
    pub fn cookie_provider<C: cookie::CookieStore + 'static>(
        mut self,
        cookie_store: Arc<C>,
    ) -> ClientBuilder {
        self.config.cookie_store = Some(cookie_store as _);
        self
    }

    /// Enable auto gzip decompression by checking the `Content-Encoding` response header.
    ///
    /// If auto gzip decompression is turned on:
    ///
    /// - When sending a request and if the request's headers do not already contain
    ///   an `Accept-Encoding` **and** `Range` values, the `Accept-Encoding` header is set to `gzip`.
    ///   The request body is **not** automatically compressed.
    /// - When receiving a response, if its headers contain a `Content-Encoding` value of
    ///   `gzip`, both `Content-Encoding` and `Content-Length` are removed from the
    ///   headers' set. The response body is automatically decompressed.
    ///
    /// If the `gzip` feature is turned on, the default option is enabled.
    ///
    /// # Optional
    ///
    /// This requires the optional `gzip` feature to be enabled
    #[cfg(feature = "gzip")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    pub fn gzip(mut self, enable: bool) -> ClientBuilder {
        self.config.accepts.gzip = enable;
        self
    }

    /// Enable auto brotli decompression by checking the `Content-Encoding` response header.
    ///
    /// If auto brotli decompression is turned on:
    ///
    /// - When sending a request and if the request's headers do not already contain
    ///   an `Accept-Encoding` **and** `Range` values, the `Accept-Encoding` header is set to `br`.
    ///   The request body is **not** automatically compressed.
    /// - When receiving a response, if its headers contain a `Content-Encoding` value of
    ///   `br`, both `Content-Encoding` and `Content-Length` are removed from the
    ///   headers' set. The response body is automatically decompressed.
    ///
    /// If the `brotli` feature is turned on, the default option is enabled.
    ///
    /// # Optional
    ///
    /// This requires the optional `brotli` feature to be enabled
    #[cfg(feature = "brotli")]
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    pub fn brotli(mut self, enable: bool) -> ClientBuilder {
        self.config.accepts.brotli = enable;
        self
    }

    /// Enable auto zstd decompression by checking the `Content-Encoding` response header.
    ///
    /// If auto zstd decompression is turned on:
    ///
    /// - When sending a request and if the request's headers do not already contain
    ///   an `Accept-Encoding` **and** `Range` values, the `Accept-Encoding` header is set to `zstd`.
    ///   The request body is **not** automatically compressed.
    /// - When receiving a response, if its headers contain a `Content-Encoding` value of
    ///   `zstd`, both `Content-Encoding` and `Content-Length` are removed from the
    ///   headers' set. The response body is automatically decompressed.
    ///
    /// If the `zstd` feature is turned on, the default option is enabled.
    ///
    /// # Optional
    ///
    /// This requires the optional `zstd` feature to be enabled
    #[cfg(feature = "zstd")]
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    pub fn zstd(mut self, enable: bool) -> ClientBuilder {
        self.config.accepts.zstd = enable;
        self
    }

    /// Enable auto deflate decompression by checking the `Content-Encoding` response header.
    ///
    /// If auto deflate decompression is turned on:
    ///
    /// - When sending a request and if the request's headers do not already contain
    ///   an `Accept-Encoding` **and** `Range` values, the `Accept-Encoding` header is set to `deflate`.
    ///   The request body is **not** automatically compressed.
    /// - When receiving a response, if it's headers contain a `Content-Encoding` value that
    ///   equals to `deflate`, both values `Content-Encoding` and `Content-Length` are removed from the
    ///   headers' set. The response body is automatically decompressed.
    ///
    /// If the `deflate` feature is turned on, the default option is enabled.
    ///
    /// # Optional
    ///
    /// This requires the optional `deflate` feature to be enabled
    #[cfg(feature = "deflate")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    pub fn deflate(mut self, enable: bool) -> ClientBuilder {
        self.config.accepts.deflate = enable;
        self
    }

    /// Disable auto response body gzip decompression.
    ///
    /// This method exists even if the optional `gzip` feature is not enabled.
    /// This can be used to ensure a `Client` doesn't use gzip decompression
    /// even if another dependency were to enable the optional `gzip` feature.
    pub fn no_gzip(self) -> ClientBuilder {
        #[cfg(feature = "gzip")]
        {
            self.gzip(false)
        }

        #[cfg(not(feature = "gzip"))]
        {
            self
        }
    }

    /// Disable auto response body brotli decompression.
    ///
    /// This method exists even if the optional `brotli` feature is not enabled.
    /// This can be used to ensure a `Client` doesn't use brotli decompression
    /// even if another dependency were to enable the optional `brotli` feature.
    pub fn no_brotli(self) -> ClientBuilder {
        #[cfg(feature = "brotli")]
        {
            self.brotli(false)
        }

        #[cfg(not(feature = "brotli"))]
        {
            self
        }
    }

    /// Disable auto response body zstd decompression.
    ///
    /// This method exists even if the optional `zstd` feature is not enabled.
    /// This can be used to ensure a `Client` doesn't use zstd decompression
    /// even if another dependency were to enable the optional `zstd` feature.
    pub fn no_zstd(self) -> ClientBuilder {
        #[cfg(feature = "zstd")]
        {
            self.zstd(false)
        }

        #[cfg(not(feature = "zstd"))]
        {
            self
        }
    }

    /// Disable auto response body deflate decompression.
    ///
    /// This method exists even if the optional `deflate` feature is not enabled.
    /// This can be used to ensure a `Client` doesn't use deflate decompression
    /// even if another dependency were to enable the optional `deflate` feature.
    pub fn no_deflate(self) -> ClientBuilder {
        #[cfg(feature = "deflate")]
        {
            self.deflate(false)
        }

        #[cfg(not(feature = "deflate"))]
        {
            self
        }
    }

    // Redirect options

    /// Set a `RedirectPolicy` for this client.
    ///
    /// Default will follow redirects up to a maximum of 10.
    pub fn redirect(mut self, policy: redirect::Policy) -> ClientBuilder {
        self.config.redirect_policy = policy;
        self
    }

    /// Enable or disable automatic setting of the `Referer` header.
    ///
    /// Default is `true`.
    pub fn referer(mut self, enable: bool) -> ClientBuilder {
        self.config.referer = enable;
        self
    }

    // Retry options

    /// Set a request retry policy.
    ///
    /// Default behavior is to retry protocol NACKs.
    // XXX: accept an `impl retry::IntoPolicy` instead?
    pub fn retry(mut self, policy: crate::retry::Builder) -> ClientBuilder {
        self.config.retry_policy = policy;
        self
    }

    // Proxy options

    /// Add a `Proxy` to the list of proxies the `Client` will use.
    ///
    /// # Note
    ///
    /// Adding a proxy will disable the automatic usage of the "system" proxy.
    pub fn proxy(mut self, proxy: Proxy) -> ClientBuilder {
        self.config.proxies.push(proxy.into_matcher());
        self.config.auto_sys_proxy = false;
        self
    }

    /// Clear all `Proxies`, so `Client` will use no proxy anymore.
    ///
    /// # Note
    /// To add a proxy exclusion list, use [crate::proxy::Proxy::no_proxy()]
    /// on all desired proxies instead.
    ///
    /// This also disables the automatic usage of the "system" proxy.
    pub fn no_proxy(mut self) -> ClientBuilder {
        self.config.proxies.clear();
        self.config.auto_sys_proxy = false;
        self
    }

    // Timeout options

    /// Enables a total request timeout.
    ///
    /// The timeout is applied from when the request starts connecting until the
    /// response body has finished. Also considered a total deadline.
    ///
    /// Default is no timeout.
    pub fn timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.config.timeout = Some(timeout);
        self
    }

    /// Enables a read timeout.
    ///
    /// The timeout applies to each read operation, and resets after a
    /// successful read. This is more appropriate for detecting stalled
    /// connections when the size isn't known beforehand.
    ///
    /// Default is no timeout.
    pub fn read_timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.config.read_timeout = Some(timeout);
        self
    }

    /// Set a timeout for only the connect phase of a `Client`.
    ///
    /// Default is `None`.
    ///
    /// # Note
    ///
    /// This **requires** the futures be executed in a tokio runtime with
    /// a tokio timer enabled.
    pub fn connect_timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.config.connect_timeout = Some(timeout);
        self
    }

    /// Set whether connections should emit verbose logs.
    ///
    /// Enabling this option will emit [log][] messages at the `TRACE` level
    /// for read and write operations on connections.
    ///
    /// [log]: https://crates.io/crates/log
    pub fn connection_verbose(mut self, verbose: bool) -> ClientBuilder {
        self.config.connection_verbose = verbose;
        self
    }

    // HTTP options

    /// Set an optional timeout for idle sockets being kept-alive.
    ///
    /// Pass `None` to disable timeout.
    ///
    /// Default is 90 seconds.
    pub fn pool_idle_timeout<D>(mut self, val: D) -> ClientBuilder
    where
        D: Into<Option<Duration>>,
    {
        self.config.pool_idle_timeout = val.into();
        self
    }

    /// Sets the maximum idle connection per host allowed in the pool.
    ///
    /// Default is `usize::MAX` (no limit).
    pub fn pool_max_idle_per_host(mut self, max: usize) -> ClientBuilder {
        self.config.pool_max_idle_per_host = max;
        self
    }

    /// Send headers as title case instead of lowercase.
    pub fn http1_title_case_headers(mut self) -> ClientBuilder {
        self.config.http1_title_case_headers = true;
        self
    }

    /// Set whether HTTP/1 connections will accept obsolete line folding for
    /// header values.
    ///
    /// Newline codepoints (`\r` and `\n`) will be transformed to spaces when
    /// parsing.
    pub fn http1_allow_obsolete_multiline_headers_in_responses(
        mut self,
        value: bool,
    ) -> ClientBuilder {
        self.config
            .http1_allow_obsolete_multiline_headers_in_responses = value;
        self
    }

    /// Sets whether invalid header lines should be silently ignored in HTTP/1 responses.
    pub fn http1_ignore_invalid_headers_in_responses(mut self, value: bool) -> ClientBuilder {
        self.config.http1_ignore_invalid_headers_in_responses = value;
        self
    }

    /// Set whether HTTP/1 connections will accept spaces between header
    /// names and the colon that follow them in responses.
    ///
    /// Newline codepoints (`\r` and `\n`) will be transformed to spaces when
    /// parsing.
    pub fn http1_allow_spaces_after_header_name_in_responses(
        mut self,
        value: bool,
    ) -> ClientBuilder {
        self.config
            .http1_allow_spaces_after_header_name_in_responses = value;
        self
    }

    /// Set the maximum number of headers accepted in an HTTP/1 response.
    ///
    /// When a response contains more headers than this value, it is rejected
    /// with a parse error and the request fails.
    ///
    /// Default is 100.
    pub fn http1_max_headers(mut self, max: usize) -> ClientBuilder {
        self.config.http1_max_headers = Some(max);
        self
    }

    /// Only use HTTP/1.
    pub fn http1_only(mut self) -> ClientBuilder {
        self.config.http_version_pref = HttpVersionPref::Http1;
        self
    }

    /// Allow HTTP/0.9 responses
    pub fn http09_responses(mut self) -> ClientBuilder {
        self.config.http09_responses = true;
        self
    }

    /// Only use HTTP/2.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_prior_knowledge(mut self) -> ClientBuilder {
        self.config.http_version_pref = HttpVersionPref::Http2;
        self
    }

    /// Only use HTTP/3.
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_prior_knowledge(mut self) -> ClientBuilder {
        self.config.http_version_pref = HttpVersionPref::Http3;
        self
    }

    /// Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2 stream-level flow control.
    ///
    /// Default may change internally to optimize for common uses.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_initial_stream_window_size(mut self, sz: impl Into<Option<u32>>) -> ClientBuilder {
        self.config.http2_initial_stream_window_size = sz.into();
        self
    }

    /// Sets the max connection-level flow control for HTTP2
    ///
    /// Default may change internally to optimize for common uses.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_initial_connection_window_size(
        mut self,
        sz: impl Into<Option<u32>>,
    ) -> ClientBuilder {
        self.config.http2_initial_connection_window_size = sz.into();
        self
    }

    /// Sets whether to use an adaptive flow control.
    ///
    /// Enabling this will override the limits set in `http2_initial_stream_window_size` and
    /// `http2_initial_connection_window_size`.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_adaptive_window(mut self, enabled: bool) -> ClientBuilder {
        self.config.http2_adaptive_window = enabled;
        self
    }

    /// Sets the maximum frame size to use for HTTP2.
    ///
    /// Default is currently 16,384 but may change internally to optimize for common uses.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_max_frame_size(mut self, sz: impl Into<Option<u32>>) -> ClientBuilder {
        self.config.http2_max_frame_size = sz.into();
        self
    }

    /// Sets the maximum size of received header frames for HTTP2.
    ///
    /// Default is currently 16KB, but can change.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_max_header_list_size(mut self, max_header_size_bytes: u32) -> ClientBuilder {
        self.config.http2_max_header_list_size = Some(max_header_size_bytes);
        self
    }

    /// Sets the header table size to use for HTTP2.
    ///
    /// Passing `None` will do nothing.
    ///
    /// If not set, hyper will use a default.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_header_table_size(mut self, sz: impl Into<Option<u32>>) -> ClientBuilder {
        self.config.http2_header_table_size = sz.into();
        self
    }

    /// Sets the `SETTINGS_MAX_CONCURRENT_STREAMS` option for HTTP2 connections.
    ///
    /// Passing `None` will do nothing.
    ///
    /// The default value is determined by the `h2` crate.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_max_concurrent_streams(mut self, max: impl Into<Option<u32>>) -> ClientBuilder {
        self.config.http2_max_concurrent_streams = max.into();
        self
    }

    /// Sets an interval for HTTP2 Ping frames should be sent to keep a connection alive.
    ///
    /// Pass `None` to disable HTTP2 keep-alive.
    /// Default is currently disabled.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_keep_alive_interval(
        mut self,
        interval: impl Into<Option<Duration>>,
    ) -> ClientBuilder {
        self.config.http2_keep_alive_interval = interval.into();
        self
    }

    /// Sets a timeout for receiving an acknowledgement of the keep-alive ping.
    ///
    /// If the ping is not acknowledged within the timeout, the connection will be closed.
    /// Does nothing if `http2_keep_alive_interval` is disabled.
    /// Default is currently disabled.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_keep_alive_timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.config.http2_keep_alive_timeout = Some(timeout);
        self
    }

    /// Sets whether HTTP2 keep-alive should apply while the connection is idle.
    ///
    /// If disabled, keep-alive pings are only sent while there are open request/responses streams.
    /// If enabled, pings are also sent when no streams are active.
    /// Does nothing if `http2_keep_alive_interval` is disabled.
    /// Default is `false`.
    #[cfg(feature = "http2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
    pub fn http2_keep_alive_while_idle(mut self, enabled: bool) -> ClientBuilder {
        self.config.http2_keep_alive_while_idle = enabled;
        self
    }

    // TCP options

    /// Set whether sockets have `TCP_NODELAY` enabled.
    ///
    /// Default is `true`.
    pub fn tcp_nodelay(mut self, enabled: bool) -> ClientBuilder {
        self.config.nodelay = enabled;
        self
    }

    /// Bind to a local IP Address.
    ///
    /// # Example
    ///
    /// ```
    /// # fn doc() -> Result<(), reqwest::Error> {
    /// use std::net::IpAddr;
    /// let local_addr = IpAddr::from([12, 4, 1, 8]);
    /// let client = reqwest::Client::builder()
    ///     .local_address(local_addr)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn local_address<T>(mut self, addr: T) -> ClientBuilder
    where
        T: Into<Option<IpAddr>>,
    {
        self.config.local_address = addr.into();
        self
    }

    /// Bind connections only on the specified network interface.
    ///
    /// This option is only available on the following operating systems:
    ///
    /// - Android
    /// - Fuchsia
    /// - Linux,
    /// - macOS and macOS-like systems (iOS, tvOS, watchOS and visionOS)
    /// - Solaris and illumos
    ///
    /// On Android, Linux, and Fuchsia, this uses the
    /// [`SO_BINDTODEVICE`][man-7-socket] socket option. On macOS and macOS-like
    /// systems, Solaris, and illumos, this instead uses the [`IP_BOUND_IF` and
    /// `IPV6_BOUND_IF`][man-7p-ip] socket options (as appropriate).
    ///
    /// Note that connections will fail if the provided interface name is not a
    /// network interface that currently exists when a connection is established.
    ///
    /// # Example
    ///
    /// ```
    /// # fn doc() -> Result<(), reqwest::Error> {
    /// let interface = "lo";
    /// let client = reqwest::Client::builder()
    ///     .interface(interface)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [man-7-socket]: https://man7.org/linux/man-pages/man7/socket.7.html
    /// [man-7p-ip]: https://docs.oracle.com/cd/E86824_01/html/E54777/ip-7p.html
    #[cfg(any(
        target_os = "android",
        target_os = "fuchsia",
        target_os = "illumos",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "solaris",
        target_os = "tvos",
        target_os = "visionos",
        target_os = "watchos",
    ))]
    pub fn interface(mut self, interface: &str) -> ClientBuilder {
        self.config.interface = Some(interface.to_string());
        self
    }

    /// Set that all sockets have `SO_KEEPALIVE` set with the supplied duration.
    ///
    /// If `None`, the option will not be set.
    pub fn tcp_keepalive<D>(mut self, val: D) -> ClientBuilder
    where
        D: Into<Option<Duration>>,
    {
        self.config.tcp_keepalive = val.into();
        self
    }

    /// Set that all sockets have `SO_KEEPALIVE` set with the supplied interval.
    ///
    /// If `None`, the option will not be set.
    pub fn tcp_keepalive_interval<D>(mut self, val: D) -> ClientBuilder
    where
        D: Into<Option<Duration>>,
    {
        self.config.tcp_keepalive_interval = val.into();
        self
    }

    /// Set that all sockets have `SO_KEEPALIVE` set with the supplied retry count.
    ///
    /// If `None`, the option will not be set.
    pub fn tcp_keepalive_retries<C>(mut self, retries: C) -> ClientBuilder
    where
        C: Into<Option<u32>>,
    {
        self.config.tcp_keepalive_retries = retries.into();
        self
    }

    /// Set that all sockets have `TCP_USER_TIMEOUT` set with the supplied duration.
    ///
    /// This option controls how long transmitted data may remain unacknowledged before
    /// the connection is force-closed.
    ///
    /// If `None`, the option will not be set.
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    pub fn tcp_user_timeout<D>(mut self, val: D) -> ClientBuilder
    where
        D: Into<Option<Duration>>,
    {
        self.config.tcp_user_timeout = val.into();
        self
    }

    // Alt Transports

    /// Set that all connections will use this Unix socket.
    ///
    /// If a request URI uses the `https` scheme, TLS will still be used over
    /// the Unix socket.
    ///
    /// # Note
    ///
    /// This option is not compatible with any of the TCP or Proxy options.
    /// Setting this will ignore all those options previously set.
    ///
    /// Likewise, DNS resolution will not be done on the domain name.
    #[cfg(unix)]
    pub fn unix_socket(mut self, path: impl UnixSocketProvider) -> ClientBuilder {
        self.config.unix_socket = Some(path.reqwest_uds_path(crate::connect::uds::Internal).into());
        self
    }

    /// Set that all connections will use this Windows named pipe.
    ///
    /// If a request URI uses the `https` scheme, TLS will still be used over
    /// the Windows named pipe.
    ///
    /// # Note
    ///
    /// This option is not compatible with any of the TCP or Proxy options.
    /// Setting this will ignore all those options previously set.
    ///
    /// Likewise, DNS resolution will not be done on the domain name.
    #[cfg(target_os = "windows")]
    pub fn windows_named_pipe(mut self, pipe: impl WindowsNamedPipeProvider) -> ClientBuilder {
        self.config.windows_named_pipe = Some(
            pipe.reqwest_windows_named_pipe_path(crate::connect::windows_named_pipe::Internal)
                .into(),
        );
        self
    }

    // TLS options

    /// Add custom certificate roots.
    ///
    /// This can be used to connect to a server that has a self-signed
    /// certificate for example.
    ///
    /// This optional attempts to merge with any native or built-in roots.
    ///
    /// # Errors
    ///
    /// If the selected TLS backend or verifier does not support merging
    /// certificates, the builder will return an error.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_certs_merge(
        mut self,
        certs: impl IntoIterator<Item = Certificate>,
    ) -> ClientBuilder {
        self.config.root_certs.extend(certs);
        self
    }

    /// Use only the provided certificate roots.
    ///
    /// This can be used to connect to a server that has a self-signed
    /// certificate for example.
    ///
    /// This option disables any native or built-in roots, and **only** uses
    /// the roots provided to this method.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_certs_only(mut self, certs: impl IntoIterator<Item = Certificate>) -> ClientBuilder {
        self.config.root_certs.extend(certs);
        self.config.tls_certs_only = true;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_certs_merge()`] or
    /// [`ClientBuilder::tls_certs_only()`] instead.
    #[cfg(feature = "__tls")]
    pub fn add_root_certificate(mut self, cert: Certificate) -> ClientBuilder {
        self.config.root_certs.push(cert);
        self
    }

    /// Add multiple certificate revocation lists.
    ///
    /// # Errors
    ///
    /// This only works if also using only provided root certificates. This
    /// cannot work with the native verifier.
    ///
    /// If CRLs are added but `tls_certs_only()` is not called, the builder
    /// will return an error.
    ///
    /// # Optional
    ///
    /// This requires the `rustls(-...)` Cargo feature enabled.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn tls_crls_only(
        mut self,
        crls: impl IntoIterator<Item = CertificateRevocationList>,
    ) -> ClientBuilder {
        self.config.crls.extend(crls);
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_crls_only()`] instead.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn add_crl(mut self, crl: CertificateRevocationList) -> ClientBuilder {
        self.config.crls.push(crl);
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_crls_only()`] instead.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn add_crls(
        mut self,
        crls: impl IntoIterator<Item = CertificateRevocationList>,
    ) -> ClientBuilder {
        self.config.crls.extend(crls);
        self
    }

    /// Sets the identity to be used for client certificate authentication.
    ///
    /// # Optional
    ///
    /// This requires the optional `native-tls` or `rustls(-...)` feature to be
    /// enabled.
    #[cfg(any(feature = "__native-tls", feature = "__rustls"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "native-tls", feature = "rustls"))))]
    pub fn identity(mut self, identity: Identity) -> ClientBuilder {
        self.config.identity = Some(identity);
        self
    }

    /// Controls the use of hostname verification.
    ///
    /// Defaults to `false`.
    ///
    /// # Warning
    ///
    /// You should think very carefully before you use this method. If
    /// hostname verification is not used, any valid certificate for any
    /// site will be trusted for use from any other. This introduces a
    /// significant vulnerability to man-in-the-middle attacks.
    ///
    /// # Errors
    ///
    /// Depending on the TLS backend and verifier, this might not work with
    /// native certificates, only those added with [`ClientBuilder::tls_certs_only()`].
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_danger_accept_invalid_hostnames(
        mut self,
        accept_invalid_hostname: bool,
    ) -> ClientBuilder {
        self.config.hostname_verification = !accept_invalid_hostname;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_danger_accept_invalid_hostnames()`] instead.
    #[cfg(feature = "__tls")]
    pub fn danger_accept_invalid_hostnames(self, accept_invalid_hostname: bool) -> ClientBuilder {
        self.tls_danger_accept_invalid_hostnames(accept_invalid_hostname)
    }

    /// Controls the use of certificate validation.
    ///
    /// Defaults to `false`.
    ///
    /// # Warning
    ///
    /// You should think very carefully before using this method. If
    /// invalid certificates are trusted, *any* certificate for *any* site
    /// will be trusted for use. This includes expired certificates. This
    /// introduces significant vulnerabilities, and should only be used
    /// as a last resort.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_danger_accept_invalid_certs(mut self, accept_invalid_certs: bool) -> ClientBuilder {
        self.config.certs_verification = !accept_invalid_certs;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_danger_accept_invalid_certs()`] instead.
    #[cfg(feature = "__tls")]
    pub fn danger_accept_invalid_certs(self, accept_invalid_certs: bool) -> ClientBuilder {
        self.tls_danger_accept_invalid_certs(accept_invalid_certs)
    }

    /// Controls the use of TLS server name indication.
    ///
    /// Defaults to `true`.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_sni(mut self, tls_sni: bool) -> ClientBuilder {
        self.config.tls_sni = tls_sni;
        self
    }

    /// Controls if the SSLKEYLOGFILE environment variable is respected.
    ///
    /// When enabled, if the environment variable `SSLKEYLOGFILE` is present at runtime,
    /// TLS keys will be logged to the file at the path described in the variable.
    /// This can be used by end-users to allow debugging TLS connections.
    ///
    /// Defaults to `false`.
    ///
    /// # Optional
    ///
    /// This requires the `rustls(-...)` Cargo feature enabled.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn tls_sslkeylogfile(mut self, on: bool) -> ClientBuilder {
        self.config.tls_sslkeylogfile = on;
        self
    }

    /// Set the minimum required TLS version for connections.
    ///
    /// By default, the TLS backend's own default is used.
    ///
    /// On Apple platforms, a value of `tls::Version::TLS_1_3` may cause requests
    /// to fail (with error -9830) with the `native-tls` backend due to lack of
    /// TLS 1.3 support.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_version_min(mut self, version: tls::Version) -> ClientBuilder {
        self.config.min_tls_version = Some(version);
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_version_min()`] instead.
    #[cfg(feature = "__tls")]
    pub fn min_tls_version(self, version: tls::Version) -> ClientBuilder {
        self.tls_version_min(version)
    }

    /// Set the maximum allowed TLS version for connections.
    ///
    /// By default, there's no maximum.
    ///
    /// On Apple platforms, a value of `tls::Version::TLS_1_3` may cause requests
    /// to fall back to TLS 1.2 if allowed by `tls_version_min`, or fail (with error
    /// -9830) with the `native-tls` backend due to lack of TLS 1.3 support.
    ///
    /// # Errors
    ///
    /// Cannot set a maximum outside the protocol versions supported by
    /// `rustls` with the `rustls` backend.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_version_max(mut self, version: tls::Version) -> ClientBuilder {
        self.config.max_tls_version = Some(version);
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_version_max()`] instead.
    #[cfg(feature = "__tls")]
    pub fn max_tls_version(self, version: tls::Version) -> ClientBuilder {
        self.tls_version_max(version)
    }

    /// Force using the native TLS backend.
    ///
    /// Since multiple TLS backends can be optionally enabled, this option will
    /// force the `native-tls` backend to be used for this `Client`.
    ///
    /// # Optional
    ///
    /// This requires the optional `native-tls` feature to be enabled.
    #[cfg(feature = "__native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn tls_backend_native(mut self) -> ClientBuilder {
        self.config.tls = TlsBackend::NativeTls;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_backend_native()`] instead.
    #[cfg(feature = "__native-tls")]
    pub fn use_native_tls(self) -> ClientBuilder {
        self.tls_backend_native()
    }

    /// Force using the Rustls TLS backend.
    ///
    /// Since multiple TLS backends can be optionally enabled, this option will
    /// force the `rustls` backend to be used for this `Client`.
    ///
    /// # Optional
    ///
    /// This requires the optional `rustls(-...)` feature to be enabled.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn tls_backend_rustls(mut self) -> ClientBuilder {
        self.config.tls = TlsBackend::Rustls;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_backend_rustls()`] instead.
    #[cfg(feature = "__rustls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    pub fn use_rustls_tls(self) -> ClientBuilder {
        self.tls_backend_rustls()
    }

    /// Use a preconfigured TLS backend.
    ///
    /// If the passed `Any` argument is not a TLS backend that reqwest
    /// understands, the `ClientBuilder` will error when calling `build`.
    ///
    /// # Advanced
    ///
    /// <div class="warning">
    ///
    /// There is no semver stability on the internals of this method. Use at
    /// your own risk.
    ///
    /// </div>
    ///
    /// This is an advanced option, and can be somewhat brittle. Usage requires
    /// keeping the preconfigured TLS argument version in sync with reqwest,
    /// since version mismatches will result in an "unknown" TLS backend.
    ///
    /// If possible, it's preferable to use the methods on `ClientBuilder`
    /// to configure reqwest's TLS.
    ///
    /// # Optional
    ///
    /// This requires one of the optional features `native-tls` or
    /// `rustls(-...)` to be enabled.
    #[cfg(any(feature = "__native-tls", feature = "__rustls",))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "native-tls", feature = "rustls"))))]
    pub fn tls_backend_preconfigured(mut self, tls: impl Any) -> ClientBuilder {
        let mut tls = Some(tls);
        #[cfg(feature = "__native-tls")]
        {
            if let Some(conn) = (&mut tls as &mut dyn Any).downcast_mut::<Option<TlsConnector>>() {
                let tls = conn.take().expect("is definitely Some");
                let tls = crate::tls::TlsBackend::BuiltNativeTls(tls);
                self.config.tls = tls;
                return self;
            }
        }
        #[cfg(feature = "__rustls")]
        {
            if let Some(conn) =
                (&mut tls as &mut dyn Any).downcast_mut::<Option<rustls::ClientConfig>>()
            {
                let tls = conn.take().expect("is definitely Some");
                let tls = crate::tls::TlsBackend::BuiltRustls(tls);
                self.config.tls = tls;
                return self;
            }
        }

        // Otherwise, we don't recognize the TLS backend!
        self.config.tls = crate::tls::TlsBackend::UnknownPreconfigured;
        self
    }

    /// Deprecated: use [`ClientBuilder::tls_backend_preconfigured()`] instead.
    #[cfg(any(feature = "__native-tls", feature = "__rustls",))]
    pub fn use_preconfigured_tls(self, tls: impl Any) -> ClientBuilder {
        self.tls_backend_preconfigured(tls)
    }

    /// Add TLS information as `TlsInfo` extension to responses.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "default-tls", feature = "native-tls", feature = "rustls")))
    )]
    pub fn tls_info(mut self, tls_info: bool) -> ClientBuilder {
        self.config.tls_info = tls_info;
        self
    }

    /// Restrict the Client to be used with HTTPS only requests.
    ///
    /// Defaults to false.
    pub fn https_only(mut self, enabled: bool) -> ClientBuilder {
        self.config.https_only = enabled;
        self
    }

    /// Enables the [hickory-dns](hickory_resolver) async resolver instead of a default threadpool
    /// using `getaddrinfo`.
    ///
    /// If the `hickory-dns` feature is turned on, the default option is enabled.
    ///
    /// # Optional
    ///
    /// This requires the optional `hickory-dns` feature to be enabled
    ///
    /// # Warning
    ///
    /// The hickory resolver does not work exactly the same, or on all the platforms
    /// that the default resolver does
    #[cfg(feature = "hickory-dns")]
    #[cfg_attr(docsrs, doc(cfg(feature = "hickory-dns")))]
    pub fn hickory_dns(mut self, enable: bool) -> ClientBuilder {
        self.config.hickory_dns = enable;
        self
    }

    /// Disables the hickory-dns async resolver.
    ///
    /// This method exists even if the optional `hickory-dns` feature is not enabled.
    /// This can be used to ensure a `Client` doesn't use the hickory-dns async resolver
    /// even if another dependency were to enable the optional `hickory-dns` feature.
    pub fn no_hickory_dns(self) -> ClientBuilder {
        #[cfg(feature = "hickory-dns")]
        {
            self.hickory_dns(false)
        }

        #[cfg(not(feature = "hickory-dns"))]
        {
            self
        }
    }

    /// Override DNS resolution for specific domains to a particular IP address.
    ///
    /// Set the port to `0` to use the conventional port for the given scheme (e.g. 80 for http).
    /// Ports in the URL itself will always be used instead of the port in the overridden addr.
    pub fn resolve(self, domain: &str, addr: SocketAddr) -> ClientBuilder {
        self.resolve_to_addrs(domain, &[addr])
    }

    /// Override DNS resolution for specific domains to particular IP addresses.
    ///
    /// Set the port to `0` to use the conventional port for the given scheme (e.g. 80 for http).
    /// Ports in the URL itself will always be used instead of the port in the overridden addr.
    pub fn resolve_to_addrs(mut self, domain: &str, addrs: &[SocketAddr]) -> ClientBuilder {
        self.config
            .dns_overrides
            .insert(domain.to_ascii_lowercase(), addrs.to_vec());
        self
    }

    /// Override the DNS resolver implementation.
    ///
    /// Overrides for specific names passed to `resolve` and `resolve_to_addrs` will
    /// still be applied on top of this resolver.
    pub fn dns_resolver<R>(mut self, resolver: R) -> ClientBuilder
    where
        R: crate::dns::resolve::IntoResolve,
    {
        self.config.dns_resolver = Some(resolver.into_resolve());
        self
    }

    /// Whether to send data on the first flight ("early data") in TLS 1.3 handshakes
    /// for HTTP/3 connections.
    ///
    /// The default is false.
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn tls_early_data(mut self, enabled: bool) -> ClientBuilder {
        self.config.tls_enable_early_data = enabled;
        self
    }

    /// Maximum duration of inactivity to accept before timing out the QUIC connection.
    ///
    /// Please see docs in [`TransportConfig`] in [`quinn`].
    ///
    /// [`TransportConfig`]: https://docs.rs/quinn/latest/quinn/struct.TransportConfig.html
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_max_idle_timeout(mut self, value: Duration) -> ClientBuilder {
        self.config.quic_max_idle_timeout = Some(value);
        self
    }

    /// Maximum number of bytes the peer may transmit without acknowledgement on any one stream
    /// before becoming blocked.
    ///
    /// Please see docs in [`TransportConfig`] in [`quinn`].
    ///
    /// [`TransportConfig`]: https://docs.rs/quinn/latest/quinn/struct.TransportConfig.html
    ///
    /// # Panics
    ///
    /// Panics if the value is over 2^62.
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_stream_receive_window(mut self, value: u64) -> ClientBuilder {
        self.config.quic_stream_receive_window = Some(value.try_into().unwrap());
        self
    }

    /// Maximum number of bytes the peer may transmit across all streams of a connection before
    /// becoming blocked.
    ///
    /// Please see docs in [`TransportConfig`] in [`quinn`].
    ///
    /// [`TransportConfig`]: https://docs.rs/quinn/latest/quinn/struct.TransportConfig.html
    ///
    /// # Panics
    ///
    /// Panics if the value is over 2^62.
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_conn_receive_window(mut self, value: u64) -> ClientBuilder {
        self.config.quic_receive_window = Some(value.try_into().unwrap());
        self
    }

    /// Maximum number of bytes to transmit to a peer without acknowledgment
    ///
    /// Please see docs in [`TransportConfig`] in [`quinn`].
    ///
    /// [`TransportConfig`]: https://docs.rs/quinn/latest/quinn/struct.TransportConfig.html
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_send_window(mut self, value: u64) -> ClientBuilder {
        self.config.quic_send_window = Some(value);
        self
    }

    /// Override the default congestion control algorithm to use [BBR]
    ///
    /// The current default congestion control algorithm is [CUBIC]. This method overrides the
    /// default.
    ///
    /// [BBR]: https://datatracker.ietf.org/doc/html/draft-ietf-ccwg-bbr
    /// [CUBIC]: https://datatracker.ietf.org/doc/html/rfc8312
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_congestion_bbr(mut self) -> ClientBuilder {
        self.config.quic_congestion_bbr = true;
        self
    }

    /// Set the maximum HTTP/3 header size this client is willing to accept.
    ///
    /// See [header size constraints] section of the specification for details.
    ///
    /// [header size constraints]: https://www.rfc-editor.org/rfc/rfc9114.html#name-header-size-constraints
    ///
    /// Please see docs in [`Builder`] in [`h3`].
    ///
    /// [`Builder`]: https://docs.rs/h3/latest/h3/client/struct.Builder.html#method.max_field_section_size
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_max_field_section_size(mut self, value: u64) -> ClientBuilder {
        self.config.h3_max_field_section_size = Some(value.try_into().unwrap());
        self
    }

    /// Enable whether to send HTTP/3 protocol grease on the connections.
    ///
    /// HTTP/3 uses the concept of "grease"
    ///
    /// to prevent potential interoperability issues in the future.
    /// In HTTP/3, the concept of grease is used to ensure that the protocol can evolve
    /// and accommodate future changes without breaking existing implementations.
    ///
    /// Please see docs in [`Builder`] in [`h3`].
    ///
    /// [`Builder`]: https://docs.rs/h3/latest/h3/client/struct.Builder.html#method.send_grease
    #[cfg(feature = "http3")]
    #[cfg_attr(docsrs, doc(cfg(all(reqwest_unstable, feature = "http3",))))]
    pub fn http3_send_grease(mut self, enabled: bool) -> ClientBuilder {
        self.config.h3_send_grease = Some(enabled);
        self
    }

    /// Adds a new Tower [`Layer`](https://docs.rs/tower/latest/tower/trait.Layer.html) to the
    /// base connector [`Service`](https://docs.rs/tower/latest/tower/trait.Service.html) which
    /// is responsible for connection establishment.
    ///
    /// Each subsequent invocation of this function will wrap previous layers.
    ///
    /// If configured, the `connect_timeout` will be the outermost layer.
    ///
    /// Example usage:
    /// ```
    /// use std::time::Duration;
    ///
    /// # #[cfg(not(feature = "rustls-no-provider"))]
    /// let client = reqwest::Client::builder()
    ///                      // resolved to outermost layer, meaning while we are waiting on concurrency limit
    ///                      .connect_timeout(Duration::from_millis(200))
    ///                      // underneath the concurrency check, so only after concurrency limit lets us through
    ///                      .connector_layer(tower::timeout::TimeoutLayer::new(Duration::from_millis(50)))
    ///                      .connector_layer(tower::limit::concurrency::ConcurrencyLimitLayer::new(2))
    ///                      .build()
    ///                      .unwrap();
    /// ```
    ///
    pub fn connector_layer<L>(mut self, layer: L) -> ClientBuilder
    where
        L: Layer<BoxedConnectorService> + Clone + Send + Sync + 'static,
        L::Service:
            Service<Unnameable, Response = Conn, Error = BoxError> + Clone + Send + Sync + 'static,
        <L::Service as Service<Unnameable>>::Future: Send + 'static,
    {
        let layer = BoxCloneSyncServiceLayer::new(layer);

        self.config.connector_layers.push(layer);

        self
    }
}

type HyperClient = hyper_util::client::legacy::Client<Connector, super::Body>;

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "__rustls")]
fn default_rustls_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    #[cfg(not(feature = "__rustls-aws-lc-rs"))]
    panic!(
        "No rustls crypto provider is configured. \
        When using the `rustls-no-provider` feature you must install a \
        crypto provider before building a Client. For example: \
        `rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();` \
        See https://docs.rs/rustls/latest/rustls/#cryptography-providers for details."
    );

    #[cfg(feature = "__rustls-aws-lc-rs")]
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

impl Client {
    /// Constructs a new `Client`.
    ///
    /// # Panics
    ///
    /// This method panics if a TLS backend cannot be initialized, or the resolver
    /// cannot load the system configuration.
    ///
    /// Use `Client::builder()` if you wish to handle the failure as an `Error`
    /// instead of panicking.
    pub fn new() -> Client {
        ClientBuilder::new().build().expect("Client::new()")
    }

    /// Creates a `ClientBuilder` to configure a `Client`.
    ///
    /// This is the same as `ClientBuilder::new()`.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Convenience method to make a `GET` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Convenience method to make a `POST` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Convenience method to make a `PUT` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Convenience method to make a `PATCH` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Convenience method to make a `DELETE` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Convenience method to make a `HEAD` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Start building a `Request` with the `Method` and `Url`.
    ///
    /// Returns a `RequestBuilder`, which will allow setting headers and
    /// the request body before sending.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        let req = url.into_url().map(move |url| Request::new(method, url));
        RequestBuilder::new(self.clone(), req)
    }

    /// Executes a `Request`.
    ///
    /// A `Request` can be built manually with `Request::new()` or obtained
    /// from a RequestBuilder with `RequestBuilder::build()`.
    ///
    /// You should prefer to use the `RequestBuilder` and
    /// `RequestBuilder::send()`.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    pub fn execute(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Response, crate::Error>> {
        self.execute_request(request)
    }

    pub(super) fn execute_request(&self, req: Request) -> Pending {
        let (method, url, mut headers, body, version, extensions) = req.pieces();
        if url.scheme() != "http" && url.scheme() != "https" {
            return Pending::new_err(error::url_bad_scheme(url));
        }

        // check if we're in https_only mode and check the scheme of the current URL
        if self.inner.https_only && url.scheme() != "https" {
            return Pending::new_err(error::url_bad_scheme(url));
        }

        // insert default headers in the request headers
        // without overwriting already appended headers.
        for (key, value) in &self.inner.headers {
            if let Entry::Vacant(entry) = headers.entry(key) {
                entry.insert(value.clone());
            }
        }

        let uri = match try_uri(&url) {
            Ok(uri) => uri,
            _ => return Pending::new_err(error::url_invalid_uri(url)),
        };

        let body = body.unwrap_or_else(Body::empty);

        self.proxy_auth(&uri, &mut headers);
        self.proxy_custom_headers(&uri, &mut headers);

        let builder = hyper::Request::builder()
            .method(method.clone())
            .uri(uri)
            .version(version);

        let in_flight = match version {
            #[cfg(feature = "http3")]
            http::Version::HTTP_3 if self.inner.h3_client.is_some() => {
                let mut req = builder.body(body).expect("valid request parts");
                *req.headers_mut() = headers;
                let mut h3 = self.inner.h3_client.as_ref().unwrap().clone();
                ResponseFuture::H3(h3.call(req))
            }
            _ => {
                let mut req = builder.body(body).expect("valid request parts");
                *req.headers_mut() = headers;
                let mut hyper = self.inner.hyper.clone();
                ResponseFuture::Default(hyper.call(req))
            }
        };

        let total_timeout = self
            .inner
            .total_timeout
            .fetch(&extensions)
            .copied()
            .map(tokio::time::sleep)
            .map(Box::pin);

        let read_timeout_fut = self
            .inner
            .read_timeout
            .map(tokio::time::sleep)
            .map(Box::pin);

        Pending {
            inner: PendingInner::Request(Box::pin(PendingRequest {
                method,
                url,

                client: self.inner.clone(),

                in_flight,
                total_timeout,
                read_timeout_fut,
                read_timeout: self.inner.read_timeout,
            })),
        }
    }

    fn proxy_auth(&self, dst: &Uri, headers: &mut HeaderMap) {
        if !self.inner.proxies_maybe_http_auth {
            return;
        }

        // Only set the header here if the destination scheme is 'http',
        // since otherwise, the header will be included in the CONNECT tunnel
        // request instead.
        if dst.scheme() != Some(&Scheme::HTTP) {
            return;
        }

        if headers.contains_key(PROXY_AUTHORIZATION) {
            return;
        }

        for proxy in self.inner.proxies.iter() {
            if let Some(proxy) = proxy.intercept(dst) {
                if let Some(header) = proxy.http_non_tunnel_basic_auth() {
                    headers.insert(PROXY_AUTHORIZATION, header);
                }
                // Use only the first matching proxy, as the connector does.
                break;
            }
        }
    }

    fn proxy_custom_headers(&self, dst: &Uri, headers: &mut HeaderMap) {
        if !self.inner.proxies_maybe_http_custom_headers {
            return;
        }

        if dst.scheme() != Some(&Scheme::HTTP) {
            return;
        }

        for proxy in self.inner.proxies.iter() {
            if let Some(proxy) = proxy.intercept(dst) {
                if let Some(iter) = proxy.http_non_tunnel_custom_headers() {
                    iter.iter().for_each(|(key, value)| {
                        headers.insert(key, value.clone());
                    });
                }
                // Use only the first matching proxy, as the connector does.
                break;
            }
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("Client");
        self.inner.fmt_fields(&mut builder);
        builder.finish()
    }
}

impl tower_service::Service<Request> for Client {
    type Response = Response;
    type Error = crate::Error;
    type Future = Pending;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.execute_request(req)
    }
}

impl tower_service::Service<Request> for &'_ Client {
    type Response = Response;
    type Error = crate::Error;
    type Future = Pending;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.execute_request(req)
    }
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("ClientBuilder");
        self.config.fmt_fields(&mut builder);
        builder.finish()
    }
}

impl Config {
    fn fmt_fields(&self, f: &mut fmt::DebugStruct<'_, '_>) {
        // Instead of deriving Debug, only print fields when their output
        // would provide relevant or interesting data.

        #[cfg(feature = "cookies")]
        {
            if let Some(_) = self.cookie_store {
                f.field("cookie_store", &true);
            }
        }

        f.field("accepts", &self.accepts);

        if !self.proxies.is_empty() {
            f.field("proxies", &self.proxies);
        }

        if !self.redirect_policy.is_default() {
            f.field("redirect_policy", &self.redirect_policy);
        }

        if self.referer {
            f.field("referer", &true);
        }

        f.field("default_headers", &self.headers);

        if self.http1_title_case_headers {
            f.field("http1_title_case_headers", &true);
        }

        if self.http1_allow_obsolete_multiline_headers_in_responses {
            f.field("http1_allow_obsolete_multiline_headers_in_responses", &true);
        }

        if self.http1_ignore_invalid_headers_in_responses {
            f.field("http1_ignore_invalid_headers_in_responses", &true);
        }

        if self.http1_allow_spaces_after_header_name_in_responses {
            f.field("http1_allow_spaces_after_header_name_in_responses", &true);
        }

        if matches!(self.http_version_pref, HttpVersionPref::Http1) {
            f.field("http1_only", &true);
        }

        #[cfg(feature = "http2")]
        if matches!(self.http_version_pref, HttpVersionPref::Http2) {
            f.field("http2_prior_knowledge", &true);
        }

        if let Some(ref d) = self.connect_timeout {
            f.field("connect_timeout", d);
        }

        if let Some(ref d) = self.timeout {
            f.field("timeout", d);
        }

        if let Some(ref v) = self.local_address {
            f.field("local_address", v);
        }

        #[cfg(any(
            target_os = "android",
            target_os = "fuchsia",
            target_os = "illumos",
            target_os = "ios",
            target_os = "linux",
            target_os = "macos",
            target_os = "solaris",
            target_os = "tvos",
            target_os = "visionos",
            target_os = "watchos",
        ))]
        if let Some(ref v) = self.interface {
            f.field("interface", v);
        }

        if self.nodelay {
            f.field("tcp_nodelay", &true);
        }

        #[cfg(feature = "__tls")]
        {
            if !self.hostname_verification {
                f.field("tls_danger_accept_invalid_hostnames", &true);
            }
        }

        #[cfg(feature = "__tls")]
        {
            if !self.certs_verification {
                f.field("tls_danger_accept_invalid_certs", &true);
            }

            if let Some(ref min_tls_version) = self.min_tls_version {
                f.field("tls_version_min", min_tls_version);
            }

            if let Some(ref max_tls_version) = self.max_tls_version {
                f.field("tls_version_max", max_tls_version);
            }

            f.field("tls_sni", &self.tls_sni);

            f.field("tls_info", &self.tls_info);
        }

        #[cfg(feature = "__rustls")]
        {
            f.field("tls_sslkeylogfile", &self.tls_sslkeylogfile);
        }

        #[cfg(all(feature = "default-tls", feature = "__rustls"))]
        {
            f.field("tls_backend", &self.tls);
        }

        if !self.dns_overrides.is_empty() {
            f.field("dns_overrides", &self.dns_overrides);
        }

        #[cfg(feature = "http3")]
        {
            if self.tls_enable_early_data {
                f.field("tls_enable_early_data", &true);
            }
        }

        #[cfg(unix)]
        if let Some(ref p) = self.unix_socket {
            f.field("unix_socket", p);
        }
    }
}

#[cfg(not(feature = "cookies"))]
type MaybeCookieService<T> = T;

#[cfg(feature = "cookies")]
type MaybeCookieService<T> = CookieService<T>;

#[cfg(not(any(
    feature = "gzip",
    feature = "brotli",
    feature = "zstd",
    feature = "deflate"
)))]
type MaybeDecompression<T> = T;

#[cfg(any(
    feature = "gzip",
    feature = "brotli",
    feature = "zstd",
    feature = "deflate"
))]
type MaybeDecompression<T> = Decompression<T>;

type LayeredService<T> = MaybeDecompression<
    FollowRedirect<
        MaybeCookieService<tower::retry::Retry<crate::retry::Policy, T>>,
        TowerRedirectPolicy,
    >,
>;
type LayeredFuture<T> = <LayeredService<T> as Service<http::Request<Body>>>::Future;

struct ClientRef {
    accepts: Accepts,
    #[cfg(feature = "cookies")]
    cookie_store: Option<Arc<dyn cookie::CookieStore>>,
    headers: HeaderMap,
    hyper: LayeredService<HyperService>,
    #[cfg(feature = "http3")]
    h3_client: Option<LayeredService<H3Client>>,
    referer: bool,
    total_timeout: RequestConfig<TotalTimeout>,
    read_timeout: Option<Duration>,
    proxies: Arc<Vec<ProxyMatcher>>,
    proxies_maybe_http_auth: bool,
    proxies_maybe_http_custom_headers: bool,
    https_only: bool,
    redirect_policy_desc: Option<String>,
}

impl ClientRef {
    fn fmt_fields(&self, f: &mut fmt::DebugStruct<'_, '_>) {
        // Instead of deriving Debug, only print fields when their output
        // would provide relevant or interesting data.

        #[cfg(feature = "cookies")]
        {
            if let Some(_) = self.cookie_store {
                f.field("cookie_store", &true);
            }
        }

        f.field("accepts", &self.accepts);

        if !self.proxies.is_empty() {
            f.field("proxies", &self.proxies);
        }

        if let Some(s) = &self.redirect_policy_desc {
            f.field("redirect_policy", s);
        }

        if self.referer {
            f.field("referer", &true);
        }

        f.field("default_headers", &self.headers);

        self.total_timeout.fmt_as_field(f);

        if let Some(ref d) = self.read_timeout {
            f.field("read_timeout", d);
        }
    }
}

pin_project! {
    pub struct Pending {
        #[pin]
        inner: PendingInner,
    }
}

enum PendingInner {
    Request(Pin<Box<PendingRequest>>),
    Error(Option<crate::Error>),
}

pin_project! {
    struct PendingRequest {
        method: Method,
        url: Url,

        client: Arc<ClientRef>,

        #[pin]
        in_flight: ResponseFuture,
        #[pin]
        total_timeout: Option<Pin<Box<Sleep>>>,
        #[pin]
        read_timeout_fut: Option<Pin<Box<Sleep>>>,
        read_timeout: Option<Duration>,
    }
}

enum ResponseFuture {
    Default(LayeredFuture<HyperService>),
    #[cfg(feature = "http3")]
    H3(LayeredFuture<H3Client>),
}

impl PendingRequest {
    fn in_flight(self: Pin<&mut Self>) -> Pin<&mut ResponseFuture> {
        self.project().in_flight
    }

    fn total_timeout(self: Pin<&mut Self>) -> Pin<&mut Option<Pin<Box<Sleep>>>> {
        self.project().total_timeout
    }

    fn read_timeout(self: Pin<&mut Self>) -> Pin<&mut Option<Pin<Box<Sleep>>>> {
        self.project().read_timeout_fut
    }
}

impl Pending {
    pub(super) fn new_err(err: crate::Error) -> Pending {
        Pending {
            inner: PendingInner::Error(Some(err)),
        }
    }

    fn inner(self: Pin<&mut Self>) -> Pin<&mut PendingInner> {
        self.project().inner
    }
}

impl Future for Pending {
    type Output = Result<Response, crate::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner();
        match inner.get_mut() {
            PendingInner::Request(ref mut req) => Pin::new(req).poll(cx),
            PendingInner::Error(ref mut err) => Poll::Ready(Err(err
                .take()
                .expect("Pending error polled more than once"))),
        }
    }
}

impl Future for PendingRequest {
    type Output = Result<Response, crate::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(delay) = self.as_mut().total_timeout().as_mut().as_pin_mut() {
            if let Poll::Ready(()) = delay.poll(cx) {
                return Poll::Ready(Err(
                    crate::error::request(crate::error::TimedOut).with_url(self.url.clone())
                ));
            }
        }

        if let Some(delay) = self.as_mut().read_timeout().as_mut().as_pin_mut() {
            if let Poll::Ready(()) = delay.poll(cx) {
                return Poll::Ready(Err(
                    crate::error::request(crate::error::TimedOut).with_url(self.url.clone())
                ));
            }
        }

        let res = match self.as_mut().in_flight().get_mut() {
            ResponseFuture::Default(r) => match ready!(Pin::new(r).poll(cx)) {
                Err(e) => {
                    return Poll::Ready(Err(e.if_no_url(|| self.url.clone())));
                }
                Ok(res) => res.map(super::body::boxed),
            },
            #[cfg(feature = "http3")]
            ResponseFuture::H3(r) => match ready!(Pin::new(r).poll(cx)) {
                Err(e) => {
                    return Poll::Ready(Err(crate::error::request(e).with_url(self.url.clone())));
                }
                Ok(res) => res.map(super::body::boxed),
            },
        };

        if let Some(url) = &res
            .extensions()
            .get::<tower_http::follow_redirect::RequestUri>()
        {
            self.url = match Url::parse(&url.0.to_string()) {
                Ok(url) => url,
                Err(e) => return Poll::Ready(Err(crate::error::decode(e))),
            }
        };

        let res = Response::new(
            res,
            self.url.clone(),
            self.total_timeout.take(),
            self.read_timeout,
        );
        Poll::Ready(Ok(res))
    }
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.inner {
            PendingInner::Request(ref req) => f
                .debug_struct("Pending")
                .field("method", &req.method)
                .field("url", &req.url)
                .finish(),
            PendingInner::Error(ref err) => f.debug_struct("Pending").field("error", err).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![cfg(not(feature = "rustls-no-provider"))]

    #[tokio::test]
    async fn execute_request_rejects_invalid_urls() {
        let url_str = "hxxps://www.rust-lang.org/";
        let url = url::Url::parse(url_str).unwrap();
        let result = crate::get(url.clone()).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.is_builder());
        assert_eq!(url_str, err.url().unwrap().as_str());
    }

    /// https://github.com/seanmonstar/reqwest/issues/668
    #[tokio::test]
    async fn execute_request_rejects_invalid_hostname() {
        let url_str = "https://{{hostname}}/";
        let url = url::Url::parse(url_str).unwrap();
        let result = crate::get(url.clone()).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.is_builder());
        assert_eq!(url_str, err.url().unwrap().as_str());
    }

    #[test]
    fn test_future_size() {
        let s = std::mem::size_of::<super::Pending>();
        assert!(s < 128, "size_of::<Pending>() == {s}, too big");
    }
}
