mod name {
    macro_rules! mktests {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let actual = gix_validate::reference::name_partial_or_sanitize($input.as_bstr());
                assert_eq!(actual, $expected);
                assert!(gix_validate::reference::name_partial(actual.as_ref()).is_ok());
            }
        };
    }

    mod valid {
        use bstr::ByteSlice;

        macro_rules! mktest {
            ($name:ident, $input:expr) => {
                #[test]
                fn $name() {
                    assert!(gix_validate::tag::name($input.as_bstr()).is_ok())
                }
            };
        }
        mktest!(an_at_sign, b"@");
        // `mktests!` sanitizes as a *reference* name, where a lone `@` means `HEAD` and is refused,
        // so it has to be replaced. As a tag name it stays valid - `refs/tags/@` is a legal ref.
        mktests!(an_at_sign_san, b"@", "-");
        mktest!(chinese_utf8, "你好吗".as_bytes());
        mktests!(chinese_utf8_san, "你好吗".as_bytes(), "你好吗");
        mktest!(non_text, "😅🙌".as_bytes());
        mktests!(non_text_san, "😅🙌".as_bytes(), "😅🙌");
        mktest!(contains_an_at, b"hello@foo");
        mktests!(contains_an_at_san, b"hello@foo", "hello@foo");
        mktest!(contains_dot_lock, b"file.lock.ext");
        mktests!(contains_dot_lock_san, b"file.lock.ext", "file.lock.ext");
        mktest!(contains_brackets, b"this_{is-fine}_too");
        mktests!(contains_brackets_san, b"this_{is-fine}_too", "this_{is-fine}_too");
        mktest!(contains_brackets_and_at, b"this_{@is-fine@}_too");
        mktests!(
            contains_brackets_and_at_san,
            b"this_{@is-fine@}_too",
            "this_{@is-fine@}_too"
        );
        mktest!(dot_in_the_middle, b"token.other");
        mktests!(dot_in_the_middle_san, b"token.other", "token.other");
        mktest!(slash_inbetween, b"hello/world");
        mktests!(slash_inbetween_san, b"hello/world", "hello/world");
    }

    mod invalid {
        use bstr::ByteSlice;

        macro_rules! mktest {
            ($name:ident, $input:literal, $expected:ident, @$snapshot:literal) => {
                #[test]
                fn $name() {
                    let err = gix_validate::tag::name($input.as_bstr()).expect_err("the input is invalid");
                    insta::assert_debug_snapshot!(err, "invalid tag names retain their specific failure", @$snapshot);
                    assert!(matches!(err, gix_validate::tag::name::Error::$expected), "the failure retains its error variant");
                }
            };
        }
        macro_rules! mktestb {
            ($name:ident, $input:literal, @$snapshot:literal) => {
                #[test]
                fn $name() {
                    let err = gix_validate::tag::name($input.as_bstr()).expect_err("the input is invalid");
                    insta::assert_debug_snapshot!(err, "invalid tag names retain their specific failure", @$snapshot);
                    assert!(matches!(err, gix_validate::tag::name::Error::InvalidByte { .. }), "the failure retains its error variant");
                }
            };
        }
        mktest!(contains_ref_log_portion, b"this_looks_like_a_@{reflog}", ReflogPortion, @"ReflogPortion");
        mktests!(
            contains_ref_log_portion_san,
            b"this_looks_like_a_@{reflog}",
            "this_looks_like_a_@-reflog}"
        );
        mktest!(suffix_is_dot_lock, b"prefix.lock", LockFileSuffix, @"LockFileSuffix");
        mktest!(too_many_dots, b"......", RepeatedDot, @"RepeatedDot");
        mktests!(too_many_dots_san, b"......", "-");
        mktests!(too_many_dots_and_slashes_san, b"//....///....///", "-/-");
        mktests!(suffix_is_dot_lock_san, b"prefix.lock", "prefix");
        mktest!(suffix_is_dot_lock_multiple, b"prefix.lock.lock", LockFileSuffix, @"LockFileSuffix");
        mktests!(suffix_is_dot_lock_multiple_san, b"prefix.lock.lock", "prefix");
        mktest!(ends_with_slash, b"prefix/", EndsWithSlash, @"EndsWithSlash");
        mktest!(empty_component, b"prefix//suffix", RepeatedSlash, @"RepeatedSlash");
        mktests!(empty_component_san, b"prefix//suffix", "prefix/suffix");
        mktests!(ends_with_slash_san, b"prefix/", "prefix");
        mktest!(is_dot_lock, b".lock", StartsWithDot, @"StartsWithDot");
        mktest!(dot_lock_in_component, b"foo.lock/baz.lock/bar", LockFileSuffix, @"LockFileSuffix");
        mktests!(dot_lock_in_component_san, b"foo.lock/baz.lock/bar", "foo/baz/bar");
        mktests!(
            dot_lock_in_each_component_san,
            b"foo.lock/baz.lock/bar.lock",
            "foo/baz/bar"
        );
        mktests!(
            multiple_dot_lock_in_each_component_san,
            b"foo.lock.lock/baz.lock.lock/bar.lock.lock",
            "foo/baz/bar"
        );
        mktests!(
            dot_lock_in_each_component_special_san,
            b"...lock/..lock//lock",
            "-lock/lock"
        );
        mktests!(is_dot_lock_san, b".lock", "-lock");
        mktest!(contains_double_dot, b"with..double-dot", RepeatedDot, @"RepeatedDot");
        mktests!(contains_double_dot_san, b"with..double-dot", "with.double-dot");
        mktest!(starts_with_double_dot, b"..with-double-dot", RepeatedDot, @"RepeatedDot");
        mktests!(starts_with_double_dot_san, b"..with-double-dot", "-with-double-dot");
        mktest!(ends_with_double_dot, b"with-double-dot..", RepeatedDot, @"RepeatedDot");
        mktests!(ends_with_double_dot_san, b"with-double-dot..", "with-double-dot-");
        mktest!(starts_with_asterisk, b"*suffix", Asterisk, @"Asterisk");
        mktests!(starts_with_asterisk_san, b"*suffix", "-suffix");
        mktest!(starts_with_slash, b"/suffix", StartsWithSlash, @"StartsWithSlash");
        mktests!(starts_with_slash_san, b"/suffix", "suffix");
        mktest!(ends_with_asterisk, b"prefix*", Asterisk, @"Asterisk");
        mktests!(ends_with_asterisk_san, b"prefix*", "prefix-");
        mktest!(contains_asterisk, b"prefix*suffix", Asterisk, @"Asterisk");
        mktests!(contains_asterisk_san, b"prefix*suffix", "prefix-suffix");
        mktestb!(contains_null, b"prefix\0suffix", @r#"
        InvalidByte {
            byte: "\0",
        }
        "#);
        mktests!(contains_null_san, b"prefix\0suffix", "prefix-suffix");
        mktestb!(contains_bell, b"prefix\x07suffix", @r#"
        InvalidByte {
            byte: "\x07",
        }
        "#);
        mktests!(contains_bell_san, b"prefix\x07suffix", "prefix-suffix");
        mktestb!(contains_backspace, b"prefix\x08suffix", @r#"
        InvalidByte {
            byte: "\x08",
        }
        "#);
        mktests!(contains_backspace_san, b"prefix\x08suffix", "prefix-suffix");
        mktestb!(contains_vertical_tab, b"prefix\x0bsuffix", @r#"
        InvalidByte {
            byte: "\x0b",
        }
        "#);
        mktests!(contains_vertical_tab_san, b"prefix\x0bsuffix", "prefix-suffix");
        mktestb!(contains_form_feed, b"prefix\x0csuffix", @r#"
        InvalidByte {
            byte: "\x0c",
        }
        "#);
        mktests!(contains_form_feed_san, b"prefix\x0csuffix", "prefix-suffix");
        mktestb!(contains_ctrl_z, b"prefix\x1asuffix", @r#"
        InvalidByte {
            byte: "\x1a",
        }
        "#);
        mktests!(contains_ctrl_z_san, b"prefix\x1asuffix", "prefix-suffix");
        mktestb!(contains_esc, b"prefix\x1bsuffix", @r#"
        InvalidByte {
            byte: "\x1b",
        }
        "#);
        mktests!(contains_esc_san, b"prefix\x1bsuffix", "prefix-suffix");
        mktestb!(contains_colon, b"prefix:suffix", @r#"
        InvalidByte {
            byte: ":",
        }
        "#);
        mktests!(contains_colon_san, b"prefix:suffix", "prefix-suffix");
        mktestb!(contains_questionmark, b"prefix?suffix", @r#"
        InvalidByte {
            byte: "?",
        }
        "#);
        mktests!(contains_questionmark_san, b"prefix?suffix", "prefix-suffix");
        mktestb!(contains_open_bracket, b"prefix[suffix", @r#"
        InvalidByte {
            byte: "[",
        }
        "#);
        mktests!(contains_open_bracket_san, b"prefix[suffix", "prefix-suffix");
        mktestb!(contains_backslash, br"prefix\suffix", @r#"
        InvalidByte {
            byte: "\\",
        }
        "#);
        mktests!(contains_backslash_san, br"prefix\suffix", "prefix-suffix");
        mktestb!(contains_circumflex, b"prefix^suffix", @r#"
        InvalidByte {
            byte: "^",
        }
        "#);
        mktests!(contains_circumflex_san, b"prefix^suffix", "prefix-suffix");
        mktestb!(contains_tilde, b"prefix~suffix", @r#"
        InvalidByte {
            byte: "~",
        }
        "#);
        mktests!(contains_tilde_san, b"prefix~suffix", "prefix-suffix");
        mktestb!(contains_space, b"prefix suffix", @r#"
        InvalidByte {
            byte: " ",
        }
        "#);
        mktests!(contains_space_san, b"prefix suffix", "prefix-suffix");
        mktestb!(contains_tab, b"prefix\tsuffix", @r#"
        InvalidByte {
            byte: "\t",
        }
        "#);
        mktests!(contains_tab_san, b"prefix\tsuffix", "prefix-suffix");
        mktestb!(contains_newline, b"prefix\nsuffix", @r#"
        InvalidByte {
            byte: "\n",
        }
        "#);
        mktests!(contains_newline_san, b"prefix\nsuffix", "prefix-suffix");
        mktestb!(contains_carriage_return, b"prefix\rsuffix", @r#"
        InvalidByte {
            byte: "\r",
        }
        "#);
        mktests!(contains_carriage_return_san, b"prefix\rsuffix", "prefix-suffix");
        mktest!(starts_with_dot, b".with-dot", StartsWithDot, @"StartsWithDot");
        mktests!(starts_with_dot_san, b".with-dot", "-with-dot");
        mktest!(ends_with_dot, b"with-dot.", EndsWithDot, @"EndsWithDot");
        mktests!(ends_with_dot_san, b"with-dot.", "with-dot-");
        mktest!(empty, b"", Empty, @"Empty");
        mktests!(empty_san, b"", "-");
    }
}
