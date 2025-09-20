use bstr::ByteSlice;
use gix_actor::Identity;
use winnow::stream::AsBStr;

#[test]
fn round_trip() -> gix_testtools::Result {
    static DEFAULTS: &[&[u8]] =     &[
        b"Sebastian Thiel <byronimo@gmail.com>",
        b"Sebastian Thiel < byronimo@gmail.com>",
        b"Sebastian Thiel <byronimo@gmail.com  >",
        b"Sebastian Thiel <\tbyronimo@gmail.com \t >",
        ".. ☺️Sebastian 王知明 Thiel🙌 .. <byronimo@gmail.com>".as_bytes(),
        b".. whitespace  \t  is explicitly allowed    - unicode aware trimming must be done elsewhere  <byronimo@gmail.com>"
    ];
    for input in DEFAULTS {
        let signature: Identity = gix_actor::IdentityRef::from_bytes::<()>(input).unwrap().into();
        let mut output = Vec::new();
        signature.write_to(&mut output)?;
        assert_eq!(output.as_bstr(), input.as_bstr());
    }
    Ok(())
}

#[test]
fn lenient_parsing() -> gix_testtools::Result {
    for (input, expected_email) in [
        (
            "First Last<<fl <First Last<fl@openoffice.org >> >",
            "fl <First Last<fl@openoffice.org >> ",
        ),
        (
            "First Last<fl <First Last<fl@openoffice.org>>\n",
            "fl <First Last<fl@openoffice.org",
        ),
    ] {
        let identity = gix_actor::IdentityRef::from_bytes::<()>(input.as_bytes()).unwrap();
        assert_eq!(identity.name, "First Last");
        assert_eq!(
            identity.email, expected_email,
            "emails are parsed but left as is for round-tripping"
        );
        let signature: Identity = identity.into();
        let mut output = Vec::new();
        signature.write_to(&mut output).expect("write does not complain");

        assert_eq!(output.as_bstr(), input, "round-tripping should keep these equivalent");
    }
    Ok(())
}

#[test]
fn newlines_still_rejected() -> gix_testtools::Result {
    // Test that newlines within the actual parsed name or email are still rejected
    let identity = gix_actor::IdentityRef {
        name: "First\nLast".into(),
        email: "test@example.com".into(),
    };
    let signature: Identity = identity.into();
    let mut output = Vec::new();
    let err = signature.write_to(&mut output).unwrap_err();
    assert_eq!(
        err.to_string(),
        r"Signature name or email must not contain \n",
        "newlines within parsed fields should still be rejected"
    );

    let identity = gix_actor::IdentityRef {
        name: "First Last".into(),
        email: "test\n@example.com".into(),
    };
    let signature: Identity = identity.into();
    let mut output = Vec::new();
    let err = signature.write_to(&mut output).unwrap_err();
    assert_eq!(
        err.to_string(),
        r"Signature name or email must not contain \n",
        "newlines within parsed fields should still be rejected"
    );
    Ok(())
}
