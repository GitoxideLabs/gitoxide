mod encoding {
    use gix_filter::{encoding::Encoding, worktree};
    use gix_testtools::TestResult;

    #[test]
    fn for_label() -> TestResult {
        for (label, expected) in [
            ("UTF8", Encoding::Utf8),
            ("utf-8", Encoding::Utf8),
            ("UTF-16", Encoding::Utf16),
            ("UTF-16LE", Encoding::Utf16Le),
            ("UTF-16BE", Encoding::Utf16Be),
            ("UTF-16LE-BOM", Encoding::Utf16LeBom),
            ("UTF-16BE-BOM", Encoding::Utf16BeBom),
            ("UTF-32", Encoding::Utf32),
            ("UTF-32LE", Encoding::Utf32Le),
            ("UTF-32BE", Encoding::Utf32Be),
            ("latin-1", Encoding::Iso8859_1),
            ("ISO-8859-1", Encoding::Iso8859_1),
            ("WINDOWS-1252", Encoding::Windows1252),
        ] {
            assert_eq!(
                worktree::encoding::for_label(label)?,
                expected,
                "Git's encoding labels resolve faithfully"
            );
        }
        for label in ["FOO", "UTF-32LE-BOM", "UTF-32BE-BOM"] {
            assert!(
                worktree::encoding::for_label(label).is_err(),
                "unknown labels are rejected"
            );
        }
        Ok(())
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
            worktree::encode_to_git(input, gix_encoding::Encoding::Utf8, &mut buf, round_trip)?;
            assert_eq!(buf.as_bstr(), input, "the previous content of `buf` is replaced");
        }
        Ok(())
    }

    #[test]
    fn reuses_large_buffer_for_small_inputs() -> TestResult {
        let large_input = vec![0x80; 128 * 1024];
        for round_trip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
            let mut buf = Vec::new();
            worktree::encode_to_git(&large_input, gix_encoding::Encoding::Windows1252, &mut buf, round_trip)?;
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
                    worktree::encode_to_git(input, gix_encoding::Encoding::Windows1252, &mut buf, round_trip)?;
                    assert_eq!(buf.as_bstr(), expected, "only the current input is returned");
                    assert_eq!(buf.capacity(), capacity, "small conversions retain the allocation");
                    assert_eq!(buf.as_ptr(), allocation, "the allocation is reused, not replaced");
                }
            }
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
        worktree::encode_to_worktree(input.as_bytes(), gix_encoding::Encoding::ShiftJis, &mut buf)?;

        let mut re_encoded = Vec::new();
        worktree::encode_to_git(
            &buf,
            gix_encoding::Encoding::ShiftJis,
            &mut re_encoded,
            RoundTripCheck::Fail,
        )?;

        assert_eq!(re_encoded.as_bstr(), input, "this should be round-trippable too");
        Ok(())
    }

    #[test]
    fn utf16_preserves_explicit_byte_order_and_surrogate_pairs() -> TestResult {
        // PR #3163: both checkout and round-trip checking must encode UTF-16.
        let text = "hi あ😀\n";
        for (label, expected) in [
            ("UTF-16LE", b"h\0i\0 \0\x42\x30\x3d\xd8\x00\xde\n\0".as_slice()),
            ("UTF-16BE", b"\0h\0i\0 \x30\x42\xd8\x3d\xde\x00\0\n".as_slice()),
        ] {
            let encoding = worktree::encoding::for_label(label)?;
            let mut worktree_bytes = b"previous contents".to_vec();
            worktree::encode_to_worktree(text.as_bytes(), encoding, &mut worktree_bytes)?;
            assert_eq!(
                worktree_bytes, expected,
                "checkout writes {label} code units without a BOM, including the surrogate pair"
            );

            let mut git_bytes = b"previous contents".to_vec();
            worktree::encode_to_git(expected, encoding, &mut git_bytes, RoundTripCheck::Fail)?;
            assert_eq!(
                git_bytes,
                text.as_bytes(),
                "valid {label} input passes the byte-exact round-trip check"
            );
        }
        Ok(())
    }
}

#[test]
fn utf16_bom_policy_is_independent_of_roundtrip_checks() -> gix_testtools::TestResult {
    use gix_filter::worktree::{self, encode_to_git::RoundTripCheck};

    let generic = worktree::encoding::for_label("UTF-16")?;
    let mut checked_out = Vec::new();
    worktree::encode_to_worktree(b"A", generic, &mut checked_out)?;
    assert!(
        checked_out == b"\xff\xfeA\0" || checked_out == b"\xfe\xff\0A",
        "plain UTF-16 checkout includes a BOM identifying the byte order"
    );
    let mut decoded = Vec::new();
    worktree::encode_to_git(&checked_out, generic, &mut decoded, RoundTripCheck::Fail)?;
    assert_eq!(decoded, b"A", "plain UTF-16 checkout passes its own round-trip check");

    for roundtrip in [RoundTripCheck::Skip, RoundTripCheck::Fail] {
        for input in [b"A\0".as_slice(), b"\0A".as_slice()] {
            let error = worktree::encode_to_git(input, generic, &mut decoded, roundtrip)
                .expect_err("plain UTF-16 requires a BOM even without round-trip checking");
            assert!(
                error.to_string().contains("BOM is required"),
                "the missing BOM is diagnosed"
            );
            assert!(decoded.is_empty(), "rejected input cannot expose prior decoded bytes");
        }
        for label in ["UTF-16LE", "UTF-16BE"] {
            let encoding = worktree::encoding::for_label(label)?;
            for input in [b"\xff\xfeA\0".as_slice(), b"\xfe\xff\0A".as_slice()] {
                let error = worktree::encode_to_git(input, encoding, &mut decoded, roundtrip)
                    .expect_err("either BOM is prohibited for either explicit byte order");
                assert!(
                    error.to_string().contains("BOM is prohibited"),
                    "{label} rejects the BOM before decoding or checking the round trip"
                );
                assert!(decoded.is_empty(), "BOM rejection cannot leave misdecoded text");
            }
        }
    }
    Ok(())
}

#[test]
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn git_worktree_bom_and_roundtrip_baseline() -> gix_testtools::TestResult {
    use std::fs;

    use gix_filter::worktree::{self, encode_to_git::RoundTripCheck};

    let fixture = gix_testtools::scripted_fixture_read_only("worktree_encodings.sh")?;
    for case in fs::read_dir(fixture.join("cases"))? {
        let case = case?.path();
        let label = fs::read_to_string(case.join("label"))?;
        let encoding = worktree::encoding::for_label(label.as_str())?;
        let roundtrip = fs::read_to_string(case.join("roundtrip"))?;
        let roundtrip = if roundtrip == "check" {
            RoundTripCheck::Fail
        } else {
            RoundTripCheck::Skip
        };
        let input = fs::read(case.join("input"))?;
        let mut actual = b"previous contents".to_vec();
        let result = worktree::encode_to_git(&input, encoding, &mut actual, roundtrip);
        if fs::read_to_string(case.join("status"))? == "accept" {
            result?;
            assert_eq!(
                actual,
                fs::read(case.join("expected"))?,
                "Git agrees for {label} in {}",
                case.display()
            );
            if matches!(roundtrip, RoundTripCheck::Fail) {
                let mut checked_out = Vec::new();
                worktree::encode_to_worktree(&actual, encoding, &mut checked_out)?;
                assert_eq!(
                    checked_out, input,
                    "checkout reproduces the {label} bytes that passed Git's round-trip check"
                );
            }
        } else {
            assert!(result.is_err(), "Git rejects {label} in {}", case.display());
        }
    }
    Ok(())
}

#[test]
fn empty_worktree_files_do_not_gain_a_bom() -> gix_testtools::TestResult {
    use gix_encoding::Encoding;
    use gix_filter::worktree;

    for encoding in [
        Encoding::Utf16,
        Encoding::Utf16LeBom,
        Encoding::Utf16BeBom,
        Encoding::Utf32,
    ] {
        let mut buf = b"previous content".to_vec();
        worktree::encode_to_worktree(b"", encoding, &mut buf)?;
        assert!(
            buf.is_empty(),
            "Git leaves empty files empty even for encodings with a BOM"
        );
    }
    Ok(())
}
