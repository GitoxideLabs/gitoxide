use gix_reftable::tree::{infix_walk, tree_free, tree_insert, tree_search, TreeNode};

fn cmp(a: &i32, b: &i32) -> i32 {
    a.cmp(b) as i32
}

// Upstream mapping: test_reftable_tree__tree_search
#[test]
fn tree_search_case() {
    let mut root: Option<Box<TreeNode<i32>>> = None;
    tree_insert(&mut root, 2, &cmp);
    tree_insert(&mut root, 1, &cmp);
    tree_insert(&mut root, 3, &cmp);
    assert!(tree_search(&root, &1, &cmp).is_some());
    assert!(tree_search(&root, &4, &cmp).is_none());
}

// Upstream mapping: test_reftable_tree__infix_walk
#[test]
fn infix_walk_case() {
    let mut root: Option<Box<TreeNode<i32>>> = None;
    for k in [2, 1, 3] {
        tree_insert(&mut root, k, &cmp);
    }
    let mut out = Vec::new();
    infix_walk(&root, &mut |k| out.push(*k));
    assert_eq!(out, vec![1, 2, 3]);
    tree_free(&mut root);
}
