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
                "'{input}' cleans to '{expected}'"
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
    fn walking_up_too_much_stays_at_the_root() {
        let cwd = "/users/name".as_ref();
        assert_eq!(
            normalize_saturating(p("./a/b/../../../../../actually-valid").into(), cwd).as_ref(),
            p("/actually-valid")
        );
        assert_eq!(
            normalize_saturating(p("/a/b/../../../../actually-valid").into(), cwd).as_ref(),
            p("/actually-valid")
        );
        assert_eq!(
            normalize_saturating(p("/a/b/../../../../..").into(), cwd).as_ref(),
            p("/")
        );
    }

    #[test]
    fn preserves_normal_paths() {
        let path = p("a/b");
        assert!(matches!(normalize_saturating(path.into(), p("/users")), Cow::Borrowed(actual) if actual == path));
        assert_eq!(
            normalize_saturating(p("a/../b").into(), p("/users")).as_ref(),
            p("b"),
            "paths that do not reach the root normalize as usual"
        );
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
}
