use crate::parse::parse;

fn assert_validation(input: &str, expected_message: &str, has_cause: bool) {
    let err = parse(input).unwrap_err();
    assert!(
        err.message.contains(expected_message),
        "unexpected validation message for {input:?}: {}",
        err.message
    );
    assert_eq!(
        err.input.as_ref().map(|input| input.as_slice()),
        Some(input.as_bytes()),
        "the rejected URL is retained"
    );
    assert_eq!(err.iter().count() > 1, has_cause, "cause expectation for {input:?}");
}

#[test]
fn relative_path_due_to_double_colon() {
    // Note that a non-empty name before the `::` makes this a remote-helper location instead,
    // as covered by `parse::remote_helper`.
    assert_validation(":://host.xz/path/to/repo.git/", "can not be parsed as valid URL", true);
}

#[test]
fn ssh_missing_path() {
    assert_validation("ssh://host.xz", "does not specify a path to a repository", false);
}

#[test]
fn git_missing_path() {
    assert_validation("git://host.xz", "does not specify a path to a repository", false);
}

#[test]
fn file_missing_path() {
    assert_validation("file://", "does not specify a path to a repository", false);
}

#[test]
fn empty_input() {
    assert_validation("", "does not specify a path to a repository", false);
}

#[test]
fn file_missing_host_path_separator() {
    for input in ["file://..", "file://.", "file://a"] {
        assert_validation(input, "does not specify a path to a repository", false);
    }
}

#[test]
fn missing_port_despite_indication() {
    assert_validation("ssh://host.xz:", "does not specify a path to a repository", false);
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
    for input in [
        "ssh://host.xz:abc/path",
        "git://host.xz:abc/path",
        "ssh://host.xz:65536/path",
        "ssh://host.xz:99999/path",
        "git://host.xz:65536/path",
    ] {
        assert_validation(input, "can not be parsed as valid URL", true);
    }
}

#[test]
fn host_with_space() {
    for input in [
        "http://has a space",
        "http://has a space/path",
        "https://example.com with space/path",
    ] {
        assert_validation(input, "can not be parsed as valid URL", true);
    }
}

#[test]
fn url_with_space_in_path() {
    // Spaces in path should be rejected for http URLs per RFC 3986
    assert_validation("http://example.com/ path", "can not be parsed as valid URL", true);
}

#[test]
fn url_with_space_in_username() {
    // Spaces in username should be rejected for http URLs per RFC 3986
    assert_validation(
        "http://user name@example.com/path",
        "can not be parsed as valid URL",
        true,
    );
}

#[test]
fn url_with_space_in_password() {
    // Spaces in password should be rejected for http URLs per RFC 3986
    assert_validation(
        "http://user:pass word@example.com/path",
        "can not be parsed as valid URL",
        true,
    );
}

#[test]
fn url_with_tab_in_path() {
    // Tabs in path should be rejected for http URLs per RFC 3986
    assert_validation("http://example.com/\tpath", "can not be parsed as valid URL", true);
}

#[test]
fn url_with_newline_in_path() {
    // Newlines in path should be rejected for http URLs per RFC 3986
    assert_validation("http://example.com/\npath", "can not be parsed as valid URL", true);
}

#[test]
fn url_with_tab_in_username() {
    // Tabs in username should be rejected for http URLs per RFC 3986
    assert_validation(
        "http://user\tname@example.com/path",
        "can not be parsed as valid URL",
        true,
    );
}

#[test]
fn url_with_tab_in_password() {
    // Tabs in password should be rejected for http URLs per RFC 3986
    assert_validation(
        "http://user:pass\tword@example.com/path",
        "can not be parsed as valid URL",
        true,
    );
}
