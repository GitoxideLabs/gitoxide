use assert_matches::assert_matches;
use gix_url::parse::Error::*;

use crate::parse::parse;

#[test]
fn relative_path_due_to_double_colon() {
    // Note that a non-empty name before the `::` makes this a remote-helper location instead,
    // as covered by `parse::remote_helper`.
    assert_matches!(parse(":://host.xz/path/to/repo.git/"), Err(RelativeUrl { .. }));
}

#[test]
fn ssh_missing_path() {
    assert_matches!(parse("ssh://host.xz"), Err(MissingRepositoryPath { .. }));
}

#[test]
fn git_missing_path() {
    assert_matches!(parse("git://host.xz"), Err(MissingRepositoryPath { .. }));
}

#[test]
fn file_missing_path() {
    assert_matches!(parse("file://"), Err(MissingRepositoryPath { .. }));
}

#[test]
fn empty_input() {
    assert_matches!(parse(""), Err(MissingRepositoryPath { .. }));
}

#[test]
fn file_missing_host_path_separator() {
    assert_matches!(parse("file://.."), Err(MissingRepositoryPath { .. }));
    assert_matches!(parse("file://."), Err(MissingRepositoryPath { .. }));
    assert_matches!(parse("file://a"), Err(MissingRepositoryPath { .. }));
}

#[test]
fn missing_port_despite_indication() {
    assert_matches!(parse("ssh://host.xz:"), Err(MissingRepositoryPath { .. }));
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
        assert_matches!(
            parse(input),
            Err(Url { .. }),
            "invalid ports are diagnosed instead of being treated as host text: {input}"
        );
    }
}

#[test]
fn host_with_space() {
    assert_matches!(parse("http://has a space"), Err(Url { .. }));
    assert_matches!(parse("http://has a space/path"), Err(Url { .. }));
    assert_matches!(parse("https://example.com with space/path"), Err(Url { .. }));
}

#[test]
fn url_with_space_in_path() {
    // Spaces in path should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://example.com/ path"), Err(Url { .. }));
}

#[test]
fn url_with_space_in_username() {
    // Spaces in username should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://user name@example.com/path"), Err(Url { .. }));
}

#[test]
fn url_with_space_in_password() {
    // Spaces in password should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://user:pass word@example.com/path"), Err(Url { .. }));
}

#[test]
fn url_with_tab_in_path() {
    // Tabs in path should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://example.com/\tpath"), Err(Url { .. }));
}

#[test]
fn url_with_newline_in_path() {
    // Newlines in path should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://example.com/\npath"), Err(Url { .. }));
}

#[test]
fn url_with_tab_in_username() {
    // Tabs in username should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://user\tname@example.com/path"), Err(Url { .. }));
}

#[test]
fn url_with_tab_in_password() {
    // Tabs in password should be rejected for http URLs per RFC 3986
    assert_matches!(parse("http://user:pass\tword@example.com/path"), Err(Url { .. }));
}
