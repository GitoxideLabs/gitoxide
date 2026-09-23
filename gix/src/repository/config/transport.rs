use std::any::Any;

use crate::error::{ResultExt, message};
use crate::{Result, bstr::BStr};

impl crate::Repository {
    /// Produce configuration suitable for `url`, as differentiated by its protocol/scheme, to be passed to a transport instance via
    /// [configure()][gix_transport::client::TransportWithoutIO::configure()] (via `&**config` to pass the contained `Any` and not the `Box`).
    /// `None` is returned if there is no known configuration.
    /// If `remote_name` is `Some`, its bytes are used as the exact subsection name in `[remote "<name>"]`
    /// when looking up transport overrides. Passing `None` skips remote-specific overrides.
    ///
    /// Note that the caller may cast the instance themselves to modify it before passing it on.
    ///
    /// For transports that support proxy authentication, the
    /// [default authentication method](crate::config::Snapshot::credential_helpers()) will be used with the url of the proxy
    /// if it contains a user name.
    #[cfg_attr(
        not(any(
            feature = "blocking-http-transport-reqwest",
            feature = "blocking-http-transport-curl"
        )),
        allow(unused_variables)
    )]
    pub fn transport_options<'a>(
        &self,
        url: impl Into<&'a BStr>,
        remote_name: Option<&BStr>,
    ) -> Result<Option<Box<dyn Any>>> {
        let url = gix_url::parse(url.into()).or_raise(|| message("Invalid URL passed for configuration"))?;
        use gix_url::Scheme::*;

        match &url.scheme {
            Http | Https => {
                let options: Result<Option<Box<dyn Any>>> = {
                    #[cfg(not(any(
                        feature = "blocking-http-transport-reqwest",
                        feature = "blocking-http-transport-curl"
                    )))]
                    {
                        Ok(None)
                    }
                    #[cfg(any(
                        feature = "blocking-http-transport-reqwest",
                        feature = "blocking-http-transport-curl"
                    ))]
                    {
                        use std::sync::{Arc, Mutex};

                        use gix_transport::client::blocking_io::http::{
                            self,
                            options::{ProxyAuthMethod, SslVersion, SslVersionRangeInclusive},
                        };

                        use crate::{
                            bstr::BString,
                            config,
                            config::{
                                cache::util::ApplyLeniency,
                                tree::{Key, Remote, gitoxide},
                            },
                        };

                        fn try_to_string(
                            v: BString,
                            lenient: bool,
                            remote_name: Option<&BStr>,
                            key: &'static config::tree::keys::String,
                        ) -> Result<Option<String>> {
                            let value = key.try_into_string(v);
                            let value = match remote_name {
                                Some(name) => value.or_raise(|| message!("Invalid configuration for remote {name:?}")),
                                None => value,
                            };
                            value.map(Some).with_leniency(lenient)
                        }

                        fn proxy_auth_method(
                            value_and_key: Option<(
                                BString,
                                Option<&BStr>,
                                &'static config::tree::http::ProxyAuthMethod,
                            )>,
                        ) -> Result<ProxyAuthMethod> {
                            let value = value_and_key
                                .map(|(method, remote_name, key)| {
                                    let method = key.try_into_proxy_auth_method(method);
                                    match remote_name {
                                        Some(name) => {
                                            method.or_raise(|| message!("Invalid configuration for remote {name:?}"))
                                        }
                                        None => method,
                                    }
                                })
                                .transpose()?
                                .unwrap_or_default();
                            Ok(value)
                        }

                        fn ssl_version(
                            config: &gix_config::File,
                            key_str: &'static str,
                            key: &'static config::tree::http::SslVersion,
                            mut filter: fn(&gix_config::file::Metadata) -> bool,
                            lenient: bool,
                        ) -> Result<Option<SslVersion>> {
                            debug_assert_eq!(
                                key_str,
                                key.logical_name(),
                                "BUG: hardcoded and generated key names must match"
                            );
                            config
                                .string_filter(key_str, &mut filter)
                                .filter(|v| !v.is_empty())
                                .map(|v| key.try_into_ssl_version(v))
                                .transpose()
                                .with_leniency(lenient)
                        }

                        fn proxy(
                            value: Option<(BString, Option<&BStr>, &'static config::tree::keys::String)>,
                            lenient: bool,
                        ) -> Result<Option<String>> {
                            Ok(value
                                .and_then(|(v, remote_name, key)| {
                                    try_to_string(v, lenient, remote_name, key).transpose()
                                })
                                .transpose()?
                                .map(|mut proxy| {
                                    if !proxy.trim().is_empty() && !proxy.contains("://") {
                                        proxy.insert_str(0, "http://");
                                        proxy
                                    } else {
                                        proxy
                                    }
                                }))
                        }

                        let mut opts = http::Options::default();
                        let config = &self.config.resolved;
                        let mut trusted_only = self.filter_config_section();
                        let lenient = self.config.lenient_config;
                        opts.extra_headers = {
                            let key = "http.extraHeader";
                            debug_assert_eq!(key, &config::tree::Http::EXTRA_HEADER.logical_name());
                            config
                                .strings_filter(key, &mut trusted_only)
                                .map(|values| config::tree::Http::EXTRA_HEADER.try_into_extra_header(values))
                                .transpose()?
                                .unwrap_or_default()
                        };

                        opts.follow_redirects = {
                            let key = "http.followRedirects";

                            config::tree::Http::FOLLOW_REDIRECTS.try_into_follow_redirects(
                                config.string_filter(key, &mut trusted_only).unwrap_or_default(),
                                || config.boolean_filter(key, &mut trusted_only).with_leniency(lenient),
                            )?
                        };

                        opts.low_speed_time_seconds = config::tree::Http::LOW_SPEED_TIME
                            .try_into_u64(config.integer_filter("http.lowSpeedTime", &mut trusted_only))
                            .with_leniency(lenient)?
                            .unwrap_or_default();
                        opts.low_speed_limit_bytes_per_second = config::tree::Http::LOW_SPEED_LIMIT
                            .try_into_u32(config.integer_filter("http.lowSpeedLimit", &mut trusted_only))
                            .with_leniency(lenient)?
                            .unwrap_or_default();
                        // Reqwest selects environment proxies per destination, including after redirects.
                        // Keep curl's existing configuration and credential-helper setup.
                        let mut explicit_proxy = |meta: &gix_config::file::Metadata| {
                            (cfg!(feature = "blocking-http-transport-curl")
                                || meta.source != gix_config::Source::EnvOverride)
                                && trusted_only(meta)
                        };
                        opts.proxy = proxy(
                            remote_name
                                .and_then(|name| {
                                    config
                                        .string_filter_by("remote", Some(name), Remote::PROXY.name, &mut explicit_proxy)
                                        .map(|v| (v, Some(name), &Remote::PROXY))
                                })
                                .or_else(|| {
                                    let key = "http.proxy";
                                    debug_assert_eq!(key, config::tree::Http::PROXY.logical_name());
                                    let http_proxy = config
                                        .string_filter(key, &mut explicit_proxy)
                                        .map(|v| (v, None, &config::tree::Http::PROXY))
                                        .or_else(|| {
                                            let key = "gitoxide.http.proxy";
                                            debug_assert_eq!(key, gitoxide::Http::PROXY.logical_name());
                                            config
                                                .string_filter(key, &mut explicit_proxy)
                                                .map(|v| (v, None, &gitoxide::Http::PROXY))
                                        });
                                    if url.scheme == Https {
                                        http_proxy.or_else(|| {
                                            let key = "gitoxide.https.proxy";
                                            debug_assert_eq!(key, gitoxide::Https::PROXY.logical_name());
                                            config
                                                .string_filter(key, &mut explicit_proxy)
                                                .map(|v| (v, None, &gitoxide::Https::PROXY))
                                        })
                                    } else {
                                        http_proxy
                                    }
                                })
                                .or_else(|| {
                                    let key = "gitoxide.http.allProxy";
                                    debug_assert_eq!(key, gitoxide::Http::ALL_PROXY.logical_name());
                                    config
                                        .string_filter(key, &mut explicit_proxy)
                                        .map(|v| (v, None, &gitoxide::Http::ALL_PROXY))
                                }),
                            lenient,
                        )?;
                        {
                            let key = "gitoxide.http.noProxy";
                            debug_assert_eq!(key, gitoxide::Http::NO_PROXY.logical_name());
                            opts.no_proxy = config
                                .string_filter(key, &mut explicit_proxy)
                                .and_then(|v| try_to_string(v, lenient, None, &gitoxide::Http::NO_PROXY).transpose())
                                .transpose()?;
                        }
                        opts.proxy_auth_method = proxy_auth_method({
                            let key = "gitoxide.http.proxyAuthMethod";
                            debug_assert_eq!(key, gitoxide::Http::PROXY_AUTH_METHOD.logical_name());
                            config
                                .string_filter(key, &mut trusted_only)
                                .map(|v| (v, None, &gitoxide::Http::PROXY_AUTH_METHOD))
                                .or_else(|| {
                                    remote_name
                                        .and_then(|name| {
                                            config
                                                .string_filter_by(
                                                    "remote",
                                                    Some(name),
                                                    Remote::PROXY_AUTH_METHOD.name,
                                                    &mut trusted_only,
                                                )
                                                .map(|v| (v, Some(name), &Remote::PROXY_AUTH_METHOD))
                                        })
                                        .or_else(|| {
                                            let key = "http.proxyAuthMethod";
                                            debug_assert_eq!(key, config::tree::Http::PROXY_AUTH_METHOD.logical_name());
                                            config
                                                .string_filter(key, &mut trusted_only)
                                                .map(|v| (v, None, &config::tree::Http::PROXY_AUTH_METHOD))
                                        })
                                })
                        })?;
                        opts.proxy_authenticate = opts
                            .proxy
                            .as_deref()
                            .filter(|url| !url.is_empty())
                            .map(gix_url::parse)
                            .transpose()
                            .or_raise(|| message("Invalid URL passed for configuration"))?
                            .filter(|url| url.user().is_some())
                            .map(|url| -> Result<_> {
                                let (mut cascade, action_with_normalized_url, prompt_opts) =
                                self.config_snapshot().credential_helpers(url).or_raise(|| {
                                    message(
                                        "Could not configure the credential helpers for the authenticated proxy url",
                                    )
                                })?;
                                Ok((
                                    action_with_normalized_url,
                                    Arc::new(Mutex::new(move |action| cascade.invoke(action, prompt_opts.clone())))
                                        as Arc<Mutex<http::options::AuthenticateFn>>,
                                ))
                            })
                            .transpose()?;
                        #[cfg(all(
                            feature = "blocking-http-transport-reqwest",
                            not(feature = "blocking-http-transport-curl")
                        ))]
                        if opts.proxy.is_none() {
                            use crate::bstr::ByteSlice;
                            use gix_credentials::{
                                helper::Action,
                                protocol::{Context, ContextOptions},
                            };
                            use gix_error::ErrorExt;

                            // Prepare only the imported proxy candidates: the callback must be Send + Sync,
                            // whereas the repository/configuration can use Rc in non-parallel builds.
                            let mut candidates = Vec::new();
                            for key in ["gitoxide.http.proxy", "gitoxide.https.proxy", "gitoxide.http.allProxy"] {
                                for value in config
                                    .strings_filter(key, &mut |meta: &gix_config::file::Metadata| {
                                        meta.source == gix_config::Source::EnvOverride && trusted_only(meta)
                                    })
                                    .into_iter()
                                    .flatten()
                                {
                                    let Some(value) = value.to_str().ok().filter(|value| !value.is_empty()) else {
                                        continue;
                                    };
                                    let value = if value.contains("://") {
                                        value.to_owned()
                                    } else {
                                        format!("http://{value}")
                                    };
                                    let Ok(mut url) = gix_url::parse(value.as_str()) else {
                                        continue; // The backend reports invalid settings only when selected.
                                    };
                                    if url.user().is_none() {
                                        continue;
                                    }
                                    let helpers =
                                        self.config_snapshot().credential_helpers(url.clone()).map_err(Arc::new);
                                    // Match the backend's routing URL, but preserve the original declaration for
                                    // URL-specific credential configuration and credential-store lookups.
                                    if url.scheme == Http && url.port.is_none() {
                                        url.port = Some(1080);
                                    }
                                    candidates.push((url.to_bstring(), helpers));
                                }
                            }
                            if let Some((first_url, _)) = candidates.first() {
                                let action = Action::get_for_url(first_url.clone());
                                opts.proxy_authenticate = Some((
                                    action,
                                    Arc::new(Mutex::new(move |mut action: Action| {
                                        let get = action.expects_output();
                                        let selected = match &mut action {
                                            Action::Get(context) => {
                                                let url = context.url.clone().or_else(|| context.to_url()).ok_or_else(
                                                    || {
                                                        gix_error::message(
                                                            "The URL for proxy authentication is missing",
                                                        )
                                                        .raise()
                                                    },
                                                )?;
                                                let url = gix_url::parse(&url)?.to_bstring();
                                                candidates.iter().position(|(candidate, _)| candidate == &url)
                                            }
                                            Action::Store(payload) | Action::Erase(payload) => {
                                                let Some((index, original)) = payload
                                                    .strip_prefix(b"url=gix-proxy:")
                                                    .and_then(|payload| payload.strip_suffix(b"\n"))
                                                    .and_then(|url| url.split_once_str(b":"))
                                                else {
                                                    return Ok(None);
                                                };
                                                if original.iter().any(|byte| matches!(byte, 0 | b'\n' | b'\r')) {
                                                    return Err(message("Invalid proxy authentication handle").raise());
                                                }
                                                let index = index.to_str().or_error()?.parse::<usize>().or_error()?;
                                                *payload = original
                                                    .replace(b"%0D", b"\r")
                                                    .replace(b"%0A", b"\n")
                                                    .replace(b"%25", b"%")
                                                    .into();
                                                Some(index)
                                            }
                                        };
                                        let Some(index) = selected else { return Ok(None) };
                                        let Some((_, helpers)) = candidates.get_mut(index) else {
                                            return Ok(None);
                                        };
                                        let (cascade, normalized_action, prompt_opts) =
                                            helpers.as_mut().map_err(|source| {
                                                source.clone().and_raise(gix_error::message(
                                                    "Could not configure credential helpers for the proxy URL",
                                                ))
                                            })?;
                                        let mut outcome = cascade.invoke(
                                            if get { normalized_action.clone() } else { action },
                                            prompt_opts.clone(),
                                        )?;
                                        if let Some(outcome) = &mut outcome
                                            && let Action::Store(payload) = outcome.next.clone().store()
                                        {
                                            // Keep routing in the handle: clones may obtain identical helper credentials.
                                            let mut url = format!("gix-proxy:{index}:").into_bytes();
                                            url.extend(
                                                payload
                                                    .replace(b"%", b"%25")
                                                    .replace(b"\n", b"%0A")
                                                    .replace(b"\r", b"%0D"),
                                            );
                                            outcome.next = Context::from_url(url, ContextOptions::default()).into();
                                        }
                                        Ok(outcome)
                                    }))
                                        as Arc<Mutex<http::options::AuthenticateFn>>,
                                ));
                            }
                        }
                        opts.connect_timeout = {
                            let key = "gitoxide.http.connectTimeout";
                            debug_assert_eq!(key, gitoxide::Http::CONNECT_TIMEOUT.logical_name());
                            gitoxide::Http::CONNECT_TIMEOUT
                                .try_into_duration(config.integer_filter(key, &mut trusted_only))
                                .with_leniency(lenient)?
                        };
                        {
                            let key = "http.userAgent";
                            opts.user_agent = config
                                .string_filter(key, &mut trusted_only)
                                .and_then(|v| {
                                    try_to_string(v, lenient, None, &config::tree::Http::USER_AGENT).transpose()
                                })
                                .transpose()?
                                .or_else(|| Some(crate::env::agent().into()));
                        }

                        {
                            let key = "http.version";
                            opts.http_version = config
                                .string_filter(key, &mut trusted_only)
                                .map(|v| config::tree::Http::VERSION.try_into_http_version(v))
                                .transpose()?;
                        }

                        {
                            opts.verbose = config
                                .boolean_filter(gitoxide::Http::VERBOSE, &mut trusted_only)
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                        }

                        let may_use_cainfo = {
                            let key = "http.schannelUseSSLCAInfo";
                            config::tree::Http::SCHANNEL_USE_SSL_CA_INFO
                                .enrich_error(config.boolean_filter(key, &mut trusted_only))
                                .with_leniency(lenient)?
                                .unwrap_or(true)
                        };

                        if may_use_cainfo {
                            let key = "http.sslCAInfo";
                            debug_assert_eq!(key, config::tree::Http::SSL_CA_INFO.logical_name());
                            opts.ssl_ca_info = config
                                .path_filter(key, &mut trusted_only)
                                .map(|path| self.interpolate_path(path))
                                .transpose()
                                .with_leniency(lenient)
                                .or_raise(|| message!("Could not interpolate path at key {key:?}"))?;
                        }

                        {
                            opts.ssl_version = ssl_version(
                                config,
                                "http.sslVersion",
                                &config::tree::Http::SSL_VERSION,
                                trusted_only,
                                lenient,
                            )?
                            .map(|v| SslVersionRangeInclusive { min: v, max: v });
                            let min_max = ssl_version(
                                config,
                                "gitoxide.http.sslVersionMin",
                                &gitoxide::Http::SSL_VERSION_MIN,
                                trusted_only,
                                lenient,
                            )
                            .and_then(|min| {
                                ssl_version(
                                    config,
                                    "gitoxide.http.sslVersionMax",
                                    &gitoxide::Http::SSL_VERSION_MAX,
                                    trusted_only,
                                    lenient,
                                )
                                .map(|max| min.zip(max))
                            })?;
                            if let Some((min, max)) = min_max {
                                let v = opts.ssl_version.get_or_insert(SslVersionRangeInclusive {
                                    min: SslVersion::TlsV1_3,
                                    max: SslVersion::TlsV1_3,
                                });
                                v.min = min;
                                v.max = max;
                            }
                        }

                        {
                            let key = "gitoxide.http.sslNoVerify";
                            let ssl_no_verify = config::tree::gitoxide::Http::SSL_NO_VERIFY
                                .enrich_error(config.boolean_filter(key, &mut trusted_only))
                                .with_leniency(lenient)?
                                .unwrap_or_default();

                            if ssl_no_verify {
                                opts.ssl_verify = false;
                            } else {
                                let key = "http.sslVerify";
                                opts.ssl_verify = config::tree::Http::SSL_VERIFY
                                    .enrich_error(config.boolean_filter(key, &mut trusted_only))
                                    .with_leniency(lenient)?
                                    .unwrap_or(true);
                            }
                        }

                        #[cfg(feature = "blocking-http-transport-curl")]
                        {
                            let key = "http.schannelCheckRevoke";
                            let schannel_check_revoke = config::tree::Http::SCHANNEL_CHECK_REVOKE
                                .enrich_error(config.boolean_filter(key, &mut trusted_only))
                                .with_leniency(lenient)?;
                            let backend = gix_protocol::transport::client::blocking_io::http::curl::Options {
                                schannel_check_revoke,
                            };
                            opts.backend =
                                Some(Arc::new(Mutex::new(backend)) as Arc<Mutex<dyn Any + Send + Sync + 'static>>);
                        }

                        Ok(Some(Box::new(opts) as Box<dyn Any>))
                    }
                };
                options.or_raise(|| message("Could obtain configuration for an HTTP url"))
            }
            File | Git | Ssh | Ext | Helper(_) | HelperUrl(_) => Ok(None),
        }
    }
}
