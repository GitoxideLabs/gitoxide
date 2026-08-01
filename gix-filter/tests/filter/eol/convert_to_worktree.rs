use bstr::ByteSlice;
use gix_filter::{
    eol,
    eol::{AttributesDigest, Configuration, Mode},
};

#[test]
fn no_conversion_if_attribute_digest_does_not_allow_it() -> crate::Result {
    let mut buf = Vec::new();
    for digest in [
        AttributesDigest::Binary,
        AttributesDigest::TextInput,
        AttributesDigest::TextAutoInput,
    ] {
        let changed = eol::convert_to_worktree(b"hi\nho", digest, &mut buf, Default::default())?;
        assert!(!changed, "the digest doesn't allow for CRLF changes");
    }
    Ok(())
}

#[test]
fn no_conversion_if_configuration_does_not_allow_it() -> crate::Result {
    let mut buf = Vec::new();
    for digest in [AttributesDigest::Text, AttributesDigest::TextAuto] {
        for config in [
            Configuration {
                auto_crlf: eol::AutoCrlf::Input,
                eol: Some(Mode::CrLf),
            },
            Configuration {
                auto_crlf: eol::AutoCrlf::Disabled,
                eol: Some(Mode::Lf),
            },
        ] {
            let changed = eol::convert_to_worktree(b"hi\nho", digest, &mut buf, config)?;
            assert!(!changed, "the configuration doesn't allow for changes");
        }
    }
    Ok(())
}

#[test]
fn no_conversion_if_nothing_to_do() -> crate::Result {
    let mut buf = Vec::new();
    for (input, digest, msg) in [
        (
            &b"hi\r\nho"[..],
            AttributesDigest::TextCrlf,
            "no lone line feed to handle",
        ),
        (
            &b"binary\0linefeed\nho"[..],
            AttributesDigest::TextAutoCrlf,
            "binary in auto-mode is never handled",
        ),
        (
            &b"binary\nlinefeed\r\nho"[..],
            AttributesDigest::TextAutoCrlf,
            "mixed crlf and lf is avoided",
        ),
        (
            &b"eligible-but-disabled\nhere"[..],
            AttributesDigest::Binary,
            "designated binary is never handled",
        ),
    ] {
        let changed = eol::convert_to_worktree(input, digest, &mut buf, Default::default())?;
        assert!(!changed, "{msg}");
    }
    Ok(())
}

#[test]
fn each_nl_is_replaced_with_crnl() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_worktree(
        b"hi\n\nho\nend",
        AttributesDigest::TextCrlf,
        &mut buf,
        Default::default(),
    )?;
    assert!(
        changed,
        "the buffer has to be changed as it is explicitly demanded and has newlines to convert"
    );
    assert_eq!(buf.as_bstr(), "hi\r\n\r\nho\r\nend");
    Ok(())
}

#[test]
fn trailing_dos_eof_marker_is_not_detected_as_binary() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_worktree(
        b"a\nb\n\x1a",
        AttributesDigest::TextAutoCrlf,
        &mut buf,
        Default::default(),
    )?;
    assert!(changed, "the DOS EOF marker doesn't stand in the way of conversion");
    assert_eq!(buf.as_bstr(), "a\r\nb\r\n\x1a");

    let changed = eol::convert_to_worktree(
        b"a\nb\n\x1a\x1a",
        AttributesDigest::TextAutoCrlf,
        &mut buf,
        Default::default(),
    )?;
    assert!(!changed, "a second marker isn't discounted, so this is binary");
    Ok(())
}

#[test]
fn existing_crnl_are_not_replaced_for_safety_nor_are_lone_cr() -> crate::Result {
    let mut buf = Vec::new();
    let changed = eol::convert_to_worktree(
        b"hi\r\n\nho\r\nend\r",
        AttributesDigest::TextCrlf,
        &mut buf,
        Default::default(),
    )?;
    assert!(changed);
    assert_eq!(buf.as_bstr(), "hi\r\n\r\nho\r\nend\r");
    Ok(())
}
