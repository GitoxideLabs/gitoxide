mod options {
    mod ssh_command {
        use crate::client::blocking_io::ssh::{ProgramKind, connect::Options};

        #[test]
        fn no_field_means_ssh() {
            assert_eq!(Options::default().ssh_command(), "ssh");
        }

        #[test]
        fn command_field_determines_ssh_command() {
            assert_eq!(
                Options {
                    command: Some("field-value".into()),
                    ..Default::default()
                }
                .ssh_command(),
                "field-value"
            );
            assert_eq!(
                Options {
                    command: Some("field-value".into()),
                    kind: Some(ProgramKind::TortoisePlink),
                    ..Default::default()
                }
                .ssh_command(),
                "field-value"
            );
        }

        #[test]
        fn kind_serves_as_fallback() {
            assert_eq!(
                Options {
                    kind: Some(ProgramKind::TortoisePlink),
                    ..Default::default()
                }
                .ssh_command(),
                "tortoiseplink.exe"
            );
        }
    }
}

mod build_client_feature_check_command {
    use std::ffi::OsStr;

    use crate::client::blocking_io::ssh::{self, build_client_feature_check_command};

    #[test]
    fn ipv6_destinations_have_no_brackets_user_or_port() -> gix_error::TestResult {
        for (input, host) in [
            ("ssh://[::1]/p", "::1"),
            ("ssh://user@[::1]/p", "::1"),
            ("ssh://[::1]:42/p", "::1"),
            ("ssh://user@[::1]:42/p", "::1"),
            ("ssh://user@[fe80::1%25Eth0]:42/p", "fe80::1%Eth0"),
            ("[::1]:p", "::1"),
            ("[fe80::1%Eth0]:p", "fe80::1%Eth0"),
        ] {
            let url = gix_url::parse(input)?;
            let cmd = build_client_feature_check_command(OsStr::new("ssh"), &url, true)?;
            assert_eq!(
                cmd.get_args().collect::<Vec<_>>(),
                [OsStr::new("-G"), OsStr::new(host)],
                "the feature probe for {input:?} uses only the bare host"
            );
        }
        Ok(())
    }

    #[test]
    fn bracketed_option_like_hosts_are_rejected_even_with_user() -> gix_error::TestResult {
        for input in ["ssh://[-arg]/p", "ssh://user@[-arg]/p", "[-arg]:p"] {
            let url = gix_url::parse(input)?;
            let failure = build_client_feature_check_command(OsStr::new("ssh"), &url, true)
                .expect_err("brackets and a user must not hide an option from the bare-host probe");
            assert!(
                matches!(failure, ssh::Error::AmbiguousHostName { host } if host == "-arg"),
                "the feature probe must reject the unbracketed option in {input:?}"
            );
        }
        for input in ["ssh://host/p", "ssh://user@host/p"] {
            let mut url = gix_url::parse(input)?;
            url.host = Some("[-arg]".into());
            let failure = build_client_feature_check_command(OsStr::new("ssh"), &url, true)
                .expect_err("manually assigned brackets must not bypass host validation");
            assert!(
                matches!(failure, ssh::Error::AmbiguousHostName { host } if host == "-arg"),
                "the feature probe also rejects mutated hosts, even with a user"
            );
        }
        Ok(())
    }
}

mod program_kind {
    mod from_os_str {
        use std::ffi::OsStr;

        use crate::client::blocking_io::ssh::ProgramKind;

        #[test]
        fn known_variants_are_derived_from_basename() {
            for name_or_path in [
                "ssh",
                "ssh.exe",
                "SSH",
                "SSH.exe",
                "/bin/ssh",
                "/bin/SSH",
                #[cfg(windows)]
                r"c:\bin\ssh.exe",
            ] {
                assert_eq!(
                    ProgramKind::from(OsStr::new(name_or_path)),
                    ProgramKind::Ssh,
                    "{name_or_path:?} could not be identified correctly"
                );
            }
            assert_eq!(
                ProgramKind::from(OsStr::new("TortoisePlink.exe")),
                ProgramKind::TortoisePlink
            );
            assert_eq!(ProgramKind::from(OsStr::new("putty")), ProgramKind::Putty);
            assert_eq!(
                ProgramKind::from(OsStr::new("../relative/Plink.exe")),
                ProgramKind::Plink
            );
        }

        #[test]
        fn unknown_variants_fallback_to_simple() {
            assert_eq!(
                ProgramKind::from(OsStr::new("something-unknown-that-does-not-exist-for-sure-foobar")),
                ProgramKind::Simple,
                "in theory, we could fail right here but we don't and leave non-existing programs to fail during handshake"
            );
        }

        #[test]
        fn ssh_disguised_within_a_script_cannot_be_detected_due_to_invocation_with_dash_g() {
            assert_eq!(
                ProgramKind::from(OsStr::new("ssh -VVV")),
                ProgramKind::Simple,
                "we don't execute the command here but assume simple, even though we could determine it's ssh if we would do what git does here"
            );
        }
    }

    mod prepare_invocation {
        use std::ffi::OsStr;

        use crate::{
            Protocol,
            client::blocking_io::ssh::{self, ProgramKind},
        };

        #[test]
        fn ssh() {
            for (url, protocol, expected) in [
                ("ssh://user@host:42/p", Protocol::V1, &["ssh", "-p42", "user@host"][..]),
                ("ssh://user@host/p", Protocol::V1, &["ssh", "user@host"][..]),
                ("ssh://host/p", Protocol::V1, &["ssh", "host"][..]),
                ("ssh://[::1]/p", Protocol::V1, &["ssh", "::1"][..]),
                ("ssh://user@[::1]/p", Protocol::V1, &["ssh", "user@::1"][..]),
                ("ssh://[::1]:42/p", Protocol::V1, &["ssh", "-p42", "::1"][..]),
                ("ssh://user@[::1]:42/p", Protocol::V1, &["ssh", "-p42", "user@::1"][..]),
                ("[::1]:p", Protocol::V1, &["ssh", "::1"][..]),
                ("[fe80::1%Eth0]:p", Protocol::V1, &["ssh", "fe80::1%Eth0"][..]),
                (
                    "ssh://user@[fe80::1%25Eth0]:42/p",
                    Protocol::V2,
                    &["ssh", "-o", "SendEnv=GIT_PROTOCOL", "-p42", "user@fe80::1%Eth0"][..],
                ),
                (
                    "ssh://user@host:42/p",
                    Protocol::V2,
                    &["ssh", "-o", "SendEnv=GIT_PROTOCOL", "-p42", "user@host"][..],
                ),
                (
                    "ssh://user@host/p",
                    Protocol::V2,
                    &["ssh", "-o", "SendEnv=GIT_PROTOCOL", "user@host"][..],
                ),
                (
                    "ssh://host/p",
                    Protocol::V2,
                    &["ssh", "-o", "SendEnv=GIT_PROTOCOL", "host"][..],
                ),
            ] {
                assert_eq!(call_args(ProgramKind::Ssh, url, protocol), expected);
            }
        }

        #[test]
        fn tortoise_plink_has_batch_command() {
            assert_eq!(
                call_args(ProgramKind::TortoisePlink, "ssh://user@host:42/p", Protocol::V2),
                ["tortoiseplink.exe", "-batch", "-P", "42", "user@host"]
            );
        }

        #[test]
        fn port_for_all() {
            for kind in [ProgramKind::TortoisePlink, ProgramKind::Plink, ProgramKind::Putty] {
                for (url, destination) in [
                    ("ssh://user@host:43/p", "user@host"),
                    ("ssh://[::1]:43/p", "::1"),
                    ("ssh://user@[::1]:43/p", "user@::1"),
                    ("ssh://user@[fe80::1%25Eth0]:43/p", "user@fe80::1%Eth0"),
                ] {
                    assert!(
                        call_args(kind, url, Protocol::V2).ends_with(&["-P".into(), "43".into(), destination.into()]),
                        "{kind:?} must pass the port separately from the bare destination for {url:?}"
                    );
                }
            }
        }

        #[test]
        fn manually_assigned_bracketed_hosts_are_stripped_for_all_variants() -> gix_error::TestResult {
            for kind in [
                ProgramKind::Ssh,
                ProgramKind::TortoisePlink,
                ProgramKind::Plink,
                ProgramKind::Putty,
                ProgramKind::Simple,
            ] {
                for (input, host, destination) in [
                    ("ssh://host/p", "[::1]", "::1"),
                    ("ssh://user@host/p", "[::1]", "user@::1"),
                    ("ssh://user@host/p", "[fe80::1%Eth0]", "user@fe80::1%Eth0"),
                ] {
                    let mut url = gix_url::parse(input)?;
                    url.host = Some(host.into());
                    let prepare = kind.prepare_invocation(
                        kind.exe().unwrap_or_else(|| OsStr::new("simple")),
                        &url,
                        Protocol::V1,
                        true,
                    )?;
                    let cmd = std::process::Command::try_from(prepare)?;
                    let expected = if kind == ProgramKind::TortoisePlink {
                        vec![OsStr::new("-batch"), OsStr::new(destination)]
                    } else {
                        vec![OsStr::new(destination)]
                    };
                    assert_eq!(
                        cmd.get_args().collect::<Vec<_>>(),
                        expected,
                        "{kind:?} strips brackets even when the host was not parsed"
                    );
                }
            }
            Ok(())
        }

        #[test]
        fn bracketed_option_like_hosts_cannot_hide_options() -> gix_error::TestResult {
            for kind in [
                ProgramKind::Ssh,
                ProgramKind::TortoisePlink,
                ProgramKind::Plink,
                ProgramKind::Putty,
                ProgramKind::Simple,
            ] {
                for (input, assigned_host, expected_destination) in [
                    ("ssh://[-arg]/p", None, None),
                    ("[-arg]:p", None, None),
                    ("ssh://user@[-arg]/p", None, Some("user@-arg")),
                    ("ssh://host/p", Some("[-arg]"), None),
                    ("ssh://user@host/p", Some("[-arg]"), Some("user@-arg")),
                ] {
                    let mut url = gix_url::parse(input)?;
                    if let Some(host) = assigned_host {
                        url.host = Some(host.into());
                    }
                    let result = kind.prepare_invocation(
                        kind.exe().unwrap_or_else(|| OsStr::new("simple")),
                        &url,
                        Protocol::V1,
                        true,
                    );
                    if let Some(destination) = expected_destination {
                        let cmd = std::process::Command::try_from(result?)?;
                        assert_eq!(
                            cmd.get_args().last(),
                            Some(OsStr::new(destination)),
                            "{kind:?} strips brackets, but a safe user prefix keeps {input:?} from becoming an option"
                        );
                    } else {
                        let failure = result
                            .err()
                            .expect("brackets must not hide an option-like host without a user");
                        assert!(
                            matches!(failure, ssh::invocation::Error::AmbiguousHostName { host } if host == "-arg"),
                            "{kind:?} must classify the host after stripping brackets for {input:?}"
                        );
                    }
                }
            }
            Ok(())
        }

        #[test]
        fn ambiguous_user_is_disallowed_explicit_ssh() {
            let failure = try_call(ProgramKind::Ssh, "ssh://-arg@host/p", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "ambiguous user is disallowed explicit ssh", @r#"
            AmbiguousUserName {
                user: "-arg",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::AmbiguousUserName { user } if user == "-arg"),
                "ambiguous user is disallowed explicit ssh"
            );
        }

        #[test]
        fn ambiguous_user_is_disallowed_implicit_ssh() {
            let failure = try_call(ProgramKind::Ssh, "-arg@host:p/q", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "ambiguous user is disallowed implicit ssh", @r#"
            AmbiguousUserName {
                user: "-arg",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::AmbiguousUserName { user } if user == "-arg"),
                "ambiguous user is disallowed implicit ssh"
            );
        }

        #[test]
        fn ambiguous_host_is_allowed_with_user_explicit_ssh() {
            assert_eq!(
                call_args(ProgramKind::Ssh, "ssh://user@-arg/p", Protocol::V2),
                ["ssh", "-o", "SendEnv=GIT_PROTOCOL", "user@-arg"]
            );
        }

        #[test]
        fn ambiguous_host_is_allowed_with_user_implicit_ssh() {
            assert_eq!(
                call_args(ProgramKind::Ssh, "user@-arg:p/q", Protocol::V2),
                ["ssh", "-o", "SendEnv=GIT_PROTOCOL", "user@-arg"]
            );
        }

        #[test]
        fn ambiguous_host_is_disallowed_without_user() {
            let failure = try_call(ProgramKind::Ssh, "ssh://-arg/p", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "ambiguous host is disallowed without user", @r#"
            AmbiguousHostName {
                host: "-arg",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::AmbiguousHostName { host } if host == "-arg"),
                "ambiguous host is disallowed without user"
            );
        }

        #[test]
        fn ambiguous_user_and_host_remain_disallowed_together_explicit_ssh() {
            let failure = try_call(ProgramKind::Ssh, "ssh://-arg@host/p", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "ambiguous user and host remain disallowed together explicit ssh", @r#"
            AmbiguousUserName {
                user: "-arg",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::AmbiguousUserName { user } if user == "-arg"),
                "ambiguous user and host remain disallowed together explicit ssh"
            );
        }

        #[test]
        fn ambiguous_user_and_host_remain_disallowed_together_implicit_ssh() {
            let failure = try_call(ProgramKind::Ssh, "-userarg@-hostarg:p/q", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "ambiguous user and host remain disallowed together implicit ssh", @r#"
            AmbiguousUserName {
                user: "-userarg",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::AmbiguousUserName { user } if user == "-userarg"),
                "ambiguous user and host remain disallowed together implicit ssh"
            );
        }

        #[test]
        fn simple_cannot_handle_any_arguments() {
            let failure = try_call(ProgramKind::Simple, "ssh://user@host:42/p", Protocol::V2)
                .err()
                .expect("the operation must fail");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[]), "simple cannot handle any arguments", @r#"
            Unsupported {
                command: "simple",
                function: "setting the port",
            }
            "#);
            assert!(
                matches!(failure, ssh::invocation::Error::Unsupported { .. }),
                "simple cannot handle any arguments"
            );
            for (url, destination) in [
                ("ssh://user@host/p", "user@host"),
                ("ssh://[::1]/p", "::1"),
                ("ssh://user@[::1]/p", "user@::1"),
                ("[::1]:p", "::1"),
                ("[fe80::1%Eth0]:p", "fe80::1%Eth0"),
            ] {
                assert_eq!(
                    call_args(ProgramKind::Simple, url, Protocol::V2),
                    ["simple", destination],
                    "simple can only do simple invocations, with bare destinations for {url:?}"
                );
            }
        }

        #[test]
        fn ssh_env_v2() {
            let prepare = call(ProgramKind::Ssh, "ssh://host/p", Protocol::V2);
            assert_eq!(
                prepare.env,
                &[
                    ("GIT_PROTOCOL".into(), "version=2".into()),
                    ("LANG".into(), "C".into()),
                    ("LC_ALL".into(), "C".into())
                ]
            );
            assert!(!prepare.use_shell);
        }

        #[test]
        fn disallow_shell_is_honored() -> Result {
            let url = gix_url::parse("ssh://host/path").expect("valid url");

            let disallow_shell = false;
            let prepare =
                ProgramKind::Ssh.prepare_invocation(OsStr::new("echo hi"), &url, Protocol::V1, disallow_shell)?;
            assert!(prepare.use_shell, "shells are used when needed");

            let disallow_shell = true;
            let prepare =
                ProgramKind::Ssh.prepare_invocation(OsStr::new("echo hi"), &url, Protocol::V1, disallow_shell)?;
            assert!(
                !prepare.use_shell,
                "but we can enforce it not to be used as well for historical reasons"
            );
            Ok(())
        }

        fn try_call(
            kind: ProgramKind,
            url: &str,
            version: Protocol,
        ) -> std::result::Result<gix_command::Prepare, ssh::invocation::Error> {
            let ssh_cmd = kind.exe().unwrap_or_else(|| OsStr::new("simple"));
            let url = gix_url::parse(url).expect("valid url");
            kind.prepare_invocation(ssh_cmd, &url, version, false)
        }
        fn call(kind: ProgramKind, url: &str, version: Protocol) -> gix_command::Prepare {
            try_call(kind, url, version).expect("no error")
        }
        fn call_args(kind: ProgramKind, url: &str, version: Protocol) -> Vec<String> {
            let prepare = call(kind, url, version);
            let program = prepare.command.clone();
            let cmd =
                std::process::Command::try_from(prepare).expect("command fixture can be represented by the platform");
            let expected_program = std::process::Command::try_from(gix_command::prepare(&program))
                .expect("command fixture can be represented by the platform");
            assert_eq!(
                cmd.get_program(),
                expected_program.get_program(),
                "the selected SSH program follows the platform's command lookup"
            );
            std::iter::once(program.as_os_str())
                .chain(cmd.get_args())
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        }

        type Result = std::result::Result<(), ssh::invocation::Error>;
    }

    mod line_to_err {
        use std::io::ErrorKind;

        use crate::client::blocking_io::ssh::ProgramKind;

        #[test]
        fn all() {
            let mut diagnostics = Vec::new();
            for (kind, line, expected) in [
                (
                    ProgramKind::Ssh,
                    "byron@github.com: Permission denied (publickey).",
                    ErrorKind::PermissionDenied,
                ),
                (
                    ProgramKind::Ssh,
                    "ssh: Could not resolve hostname hostfoobar: nodename nor servname provided, or not known",
                    ErrorKind::ConnectionRefused,
                ),
                (
                    ProgramKind::Ssh,
                    "ssh: connect to host example.org port 22: No route to host",
                    ErrorKind::NotFound,
                ),
                // connection closed by remote on windows
                (
                    ProgramKind::Ssh,
                    "banner exchange: Connection to 127.0.0.1 port 61024: Software caused connection abort",
                    ErrorKind::NotFound,
                ),
                // connection closed by remote on unix
                (
                    ProgramKind::Ssh,
                    "Connection closed by 127.0.0.1 port 8888", //
                    ErrorKind::NotFound,
                ),
                // this kind is basically unknown but we try our best, and simple equals ssh
                (
                    ProgramKind::Simple,
                    "something permission denied something",
                    ErrorKind::PermissionDenied,
                ),
                (
                    ProgramKind::Simple,
                    "something resolve hostname hostfoobar: nodename nor servname something",
                    ErrorKind::ConnectionRefused,
                ),
                (
                    ProgramKind::Simple,
                    "something connect to host something",
                    ErrorKind::NotFound,
                ),
            ] {
                let err = kind.line_to_err(line.into()).expect("the SSH diagnostic is recognized");
                assert_eq!(err.kind(), expected);
                diagnostics.push((kind, err));
            }
            insta::assert_debug_snapshot!(diagnostics, "SSH diagnostics retain the server message alongside their I/O classification", @r#"
            [
                (
                    Ssh,
                    Custom {
                        kind: PermissionDenied,
                        error: "byron@github.com: Permission denied (publickey).",
                    },
                ),
                (
                    Ssh,
                    Custom {
                        kind: ConnectionRefused,
                        error: "ssh: Could not resolve hostname hostfoobar: nodename nor servname provided, or not known",
                    },
                ),
                (
                    Ssh,
                    Custom {
                        kind: NotFound,
                        error: "ssh: connect to host example.org port 22: No route to host",
                    },
                ),
                (
                    Ssh,
                    Custom {
                        kind: NotFound,
                        error: "banner exchange: Connection to 127.0.0.1 port 61024: Software caused connection abort",
                    },
                ),
                (
                    Ssh,
                    Custom {
                        kind: NotFound,
                        error: "Connection closed by 127.0.0.1 port 8888",
                    },
                ),
                (
                    Simple,
                    Custom {
                        kind: PermissionDenied,
                        error: "something permission denied something",
                    },
                ),
                (
                    Simple,
                    Custom {
                        kind: ConnectionRefused,
                        error: "something resolve hostname hostfoobar: nodename nor servname something",
                    },
                ),
                (
                    Simple,
                    Custom {
                        kind: NotFound,
                        error: "something connect to host something",
                    },
                ),
            ]
            "#);
        }

        #[test]
        fn tortoiseplink_putty_plink() {
            let mut diagnostics = Vec::new();
            for kind in [ProgramKind::TortoisePlink, ProgramKind::Plink, ProgramKind::Putty] {
                let err = kind
                    .line_to_err("publickey".into())
                    .expect("publickey is a recognized authentication failure");
                assert_eq!(
                    err.kind(),
                    std::io::ErrorKind::PermissionDenied,
                    "this program pops up error messages in a window, no way to extract information from it. Maybe there is other ways to use it, 'publickey' they mention all"
                );
                diagnostics.push((kind, err));
            }
            insta::assert_debug_snapshot!(diagnostics, "PuTTY variants retain their public-key authentication failure", @r#"
            [
                (
                    TortoisePlink,
                    Custom {
                        kind: PermissionDenied,
                        error: "publickey",
                    },
                ),
                (
                    Plink,
                    Custom {
                        kind: PermissionDenied,
                        error: "publickey",
                    },
                ),
                (
                    Putty,
                    Custom {
                        kind: PermissionDenied,
                        error: "publickey",
                    },
                ),
            ]
            "#);
        }
    }
}
