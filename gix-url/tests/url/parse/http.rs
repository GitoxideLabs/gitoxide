use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url, url_with_pass};

fn test_password(with_dot: bool) -> String {
    let mut value = std::process::id().to_string();
    if with_dot {
        value.push('.');
        value.push_str(&std::process::id().to_string());
    }
    value
}

#[test]
fn username_expansion_is_unsupported() -> crate::Result {
    Ok(assert_url_roundtrip(
        "http://example.com/~byron/hello",
        url(Scheme::Http, None, "example.com", None, b"/~byron/hello"),
    )?)
}

#[test]
fn empty_user_cannot_roundtrip() -> crate::Result {
    let actual = gix_url::parse("http://@example.com/~byron/hello")?;
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
    let password = test_password(false);
    Ok(assert_url_roundtrip(
        &format!("http://user:{password}@example.com/~byron/hello"),
        url_with_pass(Scheme::Http, "user", password, "example.com", None, b"/~byron/hello"),
    )?)
}

#[test]
fn colon_in_username_roundtrips() -> crate::Result {
    Ok(assert_url_roundtrip(
        "http://a%3Ab@example.com/",
        url(Scheme::Http, "a:b", "example.com", None, b"/"),
    )?)
}

#[test]
fn colon_in_password_roundtrips() -> crate::Result {
    let password = format!("a:{}", std::process::id());
    Ok(assert_url_roundtrip(
        &format!("http://user:{password}@example.com/"),
        url_with_pass(Scheme::Http, "user", password, "example.com", None, b"/"),
    )?)
}

#[test]
fn username_and_password_and_port() -> crate::Result {
    let password = test_password(false);
    Ok(assert_url_roundtrip(
        &format!("http://user:{password}@example.com:8080/~byron/hello"),
        url_with_pass(Scheme::Http, "user", password, "example.com", 8080, b"/~byron/hello"),
    )?)
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
    let password = test_password(false);
    Ok(assert_url_roundtrip(
        &format!("http://:{password}@example.com/~byron/hello"),
        url_with_pass(Scheme::Http, "", password, "example.com", None, b"/~byron/hello"),
    )?)
}

#[test]
fn username_and_empty_password() -> crate::Result {
    let actual = gix_url::parse("http://user:@example.com/~byron/hello")?;
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
    Ok(assert_url_roundtrip(
        "https://github.com/byron/gitoxide",
        url(Scheme::Https, None, "github.com", None, b"/byron/gitoxide"),
    )?)
}

#[test]
fn http_missing_path() -> crate::Result {
    assert_url_roundtrip("http://host.xz/", url(Scheme::Http, None, "host.xz", None, b"/"))?;
    assert_url("http://host.xz", url(Scheme::Http, None, "host.xz", None, b"/"))?;
    Ok(())
}

#[test]
fn username_with_dot_is_not_percent_encoded() -> crate::Result {
    Ok(assert_url_roundtrip(
        "http://user.name@example.com/repo",
        url(Scheme::Http, "user.name", "example.com", None, b"/repo"),
    )?)
}

#[test]
fn password_with_dot_is_not_percent_encoded() -> crate::Result {
    let password = test_password(true);
    Ok(assert_url_roundtrip(
        &format!("http://user:{password}@example.com/repo"),
        url_with_pass(Scheme::Http, "user", password, "example.com", None, b"/repo"),
    )?)
}

#[test]
fn username_and_password_with_dots_are_not_percent_encoded() -> crate::Result {
    let password = test_password(true);
    Ok(assert_url_roundtrip(
        &format!("http://user.name:{password}@example.com/repo"),
        url_with_pass(Scheme::Http, "user.name", password, "example.com", None, b"/repo"),
    )?)
}

#[test]
fn http_with_ipv6() -> crate::Result {
    Ok(assert_url_roundtrip(
        "http://[::1]/repo",
        url(Scheme::Http, None, "[::1]", None, b"/repo"),
    )?)
}

#[test]
fn http_with_ipv6_and_port() -> crate::Result {
    Ok(assert_url_roundtrip(
        "http://[::1]:8080/repo",
        url(Scheme::Http, None, "[::1]", 8080, b"/repo"),
    )?)
}

#[test]
fn https_with_ipv6_user_and_port() -> crate::Result {
    Ok(assert_url_roundtrip(
        "https://user@[2001:db8::1]:8443/repo",
        url(Scheme::Https, "user", "[2001:db8::1]", 8443, b"/repo"),
    )?)
}

#[test]
fn percent_encoded_path() -> crate::Result {
    let url = gix_url::parse("https://example.com/path/with%20spaces/file")?;
    assert_eq!(url.path, "/path/with spaces/file", "paths are now decoded");
    assert_eq!(
        url.original_path(),
        "/path/with%20spaces/file",
        "the encoded request path remains available"
    );
    Ok(())
}

#[test]
fn percent_encoded_international_path() -> crate::Result {
    let url = gix_url::parse("https://example.com/caf%C3%A9")?;
    assert_eq!(url.path, "/café", "international characters are decoded in path");
    Ok(())
}

#[test]
fn original_path_preserves_exact_spelling() -> crate::Result {
    for (input, decoded_path, original_path, message) in [
        (
            "https://example.com/plain",
            "/plain",
            "/plain",
            "a path without escapes falls back to the decoded path",
        ),
        (
            "https://example.com/%41",
            "/A",
            "/%41",
            "an unreserved escape retains its spelling",
        ),
        (
            "https://example.com/%2f",
            "//",
            "/%2f",
            "lowercase escape digits retain their case",
        ),
        (
            "https://example.com/caf%C3%A9",
            "/café",
            "/caf%C3%A9",
            "a multi-byte escape retains its complete spelling",
        ),
    ] {
        let url = gix_url::parse(input)?;
        assert_eq!(url.path, decoded_path, "the public path is decoded: {message}");
        assert_eq!(url.original_path(), original_path, "{message}");
        assert_eq!(url.to_bstring(), input, "serialization remains lossless: {message}");
    }
    Ok(())
}

#[test]
fn reserved_percent_encoded_path_octets_remain_lossless() -> crate::Result {
    for (input, expected_path, message) in [
        (
            "https://example.com/a%2Fb",
            "/a/b",
            "an encoded slash remains path data",
        ),
        (
            "https://example.com/%3Fquery",
            "/?query",
            "an encoded question mark does not start a query",
        ),
        (
            "https://example.com/%23fragment",
            "/#fragment",
            "an encoded hash does not start a fragment",
        ),
        (
            "https://example.com/%252F",
            "/%2F",
            "an encoded percent sign remains lossless",
        ),
    ] {
        let url = gix_url::parse(input)?;
        assert_eq!(url.to_bstring(), input, "{message}");
        assert_eq!(url.path, expected_path, "the public path is decoded: {message}");
    }
    Ok(())
}

#[test]
fn literal_percent_escape_text_from_parts_is_encoded() -> crate::Result {
    let url = gix_url::Url::from_parts(
        Scheme::Https,
        None,
        None,
        Some("example.com".into()),
        None,
        "/foo%2Fbar".into(),
        false,
    )?;
    assert_eq!(url.path, "/foo%2Fbar", "the decoded path remains unchanged");
    assert_eq!(
        url.to_bstring(),
        "https://example.com/foo%252Fbar",
        "literal percent text is encoded"
    );
    let empty = gix_url::Url::from_parts(
        Scheme::Https,
        None,
        None,
        Some("example.com".into()),
        None,
        "".into(),
        false,
    )?;
    assert_eq!(empty.path, "/", "an empty HTTP path is normalized");
    assert_eq!(empty.to_bstring(), "https://example.com/");

    let mut parsed = gix_url::parse("https://example.com/a%2Fb")?;
    parsed.path = "/literal%2Ftext".into();
    assert_eq!(
        parsed.original_path(),
        "/literal%2Ftext",
        "mutating the path invalidates its encoded spelling"
    );
    assert_eq!(
        parsed.to_bstring(),
        "https://example.com/literal%252Ftext",
        "mutating a parsed path invalidates preserved escapes"
    );
    Ok(())
}

#[test]
fn percent_encoded_path_roundtrips_in_lossless_serialization() -> crate::Result {
    for (input, message, expected_host, expected_path) in [
        (
            "https://%20@%40.example.org/%20%25",
            "a single percent-encoded path segment roundtrips losslessly",
            "%40.example.org",
            "/ %",
        ),
        (
            "https://%20@%40.example.org/%20%25/%20%25",
            "multiple percent-encoded path segments roundtrip losslessly",
            "%40.example.org",
            "/ %/ %",
        ),
    ] {
        let url = gix_url::parse(input)?;
        let serialized = url.to_bstring();
        assert_eq!(serialized, input, "{message}");
        assert_eq!(url.host(), Some(expected_host), "{message}");
        assert_eq!(url.path, expected_path, "{message}");
        assert_eq!(gix_url::parse(&serialized)?, url, "{message}");
    }
    Ok(())
}

#[test]
fn query_and_fragment_delimiters_in_path_roundtrip() -> crate::Result {
    assert_url_roundtrip(
        "https://host/repo.git?token=abc",
        url(Scheme::Https, None, "host", None, b"/repo.git?token=abc"),
    )?;
    assert_url_roundtrip(
        "https://host/repo.git#section",
        url(Scheme::Https, None, "host", None, b"/repo.git#section"),
    )?;
    Ok(())
}

#[test]
fn query_and_fragment_delimiters_end_the_authority() -> crate::Result {
    for input in ["https://host?@redirected/repo", "https://host#@redirected/repo"] {
        let url = gix_url::parse(input)?;
        assert_eq!(url.host(), Some("host"), "the authority ends at the delimiter");
        assert_eq!(
            &url.path,
            &input["https://host".len()..],
            "the remainder is kept in the path"
        );
    }
    for delimiter in ['?', '#'] {
        let input = format!("https://host{delimiter}{}", "x".repeat(1025));
        assert_eq!(
            gix_url::parse(input)?.host(),
            Some("host"),
            "the remainder does not count toward the authority length"
        );
    }
    Ok(())
}

#[test]
fn authority_length_limit_excludes_the_scheme_separator() -> crate::Result {
    let at_limit = format!("https://{}", "a".repeat(1024));
    assert_eq!(
        gix_url::parse(&at_limit)?.host().map(str::len),
        Some(1024),
        "the full authority limit is accepted"
    );
    let over_limit = format!("https://{}", "a".repeat(1025));
    assert!(
        gix_url::parse(over_limit)
            .unwrap_err()
            .message
            .contains("host portion of the URL is too long"),
        "one byte beyond the authority limit is rejected"
    );
    Ok(())
}
