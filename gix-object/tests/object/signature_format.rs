use gix_object::{bstr::BStr, signature::Format};

#[test]
fn parse() -> gix_error::TestResult {
    for (value, expected) in [
        ("openpgp", Format::OpenPgp),
        ("OpenPGP", Format::OpenPgp),
        (" x509 ", Format::X509),
        ("SSH", Format::Ssh),
        (" ssh ", Format::Ssh),
    ] {
        assert_eq!(
            Format::parse(value.into())?,
            expected,
            "format names are case-insensitive and trim whitespace"
        );
    }
    Ok(())
}

#[test]
fn parse_invalid() {
    for (value, unsupported) in [
        (BStr::new(b""), false),
        (BStr::new(b" "), false),
        (BStr::new(b"open pgp"), false),
        (BStr::new(b"ssh\nformat"), false),
        (BStr::new(b"\xff"), false),
        (BStr::new(b"unknown"), true),
        (BStr::new(b"future-format"), true),
        (BStr::new(b" future_format "), true),
    ] {
        let err = Format::parse(value).expect_err("the format cannot be used");
        assert_eq!(
            err.is_unsupported(),
            unsupported,
            "well-formed unknown names are unsupported"
        );
        assert_eq!(
            err.is_validation(),
            !unsupported,
            "malformed names require correcting input"
        );
        assert!(
            err.metadata()
                .any(|metadata| { metadata.get("input") == Some(&gix_error::MetadataValue::from(value)) }),
            "the original input is preserved, including non-UTF-8 bytes"
        );
    }
}
