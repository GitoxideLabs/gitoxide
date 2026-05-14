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
mod matching;
