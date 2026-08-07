use gix_refspec::parse::{Error, Operation};

use crate::parse::try_parse;

#[test]
fn empty() {
    assert!(matches!(try_parse("", Operation::Push).unwrap_err(), Error::Empty));
}

#[test]
fn empty_component() {
    assert!(matches!(
        try_parse("refs/heads/test:refs/remotes//test", Operation::Fetch).unwrap_err(),
        Error::ReferenceName(gix_validate::reference::name::Error::RepeatedSlash)
    ));
}

#[test]
fn whitespace() {
    assert!(matches!(
        try_parse("refs/heads/test:refs/remotes/ /test", Operation::Fetch).unwrap_err(),
        Error::ReferenceName(gix_validate::reference::name::Error::InvalidByte { .. })
    ));
}

#[test]
fn destination_cannot_be_a_lone_at_sign() {
    for op in [Operation::Fetch, Operation::Push] {
        assert!(
            matches!(
                try_parse("HEAD:@", op).expect_err("a lone '@' is not a valid destination"),
                Error::ReferenceName(gix_validate::reference::name::Error::Reserved { name }) if name == "@"
            ),
            "{op:?} validates refspec destinations"
        );
    }
}

#[test]
fn patterns_may_contain_only_one_asterisk() {
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a/*/c/*", "a/*/c/*:x/*/y/*", "a**:**b", "+:**/"] {
            assert!(matches!(
                try_parse(spec, op).unwrap_err(),
                Error::PatternUnsupported { .. }
            ));
        }
    }

    assert!(matches!(
        try_parse("^*/*", Operation::Fetch).unwrap_err(),
        Error::PatternUnsupported { .. }
    ));
    // Negative refspec patterns follow Git's single-asterisk refspec-pattern rule.
    for op in [Operation::Fetch, Operation::Push] {
        assert!(matches!(
            try_parse("^refs/heads/qa/*/*", op).unwrap_err(),
            Error::PatternUnsupported { .. }
        ));
        for spec in [
            "^refs/heads/a*?",
            "^refs/heads/a[bc]*",
            "^refs/heads/*..bad",
            "^refs/heads/*/",
        ] {
            assert!(
                matches!(try_parse(spec, op).unwrap_err(), Error::ReferenceName(_)),
                "{spec}"
            );
        }
    }
}

#[test]
fn one_sided_push_patterns_still_use_refspec_pattern_syntax() {
    for spec in ["refs/heads/[ab]*", "refs/heads/a?*", "refs/heads/*..bad"] {
        assert!(
            matches!(try_parse(spec, Operation::Push).unwrap_err(), Error::ReferenceName(_)),
            "{spec} contains syntax Git refspec patterns do not support"
        );
    }
}

#[test]
fn both_sides_need_pattern_if_one_uses_it() {
    // For two-sided refspecs, both sides still need patterns if one uses it
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a*:b/c", "a:b/*"] {
            assert!(
                matches!(try_parse(spec, op).unwrap_err(), Error::PatternUnbalanced),
                "{}",
                spec
            );
        }
    }

    assert!(matches!(
        try_parse("refs/*/a", Operation::Fetch).unwrap_err(),
        Error::PatternUnbalanced
    ));
}

#[test]
fn push_to_empty() {
    assert!(matches!(
        try_parse("HEAD:", Operation::Push).unwrap_err(),
        Error::PushToEmpty
    ));
}

#[test]
fn fuzzed() {
    let input =
        include_bytes!("../../fixtures/fuzzed/clusterfuzz-testcase-minimized-gix-refspec-parse-4658733962887168");
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Fetch).unwrap_err());
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Push).unwrap_err());
}
