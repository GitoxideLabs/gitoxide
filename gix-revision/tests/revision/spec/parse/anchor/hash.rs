use crate::spec::parse::{Options, parse, try_parse_opts};

#[test]
fn short_hex_literals_are_considered_prefixes() {
    let rec = parse("abCD");
    assert!(rec.kind.is_none());
    assert_eq!(
        rec.find_ref[0], None,
        "references are not resolved if prefix lookups succeed"
    );
    assert_eq!(rec.prefix[0], Some(gix_hash::Prefix::from_hex("abcd").unwrap()));
    assert_eq!(rec.prefix_hint[0], None);
    assert_eq!(rec.calls, 1);

    let rec = parse("gabcd123");
    assert!(rec.kind.is_none());
    assert_eq!(
        rec.get_ref(0),
        "gabcd123",
        "ref lookups are performed if it doesn't look like a hex sha"
    );
    assert_eq!(
        rec.prefix[0], None,
        "prefix lookups are not attempted at all (and they are impossible even)"
    );
    assert_eq!(rec.prefix_hint[0], None);
    assert_eq!(rec.calls, 1);
}

#[test]
fn unresolvable_hex_literals_are_resolved_as_refs() {
    let rec = try_parse_opts(
        "abCD",
        Options {
            reject_prefix: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "abCD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.prefix_hint[0], None);
    assert_eq!(rec.calls, 2);
}

#[test]
fn hex_literals_that_are_too_long_are_resolved_as_refs() {
    let spec = "abcd123456789abcd123456789abcd123456789abcd123456789abcd123456789abcd123456789abcd123456789";
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
    assert_eq!(
        rec.calls, 1,
        "we can't create a prefix from it, hence only ref resolution is attempted"
    );
}
