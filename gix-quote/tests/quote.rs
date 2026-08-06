mod single {
    use gix_quote::single;

    #[test]
    fn empty() {
        assert_eq!(single("".into()), "''");
    }

    #[test]
    fn unquoted_becomes_quoted() {
        assert_eq!(single("a".into()), "'a'");
        assert_eq!(single("a b".into()), "'a b'");
        assert_eq!(single("a\nb".into()), "'a\nb'", "newlines play no role");
    }

    #[test]
    fn existing_exclamation_mark_gets_escaped() {
        assert_eq!(single(r"a!b".into()), r"'a'\!'b'");
        assert_eq!(single(r"!".into()), r"''\!''");
        assert_eq!(single(r"\!".into()), r"'\'\!''");
    }

    #[test]
    fn existing_quote_gets_escaped() {
        assert_eq!(single(r"a'b".into()), r"'a'\''b'");
        assert_eq!(single(r"'".into()), r"''\'''");
        assert_eq!(single(r"'\''".into()), r"''\''\'\'''\'''");
    }

    #[test]
    fn complex() {
        let expected = "\'\0cmd `arg` $var\\\'\\\'\'ring\\// arg \"quoted\'\\!\'\"\'";
        assert_eq!(single("\0cmd `arg` $var\\'ring\\// arg \"quoted!\"".into()), expected);
    }
}

mod ansi_c {
    mod undo {
        use bstr::ByteSlice;
        use gix_quote::ansi_c;

        macro_rules! test {
            ($name:ident, $input:literal, $expected:literal, $consumed:literal) => {
                #[test]
                fn $name() {
                    assert_eq!(
                        ansi_c::undo($input.as_bytes().as_bstr()).expect("valid input"),
                        (
                            std::borrow::Cow::Borrowed($expected.as_bytes().as_bstr()),
                            $consumed
                        )
                    );
                }
            };
        }

        test!(unquoted_remains_unchanged, "hello", "hello", 5);
        test!(empty_surrounded_by_quotes, "\"\"", "", 2);
        test!(surrounded_only_by_quotes, "\"hello\"", "hello", 7);
        test!(typical_escapes, r#""\n\r\t""#, b"\n\r\t", 8);
        test!(untypical_escapes, r#""\a\b\f\v""#, b"\x07\x08\x0c\x0b", 10);
        test!(literal_escape_and_double_quote, r#""\"\\""#, br#""\"#, 6);
        test!(
            unicode_byte_escapes_by_number,
            r#""\346\277\261\351\207\216\t\347\264\224""#,
            "濱野\t純",
            40
        );
        test!(
            exclamation_and_tilde_survive_an_escape_with_double_escaping,
            r#""\\!\\#hello there/file.ext""#,
            r"\!\#hello there/file.ext",
            28
        );

        #[test]
        fn out_of_quote_characters_can_be_passed_and_will_not_be_consumed() {
            let input = br#""hello there" out of quote"#.as_bstr();
            let (unquoted, consumed) = ansi_c::undo(input).expect("valid input");
            assert_eq!(unquoted, std::borrow::Cow::Borrowed(b"hello there".as_bstr()));
            assert_eq!(&input[consumed..], " out of quote");
        }

        #[test]
        fn a_quote_that_is_never_closed_is_an_error() {
            for unterminated in [r#"""#, r#""abc"#, r#""abc def"#, r#""abc\"#, r#""\""#] {
                assert!(
                    ansi_c::undo(unterminated.into()).is_err(),
                    "{unterminated:?} should not parse, just like in `unquote_c_style()`"
                );
            }
        }

        #[test]
        fn fuzzed() {
            for invalid in ["\"\\", "\"Q\u{2}QT\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\0\0\\"] {
                assert!(ansi_c::undo(invalid.into()).is_err());
            }
        }
    }
}
