use crate::parse::{assert_parse, assert_reference_error, assert_validation, b};
use gix_refspec::{Instruction, instruction::Push, parse::Operation};

#[test]
fn negative_must_not_be_empty() {
    assert_validation("^", Operation::Push, "Negative specs must not be empty");
}

#[test]
fn negative_must_not_be_object_hash() {
    assert_validation(
        "^e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
        Operation::Push,
        "Negative specs must not be object hashes",
    );
}

#[test]
fn negative_with_destination() {
    for spec in ["^a:b", "^a:", "^:", "^:b"] {
        assert_validation(
            spec,
            Operation::Push,
            "Negative refspecs cannot have destinations as they exclude sources",
        );
    }
}

#[test]
fn exclude() {
    assert_parse("^a", Instruction::Push(Push::Exclude { src: b("a") }));
    assert_parse("^a*", Instruction::Push(Push::Exclude { src: b("a*") }));
    assert_parse(
        "^refs/heads/a",
        Instruction::Push(Push::Exclude { src: b("refs/heads/a") }),
    );
    assert_parse(
        "^refs/heads/*-deploy",
        Instruction::Push(Push::Exclude {
            src: b("refs/heads/*-deploy"),
        }),
    );
}

#[test]
fn one_sided_pattern_matches_the_same_remote_refs() {
    assert_parse(
        "refs/heads/*",
        Instruction::Push(Push::Matching {
            src: b("refs/heads/*"),
            dst: b("refs/heads/*"),
            allow_non_fast_forward: false,
        }),
    );
}

#[test]
fn revspecs_with_ref_name_destination() {
    assert_parse(
        "main~1:b",
        Instruction::Push(Push::Matching {
            src: b("main~1"),
            dst: b("b"),
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+main~1:b",
        Instruction::Push(Push::Matching {
            src: b("main~1"),
            dst: b("b"),
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn destinations_must_be_ref_names() {
    assert_reference_error("a~1:b~1", Operation::Push);
}

#[test]
fn single_refs_must_be_refnames() {
    assert_reference_error("a~1", Operation::Push);
}

#[test]
fn ampersand_is_resolved_to_head() {
    assert_parse(
        "@",
        Instruction::Push(Push::Matching {
            src: b("HEAD"),
            dst: b("HEAD"),
            allow_non_fast_forward: false,
        }),
    );

    assert_parse(
        "+@",
        Instruction::Push(Push::Matching {
            src: b("HEAD"),
            dst: b("HEAD"),
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn lhs_colon_rhs_pushes_single_ref() {
    assert_parse(
        "a:b",
        Instruction::Push(Push::Matching {
            src: b("a"),
            dst: b("b"),
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+a:b",
        Instruction::Push(Push::Matching {
            src: b("a"),
            dst: b("b"),
            allow_non_fast_forward: true,
        }),
    );
    assert_parse(
        "a/*:b/*",
        Instruction::Push(Push::Matching {
            src: b("a/*"),
            dst: b("b/*"),
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+a/*:b/*",
        Instruction::Push(Push::Matching {
            src: b("a/*"),
            dst: b("b/*"),
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn colon_alone_is_for_pushing_matching_refs() {
    assert_parse(
        ":",
        Instruction::Push(Push::AllMatchingBranches {
            allow_non_fast_forward: false,
        }),
    );
    assert_parse(
        "+:",
        Instruction::Push(Push::AllMatchingBranches {
            allow_non_fast_forward: true,
        }),
    );
}

#[test]
fn delete() {
    assert_parse(":a", Instruction::Push(Push::Delete { ref_or_pattern: b("a") }));
    assert_parse("+:a", Instruction::Push(Push::Delete { ref_or_pattern: b("a") }));

    for spec in [":refs/heads/*", "+:refs/heads/*"] {
        assert_validation(
            spec,
            Operation::Push,
            "Both sides of a two-sided specification need a pattern, like 'a/*:b/*'",
        );
    }
}
