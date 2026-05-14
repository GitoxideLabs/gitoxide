use std::cmp::Ordering;

use crate::{basic_repo, util::hex_to_id};

#[test]
fn short_id() -> crate::Result {
    let repo = basic_repo()?;
    let commit = repo.head_commit()?;
    assert_eq!(commit.short_id()?.cmp_oid(&commit.id), Ordering::Equal);
    Ok(())
}

#[test]
fn tree() -> crate::Result {
    let repo = basic_repo()?;
    let tree_id = repo.head_tree_id()?;
    assert_eq!(tree_id, hex_to_id("21d3ba9a26b790a4858d67754ae05d04dfce4d0c"));

    let tree = tree_id.object()?.into_tree();
    assert_eq!(tree.id, tree_id);

    // It's possible to convert a `gix::Tree` into a lower-level tree and modify it.
    let _modififyable_tree: gix::objs::Tree = tree.clone().try_into()?;
    assert_eq!(
        _modififyable_tree,
        tree.decode()?.into(),
        "try_from and decode() yield the same object"
    );
    Ok(())
}

#[test]
fn decode() -> crate::Result {
    let repo = basic_repo()?;
    let commit = repo.head_commit()?;
    assert_eq!(commit.decode()?.message, commit.message_raw()?);
    assert_eq!(commit.decode()?.message(), commit.message()?);
    assert_eq!(commit.decode()?.message, "c2\n");
    Ok(())
}
