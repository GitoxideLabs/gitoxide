use crate::spec::parse::{parse, try_parse};

#[test]
fn at_by_itself_is_shortcut_for_head() {
    let rec = parse("@");
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
}

#[test]
fn at_is_allowed() {
    for name in ["a@b", "@branch", "branch@", "@@", "@inner@"] {
        let rec = parse(name);
        assert!(rec.kind.is_none());
        assert_eq!(rec.get_ref(0), name);
        assert_eq!(rec.find_ref[1], None);
    }
}

#[test]
fn at_in_ranges_is_allowed() {
    let input = "@@@..";
    let rec = parse(input);
    assert_eq!(rec.kind, Some(gix_revision::spec::Kind::RangeBetween));
    assert_eq!(rec.get_ref(0), "@@@");
    assert_eq!(rec.get_ref(1), "HEAD");

    let input = "@@...@@";
    let rec = parse(input);
    assert_eq!(rec.kind, Some(gix_revision::spec::Kind::ReachableToMergeBase));
    assert_eq!(rec.get_ref(0), "@@");
    assert_eq!(rec.get_ref(1), "@@");
}

#[test]
fn strange_revspecs_do_not_panic() {
    let err = try_parse(".@.").unwrap_err();
    assert_eq!(err.input.as_ref().map(|i| i.as_ref()), Some(b"@.".as_ref()));
    assert!(err.message.contains("@ character"));
}

#[test]
fn refname_head() {
    let rec = parse("HEAD");
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
}

#[test]
fn refname_tag() {
    let spec = "v1.2.3.4-beta.1";
    let rec = parse(spec);
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), spec);
}

#[test]
fn refname_with_head_prefix() {
    let rec = parse("HEADfake");
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEADfake");
}

#[test]
fn full_head_ref_name() {
    let rec = parse("refs/heads/main");
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "refs/heads/main");
}
