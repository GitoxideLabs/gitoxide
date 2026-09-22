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
        fn various_utf_16_without_bom_suffix_are_supported() {
            for label in ["UTF-16BE", "UTF-16LE"] {
                let enc = worktree::encoding::for_label(label).unwrap();
                assert_eq!(enc.name(), label);
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
    use crate::Result;
    use bstr::ByteSlice;
    use gix_filter::{worktree, worktree::encode_to_git::RoundTripCheck};

    #[test]
    fn simple() -> Result {
        let input = &b"hello"[..];
        for round_trip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
            let mut buf = Vec::new();
            worktree::encode_to_git(input, encoding_rs::UTF_8, &mut buf, round_trip)?;
            assert_eq!(buf.as_bstr(), input);
        }
        Ok(())
    }
}

mod encode_to_worktree {
    use crate::Result;
    use bstr::ByteSlice;
    use gix_filter::{worktree, worktree::encode_to_git::RoundTripCheck};

    #[test]
    fn shift_jis() -> Result {
        let input = "ハローワールド";
        let mut buf = Vec::new();
        worktree::encode_to_worktree(input.as_bytes(), encoding_rs::SHIFT_JIS, &mut buf)?;

        let mut re_encoded = Vec::new();
        worktree::encode_to_git(&buf, encoding_rs::SHIFT_JIS, &mut re_encoded, RoundTripCheck::Fail)?;

        assert_eq!(re_encoded.as_bstr(), input, "this should be round-trippable too");
        Ok(())
    }
}
