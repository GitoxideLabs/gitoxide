use bstr::ByteSlice;
use gix_path::{to_unix_separators, to_windows_separators};

#[test]
fn assure_unix_separators() {
    assert_eq!(to_unix_separators(b"no-backslash".as_bstr()).as_bstr(), "no-backslash");

    assert_eq!(to_unix_separators(br"\a\b\\".as_bstr()).as_bstr(), "/a/b//");
}

#[test]
fn assure_windows_separators() {
    assert_eq!(
        to_windows_separators(b"no-backslash".as_bstr()).as_bstr(),
        "no-backslash"
    );

    assert_eq!(to_windows_separators(b"/a/b//".as_bstr()).as_bstr(), r"\a\b\\");
}

mod normalize;

fn assert_roundtrip(bytes: &[u8]) -> gix_testtools::Result {
    use std::borrow::Cow;

    let path = gix_path::from_byte_slice(bytes)?;
    let borrowed = gix_path::into_bstr(path)?;
    assert!(matches!(borrowed, Cow::Borrowed(_)), "borrowed paths stay borrowed");
    assert_eq!(
        borrowed.as_ptr(),
        bytes.as_ptr(),
        "borrowed conversion keeps the original storage"
    );
    assert_eq!(borrowed.as_ref(), bytes.as_bstr(), "conversion preserves every byte");
    assert!(
        matches!(gix_path::from_bstr(bytes.as_bstr())?, Cow::Borrowed(_)),
        "borrowed bytes stay borrowed"
    );

    let owned = gix_path::from_bstring(bytes.as_bstr())?;
    let owned = gix_path::into_bstr(owned)?;
    assert!(matches!(owned, Cow::Owned(_)), "owned paths stay owned");
    assert_eq!(owned.as_ref(), bytes.as_bstr(), "owned conversion preserves every byte");
    let owned = gix_path::from_bstr(owned)?;
    assert!(matches!(owned, Cow::Owned(_)), "owned bytes stay owned");
    assert_eq!(owned.as_ref(), path, "owned and borrowed conversions agree");
    assert_eq!(gix_path::os_str_into_bstr(path.as_os_str())?, bytes.as_bstr());
    assert_eq!(
        gix_path::os_string_into_bstring(path.as_os_str().to_owned())?,
        bytes.as_bstr()
    );
    assert!(matches!(
        gix_path::try_os_str_into_bstr(Cow::Borrowed(path.as_os_str()))?,
        Cow::Borrowed(_)
    ));
    assert!(matches!(
        gix_path::try_os_str_into_bstr(Cow::Owned(path.as_os_str().to_owned()))?,
        Cow::Owned(_)
    ));
    Ok(())
}

#[test]
fn unicode_paths_preserve_bytes_and_ownership() -> gix_testtools::TestResult {
    for input in ["", "directory/file", "é/😀"] {
        assert_roundtrip(input.as_bytes())?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn unix_paths_preserve_non_utf8_bytes() -> gix_testtools::TestResult {
    let bytes = b"directory/\xff\x80\xed\xa0\x80";
    assert_roundtrip(bytes)?;
    assert_eq!(
        gix_path::to_native_path_on_windows(bytes.as_bstr())?
            .as_os_str()
            .as_encoded_bytes(),
        bytes,
        "Unix native paths retain arbitrary bytes"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn all_conversions_reject_invalid_windows_encodings() -> gix_testtools::TestResult {
    use std::{
        borrow::Cow,
        ffi::OsString,
        os::windows::ffi::OsStringExt,
        path::{Path, PathBuf},
    };

    let check = |err: gix_error::Error| {
        assert!(err.is_validation(), "invalid path encoding is a validation error");
        assert!(
            err.downcast_any_ref::<std::str::Utf8Error>().is_some()
                || err.downcast_any_ref::<std::string::FromUtf8Error>().is_some(),
            "the encoding failure remains available in the error chain"
        );
    };
    for surrogate in [0xd800, 0xdc00] {
        let path = OsString::from_wide(&[surrogate]);
        check(gix_path::into_bstr(Path::new(&path)).expect_err("borrowed native path rejects lone surrogates"));
        check(gix_path::into_bstr(PathBuf::from(&path)).expect_err("owned native path rejects lone surrogates"));
        check(gix_path::os_str_into_bstr(&path).expect_err("borrowed OS string rejects lone surrogates"));
        check(gix_path::os_string_into_bstring(path.clone()).expect_err("owned OS string rejects lone surrogates"));
        check(gix_path::try_os_str_into_bstr(Cow::Borrowed(&path)).expect_err("borrowed Cow rejects lone surrogates"));
        check(gix_path::try_os_str_into_bstr(Cow::Owned(path)).expect_err("owned Cow rejects lone surrogates"));
    }
    for bytes in [b"\xff".as_slice(), b"\xc0\x80", b"\xed\xa0\x80", b"\xe2\x82"] {
        check(gix_path::from_byte_slice(bytes).expect_err("byte slice must be UTF-8"));
        check(gix_path::from_bstring(bytes.as_bstr()).expect_err("owned bytes must be UTF-8"));
        check(gix_path::from_bstr(bytes.as_bstr()).expect_err("borrowed bytes must be UTF-8"));
        check(gix_path::from_bstr(bytes.as_bstr().to_owned()).expect_err("owned Cow must be UTF-8"));
        check(
            gix_path::to_native_path_on_windows(bytes.as_bstr())
                .expect_err("native separators do not hide invalid UTF-8"),
        );
    }
    let pair = OsString::from_wide(&[0xd83d, 0xde00]);
    assert_eq!(
        gix_path::os_str_into_bstr(&pair)?,
        "😀",
        "paired surrogates are valid Unicode"
    );
    assert_eq!(
        gix_path::from_bstring("😀")?.as_os_str(),
        pair,
        "valid pairs round-trip losslessly"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn invalid_encodings_retain_their_sources() {
    let mut error_snapshots = Vec::new();
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    let path = OsString::from_wide(&[0xd800]);
    for err in [
        gix_path::os_str_into_bstr(&path).expect_err("a lone surrogate is not UTF-8"),
        gix_path::os_string_into_bstring(path).expect_err("owned paths reject lone surrogates too"),
        gix_path::from_byte_slice(b"\xff").expect_err("Windows paths require UTF-8"),
        gix_path::from_bstring(b"\xff".as_bstr()).expect_err("owned Windows paths require UTF-8"),
    ] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_validation(), "invalid encodings remain validation errors");
        assert!(
            err.downcast_any_ref::<std::str::Utf8Error>().is_some()
                || err.downcast_any_ref::<std::string::FromUtf8Error>().is_some(),
            "both borrowed and owned conversions retain the concrete encoding failure"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "borrowed and owned conversions retain the malformed UTF-8 cause", @"
    [
        Could not convert to UTF8 or from UTF8 due to ill-formed input
        
        Caused by:
            0: invalid utf-8 sequence of 1 bytes from index 0,
        Could not convert to UTF8 or from UTF8 due to ill-formed input
        
        Caused by:
            0: invalid utf-8 sequence of 1 bytes from index 0,
        Could not convert to UTF8 or from UTF8 due to ill-formed input
        
        Caused by:
            0: invalid utf-8 sequence of 1 bytes from index 0,
        Could not convert to UTF8 or from UTF8 due to ill-formed input
        
        Caused by:
            0: invalid utf-8 sequence of 1 bytes from index 0,
    ]
    ");
}

mod normalize_and_clean {
    use std::{borrow::Cow, path::Path};

    use gix_path::normalize_and_clean;

    fn p(input: &str) -> &Path {
        Path::new(input)
    }

    #[test]
    fn clean_paths_and_use_the_cwd_for_empty_results() {
        let cwd = p("/cwd");
        for (input, expected) in [
            ("", "/cwd"),
            (".", "/cwd"),
            ("./a", "a"),
            ("a/./b", "a/b"),
            ("a//b/", "a/b"),
            ("a/..", "/cwd"),
        ] {
            assert_eq!(
                normalize_and_clean(p(input).into(), cwd).expect("path can be normalized"),
                p(expected),
                "\"{input}\" cleans to \"{expected}\""
            );
        }
        assert_eq!(
            normalize_and_clean(p(".").into(), p(""))
                .expect("path can be normalized")
                .as_ref(),
            p(""),
            "an empty CWD allows an empty normalized path"
        );
        assert_eq!(
            normalize_and_clean(p(".").into(), cwd)
                .expect("path can be normalized")
                .as_ref(),
            cwd,
            "an empty input path is the CWD"
        );
        assert!(
            matches!(normalize_and_clean(p("a/b").into(), cwd), Some(Cow::Borrowed(path)) if path == p("a/b")),
            "already-clean borrowed paths stay borrowed"
        );
    }
}

mod normalize_saturating {
    use std::{borrow::Cow, path::Path};

    use gix_path::normalize_saturating;

    fn p(input: &str) -> &Path {
        Path::new(input)
    }

    #[test]
    fn walking_up_too_much_stays_at_the_root() -> gix_testtools::TestResult {
        let cwd = "/users/name".as_ref();
        assert_eq!(
            normalize_saturating(p("./a/b/../../../../../actually-valid").into(), cwd)?.as_ref(),
            p("/actually-valid")
        );
        assert_eq!(
            normalize_saturating(p("/a/b/../../../../actually-valid").into(), cwd)?.as_ref(),
            p("/actually-valid")
        );
        assert_eq!(
            normalize_saturating(p("/a/b/../../../../..").into(), cwd)?.as_ref(),
            p("/")
        );
        Ok(())
    }

    #[test]
    fn preserves_normal_paths() -> gix_testtools::TestResult {
        let path = p("a/b");
        assert!(matches!(normalize_saturating(path.into(), p("/users"))?, Cow::Borrowed(actual) if actual == path));
        assert_eq!(
            normalize_saturating(p("a/../b").into(), p("/users"))?.as_ref(),
            p("b"),
            "paths that do not reach the root normalize as usual"
        );
        Ok(())
    }

    #[test]
    fn exhausted_relative_cwd_returns_an_error() -> gix_testtools::TestResult {
        for (input, cwd) in [("../../x", "a"), ("../..", "")] {
            let err = normalize_saturating(p(input).into(), p(cwd))
                .expect_err("the relative current directory cannot supply another parent");
            assert!(
                err.is_validation(),
                "an exhausted current directory is a validation error"
            );
        }
        for cwd in ["", "a"] {
            assert!(
                matches!(normalize_saturating(p("a/b").into(), p(cwd))?, Cow::Borrowed(_)),
                "an unused current directory does not prevent borrowing an unchanged path"
            );
            assert_eq!(
                normalize_saturating(p("a/../b").into(), p(cwd))?.as_ref(),
                p("b"),
                "local normalization does not require an absolute current directory"
            );
        }
        Ok(())
    }
}

mod join_bstr_unix_pathsep {
    use bstr::BStr;
    use gix_path::join_bstr_unix_pathsep;

    fn b(s: &str) -> &BStr {
        s.into()
    }

    #[test]
    fn typical_with_double_slash_avoidance() {
        assert_eq!(join_bstr_unix_pathsep(b("base"), "path"), b("base/path"));
        assert_eq!(
            join_bstr_unix_pathsep(b("base/"), "path"),
            b("base/path"),
            "no double slashes"
        );
        assert_eq!(join_bstr_unix_pathsep(b("/base"), "path"), b("/base/path"));
        assert_eq!(join_bstr_unix_pathsep(b("/base/"), "path"), b("/base/path"));
    }
    #[test]
    fn relative_base_or_path_are_nothing_special() {
        assert_eq!(join_bstr_unix_pathsep(b("base"), "."), b("base/."));
        assert_eq!(join_bstr_unix_pathsep(b("base"), ".."), b("base/.."));
        assert_eq!(join_bstr_unix_pathsep(b("base"), "../dir"), b("base/../dir"));
    }
    #[test]
    fn absolute_path_produces_double_slashes() {
        assert_eq!(join_bstr_unix_pathsep(b("/base"), "/root"), b("/base//root"));
        assert_eq!(join_bstr_unix_pathsep(b("base/"), "/root"), b("base//root"));
    }
    #[test]
    fn empty_path_makes_base_end_with_a_slash() {
        assert_eq!(join_bstr_unix_pathsep(b("base"), ""), b("base/"));
        assert_eq!(join_bstr_unix_pathsep(b("base/"), ""), b("base/"));
    }
    #[test]
    fn empty_base_leaves_everything_untouched() {
        assert_eq!(join_bstr_unix_pathsep(b(""), ""), b(""));
        assert_eq!(join_bstr_unix_pathsep(b(""), "hi"), b("hi"));
        assert_eq!(join_bstr_unix_pathsep(b(""), "/hi"), b("/hi"));
    }
}

mod relativize_with_prefix {
    fn r(path: &str, prefix: &str) -> String {
        gix_path::to_unix_separators_on_windows(
            gix_path::os_str_into_bstr(gix_path::relativize_with_prefix(path.as_ref(), prefix.as_ref()).as_os_str())
                .expect("no illformed UTF-8"),
        )
        .to_string()
    }

    #[test]
    fn basics() {
        assert_eq!(
            r("a", "a"),
            ".",
            "reaching the prefix is signalled by a '.', the current dir"
        );
        assert_eq!(r("a/b/c", "a/b"), "c", "'c' is clearly within the current directory");
        assert_eq!(
            r("c/b/c", "a/b"),
            "../../c/b/c",
            "when there is a complete disjoint prefix, we have to get out of it with ../"
        );
        assert_eq!(
            r("a/a", "a/b"),
            "../a",
            "when there is mismatch, we have to get out of the CWD"
        );
        assert_eq!(
            r("a/a", ""),
            "a/a",
            "empty prefix means nothing happens (and no work is done)"
        );
        assert_eq!(r("", ""), "", "empty stays empty");
    }

    #[test]
    fn absolute_paths_with_a_shared_root() {
        let root = if cfg!(windows) { "C:/" } else { "/" };
        for (path, prefix, expected) in [
            ("repo/m1", "repo/a/b", "../../m1"),
            ("repo/a", "repo", "a"),
            ("", "repo/a", "../.."),
            ("repo/a", "", "repo/a"),
            ("repo", "repo", "."),
            ("", "", "."),
        ] {
            assert_eq!(
                r(&format!("{root}{path}"), &format!("{root}{prefix}")),
                expected,
                "absolute paths are relativized after removing their shared root"
            );
        }
    }

    #[test]
    fn mixed_absolute_and_relative_paths_are_unchanged() {
        let absolute = if cfg!(windows) { "C:/repo/a" } else { "/repo/a" };
        for (path, prefix) in [(absolute, "repo/b"), ("repo/a", absolute), (absolute, "")] {
            assert!(
                matches!(
                    gix_path::relativize_with_prefix(path.as_ref(), prefix.as_ref()),
                    std::borrow::Cow::Borrowed(actual) if actual == std::path::Path::new(path)
                ),
                "without a common coordinate system or a prefix, the original path is borrowed"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn windows_roots() {
        assert_eq!(
            r(r"\\server\share\repo\m1", r"\\server\share\repo\a\b"),
            "../../m1",
            "paths on the same UNC share can be relativized"
        );
        for (path, prefix) in [
            ("C:/repo/a", "D:/repo/b"),
            (r"\\server\share\repo\a", r"\\other\share\repo\b"),
            (r"\\server\share\repo\a", r"\\server\other\repo\b"),
            ("C:/repo/a", r"\\?\C:\repo\b"),
        ] {
            assert!(
                matches!(
                    gix_path::relativize_with_prefix(path.as_ref(), prefix.as_ref()),
                    std::borrow::Cow::Borrowed(actual) if actual == std::path::Path::new(path)
                ),
                "paths on different drives, shares, or namespaces stay absolute"
            );
        }
    }
}
