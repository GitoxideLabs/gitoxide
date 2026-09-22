use crate::Result;
use std::path::Path;

use gix_discover::parse;

#[test]
fn valid() -> Result {
    assert_eq!(
        parse::gitdir(b"gitdir: a").map_err(gix_error::Exn::into_error)?,
        Path::new("a")
    );
    assert_eq!(
        parse::gitdir(b"gitdir: relative/path").map_err(gix_error::Exn::into_error)?,
        Path::new("relative/path")
    );
    assert_eq!(
        parse::gitdir(b"gitdir: ./relative/path").map_err(gix_error::Exn::into_error)?,
        Path::new("./relative/path")
    );
    assert_eq!(
        parse::gitdir(b"gitdir: /absolute/path\n").map_err(gix_error::Exn::into_error)?,
        Path::new("/absolute/path")
    );
    assert_eq!(
        parse::gitdir(b"gitdir: C:/hello/there\r\n").map_err(gix_error::Exn::into_error)?,
        Path::new("C:/hello/there")
    );

    Ok(())
}

#[test]
fn invalid() {
    let mut error_snapshots = Vec::new();
    for (input, reason) in [
        (b"gitdir:".as_slice(), "missing prefix"),
        (b"bogus: foo".as_slice(), "invalid prefix"),
        (b"gitdir: ".as_slice(), "empty path"),
    ] {
        let err = parse::gitdir(input).expect_err(reason);
        assert_eq!(
            err.values.get("input"),
            Some(&gix_error::MetadataValue::Bytes(input.into())),
            "{reason}"
        );
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
    }
    insta::assert_debug_snapshot!(error_snapshots, "invalid", @r#"
    [
        Format should be 'gitdir: <path>', but got, "input"="gitdir:",
        Format should be 'gitdir: <path>', but got, "input"="bogus: foo",
        Format should be 'gitdir: <path>', but got, "input"="gitdir: ",
    ]
    "#);
}
