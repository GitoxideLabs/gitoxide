use bstr::BString;
use gix_attributes::{StateRef, parse, state::ValueRef};
use gix_glob::pattern::Mode;
use gix_testtools::fixture_bytes;

#[test]
fn byte_order_marks_are_no_patterns() {
    assert_eq!(
        line("\u{feff}hello"),
        (pattern(r"hello", Mode::NO_SUB_DIR, None), vec![], 1)
    );
    assert_eq!(
        line("\u{feff}\"hello\""),
        (pattern(r"hello", Mode::NO_SUB_DIR, None), vec![], 1)
    );
}

#[test]
fn line_numbers_are_counted_correctly() {
    let input = fixture_bytes("attributes/various.txt");
    assert_eq!(
        try_lines(&String::from_utf8(input).unwrap()).unwrap(),
        vec![
            (pattern(r"*.[oa]", Mode::NO_SUB_DIR, Some(0)), vec![set("c")], 2),
            (
                pattern(r"*.html", Mode::NO_SUB_DIR | Mode::ENDS_WITH, Some(0)),
                vec![set("a"), value("b", "c")],
                5
            ),
            (pattern(r"!foo.html", Mode::NO_SUB_DIR, None), vec![set("x")], 8),
            (pattern(r"#a/path", Mode::empty(), None), vec![unset("a")], 10),
            (
                pattern(r"*", Mode::ABSOLUTE | Mode::NO_SUB_DIR | Mode::ENDS_WITH, Some(0)),
                vec![unspecified("b")],
                11
            ),
        ]
    );
}

#[test]
fn line_endings_can_be_windows_or_unix() {
    assert_eq!(
        try_lines("unix\nwindows\r\nlast").unwrap(),
        vec![
            (pattern(r"unix", Mode::NO_SUB_DIR, None), vec![], 1),
            (pattern(r"windows", Mode::NO_SUB_DIR, None), vec![], 2),
            (pattern(r"last", Mode::NO_SUB_DIR, None), vec![], 3)
        ]
    );
}

#[test]
fn comment_lines_are_ignored_as_well_as_empty_ones() {
    assert!(gix_attributes::parse(b"# hello world").next().is_none());
    assert!(gix_attributes::parse(b"# \"hello world\"").next().is_none());
    assert!(
        gix_attributes::parse(b" \t\r# \"hello world\"").next().is_none(),
        "also behind leading whitespace"
    );
    assert!(gix_attributes::parse(b"\n\r\n\t\t   \n").next().is_none());
}

#[test]
fn leading_whitespace_is_ignored() {
    assert_eq!(line(" \r\tp"), (pattern(r"p", Mode::NO_SUB_DIR, None), vec![], 1));
    assert_eq!(line(" \r\t\"p\""), (pattern(r"p", Mode::NO_SUB_DIR, None), vec![], 1));
}

#[test]
fn quotes_separate_attributes_even_without_whitespace() {
    assert_eq!(
        line(r#""path"a b"#),
        (pattern(r"path", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1)
    );
}

#[test]
fn comment_can_be_escaped_like_gitignore_or_quoted() {
    assert_eq!(
        line(r"\#hello"),
        (pattern(r"#hello", Mode::NO_SUB_DIR, None), vec![], 1),
        "undocumented, but definitely works"
    );
    assert_eq!(
        line("\"# hello\""),
        (pattern(r"# hello", Mode::NO_SUB_DIR, None), vec![], 1)
    );
}

#[test]
fn exclamation_marks_must_be_escaped_or_error_unlike_gitignore() {
    assert_eq!(
        line(r"\!hello"),
        (pattern(r"!hello", Mode::NO_SUB_DIR, None), vec![], 1)
    );
    assert!(matches!(
        try_line(r"!hello"),
        Err(parse::Error::PatternNegation { line_number: 1, .. })
    ));
    assert!(lenient_lines(r#"!hello"#).is_empty());
    assert!(
        matches!(
            try_line(r#""!hello""#),
            Err(parse::Error::PatternNegation { line_number: 1, .. }),
        ),
        "even in quotes they trigger…"
    );
    assert!(lenient_lines(r#""!hello""#).is_empty());
    assert_eq!(
        line(r#""\\!hello""#),
        (pattern(r"!hello", Mode::NO_SUB_DIR, None), vec![], 1),
        "…and must be double-escaped, once to get through quote, then to get through parse ignore line"
    );
}

#[test]
fn invalid_escapes_in_quotes_are_an_error() {
    assert!(matches!(try_line(r#""\!hello""#), Err(parse::Error::Unquote(_))));
    assert!(lenient_lines(r#""\!hello""#).is_empty());
}

#[test]
fn custom_macros_can_be_differentiated() {
    let output = line(r#"[attr]foo bar -baz"#);
    match output.0 {
        parse::Kind::Pattern(_) => unreachable!(),
        parse::Kind::Macro(name) => {
            assert_eq!(
                (name.as_str(), output.1, output.2),
                (r"foo", vec![set("bar"), unset("baz")], 1)
            );
        }
    }

    let output = line(r#""[attr]foo" bar -baz"#);
    match output.0 {
        parse::Kind::Pattern(_) => unreachable!(),
        parse::Kind::Macro(name) => {
            assert_eq!(
                (name.as_str(), output.1, output.2),
                (r"foo", vec![set("bar"), unset("baz")], 1),
                "it works after unquoting even, making it harder to denote a file name with [attr] prefix"
            );
        }
    }
}

#[test]
fn parsing_continues_even_in_the_face_of_invalid_lines_when_using_leniency() {
    assert_eq!(
        lenient_lines("[attr]-bad\np good\n[attr]-bad\np good2"),
        vec![
            (pattern(r"p", Mode::NO_SUB_DIR, None), vec![set("good")], 2),
            (pattern(r"p", Mode::NO_SUB_DIR, None), vec![set("good2")], 4),
        ]
    );
}

#[test]
fn macros_can_be_empty() {
    let output = line(r#"[attr]macro"#);
    match output.0 {
        parse::Kind::Pattern(_) => unreachable!(),
        parse::Kind::Macro(name) => {
            assert_eq!((name.as_str(), output.1, output.2), (r"macro", vec![], 1));
        }
    }
}

#[test]
fn custom_macros_must_be_valid_attribute_names() {
    assert!(matches!(
        try_line(r"[attr]-prefixdash"),
        Err(parse::Error::MacroName { line_number: 1, .. })
    ));
    assert!(lenient_lines(r"[attr]-prefixdash").is_empty());
    assert!(matches!(
        try_line(r"[attr]!exclamation"),
        Err(parse::Error::MacroName { line_number: 1, .. })
    ));
    assert!(matches!(
        try_line(r"[attr]assignment=value"),
        Err(parse::Error::MacroName { line_number: 1, .. })
    ));
    assert!(matches!(
        try_line(r"[attr]你好"),
        Err(parse::Error::MacroName { line_number: 1, .. })
    ));
    assert!(lenient_lines(r"[attr]你好").is_empty());
}

#[test]
fn attribute_names_must_not_begin_with_dash_and_must_be_ascii_only() {
    assert!(matches!(
        try_line(r"p !-a"),
        Err(parse::Error::AttributeName { line_number: 1, .. })
    ));
    assert!(lenient_lines(r"p !-a").is_empty());
    assert!(
        matches!(
            try_line(r#"p !!a"#),
            Err(parse::Error::AttributeName { line_number: 1, .. })
        ),
        "exclamation marks aren't allowed either"
    );
    assert!(lenient_lines(r#"p !!a"#).is_empty());
    assert!(
        matches!(
            try_line(r#"p 你好"#),
            Err(parse::Error::AttributeName { line_number: 1, .. })
        ),
        "nor is utf-8 encoded characters - gitoxide could consider to relax this when established"
    );
    assert!(lenient_lines(r#"p 你好"#).is_empty());
}

#[test]
fn attributes_are_parsed_behind_various_whitespace_characters() {
    assert_eq!(
        line(r#"p a b"#),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind space"
    );
    assert_eq!(
        line(r#""p" a b"#),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind space"
    );
    assert_eq!(
        line("p\ta\tb"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind tab"
    );
    assert_eq!(
        line("\"p\"\ta\tb"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind tab"
    );
    assert_eq!(
        line("p \t a \t b"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind a mix of space and tab"
    );
    assert_eq!(
        line("\"p\" \t a \t b"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a"), set("b")], 1),
        "behind a mix of space and tab"
    );
}

#[test]
fn attributes_come_in_different_flavors_due_to_prefixes() {
    assert_eq!(
        line(r#"p set -unset !unspecified -set"#),
        (
            pattern("p", Mode::NO_SUB_DIR, None),
            vec![set("set"), unset("unset"), unspecified("unspecified"), unset("set")],
            1
        ),
        "the parser doesn't care about double-mentions either"
    );
}

#[test]
fn attributes_can_have_values() {
    assert_eq!(
        line(r#"p a=one b=2 c=你好 "#),
        (
            pattern("p", Mode::NO_SUB_DIR, None),
            vec![value("a", "one"), value("b", "2"), value("c", "你好")],
            1
        ),
        "only non-whitespace ascii values are allowed, no escaping or anything fancy is possible there"
    );
}

#[test]
fn attributes_can_have_illformed_utf8() {
    assert_eq!(
        byte_line(b"p a=one b=\xC3\x28\x41 c=d "),
        (
            pattern("p", Mode::NO_SUB_DIR, None),
            vec![value("a", "one"), byte_value("b", b"\xC3\x28\x41"), value("c", "d")],
            1
        ),
        "illformed UTF8 is fully supported"
    );
}

#[test]
fn attributes_see_state_adjustments_over_value_assignments() {
    assert_eq!(
        line(r#"p set -unset=a !unspecified=b"#),
        (
            pattern("p", Mode::NO_SUB_DIR, None),
            vec![set("set"), unset("unset"), unspecified("unspecified")],
            1
        )
    );
}

#[test]
fn whitespace_around_patterns_can_be_quoted() {
    assert_eq!(
        line("\" p \" a \r\t"),
        (pattern(" p ", Mode::NO_SUB_DIR, None), vec![set("a")], 1),
    );
}

#[test]
fn trailing_whitespace_in_attributes_is_ignored() {
    assert_eq!(
        line("p a \r\t"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a")], 1),
    );
    assert_eq!(
        line("\"p\" a \r\t"),
        (pattern("p", Mode::NO_SUB_DIR, None), vec![set("a")], 1),
    );
}

type ExpandedAttribute<'a> = (parse::Kind, Vec<(BString, gix_attributes::StateRef<'a>)>, usize);

fn set(attr: &str) -> (BString, StateRef<'_>) {
    (attr.into(), StateRef::Set)
}

fn unset(attr: &str) -> (BString, StateRef<'_>) {
    (attr.into(), StateRef::Unset)
}

fn unspecified(attr: &str) -> (BString, StateRef<'_>) {
    (attr.into(), StateRef::Unspecified)
}

fn value<'b>(attr: &str, value: &'b str) -> (BString, StateRef<'b>) {
    (attr.into(), StateRef::Value(value.into()))
}

fn byte_value<'b>(attr: &str, value: &'b [u8]) -> (BString, StateRef<'b>) {
    (attr.into(), StateRef::Value(ValueRef::from_bytes(value)))
}

fn pattern(name: &str, flags: gix_glob::pattern::Mode, first_wildcard_pos: Option<usize>) -> parse::Kind {
    parse::Kind::Pattern(gix_glob::Pattern {
        text: name.into(),
        mode: flags,
        first_wildcard_pos,
    })
}

fn try_line(input: &str) -> Result<ExpandedAttribute<'_>, parse::Error> {
    let mut lines = gix_attributes::parse(input.as_bytes());
    let res = expand(lines.next().unwrap())?;
    assert!(lines.next().is_none(), "expected only one line");
    Ok(res)
}

fn line(input: &str) -> ExpandedAttribute<'_> {
    try_line(input).unwrap()
}

fn byte_line(input: &[u8]) -> ExpandedAttribute<'_> {
    try_byte_line(input).unwrap()
}

fn try_byte_line(input: &[u8]) -> Result<ExpandedAttribute<'_>, parse::Error> {
    let mut lines = gix_attributes::parse(input);
    let res = expand(lines.next().unwrap())?;
    assert!(lines.next().is_none(), "expected only one line");
    Ok(res)
}

fn lenient_lines(input: &str) -> Vec<ExpandedAttribute<'_>> {
    gix_attributes::parse(input.as_bytes())
        .map(expand)
        .filter_map(Result::ok)
        .collect()
}

fn try_lines(input: &str) -> Result<Vec<ExpandedAttribute<'_>>, parse::Error> {
    gix_attributes::parse(input.as_bytes()).map(expand).collect()
}

fn expand(
    input: Result<(parse::Kind, parse::Iter<'_>, usize), parse::Error>,
) -> Result<ExpandedAttribute<'_>, parse::Error> {
    let (pattern, attrs, line_no) = input?;
    let attrs = attrs
        .map(|r| r.map(|attr| (attr.name.as_str().into(), attr.state)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| parse::Error::AttributeName {
            attribute: e.attribute,
            line_number: line_no,
        })?;
    Ok((pattern, attrs, line_no))
}
