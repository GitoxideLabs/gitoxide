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
    let url = parse("ssh://host.xz:abc/path").expect("non-numeric port is treated as part of host");
    assert_eq!(
        url.host(),
        Some("host.xz:abc"),
        "port parse failure makes it part of hostname"
    );
    assert_eq!(url.port, None);
}
