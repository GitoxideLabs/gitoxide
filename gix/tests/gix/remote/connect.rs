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
                let err = remote.connect(Fetch).err().expect("protocol is denied");
                assert!(err.is_validation());
                let validation = err
                    .downcast_any_ref::<gix::error::ValidationError>()
                    .expect("protocol denial retains its validation details");
                assert_eq!(validation.message, "Protocol File is denied per configuration");
                assert!(validation.input.is_some(), "the denied URL is retained");
            }
        }

        #[test]
        #[serial]
        fn user() -> crate::Result {
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
                    assert!(result.is_err(), "invalid booleans must be reported");
                }
            }
            Ok(())
        }
    }
}
