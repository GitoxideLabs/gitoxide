mod decompose {
    use std::borrow::Cow;

    #[test]
    fn precomposed_unicode_is_decomposed() {
        let precomposed = "ä";
        let actual = gix_utils::str::decompose(precomposed.into());
        assert!(matches!(actual, Cow::Owned(_)), "new data is produced");
        assert_eq!(actual, "a\u{308}");
    }

    #[test]
    fn already_decomposed_does_not_copy() {
        let decomposed = "a\u{308}";
        let actual = gix_utils::str::decompose(decomposed.into());
        assert!(
            matches!(actual, Cow::Borrowed(_)),
            "pass-through as nothing needs to be done"
        );
        assert_eq!(actual, decomposed);
    }
}

mod precompose {
    use std::borrow::Cow;

    #[test]
    fn decomposed_unicode_is_precomposed() {
        let decomposed = "a\u{308}";
        let actual = gix_utils::str::precompose(decomposed.into());
        assert!(matches!(actual, Cow::Owned(_)), "new data is produced");
        assert_eq!(actual.chars().collect::<Vec<_>>(), ['ä']);
    }

    #[test]
    fn already_precomposed_does_not_copy() {
        for input in ["", "git_status.rs", "ä", "äq\u{308}", "한글"] {
            let actual = gix_utils::str::precompose(input.into());
            assert!(
                matches!(actual, Cow::Borrowed(_)),
                "unchanged input must be borrowed even when NFC quick-check is inconclusive: {input:?}"
            );
            assert_eq!(actual, input, "already precomposed text stays unchanged");
        }
    }

    #[test]
    fn non_bmp_characters_prevent_precomposition() {
        for input in [
            "📹U\u{308}.md",
            "U\u{308}📹.md",
            "U\u{308}\u{10000}.md",
            "\u{212b}U\u{308}📹.md",
            "A\u{315}\u{323}\u{301}📹.md",
        ] {
            let actual = gix_utils::str::precompose(input.into());
            assert_eq!(
                actual, input,
                "Git preserves the entire input when UTF-8-MAC conversion fails"
            );
            assert!(matches!(actual, Cow::Borrowed(_)), "unchanged input is not copied");
        }
    }

    #[test]
    fn noncanonical_combining_mark_order_is_preserved() {
        let input = "ا\u{651}\u{64f}";
        let actual = gix_utils::str::precompose(input.into());
        assert!(
            matches!(actual, Cow::Borrowed(_)),
            "unrelated combining marks must not be normalized or copied"
        );
        assert_eq!(actual, input, "combining mark order must stay unchanged");
    }

    #[test]
    fn earlier_combining_marks_block_composition() {
        let input = "A\u{315}\u{323}\u{301}";
        let actual = gix_utils::str::precompose(input.into());
        assert!(
            matches!(actual, Cow::Borrowed(_)),
            "an earlier, higher-class mark must block composition"
        );
        assert_eq!(actual, input, "combining mark order must stay unchanged");
    }

    #[test]
    fn canonically_equivalent_starter_is_decomposed_before_composition() {
        let actual = gix_utils::str::precompose("\u{212b}\u{301}".into());
        assert_eq!(actual, "\u{1fa}", "canonical composition must remain complete");
    }

    #[test]
    fn hangul_jamo_are_composed() {
        let actual = gix_utils::str::precompose("\u{1112}\u{1161}\u{11ab}\u{1100}\u{1173}\u{11af}".into());
        assert_eq!(
            actual, "한글",
            "class-zero characters can compose into Hangul syllables"
        );
    }

    #[test]
    fn nfc_bmp_starters_with_combining_marks_stay_unchanged() {
        use unicode_normalization::UnicodeNormalization;

        for starter in (0..=0xffff).filter_map(char::from_u32) {
            for mark in ['\u{301}', '\u{308}', '\u{323}'] {
                let input: String = [starter, mark].into_iter().nfc().collect();
                let actual = gix_utils::str::precompose(input.as_str().into());
                assert_eq!(actual, input, "NFC text must remain unchanged: {input:?}");
                assert!(
                    matches!(actual, Cow::Borrowed(_)),
                    "inconclusive quick-checks must still borrow unchanged text: {input:?}"
                );
            }
        }
    }
}

mod precompose_path {
    use std::{borrow::Cow, ffi::OsStr, path::Path};

    #[test]
    fn non_bmp_fallback_is_component_local() {
        for (input, expected) in [
            ("//src//./git_status.rs/", "//src//./git_status.rs/"),
            ("Teaching/Überwachung.md", "Teaching/Überwachung.md"),
            ("U\u{308}/\u{308}/A\u{308}", "Ü/\u{308}/Ä"),
            ("\u{212a}/📹U\u{308}/U\u{308}", "K/📹U\u{308}/Ü"),
            ("📹/U\u{308}", "📹/Ü"),
            ("U\u{308}/📹U\u{308}", "Ü/📹U\u{308}"),
            ("//📹//./U\u{308}/", "//📹//./Ü/"),
            ("📹U\u{308}/plain", "📹U\u{308}/plain"),
            ("📹\\U\u{308}", if cfg!(windows) { "📹\\Ü" } else { "📹\\U\u{308}" }),
        ] {
            let actual = gix_utils::str::precompose_path(Path::new(input).into());
            assert_eq!(
                actual.as_os_str(),
                OsStr::new(expected),
                "each filename is composed independently while path syntax is preserved"
            );
            assert_eq!(
                matches!(actual, Cow::Borrowed(_)),
                input == expected,
                "unchanged paths are borrowed"
            );
        }
    }
}

mod stripspace {
    use std::{io::Write, process::Stdio};

    use gix_testtools::TestResult;

    #[test]
    fn whitespace_is_normalized_without_changing_other_bytes() {
        let mut output = Vec::new();
        for (input, expected) in [
            (b"".as_slice(), b"".as_slice()),
            (b" \t\r\n\n", b""),
            (b"\n\tone \r\n \t\n\n two\t", b"\tone\n\n two\n"),
            (b"line\rbreak \r\nlast", b"line\rbreak\nlast\n"),
            (b"\0\xff\x0b\x0c \n", b"\0\xff\x0b\x0c\n"),
        ] {
            output.extend_from_slice(b"replace this");
            gix_utils::str::stripspace(input, None, &mut output);
            assert_eq!(output, expected, "only Git whitespace is stripped: {input:?}");
        }
    }

    #[test]
    fn comments_are_removed_before_blank_lines_are_collapsed() {
        let mut output = Vec::new();
        for (input, prefix, expected) in [
            (
                b"#first\n  #indented \t\n\n#between\n\nsecond\n#last\n".as_slice(),
                b"#".as_slice(),
                b"  #indented\n\nsecond\n".as_slice(),
            ),
            (b"//first\none\n//between\ntwo\n//last", b"//", b"one\ntwo\n"),
            (b"#only\n#comments", b"#", b""),
        ] {
            gix_utils::str::stripspace(input, Some(prefix), &mut output);
            assert_eq!(output, expected, "comment removal preserves paragraph boundaries");
        }
    }

    #[test]
    fn commenting_preserves_input_and_avoids_space_before_tab() {
        let mut output = Vec::new();
        for (input, expected) in [
            (b"".as_slice(), b"".as_slice()),
            (b"one", b"// one\n"),
            (b"one\n\n", b"// one\n//\n"),
            (b"\tone\n\ntwo\n", b"//\tone\n//\n// two\n"),
            (b" \r\n\xff\0\x0b\x0c", b"//  \r\n// \xff\0\x0b\x0c\n"),
        ] {
            output.extend_from_slice(b"replace this");
            gix_utils::str::comment_lines(input, b"//", &mut output);
            assert_eq!(
                output, expected,
                "commenting only adds the prefix and missing final newline"
            );
        }
    }

    #[test]
    fn binary_input_matches_git() -> TestResult {
        // Git reference: t0030-stripspace.sh and strbuf.c at 6de20f6092dcf9bdb1c8efe03db4b70c82b423dd.
        let directory = gix_testtools::tempfile::tempdir()?;
        let mut input: Vec<u8> = (u8::MIN..=u8::MAX).collect();
        input.extend_from_slice(b"\n\n \t\r\n#comment\n\n\tone \t\n\nlast");
        for mode in [None, Some("-s"), Some("-c")] {
            let mut command = gix_testtools::git_command(directory.path());
            command.arg("stripspace");
            command.args(mode);
            let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
            child.stdin.take().expect("stdin was piped").write_all(&input)?;
            let reference = child.wait_with_output()?;
            assert!(reference.status.success(), "Git accepts the binary input");
            let mut actual = Vec::new();
            match mode {
                Some("-c") => gix_utils::str::comment_lines(&input, b"#", &mut actual),
                _ => gix_utils::str::stripspace(&input, mode.map(|_| b"#".as_slice()), &mut actual),
            }
            assert_eq!(
                actual, reference.stdout,
                "Git stripspace mode {mode:?} is the reference"
            );
        }
        Ok(())
    }
}
