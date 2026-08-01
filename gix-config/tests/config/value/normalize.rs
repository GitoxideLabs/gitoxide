use std::borrow::Cow;

use gix_config::value::normalize;

#[test]
fn input_without_quotes_or_escapes_is_unchanged() {
    assert!(
        matches!(
            normalize("hello world"),
            Cow::Borrowed(value) if value == "hello world"
        ),
        "normalization leaves every byte unchanged, so the output can borrow the entire input"
    );
}

#[test]
fn embedded_quotes_are_removed() {
    assert!(matches!(
        normalize("hello \"world\""),
        Cow::Owned(value) if value == "hello world"
    ));
}

#[test]
fn empty_quotes_are_empty() {
    assert!(
        matches!(normalize("\"\""), Cow::Borrowed(value) if value.is_empty()),
        "removing the quotes leaves the empty subslice between them, so the output can borrow that subslice"
    );
}

#[test]
fn all_quoted_is_unquoted() {
    assert!(
        matches!(
            normalize("\"hello world\""),
            Cow::Borrowed(value) if value == "hello world"
        ),
        "removing only the enclosing bytes leaves a contiguous inner subslice that can be borrowed"
    );
}

#[test]
fn an_escaped_trailing_quote_is_preserved() {
    assert_eq!(&*normalize(r#""hello" world\""#), "hello world\"");
}

#[test]
fn quotes_right_next_to_each_other() {
    assert_eq!(&*normalize("\"hello\"\" world\""), "hello world");
}

#[test]
fn escaped_quotes_are_kept() {
    assert_eq!(&*normalize(r#""hello \"\" world""#), "hello \"\" world");
}

#[test]
fn empty_string() {
    assert!(
        matches!(normalize(""), Cow::Borrowed(value) if value.is_empty()),
        "the normalized output is identical to the empty input, so it can borrow the input"
    );
}

#[test]
fn quotes_are_removed_from_partially_quoted_values() {
    assert_eq!(&*normalize(r#"5"hello world""#), "5hello world");
    assert_eq!(&*normalize(r#"true"""#), "true");
    assert_eq!(&*normalize(r#"fa"lse""#), "false");
}

#[test]
fn newline_tab_and_backspace_escapes_are_interpreted() {
    // `\b` is the backspace character (0x08), matching git, not a request to
    // delete the preceding character.
    assert_eq!(&*normalize(r"\n\ta\b"), "\n\ta\x08");
}

#[test]
fn tabs_are_not_resolved_to_spaces_unlike_what_git_does() {
    assert!(
        matches!(normalize("\t"), Cow::Borrowed(value) if value == "\t"),
        "a literal tab is not an escape sequence, so normalization can borrow the unchanged input"
    );
}

#[test]
fn unsupported_escapes_drop_the_backslash() {
    assert_eq!(
        &*normalize(r"\x"),
        "x",
        "however, these would cause failure on parsing level so we ignore it similar to subsections"
    );
    assert_eq!(&*normalize(r#""\x""#), "x", "same if within quotes");
    assert_eq!(&*normalize(r#""\"#), "", "freestanding escapes are ignored as well");
}

