use gix_refspec::parse::Operation;

use crate::parse::{assert_reference_error, assert_unsupported_pattern, assert_validation};

#[test]
fn empty() {
    insta::assert_debug_snapshot!(assert_validation("", Operation::Push), "empty", @"Empty refspecs are invalid");
}

#[test]
fn empty_component() {
    let err = assert_reference_error("refs/heads/test:refs/remotes//test", Operation::Fetch);
    insta::assert_debug_snapshot!(err, "empty component", @"
    Reference name cannot contain repeated slashes
    |
    └─ Reference name cannot contain repeated slashes
    ");
    assert!(matches!(
        err.downcast_any_ref::<gix_validate::reference::name::Error>(),
        Some(gix_validate::reference::name::Error::RepeatedSlash)
    ));
}

#[test]
fn whitespace() {
    let err = assert_reference_error("refs/heads/test:refs/remotes/ /test", Operation::Fetch);
    insta::assert_debug_snapshot!(err, "whitespace", @r#"
    Reference name contains invalid byte: " "
    |
    └─ Reference name contains invalid byte: " "
    "#);
    assert!(matches!(
        err.downcast_any_ref::<gix_validate::reference::name::Error>(),
        Some(gix_validate::reference::name::Error::InvalidByte { .. })
    ));
}

#[test]
fn destination_cannot_be_a_lone_at_sign() {
    let mut error_snapshots = Vec::new();
    for op in [Operation::Fetch, Operation::Push] {
        let err = assert_reference_error("HEAD:@", op);
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(
            matches!(
                err.downcast_any_ref::<gix_validate::reference::name::Error>(),
                Some(gix_validate::reference::name::Error::Reserved { name }) if name == "@"
            ),
            "{op:?} validates refspec destinations"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "destination cannot be a lone at sign", @r#"
    [
        Reference name is reserved and cannot be used: "@"
        |
        └─ Reference name is reserved and cannot be used: "@",
        Reference name is reserved and cannot be used: "@"
        |
        └─ Reference name is reserved and cannot be used: "@",
    ]
    "#);
}

#[test]
fn patterns_may_contain_only_one_asterisk() {
    let mut diagnostics = Vec::new();
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a/*/c/*", "a/*/c/*:x/*/y/*", "a**:**b", "+:**/"] {
            diagnostics.push(gix_testtools::redact_debug_snapshot(
                &assert_unsupported_pattern(spec, op),
                &[],
            ));
        }
    }

    insta::assert_debug_snapshot!(assert_unsupported_pattern("^*/*", Operation::Fetch), "patterns may contain only one asterisk", @r#"refspec patterns may only contain a single '*' character, "input"="*/*""#);
    // Negative refspec patterns follow Git's single-asterisk refspec-pattern rule.
    for op in [Operation::Fetch, Operation::Push] {
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            &assert_unsupported_pattern("^refs/heads/qa/*/*", op),
            &[],
        ));
        for spec in [
            "^refs/heads/a*?",
            "^refs/heads/a[bc]*",
            "^refs/heads/*..bad",
            "^refs/heads/*/",
        ] {
            diagnostics.push(gix_testtools::redact_debug_snapshot(
                &assert_reference_error(spec, op),
                &[],
            ));
        }
    }
    insta::assert_debug_snapshot!(diagnostics, "patterns may contain only one asterisk", @r#"
    [
        refspec patterns may only contain a single '*' character, "input"="a/*/c/*",
        refspec patterns may only contain a single '*' character, "input"="a/*/c/*",
        refspec patterns may only contain a single '*' character, "input"="a**",
        refspec patterns may only contain a single '*' character, "input"="**/",
        refspec patterns may only contain a single '*' character, "input"="a/*/c/*",
        refspec patterns may only contain a single '*' character, "input"="a/*/c/*",
        refspec patterns may only contain a single '*' character, "input"="a**",
        refspec patterns may only contain a single '*' character, "input"="**/",
        refspec patterns may only contain a single '*' character, "input"="refs/heads/qa/*/*",
        Reference name contains invalid byte: "?"
        |
        └─ Reference name contains invalid byte: "?",
        Reference name contains invalid byte: "["
        |
        └─ Reference name contains invalid byte: "[",
        Reference name cannot contain repeated dots
        |
        └─ Reference name cannot contain repeated dots,
        Reference name cannot end with a slash
        |
        └─ Reference name cannot end with a slash,
        refspec patterns may only contain a single '*' character, "input"="refs/heads/qa/*/*",
        Reference name contains invalid byte: "?"
        |
        └─ Reference name contains invalid byte: "?",
        Reference name contains invalid byte: "["
        |
        └─ Reference name contains invalid byte: "[",
        Reference name cannot contain repeated dots
        |
        └─ Reference name cannot contain repeated dots,
        Reference name cannot end with a slash
        |
        └─ Reference name cannot end with a slash,
    ]
    "#);
}

#[test]
fn one_sided_push_patterns_still_use_refspec_pattern_syntax() {
    let mut diagnostics = Vec::new();
    for spec in ["refs/heads/[ab]*", "refs/heads/a?*", "refs/heads/*..bad"] {
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            &assert_reference_error(spec, Operation::Push),
            &[],
        ));
    }
    insta::assert_debug_snapshot!(diagnostics, "one sided push patterns still use refspec pattern syntax", @r#"
    [
        Reference name contains invalid byte: "["
        |
        └─ Reference name contains invalid byte: "[",
        Reference name contains invalid byte: "?"
        |
        └─ Reference name contains invalid byte: "?",
        Reference name cannot contain repeated dots
        |
        └─ Reference name cannot contain repeated dots,
    ]
    "#);
}

#[test]
fn both_sides_need_pattern_if_one_uses_it() {
    let mut diagnostics = Vec::new();
    // For two-sided refspecs, both sides still need patterns if one uses it
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a*:b/c", "a:b/*"] {
            diagnostics.push(gix_testtools::redact_debug_snapshot(&assert_validation(spec, op), &[]));
        }
    }

    insta::assert_debug_snapshot!(assert_validation("refs/*/a", Operation::Fetch), "both sides need pattern if one uses it", @"Both sides of a two-sided specification need a pattern, like 'a/*:b/*'");
    insta::assert_debug_snapshot!(diagnostics, "both sides need pattern if one uses it", @"
    [
        Both sides of a two-sided specification need a pattern, like 'a/*:b/*',
        Both sides of a two-sided specification need a pattern, like 'a/*:b/*',
        Both sides of a two-sided specification need a pattern, like 'a/*:b/*',
        Both sides of a two-sided specification need a pattern, like 'a/*:b/*',
    ]
    ");
}

#[test]
fn push_to_empty() {
    insta::assert_debug_snapshot!(assert_validation("HEAD:", Operation::Push), "push to empty", @"Cannot push into an empty destination");
}

#[test]
fn fuzzed() {
    let input =
        include_bytes!("../../fixtures/fuzzed/clusterfuzz-testcase-minimized-gix-refspec-parse-4658733962887168");
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Fetch).unwrap_err());
    drop(gix_refspec::parse(input.into(), gix_refspec::parse::Operation::Push).unwrap_err());
}
