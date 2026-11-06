use gix_encoding::{
    Converter, Encoding, convert,
    convert::{ConversionError, ErrorKind, Progress, Status},
    convert_to,
};
use gix_error::TestResult;

#[test]
fn unicode_roundtrips_and_boms() -> TestResult {
    let text = "A\0é日本🦀\u{feff}";
    for encoding in [
        Encoding::Utf8,
        Encoding::Utf16,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf16LeBom,
        Encoding::Utf32,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ] {
        let encoded = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        assert_eq!(
            convert(&encoded, encoding, Encoding::Utf8)?,
            text.as_bytes(),
            "{encoding} retains all scalars, including NUL and BOM"
        );
    }
    assert_eq!(
        convert(b"A", Encoding::Utf8, Encoding::Utf16Le)?,
        b"A\0",
        "explicit byte order does not emit a BOM"
    );
    assert_eq!(
        convert(b"A", Encoding::Utf8, Encoding::Utf16Be)?,
        b"\0A",
        "big-endian output is UTF-16, not UTF-8"
    );
    assert_eq!(
        convert(b"A", Encoding::Utf8, Encoding::Utf16BeBom)?,
        b"\xfe\xff\0A",
        "Git's BE-BOM output extension prepends the requested byte order mark"
    );
    for input in [&b""[..], &b"\xfe\xff\0A"[..]] {
        assert!(
            convert(input, Encoding::Utf16BeBom, Encoding::Utf8).is_err(),
            "Git recognizes UTF-16BE-BOM only as an output name"
        );
    }
    assert_eq!(
        convert(b"\xef\xbb\xbfA", Encoding::Utf8, Encoding::Utf8)?,
        b"\xef\xbb\xbfA",
        "UTF-8 BOM is preserved"
    );
    assert_eq!(
        convert(b"\xff\xfeA\0", Encoding::Utf16Le, Encoding::Utf8)?,
        "\u{feff}A".as_bytes(),
        "explicit UTF-16 byte order retains a BOM as data"
    );
    assert!(
        convert(b"", Encoding::Utf8, Encoding::Utf16)?.is_empty(),
        "generic iconv output is lazy even for BOMs"
    );
    assert_eq!(
        convert(b"", Encoding::Utf8, Encoding::Utf16LeBom)?,
        b"\xff\xfe",
        "Git's explicit BOM extension prepends it even for an empty raw conversion"
    );
    Ok(())
}

#[test]
fn strict_errors_and_reused_output() -> TestResult {
    for (encoding, input, kind) in [
        (Encoding::Utf8, &b"a\xc0\xaf"[..], ErrorKind::InvalidSequence),
        (Encoding::Utf8, &b"a\xed\xa0\x80"[..], ErrorKind::InvalidSequence),
        (Encoding::Utf8, &b"a\xf4\x90\x80\x80"[..], ErrorKind::InvalidSequence),
        (Encoding::Utf8, &b"a\xc3"[..], ErrorKind::IncompleteSequence),
        (Encoding::Utf16Le, &b"a\0\0\xdc"[..], ErrorKind::InvalidSequence),
        (Encoding::Utf16Le, &b"a\0\0\xd8"[..], ErrorKind::IncompleteSequence),
        (Encoding::Utf32Le, &b"a\0\0\0\0\0\x11\0"[..], ErrorKind::InvalidSequence),
    ] {
        let err = convert(input, encoding, Encoding::Utf8).expect_err("malformed data must never be replaced");
        let err = err
            .downcast_any_ref::<ConversionError>()
            .expect("conversion context survives erasure");
        assert_eq!(
            err.kind, kind,
            "{encoding} identifies malformed and incomplete sequences"
        );
        assert_eq!(
            err.offset,
            if encoding == Encoding::Utf8 {
                1
            } else if encoding == Encoding::Utf16Le {
                2
            } else {
                4
            },
            "error offsets count source bytes"
        );
    }
    let mut buffer = b"previous output".to_vec();
    assert!(
        convert_to("€".as_bytes(), Encoding::Utf8, Encoding::Iso8859_1, &mut buffer).is_err(),
        "Latin-1 cannot encode the euro sign"
    );
    assert!(
        buffer.is_empty(),
        "failed conversion cannot expose stale or partial data"
    );
    assert_eq!(
        convert(&[0x80], Encoding::Iso8859_1, Encoding::Utf8)?,
        "\u{80}".as_bytes(),
        "Latin-1 is distinct from Windows-1252"
    );
    assert!(
        convert(&[0x80], Encoding::Ascii, Encoding::Utf8).is_err(),
        "ASCII does not accept the high half of Windows-1252"
    );
    Ok(())
}

#[test]
fn converter_tracks_offsets_and_requires_reset() -> TestResult {
    let mut converter = Converter::new(Encoding::Utf8, Encoding::Utf16Le);
    let mut output = [0; 32];
    let first = converter.convert(b"abc", &mut output, false)?;
    assert_eq!(
        first,
        Progress {
            read: 3,
            written: 6,
            status: Status::InputEmpty,
        },
        "complete chunks make progress without ending the stream"
    );
    let second = converter.convert(b"\xc3", &mut output, false)?;
    assert_eq!(second.read, 0, "incomplete input remains available for resubmission");
    let err = converter
        .convert(b"\xc3", &mut output, true)
        .expect_err("EOF diagnoses truncated input");
    assert_eq!(
        err.downcast_any_ref::<ConversionError>().expect("typed context").offset,
        3,
        "offsets accumulate only consumed input"
    );
    assert!(
        converter.convert(b"a", &mut output, true).is_err(),
        "failed streams cannot be resumed"
    );
    converter.reset();
    assert_eq!(
        converter.convert(b"a", &mut output, true)?.status,
        Status::Finished,
        "reset starts an independent stream"
    );
    assert_eq!(
        converter.convert(b"", &mut output, true)?.status,
        Status::Finished,
        "empty finish calls are idempotent"
    );
    assert!(
        converter.convert(b"b", &mut output, true).is_err(),
        "finished streams do not silently concatenate"
    );
    Ok(())
}

#[test]
fn every_unicode_scalar_and_invalid_utf8_prefixes() -> TestResult {
    let text: String = (0..=0x10ffff).filter_map(char::from_u32).collect();
    for encoding in [
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ] {
        let encoded = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        assert_eq!(
            convert(&encoded, encoding, Encoding::Utf8)?,
            text.as_bytes(),
            "every Unicode scalar roundtrips through {encoding}"
        );
    }
    let mut random = 0x1234_abcd_u32;
    for len in 1..=8 {
        for _ in 0..10_000 {
            let mut bytes = vec![0; len];
            for byte in &mut bytes {
                random ^= random << 13;
                random ^= random >> 17;
                random ^= random << 5;
                *byte = random as u8;
            }
            let expected = std::str::from_utf8(&bytes);
            for encoding in [Encoding::Utf16Le, Encoding::Utf32Be] {
                let actual = convert(&bytes, Encoding::Utf8, encoding);
                assert_eq!(
                    actual.is_ok(),
                    expected.is_ok(),
                    "strict UTF-8 validity matches Rust for {bytes:x?}"
                );
                if let Ok(encoded) = actual {
                    assert_eq!(
                        convert(&encoded, encoding, Encoding::Utf8)?,
                        bytes,
                        "valid random inputs preserve their scalars"
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "legacy")]
#[test]
fn source_finalization_retries_when_output_is_full() -> TestResult {
    let mut converter = Converter::new(Encoding::Windows1255, Encoding::Utf8);
    let first = converter.convert(&[0xe0], &mut [], true)?;
    assert_eq!(
        (first.read, first.written, first.status),
        (1, 0, Status::OutputFull),
        "a buffered Hebrew letter must not be discarded by destination finalization"
    );
    let mut output = [0; 32];
    let last = converter.convert(&[], &mut output, true)?;
    assert_eq!(
        last.status,
        Status::Finished,
        "the pending source scalar finishes after output becomes available"
    );
    assert_eq!(
        &output[..last.written],
        "א".as_bytes(),
        "end-of-stream backpressure preserves the pending character"
    );
    Ok(())
}
