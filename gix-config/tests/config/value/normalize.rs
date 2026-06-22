use gix_config::value::normalize_bstr;

use crate::file::cow_str;

#[test]
fn not_modified_is_preserved() {
    let cow = normalize_bstr("hello world");
    assert_eq!(cow, cow_str("hello world"));
}

#[test]
fn modified_is_owned() {
    let cow = normalize_bstr("hello \"world\"");
    assert_eq!(cow, cow_str("hello world"));
}

#[test]
fn empty_quotes_are_empty() {
    let cow = normalize_bstr("\"\"");
    assert_eq!(cow, cow_str(""));
}

#[test]
fn all_quoted_is_unquoted() {
    let cow = normalize_bstr("\"hello world\"");
    assert_eq!(cow, cow_str("hello world"));
}

#[test]
fn all_quote_optimization_is_correct() {
    let cow = normalize_bstr(r#""hello" world\""#);
    assert_eq!(cow, cow_str("hello world\""));
}

#[test]
fn quotes_right_next_to_each_other() {
    let cow = normalize_bstr("\"hello\"\" world\"");
    assert_eq!(cow, cow_str("hello world").clone());
}

#[test]
fn escaped_quotes_are_kept() {
    let cow = normalize_bstr(r#""hello \"\" world""#);
    assert_eq!(cow, cow_str("hello \"\" world").clone());
}

#[test]
fn empty_string() {
    let cow = normalize_bstr("");
    assert_eq!(cow, cow_str(""));
}

#[test]
fn inner_quotes_are_removed() {
    assert_eq!(normalize_bstr(r#"5"hello world""#), cow_str("5hello world"));
    assert_eq!(normalize_bstr(r#"true"""#), cow_str("true"));
    assert_eq!(normalize_bstr(r#"fa"lse""#), cow_str("false"));
}

#[test]
fn newline_tab_backspace_are_escapable() {
    assert_eq!(normalize_bstr(r"\n\ta\b"), cow_str("\n\t"));
}

#[test]
fn tabs_are_not_resolved_to_spaces_unlike_what_git_does() {
    assert_eq!(normalize_bstr("\t"), cow_str("\t"));
}

#[test]
fn other_escapes_are_ignored_entirely() {
    assert_eq!(
        normalize_bstr(r"\x"),
        cow_str("x"),
        "however, these would cause failure on parsing level so we ignore it similar to subsections"
    );
    assert_eq!(normalize_bstr(r#""\x""#), cow_str("x"), "same if within quotes");
    assert_eq!(
        normalize_bstr(r#""\"#),
        cow_str(""),
        "freestanding escapes are ignored as well"
    );
}
