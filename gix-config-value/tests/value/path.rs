mod interpolate {
    use gix_error::Result;
    use std::path::{Path, PathBuf};

    use bstr::BString;
    use gix_config_value::path;

    #[test]
    fn backslash_is_not_special_and_they_are_not_escaping_anything() -> gix_testtools::TestResult {
        for path in [r"C:\foo\bar", "/foo/bar"] {
            let actual = gix_config_value::Path::from(path).interpolate(Default::default())?;
            assert_eq!(actual, Path::new(path));
        }
        Ok(())
    }

    #[test]
    fn empty_path_is_error() {
        let err = interpolate_without_context("").expect_err("empty paths are invalid");
        insta::assert_debug_snapshot!(err, "empty path is error", @"path is missing");
        assert!(err.is_not_found());
    }

    #[test]
    fn prefix_substitutes_git_install_dir() -> gix_testtools::TestResult {
        for git_install_dir in &["/tmp/git", r"C:\git"] {
            for (val, expected) in &[("%(prefix)/foo/bar", "foo/bar"), (r"%(prefix)/foo\bar", r"foo\bar")] {
                let expected =
                    std::path::PathBuf::from(format!("{}{}{}", git_install_dir, std::path::MAIN_SEPARATOR, expected));
                assert_eq!(
                    gix_config_value::Path::from(*val).interpolate(path::interpolate::Context {
                        git_install_dir: Path::new(git_install_dir).into(),
                        ..Default::default()
                    })?,
                    expected,
                    "prefix interpolation keeps separators as they are"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn prefix_substitution_skipped_with_dot_slash() -> gix_testtools::TestResult {
        let path = "./%(prefix)/foo/bar";
        let git_install_dir = "/tmp/git";
        assert_eq!(
            gix_config_value::Path::from(path).interpolate(path::interpolate::Context {
                git_install_dir: Path::new(git_install_dir).into(),
                ..Default::default()
            })?,
            Path::new(path)
        );
        Ok(())
    }

    #[test]
    fn tilde_alone_substitutes_current_user() -> gix_testtools::TestResult {
        let home = std::env::current_dir().expect("current directory is available");
        assert_eq!(
            gix_config_value::Path::from("~").interpolate(path::interpolate::Context {
                home_dir: Some(&home),
                ..Default::default()
            })?,
            home
        );
        let err = interpolate_without_context("~").expect_err("tilde expansion needs the current user's home");
        insta::assert_debug_snapshot!(err.classify()
                .find(|classification| classification.class() == gix_error::Class::NotFound)
                .expect("missing home directories are classified as not found")
                .error(), "tilde expansion reports the missing home directory", @r#"
        Message {
            message: "home dir is missing",
            class: NotFound,
        }
        "#);
        Ok(())
    }

    #[test]
    fn tilde_slash_substitutes_current_user() -> gix_testtools::TestResult {
        let home = std::env::current_dir()?;
        for suffix in ["", "user/bar", r"user\bar", "/user/bar"] {
            let actual = gix_config_value::Path::from(format!("~/{suffix}").as_str()).interpolate(
                path::interpolate::Context {
                    home_dir: Some(&home),
                    home_for_user: Some(home_for_user),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                actual.as_os_str(),
                home.join(suffix).as_os_str(),
                "tilde expansion preserves the suffix, including empty or leading-slash suffixes"
            );
        }
        Ok(())
    }

    #[test]
    fn tilde_with_given_user() -> gix_testtools::TestResult {
        let mut error_snapshots = Vec::new();
        let home = std::env::current_dir().expect("current directory is available");

        for path_suffix in &["foo/bar", r"foo\bar", ""] {
            let path = format!("~user/{path_suffix}");
            let expected = home.join("user").join(path_suffix);

            assert_eq!(
                interpolate_without_context(path)?.as_os_str(),
                expected.as_os_str(),
                "named-user expansion preserves the suffix, including a trailing slash"
            );
        }

        assert_eq!(
            interpolate_without_context("~user")?,
            home.join("user"),
            "~user without trailing slash is expanded like git does"
        );
        for path in ["~nonexistent", "~nonexistent/foo"] {
            let err = interpolate_without_context(path).expect_err("the named user does not exist");
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert!(err.is_not_found(), "named-user expansion classifies missing users");
        }
        insta::assert_debug_snapshot!(error_snapshots, "tilde with given user", @"
        [
            pwd user info is missing,
            pwd user info is missing,
        ]
        ");
        Ok(())
    }

    #[test]
    fn malformed_usernames_are_validation_errors_with_the_utf8_cause() {
        let err = gix_config_value::Path::from(BString::from(vec![b'~', 0xff, b'/', b'x']))
            .interpolate(path::interpolate::Context {
                home_for_user: Some(home_for_user),
                ..Default::default()
            })
            .expect_err("the username is not UTF-8");
        insta::assert_debug_snapshot!(err, "malformed usernames are validation errors with the utf8 cause", @r#"
        Ill-formed UTF-8 in username, input="\xff"

        Caused by:
            0: invalid utf-8 sequence of 1 bytes from index 0
        "#);
        assert!(err.is_validation());
        assert!(err.downcast_any_ref::<std::str::Utf8Error>().is_some());
    }

    fn interpolate_without_context(path: impl AsRef<str>) -> Result<PathBuf> {
        gix_config_value::Path::from(path.as_ref()).interpolate(path::interpolate::Context {
            home_for_user: Some(home_for_user),
            ..Default::default()
        })
    }

    fn home_for_user(name: &str) -> Option<PathBuf> {
        if name == "nonexistent" {
            return None;
        }
        std::env::current_dir()
            .expect("current directory is available")
            .join(name)
            .into()
    }
}

mod optional_prefix {
    use std::borrow::Cow;

    use bstr::BString;

    #[test]
    fn cow_inputs_become_owned_paths() {
        let borrowed = gix_config_value::Path::from(Cow::Borrowed(bstr::BStr::new("relative/path")));
        assert_eq!(
            borrowed.value, "relative/path",
            "borrowed Cow input is copied into the path"
        );

        let owned = gix_config_value::Path::from(Cow::Owned(BString::from("relative/path")));
        assert_eq!(owned.value, "relative/path", "owned Cow input is moved into the path");
    }

    #[test]
    fn path_without_optional_prefix_is_not_optional() {
        let path = gix_config_value::Path::from("/some/path");
        assert!(!path.is_optional, "path without prefix should not be optional");
        assert_eq!(path.value, "/some/path");
    }

    #[test]
    fn path_with_optional_prefix_is_optional() {
        let path = gix_config_value::Path::from(":(optional)/some/path");
        assert!(path.is_optional, "path with :(optional) prefix should be optional");
        assert_eq!(path.value, "/some/path", "prefix should be stripped");
    }

    #[test]
    fn optional_prefix_with_relative_path() {
        let path = gix_config_value::Path::from(":(optional)relative/path");
        assert!(path.is_optional);
        assert_eq!(path.value, "relative/path");
    }

    #[test]
    fn optional_prefix_with_tilde_expansion() {
        let path = gix_config_value::Path::from(":(optional)~/config/file");
        assert!(path.is_optional);
        assert_eq!(
            path.value, "~/config/file",
            "tilde should be preserved for interpolation"
        );
    }

    #[test]
    fn optional_prefix_with_prefix_substitution() {
        let path = gix_config_value::Path::from(":(optional)%(prefix)/share/git");
        assert!(path.is_optional);
        assert_eq!(
            path.value, "%(prefix)/share/git",
            "prefix should be preserved for interpolation"
        );
    }

    #[test]
    fn optional_prefix_with_windows_path() {
        let path = gix_config_value::Path::from(r":(optional)C:\Users\file");
        assert!(path.is_optional);
        assert_eq!(path.value, r"C:\Users\file");
    }

    #[test]
    fn optional_prefix_followed_by_empty_path() {
        let path = gix_config_value::Path::from(":(optional)");
        assert!(path.is_optional);
        assert_eq!(path.value, "", "empty path after prefix is valid");
    }

    #[test]
    fn partial_optional_string_is_not_treated_as_prefix() {
        let path = gix_config_value::Path::from(":(opt)ional/path");
        assert!(
            !path.is_optional,
            "incomplete prefix should not be treated as optional marker"
        );
        assert_eq!(path.value, ":(opt)ional/path");
    }

    #[test]
    fn optional_prefix_case_sensitive() {
        let path = gix_config_value::Path::from(":(OPTIONAL)/some/path");
        assert!(!path.is_optional, "prefix should be case-sensitive");
        assert_eq!(path.value, ":(OPTIONAL)/some/path");
    }

    #[test]
    fn optional_prefix_with_spaces() {
        let path = gix_config_value::Path::from(":(optional) /path/with/space");
        assert!(path.is_optional);
        assert_eq!(
            path.value, " /path/with/space",
            "space after prefix should be preserved"
        );
    }

    #[test]
    fn owned_path_strips_optional_prefix_in_place() {
        let owned_input = bstr::BString::from(":(optional)/some/path");
        let path = gix_config_value::Path::from(owned_input);

        assert!(path.is_optional);
        assert_eq!(path.value, "/some/path");
    }
}

mod resolve {
    use gix_config_value::{Path, path::interpolate::Context};
    use gix_testtools::TestResult;

    #[test]
    fn only_missing_optional_paths_are_omitted() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        std::fs::write(dir.path().join("exists"), "")?;
        let context = Context {
            home_dir: Some(dir.path()),
            ..Default::default()
        };
        for (input, expected) in [
            ("~/exists", Some(dir.path().join("exists"))),
            ("~/missing", Some(dir.path().join("missing"))),
            (":(optional)~/exists", Some(dir.path().join("exists"))),
            (":(optional)~/missing", None),
        ] {
            assert_eq!(
                Path::from(input).resolve(context)?,
                expected,
                "{input:?} resolves after interpolation"
            );
            // CI's 32-bit images can also predate optional path support.
            if *gix_testtools::GIT_VERSION >= (2, 53, 0) {
                let git = gix_testtools::git_command(dir.path())
                    .args([
                        "-c",
                        &format!("test.path={input}"),
                        "config",
                        "--get",
                        "--path",
                        "test.path",
                    ])
                    .env("HOME", dir.path())
                    .output()?;
                assert_eq!(
                    git.status.code(),
                    Some(i32::from(expected.is_none())),
                    "Git agrees for {input:?}"
                );
                let output = String::from_utf8(git.stdout)?;
                let actual = output.strip_suffix('\n').map(std::path::PathBuf::from);
                assert_eq!(actual, expected, "Git returns the same interpolated path");
            }
        }
        assert_eq!(
            Path::from("relative-missing-path").resolve(context)?,
            Some("relative-missing-path".into()),
            "required relative paths are returned without requiring their existence"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn dangling_links_are_missing_but_other_io_failures_are_errors() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        std::os::unix::fs::symlink("missing", dir.path().join("dangling"))?;
        std::fs::write(dir.path().join("file"), "")?;
        let context = Context {
            home_dir: Some(dir.path()),
            ..Default::default()
        };
        assert_eq!(
            Path::from(":(optional)~/dangling").resolve(context)?,
            None,
            "stat follows dangling symlinks"
        );
        let err = Path::from(":(optional)~/file/child")
            .resolve(context)
            .expect_err("ENOTDIR is not a missing optional path");
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(std::io::ErrorKind::NotADirectory),
            "the original filesystem error remains available"
        );
        if *gix_testtools::GIT_VERSION >= (2, 53, 0) {
            for (input, status) in [(":(optional)~/dangling", 1), (":(optional)~/file/child", 128)] {
                let git = gix_testtools::git_command(dir.path())
                    .args([
                        "-c",
                        &format!("test.path={input}"),
                        "config",
                        "--get",
                        "--path",
                        "test.path",
                    ])
                    .env("HOME", dir.path())
                    .output()?;
                assert_eq!(
                    git.status.code(),
                    Some(status),
                    "Git distinguishes missing paths from ENOTDIR"
                );
                assert!(git.stdout.is_empty(), "failed or missing paths produce no output");
            }
        }
        Ok(())
    }

    #[test]
    fn empty_paths_match_git() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        if *gix_testtools::GIT_VERSION >= (2, 53, 0) {
            for (input, status, output) in [("", 0, "\n"), (":(optional)", 1, "")] {
                let git = gix_testtools::git_command(dir.path())
                    .args([
                        "-c",
                        &format!("test.path={input}"),
                        "config",
                        "--get",
                        "--path",
                        "test.path",
                    ])
                    .output()?;
                assert_eq!(
                    git.status.code(),
                    Some(status),
                    "Git accepts an explicit empty path and omits an optional empty path"
                );
                assert_eq!(git.stdout, output.as_bytes(), "{input:?} matches Git's output");
            }
        }
        assert_eq!(
            Path::from("").resolve(Context::default())?,
            Some(std::path::PathBuf::new()),
            "an explicit empty config path is returned unchanged"
        );
        assert_eq!(
            Path::from(":(optional)").resolve(Context::default())?,
            None,
            "an optional empty config path has no filesystem target"
        );
        Ok(())
    }

    #[test]
    fn interpolation_errors_are_unchanged() {
        for input in ["~", ":(optional)~"] {
            let expected = Path::from(input)
                .interpolate(Context::default())
                .expect_err("invalid interpolation");
            let actual = Path::from(input)
                .resolve(Context::default())
                .expect_err("invalid interpolation");
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "{input:?} retains its interpolation error"
            );
        }
    }
}
