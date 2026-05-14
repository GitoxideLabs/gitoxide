use crate::spec::parse::{Options, PrefixHintOwned, parse, try_parse_opts};

fn anchor_hint() -> Option<PrefixHintOwned> {
    Some(PrefixHintOwned::DescribeAnchor {
        ref_name: "cargo-smart-release".into(),
        generation: 679,
    })
}

#[test]
fn full_format_parses_hash_portion_as_prefix() {
    let rec = parse("cargo-smart-release-679-g3bee7fb");
    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None, "references are not resolved in describe output");
    assert_eq!(rec.prefix[0], Some(gix_hash::Prefix::from_hex("3bee7fb").unwrap()));
    assert_eq!(rec.prefix_hint[0], anchor_hint());
    assert_eq!(rec.calls, 1);

    let rec = parse("v1.0-0-g3bee7fb");
    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None, "references are not resolved in describe output");
    assert_eq!(rec.prefix[0], Some(gix_hash::Prefix::from_hex("3bee7fb").unwrap()));
    assert_eq!(
        rec.prefix_hint[0],
        Some(PrefixHintOwned::DescribeAnchor {
            ref_name: "v1.0".into(),
            generation: 0,
        })
    );
    assert_eq!(rec.calls, 1);
}

#[test]
fn full_format_lookalikes_fallback_to_ref() {
    let spec = "cargo-smart-release-679-g3bee7fb";
    let rec = try_parse_opts(
        spec,
        Options {
            reject_prefix: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), spec);
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.prefix_hint[0], None);
    assert_eq!(rec.calls, 2, "call prefix, then call ref");
}

#[test]
fn any_hash_without_suffix_and_prefix_g_is_assumed_to_be_describe_output() {
    let spec = "foo--bar-gabcdef1";
    let rec = parse(spec);
    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None);
    assert_eq!(
        rec.prefix[0],
        Some(gix_hash::Prefix::from_hex("abcdef1").unwrap()),
        "git does not parse very precisely here"
    );
    assert_eq!(rec.prefix_hint[0], Some(PrefixHintOwned::MustBeCommit));
    assert_eq!(rec.calls, 1);

    for invalid_describe in ["-gabcdef1", "gabcdef1"] {
        let rec = parse(invalid_describe);
        assert!(rec.kind.is_none());
        assert_eq!(
            rec.get_ref(0),
            invalid_describe,
            "we don't consider this a prefix from a describe block"
        );
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.prefix_hint[0], None);
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn full_format_with_dirty_suffix_is_recognized() {
    let rec = parse("cargo-smart-release-679-g3bee7fb-dirty");
    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None, "git does not see this as prefix, we do");
    assert_eq!(rec.prefix[0], Some(gix_hash::Prefix::from_hex("3bee7fb").unwrap()));
    assert_eq!(rec.prefix_hint[0], anchor_hint());
    assert_eq!(rec.calls, 1);
}

#[test]
fn partial_format_with_dirty_suffix_is_recognized() {
    let spec = "abcdef1-dirty";
    let rec = parse(spec);
    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None);
    assert_eq!(
        rec.prefix[0],
        Some(gix_hash::Prefix::from_hex("abcdef1").unwrap()),
        "git does not see this as prefix anymore, we do"
    );
    assert_eq!(
        rec.prefix_hint[0], None,
        "This leaves room for improvement as we could assume that -dirty belongs to a revision, so this could be PrefixHint::MustBeCommit"
    );
    assert_eq!(rec.calls, 1);
}

#[test]
fn partial_format_lookalikes_are_never_considered() {
    let spec = "abcdef1-dirty-laundry";
    let rec = parse(spec);
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), spec);
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.calls, 1, "we don't even try the prefix");
}

#[test]
fn partial_format_with_dirty_suffix_lookalikes_are_treated_as_refs() {
    let spec = "abcdef1-dirty";
    let rec = try_parse_opts(
        spec,
        Options {
            reject_prefix: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), spec);
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.calls, 2);
}
