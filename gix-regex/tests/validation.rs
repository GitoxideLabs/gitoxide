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
    for pattern in [b"[z-a]".as_slice(), b"[[:alpha:]-z]", b"[z-[:alpha:]]", b"[]"] {
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
