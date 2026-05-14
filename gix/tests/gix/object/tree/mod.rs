use crate::util::{named_repo, named_subrepo_opts};

#[cfg(all(feature = "blob-diff", feature = "revision"))]
mod diff;

fn worktree_repo() -> Result<gix::Repository, gix::open::Error> {
    named_subrepo_opts("make_worktree_repo.sh", "repo", gix::open::Options::isolated())
}

#[test]
fn find_entry() -> crate::Result {
    let repo = named_repo("make_basic_repo.sh")?;
    let tree = repo.head_commit()?.tree()?;
    assert_eq!(tree.find_entry("this").expect("present").filename(), "this");

    assert!(tree.find_entry("not there").is_none());
    Ok(())
}

#[test]
fn lookup_entry_by_path() -> crate::Result {
    let repo = worktree_repo()?;
    let tree = repo.head_commit()?.tree()?;
    assert_eq!(tree.lookup_entry_by_path("dir/c")?.expect("present").filename(), "c");
    Ok(())
}

#[test]
fn decode_uses_the_tree_id_hash_kind() -> crate::Result {
    use gix::bstr::ByteSlice;

    let repo = named_repo("make_basic_repo.sh")?;
    let other_hash = match repo.object_hash() {
        gix::hash::Kind::Sha1 => gix::hash::Kind::Sha256,
        gix::hash::Kind::Sha256 => gix::hash::Kind::Sha1,
        _ => unimplemented!(),
    };

    let bogus_other_hash_entry_id = other_hash.null();
    let mut data = b"100644 file\0".to_vec();
    data.extend_from_slice(bogus_other_hash_entry_id.as_bytes());

    let bogus_other_hash_tree_id = other_hash.empty_tree();
    let tree = gix::Tree::from_data(bogus_other_hash_tree_id, data, &repo);
    let decoded = tree.decode()?;

    assert_eq!(decoded.entries.len(), 1);
    assert_eq!(decoded.entries[0].filename, b"file".as_bstr());
    assert_eq!(decoded.entries[0].oid, bogus_other_hash_entry_id.as_ref());
    assert_eq!(
        bogus_other_hash_entry_id.kind(),
        bogus_other_hash_tree_id.kind(),
        "both kinds are expected to match, the `repo.object_hash()` doesn't matter here"
    );
    Ok(())
}

mod peel_to_entry {
    #[test]
    fn top_level_file_keeps_the_current_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let root_id = tree.id();

        let entry = tree.peel_to_entry(["a"])?.expect("file entry");

        assert!(!entry.mode().is_tree());
        assert_eq!(tree.id(), root_id);
        Ok(())
    }

    #[test]
    fn nested_file_moves_to_the_last_seen_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let dir_id = tree.lookup_entry(["dir"])?.expect("tree entry").object_id();

        let entry = tree.peel_to_entry(["dir", "c"])?.expect("file entry");

        assert!(!entry.mode().is_tree());
        assert_eq!(tree.id(), dir_id);
        Ok(())
    }

    #[test]
    fn tree_leaf_moves_to_the_returned_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let dir_id = tree.lookup_entry(["dir"])?.expect("tree entry").object_id();

        let entry = tree.peel_to_entry(["dir"])?.expect("tree entry");

        assert!(entry.mode().is_tree());
        assert_eq!(tree.id(), dir_id);
        assert_eq!(
            tree.find_entry("c").expect("subtree entry").filename(),
            "c",
            "the data matches the id"
        );
        Ok(())
    }

    #[test]
    fn missing_top_level_entry_keeps_the_current_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let root_id = tree.id();

        let entry = tree.peel_to_entry(["missing"])?;

        assert!(entry.is_none());
        assert_eq!(tree.id(), root_id);
        Ok(())
    }

    #[test]
    fn missing_nested_entry_moves_to_the_last_seen_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let dir_id = tree.lookup_entry(["dir"])?.expect("tree entry").object_id();

        let entry = tree.peel_to_entry(["dir", "missing"])?;

        assert!(entry.is_none());
        assert_eq!(tree.id(), dir_id);
        Ok(())
    }

    #[test]
    fn path_continuing_past_a_top_level_file_keeps_the_current_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let root_id = tree.id();

        let entry = tree.peel_to_entry(["a", "missing"])?;

        assert!(entry.is_none());
        assert_eq!(tree.id(), root_id);
        assert_eq!(
            tree.find_entry("dir").expect("root tree entry").filename(),
            "dir",
            "the data matches the id"
        );
        Ok(())
    }

    #[test]
    fn path_continuing_past_a_nested_file_keeps_the_last_seen_tree() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let dir_id = tree.lookup_entry(["dir"])?.expect("tree entry").object_id();

        let entry = tree.peel_to_entry(["dir", "c", "missing"])?;

        assert!(entry.is_none());
        assert_eq!(tree.id(), dir_id);
        assert_eq!(
            tree.find_entry("c").expect("nested tree entry").filename(),
            "c",
            "the data matches the id"
        );
        Ok(())
    }

    #[test]
    fn by_path_has_the_same_tree_leaf_behavior() -> crate::Result {
        let repo = super::worktree_repo()?;
        let mut tree = repo.head_commit()?.tree()?;
        let dir_id = tree.lookup_entry(["dir"])?.expect("tree entry").object_id();

        let entry = tree.peel_to_entry_by_path("dir")?.expect("tree entry");

        assert!(entry.mode().is_tree());
        assert_eq!(tree.id(), dir_id);
        Ok(())
    }
}
