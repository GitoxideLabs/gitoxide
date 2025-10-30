use gix_refspec::{
    instruction::Fetch,
    parse::{Error, Operation},
    Instruction,
};

use crate::parse::{assert_parse, b, try_parse};

#[test]
fn revspecs_are_disallowed() {
    for spec in ["main~1", "^@^{}", "HEAD:main~1"] {
        assert!(matches!(
            try_parse(spec, Operation::Fetch).unwrap_err(),
            Error::ReferenceName(_)
        ));
    }
}

#[test]
fn object_hash_as_source() {
    assert_parse(
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391:",
        Instruction::Fetch(Fetch::Only {
            src: b("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
        }),
    );
}

#[test]
fn object_hash_destination_are_valid_as_they_might_be_a_strange_partial_branch_name() {
    assert_parse(
        "a:e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("a"),
            dst: b("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
            allow_non_fast_forward: false,
        }),
    );
}

#[test]
fn negative_must_not_be_empty() {
    assert!(matches!(
        try_parse("^", Operation::Fetch).unwrap_err(),
        Error::NegativeEmpty
    ));
}

#[test]
fn negative_must_not_be_object_hash() {
    assert!(matches!(
        try_parse("^e69de29bb2d1d6434b8b29ae775ad8c2e48c5391", Operation::Fetch).unwrap_err(),
        Error::NegativeObjectHash
    ));
}

#[test]
fn negative_with_destination() {
    for spec in ["^a:b", "^a:", "^:", "^:b"] {
        assert!(matches!(
            try_parse(spec, Operation::Fetch).unwrap_err(),
            Error::NegativeWithDestination
        ));
    }
}

#[test]
fn exclude() {
    assert!(matches!(
        try_parse("^a", Operation::Fetch).unwrap_err(),
        Error::NegativePartialName
    ));
    assert!(matches!(
        try_parse("^a*", Operation::Fetch).unwrap_err(),
        Error::NegativeGlobPattern
    ));
    assert_parse(
        "^refs/heads/a",
        Instruction::Fetch(Fetch::Exclude { src: b("refs/heads/a") }),
    );
}

#[test]
fn ampersand_is_resolved_to_head() {
    assert_parse("@", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
    assert_parse("+@", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
    assert_parse("^@", Instruction::Fetch(Fetch::Exclude { src: b("HEAD") }));
}

#[test]
fn lhs_colon_empty_fetches_only() {
    assert_parse("src:", Instruction::Fetch(Fetch::Only { src: b("src") }));
    assert_parse("+src:", Instruction::Fetch(Fetch::Only { src: b("src") }));
}

#[test]
fn lhs_colon_rhs_updates_single_ref() {
    assert_parse(
        "a:b",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("a"),
            dst: b("b"),
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+a:b",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("a"),
            dst: b("b"),
            allow_non_fast_forward: true,
        }),
    );

    assert_parse(
        "a/*:b/*",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("a/*"),
            dst: b("b/*"),
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+a/*:b/*",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("a/*"),
            dst: b("b/*"),
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn empty_lhs_colon_rhs_fetches_head_to_destination() {
    assert_parse(
        ":a",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("HEAD"),
            dst: b("a"),
            allow_non_fast_forward: false,
        }),
    );

    assert_parse(
        "+:a",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("HEAD"),
            dst: b("a"),
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn colon_alone_is_for_fetching_head_into_fetchhead() {
    assert_parse(":", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
    assert_parse("+:", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
}

#[test]
fn ampersand_on_left_hand_side_is_head() {
    assert_parse("@:", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
    assert_parse(
        "@:HEAD",
        Instruction::Fetch(Fetch::AndUpdate {
            src: b("HEAD"),
            dst: b("HEAD"),
            allow_non_fast_forward: false,
        }),
    );
}

#[test]
fn empty_refspec_is_enough_for_fetching_head_into_fetchhead() {
    assert_parse("", Instruction::Fetch(Fetch::Only { src: b("HEAD") }));
}

#[test]
fn complex_glob_patterns_are_allowed_in_one_sided_refspecs() {
    // Complex patterns with multiple asterisks should work for one-sided refspecs
    assert_parse(
        "refs/*/foo/*",
        Instruction::Fetch(Fetch::Only {
            src: b("refs/*/foo/*"),
        }),
    );
    
    assert_parse(
        "+refs/heads/*/release/*",
        Instruction::Fetch(Fetch::Only {
            src: b("refs/heads/*/release/*"),
        }),
    );
    
    // Even more complex patterns
    assert_parse(
        "refs/*/*/branch",
        Instruction::Fetch(Fetch::Only {
            src: b("refs/*/*/branch"),
        }),
    );
}

#[test]
fn complex_glob_patterns_still_fail_for_two_sided_refspecs() {
    // Two-sided refspecs with complex patterns (multiple asterisks) should still fail
    for spec in [
        "refs/*/foo/*:refs/remotes/origin/*",
        "refs/*/*:refs/remotes/*",
        "a/*/c/*:b/*",
    ] {
        assert!(matches!(
            try_parse(spec, Operation::Fetch).unwrap_err(),
            Error::PatternUnsupported { .. }
        ));
    }
}
