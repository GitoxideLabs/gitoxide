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

    #[cfg(all(
        feature = "blocking-http-transport-reqwest",
        not(feature = "blocking-http-transport-curl")
    ))]
    #[test]
    fn environment_proxy_selection_is_left_to_the_backend() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _environment = gix_testtools::Env::new()
            .set("http_proxy", "http://http.invalid")
            .set("HTTPS_PROXY", "http://https.invalid")
            .set("ALL_PROXY", "http://all.invalid")
            .set("no_proxy", "env.invalid");
        for (fixture, proxy, no_proxy) in [
            ("http-verbose", None, None),
            ("http-proxy-empty", Some(""), None),
            ("gitoxide-http-proxy-only", Some("http://http-fallback"), None),
            ("http-no-proxy", None, Some("no validation done here")),
        ] {
            let repo = repo_opts(fixture, |mut opts| {
                opts.permissions.env.http_transport = gix::sec::Permission::Allow;
                opts
            });
            assert!(
                repo.config_snapshot().string("gitoxide.http.proxy").is_some(),
                "the configuration snapshot imports proxy environment variables"
            );
            for url in ["http://host.local/repo", "https://host.local/repo"] {
                let opts = http_options(&repo, None, url);
                assert_eq!(
                    opts.proxy.as_deref(),
                    proxy,
                    "only explicit proxy settings may override per-scheme environment routing: {fixture}, {url}"
                );
                assert_eq!(
                    opts.no_proxy.as_deref(),
                    no_proxy,
                    "explicit bypass settings override the environment: {fixture}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn environment_proxies_keep_credential_helpers() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _environment = gix_testtools::Env::new()
            .set("http_proxy", "http://one@http.invalid:3128")
            .set("https_proxy", "http://two@http.invalid:3128")
            .set("HTTPS_PROXY", "http://two@http.invalid:3128")
            .set("all_proxy", "http://three@all.invalid")
            .set("ALL_PROXY", "http://three@all.invalid");
        let records = gix_testtools::tempfile::tempdir()?;
        let mut overrides = vec![
            "credential.helper=".to_owned(),
            "credential.protectProtocol=false".to_owned(),
            "gitoxide.credentials.terminalPrompt=false".to_owned(),
        ];
        for (host, user, password) in [
            ("http.invalid:3128", "one", "shared-secret"),
            ("http.invalid:3128", "two", "shared-secret"),
            ("all.invalid", "three", "all-%0A-%25-%0D"),
        ] {
            let path = records.path().join(user);
            let path = gix::path::to_unix_separators_on_windows(gix::path::into_bstr(&path)?);
            let path = gix::quote::single(path.as_ref());
            overrides.push(format!(
                "credential.http://{user}@{host}.helper=!f() {{ if test \"$1\" = get; then echo username=helper-{password}; printf 'path=\\377\\n'; printf '%s\\r\\r\\n' password={password}; else {{ echo \"$1\"; cat; }} >>{path}; fi; }}; f"
            ));
        }
        let repo = repo_opts("http-verbose", |mut opts| {
            opts.permissions.env.http_transport = gix::sec::Permission::Allow;
            opts.config_overrides(overrides.clone())
        });
        let opts = http_options(&repo, None, "http://host.local/repo");
        let cloned_opts = opts.clone();
        let (_, authenticate) = opts
            .proxy_authenticate
            .as_ref()
            .expect("environment proxy usernames still enable credential helpers");
        let cases = [
            ("http://one@http.invalid:3128", "one", "shared-secret"),
            ("http://two@http.invalid:3128", "two", "shared-secret"),
            ("http://three@all.invalid:1080", "three", "all-%0A-%25-%0D"),
        ];
        let count = if cfg!(feature = "blocking-http-transport-curl") {
            1
        } else {
            cases.len()
        };
        let mut follow_ups = Vec::new();
        for (index, (url, _, password)) in cases.into_iter().take(count).enumerate() {
            let options = if index == 0 { &opts } else { &cloned_opts };
            let (_, authenticate) = options.proxy_authenticate.as_ref().expect("cloning preserves helpers");
            let credentials =
                authenticate.lock().expect("no panics")(gix::credentials::helper::Action::get_for_url(url))?
                    .expect("the selected proxy has a credential helper");
            assert!(
                matches!(
                    credentials.credentials,
                    gix::sec::identity::Credentials::UsernamePassword { username, password: actual, .. }
                        if actual == format!("{password}\r") && username == format!("helper-{password}")
                ),
                "helper selection follows the proxy URL"
            );
            follow_ups.push(credentials.next);
        }
        for next in follow_ups {
            authenticate.lock().expect("no panics")(next.clone().store())?;
            authenticate.lock().expect("no panics")(next.clone().store())?;
            authenticate.lock().expect("no panics")(next.erase())?;
        }
        for (_, user, password) in cases.into_iter().take(count) {
            let bytes = std::fs::read(records.path().join(user))?;
            let record = String::from_utf8_lossy(&bytes);
            assert!(
                bytes.windows(b"path=\xff\n".len()).any(|bytes| bytes == b"path=\xff\n"),
                "the helper's byte-oriented path is retained in follow-ups"
            );
            assert!(
                record.contains(&format!("password={password}\r\n")),
                "follow-ups preserve every payload byte, including trailing CR and percent escapes"
            );
            assert!(
                !record.lines().any(|line| line.starts_with("url=")),
                "the original helper payload without a URL is restored before follow-up"
            );
            assert_eq!(
                record.lines().filter(|line| *line == "store").count(),
                2,
                "each Get's helper stores its credentials for every successful redirect hop"
            );
            assert_eq!(
                record.lines().filter(|line| *line == "erase").count(),
                1,
                "each Get's helper erases its credentials"
            );
            assert!(
                record
                    .lines()
                    .filter(|line| line.starts_with("password="))
                    .all(|line| line == format!("password={password}")),
                "each helper receives only its own credentials after interleaved Get calls"
            );
        }
        let repo = repo_opts("http-verbose", |mut opts| {
            opts.permissions.env.http_transport = gix::sec::Permission::Allow;
            opts.config_overrides(
                overrides
                    .into_iter()
                    .chain(["gitoxide.credentials.terminalPrompt=true".to_owned()]),
            )
        });
        let opts = http_options(&repo, None, "http://host.local/repo");
        let (action, authenticate) = opts.proxy_authenticate.as_ref().expect("proxy helpers are configured");
        let credentials = authenticate.lock().expect("no panics")(action.clone())?
            .expect("the helper provides credentials without prompting");
        authenticate.lock().expect("no panics")(credentials.next.store())?;
        let bytes = std::fs::read(records.path().join("one"))?;
        let record = String::from_utf8_lossy(&bytes);
        assert_eq!(
            record
                .lines()
                .filter(|line| line.starts_with("url="))
                .collect::<Vec<_>>(),
            ["url=http://one@http.invalid:3128/"],
            "the original helper URL is restored without routing metadata"
        );
        #[cfg(all(
            feature = "blocking-http-transport-reqwest",
            not(feature = "blocking-http-transport-curl")
        ))]
        {
            let secret = "routing-secret";
            let error = authenticate.lock().expect("no panics")(gix::credentials::helper::Action::Store(
                format!("url=gix-proxy:0:{secret}\r\n").into(),
            ))
            .expect_err("raw control bytes are rejected in the routing handle");
            assert!(
                !format!("{error:?}").contains(secret),
                "invalid routing diagnostics must not expose the encoded credential payload"
            );
        }
        Ok(())
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
