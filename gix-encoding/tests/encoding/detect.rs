use gix_encoding::{Detection, Encoding, convert, detect, detect::Evidence};
use gix_error::TestResult;

fn selected(encoding: Encoding, evidence: Evidence, bom_len: usize) -> Detection {
    Detection::Detected {
        encoding,
        evidence,
        bom_len,
    }
}

fn candidates(input: &[u8]) -> Vec<Encoding> {
    let Detection::Ambiguous { candidates } = detect(input) else {
        panic!("{input:?} should not have a preferred encoding");
    };
    candidates
}

#[test]
fn empty_ascii_and_utf8_have_distinct_evidence() {
    assert_eq!(detect(b""), Detection::Empty, "empty input provides no evidence");
    assert_eq!(
        detect(b"plain ASCII\ttext\r\n\x0c"),
        selected(Encoding::Ascii, Evidence::Ascii, 0),
        "ASCII compatibility does not identify the original encoding"
    );
    for text in ["é", "日本語", "🦀", "hello\u{feff}world"] {
        assert_eq!(
            detect(text.as_bytes()),
            selected(Encoding::Utf8, Evidence::Utf8, 0),
            "valid non-ASCII UTF-8 is preferred, not claimed to be certain"
        );
    }
    assert!(
        convert("é".as_bytes(), Encoding::Iso8859_1, Encoding::Utf8).is_ok(),
        "UTF-8 validity does not exclude other interpretations"
    );
    #[cfg(feature = "western")]
    assert!(
        convert("é".as_bytes(), Encoding::Windows1252, Encoding::Utf8).is_ok(),
        "UTF-8 validity does not exclude a Windows-1252 interpretation"
    );
}

#[test]
fn byte_order_marks_report_explicit_endianness_and_payload_offset() -> TestResult {
    for (encoding, bom) in [
        (Encoding::Utf8, &b"\xef\xbb\xbf"[..]),
        (Encoding::Utf16Le, &b"\xff\xfe"[..]),
        (Encoding::Utf16Be, &b"\xfe\xff"[..]),
        (Encoding::Utf32Le, &b"\xff\xfe\0\0"[..]),
        (Encoding::Utf32Be, &b"\0\0\xfe\xff"[..]),
    ] {
        for text in ["", "Aé日本🦀\u{feff}", "\u{1}\0"] {
            let mut input = bom.to_vec();
            input.extend(convert(text.as_bytes(), Encoding::Utf8, encoding)?);
            assert_eq!(
                detect(&input),
                selected(encoding, Evidence::ByteOrderMark, bom.len()),
                "{encoding}: a BOM is evidence even for empty or control-containing text"
            );
            assert_eq!(
                convert(&input[bom.len()..], encoding, Encoding::Utf8)?,
                text.as_bytes(),
                "callers can skip just the signature, preserving interior BOMs"
            );
            assert_eq!(
                convert(&input, encoding, Encoding::Utf8)?,
                format!("\u{feff}{text}").as_bytes(),
                "detection does not change the existing BOM-as-data conversion semantics"
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_bom_payloads_do_not_fall_back_to_legacy_encodings() {
    for (input, encoding, bom_len) in [
        (&b"\xef\xbb\xbf\xc3"[..], Encoding::Utf8, 3),
        (&b"\xef\xbb\xbf\xff"[..], Encoding::Utf8, 3),
        (&b"\xff\xfeA"[..], Encoding::Utf16Le, 2),
        (&b"\xfe\xff\xd8\0"[..], Encoding::Utf16Be, 2),
        (&b"\xff\xfe\0\0A\0"[..], Encoding::Utf32Le, 4),
        (&b"\0\0\xfe\xff\0\x11\0\0"[..], Encoding::Utf32Be, 4),
    ] {
        assert_eq!(
            detect(input),
            Detection::InvalidBom { encoding, bom_len },
            "the complete BOM is recognized but its entire payload must validate"
        );
    }
    for input in [&b"\xef\xbb"[..], &b"\xff"[..], &b"\0\0\xfe"[..]] {
        assert!(
            matches!(detect(input), Detection::Ambiguous { .. }),
            "an incomplete signature cannot identify an encoding"
        );
    }
}

#[test]
fn bomless_unicode_requires_repeated_text_patterns() -> TestResult {
    for encoding in [
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ] {
        for text in ["abcd", "ab日本", "hello world", "hello é日本🦀\n"] {
            let input = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
            assert_eq!(
                detect(&input),
                selected(encoding, Evidence::UnicodeBytePattern, 0),
                "{encoding}: repeated ASCII code units plus strict validation support a heuristic"
            );
        }
        for text in ["a", "abc", "a日本語"] {
            let input = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
            assert!(
                candidates(&input).contains(&encoding),
                "{encoding}: fewer than four units or fewer than half ASCII units provide insufficient evidence"
            );
        }
        let mut input = convert(b"hello world", Encoding::Utf8, encoding)?;
        input.push(0xff);
        assert!(
            !candidates(&input).contains(&encoding),
            "the full slice, including an incomplete final code unit, must validate"
        );
    }
    let tied = candidates(b"A\0\0BC\0\0D");
    assert!(
        tied.contains(&Encoding::Utf16Le) && tied.contains(&Encoding::Utf16Be),
        "equally strong byte-pattern evidence for both byte orders must not be broken by iteration order"
    );
    let input = convert("日本語日本語".as_bytes(), Encoding::Utf8, Encoding::Utf16Le)?;
    let compatible = candidates(&input);
    assert!(
        compatible.contains(&Encoding::Utf16Le) && compatible.contains(&Encoding::Utf16Be),
        "non-ASCII UTF-16 without a BOM can be valid in both byte orders"
    );
    Ok(())
}

#[cfg(feature = "asian")]
#[test]
fn iso2022jp_is_checked_before_seven_bit_text() -> TestResult {
    for text in ["日本語", "yen: ¥, overline: ‾"] {
        let input = convert(text.as_bytes(), Encoding::Utf8, Encoding::Iso2022Jp)?;
        assert!(input.is_ascii(), "ISO-2022-JP is a seven-bit encoding");
        assert_eq!(
            detect(&input),
            selected(Encoding::Iso2022Jp, Evidence::Iso2022JpEscape, 0),
            "recognized escape sequences are stronger evidence than seven-bit validity"
        );
    }
    assert!(
        !candidates(b"\x1b$B$").contains(&Encoding::Iso2022Jp),
        "a truncated shifted character invalidates the ISO-2022-JP candidate"
    );
    assert!(
        matches!(detect(b"\x1b[31mred\x1b[0m"), Detection::Ambiguous { .. }),
        "terminal escapes are not ISO-2022-JP signatures"
    );
    Ok(())
}

#[test]
fn legacy_candidates_are_compatible_not_ranked_or_merged() -> TestResult {
    let compatible = candidates(b"caf\xe9");
    assert!(
        compatible.contains(&Encoding::Iso8859_1),
        "Latin-1 remains a candidate without optional encoding features"
    );
    #[cfg(feature = "western")]
    assert!(
        compatible.contains(&Encoding::Windows1252),
        "Latin-1 and Windows-1252 cannot be distinguished by these bytes"
    );
    assert!(
        !compatible.contains(&Encoding::Utf8) && !compatible.contains(&Encoding::Ascii),
        "an invalid encoding is not a candidate"
    );
    let compatible = candidates(&[0x81]);
    assert!(compatible.contains(&Encoding::Iso8859_1), "Latin-1 accepts all bytes");
    #[cfg(feature = "western")]
    assert!(
        !compatible.contains(&Encoding::Windows1252),
        "undefined Windows-1252 bytes remain invalid under iconv semantics"
    );
    #[cfg(feature = "asian")]
    for (encoding, superset, text) in [
        (Encoding::ShiftJis, Encoding::Cp932, "日本語"),
        (Encoding::EucKr, Encoding::Cp949, "한국어"),
        (Encoding::Gb2312, Encoding::Gbk, "中文"),
        (Encoding::Big5, Encoding::Big5Hkscs, "中文"),
    ] {
        let input = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        let compatible = candidates(&input);
        assert!(
            compatible.contains(&encoding) && compatible.contains(&superset),
            "validity alone must not prefer {encoding} over {superset}"
        );
    }
    Ok(())
}

#[test]
fn controls_and_binary_looking_bytes_do_not_force_a_text_guess() {
    for input in [
        &b"\0\0\0\0\0\0\0\0"[..],
        &b"\x01\x02\x03"[..],
        &b"hello\0world"[..],
        &b"\x1b[31mred\x1b[0m"[..],
    ] {
        assert!(
            candidates(input).contains(&Encoding::Iso8859_1),
            "compatibility is not proof that arbitrary bytes are text"
        );
    }
}

#[cfg(not(feature = "asian"))]
#[test]
fn disabled_iso2022jp_is_not_detected_from_its_escape_sequence() -> TestResult {
    let compatible = candidates(b"\x1b$B$3$s$K$A$O\x1b(B");
    assert!(
        compatible.contains(&Encoding::Ascii) && compatible.contains(&Encoding::Iso8859_1),
        "a disabled Japanese codec cannot turn an ambiguous seven-bit stream into a preferred interpretation"
    );
    for encoding in compatible {
        assert_ne!(encoding.name(), "ISO-2022-JP", "disabled codecs are not candidates");
    }
    Ok(())
}

#[test]
fn long_preferred_paths_validate_every_chunk() -> TestResult {
    let text = "hello 日本 ".repeat(1000);
    for (encoding, evidence, bom) in [
        (Encoding::Utf8, Evidence::ByteOrderMark, &b"\xef\xbb\xbf"[..]),
        (Encoding::Utf16Le, Evidence::ByteOrderMark, &b"\xff\xfe"[..]),
        (Encoding::Utf16Be, Evidence::ByteOrderMark, &b"\xfe\xff"[..]),
        (Encoding::Utf32Le, Evidence::ByteOrderMark, &b"\xff\xfe\0\0"[..]),
        (Encoding::Utf32Be, Evidence::ByteOrderMark, &b"\0\0\xfe\xff"[..]),
        (Encoding::Utf16Le, Evidence::UnicodeBytePattern, &b""[..]),
        (Encoding::Utf16Be, Evidence::UnicodeBytePattern, &b""[..]),
        (Encoding::Utf32Le, Evidence::UnicodeBytePattern, &b""[..]),
        (Encoding::Utf32Be, Evidence::UnicodeBytePattern, &b""[..]),
        #[cfg(feature = "asian")]
        (Encoding::Iso2022Jp, Evidence::Iso2022JpEscape, &b""[..]),
    ] {
        let mut input = bom.to_vec();
        input.extend(convert(text.as_bytes(), Encoding::Utf8, encoding)?);
        assert_eq!(
            detect(&input),
            selected(encoding, evidence, bom.len()),
            "{encoding}: valid text can span many scratch buffers"
        );
        let valid_len = input.len();
        input.push(0xff);
        if bom.is_empty() {
            assert!(
                !candidates(&input).contains(&encoding),
                "{encoding}: a malformed tail invalidates a candidate despite earlier evidence"
            );
        } else {
            assert_eq!(
                detect(&input),
                Detection::InvalidBom {
                    encoding,
                    bom_len: bom.len(),
                },
                "{encoding}: a BOM does not bypass final validation"
            );
        }
        input.truncate(valid_len);
        input.extend(convert(b"\0", Encoding::Utf8, encoding)?);
        if bom.is_empty() {
            assert!(
                candidates(&input).contains(&encoding),
                "{encoding}: a final control prevents a preferred interpretation without invalidating decoding"
            );
            let input = convert(format!("\0{text}").as_bytes(), Encoding::Utf8, encoding)?;
            assert!(
                candidates(&input).contains(&encoding),
                "{encoding}: later text must not hide an earlier control"
            );
        } else {
            assert_eq!(
                detect(&input),
                selected(encoding, evidence, bom.len()),
                "{encoding}: control characters do not invalidate BOM evidence"
            );
        }
    }
    Ok(())
}

#[test]
fn validation_reaches_the_end_and_candidate_sets_match_strict_conversion() -> TestResult {
    let mut long = "hello é🦀 ".repeat(1000).into_bytes();
    assert_eq!(
        detect(&long),
        selected(Encoding::Utf8, Evidence::Utf8, 0),
        "long UTF-8 input is recognized"
    );
    long.push(0xc3);
    let compatible = candidates(&long);
    assert!(
        !compatible.contains(&Encoding::Utf8),
        "an invalid suffix beyond the scratch buffer is not overlooked"
    );
    for input in [
        &long[..],
        &b"caf\xe9"[..],
        &b"\x1b$B$"[..],
        &b"\x81"[..],
        &b"\0\0\0\0"[..],
    ] {
        let expected: Vec<_> = Encoding::ALL
            .iter()
            .copied()
            .filter(|encoding| {
                !matches!(
                    encoding,
                    Encoding::Utf16 | Encoding::Utf32 | Encoding::Utf16LeBom | Encoding::Utf16BeBom
                ) && convert(input, *encoding, Encoding::Utf8).is_ok()
            })
            .collect();
        assert_eq!(
            candidates(input),
            expected,
            "candidates use the strict codec and stable declaration order, not a likelihood ranking"
        );
    }
    for byte in 0..=u8::MAX {
        match detect(&[byte]) {
            Detection::Detected { encoding, bom_len, .. } => {
                convert(&[byte][bom_len..], encoding, Encoding::Utf8)?;
            }
            Detection::Ambiguous { candidates } => {
                for encoding in candidates {
                    convert(&[byte], encoding, Encoding::Utf8)?;
                }
            }
            other => panic!("a single byte must be detected or ambiguous, got {other:?}"),
        }
    }
    Ok(())
}
