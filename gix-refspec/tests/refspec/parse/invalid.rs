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
        Error::ReferenceName(gix_validate::reference::name::Error::Tag(
            gix_validate::tag::name::Error::RepeatedSlash
        ))
    ));
}

#[test]
fn whitespace() {
    assert!(matches!(
        try_parse("refs/heads/test:refs/remotes/ /test", Operation::Fetch).unwrap_err(),
        Error::ReferenceName(gix_validate::reference::name::Error::Tag(_))
    ));
}

#[test]
fn complex_patterns_with_more_than_one_asterisk() {
    // For one-sided refspecs, complex patterns are now allowed
    for op in [Operation::Fetch, Operation::Push] {
        assert!(try_parse("a/*/c/*", op).is_ok());
    }
    
    // For two-sided refspecs, complex patterns should still fail
    for op in [Operation::Fetch, Operation::Push] {
        for spec in ["a/*/c/*:x/*/y/*", "a**:**b", "+:**/"] {
            assert!(matches!(
                try_parse(spec, op).unwrap_err(),
                Error::PatternUnsupported { .. }
            ));
        }
    }
    
    // Negative specs with multiple patterns still fail
    assert!(matches!(
        try_parse("^*/*", Operation::Fetch).unwrap_err(),
        Error::NegativeGlobPattern
    ));
}

#[test]
fn both_sides_need_pattern_if_one_uses_it() {
    // For two-sided refspecs, both sides still need patterns if one uses it
    for op in [Operation::Fetch, Operation::Push] {
        for spec in [":a/*", "+:a/*", "a*:b/c", "a:b/*"] {
            assert!(
                matches!(try_parse(spec, op).unwrap_err(), Error::PatternUnbalanced),
                "{}",
                spec
            );
        }
    }
    
    // One-sided refspecs with patterns are now allowed
    for op in [Operation::Fetch, Operation::Push] {
        assert!(try_parse("refs/*/a", op).is_ok());
    }
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
