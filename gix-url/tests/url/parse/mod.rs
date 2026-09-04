use bstr::ByteSlice;
use gix_url::{Scheme, parse, testing::TestUrlExtension};

fn assert_url(url: &str, expected: gix_url::Url) -> Result<gix_url::Url, gix_url::parse::Error> {
    let actual = gix_url::parse(url)?;
    assert_eq!(actual, expected);
    // Note that this must not match on the name, as `Scheme::Helper("http")` is a remote helper.
    if matches!(actual.scheme, Scheme::Http | Scheme::Https) {
        assert!(
            actual.path.starts_with_str("/"),
            "paths are never empty and at least '/': {:?}",
            actual.path
        );
        if actual.path.len() < 2 {
            assert!(actual.path_is_root());
        }
    }
    Ok(expected)
}

fn assert_url_roundtrip(url: &str, expected: gix_url::Url) -> Result<(), gix_url::parse::Error> {
    assert_eq!(assert_url(url, expected)?.to_bstring(), url);
    Ok(())
}

fn url<'a, 'b>(
    protocol: Scheme,
    user: impl Into<Option<&'a str>>,
    host: impl Into<Option<&'b str>>,
    port: impl Into<Option<u16>>,
    path: &[u8],
) -> gix_url::Url {
    gix_url::Url::from_parts_unchecked(
        protocol,
        user.into().map(Into::into),
        None,
        host.into().map(Into::into),
        port.into(),
        path.into(),
        false,
    )
}

fn url_with_pass<'a, 'b>(
    protocol: Scheme,
    user: impl Into<Option<&'a str>>,
    password: impl Into<String>,
    host: impl Into<Option<&'b str>>,
    port: impl Into<Option<u16>>,
    path: &[u8],
) -> gix_url::Url {
    gix_url::Url::from_parts_unchecked(
        protocol,
        user.into().map(Into::into),
        Some(password.into()),
        host.into().map(Into::into),
        port.into(),
        path.into(),
        false,
    )
}

fn url_alternate<'a, 'b>(
    protocol: Scheme,
    user: impl Into<Option<&'a str>>,
    host: impl Into<Option<&'b str>>,
    port: impl Into<Option<u16>>,
    path: &[u8],
) -> gix_url::Url {
    gix_url::Url::from_parts_unchecked(
        protocol.clone(),
        user.into().map(Into::into),
        None,
        host.into().map(Into::into),
        port.into(),
        path.into(),
        true,
    )
}

mod file;
mod invalid;
mod remote_helper;
mod ssh;

mod radicle {
    use gix_url::Scheme;

    use crate::parse::{assert_url_roundtrip, url};

    #[test]
    fn basic() -> crate::Result {
        Ok(assert_url_roundtrip(
            "rad://hynkuwzskprmswzeo4qdtku7grdrs4ffj3g9tjdxomgmjzhtzpqf81@hwd1yregyf1dudqwkx85x5ps3qsrqw3ihxpx3ieopq6ukuuq597p6m8161c.git",
            url(
                Scheme::HelperUrl("rad".into()),
                "hynkuwzskprmswzeo4qdtku7grdrs4ffj3g9tjdxomgmjzhtzpqf81",
                "hwd1yregyf1dudqwkx85x5ps3qsrqw3ihxpx3ieopq6ukuuq597p6m8161c.git",
                None,
                b"",
            ),
        )?)
    }
}

mod http;

mod ports {
    use crate::parse::{assert_url_roundtrip, url};
    use gix_url::Scheme;

    #[test]
    fn max_valid_port() -> crate::Result {
        Ok(assert_url_roundtrip(
            "ssh://host.xz:65535/repo",
            url(Scheme::Ssh, None, "host.xz", 65535, b"/repo"),
        )?)
    }

    #[test]
    fn port_one() -> crate::Result {
        Ok(assert_url_roundtrip(
            "ssh://host.xz:1/repo",
            url(Scheme::Ssh, None, "host.xz", 1, b"/repo"),
        )?)
    }
}

mod git {
    use gix_url::Scheme;

    use crate::parse::{assert_url_roundtrip, url};

    #[test]
    fn username_expansion_with_username() -> crate::Result {
        Ok(assert_url_roundtrip(
            "git://example.com/~byron/hello",
            url(Scheme::Git, None, "example.com", None, b"~byron/hello"),
        )?)
    }

    #[test]
    fn default_port_is_9418() -> crate::Result {
        let url = url(Scheme::Git, None, "example.com", None, b"/repo");
        assert_eq!(url.port_or_default(), Some(9418));
        Ok(())
    }

    #[test]
    fn git_with_explicit_port() -> crate::Result {
        Ok(assert_url_roundtrip(
            "git://example.com:1234/repo",
            url(Scheme::Git, None, "example.com", 1234, b"/repo"),
        )?)
    }
}

mod unknown {
    use gix_url::Scheme;

    use crate::parse::{assert_url_roundtrip, url};

    #[test]
    fn any_protocol_is_supported_via_a_remote_helper_url() -> crate::Result {
        Ok(assert_url_roundtrip(
            "abc://example.com/~byron/hello",
            url(
                Scheme::HelperUrl("abc".into()),
                None,
                "example.com",
                None,
                b"/~byron/hello",
            ),
        )?)
    }
}
