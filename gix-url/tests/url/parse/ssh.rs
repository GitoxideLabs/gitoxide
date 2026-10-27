use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url, url_alternate, url_with_pass};

fn assert_ipv6_url_roundtrip(input: &str, expected: gix_url::Url, bare_host: &str) -> gix_testtools::TestResult {
    let actual = gix_url::parse(input)?;
    assert_eq!(
        actual, expected,
        "SSH IPv6 components retain their bracketed host: {input}"
    );
    assert_eq!(
        actual.host_unbracketed(),
        Some(bare_host),
        "the bare-host accessor removes exactly one enclosing bracket pair: {input}"
    );
    assert_eq!(
        actual.to_bstring(),
        input,
        "serialization retains exactly one IPv6 bracket pair: {input}"
    );
    assert_eq!(
        gix_url::parse(actual.to_bstring())?,
        actual,
        "the bracketed SSH host survives reparsing: {input}"
    );
    Ok(())
}

#[test]
fn without_user_and_without_port() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://host.xz/path/to/repo.git/",
        url(Scheme::Ssh, None, "host.xz", None, b"/path/to/repo.git/"),
    )?)
}

#[test]
fn without_user_and_with_port() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://host.xz:21/",
        url(Scheme::Ssh, None, "host.xz", 21, b"/"),
    )?)
}

#[test]
fn host_is_ipv4() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://127.69.0.1/hello",
        url(Scheme::Ssh, None, "127.69.0.1", None, b"/hello"),
    )?)
}

#[test]
fn username_expansion_with_username() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://example.com/~byron/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"~byron/hello/git"),
    )?)
}

#[test]
fn username_expansion_without_username() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://example.com/~/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"~/hello/git"),
    )?)
}

#[test]
fn scp_like_with_ssh_host_alias() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "user@alias:username/repo.git",
        url_alternate(Scheme::Ssh, "user", "alias", None, b"username/repo.git"),
    )?)
}

#[test]
fn with_user_and_without_port() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://user@host.xz/.git",
        url(Scheme::Ssh, "user", "host.xz", None, b"/.git"),
    )?)
}

#[test]
fn username_with_dot_is_not_percent_encoded() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://user.name@host.xz/.git",
        url(Scheme::Ssh, "user.name", "host.xz", None, b"/.git"),
    )?)
}

#[test]
fn with_user_and_port_and_absolute_path() -> gix_testtools::TestResult {
    Ok(assert_url_roundtrip(
        "ssh://user@host.xz:42/.git",
        url(Scheme::Ssh, "user", "host.xz", 42, b"/.git"),
    )?)
}

#[test]
fn ssh_alias_without_username() -> gix_testtools::TestResult {
    let url = assert_url(
        "host:/path/to/git",
        url_alternate(Scheme::Ssh, None, "host", None, b"/path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host:/path/to/git");
    Ok(())
}

#[test]
fn default_port_is_22() -> gix_testtools::TestResult {
    let url = url_alternate(Scheme::Ssh, None, "host.xz", None, b"path/to/git");
    assert_eq!(url.port_or_default(), Some(22));
    Ok(())
}

#[test]
fn scp_like_without_user() -> gix_testtools::TestResult {
    let url = assert_url(
        "host.xz:path/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:path/to/git");
    Ok(())
}

#[test]
fn scp_like_with_absolute_path() -> gix_testtools::TestResult {
    let url = assert_url(
        "host.xz:/path/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"/path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:/path/to/git");
    Ok(())
}

#[test]
fn scp_like_with_absolute_path_with_whitespace() -> gix_testtools::TestResult {
    let url = assert_url(
        "host.xz:/path/to/git with space",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"/path/to/git with space"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:/path/to/git with space");
    Ok(())
}

#[test]
fn scp_like_without_user_and_username_expansion_without_username() -> gix_testtools::TestResult {
    let url = assert_url(
        "host.xz:~/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"~/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:~/to/git");
    Ok(())
}

#[test]
fn scp_like_without_user_and_username_expansion_with_username() -> gix_testtools::TestResult {
    let url = assert_url(
        "host.xz:~byron/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"~byron/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:~byron/to/git");
    Ok(())
}

#[test]
fn scp_like_with_user_and_relative_path_keep_relative_path() -> gix_testtools::TestResult {
    let url = assert_url(
        "user@host.xz:relative",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"relative"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:relative");

    let url = assert_url(
        "user@host.xz:./relative",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"./relative"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:./relative", "./ is maintained");

    let url = assert_url(
        "user@host.xz:././relative",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"././relative"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:././relative", "./ is maintained, even if repeated");

    let url = assert_url(
        "user@host.xz:../relative",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"../relative"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:../relative");

    let url = assert_url(
        "user@host.xz:../relative with space",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"../relative with space"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:../relative with space");
    Ok(())
}

#[test]
fn canonical_form_is_used_only_if_it_preserves_scp_path_semantics() -> gix_testtools::TestResult {
    assert_eq!(
        gix_url::parse("user@host.xz:relative")?
            .with_request_alternate_form(false)
            .to_bstring(),
        "user@host.xz:relative",
        "canonical form would turn the relative path into an absolute one"
    );
    assert_eq!(
        gix_url::parse("user@host.xz:/absolute")?
            .with_request_alternate_form(false)
            .to_bstring(),
        "ssh://user@host.xz/absolute",
        "an absolute path has a lossless canonical form"
    );
    assert_eq!(
        gix_url::parse("user@host.xz:~user/repo")?
            .with_request_alternate_form(false)
            .to_bstring(),
        "ssh://user@host.xz/~user/repo",
        "Git treats this canonical spelling as the same home-relative path"
    );
    for request_alternative_form in [false, true] {
        assert!(
            url(Scheme::Ssh, None, "host.xz", 22, b"relative")
                .with_request_alternate_form(request_alternative_form)
                .write_to(&mut Vec::new())
                .is_err(),
            "a port prevents using lossless SCP-like form"
        );
    }
    Ok(())
}

#[test]
fn scp_like_with_windows_path() -> gix_testtools::TestResult {
    let url = assert_url(
        "user@host.xz:C:/strange/absolute/path",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"C:/strange/absolute/path"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:C:/strange/absolute/path");
    Ok(())
}

#[test]
fn scp_like_with_windows_path_and_port_thinks_port_is_part_of_path() -> gix_testtools::TestResult {
    let url = gix_url::parse("user@host.xz:42:C:/strange/absolute/path")?;
    assert_eq!(
        url.to_bstring(),
        "user@host.xz:42:C:/strange/absolute/path",
        "it reproduces correctly"
    );
    assert_eq!(
        url.path, "42:C:/strange/absolute/path",
        "but in fact it gets it quite wrong - git does the same on windows and linux"
    );
    assert_eq!(url.port, None, "the port wasn't actually parsed");
    Ok(())
}

#[test]
fn scp_like_with_non_alphanumeric_username() -> gix_testtools::TestResult {
    let url = assert_url(
        "_user.name@host.xz:C:/path",
        url_alternate(Scheme::Ssh, "_user.name", "host.xz", None, b"C:/path"),
    )?
    .to_bstring();
    assert_eq!(url, "_user.name@host.xz:C:/path");
    Ok(())
}

// Git passes the non-path part "user@name@host.xz" to OpenSSH, and the ssh
// command interprets it as user = "user@name", host = "host.xz".
#[test]
fn scp_like_with_username_including_at() -> gix_testtools::TestResult {
    let url = assert_url(
        "user@name@host.xz:path",
        url_alternate(Scheme::Ssh, "user@name", "host.xz", None, b"path"),
    )?
    .to_bstring();
    assert_eq!(url, "user@name@host.xz:path");
    Ok(())
}

#[test]
fn scp_like_username_with_colon_is_not_mistaken_for_a_password() -> gix_testtools::TestResult {
    let input = "[:[@]:\x1a";
    let url = gix_url::parse(input)?;
    assert_eq!(url.user(), Some("[:["), "OpenSSH splits user and host at the last @");
    assert_eq!(url.host(), Some("]"), "the remainder is the host");
    assert_eq!(url.password(), None, "SCP-like syntax has no password field");
    assert_eq!(url.to_bstring(), input, "successfully parsed inputs must serialize");
    Ok(())
}

// Git does not care that the host is named `file`, it still treats it as an SCP url.
// I btw tested this, yes you can really clone a repository from there, just `git init`
// in the directory above your home directory on the remote machine.
#[test]
fn strange_scp_like_with_host_named_file() -> gix_testtools::TestResult {
    let url = assert_url("file:..", url_alternate(Scheme::Ssh, None, "file", None, b".."))?;
    assert_eq!(url.to_bstring(), "file:..");
    Ok(())
}

#[test]
fn bad_alternative_form_with_password() -> gix_testtools::TestResult {
    let password = std::process::id().to_string();
    let url = url_with_pass(Scheme::Ssh, "user", password.as_str(), "host.xz", None, b"/")
        .with_request_alternate_form(true)
        .to_bstring();
    assert_eq!(url, format!("ssh://user:{password}@host.xz/"));
    Ok(())
}

#[test]
fn bad_alternative_form_with_port() -> gix_testtools::TestResult {
    let url = url_alternate(Scheme::Ssh, None, "host.xz", 21, b"/").to_bstring();
    assert_eq!(url, "ssh://host.xz:21/");
    Ok(())
}

#[test]
fn ipv6_address_without_port() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "ssh://[::1]/repo",
        url(Scheme::Ssh, None, "[::1]", None, b"/repo"),
        "::1",
    )
}

#[test]
fn ipv6_address_with_port() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "ssh://[::1]:22/repo",
        url(Scheme::Ssh, None, "[::1]", 22, b"/repo"),
        "::1",
    )
}

#[test]
fn ipv6_address_with_user() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "ssh://user@[::1]/repo",
        url(Scheme::Ssh, "user", "[::1]", None, b"/repo"),
        "::1",
    )
}

#[test]
fn ipv6_address_with_user_and_port() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "ssh://user@[::1]:22/repo",
        url(Scheme::Ssh, "user", "[::1]", 22, b"/repo"),
        "::1",
    )
}

#[test]
fn ipv6_full_address() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "ssh://[2001:db8::1]/repo",
        url(Scheme::Ssh, None, "[2001:db8::1]", None, b"/repo"),
        "2001:db8::1",
    )
}

#[test]
fn scoped_ipv6_address() -> gix_testtools::TestResult {
    for (input, bare_host, serialized) in [
        (
            "ssh://[fe80::1%25Eth0]/repo",
            "fe80::1%Eth0",
            "ssh://[fe80::1%25Eth0]/repo",
        ),
        (
            "ssh://[fe80::1%25E%74h0]/repo",
            "fe80::1%Eth0",
            "ssh://[fe80::1%25Eth0]/repo",
        ),
        (
            "ssh://[fe80::1%25Eth%2530]/repo",
            "fe80::1%Eth%30",
            "ssh://[fe80::1%25Eth%2530]/repo",
        ),
    ] {
        let url = gix_url::parse(input)?;
        let bracketed_host = format!("[{bare_host}]");
        assert_eq!(
            url.host(),
            Some(bracketed_host.as_str()),
            "the zone identifier is decoded once without losing brackets or case: {input}"
        );
        assert_eq!(
            url.host_unbracketed(),
            Some(bare_host),
            "the bare-host accessor retains the decoded zone identifier: {input}"
        );
        assert_eq!(
            url.to_bstring(),
            serialized,
            "serialization percent-encodes the scope separator exactly once: {input}"
        );
        assert_eq!(
            gix_url::parse(url.to_bstring())?,
            url,
            "the scoped address roundtrips with brackets and decoded host data: {input}"
        );
    }
    Ok(())
}

#[test]
fn scp_like_scoped_ipv6_address_uses_a_raw_zone_separator() -> gix_testtools::TestResult {
    for (input, host, bare_host) in [
        ("[fe80::1%Eth0]:repo", "[fe80::1%Eth0]", "fe80::1%Eth0"),
        ("[fe80::1%Eth%30]:repo", "[fe80::1%Eth%30]", "fe80::1%Eth%30"),
    ] {
        assert_ipv6_url_roundtrip(input, url_alternate(Scheme::Ssh, None, host, None, b"repo"), bare_host)?;
    }
    Ok(())
}

#[test]
fn scoped_ipv6_address_with_empty_port() -> gix_testtools::TestResult {
    let url = gix_url::parse("ssh://[fe80::1%25Eth0]:/repo")?;
    assert_eq!(
        url.host(),
        Some("[fe80::1%Eth0]"),
        "the empty port is removed without stripping IPv6 brackets or changing the decoded scope"
    );
    assert_eq!(
        url.host_unbracketed(),
        Some("fe80::1%Eth0"),
        "the bare-host accessor strips brackets after empty-port normalization"
    );
    assert_eq!(url.port, None, "an empty port is not represented");
    assert_eq!(
        url.to_bstring(),
        "ssh://[fe80::1%25Eth0]/repo",
        "serialization omits the empty port and encodes the scope separator once"
    );
    assert_eq!(
        gix_url::parse(url.to_bstring())?,
        url,
        "the normalized scoped URL reparses unchanged"
    );
    Ok(())
}

#[test]
fn empty_port_is_removed_without_changing_the_host() -> gix_testtools::TestResult {
    for (input, host, bare_host, serialized) in [
        ("ssh://[::1]:/repo", "[::1]", "::1", "ssh://[::1]/repo"),
        ("ssh://host:/repo", "host", "host", "ssh://host/repo"),
    ] {
        let url = gix_url::parse(input)?;
        assert_eq!(
            url.host(),
            Some(host),
            "empty-port normalization preserves the host: {input}"
        );
        assert_eq!(
            url.host_unbracketed(),
            Some(bare_host),
            "the bare-host accessor excludes brackets and the empty port: {input}"
        );
        assert_eq!(url.port, None, "an empty port is not represented: {input}");
        assert_eq!(
            url.to_bstring(),
            serialized,
            "serialization omits the empty port: {input}"
        );
        assert_eq!(
            gix_url::parse(url.to_bstring())?,
            url,
            "the normalized URL reparses unchanged: {input}"
        );
    }
    Ok(())
}

#[test]
fn bracketed_host_is_percent_decoded_once() -> gix_testtools::TestResult {
    let url = gix_url::parse("ssh://[::81ssssssssssssssssssssssssssssssssssssss%2585]:/00%2585]://")?;
    assert_eq!(
        url.host(),
        Some("[::81ssssssssssssssssssssssssssssssssssssss%85]"),
        "an escape produced by decoding must not be decoded again, and brackets remain host data"
    );
    assert_eq!(
        url.host_unbracketed(),
        Some("::81ssssssssssssssssssssssssssssssssssssss%85"),
        "stripping brackets must not decode host data again"
    );
    assert_eq!(url.path, "/00%85]://", "the path is decoded once as well");
    assert_eq!(
        gix_url::parse(url.to_bstring())?,
        url,
        "the decoded host and path survive reparsing"
    );
    Ok(())
}

#[test]
fn escaped_authority_delimiters_remain_host_data() -> gix_testtools::TestResult {
    for (input, expected_host) in [
        ("ssh://host%3A123/repo", "host:123"),
        ("ssh://host%2Fname/repo", "host/name"),
    ] {
        let url = gix_url::parse(input)?;
        assert_eq!(
            url.host(),
            Some(expected_host),
            "the escaped delimiter is host data: {input}"
        );
        assert_eq!(
            url.to_bstring(),
            input,
            "serialization re-escapes the delimiter: {input}"
        );
        assert_eq!(
            gix_url::parse(url.to_bstring())?,
            url,
            "the URL reparses unchanged: {input}"
        );
    }
    Ok(())
}

#[test]
fn bracketed_host_alternate_form_falls_back_to_canonical_for_decoded_delimiters() -> gix_testtools::TestResult {
    for (encoded_host, host) in [
        ("host%5D", "[host]]"),
        ("host%2F", "[host/]"),
        ("host%40", "[host@]"),
        ("host%20", "[host ]"),
    ] {
        for path in ["repo", "~user/repo"] {
            let input = format!("ssh://[{encoded_host}]/{path}");
            let url = gix_url::parse(input.as_str())?.with_request_alternate_form(true);
            assert_eq!(
                url.host(),
                Some(host),
                "decoded delimiters remain inside the retained brackets: {input}"
            );
            let serialized = url.to_bstring();
            assert_eq!(
                serialized, input,
                "unrepresentable SCP hosts fall back to canonical form without changing path meaning"
            );
            assert_eq!(
                gix_url::parse(serialized)?,
                url.with_request_alternate_form(false),
                "canonical fallback roundtrips the bracketed host and repository path: {input}"
            );
        }
    }
    Ok(())
}

#[test]
fn bracketed_host_alternate_form_rejects_relative_paths_with_decoded_delimiters() -> gix_testtools::TestResult {
    for host in ["[host]]", "[host/]", "[host@]", "[host ]"] {
        for request_alternate_form in [false, true] {
            let mut url = gix_url::parse("ssh://host/repo")?.with_request_alternate_form(request_alternate_form);
            url.host = Some(host.to_owned());
            url.path = "relative".into();
            assert!(
                url.write_to(&mut Vec::new()).is_err(),
                "SCP cannot represent host {host:?}, and canonical fallback would make the mutated path absolute"
            );
        }
    }
    Ok(())
}

#[test]
fn bracketed_host_alternate_form_preserves_representable_hosts() -> gix_testtools::TestResult {
    for (input, alternate) in [
        ("ssh://[::1]/repo", "[::1]:/repo"),
        ("ssh://[fe80::1%25Eth0]/repo", "[fe80::1%Eth0]:/repo"),
    ] {
        let url = gix_url::parse(input)?.with_request_alternate_form(true);
        let serialized = url.to_bstring();
        assert_eq!(
            serialized, alternate,
            "representable bracketed hosts still use SCP form with a raw scope separator: {input}"
        );
        assert_eq!(
            gix_url::parse(serialized)?,
            url,
            "SCP serialization preserves the retained brackets and decoded scope: {input}"
        );
    }
    Ok(())
}

#[test]
fn percent_encoded_paths_are_decoded_and_the_original_is_retained() -> gix_testtools::TestResult {
    for (input, decoded_path, original_path, message) in [
        (
            "ssh://example.com/a%2Fb",
            "/a/b",
            "/a%2Fb",
            "Git decodes reserved escapes in SSH repository paths",
        ),
        (
            "ssh://example.com/a%20b",
            "/a b",
            "/a%20b",
            "original paths are available independently of the URL scheme",
        ),
    ] {
        let url = gix_url::parse(input)?;
        assert_eq!(url.path, decoded_path, "{message}");
        assert_eq!(url.original_path(), original_path, "{message}");
        assert_eq!(url.to_bstring(), input, "serialization remains lossless: {message}");
    }
    Ok(())
}

#[test]
fn ipv6_address_scp_like() -> gix_testtools::TestResult {
    assert_ipv6_url_roundtrip(
        "[::1]:repo",
        url_alternate(Scheme::Ssh, None, "[::1]", None, b"repo"),
        "::1",
    )
}

#[test]
fn ipv6_address_scp_like_with_user() -> gix_testtools::TestResult {
    let result = gix_url::parse("user@[::1]:repo");
    assert!(
        result.is_err(),
        "SCP-like format with brackets is not supported - Git doesn't support this either"
    );
    Ok(())
}
