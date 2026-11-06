mod encoding {
    mod for_label {
        use gix_filter::worktree;

        #[test]
        fn unknown() {
            insta::assert_debug_snapshot!(worktree::encoding::for_label("FOO").expect_err("unknown"), "unknown", @"An encoding named 'FOO' is not known");
        }

        #[test]
        fn utf32_is_not_supported() {
            let mut message_diagnostics = Vec::new();
            for enc in ["UTF-32BE", "UTF-32LE", "UTF-32", "UTF-32LE-BOM", "UTF-32BE-BOM"] {
                message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                    &(worktree::encoding::for_label(enc).expect_err("the input must be rejected")),
                    &[],
                ));
            }
            insta::assert_debug_snapshot!(message_diagnostics, "utf32 is not supported", @"
            [
                An encoding named 'UTF-32BE' is not known,
                An encoding named 'UTF-32LE' is not known,
                An encoding named 'UTF-32' is not known,
                An encoding named 'UTF-32LE-BOM' is not known,
                An encoding named 'UTF-32BE-BOM' is not known,
            ]
            ");
        }

        #[test]
        fn various_spellings_of_utf_8_are_supported() {
            for enc in ["UTF8", "UTF-8", "utf-8", "utf8"] {
                let enc = worktree::encoding::for_label(enc).unwrap();
                assert_eq!(enc.name(), "UTF-8");
            }
        }

        #[test]
        fn explicit_utf_16_endianness_is_supported() -> gix_testtools::TestResult {
            for (label, expected) in [
                ("UTF-16BE", encoding_rs::UTF_16BE),
                ("UTF-16LE", encoding_rs::UTF_16LE),
                ("utf-16be", encoding_rs::UTF_16BE),
                (" \tUtf-16Le\r\n", encoding_rs::UTF_16LE),
            ] {
                assert_eq!(
                    worktree::encoding::for_label(label)?,
                    expected,
                    "explicit endianness is preserved despite case and surrounding whitespace"
                );
            }
            Ok(())
        }

        #[test]
        fn utf_16_labels_without_explicit_endianness_are_rejected() {
            for label in [
                "UTF-16",
                "utf-16",
                " \tUtf-16\r\n",
                "csunicode",
                "iso-10646-ucs-2",
                "ucs-2",
                "unicode",
                "unicodefeff",
                "unicodefffe",
            ] {
                let err = worktree::encoding::for_label(label)
                    .expect_err("UTF-16 aliases must not lose their BOM semantics when resolved");
                assert!(
                    err.is_validation(),
                    "{label:?}: unsupported labels are validation errors"
                );
                assert_eq!(
                    err.error().to_string(),
                    format!("The UTF-16 encoding '{label}' is not supported; use UTF-16LE or UTF-16BE without a BOM"),
                    "the error identifies the unsupported label and the supported alternatives"
                );
            }
        }

        #[test]
        fn various_utf_16_with_bom_suffix_are_unsupported() {
            let mut message_diagnostics = Vec::new();
            for label in ["UTF-16BE-BOM", "UTF-16LE-BOM"] {
                message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                    &(worktree::encoding::for_label(label).expect_err("the input must be rejected")),
                    &[],
                ));
            }
            insta::assert_debug_snapshot!(message_diagnostics, "various utf 16 with bom suffix are unsupported", @"
            [
                An encoding named 'UTF-16BE-BOM' is not known,
                An encoding named 'UTF-16LE-BOM' is not known,
            ]
            ");
        }

        #[test]
        fn latin_1_is_supported_with_fallback() {
            let enc = worktree::encoding::for_label("latin-1").unwrap();
            assert_eq!(
                enc.name(),
                "windows-1252",
                "the encoding crate has its own fallback for ISO-8859-1 which we try to use"
            );
        }
    }
}

mod encode_to_git {
    use bstr::ByteSlice;
    use gix_filter::{worktree, worktree::encode_to_git::RoundTripCheck};
    use gix_testtools::TestResult;

    #[test]
    fn simple() -> TestResult {
        let input = &b"hello"[..];
        for round_trip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
            let mut buf = b"previous content, not UTF-8: \xff".to_vec();
            worktree::encode_to_git(input, encoding_rs::UTF_8, &mut buf, round_trip)?;
            assert_eq!(buf.as_bstr(), input, "the previous content of `buf` is replaced");
        }
        Ok(())
    }

    #[test]
    fn reuses_large_buffer_for_small_inputs() -> TestResult {
        let large_input = vec![0x80; 128 * 1024];
        for round_trip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
            let mut buf = Vec::new();
            worktree::encode_to_git(&large_input, encoding_rs::WINDOWS_1252, &mut buf, round_trip)?;
            assert_eq!(
                buf.as_bstr(),
                "€".repeat(large_input.len()),
                "decoding can expand the input"
            );
            let capacity = buf.capacity();
            let allocation = buf.as_ptr();

            for _ in 0..32 {
                for (input, expected) in [(b"\x80".as_slice(), "€"), (b"a".as_slice(), "a"), (b"".as_slice(), "")] {
                    buf.clear();
                    worktree::encode_to_git(input, encoding_rs::WINDOWS_1252, &mut buf, round_trip)?;
                    assert_eq!(buf.as_bstr(), expected, "only the current input is returned");
                    assert_eq!(buf.capacity(), capacity, "small conversions retain the allocation");
                    assert_eq!(buf.as_ptr(), allocation, "the allocation is reused, not replaced");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn utf16_rejects_either_bom_before_decoding() {
        for encoding in [encoding_rs::UTF_16LE, encoding_rs::UTF_16BE] {
            for input in [b"\xff\xfeA\0", b"\xfe\xff\0A"] {
                for round_trip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
                    let mut buf = b"previous content".to_vec();
                    let err = worktree::encode_to_git(input, encoding, &mut buf, round_trip)
                        .expect_err("Git prohibits either BOM for both explicit-endian UTF-16 encodings");
                    assert!(err.is_validation(), "a prohibited BOM is a validation error");
                    assert_eq!(
                        err.error().to_string(),
                        format!("A BOM is prohibited for '{}'", encoding.name()),
                        "BOM validation is independent of decoding and round-trip checking"
                    );
                    assert_eq!(
                        buf, b"previous content",
                        "BOMs are rejected before decoding into the buffer"
                    );
                }
            }
        }
    }

    #[test]
    fn utf16_passes_the_round_trip_check() -> TestResult {
        for (encoding, input) in [
            (encoding_rs::UTF_16LE, super::UTF16LE),
            (encoding_rs::UTF_16BE, super::UTF16BE),
        ] {
            let mut buf = Vec::new();
            worktree::encode_to_git(input, encoding, &mut buf, RoundTripCheck::Fail)?;
            assert_eq!(
                buf.as_bstr(),
                super::UTF8,
                "valid {} input can be stored as UTF-8 losslessly, which Git accepts as well",
                encoding.name()
            );
        }
        Ok(())
    }
}

mod encode_to_worktree {
    use bstr::ByteSlice;
    use gix_filter::{worktree, worktree::encode_to_git::RoundTripCheck};
    use gix_testtools::TestResult;

    #[test]
    fn shift_jis() -> TestResult {
        let input = "ハローワールド";
        let mut buf = Vec::new();
        worktree::encode_to_worktree(input.as_bytes(), encoding_rs::SHIFT_JIS, &mut buf)?;

        let mut re_encoded = Vec::new();
        worktree::encode_to_git(&buf, encoding_rs::SHIFT_JIS, &mut re_encoded, RoundTripCheck::Fail)?;

        assert_eq!(re_encoded.as_bstr(), input, "this should be round-trippable too");
        Ok(())
    }

    #[test]
    fn utf16() -> TestResult {
        for (encoding, expected) in [
            (encoding_rs::UTF_16LE, super::UTF16LE),
            (encoding_rs::UTF_16BE, super::UTF16BE),
        ] {
            let mut buf = Vec::new();
            worktree::encode_to_worktree(super::UTF8.as_bytes(), encoding, &mut buf)?;
            assert_eq!(
                buf.as_bstr(),
                expected.as_bstr(),
                "Git writes files with a `working-tree-encoding` of {} as UTF-16, not UTF-8",
                encoding.name()
            );

            let mut re_encoded = Vec::new();
            worktree::encode_to_git(&buf, encoding, &mut re_encoded, RoundTripCheck::Fail)?;
            assert_eq!(re_encoded.as_bstr(), super::UTF8, "this should be round-trippable too");
        }
        Ok(())
    }
}

/// Text with ASCII, a character from the Basic Multilingual Plane and one that needs a surrogate pair in UTF-16.
const UTF8: &str = "hi あ😀\n";
/// [`UTF8`] as `UTF-16LE`, without a byte order mark.
const UTF16LE: &[u8] = b"h\0i\0 \0\x420\x3d\xd8\x00\xde\n\0";
/// [`UTF8`] as `UTF-16BE`, without a byte order mark.
const UTF16BE: &[u8] = b"\0h\0i\0 0\x42\xd8\x3d\xde\x00\0\n";
