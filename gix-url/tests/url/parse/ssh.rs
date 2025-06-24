use gix_url::Scheme;

use crate::parse::{assert_url, assert_url_roundtrip, url, url_alternate};

#[test]
fn without_user_and_without_port() -> crate::Result {
    assert_url_roundtrip(
        "ssh://host.xz/path/to/repo.git/",
        url(Scheme::Ssh, None, "host.xz", None, b"/path/to/repo.git/"),
    )
}

#[test]
fn without_user_and_with_port() -> crate::Result {
    assert_url_roundtrip("ssh://host.xz:21/", url(Scheme::Ssh, None, "host.xz", 21, b"/"))
}

#[test]
fn host_is_ipv4() -> crate::Result {
    assert_url_roundtrip(
        "ssh://127.69.0.1/hello",
        url(Scheme::Ssh, None, "127.69.0.1", None, b"/hello"),
    )
}

#[test]
fn username_expansion_with_username() -> crate::Result {
    assert_url_roundtrip(
        "ssh://example.com/~byron/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"/~byron/hello/git"),
    )
}

#[test]
fn username_expansion_without_username() -> crate::Result {
    assert_url_roundtrip(
        "ssh://example.com/~/hello/git",
        url(Scheme::Ssh, None, "example.com", None, b"/~/hello/git"),
    )
}

#[test]
fn scp_like_with_ssh_host_alias() -> crate::Result {
    assert_url_roundtrip(
        "user@alias:username/repo.git",
        url_alternate(Scheme::Ssh, "user", "alias", None, b"username/repo.git"),
    )
}

#[test]
fn with_user_and_without_port() -> crate::Result {
    assert_url_roundtrip(
        "ssh://user@host.xz/.git",
        url(Scheme::Ssh, "user", "host.xz", None, b"/.git"),
    )
}

#[test]
fn with_user_and_port_and_absolute_path() -> crate::Result {
    assert_url_roundtrip(
        "ssh://user@host.xz:42/.git",
        url(Scheme::Ssh, "user", "host.xz", 42, b"/.git"),
    )
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
    let url = gix_url::parse("user@host.xz:42:C:/strange/absolute/path".into())?;
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
fn scp_like_with_underscore_in_username() -> crate::Result {
    let url = assert_url(
        "_git@source.example.tld:org/repo.git",
        url_alternate(Scheme::Ssh, "_git", "source.example.tld", None, b"org/repo.git"),
    )?;

    let serialized = url.to_bstring();
    assert_eq!(
        serialized, "_git@source.example.tld:org/repo.git",
        "SCP-like URLs should not percent-encode underscores in usernames"
    );
    Ok(())
}

#[test]
fn ssh_url_with_underscore_in_username_still_works() -> crate::Result {
    assert_url_roundtrip(
        "ssh://_git@source.example.tld/org/repo.git",
        url(Scheme::Ssh, "_git", "source.example.tld", None, b"/org/repo.git"),
    )
}

#[test]
fn scp_like_with_special_characters_in_username() -> crate::Result {
    let input = "user@with@problem@host.com:repo.git";
    let parsed = gix_url::parse(input.as_ref())?;

    // The @ in username should be encoded when serialized
    let serialized = parsed.to_bstring();
    assert!(
        serialized.to_string().contains("user%40with%40problem"),
        "@ characters in username should be percent-encoded, got: {serialized}"
    );
    Ok(())
}
