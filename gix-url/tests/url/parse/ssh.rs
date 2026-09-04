use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url, url_alternate, url_with_pass};

#[test]
fn without_user_and_without_port() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://host.xz/path/to/repo.git/",
        url(Scheme::Ssh, None, "host.xz", None, b"/path/to/repo.git/"),
    )?)
}

#[test]
fn without_user_and_with_port() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://host.xz:21/",
        url(Scheme::Ssh, None, "host.xz", 21, b"/"),
    )?)
}

#[test]
fn host_is_ipv4() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://127.69.0.1/hello",
        url(Scheme::Ssh, None, "127.69.0.1", None, b"/hello"),
    )?)
}

#[test]
fn username_expansion_with_username() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://example.com/~byron/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"~byron/hello/git"),
    )?)
}

#[test]
fn username_expansion_without_username() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://example.com/~/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"~/hello/git"),
    )?)
}

#[test]
fn scp_like_with_ssh_host_alias() -> crate::Result {
    Ok(assert_url_roundtrip(
        "user@alias:username/repo.git",
        url_alternate(Scheme::Ssh, "user", "alias", None, b"username/repo.git"),
    )?)
}

#[test]
fn with_user_and_without_port() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://user@host.xz/.git",
        url(Scheme::Ssh, "user", "host.xz", None, b"/.git"),
    )?)
}

#[test]
fn username_with_dot_is_not_percent_encoded() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://user.name@host.xz/.git",
        url(Scheme::Ssh, "user.name", "host.xz", None, b"/.git"),
    )?)
}

#[test]
fn with_user_and_port_and_absolute_path() -> crate::Result {
    Ok(assert_url_roundtrip(
        "ssh://user@host.xz:42/.git",
        url(Scheme::Ssh, "user", "host.xz", 42, b"/.git"),
    )?)
}

#[test]
fn ssh_alias_without_username() -> crate::Result {
    let url = assert_url(
        "host:/path/to/git",
        url_alternate(Scheme::Ssh, None, "host", None, b"/path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host:/path/to/git");
    Ok(())
}

#[test]
fn default_port_is_22() -> crate::Result {
    let url = url_alternate(Scheme::Ssh, None, "host.xz", None, b"path/to/git");
    assert_eq!(url.port_or_default(), Some(22));
    Ok(())
}

#[test]
fn scp_like_without_user() -> crate::Result {
    let url = assert_url(
        "host.xz:path/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:path/to/git");
    Ok(())
}

#[test]
fn scp_like_with_absolute_path() -> crate::Result {
    let url = assert_url(
        "host.xz:/path/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"/path/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:/path/to/git");
    Ok(())
}

#[test]
fn scp_like_with_absolute_path_with_whitespace() -> crate::Result {
    let url = assert_url(
        "host.xz:/path/to/git with space",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"/path/to/git with space"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:/path/to/git with space");
    Ok(())
}

#[test]
fn scp_like_without_user_and_username_expansion_without_username() -> crate::Result {
    let url = assert_url(
        "host.xz:~/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"~/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:~/to/git");
    Ok(())
}

#[test]
fn scp_like_without_user_and_username_expansion_with_username() -> crate::Result {
    let url = assert_url(
        "host.xz:~byron/to/git",
        url_alternate(Scheme::Ssh, None, "host.xz", None, b"~byron/to/git"),
    )?
    .to_bstring();
    assert_eq!(url, "host.xz:~byron/to/git");
    Ok(())
}

#[test]
fn scp_like_with_user_and_relative_path_keep_relative_path() -> crate::Result {
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
fn canonical_form_is_used_only_if_it_preserves_scp_path_semantics() -> crate::Result {
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
fn scp_like_with_windows_path() -> crate::Result {
    let url = assert_url(
        "user@host.xz:C:/strange/absolute/path",
        url_alternate(Scheme::Ssh, "user", "host.xz", None, b"C:/strange/absolute/path"),
    )?
    .to_bstring();
    assert_eq!(url, "user@host.xz:C:/strange/absolute/path");
    Ok(())
}

#[test]
fn scp_like_with_windows_path_and_port_thinks_port_is_part_of_path() -> crate::Result {
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
fn scp_like_with_non_alphanumeric_username() -> crate::Result {
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
fn scp_like_with_username_including_at() -> crate::Result {
    let url = assert_url(
        "user@name@host.xz:path",
        url_alternate(Scheme::Ssh, "user@name", "host.xz", None, b"path"),
    )?
    .to_bstring();
    assert_eq!(url, "user@name@host.xz:path");
    Ok(())
}

#[test]
fn scp_like_username_with_colon_is_not_mistaken_for_a_password() -> crate::Result {
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
fn strange_scp_like_with_host_named_file() -> crate::Result {
    let url = assert_url("file:..", url_alternate(Scheme::Ssh, None, "file", None, b".."))?;
    assert_eq!(url.to_bstring(), "file:..");
    Ok(())
}

#[test]
fn bad_alternative_form_with_password() -> crate::Result {
    let password = std::process::id().to_string();
    let url = url_with_pass(Scheme::Ssh, "user", password.as_str(), "host.xz", None, b"/")
        .with_request_alternate_form(true)
        .to_bstring();
    assert_eq!(url, format!("ssh://user:{password}@host.xz/"));
    Ok(())
}

#[test]
fn bad_alternative_form_with_port() -> crate::Result {
    let url = url_alternate(Scheme::Ssh, None, "host.xz", 21, b"/").to_bstring();
    assert_eq!(url, "ssh://host.xz:21/");
    Ok(())
}

#[test]
fn ipv6_address_without_port() -> crate::Result {
    let url = assert_url("ssh://[::1]/repo", url(Scheme::Ssh, None, "::1", None, b"/repo"))?;
    assert_eq!(url.host(), Some("::1"), "brackets are stripped for SSH");
    Ok(())
}

#[test]
fn ipv6_address_with_port() -> crate::Result {
    let url = assert_url("ssh://[::1]:22/repo", url(Scheme::Ssh, None, "::1", 22, b"/repo"))?;
    assert_eq!(url.host(), Some("::1"));
    assert_eq!(url.port, Some(22));
    Ok(())
}

#[test]
fn ipv6_address_with_user() -> crate::Result {
    let url = assert_url("ssh://user@[::1]/repo", url(Scheme::Ssh, "user", "::1", None, b"/repo"))?;
    assert_eq!(url.host(), Some("::1"));
    assert_eq!(url.user(), Some("user"));
    Ok(())
}

#[test]
fn ipv6_address_with_user_and_port() -> crate::Result {
    let url = assert_url(
        "ssh://user@[::1]:22/repo",
        url(Scheme::Ssh, "user", "::1", 22, b"/repo"),
    )?;
    assert_eq!(url.host(), Some("::1"));
    assert_eq!(url.user(), Some("user"));
    assert_eq!(url.port, Some(22));
    Ok(())
}

#[test]
fn ipv6_full_address() -> crate::Result {
    let url = assert_url(
        "ssh://[2001:db8::1]/repo",
        url(Scheme::Ssh, None, "2001:db8::1", None, b"/repo"),
    )?;
    assert_eq!(url.host(), Some("2001:db8::1"));
    Ok(())
}

#[test]
fn scoped_ipv6_address() -> crate::Result {
    let input = "ssh://[fe80::1%25Eth0]/repo";
    let url = gix_url::parse(input)?;
    assert_eq!(
        url.host(),
        Some("fe80::1%Eth0"),
        "the zone identifier is decoded for SSH"
    );
    assert_eq!(url.to_bstring(), input, "the URI spelling remains encoded");
    assert_eq!(gix_url::parse(url.to_bstring())?, url, "the scoped address roundtrips");
    Ok(())
}

#[test]
fn scp_like_scoped_ipv6_address_uses_a_raw_zone_separator() -> crate::Result {
    let input = "[fe80::1%Eth0]:repo";
    let url = gix_url::parse(input)?;
    assert_eq!(url.host(), Some("fe80::1%Eth0"), "the raw percent sign is host data");
    assert_eq!(
        url.to_bstring(),
        input,
        "alternative serialization retains the raw separator"
    );
    assert_eq!(gix_url::parse(url.to_bstring())?, url, "the alternative form reparses");
    Ok(())
}

#[test]
fn scoped_ipv6_address_with_empty_port() -> crate::Result {
    let url = gix_url::parse("ssh://[fe80::1%25Eth0]:/repo")?;
    assert_eq!(
        url.host(),
        Some("fe80::1%Eth0"),
        "the bracketed host with an empty port is decoded"
    );
    assert_eq!(url.port, None, "an empty port is not represented");
    assert_eq!(
        url.to_bstring(),
        "ssh://[fe80::1%25Eth0]/repo",
        "serialization omits the empty port"
    );
    Ok(())
}

#[test]
fn bracketed_host_is_percent_decoded_once() -> crate::Result {
    let url = gix_url::parse("ssh://[::81ssssssssssssssssssssssssssssssssssssss%2585]:/00%2585]://")?;
    assert_eq!(
        url.host(),
        Some("::81ssssssssssssssssssssssssssssssssssssss%85"),
        "an escape produced by decoding must not be decoded again"
    );
    assert_eq!(url.path, "/00%85]://", "the path is decoded once as well");
    Ok(())
}

#[test]
fn escaped_authority_delimiters_remain_host_data() -> crate::Result {
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
fn percent_encoded_paths_are_decoded_and_the_original_is_retained() -> crate::Result {
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
fn ipv6_address_scp_like() -> crate::Result {
    let url = assert_url("[::1]:repo", url_alternate(Scheme::Ssh, None, "::1", None, b"repo"))?;
    assert_eq!(url.host(), Some("::1"), "SCP-like format with IPv6");
    Ok(())
}

#[test]
fn ipv6_address_scp_like_with_user() -> crate::Result {
    let result = gix_url::parse("user@[::1]:repo");
    assert!(
        result.is_err(),
        "SCP-like format with brackets is not supported - Git doesn't support this either"
    );
    Ok(())
}
