use gix_revision::spec::parse::delegate::Traversal;

use crate::spec::parse::{PeelToOwned as PeelTo, parse};

#[test]
fn paths_consume_all_remaining_input_as_they_refer_to_blobs() {
    let rec = parse("@:../relative/path...@^^~~");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.traversal.len(), 0);
    assert_eq!(rec.peel_to, vec![PeelTo::Path("../relative/path...@^^~~".into())]);
    assert_eq!(rec.calls, 2);

    let rec = parse("@:absolute/path^{object}");
    assert_eq!(
        rec.peel_to,
        vec![PeelTo::Path("absolute/path^{object}".into())],
        "this includes useful navigation like object-existence, a shortcoming git shares, proper implementation needs escaping as well."
    );

    let rec = parse("@:absolute/path^{tree}");
    assert_eq!(
        rec.peel_to,
        vec![PeelTo::Path("absolute/path^{tree}".into())],
        "this includes useful navigation like assertion of trees/blobs, we may make this possible in future but for now are as open as git"
    );
}

#[test]
fn empty_paths_refer_to_the_root_tree() {
    let rec = parse("@:");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.peel_to, vec![PeelTo::Path("".into())]);
    assert_eq!(rec.calls, 2);
}

#[test]
fn paths_have_to_be_last_but_stack_with_other_navigation() {
    let rec = parse("HEAD@{1}~10^2^{commit}:README.md");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.current_branch_reflog_entry[0], Some("1".to_string()));
    assert_eq!(rec.traversal, vec![Traversal::NthAncestor(10), Traversal::NthParent(2)]);
    assert_eq!(
        rec.peel_to,
        vec![
            PeelTo::ObjectKind(gix_object::Kind::Commit),
            PeelTo::Path("README.md".into())
        ]
    );
    assert_eq!(rec.calls, 6);
}
