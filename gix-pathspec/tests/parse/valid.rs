use gix_attributes::State;
use gix_pathspec::{MagicSignature, SearchMode};

use crate::parse::{NormalizedPattern, check_against_baseline, check_valid_inputs};

#[test]
fn repeated_matcher_keywords() {
    let input = vec![
        (":(glob,glob)", pat_with_search_mode(SearchMode::PathAwareGlob)),
        (":(literal,literal)", pat_with_search_mode(SearchMode::Literal)),
        (":(top,top)", pat_with_sig(MagicSignature::TOP)),
        (":(icase,icase)", pat_with_sig(MagicSignature::ICASE)),
        (":(attr,attr)", pat_with_attrs(vec![])),
        (":!^(exclude,exclude)", pat_with_sig(MagicSignature::EXCLUDE)),
    ];

    check_valid_inputs(input);
}

#[test]
fn glob_negations_are_always_literal() {
    check_valid_inputs([("!a", pat_with_path("!a")), (r"\!a", pat_with_path(r"\!a"))]);
}

#[test]
fn literal_default_prevents_parsing() {
    let pattern = gix_pathspec::parse(
        ":".as_bytes(),
        gix_pathspec::Defaults {
            signature: MagicSignature::EXCLUDE,
            search_mode: SearchMode::PathAwareGlob,
            literal: true,
        },
    )
    .expect("valid");
    assert!(!pattern.is_nil());
    assert_eq!(pattern.path(), ":");
    assert!(matches!(pattern.search_mode, SearchMode::Literal));

    let input = ":(literal)f[o][o]";
    let pattern = gix_pathspec::parse(
        input.as_bytes(),
        gix_pathspec::Defaults {
            signature: MagicSignature::TOP,
            search_mode: SearchMode::Literal,
            literal: true,
        },
    )
    .expect("valid");
    assert_eq!(pattern.path(), input, "no parsing happens at all");
    assert!(matches!(pattern.search_mode, SearchMode::Literal));

    let pattern = gix_pathspec::parse(
        input.as_bytes(),
        gix_pathspec::Defaults {
            signature: MagicSignature::TOP,
            search_mode: SearchMode::Literal,
            literal: false,
        },
    )
    .expect("valid");
    assert_eq!(pattern.path(), "f[o][o]", "in literal default mode, we still parse");
    assert!(matches!(pattern.search_mode, SearchMode::Literal));
}

#[test]
fn there_is_no_pathspec_pathspec() {
    check_against_baseline(":");
    let pattern = gix_pathspec::parse(":".as_bytes(), Default::default()).expect("valid");
    assert!(pattern.is_nil());

    let actual: NormalizedPattern = pattern.into();
    assert_eq!(actual, pat_with_path(""));

    let pattern = gix_pathspec::parse(
        ":".as_bytes(),
        gix_pathspec::Defaults {
            signature: MagicSignature::EXCLUDE,
            search_mode: SearchMode::PathAwareGlob,
            literal: false,
        },
    )
    .expect("valid");
    assert!(pattern.is_nil());
}

#[test]
fn defaults_are_used() -> crate::Result {
    let defaults = gix_pathspec::Defaults {
        signature: MagicSignature::EXCLUDE,
        search_mode: SearchMode::Literal,
        literal: false,
    };
    let p = gix_pathspec::parse(".".as_bytes(), defaults)?;
    assert_eq!(p.path(), ".");
    assert_eq!(p.signature, defaults.signature);
    assert_eq!(p.search_mode, defaults.search_mode);
    assert!(p.attributes.is_empty());
    assert!(!p.is_nil());
    Ok(())
}

#[test]
fn literal_from_defaults_is_overridden_by_element_glob() -> crate::Result {
    let defaults = gix_pathspec::Defaults {
        search_mode: SearchMode::Literal,
        ..Default::default()
    };
    let p = gix_pathspec::parse(":(glob)*override".as_bytes(), defaults)?;
    assert_eq!(p.path(), "*override");
    assert_eq!(p.signature, MagicSignature::default());
    assert_eq!(p.search_mode, SearchMode::PathAwareGlob, "this is the element override");
    assert!(p.attributes.is_empty());
    assert!(!p.is_nil());
    Ok(())
}

#[test]
fn glob_from_defaults_is_overridden_by_element_glob() -> crate::Result {
    let defaults = gix_pathspec::Defaults {
        search_mode: SearchMode::PathAwareGlob,
        ..Default::default()
    };
    let p = gix_pathspec::parse(":(literal)*override".as_bytes(), defaults)?;
    assert_eq!(p.path(), "*override");
    assert_eq!(p.signature, MagicSignature::default());
    assert_eq!(p.search_mode, SearchMode::Literal, "this is the element override");
    assert!(p.attributes.is_empty());
    assert!(!p.is_nil());
    Ok(())
}

#[test]
fn empty_signatures() {
    let inputs = vec![
        (".", pat_with_path(".")),
        ("some/path", pat_with_path("some/path")),
        (":some/path", pat_with_path("some/path")),
        (":()some/path", pat_with_path("some/path")),
        ("::some/path", pat_with_path("some/path")),
        (":::some/path", pat_with_path(":some/path")),
        (":():some/path", pat_with_path(":some/path")),
    ];

    check_valid_inputs(inputs);
}

#[test]
fn whitespace_in_pathspec() {
    let inputs = vec![
        (" some/path", pat_with_path(" some/path")),
        ("some/ path", pat_with_path("some/ path")),
        ("some/path ", pat_with_path("some/path ")),
        (": some/path", pat_with_path(" some/path")),
        (": !some/path", pat_with_path(" !some/path")),
        (": :some/path", pat_with_path(" :some/path")),
        (": ()some/path", pat_with_path(" ()some/path")),
        (
            ":! some/path",
            pat_with_path_and_sig(" some/path", MagicSignature::EXCLUDE),
        ),
        (
            ":!!some/path",
            pat_with_path_and_sig("some/path", MagicSignature::EXCLUDE),
        ),
    ];

    check_valid_inputs(inputs);
}

#[test]
fn short_signatures() {
    let inputs = vec![
        (":/some/path", pat_with_path_and_sig("some/path", MagicSignature::TOP)),
        (
            ":^some/path",
            pat_with_path_and_sig("some/path", MagicSignature::EXCLUDE),
        ),
        (
            ":!some/path",
            pat_with_path_and_sig("some/path", MagicSignature::EXCLUDE),
        ),
        (
            ":/!some/path",
            pat_with_path_and_sig("some/path", MagicSignature::TOP | MagicSignature::EXCLUDE),
        ),
        (
            ":!/^/:some/path",
            pat_with_path_and_sig("some/path", MagicSignature::TOP | MagicSignature::EXCLUDE),
        ),
    ];

    check_valid_inputs(inputs);
}

#[test]
fn trailing_slash_is_turned_into_magic_signature_and_removed() {
    check_valid_inputs([
        ("a/b/", pat_with_path_and_sig("a/b", MagicSignature::MUST_BE_DIR)),
        ("a/", pat_with_path_and_sig("a", MagicSignature::MUST_BE_DIR)),
    ]);
}

#[test]
fn signatures_and_searchmodes() {
    let inputs = vec![
        (":(top)", pat_with_sig(MagicSignature::TOP)),
        (":(icase)", pat_with_sig(MagicSignature::ICASE)),
        (":(attr)", pat_with_path("")),
        (":(exclude)", pat_with_sig(MagicSignature::EXCLUDE)),
        (":(literal)", pat_with_search_mode(SearchMode::Literal)),
        (":(glob)", pat_with_search_mode(SearchMode::PathAwareGlob)),
        (
            ":(top,exclude)",
            pat_with_sig(MagicSignature::TOP | MagicSignature::EXCLUDE),
        ),
        (
            ":(icase,literal)",
            pat("", MagicSignature::ICASE, SearchMode::Literal, vec![]),
        ),
        (
            ":!(literal)some/*path",
            pat("some/*path", MagicSignature::EXCLUDE, SearchMode::Literal, vec![]),
        ),
        (
            ":(top,literal,icase,attr,exclude)some/path",
            pat(
                "some/path",
                MagicSignature::TOP | MagicSignature::EXCLUDE | MagicSignature::ICASE,
                SearchMode::Literal,
                vec![],
            ),
        ),
        (
            ":(top,glob,icase,attr,exclude)some/path",
            pat(
                "some/path",
                MagicSignature::TOP | MagicSignature::EXCLUDE | MagicSignature::ICASE,
                SearchMode::PathAwareGlob,
                vec![],
            ),
        ),
    ];

    check_valid_inputs(inputs);
}

#[test]
fn attributes_in_signature() {
    let inputs = vec![
        (":(attr:someAttr)", pat_with_attrs(vec![("someAttr", State::Set)])),
        (
            ":(attr:!someAttr)",
            pat_with_attrs(vec![("someAttr", State::Unspecified)]),
        ),
        (":(attr:-someAttr)", pat_with_attrs(vec![("someAttr", State::Unset)])),
        (
            ":(attr:someAttr=value)",
            pat_with_attrs(vec![("someAttr", State::Value("value".into()))]),
        ),
        (
            ":(attr:a=one b=)",
            pat_with_attrs(vec![("a", State::Value("one".into())), ("b", State::Value("".into()))]),
        ),
        (
            ":(attr:a= b=two)",
            pat_with_attrs(vec![("a", State::Value("".into())), ("b", State::Value("two".into()))]),
        ),
        (
            ":(attr:a=one b=two)",
            pat_with_attrs(vec![
                ("a", State::Value("one".into())),
                ("b", State::Value("two".into())),
            ]),
        ),
        (
            ":(attr:a=one   b=two)",
            pat_with_attrs(vec![
                ("a", State::Value("one".into())),
                ("b", State::Value("two".into())),
            ]),
        ),
        (
            ":(attr:someAttr anotherAttr)",
            pat_with_attrs(vec![("someAttr", State::Set), ("anotherAttr", State::Set)]),
        ),
    ];

    check_valid_inputs(inputs);
}

#[test]
fn attributes_with_escape_chars_in_state_values() {
    let inputs = vec![
        (
            r":(attr:v=one\-)",
            pat_with_attrs(vec![("v", State::Value(r"one-".into()))]),
        ),
        (
            r":(attr:v=one\_)",
            pat_with_attrs(vec![("v", State::Value(r"one_".into()))]),
        ),
        (
            r":(attr:v=one\,)",
            pat_with_attrs(vec![("v", State::Value(r"one,".into()))]),
        ),
        (
            r":(attr:v=one\,two\,three)",
            pat_with_attrs(vec![("v", State::Value(r"one,two,three".into()))]),
        ),
        (
            r":(attr:a=\d b= c=\d)",
            pat_with_attrs(vec![
                ("a", State::Value(r"d".into())),
                ("b", State::Value(r"".into())),
                ("c", State::Value(r"d".into())),
            ]),
        ),
    ];

    check_valid_inputs(inputs);
}

fn pat_with_path(path: &str) -> NormalizedPattern {
    pat_with_path_and_sig(path, MagicSignature::empty())
}

fn pat_with_path_and_sig(path: &str, signature: MagicSignature) -> NormalizedPattern {
    pat(path, signature, SearchMode::ShellGlob, vec![])
}

fn pat_with_sig(signature: MagicSignature) -> NormalizedPattern {
    pat("", signature, SearchMode::ShellGlob, vec![])
}

fn pat_with_attrs(attrs: Vec<(&'static str, State)>) -> NormalizedPattern {
    pat("", MagicSignature::empty(), SearchMode::ShellGlob, attrs)
}

fn pat_with_search_mode(search_mode: SearchMode) -> NormalizedPattern {
    pat("", MagicSignature::empty(), search_mode, vec![])
}

fn pat(
    path: &str,
    signature: MagicSignature,
    search_mode: SearchMode,
    attributes: Vec<(&str, State)>,
) -> NormalizedPattern {
    NormalizedPattern {
        path: path.into(),
        signature,
        search_mode,
        attributes: attributes
            .into_iter()
            .map(|(attr, state)| (attr.into(), state))
            .collect(),
    }
}
