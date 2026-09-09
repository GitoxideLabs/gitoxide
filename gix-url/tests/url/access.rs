mod canonicalized {
    use std::borrow::Cow;

    #[test]
    fn non_file_scheme_is_noop() -> crate::Result {
        let url = gix_url::parse("https://github.com/byron/gitoxide")?;
        assert_eq!(url.canonicalized(&std::env::current_dir()?)?, url);
        Ok(())
    }

    #[test]
    fn absolute_file_url_does_nothing() -> crate::Result {
        #[cfg(not(windows))]
        let url = gix_url::parse("/this/path/does/not/exist")?;
        #[cfg(windows)]
        let url = gix_url::parse(r"C:\non\existing")?;
        assert_eq!(url.canonicalized(&std::env::current_dir()?)?, url);
        Ok(())
    }

    #[test]
    fn file_that_is_current_dir_is_absolutized() -> crate::Result {
        let url = gix_url::parse(".")?;
        assert!(gix_path::from_bstr(Cow::Borrowed(url.path.as_ref())).is_relative());
        assert!(
            gix_path::from_bstr(Cow::Borrowed(
                url.canonicalized(&std::env::current_dir()?)?.path.as_ref()
            ))
            .is_absolute()
        );
        Ok(())
    }
}

use gix_url::ArgumentSafety;

mod path_query_fragment {
    use std::borrow::Cow;

    use bstr::BStr;
    use gix_url::{Scheme, Url};

    fn borrowed_pairs<'a>(pairs: &'a [(Cow<'_, BStr>, Cow<'_, BStr>)]) -> Vec<(&'a BStr, &'a BStr)> {
        pairs
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
            .collect()
    }

    fn str_pairs<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<(&'a BStr, &'a BStr)> {
        pairs
            .iter()
            .map(|&(name, value)| (BStr::new(name), BStr::new(value)))
            .collect()
    }

    #[test]
    fn http_delimiters_are_recognized_before_decoding() -> crate::Result {
        for scheme in ["http", "https"] {
            for (suffix, expected) in [
                ("/repo%23one?query=value#fragment", "/repo#one"),
                ("/repo%3Fone?query=value", "/repo?one"),
                ("/repo%2523one", "/repo%23one"),
                ("/repo%2523one?query=value", "/repo%23one"),
                ("/repo%3fone#fragment?query=value", "/repo?one"),
                ("/repo?query=%23value", "/repo"),
                ("/repo#fragment", "/repo"),
                ("/caf%C3%A9/%E2%98%83%23one?query=value", "/café/☃#one"),
                ("/repo%2Fone+two?query=value", "/repo/one+two"),
                ("/owner/example.github.io.git", "/owner/example.github.io.git"),
                ("", "/"),
                ("/", "/"),
                ("?query=value#fragment", "/"),
                ("#fragment?query=value", "/"),
                ("?query=%23value", "/"),
            ] {
                let input = format!("{scheme}://host{suffix}");
                let url = gix_url::parse(&input)?;
                assert_eq!(
                    url.path_query_fragment().path,
                    expected,
                    "only literal HTTP delimiters end the path, which is decoded once: {input}"
                );
                let serialized = if suffix.is_empty() {
                    format!("{input}/")
                } else {
                    input.clone()
                };
                assert_eq!(
                    url.to_bstring(),
                    serialized,
                    "serialization keeps the full URL: {input}"
                );
                assert_eq!(
                    url.original_path(),
                    if suffix.is_empty() { "/" } else { suffix },
                    "the original path still includes query and fragment text: {input}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn other_schemes_keep_their_stored_paths() -> crate::Result {
        for (input, expected) in [
            ("git@host:repo#one?two%23three", "repo#one?two%23three"),
            ("ssh://git@host/repo%23one?two", "/repo#one?two"),
            ("ssh://git@host/repo%2523one?two#three", "/repo%23one?two#three"),
            ("ssh://git@host/~repo%23one?two", "~repo#one?two"),
            ("git://host/repo%23one?two", "/repo#one?two"),
            ("file:///repo#one?two%23three", "/repo#one?two%23three"),
            ("./repo#one?two%23three", "./repo#one?two%23three"),
            ("http::repo#one?two%23three", "repo#one?two%23three"),
        ] {
            let url = gix_url::parse(input)?;
            let parts = url.path_query_fragment();
            assert_eq!(
                parts.path, expected,
                "non-HTTP paths retain literal delimiters and their existing decoding rules: {input}"
            );
            assert!(parts.query.is_none(), "non-HTTP paths have no query: {input}");
            assert!(parts.fragment.is_none(), "non-HTTP paths have no fragment: {input}");
            assert_eq!(url.path, expected, "the public path is unchanged: {input}");
        }
        Ok(())
    }

    #[test]
    fn constructed_and_mutated_http_paths_are_already_decoded() -> crate::Result {
        for scheme in [Scheme::Http, Scheme::Https] {
            for (path, expected) in [
                ("/repo%23one?query=value#fragment", "/repo%23one"),
                ("/repo%3Fone#fragment", "/repo%3Fone"),
                ("/repo%2523one", "/repo%2523one"),
                ("/repo#one?new=query", "/repo"),
                ("/café%23one?query=value", "/café%23one"),
                ("", "/"),
                ("?query=value", "/"),
                ("#fragment", "/"),
            ] {
                let constructed = Url::from_parts(
                    scheme.clone(),
                    None,
                    None,
                    Some("host".into()),
                    None,
                    path.into(),
                    false,
                )?;
                let mut mutated = gix_url::parse(format!("{scheme}://host/repo%23one?old=query"))?;
                mutated.path = path.into();
                for url in [constructed, mutated] {
                    assert_eq!(
                        url.path_query_fragment().path,
                        expected,
                        "caller-supplied paths keep literal percent escapes and ignore stale cached spelling: {path}"
                    );
                    assert_eq!(
                        gix_url::parse(url.to_bstring())?.path_query_fragment(),
                        url.path_query_fragment(),
                        "decoded components agree with serialization: {path}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn mutation_keeps_paths_byte_oriented_and_uses_the_current_scheme() -> crate::Result {
        let mut url = gix_url::parse("https://host/repo%23one?query=value#fragment")?;
        url.scheme = Scheme::Ssh;
        assert_eq!(
            url.path_query_fragment().path,
            "/repo#one?query=value#fragment",
            "changing to SSH makes all delimiters path data"
        );
        url.scheme = Scheme::Https;
        assert_eq!(
            url.path_query_fragment().path,
            "/repo#one",
            "unchanged parsed paths retain encoded delimiters when changing back to HTTP"
        );
        url.path = b"/repo\xff%23?na\xfe+me=va\xfd%26+lue#frag\xfc+%23".into();
        let parts = url.path_query_fragment();
        assert_eq!(
            parts.path,
            b"/repo\xff%23".as_slice(),
            "mutated paths need neither UTF-8 conversion nor another percent-decoding pass"
        );
        let query = parts.query.expect("the query delimiter is present");
        assert_eq!(query.len(), 1, "the query contains one name/value pair");
        assert_eq!(
            query[0].0.as_ref(),
            b"na\xfe me".as_slice(),
            "query names remain byte-oriented"
        );
        assert_eq!(
            query[0].1.as_ref(),
            b"va\xfd%26 lue".as_slice(),
            "form decoding preserves arbitrary bytes and literal percent text"
        );
        assert_eq!(
            parts.fragment,
            Some(BStr::new(b"frag\xfc+%23")),
            "fragments keep arbitrary bytes, plus signs, and caller-supplied percent text"
        );
        Ok(())
    }

    #[test]
    fn query_pairs_and_fragments_are_split_before_decoding() -> crate::Result {
        for scheme in ["http", "https"] {
            for (suffix, path, query, fragment) in [
                (
                    "/repo%23one?x=a%26b&x=c%3Dd#frag%23one%3Ftwo",
                    "/repo#one",
                    &[("x", "a&b"), ("x", "c=d")][..],
                    Some("frag#one?two"),
                ),
                (
                    "/repo?key%26name%3Dpart=value%3Fone%23two&flag&empty=&=value#frag%2523",
                    "/repo",
                    &[
                        ("key&name=part", "value?one#two"),
                        ("flag", ""),
                        ("empty", ""),
                        ("", "value"),
                    ][..],
                    Some("frag%23"),
                ),
                (
                    "/repo?x=%2526%253D%2523&x=a=b=c#%2523",
                    "/repo",
                    &[("x", "%26%3D%23"), ("x", "a=b=c")][..],
                    Some("%23"),
                ),
                (
                    "/repo+one?first+name=a+b&literal=%2B&mixed=+%2B%252B#fragment+%2B",
                    "/repo+one",
                    &[("first name", "a b"), ("literal", "+"), ("mixed", " +%2B")][..],
                    Some("fragment++"),
                ),
                (
                    "/caf%C3%A9?caf%C3%A9=%E2%98%83%26%3D#%F0%9F%A6%80",
                    "/café",
                    &[("café", "☃&=")][..],
                    Some("🦀"),
                ),
                (
                    "/repo?caf%C3%A9+%2B=%E2%98%83+%2B+#frag+%23",
                    "/repo",
                    &[("café +", "☃ + ")][..],
                    Some("frag+#"),
                ),
                ("/repo?&&x=a&&x=b&", "/repo", &[("x", "a"), ("x", "b")][..], None),
                (
                    "/repo?x=?one?two#fragment?x=y#rest",
                    "/repo",
                    &[("x", "?one?two")][..],
                    Some("fragment?x=y#rest"),
                ),
                ("?x=a%26b#frag", "/", &[("x", "a&b")][..], Some("frag")),
                ("/repo?x=a;b=two", "/repo", &[("x", "a;b=two")][..], None),
            ] {
                let input = format!("{scheme}://host{suffix}");
                let url = gix_url::parse(&input)?;
                let gix_url::PathComponents {
                    path: actual_path,
                    query: pairs,
                    fragment: actual_fragment,
                } = url.path_query_fragment();
                assert_eq!(actual_path, path, "query decoding does not change the path: {input}");
                let pairs = pairs.expect("each input contains a query delimiter");
                let actual = borrowed_pairs(&pairs);
                let expected = str_pairs(query);
                assert_eq!(
                    actual, expected,
                    "encoded separators remain data and duplicates retain their order: {input}"
                );
                assert_eq!(
                    actual_fragment,
                    fragment.map(BStr::new),
                    "fragments are decoded once without form decoding: {input}"
                );
                assert_eq!(url.to_bstring(), input, "splitting preserves serialization: {input}");
                assert_eq!(
                    url.original_path(),
                    suffix,
                    "the original spelling remains available: {input}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn empty_and_absent_components_remain_distinct() -> crate::Result {
        for (suffix, has_query, fragment) in [
            ("", false, None),
            ("/repo", false, None),
            ("/repo?", true, None),
            ("/repo#", false, Some("")),
            ("/repo?#", true, Some("")),
            ("?&&", true, None),
            ("#fragment?x=y", false, Some("fragment?x=y")),
        ] {
            let url = gix_url::parse(format!("https://host{suffix}"))?;
            let parts = url.path_query_fragment();
            assert_eq!(
                parts.query.as_ref().map(Vec::len),
                has_query.then_some(0),
                "an explicit empty query is distinct from an absent query: {suffix}"
            );
            assert_eq!(
                parts.fragment,
                fragment.map(BStr::new),
                "a question mark after the fragment delimiter does not start a query: {suffix}"
            );
        }
        Ok(())
    }

    #[test]
    fn constructed_and_mutated_components_keep_literal_percent_escapes() -> crate::Result {
        let path = "/repo%23one?x=a%26b&space=+%2B#frag%23+";
        let constructed = Url::from_parts(Scheme::Https, None, None, Some("host".into()), None, path.into(), false)?;
        let mut mutated = gix_url::parse("https://host/old%23repo?x=%26#old%23fragment")?;
        mutated.path = path.into();
        for url in [constructed, mutated] {
            let parts = url.path_query_fragment();
            assert_eq!(parts.path, "/repo%23one", "caller-supplied path escapes are literal");
            let pairs = parts.query.as_ref().expect("the query delimiter is present");
            let actual = borrowed_pairs(pairs);
            assert_eq!(
                actual,
                str_pairs(&[("x", "a%26b"), ("space", " %2B")]),
                "form decoding replaces literal plus signs without interpreting caller-supplied percent escapes"
            );
            assert_eq!(
                parts.fragment,
                Some(BStr::new("frag%23+")),
                "fragment escapes remain literal"
            );
            assert_eq!(
                gix_url::parse(url.to_bstring())?.path_query_fragment(),
                parts,
                "all components agree with serialization"
            );
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_preserves_component_boundaries() -> crate::Result {
        let mut mutated = gix_url::parse("https://host/old%23repo?x=%26#old%23fragment")?;
        mutated.path = "/repo%23one?x=a%26b&space=+%2B#frag%23+".into();
        for url in [
            gix_url::parse("https://host/repo%23one?x=a%26b&space=+%2B#frag%23+")?,
            mutated,
        ] {
            let restored: Url = serde_json::from_slice(&serde_json::to_vec(&url)?)?;
            assert_eq!(
                restored.path_query_fragment(),
                url.path_query_fragment(),
                "deserialization preserves valid cached boundaries and ignores stale cached spelling"
            );
        }
        Ok(())
    }
}

#[test]
fn user() -> crate::Result {
    let mut url = gix_url::parse("https://user:password@host/path")?;

    assert_eq!(url.user(), Some("user"));
    assert_eq!(url.set_user(Some("new-user".into())), Some("user".into()));
    assert_eq!(url.user(), Some("new-user"));

    Ok(())
}

#[test]
fn password() -> crate::Result {
    let mut url = gix_url::parse("https://user:password@host/path")?;

    assert_eq!(url.password(), Some("password"));
    assert_eq!(url.set_password(Some("new-pass".into())), Some("password".into()));
    assert_eq!(url.password(), Some("new-pass"));

    Ok(())
}

#[test]
fn mutation_roundtrip() -> crate::Result {
    let mut url = gix_url::parse("https://user@host/path")?;
    url.set_user(Some("newuser".into()));
    url.set_password(Some("secret".into()));

    let serialized = url.to_bstring();
    let reparsed = gix_url::parse(&serialized)?;

    assert_eq!(url, reparsed);
    assert_eq!(reparsed.user(), Some("newuser"));
    assert_eq!(reparsed.password(), Some("secret"));

    Ok(())
}

#[test]
fn from_bytes_roundtrip() -> crate::Result {
    let original = "https://user:password@example.com:8080/path/to/repo";
    let url = gix_url::parse(original)?;

    let bytes = url.to_bstring();
    let from_bytes = gix_url::Url::from_bytes(bytes.as_ref())?;

    assert_eq!(url, from_bytes);
    assert_eq!(from_bytes.to_bstring(), bytes);

    Ok(())
}

#[test]
fn from_bytes_with_non_utf8_path() -> crate::Result {
    let url = gix_url::parse(b"/path/to\xff/repo".as_slice())?;
    let bytes = url.to_bstring();
    let from_bytes = gix_url::Url::from_bytes(bytes.as_ref())?;

    assert_eq!(url, from_bytes);
    assert_eq!(from_bytes.path, url.path);

    Ok(())
}

#[test]
fn user_argument_safety() -> crate::Result {
    let url = gix_url::parse("ssh://-Fconfigfile@foo/bar")?;

    assert_eq!(url.user(), Some("-Fconfigfile"));
    assert_eq!(url.user_as_argument(), ArgumentSafety::Dangerous("-Fconfigfile"));
    assert_eq!(url.user_argument_safe(), None, "An unsafe username is blocked.");

    assert_eq!(url.host(), Some("foo"));
    assert_eq!(url.host_as_argument(), ArgumentSafety::Usable("foo"));
    assert_eq!(url.host_argument_safe(), Some("foo"));

    assert_eq!(url.path, "/bar");
    assert_eq!(url.path_argument_safe(), Some("/bar".into()));

    Ok(())
}

#[test]
fn host_argument_safety() -> crate::Result {
    let url = gix_url::parse("ssh://-oProxyCommand=open$IFS-aCalculator/foo")?;

    assert_eq!(url.user(), None);
    assert_eq!(url.user_as_argument(), ArgumentSafety::Absent);
    assert_eq!(
        url.user_argument_safe(),
        None,
        "As there is no user. See all_argument_safe_valid()"
    );

    assert_eq!(url.host(), Some("-oProxyCommand=open$IFS-aCalculator"));
    assert_eq!(
        url.host_as_argument(),
        ArgumentSafety::Dangerous("-oProxyCommand=open$IFS-aCalculator")
    );
    assert_eq!(url.host_argument_safe(), None, "An unsafe host string is blocked");

    assert_eq!(url.path, "/foo");
    assert_eq!(url.path_argument_safe(), Some("/foo".into()));

    Ok(())
}

#[test]
fn path_argument_safety() -> crate::Result {
    let url = gix_url::parse("ssh://foo/-oProxyCommand=open$IFS-aCalculator")?;

    assert_eq!(url.user(), None);
    assert_eq!(url.user_as_argument(), ArgumentSafety::Absent);
    assert_eq!(
        url.user_argument_safe(),
        None,
        "As there is no user. See all_argument_safe_valid()"
    );

    assert_eq!(url.host(), Some("foo"));
    assert_eq!(url.host_as_argument(), ArgumentSafety::Usable("foo"));
    assert_eq!(url.host_argument_safe(), Some("foo"));

    assert_eq!(url.path, "/-oProxyCommand=open$IFS-aCalculator");
    assert_eq!(url.path_argument_safe(), None, "An unsafe path is blocked");

    for input in ["foo:path", "foo:-option"] {
        let url = gix_url::parse(input)?;
        assert_eq!(
            url.path_argument_safe(),
            None,
            "relative paths need validation at their command-line use site"
        );
    }

    Ok(())
}

#[test]
fn all_argument_safety_safe() -> crate::Result {
    let url = gix_url::parse("ssh://user.name@example.com/path/to/file")?;

    assert_eq!(url.user(), Some("user.name"));
    assert_eq!(url.user_as_argument(), ArgumentSafety::Usable("user.name"));
    assert_eq!(url.user_argument_safe(), Some("user.name"));

    assert_eq!(url.host(), Some("example.com"));
    assert_eq!(url.host_as_argument(), ArgumentSafety::Usable("example.com"));
    assert_eq!(url.host_argument_safe(), Some("example.com"));

    assert_eq!(url.path, "/path/to/file");
    assert_eq!(url.path_argument_safe(), Some("/path/to/file".into()));

    Ok(())
}

#[test]
fn all_argument_safety_not_safe() -> crate::Result {
    let all_bad = "ssh://-Fconfigfile@-oProxyCommand=open$IFS-aCalculator/-oProxyCommand=open$IFS-aCalculator";
    let url = gix_url::parse(all_bad)?;

    assert_eq!(url.user(), Some("-Fconfigfile"));
    assert_eq!(url.user_as_argument(), ArgumentSafety::Dangerous("-Fconfigfile"));
    assert_eq!(url.user_argument_safe(), None); // An unsafe username is blocked.

    assert_eq!(url.host(), Some("-oProxyCommand=open$IFS-aCalculator"));
    assert_eq!(
        url.host_as_argument(),
        ArgumentSafety::Dangerous("-oProxyCommand=open$IFS-aCalculator")
    );
    assert_eq!(url.host_argument_safe(), None, "An unsafe host string is blocked");

    assert_eq!(url.path, "/-oProxyCommand=open$IFS-aCalculator");
    assert_eq!(url.path_argument_safe(), None, "An unsafe path is blocked");

    Ok(())
}

#[test]
fn display() {
    fn compare(input: &str, expected: &str, message: &str) {
        let url = gix_url::parse(input).expect("input is valid url");
        assert_eq!(format!("{url}"), expected, "{message}");
    }

    compare(
        "ssh://foo/-oProxyCommand=open$IFS-aCalculator",
        "ssh://foo/-oProxyCommand=open$IFS-aCalculator",
        "it round-trips with sane unicode and without password",
    );
    compare("/path/to/repo", "/path/to/repo", "same goes for simple paths");
    compare(
        "https://user:password@host/path",
        "https://user:redacted@host/path",
        "it visibly redacts passwords though, and it's still a valid URL",
    );
}
