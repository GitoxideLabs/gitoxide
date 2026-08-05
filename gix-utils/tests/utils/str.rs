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
        let actual = gix_utils::str::precompose("ä".into());
        assert!(
            matches!(actual, Cow::Borrowed(_)),
            "pass-through as nothing needs to be done"
        );
        assert_eq!(actual.chars().collect::<Vec<_>>(), ['ä']);
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
}
