use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url, url_with_pass};

#[test]
fn username_expansion_is_unsupported() -> crate::Result {
    assert_url_roundtrip(
        "http://example.com/~byron/hello",
        url(Scheme::Http, None, "example.com", None, b"/~byron/hello"),
    )
}

#[test]
fn empty_user_cannot_roundtrip() -> crate::Result {
    let actual = gix_url::parse("http://@example.com/~byron/hello".into())?;
    let expected = url(Scheme::Http, None, "example.com", None, b"/~byron/hello");
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_bstring(),
        "http://example.com/~byron/hello",
        "we cannot differentiate between empty user and no user"
    );
    Ok(())
}

#[test]
fn username_and_password() -> crate::Result {
    assert_url_roundtrip(
        "http://user:password@example.com/~byron/hello",
        url_with_pass(Scheme::Http, "user", "password", "example.com", None, b"/~byron/hello"),
    )
}

#[test]
fn username_and_password_and_port() -> crate::Result {
    assert_url_roundtrip(
        "http://user:password@example.com:8080/~byron/hello",
        url_with_pass(Scheme::Http, "user", "password", "example.com", 8080, b"/~byron/hello"),
    )
}

#[test]
fn username_and_password_with_spaces_and_port() -> crate::Result {
    let expected = gix_url::Url::from_parts(
        Scheme::Http,
        Some("user name".into()),
        Some("password secret".into()),
        Some("example.com".into()),
        Some(8080),
        b"/~byron/hello".into(),
        false,
    )?;
    assert_url_roundtrip(
        "http://user%20name:password%20secret@example.com:8080/~byron/hello",
        expected.clone(),
    )?;
    assert_eq!(expected.user(), Some("user name"));
    assert_eq!(expected.password(), Some("password secret"));
    Ok(())
}

#[test]
fn only_password() -> crate::Result {
    assert_url_roundtrip(
        "http://:password@example.com/~byron/hello",
        url_with_pass(Scheme::Http, "", "password", "example.com", None, b"/~byron/hello"),
    )
}

#[test]
fn username_and_empty_password() -> crate::Result {
    let actual = gix_url::parse("http://user:@example.com/~byron/hello".into())?;
    let expected = url(Scheme::Http, "user", "example.com", None, b"/~byron/hello");
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_bstring(),
        "http://user@example.com/~byron/hello",
        "an empty password appears like no password to us - fair enough"
    );
    Ok(())
}

#[test]
fn secure() -> crate::Result {
    assert_url_roundtrip(
        "https://github.com/byron/gitoxide",
        url(Scheme::Https, None, "github.com", None, b"/byron/gitoxide"),
    )
}

#[test]
fn http_missing_path() -> crate::Result {
    assert_url_roundtrip("http://host.xz/", url(Scheme::Http, None, "host.xz", None, b"/"))?;
    assert_url("http://host.xz", url(Scheme::Http, None, "host.xz", None, b"/"))?;
    Ok(())
}

#[test]
fn username_with_dot_is_not_percent_encoded() -> crate::Result {
    assert_url_roundtrip(
        "http://user.name@example.com/repo",
        url(Scheme::Http, "user.name", "example.com", None, b"/repo"),
    )
}

#[test]
fn password_with_dot_is_not_percent_encoded() -> crate::Result {
    assert_url_roundtrip(
        "http://user:pass.word@example.com/repo",
        url_with_pass(Scheme::Http, "user", "pass.word", "example.com", None, b"/repo"),
    )
}

#[test]
fn username_and_password_with_dots_are_not_percent_encoded() -> crate::Result {
    assert_url_roundtrip(
        "http://user.name:pass.word@example.com/repo",
        url_with_pass(Scheme::Http, "user.name", "pass.word", "example.com", None, b"/repo"),
    )
}

#[test]
fn http_with_ipv6() -> crate::Result {
    assert_url_roundtrip("http://[::1]/repo", url(Scheme::Http, None, "[::1]", None, b"/repo"))
}

#[test]
fn http_with_ipv6_and_port() -> crate::Result {
    assert_url_roundtrip(
        "http://[::1]:8080/repo",
        url(Scheme::Http, None, "[::1]", 8080, b"/repo"),
    )
}

#[test]
fn https_with_ipv6_user_and_port() -> crate::Result {
    assert_url_roundtrip(
        "https://user@[2001:db8::1]:8443/repo",
        url(Scheme::Https, "user", "[2001:db8::1]", 8443, b"/repo"),
    )
}

#[test]
fn percent_encoded_path() -> crate::Result {
    let url = gix_url::parse("https://example.com/path/with%20spaces/file".into())?;
    assert_eq!(url.path, "/path/with spaces/file", "paths are now decoded");
    Ok(())
}

#[test]
fn percent_encoded_international_path() -> crate::Result {
    let url = gix_url::parse("https://example.com/caf%C3%A9".into())?;
    assert_eq!(url.path, "/café", "international characters are decoded in path");
    Ok(())
}
