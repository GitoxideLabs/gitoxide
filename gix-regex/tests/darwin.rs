use gix_error::TestResult;
use gix_regex::{Dialect, PosixRegex, PosixRegexBuilder};

fn regex(pattern: &[u8], extended: bool) -> Result<PosixRegex<'static>, gix_regex::compile::Error> {
    PosixRegexBuilder::new(pattern)
        .dialect(Dialect::Darwin)
        .with_default_classes()
        .extended(extended)
        .compile()
}

#[test]
fn extended_syntax_rejects_empty_alternatives_and_nested_quantifiers() {
    for pattern in [b"a{2}{3}".as_slice(), b"a{2}*", b"a*+", b"(a+?)b", b"(a|)b", b"(|a)b"] {
        assert!(regex(pattern, true).is_err(), "Darwin rejects {pattern:?}");
    }
}

#[test]
fn extended_escapes_and_nonnumeric_intervals_are_literals() -> TestResult {
    for (pattern, input) in [
        (br"\bword\b".as_slice(), b"bwordb".as_slice()),
        (br"\Boo\B", b"BooB"),
        (br"\W+", b"WWW"),
        (br"\w+", b"www"),
        (br"\`word\'", b"`word'"),
        (br"\<word\>", b"<word>"),
        (br"(a)\10", b"a10"),
        (br"(a)\2", b"a2"),
        (br"(\1)", b"1"),
        (b"a{", b"a{"),
        (b"a{}", b"a{}"),
        (b"a{foo}", b"a{foo}"),
        (b"a{,1}", b"a{,1}"),
    ] {
        let captures = regex(pattern, true)?
            .matches_exact(input)?
            .expect("literal expression matches");
        assert_eq!(
            captures[0],
            Some((0, input.len())),
            "Darwin literal interpretation of {pattern:?}"
        );
    }
    Ok(())
}

#[test]
fn enhanced_basic_syntax_keeps_its_own_quantifier_and_anchor_rules() -> TestResult {
    for (pattern, input) in [
        (br"a\+\?".as_slice(), b"a?".as_slice()),
        (br"a\{2\}\+", b"aa+"),
        (br"a$\|b", b"a$"),
        (br"\`a\'", b"`a'"),
        (br"\bword\b", b"word"),
        (br"\(a\)\10", b"aa0"),
    ] {
        let captures = regex(pattern, false)?
            .matches_exact(input)?
            .expect("enhanced BRE matches");
        assert_eq!(
            captures[0],
            Some((0, input.len())),
            "Darwin BRE interpretation of {pattern:?}"
        );
    }
    for pattern in [br"a\{,1\}".as_slice(), br"\(\1\)", br"\(a\)\2", br"a**", br"a*\{2\}"] {
        assert!(regex(pattern, false).is_err(), "Darwin rejects BRE {pattern:?}");
    }
    Ok(())
}

#[test]
fn group_alternatives_choose_the_longest_match_before_repeating() -> TestResult {
    for pattern in [b"(a|aa)*".as_slice(), b"(aa|a)*"] {
        for (input, expected) in [(b"aa".as_slice(), (0, 2)), (b"aaa", (2, 3)), (b"aaaa", (2, 4))] {
            let captures = regex(pattern, true)?
                .matches_exact(input)?
                .expect("repeated group matches");
            assert_eq!(
                captures[1],
                Some(expected),
                "Darwin captures for {pattern:?} on {input:?}"
            );
        }
    }
    let captures = regex(b"(a|ab)(b*)", true)?
        .matches_exact(b"abb")?
        .expect("groups match");
    assert_eq!(
        captures[1],
        Some((0, 2)),
        "the longer first group takes priority over its following sibling"
    );
    Ok(())
}

#[test]
fn empty_groups_are_valid_but_empty_expressions_and_trailing_alternatives_are_not() -> TestResult {
    for extended in [false, true] {
        assert!(
            regex(b"", extended).is_err(),
            "Darwin rejects an empty top-level expression"
        );
    }
    for (pattern, extended) in [(b"()".as_slice(), true), (br"\(\)", false)] {
        assert!(
            regex(pattern, extended)?.matches_exact(b"anything")?.is_some(),
            "empty groups match"
        );
    }
    for (pattern, extended) in [(b"a|".as_slice(), true), (br"a\|", false), (br"\(a\|\)", false)] {
        assert!(
            regex(pattern, extended).is_err(),
            "trailing empty alternative is invalid"
        );
    }
    let capture = regex(br"\(\|a\)", false)?
        .matches_exact(b"|a")?
        .expect("leading BRE alternation is literal");
    assert_eq!(capture[1], Some((0, 2)));
    for pattern in [br"a*\+".as_slice(), br"a\+\+", br"a\?\+"] {
        assert!(
            regex(pattern, false).is_err(),
            "escaped plus is literal only after an interval"
        );
    }
    Ok(())
}
