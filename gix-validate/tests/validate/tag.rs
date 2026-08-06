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
            ($name:ident, $input:literal, $expected:ident) => {
                #[test]
                fn $name() {
                    match gix_validate::tag::name($input.as_bstr()) {
                        Err(gix_validate::tag::name::Error::$expected) => {}
                        got => panic!("Wanted {}, got {:?}", stringify!($expected), got),
                    }
                }
            };
        }
        macro_rules! mktestb {
            ($name:ident, $input:literal) => {
                #[test]
                fn $name() {
                    match gix_validate::tag::name($input.as_bstr()) {
                        Err(gix_validate::tag::name::Error::InvalidByte { .. }) => {}
                        got => panic!("Wanted {}, got {:?}", stringify!($expected), got),
                    }
                }
            };
        }
        mktest!(contains_ref_log_portion, b"this_looks_like_a_@{reflog}", ReflogPortion);
        mktests!(
            contains_ref_log_portion_san,
            b"this_looks_like_a_@{reflog}",
            "this_looks_like_a_@-reflog}"
        );
        mktest!(suffix_is_dot_lock, b"prefix.lock", LockFileSuffix);
        mktest!(too_many_dots, b"......", RepeatedDot);
        mktests!(too_many_dots_san, b"......", "-");
        mktests!(too_many_dots_and_slashes_san, b"//....///....///", "-/-");
        mktests!(suffix_is_dot_lock_san, b"prefix.lock", "prefix");
        mktest!(suffix_is_dot_lock_multiple, b"prefix.lock.lock", LockFileSuffix);
        mktests!(suffix_is_dot_lock_multiple_san, b"prefix.lock.lock", "prefix");
        mktest!(ends_with_slash, b"prefix/", EndsWithSlash);
        mktest!(empty_component, b"prefix//suffix", RepeatedSlash);
        mktests!(empty_component_san, b"prefix//suffix", "prefix/suffix");
        mktests!(ends_with_slash_san, b"prefix/", "prefix");
        mktest!(is_dot_lock, b".lock", StartsWithDot);
        mktest!(dot_lock_in_component, b"foo.lock/baz.lock/bar", LockFileSuffix);
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
        mktest!(contains_double_dot, b"with..double-dot", RepeatedDot);
        mktests!(contains_double_dot_san, b"with..double-dot", "with.double-dot");
        mktest!(starts_with_double_dot, b"..with-double-dot", RepeatedDot);
        mktests!(starts_with_double_dot_san, b"..with-double-dot", "-with-double-dot");
        mktest!(ends_with_double_dot, b"with-double-dot..", RepeatedDot);
        mktests!(ends_with_double_dot_san, b"with-double-dot..", "with-double-dot-");
        mktest!(starts_with_asterisk, b"*suffix", Asterisk);
        mktests!(starts_with_asterisk_san, b"*suffix", "-suffix");
        mktest!(starts_with_slash, b"/suffix", StartsWithSlash);
        mktests!(starts_with_slash_san, b"/suffix", "suffix");
        mktest!(ends_with_asterisk, b"prefix*", Asterisk);
        mktests!(ends_with_asterisk_san, b"prefix*", "prefix-");
        mktest!(contains_asterisk, b"prefix*suffix", Asterisk);
        mktests!(contains_asterisk_san, b"prefix*suffix", "prefix-suffix");
        mktestb!(contains_null, b"prefix\0suffix");
        mktests!(contains_null_san, b"prefix\0suffix", "prefix-suffix");
        mktestb!(contains_bell, b"prefix\x07suffix");
        mktests!(contains_bell_san, b"prefix\x07suffix", "prefix-suffix");
        mktestb!(contains_backspace, b"prefix\x08suffix");
        mktests!(contains_backspace_san, b"prefix\x08suffix", "prefix-suffix");
        mktestb!(contains_vertical_tab, b"prefix\x0bsuffix");
        mktests!(contains_vertical_tab_san, b"prefix\x0bsuffix", "prefix-suffix");
        mktestb!(contains_form_feed, b"prefix\x0csuffix");
        mktests!(contains_form_feed_san, b"prefix\x0csuffix", "prefix-suffix");
        mktestb!(contains_ctrl_z, b"prefix\x1asuffix");
        mktests!(contains_ctrl_z_san, b"prefix\x1asuffix", "prefix-suffix");
        mktestb!(contains_esc, b"prefix\x1bsuffix");
        mktests!(contains_esc_san, b"prefix\x1bsuffix", "prefix-suffix");
        mktestb!(contains_colon, b"prefix:suffix");
        mktests!(contains_colon_san, b"prefix:suffix", "prefix-suffix");
        mktestb!(contains_questionmark, b"prefix?suffix");
        mktests!(contains_questionmark_san, b"prefix?suffix", "prefix-suffix");
        mktestb!(contains_open_bracket, b"prefix[suffix");
        mktests!(contains_open_bracket_san, b"prefix[suffix", "prefix-suffix");
        mktestb!(contains_backslash, br"prefix\suffix");
        mktests!(contains_backslash_san, br"prefix\suffix", "prefix-suffix");
        mktestb!(contains_circumflex, b"prefix^suffix");
        mktests!(contains_circumflex_san, b"prefix^suffix", "prefix-suffix");
        mktestb!(contains_tilde, b"prefix~suffix");
        mktests!(contains_tilde_san, b"prefix~suffix", "prefix-suffix");
        mktestb!(contains_space, b"prefix suffix");
        mktests!(contains_space_san, b"prefix suffix", "prefix-suffix");
        mktestb!(contains_tab, b"prefix\tsuffix");
        mktests!(contains_tab_san, b"prefix\tsuffix", "prefix-suffix");
        mktestb!(contains_newline, b"prefix\nsuffix");
        mktests!(contains_newline_san, b"prefix\nsuffix", "prefix-suffix");
        mktestb!(contains_carriage_return, b"prefix\rsuffix");
        mktests!(contains_carriage_return_san, b"prefix\rsuffix", "prefix-suffix");
        mktest!(starts_with_dot, b".with-dot", StartsWithDot);
        mktests!(starts_with_dot_san, b".with-dot", "-with-dot");
        mktest!(ends_with_dot, b"with-dot.", EndsWithDot);
        mktests!(ends_with_dot_san, b"with-dot.", "with-dot-");
        mktest!(empty, b"", Empty);
        mktests!(empty_san, b"", "-");
    }
}
