use gix_object::{Tree, TreeRef};

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
