#[cfg(any(
    feature = "blocking-http-transport-reqwest",
    feature = "blocking-http-transport-curl"
))]
mod http {
    use gix_transport::client::blocking_io::http::options::{
        FollowRedirects, HttpVersion, ProxyAuthMethod, SslVersion, SslVersionRangeInclusive,
    };

    use crate::repository::config::{repo, repo_opts};

    fn http_options(
        repo: &gix::Repository,
        remote_name: Option<&str>,
        url: &str,
    ) -> gix_transport::client::blocking_io::http::Options {
        let opts = repo
            .transport_options(url, remote_name.map(Into::into))
            .expect("valid configuration")
            .expect("configuration available for http");
        opts.downcast_ref::<gix_transport::client::blocking_io::http::Options>()
            .expect("http options have been created")
            .to_owned()
    }

    #[test]
    fn remote_overrides() {
        let repo = repo("http-remote-override");
        let gix_transport::client::blocking_io::http::Options {
            proxy,
            proxy_auth_method,
            follow_redirects,
            ..
        } = http_options(&repo, Some("origin"), "https://example.com/does/not/matter");

        assert_eq!(proxy_auth_method, ProxyAuthMethod::Negotiate);
        assert_eq!(proxy.as_deref(), Some("http://overridden"));
        assert_eq!(follow_redirects, FollowRedirects::Initial);
    }

    #[test]
    fn simple_configuration() {
        let repo = repo("http-config");
        let gix_transport::client::blocking_io::http::Options {
            extra_headers,
            follow_redirects,
            low_speed_limit_bytes_per_second,
            low_speed_time_seconds,
            proxy,
            no_proxy,
            proxy_auth_method,
            proxy_authenticate,
            user_agent,
            connect_timeout,
            verbose,
            ssl_ca_info,
            ssl_version,
            ssl_verify,
            http_version,
            backend,
            should_interrupt,
        } = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(
            extra_headers,
            &["ExtraHeader: value2", "ExtraHeader: value3"],
            "it respects empty values to clear prior values"
        );
        assert_eq!(follow_redirects, FollowRedirects::All);
        assert_eq!(low_speed_limit_bytes_per_second, 5120);
        assert_eq!(low_speed_time_seconds, 10);
        assert_eq!(proxy.as_deref(), Some("http://localhost:9090"));
        assert!(
            proxy_authenticate.is_none(),
            "no username means no authentication required"
        );
        assert_eq!(proxy_auth_method, ProxyAuthMethod::Basic);
        assert_eq!(user_agent.as_deref(), Some("agentJustForHttp"));
        assert_eq!(connect_timeout, Some(std::time::Duration::from_millis(60 * 1024)));
        assert_eq!(no_proxy, None);
        assert!(!verbose, "verbose is disabled by default");
        assert_eq!(ssl_ca_info.as_deref(), Some(std::path::Path::new("./CA.pem")));
        assert!(
            should_interrupt.is_none(),
            "interrupt flags are provided by callers, never from configuration"
        );
        #[cfg(feature = "blocking-http-transport-reqwest")]
        {
            assert!(
                backend.is_none(),
                "backed is never set as it's backend specific, rather custom options typically"
            );
        }
        #[cfg(feature = "blocking-http-transport-curl")]
        {
            let backend = backend
                .as_ref()
                .map(|b| b.lock().expect("not poisoned"))
                .expect("backend is set for curl due to specific options");
            match backend.downcast_ref::<gix_protocol::transport::client::blocking_io::http::curl::Options>() {
                Some(opts) => {
                    assert_eq!(opts.schannel_check_revoke, Some(true));
                }
                None => panic!("Correct backend option type is used"),
            }
        }

        let version = SslVersion::SslV2;
        assert_eq!(
            ssl_version,
            Some(SslVersionRangeInclusive {
                min: version,
                max: version
            })
        );

        assert!(ssl_verify, "SSL verification is enabled by default if not configured");
        assert_eq!(http_version, Some(HttpVersion::V1_1));
    }

    #[test]
    fn http_ssl_cainfo_suppressed_by_() {
        let repo = repo("http-disabled-cainfo");
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert!(
            opts.ssl_ca_info.is_none(),
            "http.schannelUseSSLCAInfo is explicitly set and prevents the ssl_ca_info to be set"
        );
    }

    #[test]
    fn http_ssl_version_min_max_overrides_ssl_version() {
        let repo = repo("http-ssl-version-min-max");
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(
            opts.ssl_version,
            Some(SslVersionRangeInclusive {
                min: SslVersion::TlsV1_1,
                max: SslVersion::TlsV1_2
            })
        );
    }

    #[test]
    fn http_ssl_version_default() {
        let repo = repo("http-ssl-version-default");
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(
            opts.ssl_version,
            Some(SslVersionRangeInclusive {
                min: SslVersion::Default,
                max: SslVersion::Default
            })
        );
    }

    #[test]
    fn http_ssl_version_empty_resets_prior_values() {
        let repo = repo_opts("http-config", |opts| opts.config_overrides(["http.sslVersion="]));
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert!(opts.ssl_version.is_none(), "empty strings reset what was there");
    }

    #[test]
    fn http_verbose() {
        let repo = repo("http-verbose");
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert!(opts.verbose);
    }

    #[test]
    fn http_no_proxy() {
        let repo = repo("http-no-proxy");
        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(opts.no_proxy.as_deref(), Some("no validation done here"));
    }

    #[test]
    fn http_proxy_with_username() {
        let repo = repo("http-proxy-authenticated");

        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://user@localhost:9090"),
            "usernames in proxy urls trigger authentication before making a connection…"
        );
        assert!(
            opts.proxy_authenticate.is_some(),
            "…and credential-helpers are used to do that. This could be overridden in remotes one day"
        );
        assert_eq!(
            opts.follow_redirects,
            FollowRedirects::All,
            "an empty value is true, so we can't take shortcuts for these"
        );
    }

    #[test]
    fn empty_proxy_string_turns_it_off() {
        let repo = repo("http-proxy-empty");

        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(
            opts.proxy.as_deref(),
            Some(""),
            "empty strings indicate that the proxy is to be unset by the transport"
        );
        assert_eq!(opts.follow_redirects, FollowRedirects::None);
    }

    #[test]
    fn https_specific_proxy_only() {
        let repo = repo("https-proxy-only");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://https"),
            "if there is no other proxy setting, it will be used for https"
        );

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(opts.proxy, None, "non-https urls don't use this proxy at all");
    }

    #[test]
    fn env_http_proxy_only() {
        let repo = repo("gitoxide-http-proxy-only");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://http-fallback"),
            "the `http_proxy` env var derived value serves as fallback…"
        );

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://http-fallback"),
            "…for http-urls as well, as there is nothing else set."
        );
    }

    #[test]
    fn all_proxy_only() {
        let repo = repo("gitoxide-all-proxy-only");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://all-proxy-fallback"),
            "the `all_proxy` env var derived value serves as fallback…"
        );

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://all-proxy-fallback"),
            "…for http-urls as well, as there is nothing else set."
        );
    }

    #[test]
    fn all_proxy_is_fallback() {
        let repo = repo("gitoxide-all-proxy");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(opts.proxy.as_deref(), Some("http://http"));

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(opts.proxy.as_deref(), Some("http://http"));
    }

    #[test]
    fn env_http_proxy_is_fallback() {
        let repo = repo("gitoxide-http-proxy");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(opts.proxy.as_deref(), Some("http://http"));

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(opts.proxy.as_deref(), Some("http://http"));
    }

    #[test]
    fn https_specific_proxy_is_only_a_fallback() {
        let repo = repo("https-proxy");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://http"),
            "if the http proxy is set, it will be used even for https as the latter is only a fallback (by env vars)"
        );

        let opts = http_options(&repo, None, "http://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some("http://http"),
            "http urls use the http proxy configuration like normal"
        );
    }

    #[test]
    fn https_specific_proxy_empty() {
        let repo = repo("https-proxy-empty");

        let opts = http_options(&repo, None, "https://host.local/repo");
        assert_eq!(
            opts.proxy.as_deref(),
            Some(""),
            "empty strings work just like they do for http.proxy (and empty strings indicate to unset it)"
        );
    }

    #[test]
    fn proxy_without_protocol_is_defaulted_to_http() {
        let repo = repo("http-proxy-auto-prefix");

        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert_eq!(opts.proxy.as_deref(), Some("http://localhost:9090"));
        assert_eq!(opts.follow_redirects, FollowRedirects::Initial);
    }

    #[test]
    fn ssl_verify_disabled() {
        let repo = repo("ssl-verify-disabled");

        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert!(!opts.ssl_verify);
    }

    #[test]
    fn ssl_no_verify_takes_precedence() {
        let repo = repo("ssl-no-verify-enabled");

        let opts = http_options(&repo, None, "https://example.com/does/not/matter");
        assert!(
            !opts.ssl_verify,
            "even with `http.sslVerify` enabled, `gitoxide.http.sslNoVerify` takes precedence`"
        );
    }
}
