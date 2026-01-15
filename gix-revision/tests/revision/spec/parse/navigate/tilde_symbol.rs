use gix_revision::spec::parse::delegate::Traversal;

use crate::spec::parse::{parse, try_parse};

#[test]
fn without_anchor_is_invalid() {
    let err = try_parse("~").unwrap_err().into_inner();
    assert!(err.message.contains("tilde needs to follow an anchor"));
}

#[test]
fn single_is_first_ancestor() {
    let rec = parse("@~");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.traversal[0], Traversal::NthAncestor(1));
    assert_eq!(rec.calls, 2);
}

#[test]
fn followed_by_zero_is_no_op() {
    let rec = parse("@~0");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.calls, 1);
}

#[test]
fn multiple_calls_stack() {
    let rec = parse("@~~~10~0~020");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(
        rec.traversal,
        vec![
            Traversal::NthAncestor(1),
            Traversal::NthAncestor(1),
            Traversal::NthAncestor(10),
            Traversal::NthAncestor(20),
        ]
    );
    assert_eq!(rec.calls, 5);
}
