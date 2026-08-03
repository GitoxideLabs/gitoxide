use assert_matches::assert_matches;
use gix_url::parse::Error::*;

use crate::parse::parse;

#[test]
fn relative_path_due_to_double_colon() {
    assert_matches!(parse("invalid:://host.xz/path/to/repo.git/"), Err(RelativeUrl { .. }));
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
fn port_zero_is_invalid() {
    assert_matches!(parse("ssh://host.xz:0/path"), Err(Url { .. }));
}

#[test]
fn port_too_large() {
    assert_matches!(parse("ssh://host.xz:65536/path"), Err(Url { .. }));
    assert_matches!(parse("ssh://host.xz:99999/path"), Err(Url { .. }));
}

#[test]
fn invalid_port_format() {
    assert_matches!(
        parse("ssh://host.xz:abc/path"),
        Err(Url { .. }),
        "non-numeric ports must be rejected"
    );
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
