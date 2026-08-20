use std::path::Path;

use bstr::{ByteSlice, ByteVec};
use gix_filter::{eol, eol::AttributesDigest};

#[test]
fn with_binary_attribute_is_never_converted() {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"hi\r\nho",
        AttributesDigest::Binary,
        &mut buf,
        &mut no_call,
        Default::default(),
    )
    .expect("no error");
    assert!(!changed, "the user marked it as binary so it's never being touched");
}

#[test]
fn no_crlf_means_no_work() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"hi",
        AttributesDigest::TextCrlf,
        &mut buf,
        &mut no_call,
        Default::default(),
    )
    .expect("no error");
    assert!(!changed);

    let changed = eol::convert_to_git(
        b"hi",
        AttributesDigest::TextAutoCrlf,
        &mut buf,
        &mut no_object_in_index,
        Default::default(),
    )
    .expect("no error");
    assert!(!changed, "in auto-mode, the object is queried in the index as well.");
    Ok(())
}

#[test]
fn detected_as_binary() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"hi\0zero makes it binary",
        AttributesDigest::TextAuto,
        &mut buf,
        &mut no_call,
        Default::default(),
    )
    .expect("no error");
    assert!(
        !changed,
        "in auto-mode, we have a heuristic to see if the buffer is binary"
    );
    Ok(())
}

#[test]
fn trailing_dos_eof_marker_is_not_detected_as_binary() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"a\r\nb\r\n\x1a",
        AttributesDigest::TextAuto,
        &mut buf,
        &mut no_object_in_index,
        Default::default(),
    )
    .expect("no error");
    assert!(changed, "the DOS EOF marker doesn't stand in the way of conversion");
    assert_eq!(buf.as_bstr(), "a\nb\n\x1a");

    let changed = eol::convert_to_git(
        b"a\r\nb\r\n\x1a\x1a",
        AttributesDigest::TextAuto,
        &mut buf,
        &mut no_object_in_index,
        Default::default(),
    )
    .expect("no error");
    assert!(!changed, "a second marker isn't discounted, so this is binary");
    Ok(())
}

#[test]
fn fast_conversion_by_stripping_cr() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"a\r\nb\r\nc",
        AttributesDigest::TextCrlf,
        &mut buf,
        &mut no_call,
        Default::default(),
    )
    .expect("no error");
    assert!(changed);
    assert_eq!(buf.as_bstr(), "a\nb\nc", "here carriage returns can just be stripped");
    Ok(())
}

#[test]
fn slower_conversion_due_to_lone_cr() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_git(
        b"\r\ra\r\nb\r\nc",
        AttributesDigest::TextCrlf,
        &mut buf,
        &mut no_call,
        Default::default(),
    )
    .expect("no error");
    assert!(changed);
    assert_eq!(
        buf.as_bstr(),
        "\r\ra\nb\nc",
        "here carriage returns cannot be stripped but must be handled in pairs"
    );
    Ok(())
}

#[test]
fn crlf_in_index_prevents_conversion_to_lf() -> crate::Result {
    let mut buf = Vec::new();
    let mut called = false;
    let changed = eol::convert_to_git(
        b"eligible\n",
        AttributesDigest::TextAutoInput,
        &mut buf,
        &mut |buf| {
            called = true;
            buf.clear();
            buf.push_str("with CRLF\r\n");
            Ok(Some(()))
        },
        Default::default(),
    )
    .expect("no error");
    assert!(called, "in auto mode, the index is queried as well");
    assert!(
        !changed,
        "we saw the CRLF is present in the index, so it's unsafe to make changes"
    );
    Ok(())
}

#[test]
fn round_trip_check() -> crate::Result {
    let mut buf = Vec::new();
    for (input, expected) in [
        (&b"lone-nl\nhi\r\nho"[..], "LF would be replaced by CRLF in 'hello.txt'"),
        // despite trying, I was unable to get into the other branch
        (b"lone-cr\nhi\r\nho", "LF would be replaced by CRLF in 'hello.txt'"),
    ] {
        let err = eol::convert_to_git(
            input,
            AttributesDigest::TextCrlf,
            &mut buf,
            &mut no_call,
            eol::convert_to_git::Options {
                round_trip_check: Some(gix_filter::eol::convert_to_git::RoundTripCheck::Fail {
                    rela_path: Path::new("hello.txt"),
                }),
                config: Default::default(),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), expected);

        let changed = eol::convert_to_git(
            input,
            AttributesDigest::TextCrlf,
            &mut buf,
            &mut no_call,
            eol::convert_to_git::Options {
                round_trip_check: Some(gix_filter::eol::convert_to_git::RoundTripCheck::Warn {
                    rela_path: Path::new("hello.txt"),
                }),
                config: Default::default(),
            },
        )?;
        assert!(
            changed,
            "in warn mode, we will get a result even though it won't round-trip"
        );
    }
    Ok(())
}

fn no_call(_buf: &mut Vec<u8>) -> Result<Option<()>, gix_error::Exn> {
    unreachable!("index function will not be called")
}

fn no_object_in_index(_buf: &mut Vec<u8>) -> Result<Option<()>, gix_error::Exn> {
    Ok(None)
}
