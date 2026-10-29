use gix_diff::blob::hunk_header::Matcher;
use gix_error::TestResult;

#[test]
fn default_matches_identifiers_at_the_start_of_a_line() {
    let matcher = Matcher::default();
    for line in [b"function() {\n".as_slice(), b"_private\r\n", b"$variable\t "] {
        assert_eq!(
            matcher.find(line),
            Some(line.trim_ascii_end()),
            "the default recognizes an identifier and removes trailing whitespace"
        );
    }
    for line in [b"".as_slice(), b"    function()", b"42", b"\xffname"] {
        assert_eq!(
            matcher.find(line),
            None,
            "the default requires an ASCII identifier at column zero"
        );
    }
}

#[test]
fn regexes_are_ordered_and_select_the_first_capture() -> TestResult {
    let matcher = Matcher::from_regex(
        r"!^skip\b
^prefix (.*)
^other.*",
    )?;
    assert_eq!(matcher.find(b"skip this"), None, "a negative match rejects the line");
    assert_eq!(
        matcher.find(b"prefix declaration\t \r\n"),
        Some(b"declaration".as_slice()),
        "the first capture excludes the prefix and trailing whitespace"
    );
    assert_eq!(
        matcher.find(b"other declaration"),
        Some(b"other declaration".as_slice()),
        "without captures the complete match is returned"
    );
    let matcher = Matcher::from_regex(
        "^prefix.*
!^prefix
^other",
    )?;
    assert_eq!(
        matcher.find(b"prefix declaration"),
        Some(b"prefix declaration".as_slice()),
        "a later negative expression does not override the first positive match"
    );
    Ok(())
}

#[test]
fn regexes_match_bytes_and_strip_only_line_endings_before_matching() -> TestResult {
    let matcher = Matcher::from_regex("^([^ ]+)$")?;
    assert_eq!(
        matcher.find(b"name\xff\r\n"),
        Some(b"name\xff".as_slice()),
        "invalid UTF-8 remains byte oriented"
    );
    assert_eq!(
        matcher.find(b"name \n"),
        None,
        "trailing spaces participate in matching"
    );
    assert_eq!(
        matcher.find(b"name\r"),
        Some(b"name".as_slice()),
        "a lone CR participates in matching and is then trimmed"
    );
    let matcher = Matcher::from_regex("^(optional)?fallback$")?;
    assert_eq!(
        matcher.find(b"fallback"),
        Some(b"fallback".as_slice()),
        "an unmatched first capture falls back to the whole match"
    );
    Ok(())
}

#[test]
fn invalid_regexes_and_a_final_negative_expression_are_rejected() {
    assert!(
        Matcher::from_regex("[").is_err(),
        "invalid expressions produce a diagnostic"
    );
    assert!(
        Matcher::from_regex(
            "^positive
!negative"
        )
        .is_err(),
        "the final expression must be positive"
    );
}

#[test]
fn matching_lines_keeps_line_numbers_and_borrows_source_bytes() -> TestResult {
    let matcher = Matcher::from_regex("^fn .*")?;
    let source = b"fn first() {
  body
}
fn second() {
";
    let lines: Vec<_> = matcher.lines(source).collect();
    assert_eq!(
        lines,
        [(0, b"fn first() {".as_slice()), (3, b"fn second() {".as_slice())],
        "source positions are zero based and no artificial line is added after the final newline"
    );
    assert_eq!(Matcher::default().lines(b"").count(), 0, "an empty input has no lines");
    Ok(())
}

#[test]
fn default_and_shared_regex_syntax_agree_with_git_hunk_headers() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_hunk_header_repo.sh")?;
    for case in std::fs::read_dir(fixture.join("cases"))? {
        let case = case?.path();
        let pattern_path = case.join("pattern");
        let pattern = pattern_path
            .exists()
            .then(|| std::fs::read_to_string(pattern_path))
            .transpose()?;
        let matcher = pattern
            .as_deref()
            .map(Matcher::from_regex)
            .transpose()?
            .unwrap_or_default();
        let line = std::fs::read(case.join("source"))?;
        let baseline = std::fs::read(case.join("baseline.diff"))?;
        let git = baseline
            .split(|byte| *byte == b'\n')
            .find(|line| line.starts_with(b"@@ "))
            .ok_or("missing Git hunk")?;
        let header_start = git
            .windows(3)
            .position(|window| window == b" @@")
            .ok_or("missing header terminator")?
            + 3;
        let expected = git[header_start..].strip_prefix(b" ");
        assert_eq!(
            matcher.find(&line),
            expected,
            "matching agrees with Git for case {} and pattern {pattern:?}",
            case.display()
        );
    }
    Ok(())
}
