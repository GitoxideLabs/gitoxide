use crate::parse::parse;

fn assert_validation(input: &str, has_cause: bool) -> gix_error::Exn<gix_error::Message> {
    let err = parse(input).expect_err("the URL must be rejected");
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(input.as_bytes())),
        "the rejected URL is retained"
    );
    assert_eq!(err.iter().count() > 1, has_cause, "cause expectation for {input:?}");
    err
}

#[test]
fn relative_path_due_to_double_colon() {
    // Note that a non-empty name before the `::` makes this a remote-helper location instead,
    // as covered by `parse::remote_helper`.
    insta::assert_debug_snapshot!(assert_validation(":://host.xz/path/to/repo.git/", true), "relative path due to double colon", @r#"
    URL can not be parsed as valid URL, "input"=":://host.xz/path/to/repo.git/"
    |
    └─ relative URL without a base
    "#);
}

#[test]
fn ssh_missing_path() {
    insta::assert_debug_snapshot!(assert_validation("ssh://host.xz", false), "ssh missing path", @r#"URL does not specify a path to a repository, "input"="ssh://host.xz""#);
}

#[test]
fn git_missing_path() {
    insta::assert_debug_snapshot!(assert_validation("git://host.xz", false), "git missing path", @r#"URL does not specify a path to a repository, "input"="git://host.xz""#);
}

#[test]
fn file_missing_path() {
    insta::assert_debug_snapshot!(assert_validation("file://", false), "file missing path", @r#"URL does not specify a path to a repository, "input"="file://""#);
}

#[test]
fn empty_input() {
    insta::assert_debug_snapshot!(assert_validation("", false), "empty input", @r#"local path is empty and does not specify a path to a repository, "input"="""#);
}

#[test]
fn file_missing_host_path_separator() {
    let mut diagnostics = Vec::new();
    for input in ["file://..", "file://.", "file://a"] {
        diagnostics.push(assert_validation(input, false));
    }
    insta::assert_debug_snapshot!(diagnostics, "file missing host path separator", @r#"
    [
        URL does not specify a path to a repository, "input"="file://..",
        URL does not specify a path to a repository, "input"="file://.",
        URL does not specify a path to a repository, "input"="file://a",
    ]
    "#);
}

#[test]
fn missing_port_despite_indication() {
    insta::assert_debug_snapshot!(assert_validation("ssh://host.xz:", false), "missing port despite indication", @r#"URL does not specify a path to a repository, "input"="ssh://host.xz:""#);
}

#[test]
fn port_zero_is_accepted_for_git_compatibility() {
    for input in [
        "ssh://host.xz:0/path",
        "ssh://[::1]:0/path",
        "git://host.xz:0/path",
        "git://[::1]:0/path",
    ] {
        let url = parse(input).expect("Git accepts port zero");
        assert_eq!(url.port, Some(0), "port zero is retained: {input}");
    }
}

#[test]
fn textual_and_overflowing_ssh_and_git_ports_are_rejected_despite_git() {
    let mut diagnostics = Vec::new();
    for input in [
        "ssh://host.xz:abc/path",
        "git://host.xz:abc/path",
        "ssh://host.xz:65536/path",
        "ssh://host.xz:99999/path",
        "git://host.xz:65536/path",
    ] {
        diagnostics.push(assert_validation(input, true));
    }
    insta::assert_debug_snapshot!(diagnostics, "textual and overflowing ssh and git ports are rejected despite git", @r#"
    [
        URL can not be parsed as valid URL, "input"="ssh://host.xz:abc/path"
        |
        └─ invalid port number - must be between 1-65535,
        URL can not be parsed as valid URL, "input"="git://host.xz:abc/path"
        |
        └─ invalid port number - must be between 1-65535,
        URL can not be parsed as valid URL, "input"="ssh://host.xz:65536/path"
        |
        └─ invalid port number - must be between 1-65535
        |
        └─ number too large to fit in target type,
        URL can not be parsed as valid URL, "input"="ssh://host.xz:99999/path"
        |
        └─ invalid port number - must be between 1-65535
        |
        └─ number too large to fit in target type,
        URL can not be parsed as valid URL, "input"="git://host.xz:65536/path"
        |
        └─ invalid port number - must be between 1-65535
        |
        └─ number too large to fit in target type,
    ]
    "#);
}

#[test]
fn host_with_space() {
    let mut diagnostics = Vec::new();
    for input in [
        "http://has a space",
        "http://has a space/path",
        "https://example.com with space/path",
    ] {
        diagnostics.push(assert_validation(input, true));
    }
    insta::assert_debug_snapshot!(diagnostics, "host with space", @r#"
    [
        URL can not be parsed as valid URL, "input"="http://has a space"
        |
        └─ invalid domain character,
        URL can not be parsed as valid URL, "input"="http://has a space/path"
        |
        └─ invalid domain character,
        URL can not be parsed as valid URL, "input"="https://example.com with space/path"
        |
        └─ invalid domain character,
    ]
    "#);
}

#[test]
fn url_with_space_in_path() {
    // Spaces in path should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://example.com/ path", true), "url with space in path", @r#"
    URL can not be parsed as valid URL, "input"="http://example.com/ path"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_space_in_username() {
    // Spaces in username should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://user name@example.com/path", true), "url with space in username", @r#"
    URL can not be parsed as valid URL, "input"="http://user name@example.com/path"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_space_in_password() {
    // Spaces in password should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://user:pass word@example.com/path", true), "url with space in password", @r#"
    URL can not be parsed as valid URL, "input"="http://user:pass word@example.com/path"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_tab_in_path() {
    // Tabs in path should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://example.com/\tpath", true), "url with tab in path", @r#"
    URL can not be parsed as valid URL, "input"="http://example.com/\tpath"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_newline_in_path() {
    // Newlines in path should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://example.com/\npath", true), "url with newline in path", @r#"
    URL can not be parsed as valid URL, "input"="http://example.com/\npath"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_tab_in_username() {
    // Tabs in username should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://user\tname@example.com/path", true), "url with tab in username", @r#"
    URL can not be parsed as valid URL, "input"="http://user\tname@example.com/path"
    |
    └─ invalid domain character
    "#);
}

#[test]
fn url_with_tab_in_password() {
    // Tabs in password should be rejected for http URLs per RFC 3986
    insta::assert_debug_snapshot!(assert_validation("http://user:pass\tword@example.com/path", true), "url with tab in password", @r#"
    URL can not be parsed as valid URL, "input"="http://user:pass\tword@example.com/path"
    |
    └─ invalid domain character
    "#);
}
