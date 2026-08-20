use gix_glob::{Pattern, pattern::Mode};

#[test]
fn display() {
    fn pat(text: &str, mode: Mode) -> String {
        Pattern {
            text: text.into(),
            mode,
            first_wildcard_pos: None,
        }
        .to_string()
    }
    assert_eq!(pat("a", Mode::ABSOLUTE), "/a");
    assert_eq!(pat("a", Mode::MUST_BE_DIR), "a/");
    assert_eq!(pat("a", Mode::NEGATIVE), "!a");
    assert_eq!(pat("a", Mode::ABSOLUTE | Mode::NEGATIVE | Mode::MUST_BE_DIR), "!/a/");
}

#[test]
fn wildcard_detection_distinguishes_escapes_from_wildcard_operators() {
    for text in ["*", "?", "[a]", r"\*"] {
        let pattern = Pattern::from_bytes_without_negation(text.as_bytes()).expect("non-empty pattern");
        assert!(pattern.has_wildcard(), "{text:?} contains a wildcard operator");
    }
    for text in ["literal", r"literal\escape"] {
        let pattern = Pattern::from_bytes_without_negation(text.as_bytes()).expect("non-empty pattern");
        assert!(
            !pattern.has_wildcard(),
            "{text:?} contains no wildcard operator even if it contains an escape"
        );
    }
}

mod matching;
