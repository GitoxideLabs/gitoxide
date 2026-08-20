use std::path::Path;

use gix_discover::parse;

#[test]
fn valid() -> crate::Result {
    assert_eq!(parse::gitdir(b"gitdir: a")?, Path::new("a"));
    assert_eq!(parse::gitdir(b"gitdir: relative/path")?, Path::new("relative/path"));
    assert_eq!(parse::gitdir(b"gitdir: ./relative/path")?, Path::new("./relative/path"));
    assert_eq!(parse::gitdir(b"gitdir: /absolute/path\n")?, Path::new("/absolute/path"));
    assert_eq!(
        parse::gitdir(b"gitdir: C:/hello/there\r\n")?,
        Path::new("C:/hello/there")
    );

    Ok(())
}

#[test]
fn invalid() {
    for (input, reason) in [
        (b"gitdir:".as_slice(), "missing prefix"),
        (b"bogus: foo".as_slice(), "invalid prefix"),
        (b"gitdir: ".as_slice(), "empty path"),
    ] {
        let err = parse::gitdir(input).expect_err(reason);
        assert_eq!(
            err.input.as_ref().map(|input| input.as_slice()),
            Some(input),
            "{reason}"
        );
        assert_eq!(err.message, "Format should be 'gitdir: <path>', but got", "{reason}");
    }
}
