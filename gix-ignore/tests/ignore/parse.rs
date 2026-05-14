use bstr::BString;
use gix_glob::{Pattern, pattern::Mode};
use gix_testtools::fixture_bytes;

#[test]
fn precious() {
    let input = fixture_bytes("ignore/precious.txt");
    let actual: Vec<_> = gix_ignore::parse(&input, true).map(flat_map).collect();
    assert_eq!(
        actual,
        vec![
            pat_precious(".config", Mode::NO_SUB_DIR, 1),
            pat("$starts-with-dollar", Mode::NO_SUB_DIR, 2),
            pat_precious("*.html", Mode::NO_SUB_DIR | Mode::ENDS_WITH, 4),
            pat("foo.html", Mode::NO_SUB_DIR | Mode::NEGATIVE, 6),
            pat_precious("!/*", Mode::empty(), 12),
        ]
    );

    let actual: Vec<_> = gix_ignore::parse(&input, false).map(flat_map).collect();
    assert_eq!(
        actual,
        vec![
            pat("$.config", Mode::NO_SUB_DIR, 1),
            pat("$starts-with-dollar", Mode::NO_SUB_DIR, 2),
            pat("$*.html", Mode::NO_SUB_DIR, 4),
            pat("foo.html", Mode::NO_SUB_DIR | Mode::NEGATIVE, 6),
            pat("$!/*", Mode::empty(), 12),
        ]
    );
}

#[test]
fn byte_order_marks_are_no_patterns() {
    assert_eq!(
        flatten(gix_ignore::parse("\u{feff}hello".as_bytes(), false).next()),
        Some(pat(r"hello", Mode::NO_SUB_DIR, 1))
    );
}

#[test]
fn line_numbers_are_counted_correctly() {
    let input = fixture_bytes("ignore/various.txt");
    let actual: Vec<_> = gix_ignore::parse(&input, false).map(flat_map).collect();
    assert_eq!(
        actual,
        vec![
            pat("*.[oa]", Mode::NO_SUB_DIR, 2),
            pat("*.html", Mode::NO_SUB_DIR | Mode::ENDS_WITH, 5),
            pat("foo.html", Mode::NO_SUB_DIR | Mode::NEGATIVE, 8),
            pat("*", Mode::NO_SUB_DIR | Mode::ENDS_WITH | Mode::ABSOLUTE, 11),
            pat("foo", Mode::NEGATIVE | Mode::NO_SUB_DIR | Mode::ABSOLUTE, 12),
            pat("foo/*", Mode::ABSOLUTE, 13),
            pat("foo/bar", Mode::ABSOLUTE | Mode::NEGATIVE, 14)
        ]
    );
}

#[test]
fn line_endings_can_be_windows_or_unix() {
    assert_eq!(
        gix_ignore::parse(b"unix\nwindows\r\nlast", false)
            .map(flat_map)
            .collect::<Vec<_>>(),
        vec![
            pat(r"unix", Mode::NO_SUB_DIR, 1),
            pat(r"windows", Mode::NO_SUB_DIR, 2),
            pat(r"last", Mode::NO_SUB_DIR, 3)
        ]
    );
}

#[test]
fn comments_are_ignored_as_well_as_empty_ones() {
    assert!(gix_ignore::parse(b"# hello world", false).next().is_none());
    assert!(gix_ignore::parse(b"\n\r\n\t\t   \n", false).next().is_none());
}

#[test]
fn backslashes_before_hashes_are_no_comments() {
    assert_eq!(
        flatten(gix_ignore::parse(br"\#hello", false).next()),
        Some(pat(r"#hello", Mode::NO_SUB_DIR, 1))
    );
}

#[test]
fn trailing_spaces_can_be_escaped_to_be_literal() {
    fn parse_one(input: &str) -> (BString, Mode, usize, gix_ignore::Kind) {
        let actual: Vec<_> = gix_ignore::parse(input.as_bytes(), false).map(flat_map).collect();
        assert_eq!(actual.len(), 1, "{input:?} should match");
        actual.into_iter().next().expect("present")
    }

    assert_eq!(
        parse_one(r"a  \ "),
        pat(r"a  \ ", Mode::NO_SUB_DIR, 1),
        "a single escape in front of the last desired space is enough to keep it, along with the escape"
    );
    assert_eq!(
        parse_one(r"a  b  c "),
        pat("a  b  c", Mode::NO_SUB_DIR, 1),
        "spaces in the middle are fine, trailing ones are removed"
    );
    assert_eq!(
        parse_one(r"a\ \ \ "),
        pat(r"a\ \ \ ", Mode::NO_SUB_DIR, 1),
        "one can also escape every single one, what matters is the last escaped one"
    );
    assert_eq!(
        parse_one(r"a \  "),
        pat(r"a \ ", Mode::NO_SUB_DIR, 1),
        "or just the one in the middle, losing the last actual space"
    );
    assert_eq!(
        parse_one(r"a   \"),
        pat(r"a   \", Mode::NO_SUB_DIR, 1),
        "escaping 'nothing' also works"
    );
    assert_eq!(
        parse_one(r"a   \\\ "),
        pat(r"a   \\\ ", Mode::NO_SUB_DIR, 1),
        "an escaped backslash followed by a backslash escapes whitespace"
    );
    assert_eq!(
        parse_one(r"a   \\ "),
        pat(r"a   \\", Mode::NO_SUB_DIR, 1),
        "strange things like these work as well, but trailers are removed if the backslash is escaped"
    );
}

fn flatten(
    input: Option<(Pattern, usize, gix_ignore::Kind)>,
) -> Option<(BString, gix_glob::pattern::Mode, usize, gix_ignore::Kind)> {
    input.map(flat_map)
}

fn flat_map(input: (Pattern, usize, gix_ignore::Kind)) -> (BString, gix_glob::pattern::Mode, usize, gix_ignore::Kind) {
    (input.0.text, input.0.mode, input.1, input.2)
}

fn pat(pattern: &str, mode: Mode, pos: usize) -> (BString, Mode, usize, gix_ignore::Kind) {
    (pattern.into(), mode, pos, gix_ignore::Kind::Expendable)
}

fn pat_precious(pattern: &str, mode: Mode, pos: usize) -> (BString, Mode, usize, gix_ignore::Kind) {
    (pattern.into(), mode, pos, gix_ignore::Kind::Precious)
}
