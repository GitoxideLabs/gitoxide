use gix_object::{Tree, TreeRef};

#[test]
fn name_order_accounts_for_the_implicit_tree_separator() {
    use std::cmp::Ordering;

    use gix_object::bstr::BStr;

    assert_eq!(
        gix_object::tree::name_order(BStr::new(b"a"), false, BStr::new(b"a"), true),
        Ordering::Less,
        "a non-tree sorts as if terminated by NUL"
    );
    assert_eq!(
        gix_object::tree::name_order(BStr::new(b"a"), true, BStr::new(b"a."), false),
        Ordering::Greater,
        "the implicit slash sorts after a dot"
    );
    assert_eq!(
        gix_object::tree::name_order(BStr::new(b"a"), true, BStr::new(b"a0"), false),
        Ordering::Less,
        "the implicit slash sorts before zero"
    );
}

#[test]
fn sort_order_is_correct() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
    let input = std::fs::read(root.join("tree.baseline"))?;

    let mut tree = TreeRef::from_bytes(&input, gix_testtools::object_hash())?;
    let expected = tree.entries.clone();

    tree.entries.sort();
    assert_eq!(tree.entries, expected);
    let mut failures_when_searching_by_name = 0;
    for entry in expected {
        assert!(
            tree.entries.binary_search_by(|e| e.cmp(&entry)).is_ok(),
            "ordering works with binary search"
        );
        failures_when_searching_by_name += usize::from(
            tree.entries
                .binary_search_by(|e| e.filename.cmp(entry.filename))
                .is_err(),
        );
        assert_eq!(
            tree.bisect_entry(entry.filename, entry.mode.is_tree())
                .expect("entry is present"),
            entry
        );
    }

    assert_ne!(
        failures_when_searching_by_name, 0,
        "it's not possible to do a binary search by name alone"
    );

    let mut tree: Tree = tree.into();
    let expected = tree.entries.clone();
    tree.entries.sort();

    assert_eq!(tree.entries, expected);
    Ok(())
}
