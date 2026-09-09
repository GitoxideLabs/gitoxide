use gix_refspec::{Instruction, instruction::Fetch, parse::Operation};

use crate::parse::{assert_parse, assert_reference_error, assert_unsupported_pattern, assert_validation, b};

#[test]
fn revspecs_are_disallowed() {
    for spec in ["main~1", "^@^{}", "HEAD:main~1"] {
        assert_reference_error(spec, Operation::Fetch);
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
    assert_validation("^", Operation::Fetch, "Negative specs must not be empty");
}

#[test]
fn negative_must_not_be_object_hash() {
    assert_validation(
        "^e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        Operation::Fetch,
        "Negative specs must not be object hashes",
    );
}

#[test]
fn negative_with_destination() {
    for spec in ["^a:b", "^a:", "^:", "^:b"] {
        assert_validation(
            spec,
            Operation::Fetch,
            "Negative refspecs cannot have destinations as they exclude sources",
        );
    }
}

#[test]
fn exclude() {
    assert_parse("^a", Instruction::Fetch(Fetch::Exclude { src: b("a") }));
    assert_parse("^a*", Instruction::Fetch(Fetch::Exclude { src: b("a*") }));
    assert_parse(
        "^refs/heads/a",
        Instruction::Fetch(Fetch::Exclude { src: b("refs/heads/a") }),
    );
    assert_parse(
        "^refs/heads/*-deploy",
        Instruction::Fetch(Fetch::Exclude {
            src: b("refs/heads/*-deploy"),
        }),
    );
    assert_parse(
        "^refs/tags/*-deploy",
        Instruction::Fetch(Fetch::Exclude {
            src: b("refs/tags/*-deploy"),
        }),
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
fn glob_patterns_need_a_destination() {
    for spec in ["refs/heads/*", "refs/heads/*:", ":refs/heads/*"] {
        assert_validation(
            spec,
            Operation::Fetch,
            "Both sides of a two-sided specification need a pattern, like 'a/*:b/*'",
        );
    }
}

#[test]
fn patterns_with_multiple_asterisks_are_rejected() {
    for spec in [
        "refs/*/foo/*:refs/remotes/origin/*",
        "refs/*/*:refs/remotes/*",
        "a/*/c/*:b/*",
    ] {
        assert_unsupported_pattern(spec, Operation::Fetch);
    }
}
