mod canonicalized {
    use std::borrow::Cow;

    #[test]
    fn non_file_scheme_is_noop() -> crate::Result {
        let url = gix_url::parse("https://github.com/byron/gitoxide".into())?;
        assert_eq!(url.canonicalized(&std::env::current_dir()?)?, url);
        Ok(())
    }

    #[test]
    fn absolute_file_url_does_nothing() -> crate::Result {
        #[cfg(not(windows))]
        let url = gix_url::parse("/this/path/does/not/exist".into())?;
        #[cfg(windows)]
        let url = gix_url::parse(r"C:\non\existing".into())?;
        assert_eq!(url.canonicalized(&std::env::current_dir()?)?, url);
        Ok(())
    }

    #[test]
    fn file_that_is_current_dir_is_absolutized() -> crate::Result {
        let url = gix_url::parse(".".into())?;
        assert!(gix_path::from_bstr(Cow::Borrowed(url.path.as_ref())).is_relative());
        assert!(gix_path::from_bstr(Cow::Borrowed(
            url.canonicalized(&std::env::current_dir()?)?.path.as_ref()
        ))
        .is_absolute());
        Ok(())
    }
}

use gix_url::ArgumentSafety;

#[test]
fn user() -> crate::Result {
    let mut url = gix_url::parse("https://user:password@host/path".into())?;

    assert_eq!(url.user(), Some("user"));
    assert_eq!(url.set_user(Some("new-user".into())), Some("user".into()));
    assert_eq!(url.user(), Some("new-user"));

    Ok(())
}

#[test]
fn password() -> crate::Result {
    let mut url = gix_url::parse("https://user:password@host/path".into())?;

    assert_eq!(url.password(), Some("password"));
    assert_eq!(url.set_password(Some("new-pass".into())), Some("password".into()));
    assert_eq!(url.password(), Some("new-pass"));

    Ok(())
}

#[test]
fn mutation_roundtrip() -> crate::Result {
    let mut url = gix_url::parse("https://user@host/path".into())?;
    url.set_user(Some("newuser".into()));
    url.set_password(Some("secret".into()));

    let serialized = url.to_bstring();
    let reparsed = gix_url::parse(serialized.as_ref())?;

    assert_eq!(url, reparsed);
    assert_eq!(reparsed.user(), Some("newuser"));
    assert_eq!(reparsed.password(), Some("secret"));

    Ok(())
}

#[test]
fn from_bytes_roundtrip() -> crate::Result {
    let original = "https://user:password@example.com:8080/path/to/repo";
    let url = gix_url::parse(original.into())?;

    let bytes = url.to_bstring();
    let from_bytes = gix_url::Url::from_bytes(bytes.as_ref())?;

    assert_eq!(url, from_bytes);
    assert_eq!(from_bytes.to_bstring(), bytes);

    Ok(())
}

#[test]
fn from_bytes_with_non_utf8_path() -> crate::Result {
    let url = gix_url::parse(b"/path/to\xff/repo".as_slice().into())?;
    let bytes = url.to_bstring();
    let from_bytes = gix_url::Url::from_bytes(bytes.as_ref())?;

    assert_eq!(url, from_bytes);
    assert_eq!(from_bytes.path, url.path);

    Ok(())
}

#[test]
fn user_argument_safety() -> crate::Result {
    let url = gix_url::parse("ssh://-Fconfigfile@foo/bar".into())?;

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
    let url = gix_url::parse("ssh://-oProxyCommand=open$IFS-aCalculator/foo".into())?;

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
    let url = gix_url::parse("ssh://foo/-oProxyCommand=open$IFS-aCalculator".into())?;

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

    Ok(())
}

#[test]
fn all_argument_safety_safe() -> crate::Result {
    let url = gix_url::parse("ssh://user.name@example.com/path/to/file".into())?;

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
    let url = gix_url::parse(all_bad.into())?;

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
        let url = gix_url::parse(input.into()).expect("input is valid url");
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
