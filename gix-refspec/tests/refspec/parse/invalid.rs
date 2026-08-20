use gix_refspec::parse::Operation;

use crate::parse::{assert_reference_error, assert_unsupported_pattern, assert_validation};

#[test]
fn empty() {
    assert_validation("", Operation::Push, "Empty refspecs are invalid");
}

#[test]
fn empty_component() {
    let err = assert_reference_error("refs/heads/test:refs/remotes//test", Operation::Fetch);
    assert!(matches!(
        err.downcast_any_ref::<gix_validate::reference::name::Error>(),
        Some(gix_validate::reference::name::Error::RepeatedSlash)
    ));
}

#[test]
fn whitespace() {
    let err = assert_reference_error("refs/heads/test:refs/remotes/ /test", Operation::Fetch);
    assert!(matches!(
        err.downcast_any_ref::<gix_validate::reference::name::Error>(),
        Some(gix_validate::reference::name::Error::InvalidByte { .. })
    ));
}

#[test]
fn destination_cannot_be_a_lone_at_sign() {
    for op in [Operation::Fetch, Operation::Push] {
        let err = assert_reference_error("HEAD:@", op);
        assert!(
            matches!(
                err.downcast_any_ref::<gix_validate::reference::name::Error>(),
                Some(gix_validate::reference::name::Error::Reserved { name }) if name == "@"
            ),
            "{op:?} validates refspec destinations"
        );
    }
}

#[test]
fn patterns_may_contain_only_one_asterisk() {
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a/*/c/*", "a/*/c/*:x/*/y/*", "a**:**b", "+:**/"] {
            assert_unsupported_pattern(spec, op);
        }
    }

    assert_unsupported_pattern("^*/*", Operation::Fetch);
    // Negative refspec patterns follow Git's single-asterisk refspec-pattern rule.
    for op in [Operation::Fetch, Operation::Push] {
        assert_unsupported_pattern("^refs/heads/qa/*/*", op);
        for spec in [
            "^refs/heads/a*?",
            "^refs/heads/a[bc]*",
            "^refs/heads/*..bad",
            "^refs/heads/*/",
        ] {
            assert_reference_error(spec, op);
        }
    }
}

#[test]
fn one_sided_push_patterns_still_use_refspec_pattern_syntax() {
    for spec in ["refs/heads/[ab]*", "refs/heads/a?*", "refs/heads/*..bad"] {
        assert_reference_error(spec, Operation::Push);
    }
}

#[test]
fn both_sides_need_pattern_if_one_uses_it() {
    // For two-sided refspecs, both sides still need patterns if one uses it
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a*:b/c", "a:b/*"] {
            assert_validation(
                spec,
                op,
                "Both sides of a two-sided specification need a pattern, like 'a/*:b/*'",
            );
        }
    }

    assert_validation(
        "refs/*/a",
        Operation::Fetch,
        "Both sides of a two-sided specification need a pattern, like 'a/*:b/*'",
    );
}

#[test]
fn push_to_empty() {
    assert_validation("HEAD:", Operation::Push, "Cannot push into an empty destination");
}

#[test]
fn fuzzed() {
    let input =
        include_bytes!("../../fixtures/fuzzed/clusterfuzz-testcase-minimized-gix-refspec-parse-4658733962887168");
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Fetch).unwrap_err());
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Push).unwrap_err());
}
