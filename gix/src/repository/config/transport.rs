#![allow(clippy::result_large_err)]
use std::any::Any;

use gix_macros::momo;

use crate::bstr::BStr;

impl crate::Repository {
    /// Produce configuration suitable for `url`, as differentiated by its protocol/scheme, to be passed to a transport instance via
    /// [configure()][gix_transport::client::TransportWithoutIO::configure()] (via `&**config` to pass the contained `Any` and not the `Box`).
    /// `None` is returned if there is no known configuration. If `remote_name` is not `None`, the remote's name may contribute to
    /// configuration overrides, typically for the HTTP transport.
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
    #[momo]
    pub fn transport_options<'a>(
        &self,
        url: impl Into<&'a BStr>,
        remote_name: Option<&BStr>,
    ) -> Result<Option<Box<dyn Any>>, crate::config::transport::Error> {
        let url = gix_url::parse(url.into())?;
        use gix_url::Scheme::*;

        match &url.scheme {
            Http | Https => {
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
                    use std::{
                        borrow::Cow,
                        sync::{Arc, Mutex},
                    };

                    use gix_transport::client::{
                        http,
                        http::options::{ProxyAuthMethod, SslVersion, SslVersionRangeInclusive},
                    };

                    use crate::{
                        config,
                        config::{
                            cache::util::ApplyLeniency,
                            tree::{gitoxide, Key, Remote},
                        },
                    };
                    fn try_cow_to_string(
                        v: Cow<'_, BStr>,
                        lenient: bool,
                        key_str: impl Into<Cow<'static, BStr>>,
                        key: &'static config::tree::keys::String,
                    ) -> Result<Option<String>, config::transport::Error> {
                        key.try_into_string(v)
                            .map_err(|err| config::transport::Error::IllformedUtf8 {
                                source: err,
                                key: key_str.into(),
                            })
                            .map(Some)
                            .with_leniency(lenient)
                    }

                    fn cow_bstr(v: &str) -> Cow<'_, BStr> {
                        Cow::Borrowed(v.into())
                    }

                    fn proxy_auth_method(
                        value_and_key: Option<(
                            Cow<'_, BStr>,
                            Cow<'static, BStr>,
                            &'static config::tree::http::ProxyAuthMethod,
                        )>,
                    ) -> Result<ProxyAuthMethod, config::transport::Error> {
                        let value = value_and_key
                            .map(|(method, key, key_type)| {
                                key_type.try_into_proxy_auth_method(method).map_err(|err| {
                                    config::transport::http::Error::InvalidProxyAuthMethod { source: err, key }
                                })
                            })
                            .transpose()?
                            .unwrap_or_default();
                        Ok(value)
                    }

                    fn ssl_version(
                        config: &gix_config::File<'static>,
                        key_str: &'static str,
                        key: &'static config::tree::http::SslVersion,
                        mut filter: fn(&gix_config::file::Metadata) -> bool,
                        lenient: bool,
                    ) -> Result<Option<SslVersion>, config::transport::Error> {
                        debug_assert_eq!(
                            key_str,
                            key.logical_name(),
                            "BUG: hardcoded and generated key names must match"
                        );
                        config
                            .string_filter(key_str, &mut filter)
                            .filter(|v| !v.is_empty())
                            .map(|v| {
                                key.try_into_ssl_version(v)
                                    .map_err(crate::config::transport::http::Error::from)
                            })
                            .transpose()
                            .with_leniency(lenient)
                            .map_err(Into::into)
                    }

                    fn proxy(
                        value: Option<(Cow<'_, BStr>, Cow<'static, BStr>, &'static config::tree::keys::String)>,
                        lenient: bool,
                    ) -> Result<Option<String>, config::transport::Error> {
                        Ok(value
                            .and_then(|(v, k, key)| try_cow_to_string(v, lenient, k.clone(), key).transpose())
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
                            .transpose()
                            .map_err(|err| config::transport::Error::IllformedUtf8 {
                                source: err,
                                key: Cow::Borrowed(key.into()),
                            })?
                            .unwrap_or_default()
                    };

                    opts.follow_redirects = {
                        let key = "http.followRedirects";

                        config::tree::Http::FOLLOW_REDIRECTS
                            .try_into_follow_redirects(
                                config.string_filter(key, &mut trusted_only).unwrap_or_default(),
                                || {
                                    config
                                        .boolean_filter(key, &mut trusted_only)
                                        .transpose()
                                        .with_leniency(lenient)
                                },
                            )
                            .map_err(config::transport::http::Error::InvalidFollowRedirects)?
                    };

                    opts.low_speed_time_seconds = config
                        .integer_filter("http.lowSpeedTime", &mut trusted_only)
                        .map(|value| config::tree::Http::LOW_SPEED_TIME.try_into_u64(value))
                        .transpose()
                        .with_leniency(lenient)
                        .map_err(config::transport::http::Error::from)?
                        .unwrap_or_default();
                    opts.low_speed_limit_bytes_per_second = config
                        .integer_filter("http.lowSpeedLimit", &mut trusted_only)
                        .map(|value| config::tree::Http::LOW_SPEED_LIMIT.try_into_u32(value))
                        .transpose()
                        .with_leniency(lenient)
                        .map_err(config::transport::http::Error::from)?
                        .unwrap_or_default();
                    opts.proxy = proxy(
                        remote_name
                            .and_then(|name| {
                                config
                                    .string_filter(format!("remote.{}.{}", name, Remote::PROXY.name), &mut trusted_only)
                                    .map(|v| (v, Cow::Owned(format!("remote.{name}.proxy").into()), &Remote::PROXY))
                            })
                            .or_else(|| {
                                let key = "http.proxy";
                                debug_assert_eq!(key, config::tree::Http::PROXY.logical_name());
                                let http_proxy = config
                                    .string_filter(key, &mut trusted_only)
                                    .map(|v| (v, cow_bstr(key), &config::tree::Http::PROXY))
                                    .or_else(|| {
                                        let key = "gitoxide.http.proxy";
                                        debug_assert_eq!(key, gitoxide::Http::PROXY.logical_name());
                                        config
                                            .string_filter(key, &mut trusted_only)
                                            .map(|v| (v, cow_bstr(key), &gitoxide::Http::PROXY))
                                    });
                                if url.scheme == Https {
                                    http_proxy.or_else(|| {
                                        let key = "gitoxide.https.proxy";
                                        debug_assert_eq!(key, gitoxide::Https::PROXY.logical_name());
                                        config
                                            .string_filter(key, &mut trusted_only)
                                            .map(|v| (v, cow_bstr(key), &gitoxide::Https::PROXY))
                                    })
                                } else {
                                    http_proxy
                                }
                            })
                            .or_else(|| {
                                let key = "gitoxide.http.allProxy";
                                debug_assert_eq!(key, gitoxide::Http::ALL_PROXY.logical_name());
                                config
                                    .string_filter(key, &mut trusted_only)
                                    .map(|v| (v, cow_bstr(key), &gitoxide::Http::ALL_PROXY))
                            }),
                        lenient,
                    )?;
                    {
                        let key = "gitoxide.http.noProxy";
                        debug_assert_eq!(key, gitoxide::Http::NO_PROXY.logical_name());
                        opts.no_proxy = config
                            .string_filter(key, &mut trusted_only)
                            .and_then(|v| {
                                try_cow_to_string(v, lenient, Cow::Borrowed(key.into()), &gitoxide::Http::NO_PROXY)
                                    .transpose()
                            })
                            .transpose()?;
                    }
                    opts.proxy_auth_method = proxy_auth_method({
                        let key = "gitoxide.http.proxyAuthMethod";
                        debug_assert_eq!(key, gitoxide::Http::PROXY_AUTH_METHOD.logical_name());
                        config
                            .string_filter(key, &mut trusted_only)
                            .map(|v| (v, Cow::Borrowed(key.into()), &gitoxide::Http::PROXY_AUTH_METHOD))
                            .or_else(|| {
                                remote_name
                                    .and_then(|name| {
                                        config
                                            .string_filter(format!("remote.{name}.proxyAuthMethod"), &mut trusted_only)
                                            .map(|v| {
                                                (
                                                    v,
                                                    Cow::Owned(format!("remote.{name}.proxyAuthMethod").into()),
                                                    &Remote::PROXY_AUTH_METHOD,
                                                )
                                            })
                                    })
                                    .or_else(|| {
                                        let key = "http.proxyAuthMethod";
                                        debug_assert_eq!(key, config::tree::Http::PROXY_AUTH_METHOD.logical_name());
                                        config.string_filter(key, &mut trusted_only).map(|v| {
                                            (v, Cow::Borrowed(key.into()), &config::tree::Http::PROXY_AUTH_METHOD)
                                        })
                                    })
                            })
                    })?;
                    opts.proxy_authenticate = opts
                        .proxy
                        .as_deref()
                        .filter(|url| !url.is_empty())
                        .map(|url| gix_url::parse(url.into()))
                        .transpose()?
                        .filter(|url| url.user().is_some())
                        .map(|url| -> Result<_, config::transport::http::Error> {
                            let (mut cascade, action_with_normalized_url, prompt_opts) =
                                self.config_snapshot().credential_helpers(url)?;
                            Ok((
                                action_with_normalized_url,
                                Arc::new(Mutex::new(move |action| cascade.invoke(action, prompt_opts.clone())))
                                    as Arc<Mutex<http::options::AuthenticateFn>>,
                            ))
                        })
                        .transpose()?;
                    opts.connect_timeout = {
                        let key = "gitoxide.http.connectTimeout";
                        config
                            .integer_filter(key, &mut trusted_only)
                            .map(|v| {
                                debug_assert_eq!(key, gitoxide::Http::CONNECT_TIMEOUT.logical_name());
                                gitoxide::Http::CONNECT_TIMEOUT
                                    .try_into_duration(v)
                                    .map_err(crate::config::transport::http::Error::from)
                            })
                            .transpose()
                            .with_leniency(lenient)?
                    };
                    {
                        let key = "http.userAgent";
                        opts.user_agent = config
                            .string_filter(key, &mut trusted_only)
                            .and_then(|v| {
                                try_cow_to_string(
                                    v,
                                    lenient,
                                    Cow::Borrowed(key.into()),
                                    &config::tree::Http::USER_AGENT,
                                )
                                .transpose()
                            })
                            .transpose()?
                            .or_else(|| Some(crate::env::agent().into()));
                    }

                    {
                        let key = "http.version";
                        opts.http_version = config
                            .string_filter(key, &mut trusted_only)
                            .map(|v| {
                                config::tree::Http::VERSION
                                    .try_into_http_version(v)
                                    .map_err(config::transport::http::Error::InvalidHttpVersion)
                            })
                            .transpose()?;
                    }

                    {
                        opts.verbose = config
                            .boolean_filter(gitoxide::Http::VERBOSE, &mut trusted_only)
                            .and_then(Result::ok)
                            .unwrap_or_default();
                    }

                    let may_use_cainfo = {
                        let key = "http.schannelUseSSLCAInfo";
                        config
                            .boolean_filter(key, &mut trusted_only)
                            .map(|value| config::tree::Http::SCHANNEL_USE_SSL_CA_INFO.enrich_error(value))
                            .transpose()
                            .with_leniency(lenient)
                            .map_err(config::transport::http::Error::from)?
                            .unwrap_or(true)
                    };

                    if may_use_cainfo {
                        let key = "http.sslCAInfo";
                        debug_assert_eq!(key, config::tree::Http::SSL_CA_INFO.logical_name());
                        opts.ssl_ca_info = config
                            .path_filter(key, &mut trusted_only)
                            .map(|p| {
                                use crate::config::cache::interpolate_context;
                                p.interpolate(interpolate_context(
                                    self.install_dir().ok().as_deref(),
                                    self.config.home_dir().as_deref(),
                                ))
                                .map(std::borrow::Cow::into_owned)
                            })
                            .transpose()
                            .with_leniency(lenient)
                            .map_err(|err| config::transport::Error::InterpolatePath { source: err, key })?;
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
                            .map(|max| min.and_then(|min| max.map(|max| (min, max))))
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
                        let ssl_no_verify = config
                            .boolean_filter(key, &mut trusted_only)
                            .map(|value| config::tree::gitoxide::Http::SSL_NO_VERIFY.enrich_error(value))
                            .transpose()
                            .with_leniency(lenient)
                            .map_err(config::transport::http::Error::from)?
                            .unwrap_or_default();

                        if ssl_no_verify {
                            opts.ssl_verify = false;
                        } else {
                            let key = "http.sslVerify";
                            opts.ssl_verify = config
                                .boolean_filter(key, &mut trusted_only)
                                .map(|value| config::tree::Http::SSL_VERIFY.enrich_error(value))
                                .transpose()
                                .with_leniency(lenient)
                                .map_err(config::transport::http::Error::from)?
                                .unwrap_or(true);
                        }
                    }

                    #[cfg(feature = "blocking-http-transport-curl")]
                    {
                        let key = "http.schannelCheckRevoke";
                        let schannel_check_revoke = config
                            .boolean_filter(key, &mut trusted_only)
                            .map(|value| config::tree::Http::SCHANNEL_CHECK_REVOKE.enrich_error(value))
                            .transpose()
                            .with_leniency(lenient)
                            .map_err(config::transport::http::Error::from)?;
                        let backend = gix_protocol::transport::client::http::curl::Options { schannel_check_revoke };
                        opts.backend =
                            Some(Arc::new(Mutex::new(backend)) as Arc<Mutex<dyn Any + Send + Sync + 'static>>);
                    }

                    Ok(Some(Box::new(opts)))
                }
            }
            File | Git | Ssh | Ext(_) => Ok(None),
        }
    }
}
