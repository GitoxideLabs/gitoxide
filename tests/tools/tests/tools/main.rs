mod isolation {
    use gix_testtools::{Creation, Result, redact_debug_snapshot};

    #[test]
    fn script_configuration_adds_to_the_isolation() -> Result {
        let dir = gix_testtools::scripted_fixture_writable_with_args(
            "make_config_isolation.sh",
            None::<String>,
            Creation::Execute,
        )?;

        assert_eq!(
            std::fs::read_to_string(dir.path().join("maintenance-auto"))?.trim(),
            "false",
            "the isolation survives a script setting GIT_CONFIG_COUNT for itself"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("head"))?.trim(),
            "refs/heads/main",
            "isolation takes precedence over GIT_CONFIG_COUNT for shared keys"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("custom"))?.trim(),
            "present",
            "the script's own configuration still applies"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("override-head"))?.trim(),
            "refs/heads/other",
            "explicit git -c options can override isolation"
        );
        assert_eq!(
            gix_testtools::git(dir.path().join("repo"), "config --get maintenance.auto")?.trim(),
            "false",
            "`git()` runs with the same isolation"
        );
        Ok(())
    }

    #[test]
    fn redact_debug_snapshot_preserves_diagnostics_and_object_identity() {
        for (input, replacements, expected) in [
            (
                "read /tmp/random/repo/objects/pack failed: NotFound\nsource: /tmp/random/repo/config",
                &[("/tmp/random/repo", "<repo>")][..],
                "read <repo>/objects/pack failed: NotFound\nsource: <repo>/config",
            ),
            (
                r#"path: "C:\\Temp\\random\\repo\\config", input: "a\\b""#,
                &[(r"C:\Temp\random\repo\config", "<repo>/config")][..],
                r#"path: "<repo>/config", input: "a\\b""#,
            ),
            (
                r#"path: "C:\\Temp\\random\\repo\\dir with spaces\\config", input: "a\\b""#,
                &[(r"C:\Temp\random\repo", "<repo>")][..],
                r#"path: "<repo>/dir with spaces/config", input: "a\\b""#,
            ),
            (
                r"Alternates form a cycle -> C:\Temp\repo\a -> C:\Temp\repo\b",
                &[(r"C:\Temp\repo", "<repo>")][..],
                "Alternates form a cycle -> <repo>/a -> <repo>/b",
            ),
            (
                r#"path: "tests/fixtures\\repo\\sub\\config", input: "a\\b""#,
                &[(r"tests/fixtures\repo", "<repo>")][..],
                r#"path: "<repo>/sub/config", input: "a\\b""#,
            ),
            (
                r#"message: "Could not read \"C:\\Temp\\repo\\config\"", input: "a\\b""#,
                &[(r"C:\Temp\repo", "<repo>")][..],
                r#"message: "Could not read \"<repo>/config\"", input: "a\\b""#,
            ),
            (
                "connect 127.0.0.1:49152: ConnectionRefused; expected port 443",
                &[("127.0.0.1:49152", "127.0.0.1:<port>")][..],
                "connect 127.0.0.1:<port>: ConnectionRefused; expected port 443",
            ),
        ] {
            let snapshot = redact_debug_snapshot(&format_args!("{input}"), replacements);
            assert_eq!(
                format!("{snapshot:#?}"),
                expected,
                "explicit redactions preserve the surrounding diagnostic without extra quoting"
            );
        }
        let os_error = std::io::Error::from_raw_os_error(2);
        let snapshot = redact_debug_snapshot(&format_args!("read failed: {os_error}"), &[]);
        assert_eq!(
            format!("{snapshot:#?}"),
            format!("read failed: {:?}", os_error.kind()),
            "platform-dependent OS text is reduced to its portable error kind"
        );
        let snapshot = redact_debug_snapshot(&vec![os_error], &[]);
        insta::assert_debug_snapshot!(snapshot, "nested pretty-debug OS errors omit platform-specific codes and messages", @"
        [
            NotFound,
        ]
        ");

        #[cfg(all(feature = "sha1", feature = "sha256"))]
        {
            let input = "expected e69de29bb2d1d6434b8b29ae775ad8c2e48c5391, actual 473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813; expected e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
            let snapshot = redact_debug_snapshot(&format_args!("{input}"), &[]);
            assert_eq!(
                format!("{snapshot:#?}"),
                "expected Oid(1), actual Oid(2); expected Oid(1)"
            );
        }
    }
}
mod repository;
mod rust_fixture;
mod scripted_fixture_with_post;
