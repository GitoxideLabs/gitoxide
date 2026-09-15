#[cfg(any(
    feature = "blocking-http-transport-curl",
    feature = "blocking-http-transport-reqwest"
))]
mod http_authentication {
    use std::io::{BufRead, Write};

    #[test]
    fn cached_credentials_are_selected_without_prompting() -> crate::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _environment = gix_testtools::Env::new()
            .set("GIT_TERMINAL_PROMPT", "0")
            .set("GCM_INTERACTIVE", "never")
            .set("NO_PROXY", "*")
            .set("no_proxy", "*");
        let directory = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(directory.path(), "init --bare")?;
        let _cwd = gix_testtools::set_current_dir(directory.path())?;
        let mut repo = gix::open_opts(directory.path(), gix::open::Options::isolated())?;

        // Like GCM, this helper needs the server's account hint to choose a cached credential.
        // The credentials are fictitious, and both the helper and gix must keep prompting disabled.
        repo.config_snapshot_mut().set_raw_value(
            "credential.helper",
            r#"!f() {
                test "$GIT_TERMINAL_PROMPT" = 0 && test "$GCM_INTERACTIVE" = never || exit 1
                hint=missing
                while IFS= read -r line; do
                    case "$line" in
                        'wwwauth[]=Basic realm="GitHub" domain_hint="example"') hint=present ;;
                    esac
                done
                test "$hint" = present || exit 1
                printf 'username=cached-user\npassword=cached-password\n'
            }; f"#,
        )?;
        repo.config_snapshot_mut()
            .set_raw_value("gitoxide.credentials.terminalPrompt", "false")?;

        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}/repo", listener.local_addr()?);
        let remote = repo.remote_at(url)?;
        let connection = remote.connect(gix::remote::Direction::Fetch)?;
        let mut authenticate = connection.configured_credentials_for_current_url();
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            let (stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
            let mut stream = std::io::BufReader::new(stream);
            let mut line = String::new();
            while stream.read_line(&mut line)? != 0 && line != "\r\n" {
                line.clear();
            }
            stream.get_mut().write_all(
                b"HTTP/1.1 401 Unauthorized\r\n\
                  WWW-Authenticate: Basic realm=\"GitHub\" domain_hint=\"example\"\r\n\
                  Content-Length: 0\r\n\
                  Connection: close\r\n\r\n",
            )
        });

        let mut obtained = None;
        let result = connection
            .with_credentials(|action| {
                obtained = Some(authenticate(action));
                // Stop after credential lookup, before the transport sends these dummy credentials.
                Err(gix_credentials::protocol::Error::Quit)
            })
            .ref_map(gix::progress::Discard, Default::default());
        server.join().expect("the HTTP fixture thread does not panic")?;
        assert!(
            result.is_err(),
            "the callback stops the handshake after credential lookup"
        );
        let outcome = obtained
            .expect("the 401 response invokes the credential callback")?
            .expect("the cached credential is complete");
        assert_eq!(
            outcome.identity.username, "cached-user",
            "the challenge selects the cached account without allowing prompts"
        );
        assert_eq!(
            outcome.identity.password, "cached-password",
            "the cached secret is returned"
        );
        Ok(())
    }
}

#[cfg(feature = "blocking-network-client")]
mod blocking_io {
    mod protocol_allow {
        use gix::remote::Direction::Fetch;
        use serial_test::serial;

        use crate::remote;

        #[test]
        #[serial]
        fn deny() {
            for name in ["protocol_denied", "protocol_file_denied"] {
                let repo = remote::repo(name);
                let remote = repo.find_remote("origin").unwrap();
                assert!(matches!(
                    remote.connect(Fetch).err(),
                    Some(gix::remote::connect::Error::ProtocolDenied {
                        url: _,
                        scheme: gix::url::Scheme::File
                    })
                ));
            }
        }

        #[test]
        #[serial]
        fn user() -> crate::Result {
            let _environment = gix_testtools::isolate_git_environment()?;
            for (env_value, should_allow) in [
                (None, Some(true)),
                (Some("0"), Some(false)),
                (Some("false"), Some(false)),
                (Some("1"), Some(true)),
                (Some("true"), Some(true)),
                (Some("invalid"), None),
            ] {
                let _env = env_value.map(|value| gix_testtools::Env::new().set("GIT_PROTOCOL_FROM_USER", value));
                let repo = gix::open_opts(
                    remote::repo("protocol_file_user").git_dir(),
                    gix::open::Options::isolated().permissions(gix::open::Permissions {
                        env: gix::open::permissions::Environment {
                            git_prefix: gix_sec::Permission::Allow,
                            http_transport: gix_sec::Permission::Deny,
                            ..gix::open::permissions::Environment::all()
                        },
                        ..gix::open::Permissions::isolated()
                    }),
                )?;
                let remote = repo.find_remote("origin")?;
                let result = remote.connect(Fetch);
                if let Some(should_allow) = should_allow {
                    assert_eq!(result.is_ok(), should_allow, "Value = {env_value:?}");
                } else {
                    assert!(
                        matches!(result, Err(gix::remote::connect::Error::SchemePermission(_))),
                        "invalid booleans must be reported"
                    );
                }
            }
            Ok(())
        }
    }
}
