use gix_error::TestResult;
use gix_regex::PosixRegexBuilder;

#[test]
fn groups_must_close_and_basic_closing_parentheses_need_an_open_group() -> TestResult {
    for (pattern, extended) in [
        (b"(unclosed".as_slice(), true),
        (br"\(unclosed", false),
        (br"closed\)", false),
        (b"((nested)".as_slice(), true),
    ] {
        assert!(
            PosixRegexBuilder::new(pattern).extended(extended).compile().is_err(),
            "unbalanced parentheses must not be silently accepted: {pattern:?}"
        );
    }
    let regex = PosixRegexBuilder::new(b"closed)literal").extended(true).compile()?;
    assert!(
        regex.matches_exact(b"closed)literal").is_some(),
        "the complete expression must be consumed"
    );
    Ok(())
}

#[test]
fn bracket_ranges_validate_order_and_accept_literal_opening_brackets() -> TestResult {
    for pattern in [
        b"[z-a]".as_slice(),
        b"[[:alpha:]-z]",
        b"[z-[:alpha:]]",
        b"[a-b-c]",
        b"[]",
    ] {
        assert!(
            PosixRegexBuilder::new(pattern)
                .with_default_classes()
                .compile()
                .is_err(),
            "invalid bracket ranges and unterminated sets are rejected: {pattern:?}"
        );
    }
    for (pattern, text) in [
        (b"[[]".as_slice(), b"[".as_slice()),
        (b"[a[]", b"["),
        (b"[\xff-\xff]", b"\xff"),
    ] {
        let regex = PosixRegexBuilder::new(pattern).compile()?;
        assert!(
            regex.matches_exact(text).is_some(),
            "literal brackets and maximum-valued bytes are supported"
        );
    }
    Ok(())
}

#[test]
fn backreferences_name_a_preceding_closed_group_and_use_one_digit() -> TestResult {
    for (pattern, extended) in [
        (br"\1".as_slice(), true),
        (br"(\1)", true),
        (br"\(\1\)", false),
        (br"(a)\2", true),
    ] {
        assert!(
            PosixRegexBuilder::new(pattern).extended(extended).compile().is_err(),
            "backreferences cannot refer to absent or still-open groups: {pattern:?}"
        );
    }
    for (pattern, extended) in [(br"(a)\10".as_slice(), true), (br"\(a\)\10", false)] {
        let regex = PosixRegexBuilder::new(pattern).extended(extended).compile()?;
        assert!(
            regex.matches_exact(b"aa0").is_some(),
            "only the first digit belongs to the backreference"
        );
    }
    Ok(())
}

#[test]
fn parser_acceptance_and_captures_agree_with_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_parser_cases.sh")?;
    for case in std::fs::read_dir(fixture.join("cases"))? {
        let case = case?.path();
        let pattern = std::fs::read(case.join("pattern"))?;
        let source = std::fs::read(case.join("source"))?;
        let extended = std::fs::read_to_string(case.join("syntax"))? == "xfuncname";
        let valid = std::fs::read_to_string(case.join("status"))? == "valid";
        let result = PosixRegexBuilder::new(&pattern)
            .with_default_classes()
            .extended(extended)
            .compile();
        assert_eq!(
            result.is_ok(),
            valid,
            "parser acceptance matches Git for {}",
            case.display()
        );
        let Ok(regex) = result else { continue };
        let captures = regex.matches(&source, Some(1));
        let header = captures
            .first()
            .and_then(|groups| groups.get(1).copied().flatten().or(groups[0]));
        let actual = header
            .map(|(start, end)| {
                let header = &source[start..end];
                let end = header
                    .iter()
                    .rposition(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                    .map_or(0, |at| at + 1);
                &header[..end]
            })
            .filter(|header| !header.is_empty());
        let baseline = std::fs::read(case.join("baseline.diff"))?;
        let hunk = baseline
            .split(|byte| *byte == b'\n')
            .find(|line| line.starts_with(b"@@ "))
            .ok_or("missing hunk in Git baseline")?;
        let end = hunk
            .windows(3)
            .position(|bytes| bytes == b" @@")
            .ok_or("missing Git header terminator")?
            + 3;
        let expected = hunk[end..].strip_prefix(b" ");
        assert_eq!(actual, expected, "captures match Git for {}", case.display());
    }
    Ok(())
}

#[test]
fn gnu_escapes_and_word_boundaries_use_ascii_byte_semantics() -> TestResult {
    for (pattern, input, expected) in [
        (br"\a\d\n\r\t\0".as_slice(), b"adnrt0".as_slice(), true),
        (br"\w+", b"_word1", true),
        (br"\W+", b"!@", true),
        (br"\bword\b", b"word", true),
        (br"\Boo\B", b"food", true),
        (br"\<!", b"!", false),
        (br"!\>", b"!", false),
        (br"\B!", b"!", true),
        (br"\`word\'", b"word", true),
    ] {
        let regex = PosixRegexBuilder::new(pattern).extended(true).compile()?;
        assert_eq!(
            !regex.matches(input, Some(1)).is_empty(),
            expected,
            "GNU escape matching for {pattern:?} on {input:?}"
        );
    }
    Ok(())
}

#[test]
fn basic_anchors_depend_on_their_position_in_each_alternative() -> TestResult {
    for (pattern, input) in [
        (b"a^b".as_slice(), b"a^b".as_slice()),
        (b"a$b", b"a$b"),
        (b"^^a", b"^a"),
        (br"a\|^b", b"b"),
        (br"a$\|b", b"a"),
        (br"\(^a$\)", b"a"),
    ] {
        assert!(
            PosixRegexBuilder::new(pattern)
                .compile()?
                .matches_exact(input)
                .is_some(),
            "BRE anchor position is significant for {pattern:?}"
        );
    }
    Ok(())
}

#[test]
fn case_folding_preserves_punctuation_and_applies_to_classes_and_backreferences() -> TestResult {
    for (pattern, input, expected) in [
        (b"@".as_slice(), b"`".as_slice(), false),
        (br"\[", b"{", false),
        (b"[[:upper:]]+", b"lower", true),
        (b"[[:lower:]]+", b"UPPER", true),
        (br"([a-z]+)\1", b"wordWORD", true),
    ] {
        let regex = PosixRegexBuilder::new(pattern)
            .with_default_classes()
            .extended(true)
            .compile()?
            .case_insensitive(true);
        assert_eq!(
            !regex.matches(input, Some(1)).is_empty(),
            expected,
            "case-insensitive matching for {pattern:?} on {input:?}"
        );
    }
    Ok(())
}
