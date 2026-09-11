use super::*;
use gix::objs::tree::EntryKind;

fn commit(repo: &gix::Repository, tree_id: ObjectId, parents: &[ObjectId]) -> Result<gix::objs::Commit> {
    let mut commit = repo.find_commit(repo.head_id()?)?.decode()?.into_owned()?;
    commit.tree = tree_id;
    commit.parents = parents.iter().copied().collect();
    commit.extra_headers.clear();
    Ok(commit)
}

fn patch(repo: &gix::Repository, base_tree_id: ObjectId, tree_id: ObjectId) -> Result<gix::objs::Commit> {
    let parent_commit_id = repo.write_object(&commit(repo, base_tree_id, &[])?)?.detach();
    commit(repo, tree_id, &[parent_commit_id])
}

fn entries(repo: &gix::Repository, values: &[(&[u8], EntryKind, ObjectId)]) -> Result<ObjectId> {
    let mut entries: Vec<_> = values
        .iter()
        .map(|(path, mode, blob_id)| gix::objs::tree::Entry {
            mode: (*mode).into(),
            filename: (*path).into(),
            oid: *blob_id,
        })
        .collect();
    entries.sort();
    Ok(repo.write_object(&gix::objs::Tree { entries })?.detach())
}

fn tree(repo: &gix::Repository, values: &[(&[u8], &[u8])]) -> Result<ObjectId> {
    let values = values
        .iter()
        .map(|(path, bytes)| Ok((*path, EntryKind::Blob, repo.write_blob(*bytes)?.detach())))
        .collect::<Result<Vec<_>>>()?;
    entries(repo, &values)
}

fn text_patch(repo: &gix::Repository, before: &[u8], after: &[u8]) -> Result<gix::objs::Commit> {
    patch(
        repo,
        tree(repo, &[(b"file", before)])?,
        tree(repo, &[(b"file", after)])?,
    )
}

#[test]
fn encoding_uses_only_the_unused_ascii_letters() -> gix_testtools::Result {
    for &kind in gix::hash::Kind::all() {
        let bytes: Vec<_> = (0..kind.len_in_bytes()).map(|index| (index * 13) as u8).collect();
        let id = PatchId(ObjectId::try_from(bytes.as_slice())?);
        let encoded = id.to_string();
        assert_eq!(
            encoded.len(),
            kind.len_in_bytes() * 4,
            "each byte has four base-four digits"
        );
        assert!(encoded.bytes().all(|byte| (b'g'..=b'j').contains(&byte)));
        assert_eq!(
            encoded.parse::<PatchId>()?,
            id,
            "the full hash round-trips without truncation"
        );
        assert!(
            encoded.to_uppercase().parse::<PatchId>().is_err(),
            "the encoding is canonical lowercase"
        );
        assert!(
            encoded[1..].parse::<PatchId>().is_err(),
            "abbreviations are not stored identities"
        );
        assert!(format!("f{}", &encoded[1..]).parse::<PatchId>().is_err());
    }
    Ok(())
}

#[test]
fn context_and_hunk_grouping_do_not_change_the_patch() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open_with(fixture.path(), ["diff.algorithm=invalid", "diff.renames=true"])?;
    let mut original = text_patch(&repo, b"alpha\none\ntwo\nomega\n", b"alpha\nONE\nTWO\nomega\n")?;
    let first = refresh(&repo, &mut original)?;
    assert_eq!((first.tree_walks, first.blob_reads, first.text_diffs), (1, 2, 1));
    for (before, after) in [
        (
            b"prefix\nalpha\none\ntwo\nomega\n".as_slice(),
            b"prefix\nalpha\nONE\nTWO\nomega\n".as_slice(),
        ),
        (b"changed\none\ntwo\ncontext\n", b"changed\nONE\nTWO\ncontext\n"),
        (
            b"alpha\none\nuntouched\ntwo\nomega\n",
            b"alpha\nONE\nuntouched\nTWO\nomega\n",
        ),
        (b"alpha\none\ntwo\nomega\n", b"alpha\nONE\nomega\nTWO\n"),
    ] {
        let mut replayed = text_patch(&repo, before, after)?;
        replayed.extra_headers = original.extra_headers.clone();
        let refreshed = refresh(&repo, &mut replayed)?;
        assert_eq!(
            refreshed.id, first.id,
            "only changed bytes and their per-side order define textual edits"
        );
        assert_eq!(refreshed.text_diffs, 1, "changed blobs need one normalized text diff");
        assert_eq!(
            refreshed.tree_walks, 2,
            "the carried witnesses first attempt raw-entry reuse"
        );
    }
    for after in [
        b"alpha\nONE \nTWO\nomega\n".as_slice(),
        b"alpha\nONE\r\nTWO\nomega\n",
        b"alpha\nONE\nTWO\nomega",
        b"alpha\nTWO\nONE\nomega\n",
    ] {
        let mut changed = text_patch(&repo, b"alpha\none\ntwo\nomega\n", after)?;
        assert_ne!(
            refresh(&repo, &mut changed)?.id,
            first.id,
            "whitespace, EOF, and edit order are significant"
        );
    }
    Ok(())
}

#[test]
fn metadata_and_unchanged_leaf_deltas_avoid_blob_work() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let mut original = text_patch(&repo, b"old\n", b"new\n")?;
    let id = refresh(&repo, &mut original)?.id;
    original.message = "only the message changed".into();
    let reused = refresh(&repo, &mut original)?;
    assert_eq!(reused.id, id);
    assert_eq!((reused.tree_walks, reused.blob_reads, reused.text_diffs), (0, 0, 0));

    let base_tree_id = tree(&repo, &[(b"file", b"old\n"), (b"unrelated", b"ancestor change\n")])?;
    let tree_id = tree(&repo, &[(b"file", b"new\n"), (b"unrelated", b"ancestor change\n")])?;
    let mut replayed = patch(&repo, base_tree_id, tree_id)?;
    replayed.extra_headers = original.extra_headers;
    let reused = refresh(&repo, &mut replayed)?;
    assert_eq!(reused.id, id, "unrelated ancestry changes preserve the patch");
    assert_eq!((reused.tree_walks, reused.blob_reads, reused.text_diffs), (2, 0, 0));
    let header = stored(replayed.extra_headers().find_all(HEADER))?.expect("the fresh header exists");
    assert_eq!(
        (header.base_tree_id, header.tree_id),
        (base_tree_id, tree_id),
        "reuse updates the witnesses"
    );
    Ok(())
}

#[test]
fn reads_validate_witnesses_without_loading_trees_and_hide_pending_states() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let base_tree_id = ObjectId::from_hex(b"1111111111111111111111111111111111111111")?;
    let tree_id = ObjectId::from_hex(b"2222222222222222222222222222222222222222")?;
    let id = PatchId(ObjectId::from_hex(b"3333333333333333333333333333333333333333")?);
    let mut original = patch(&repo, base_tree_id, tree_id)?;
    store(&mut original, format!("v1 {id} {base_tree_id} {tree_id}").into());
    let commit_id = repo.write_object(&original)?.detach();
    assert_eq!(
        for_commit(&repo, commit_id)?,
        Some(id),
        "reading a valid cache requires no tree or blob objects"
    );
    let reused = refresh(&repo, &mut original)?;
    assert_eq!((reused.tree_walks, reused.blob_reads, reused.text_diffs), (0, 0, 0));

    for (name, value) in [
        ("tix-rebase-parent", "invalid but pending"),
        ("gpgsig", ""),
        ("gpgsig-sha256", ""),
    ] {
        let mut pending = original.clone();
        pending.extra_headers.push((name.into(), value.into()));
        assert_eq!(
            for_commit(&repo, repo.write_object(&pending)?.detach())?,
            None,
            "pending states never expose cached identities"
        );
    }
    let mut conflicted = original.clone();
    mark_unavailable(&mut conflicted);
    assert!(is_unavailable(&conflicted));
    assert_eq!(for_commit(&repo, repo.write_object(&conflicted)?.detach())?, None);
    assert!(
        refresh(&repo, &mut conflicted).is_err(),
        "only conflict completion can clear the unavailable sentinel"
    );
    clear_unavailable(&mut conflicted);
    assert!(!is_unavailable(&conflicted));
    assert!(conflicted.extra_headers().find(HEADER).is_none());

    let mut changed = original.clone();
    changed.tree = base_tree_id;
    assert_eq!(
        for_commit(&repo, repo.write_object(&changed)?.detach())?,
        None,
        "a copied header cannot attest a changed result tree"
    );
    let different_parent_id = repo.write_object(&commit(&repo, tree_id, &[])?)?.detach();
    changed = original;
    changed.parents = [different_parent_id].into_iter().collect();
    assert_eq!(
        for_commit(&repo, repo.write_object(&changed)?.detach())?,
        None,
        "changing only the first-parent tree also invalidates the cache"
    );
    Ok(())
}

#[test]
fn modes_paths_binary_bytes_and_submodules_are_part_of_the_identity() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let mut original = text_patch(&repo, b"old\n", b"new\n")?;
    let original_id = refresh(&repo, &mut original)?.id;
    let mut moved = patch(
        &repo,
        tree(&repo, &[(b"other\xff", b"old\n")])?,
        tree(&repo, &[(b"other\xff", b"new\n")])?,
    )?;
    assert_ne!(
        refresh(&repo, &mut moved)?.id,
        original_id,
        "byte-oriented paths identify the edited file"
    );

    let blob_id = repo.write_blob(b"unchanged\n")?.detach();
    let base_tree_id = entries(&repo, &[(b"file", EntryKind::Blob, blob_id)])?;
    let tree_id = entries(&repo, &[(b"file", EntryKind::BlobExecutable, blob_id)])?;
    let mut mode_change = patch(&repo, base_tree_id, tree_id)?;
    let mode = refresh(&repo, &mut mode_change)?;
    assert_eq!(
        (mode.blob_reads, mode.text_diffs),
        (0, 0),
        "mode-only edits need no blob reads"
    );
    let mut link_change = patch(
        &repo,
        base_tree_id,
        entries(&repo, &[(b"file", EntryKind::Link, blob_id)])?,
    )?;
    assert_ne!(
        refresh(&repo, &mut link_change)?.id,
        mode.id,
        "symlink and executable transitions differ"
    );

    let mut binary = text_patch(&repo, b"old\0bytes", b"new\0bytes")?;
    let binary_id = refresh(&repo, &mut binary)?;
    assert_eq!(binary_id.text_diffs, 0, "binary content uses exact object identities");
    let mut different_binary = text_patch(&repo, b"old\0bytes", b"NEW\0bytes")?;
    assert_ne!(refresh(&repo, &mut different_binary)?.id, binary_id.id);

    let old_commit_id = ObjectId::from_hex(b"4444444444444444444444444444444444444444")?;
    let new_commit_id = ObjectId::from_hex(b"5555555555555555555555555555555555555555")?;
    let mut submodule = patch(
        &repo,
        entries(&repo, &[(b"submodule", EntryKind::Commit, old_commit_id)])?,
        entries(&repo, &[(b"submodule", EntryKind::Commit, new_commit_id)])?,
    )?;
    let refreshed = refresh(&repo, &mut submodule)?;
    assert_eq!(
        (refreshed.blob_reads, refreshed.text_diffs),
        (0, 0),
        "submodule commits need not exist locally"
    );
    Ok(())
}

#[test]
fn roots_empty_changes_and_merges_use_the_fixed_first_parent() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let tree_id = tree(&repo, &[(b"file", b"initial\n")])?;
    let mut root = commit(&repo, tree_id, &[])?;
    let added = refresh(&repo, &mut root)?;
    assert_eq!(
        (added.blob_reads, added.text_diffs),
        (0, 0),
        "additions reuse their complete blob hash"
    );
    let mut against_empty = patch(&repo, repo.object_hash().empty_tree(), tree_id)?;
    assert_eq!(
        refresh(&repo, &mut against_empty)?.id,
        added.id,
        "roots compare against the empty tree"
    );

    let mut empty = patch(&repo, tree_id, tree_id)?;
    let empty_id = refresh(&repo, &mut empty)?.id;
    let mut empty_root = commit(&repo, repo.object_hash().empty_tree(), &[])?;
    assert_eq!(
        refresh(&repo, &mut empty_root)?.id,
        empty_id,
        "empty deltas have one patch identity"
    );
    let second_parent_id = repo.write_object(&root)?.detach();
    against_empty.parents.push(second_parent_id);
    assert_eq!(
        refresh(&repo, &mut against_empty)?.id,
        added.id,
        "merge identity always uses its first parent"
    );
    Ok(())
}

#[test]
fn malformed_headers_fail_reads_and_missing_old_witness_objects_only_disable_reuse() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let mut original = text_patch(&repo, b"old\n", b"new\n")?;
    let id = refresh(&repo, &mut original)?.id;
    let mut malformed = original.clone();
    store(&mut malformed, "v2 unsupported".into());
    assert!(for_commit(&repo, repo.write_object(&malformed)?.detach()).is_err());
    assert_eq!(
        refresh(&repo, &mut malformed)?.id,
        id,
        "final rewrites replace malformed caches"
    );
    malformed.extra_headers.push((HEADER.into(), "v1 unavailable".into()));
    assert!(
        for_commit(&repo, repo.write_object(&malformed)?.detach()).is_err(),
        "duplicate cache headers are ambiguous"
    );

    let missing_tree_id = ObjectId::from_hex(b"6666666666666666666666666666666666666666")?;
    let stale = format!("v1 {id} {missing_tree_id} {}", original.tree).into();
    store(&mut original, stale);
    let refreshed = refresh(&repo, &mut original)?;
    assert_eq!(refreshed.id, id, "pruned cache witnesses cannot break a final rewrite");
    assert_eq!(
        refreshed.text_diffs, 1,
        "missing cache inputs cause one computation from the final trees"
    );
    Ok(())
}
