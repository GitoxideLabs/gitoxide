//! Tree merges whose inputs Git's porcelain cannot produce, like gitlinks replaced by symlinks or noncanonical modes.
//! Trees are written to an in-memory object database so each case states its entries directly.

use std::fmt::Write as _;

use gix_hash::ObjectId;
use gix_merge::blob::builtin_driver::binary::ResolveWith;
use gix_object::{
    FindExt, Write,
    tree::{EntryKind, EntryMode},
};
use gix_testtools::TestResult;

use super::{basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

struct Repo {
    root: gix_testtools::tempfile::TempDir,
    objects: gix_odb::memory::Proxy<gix_object::find::Never>,
}

impl Repo {
    fn new() -> TestResult<Self> {
        Ok(Repo {
            root: gix_testtools::tempfile::TempDir::new()?,
            objects: gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash()),
        })
    }

    fn blob(&self, data: &str) -> TestResult<ObjectId> {
        Ok(self.objects.write_buf(gix_object::Kind::Blob, data.as_bytes())?)
    }

    /// Write a tree with the single entry `filename`.
    fn tree(&self, filename: &str, mode: EntryMode, oid: ObjectId) -> TestResult<ObjectId> {
        let entry = gix_object::tree::Entry {
            mode,
            filename: filename.into(),
            oid,
        };
        Ok(self.objects.write(&gix_object::Tree { entries: vec![entry] })?)
    }

    fn tree_from_paths(&self, entries: &[(&str, EntryKind, ObjectId)]) -> TestResult<ObjectId> {
        let mut tree =
            gix_object::tree::Editor::new(gix_object::Tree::empty(), &self.objects, gix_testtools::object_hash());
        for (path, kind, object_id) in entries {
            tree.upsert(path.split('/'), *kind, *object_id)?;
        }
        Ok(tree.write(|tree| self.objects.write(tree))?)
    }

    fn blob_data(&self, blob_id: ObjectId) -> TestResult<String> {
        let mut buf = Vec::new();
        let blob = self.objects.find_blob(&blob_id, &mut buf)?;
        Ok(String::from_utf8(blob.data.to_vec())?)
    }

    /// Merge `other` into `current`.
    fn merge(
        &self,
        trees: (ObjectId, ObjectId, ObjectId),
        options: gix_merge::tree::Options,
    ) -> TestResult<gix_merge::tree::Outcome<'_>> {
        self.merge_with_objects(trees, &self.objects, options)
    }

    fn merge_with_objects<'objects>(
        &self,
        (base, current, other): (ObjectId, ObjectId, ObjectId),
        objects: &'objects impl gix_object::FindObjectOrHeader,
        options: gix_merge::tree::Options,
    ) -> TestResult<gix_merge::tree::Outcome<'objects>> {
        Ok(gix_merge::tree(
            &base,
            &current,
            &other,
            Default::default(),
            objects,
            |buf| self.objects.write_buf(gix_object::Kind::Blob, buf),
            &mut gix_diff::tree::State::default(),
            &mut new_diff_resource_cache(self.root.path()),
            &mut new_blob_merge_platform(self.root.path(), None),
            gix_merge::tree::Context::default(),
            options,
        )?)
    }

    fn entries(&self, outcome: &mut gix_merge::tree::Outcome<'_>) -> TestResult<Vec<(String, EntryMode, ObjectId)>> {
        let merged_tree_id = outcome.tree.write(|tree| self.objects.write(tree))?;
        let mut buf = Vec::new();
        let merged = self.objects.find_tree(&merged_tree_id, &mut buf)?;
        Ok(merged
            .entries
            .iter()
            .map(|entry| (entry.filename.to_string(), entry.mode, entry.oid.to_owned()))
            .collect())
    }

    /// Merge `ours` and `theirs` in both directions and assert that each result has the single `expected` entry.
    fn assert_merges_to(
        &self,
        (base, ours, theirs): (ObjectId, ObjectId, ObjectId),
        symlink_conflicts: Option<ResolveWith>,
        (filename, mode, oid): (&str, EntryMode, ObjectId),
        message: &str,
    ) -> TestResult {
        for (current, other) in [(ours, theirs), (theirs, ours)] {
            let mut options = basic_merge_options().tree_merge;
            options.symlink_conflicts = symlink_conflicts;
            let mut outcome = self.merge((base, current, other), options)?;
            let actual = self.entries(&mut outcome)?;
            assert_eq!(actual, [(filename.to_owned(), mode, oid)], "{message}");
        }
        Ok(())
    }
}

#[test]
fn forced_crossing_renames_do_not_reinstall_discarded_subtrees() -> TestResult {
    use gix_merge::tree::{ResolveWith as TreeResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let first_blob_id = repo.blob("binary payload 4\0\n")?;
    let second_blob_id = repo.blob("binary payload 0\0\n")?;
    let base_tree_id = repo.tree_from_paths(&[
        ("b/a/a", EntryKind::Link, first_blob_id),
        ("c/a/c", EntryKind::Link, second_blob_id),
    ])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("b/a/a", EntryKind::Link, first_blob_id),
        ("b/a/c", EntryKind::Link, second_blob_id),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("b", EntryKind::Link, second_blob_id),
        ("c", EntryKind::Link, first_blob_id),
    ])?;

    for (current, other) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
        let mut options = basic_merge_options().tree_merge;
        options.directory_renames = Default::default();
        options
            .rewrites
            .as_mut()
            .expect("rename tracking is enabled")
            .percentage = Some(1.0);
        options.symlink_conflicts = Some(ResolveWith::Ancestor);
        options.tree_conflicts = Some(TreeResolveWith::Ours);
        let mut outcome = repo.merge((base_tree_id, current, other), options)?;
        let merged_tree_id = outcome.tree.write(|tree| repo.objects.write(tree))?;
        let index = outcome
            .index
            .to_index(TreatAsUnresolved::git(), &repo.objects, Default::default())?;
        let tree_index = gix_index::State::from_tree(&merged_tree_id, &repo.objects, Default::default())?;
        assert_eq!(
            super::baseline::debug_entries(&index),
            super::baseline::debug_entries(&tree_index),
            "forced crossing renames keep the tree and index in agreement for current {current}"
        );
    }
    Ok(())
}

#[test]
fn renamed_gitlink_colliding_with_a_replacement_gitlink() -> TestResult {
    use gix_merge::tree::{Resolution, ResolutionFailure, ResolveWith};

    let repo = Repo::new()?;
    let original_commit_id =
        gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"original")?;
    let replacement_commit_id =
        gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"replacement")?;
    let original_blob_id = repo.blob("original file")?;
    let new_blob_id = repo.blob("new file")?;
    let link_blob_id = repo.blob("target")?;
    let write_tree = |entries: &[(&str, EntryKind, ObjectId)]| -> TestResult<ObjectId> {
        let mut editor =
            gix_object::tree::Editor::new(gix_object::Tree::empty(), &repo.objects, gix_testtools::object_hash());
        for (path, kind, id) in entries {
            editor.upsert(path.split('/'), *kind, *id)?;
        }
        Ok(editor.write(|tree| repo.objects.write(tree))?)
    };
    // Reduced from clusterfuzz-testcase-gix-merge-tree-4702732561481728: the gitlink
    // moved to a/a follows the other side's a -> c directory rename and collides at c/a.
    let base_tree_id = write_tree(&[
        ("c/a", EntryKind::Blob, original_blob_id),
        ("b/a/f", EntryKind::Commit, original_commit_id),
        ("a/c/c", EntryKind::Link, link_blob_id),
    ])?;
    let renamed_tree_id = write_tree(&[
        ("b", EntryKind::Blob, new_blob_id),
        ("a/a", EntryKind::Commit, original_commit_id),
        ("c/a", EntryKind::Blob, original_blob_id),
        ("a/c/c", EntryKind::Link, link_blob_id),
    ])?;
    let replaced_tree_id = write_tree(&[
        ("c/a", EntryKind::Commit, replacement_commit_id),
        ("b/a/f", EntryKind::Commit, original_commit_id),
        ("c/c/c", EntryKind::Link, link_blob_id),
    ])?;
    let expected_tree_id = write_tree(&[
        ("a/a", EntryKind::Commit, original_commit_id),
        ("b", EntryKind::Blob, new_blob_id),
        ("c/a", EntryKind::Commit, replacement_commit_id),
        ("c/c/c", EntryKind::Link, link_blob_id),
    ])?;
    for (current_tree_id, other_tree_id) in [(renamed_tree_id, replaced_tree_id), (replaced_tree_id, renamed_tree_id)] {
        for tree_conflicts in [None, Some(ResolveWith::Ours), Some(ResolveWith::Ancestor)] {
            let mut options = basic_merge_options().tree_merge;
            options
                .rewrites
                .as_mut()
                .expect("rename tracking is enabled")
                .percentage = Some(1.0);
            options.tree_conflicts = tree_conflicts;
            let mut outcome = repo.merge((base_tree_id, current_tree_id, other_tree_id), options)?;
            super::assert_no_unknown_conflicts(&outcome, "a renamed gitlink colliding with a replacement gitlink");
            assert!(
                outcome.conflicts.iter().any(|conflict| matches!(
                    conflict.resolution,
                    Err(ResolutionFailure::DirectoryRenameBlocked { .. })
                        | Ok(Resolution::Forced(ResolutionFailure::DirectoryRenameBlocked { .. }))
                )),
                "the blocked relocation is diagnosed before completing the tree merge: {:#?}",
                outcome.conflicts
            );
            assert_eq!(
                outcome.tree.write(|tree| repo.objects.write(tree))?,
                expected_tree_id,
                "like Git, a blocked directory relocation keeps both gitlinks at their explicit paths under {tree_conflicts:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn early_forced_directory_collisions_retain_content_stages() -> TestResult {
    use gix_merge::tree::{DirectoryRenames, ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let moved_blob_id = repo.blob("moved\0")?;
    let relocated_blob_id = repo.blob("relocated\0")?;
    let base_tree_id = repo.tree_from_paths(&[
        ("e/e/e", EntryKind::Blob, moved_blob_id),
        ("h", EntryKind::Blob, relocated_blob_id),
    ])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("e/e/e", EntryKind::Blob, moved_blob_id),
        ("e/e/h", EntryKind::Blob, relocated_blob_id),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("e/h/h", EntryKind::Blob, moved_blob_id),
        ("h", EntryKind::Blob, relocated_blob_id),
    ])?;
    let mut options = basic_merge_options().tree_merge;
    options.directory_renames = DirectoryRenames::Conflict;
    options.tree_conflicts = Some(ResolveWith::Ours);
    options.fail_on_conflict = Some(TreatAsUnresolved::git());
    // CIFuzz crash-a3a5ca9afbb42f9f9aaadaa1dc0461ceca390524: directory relocation
    // collides with the opposite rename before the forced source restoration.
    let mut outcome = repo.merge((base_tree_id, our_tree_id, their_tree_id), options)?;
    assert!(
        outcome.failed_on_first_unresolved_conflict,
        "the binary collision stops normalization"
    );
    assert!(
        outcome.index.has_unmerged_entries(TreatAsUnresolved::git()),
        "the stopped content merge owns index stages before returning: {:?}",
        outcome.conflicts
    );
    let index = outcome
        .index
        .to_index(TreatAsUnresolved::git(), &repo.objects, Default::default())?;
    index.verify_entries()?;
    let stages: Vec<_> = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix_index::entry::Stage::Unconflicted)
        .map(|entry| (entry.path(&index), entry.stage(), entry.id))
        .collect();
    assert_eq!(
        stages,
        [
            (gix_index::entry::Stage::Ours, relocated_blob_id),
            (gix_index::entry::Stage::Theirs, moved_blob_id),
        ]
        .map(|(stage, blob_id)| (bstr::BStr::new(b"e/e/h"), stage, blob_id)),
        "the restored source retains both sides of its binary collision"
    );
    assert_eq!(
        outcome.tree.write(|tree| repo.objects.write(tree))?,
        our_tree_id,
        "the partial result retains the chosen source namespace and binary content"
    );
    Ok(())
}

#[test]
fn forced_rename_keeps_compatible_directory_replacements() -> TestResult {
    let repo = Repo::new()?;
    let original_blob_id = repo.blob("original\n")?;
    let replacement_blob_id = repo.blob("replacement\n")?;
    let base_tree_id = repo.tree_from_paths(&[("folder/file", EntryKind::Blob, original_blob_id)])?;
    let their_tree_id = repo.tree("folder", EntryKind::Blob.into(), replacement_blob_id)?;
    let mut options = basic_merge_options().tree_merge;
    options.tree_conflicts = Some(gix_merge::tree::ResolveWith::Ours);
    for (destination, expected) in [
        (
            "moved",
            repo.tree_from_paths(&[
                ("folder", EntryKind::Blob, replacement_blob_id),
                ("moved", EntryKind::Blob, original_blob_id),
            ])?,
        ),
        (
            "folder/moved",
            repo.tree_from_paths(&[("folder/moved", EntryKind::Blob, original_blob_id)])?,
        ),
    ] {
        let our_tree_id = repo.tree_from_paths(&[(destination, EntryKind::Blob, original_blob_id)])?;
        let mut outcome = repo.merge((base_tree_id, our_tree_id, their_tree_id), options.clone())?;
        assert_eq!(
            outcome.tree.write(|tree| repo.objects.write(tree))?,
            expected,
            "the replacement survives only when it can coexist with the selected rename to {destination}"
        );
    }
    Ok(())
}

#[test]
fn built_in_text_merges_check_blob_sizes_before_loading() -> TestResult {
    const GIT_TEXT_LIMIT: u64 = 1023 * 1024 * 1024;

    struct SizedBlob<'a> {
        repo: &'a Repo,
        blob_id: ObjectId,
        size: u64,
    }

    impl gix_object::Find for SizedBlob<'_> {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            buf: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            assert!(
                id != self.blob_id || self.size == GIT_TEXT_LIMIT,
                "the built-in merge must not load blobs above Git's text size limit"
            );
            self.repo.objects.try_find(id, buf)
        }
    }

    impl gix_object::FindHeader for SizedBlob<'_> {
        fn try_header(&self, id: &gix_hash::oid) -> gix_error::Result<Option<gix_object::Header>> {
            if id == self.blob_id {
                Ok(Some(gix_object::Header {
                    kind: gix_object::Kind::Blob,
                    size: self.size,
                }))
            } else {
                self.repo.objects.try_header(id)
            }
        }
    }

    let repo = Repo::new()?;
    let blob_ids = [repo.blob("base\n")?, repo.blob("ours\n")?, repo.blob("theirs\n")?];
    let [base_tree_id, our_tree_id, their_tree_id] =
        blob_ids.map(|blob_id| repo.tree("file", EntryKind::Blob.into(), blob_id));
    let (base_tree_id, our_tree_id, their_tree_id) = (base_tree_id?, our_tree_id?, their_tree_id?);
    for blob_id in blob_ids {
        for excess in [0, 1] {
            let objects = SizedBlob {
                repo: &repo,
                blob_id,
                size: GIT_TEXT_LIMIT + excess,
            };
            let mut options = basic_merge_options().tree_merge;
            options.rewrites = None;
            let mut outcome = repo.merge_with_objects((base_tree_id, our_tree_id, their_tree_id), &objects, options)?;
            assert_eq!(
                outcome.tree.write(|tree| repo.objects.write(tree))? == our_tree_id,
                excess > 0,
                "the binary fallback keeps ours only if a blob exceeds the limit"
            );
        }
    }
    Ok(())
}

#[test]
fn virtual_blob_merges_preserve_explicit_tree_policy() -> TestResult {
    use gix_merge::tree::ResolveWith;

    let repo = Repo::new()?;
    let base_tree_id = repo.tree("a", EntryKind::Blob.into(), repo.blob("base\n")?)?;
    let modified_tree_id = repo.tree("a", EntryKind::Blob.into(), repo.blob("modified\n")?)?;
    let deleted_tree_id = repo.objects.write(&gix_object::Tree::default())?;
    for (current_tree_id, other_tree_id) in [(modified_tree_id, deleted_tree_id), (deleted_tree_id, modified_tree_id)] {
        for (choice, expected_tree_id) in [
            (None, modified_tree_id),
            (Some(ResolveWith::Ours), current_tree_id),
            (Some(ResolveWith::Ancestor), base_tree_id),
        ] {
            let mut options = basic_merge_options().tree_merge;
            options.blob_merge.is_virtual_ancestor = true;
            options.tree_conflicts = choice;
            let mut actual = repo.merge((base_tree_id, current_tree_id, other_tree_id), options)?;
            assert_eq!(
                actual.tree.write(|tree| repo.objects.write(tree))?,
                expected_tree_id,
                "virtual blob handling does not replace the requested tree policy {choice:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn virtual_distinct_types_preserve_explicit_tree_policy() -> TestResult {
    use gix_merge::tree::{ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let base_tree_id = repo.tree("a", EntryKind::Blob.into(), repo.blob("base\n")?)?;
    let target_blob_id = repo.blob("target")?;
    let modified_blob_id = repo.blob("modified\n")?;
    let symlink_tree_id = repo.tree("a", EntryKind::Link.into(), target_blob_id)?;
    let regular_tree_id = repo.tree("a", EntryKind::Blob.into(), modified_blob_id)?;
    let unresolved_tree_id = repo.objects.write(&gix_object::Tree {
        entries: vec![
            gix_object::tree::Entry {
                mode: EntryKind::Link.into(),
                filename: "a".into(),
                oid: target_blob_id,
            },
            gix_object::tree::Entry {
                mode: EntryKind::Blob.into(),
                filename: "a~".into(),
                oid: modified_blob_id,
            },
        ],
    })?;
    for (current_tree_id, other_tree_id) in [(symlink_tree_id, regular_tree_id), (regular_tree_id, symlink_tree_id)] {
        for (choice, expected_tree_id) in [
            (None, unresolved_tree_id),
            (Some(ResolveWith::Ancestor), base_tree_id),
            (Some(ResolveWith::Ours), current_tree_id),
        ] {
            for is_virtual_ancestor in [false, true] {
                let mut options = basic_merge_options().tree_merge;
                options.rewrites = None;
                options.blob_merge.is_virtual_ancestor = is_virtual_ancestor;
                options.tree_conflicts = choice;
                let mut actual = repo.merge((base_tree_id, current_tree_id, other_tree_id), options)?;
                assert_eq!(
                    actual.tree.write(|tree| repo.objects.write(tree))?,
                    expected_tree_id,
                    "virtual={is_virtual_ancestor} distinct types honor tree policy {choice:?} in both directions"
                );
                assert_eq!(
                    actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    choice.is_none(),
                    "only an explicit tree choice resolves the distinct-types conflict"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn symlink_ancestor_resolution_restores_the_whole_ancestor_entry() -> TestResult {
    use gix_merge::tree::{TreatAsUnresolved, treat_as_unresolved};

    let repo = Repo::new()?;
    let commit_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"submodule")?;
    let our_blob_id = repo.blob("ours")?;
    let their_blob_id = repo.blob("theirs")?;
    let ours = repo.tree("a", EntryKind::Link.into(), our_blob_id)?;
    let theirs = repo.tree("a", EntryKind::Link.into(), their_blob_id)?;

    for (kind, ancestor_id) in [
        (EntryKind::Commit, commit_id),
        (EntryKind::Blob, repo.blob("target")?),
        (EntryKind::Link, repo.blob("target")?),
    ] {
        let base = repo.tree("a", kind.into(), ancestor_id)?;
        for (current, other, current_blob_id, other_blob_id) in [
            (ours, theirs, our_blob_id, their_blob_id),
            (theirs, ours, their_blob_id, our_blob_id),
        ] {
            let mut options = basic_merge_options().tree_merge;
            options.symlink_conflicts = Some(ResolveWith::Ancestor);
            let mut outcome = repo.merge((base, current, other), options)?;
            assert_eq!(
                repo.entries(&mut outcome)?,
                [("a".to_owned(), kind.into(), ancestor_id)],
                "choosing the ancestor restores its whole entry instead of writing its id as a symlink target"
            );
            let [conflict] = outcome.conflicts.as_slice() else {
                panic!("expected one forced symlink resolution, got {:?}", outcome.conflicts);
            };
            assert!(
                conflict.content_merge().is_none(),
                "restoring an ancestor entry does not expose its potentially absent commit as a merged blob"
            );
            for tree_merge in [
                treat_as_unresolved::TreeMerge::Undecidable,
                treat_as_unresolved::TreeMerge::EvasiveRenames,
                treat_as_unresolved::TreeMerge::ForcedResolution,
            ] {
                for content_merge in [
                    treat_as_unresolved::ContentMerge::Markers,
                    treat_as_unresolved::ContentMerge::ForcedResolution,
                ] {
                    let how = TreatAsUnresolved {
                        tree_merge,
                        content_merge,
                    };
                    let unresolved = content_merge == treat_as_unresolved::ContentMerge::ForcedResolution;
                    assert_eq!(
                        outcome.has_unresolved_conflicts(how),
                        unresolved,
                        "the symlink conflict remains a forced content resolution under {how:?}"
                    );
                    let index = outcome.index.to_index(how, &repo.objects, Default::default())?;
                    assert_eq!(
                        outcome.index.has_unmerged_entries(how),
                        unresolved,
                        "only a policy that considers this unresolved should add conflict stages"
                    );
                    let actual: Vec<_> = index
                        .entries()
                        .iter()
                        .map(|entry| (entry.stage_raw(), entry.mode, entry.id))
                        .collect();
                    let expected = if unresolved {
                        vec![
                            (1, kind.into(), ancestor_id),
                            (2, EntryKind::Link.into(), current_blob_id),
                            (3, EntryKind::Link.into(), other_blob_id),
                        ]
                    } else {
                        vec![(0, kind.into(), ancestor_id)]
                    };
                    assert_eq!(
                        actual, expected,
                        "index stages preserve all object kinds and side identities"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn symlink_ancestor_resolution_preserves_an_absent_ancestor_in_the_index() -> TestResult {
    use gix_index::entry::{Mode, Stage};
    use gix_merge::tree::{TreatAsUnresolved, treat_as_unresolved::ContentMerge};

    let repo = Repo::new()?;
    let base_tree_id = repo.objects.write(&gix_object::Tree::empty())?;
    let our_blob_id = repo.blob("ours")?;
    let their_blob_id = repo.blob("theirs")?;
    let our_tree_id = repo.tree("a", EntryKind::Link.into(), our_blob_id)?;
    let their_tree_id = repo.tree("a", EntryKind::Link.into(), their_blob_id)?;
    for tree_conflicts in [
        None,
        Some(gix_merge::tree::ResolveWith::Ancestor),
        Some(gix_merge::tree::ResolveWith::Ours),
    ] {
        for (current_tree_id, other_tree_id, current_blob_id, other_blob_id) in [
            (our_tree_id, their_tree_id, our_blob_id, their_blob_id),
            (their_tree_id, our_tree_id, their_blob_id, our_blob_id),
        ] {
            let mut options = basic_merge_options().tree_merge;
            options.symlink_conflicts = Some(ResolveWith::Ancestor);
            options.tree_conflicts = tree_conflicts;
            let mut outcome = repo.merge((base_tree_id, current_tree_id, other_tree_id), options)?;
            assert_eq!(
                outcome.tree.write(|tree| repo.objects.write(tree))?,
                base_tree_id,
                "selecting an absent symlink ancestor leaves the output path absent"
            );
            for how in [TreatAsUnresolved::git(), TreatAsUnresolved::forced_resolution()] {
                let unresolved = how.content_merge == ContentMerge::ForcedResolution;
                let index = outcome.index.to_index(how, &repo.objects, Default::default())?;
                let entries: Vec<_> = index
                    .entries()
                    .iter()
                    .map(|entry| (entry.path(&index), entry.stage(), entry.mode, entry.id))
                    .collect();
                let expected = if unresolved {
                    vec![
                        (bstr::BStr::new(b"a"), Stage::Ours, Mode::SYMLINK, current_blob_id),
                        (bstr::BStr::new(b"a"), Stage::Theirs, Mode::SYMLINK, other_blob_id),
                    ]
                } else {
                    Vec::new()
                };
                assert_eq!(
                    entries, expected,
                    "an absent selected ancestor never creates a stage-zero null object; review retains only real sources"
                );
                assert_eq!(
                    outcome.has_unresolved_conflicts(how),
                    unresolved,
                    "the absent ancestor is still a forced content resolution"
                );
                assert_eq!(outcome.index.has_unmerged_entries(how), unresolved);
            }
        }
    }
    Ok(())
}

#[test]
fn noncanonical_executable_mode_in_rename_edit_merge() -> TestResult {
    let repo = Repo::new()?;
    let executable = EntryKind::BlobExecutable.into();
    let noncanonical_executable = EntryMode::from_bytes(b"100775").expect("valid octal mode");
    let original_blob_id = repo.blob("1\n2\n3\n4\n5\n")?;
    let edited_blob_id = repo.blob("1\n2\n3\n4\n5\n6\n")?;
    let base = repo.tree("a", executable, original_blob_id)?;
    let renamed = repo.tree("b", noncanonical_executable, original_blob_id)?;
    let edited = repo.tree("a", executable, edited_blob_id)?;

    repo.assert_merges_to(
        (base, renamed, edited),
        None,
        ("b", executable, edited_blob_id),
        "two executable modes that differ only in permission bits merge to the canonical executable mode",
    )
}

#[test]
fn noncanonical_modes_in_modify_modify_merge() -> TestResult {
    let repo = Repo::new()?;
    let merged_blob_id = repo.blob("0\n1\n2\n3\n4\n5\n6\n")?;
    for (kind, noncanonical) in [(EntryKind::BlobExecutable, b"100775"), (EntryKind::Blob, b"100664")] {
        let canonical = kind.into();
        let noncanonical = EntryMode::from_bytes(noncanonical).expect("valid octal mode");
        assert_eq!(noncanonical.kind(), kind, "group-writable modes keep their kind");
        let base = repo.tree("a", canonical, repo.blob("1\n2\n3\n4\n5\n")?)?;
        let ours = repo.tree("a", noncanonical, repo.blob("0\n1\n2\n3\n4\n5\n")?)?;
        let theirs = repo.tree("a", canonical, repo.blob("1\n2\n3\n4\n5\n6\n")?)?;

        repo.assert_merges_to(
            (base, ours, theirs),
            None,
            ("a", canonical, merged_blob_id),
            "two modes that differ only in permission bits merge their content to the canonical mode",
        )?;
    }
    Ok(())
}

#[test]
fn replacing_a_file_with_links_to_its_content_keeps_a_link() -> TestResult {
    let repo = Repo::new()?;
    let link = EntryKind::Link.into();
    let noncanonical_link = EntryMode::from_bytes(b"120777").expect("valid octal mode");
    assert_eq!(
        noncanonical_link.kind(),
        EntryKind::Link,
        "the raw mode is still a symlink"
    );
    let target_id = repo.blob("target")?;
    let base = repo.tree("a", EntryKind::Blob.into(), target_id)?;
    let ours = repo.tree("a", link, target_id)?;
    let theirs = repo.tree("a", noncanonical_link, target_id)?;

    repo.assert_merges_to(
        (base, ours, theirs),
        None,
        ("a", link, target_id),
        "both sides turned the file into a link, so reusing its bytes as the target is not an ancestor pick",
    )
}

#[test]
fn noncanonical_executable_mode_in_add_add_merge_keeps_side_orientation() -> TestResult {
    let repo = Repo::new()?;
    let executable = EntryKind::BlobExecutable.into();
    let noncanonical_executable = EntryMode::from_bytes(b"100775").expect("valid octal mode");
    let base = repo.objects.write(&gix_object::Tree::empty())?;
    let ours = repo.tree("a", noncanonical_executable, repo.blob("ours\n")?)?;
    let theirs = repo.tree("a", executable, repo.blob("theirs\n")?)?;

    for (current, other, current_data, other_data) in
        [(ours, theirs, "ours", "theirs"), (theirs, ours, "theirs", "ours")]
    {
        let mut outcome = repo.merge((base, current, other), basic_merge_options().tree_merge)?;
        let entries = repo.entries(&mut outcome)?;
        let [(filename, mode, merged_blob_id)] = entries.as_slice() else {
            panic!("expected exactly one merged entry, got {entries:?}");
        };
        assert_eq!(
            (filename.as_str(), *mode),
            ("a", executable),
            "modes of one kind merge to the canonical mode"
        );
        let merged = repo.blob_data(*merged_blob_id)?;
        let current_pos = merged.find(current_data).expect("the current side is in the conflict");
        let other_pos = merged
            .find(&format!("\n{other_data}\n"))
            .expect("the other side is in the conflict");
        assert!(
            current_pos < other_pos,
            "the current side comes first in the conflict, as for byte-identical modes:\n{merged}"
        );
    }
    Ok(())
}

#[test]
fn mode_conflicts_are_independent_of_content_resolution() -> TestResult {
    use gix_merge::tree::{Resolution, ResolutionFailure, ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let base_tree_id = repo.tree("a", EntryKind::Link.into(), repo.blob("target")?)?;
    let our_blob_id = repo.blob("ours\n")?;
    let their_blob_id = repo.blob("theirs\n")?;
    let our_tree_id = repo.tree("a", EntryKind::BlobExecutable.into(), our_blob_id)?;
    let their_tree_id = repo.tree("a", EntryKind::Blob.into(), their_blob_id)?;

    for (current, other, current_kind, current_blob_id) in [
        (our_tree_id, their_tree_id, EntryKind::BlobExecutable, our_blob_id),
        (their_tree_id, our_tree_id, EntryKind::Blob, their_blob_id),
    ] {
        let mut options = basic_merge_options().tree_merge;
        options.blob_merge.text.conflict = gix_merge::blob::builtin_driver::text::Conflict::ResolveWithOurs;
        options.fail_on_conflict = Some(TreatAsUnresolved::git());
        let mut outcome = repo.merge((base_tree_id, current, other), options)?;
        assert_eq!(
            repo.entries(&mut outcome)?,
            [("a".to_owned(), current_kind.into(), current_blob_id)],
            "the current side provides both the conflicted mode and the explicitly selected content"
        );
        let [conflict] = outcome.conflicts.as_slice() else {
            panic!("expected one mode conflict, got {:?}", outcome.conflicts);
        };
        assert!(
            matches!(conflict.resolution, Err(ResolutionFailure::ModeConflict { .. })),
            "forcing a content merge cannot resolve an independent mode conflict"
        );
        let content_merge = conflict
            .content_merge()
            .expect("the mode conflict retains its content merge");
        assert_eq!(
            content_merge.merged_blob_id, current_blob_id,
            "the selected content is a blob"
        );
        assert_eq!(
            content_merge.resolution,
            gix_merge::blob::Resolution::CompleteWithAutoResolvedConflict,
            "the content conflict records that it was forcefully resolved"
        );
        assert!(
            outcome.failed_on_first_unresolved_conflict && outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
            "the independent mode conflict still stops a merge configured to fail on conflicts"
        );

        for (resolve_with, expected_tree_id) in [(ResolveWith::Ancestor, base_tree_id), (ResolveWith::Ours, current)] {
            let mut options = basic_merge_options().tree_merge;
            options.tree_conflicts = Some(resolve_with);
            options.fail_on_conflict = Some(TreatAsUnresolved::git());
            let mut outcome = repo.merge((base_tree_id, current, other), options)?;
            assert_eq!(
                outcome.tree.write(|tree| repo.objects.write(tree))?,
                expected_tree_id,
                "forcing the mode conflict restores the whole {resolve_with:?} entry"
            );
            let [conflict] = outcome.conflicts.as_slice() else {
                panic!("expected one forced mode conflict, got {:?}", outcome.conflicts);
            };
            assert!(
                matches!(
                    conflict.resolution,
                    Ok(Resolution::Forced(ResolutionFailure::ModeConflict { .. }))
                ),
                "the mode conflict retains its forced tree resolution"
            );
            assert!(
                conflict.content_merge().is_none(),
                "the selected entry does not retain the discarded content merge"
            );
            assert!(
                !outcome.failed_on_first_unresolved_conflict
                    && !outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
                "discarded content conflicts cannot prevent committing the selected tree"
            );
            assert!(
                outcome.has_unresolved_conflicts(TreatAsUnresolved::forced_resolution()),
                "callers can still detect the forced mode resolution"
            );
            outcome.conflicts.clear();
            let how = TreatAsUnresolved::forced_resolution();
            assert!(
                outcome.has_unresolved_conflicts(how) && !outcome.index.has_unmerged_entries(how),
                "reviewing a forced tree choice cannot resurrect discarded stages, even after filtering diagnostics"
            );
            let index = outcome.index.to_index(how, &repo.objects, Default::default())?;
            let selected = gix_index::State::from_tree(&expected_tree_id, &repo.objects, Default::default())?;
            assert_eq!(
                super::baseline::debug_entries(&index),
                super::baseline::debug_entries(&selected),
                "a forced tree choice remains a complete stage-zero index at every review sensitivity"
            );
        }
    }
    Ok(())
}

#[test]
fn fail_on_conflict_stops_before_merging_later_blobs() -> TestResult {
    use gix_merge::tree::TreatAsUnresolved;

    let repo = Repo::new()?;
    let tree = |side: &str, renamed: bool, collisions: bool| -> TestResult<ObjectId> {
        let mut entries = Vec::new();
        for filename in ["a", "b"] {
            // Retain enough distinct context to identify both renames unambiguously.
            let mut data = format!("{side} {filename}\n");
            for line in 1..10 {
                writeln!(data, "unchanged {filename} {line}")?;
            }
            entries.push(gix_object::tree::Entry {
                mode: EntryKind::Blob.into(),
                filename: if renamed {
                    format!("{filename}-{side}").into()
                } else {
                    filename.into()
                },
                oid: repo.blob(&data)?,
            });
            if collisions {
                entries.push(gix_object::tree::Entry {
                    mode: EntryKind::Blob.into(),
                    filename: format!("{filename}-ours").into(),
                    oid: repo.blob(&format!("added {filename}\n"))?,
                });
            }
        }
        Ok(repo.objects.write(&gix_object::Tree { entries })?)
    };
    let base_tree_id = tree("base", false, false)?;
    let mut buf = Vec::new();
    let base_entries = repo.objects.find_tree(&base_tree_id, &mut buf)?.to_owned().entries;
    let mut failures = Vec::new();
    for layout in ["ordinary", "rename", "rename-add"] {
        let our_tree_id = tree("ours", layout != "ordinary", false)?;
        let their_tree_id = tree("theirs", layout == "rename", layout == "rename-add")?;
        for (current, other) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
            for fail_fast in [false, true] {
                let mut options = basic_merge_options().tree_merge;
                if layout == "ordinary" {
                    options.rewrites = None;
                }
                options.fail_on_conflict = fail_fast.then(TreatAsUnresolved::git);
                let mut writes = 0;
                let mut outcome = gix_merge::tree(
                    &base_tree_id,
                    &current,
                    &other,
                    Default::default(),
                    &repo.objects,
                    |buf| {
                        writes += 1;
                        repo.objects.write_buf(gix_object::Kind::Blob, buf)
                    },
                    &mut gix_diff::tree::State::default(),
                    &mut new_diff_resource_cache(repo.root.path()),
                    &mut new_blob_merge_platform(repo.root.path(), None),
                    gix_merge::tree::Context::default(),
                    options,
                )?;
                assert_eq!(
                    outcome.failed_on_first_unresolved_conflict, fail_fast,
                    "both independent content merges conflict, including after rename normalization"
                );
                let expected = if fail_fast { 1 } else { 2 };
                assert_eq!(
                    outcome.conflicts.len(),
                    expected,
                    "only encountered conflicts are reported"
                );
                if fail_fast && layout == "ordinary" {
                    let entries = repo.entries(&mut outcome)?;
                    assert_eq!(entries.len(), 2, "an early exit keeps the unprocessed path");
                    assert_eq!(
                        entries
                            .iter()
                            .filter(|(name, _, blob_id)| base_entries
                                .iter()
                                .any(|entry| entry.filename == name.as_str() && entry.oid == *blob_id))
                            .count(),
                        1,
                        "the unprocessed path retains its ancestor entry in the partial tree"
                    );
                }
                let expected_writes = if !fail_fast && layout == "rename-add" {
                    4
                } else {
                    expected
                };
                if writes != expected_writes {
                    failures.push(format!(
                        "{layout}, reversed={}, fail_fast={fail_fast}: merged {writes} blobs, expected {expected_writes}",
                        current == their_tree_id,
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "stopping on the first unresolved conflict must avoid later blob merges, not only truncate their reports:\n{}",
        failures.join("\n")
    );
    Ok(())
}

#[test]
fn forced_relocated_additions_keep_the_accepted_directory() -> TestResult {
    use gix_merge::tree::{DirectoryRenames, ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let anchor_blob_id = repo.blob("anchor\n")?;
    let added_blob_id = repo.blob("added file\n")?;
    let link_blob_id = repo.blob("target")?;
    let base_tree_id = repo.tree_from_paths(&[("old/existing", EntryKind::Blob, anchor_blob_id)])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("old/existing", EntryKind::Blob, anchor_blob_id),
        ("old/new", EntryKind::Blob, added_blob_id),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("newdir/existing", EntryKind::Blob, anchor_blob_id),
        ("newdir/new", EntryKind::Link, link_blob_id),
    ])?;

    for directory_renames in [DirectoryRenames::Enabled, DirectoryRenames::Conflict] {
        for (current, other, kind, blob_id) in [
            (our_tree_id, their_tree_id, EntryKind::Blob, added_blob_id),
            (their_tree_id, our_tree_id, EntryKind::Link, link_blob_id),
        ] {
            let path = if directory_renames == DirectoryRenames::Conflict && current == our_tree_id {
                "old/new"
            } else {
                "newdir/new"
            };
            let expected_tree_id = repo.tree_from_paths(&[
                ("newdir/existing", EntryKind::Blob, anchor_blob_id),
                (path, kind, blob_id),
            ])?;
            let mut options = basic_merge_options().tree_merge;
            options.directory_renames = directory_renames;
            options.tree_conflicts = Some(ResolveWith::Ours);
            let mut outcome = repo.merge((base_tree_id, current, other), options)?;
            let actual_tree_id = outcome.tree.write(|tree| repo.objects.write(tree))?;
            assert_eq!(
                actual_tree_id,
                expected_tree_id,
                "{directory_renames:?}, current {kind:?}: choosing an entry preserves accepted relocation and rejects conflicting relocation\n{}\n{:#?}",
                super::baseline::visualize_tree(&actual_tree_id, &repo.objects, None),
                outcome.conflicts,
            );
            assert!(
                !outcome.has_unresolved_conflicts(TreatAsUnresolved::git())
                    && outcome.has_unresolved_conflicts(TreatAsUnresolved::forced_resolution()),
                "the chosen entry resolves the collision while retaining its forced-resolution provenance"
            );
        }
    }
    Ok(())
}

#[test]
fn forced_entries_report_only_retained_rename_premerges() -> TestResult {
    use gix_index::entry::{Mode, Stage};
    use gix_merge::tree::{ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let content = |first: &str| {
        format!(
            "{first}
one
two
three
four
five
six
seven
eight
"
        )
    };
    let base_blob_id = repo.blob(&content("base"))?;
    let our_blob_id = repo.blob(&content("OURS"))?;
    let base_tree_id = repo.tree("foo", EntryKind::Blob.into(), base_blob_id)?;
    let our_tree_id = repo.tree("bar", EntryKind::Blob.into(), our_blob_id)?;
    let their_blob_id = repo.blob(&content("THEIRS"))?;
    let added_blob_id = repo.blob("ADDED\n")?;

    for kind in [EntryKind::Link, EntryKind::BlobExecutable] {
        let their_tree_id =
            repo.tree_from_paths(&[("bar", kind, added_blob_id), ("foo", EntryKind::Blob, their_blob_id)])?;
        for (current, other) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
            for choice in [ResolveWith::Ancestor, ResolveWith::Ours] {
                let retained = choice == ResolveWith::Ours && current == our_tree_id;
                let context = format!("{kind:?}, {choice:?}, reversed={}", current == their_tree_id);
                let mut options = basic_merge_options().tree_merge;
                options.tree_conflicts = Some(choice);
                let mut full = repo.merge((base_tree_id, current, other), options.clone())?;
                let full_tree_id = full.tree.write(|tree| repo.objects.write(tree))?;
                assert_eq!(
                    full.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    retained,
                    "{context}: unresolved content belongs to the selected entry: {:#?}",
                    full.conflicts,
                );
                if retained {
                    let entries = repo.entries(&mut full)?;
                    let [(path, mode, merged_blob_id)] = entries.as_slice() else {
                        panic!("{context}: expected only the renamed file, got {entries:?}");
                    };
                    assert_eq!(
                        (path.as_str(), *mode),
                        ("bar", EntryKind::Blob.into()),
                        "{context}: the chosen regular file stays at its rename destination"
                    );
                    let data = repo.blob_data(*merged_blob_id)?;
                    assert!(
                        data.contains("OURS") && data.contains("THEIRS") && !data.contains("ADDED"),
                        "{context}: the selected entry retains its source premerge and discards the destination merge: {data}"
                    );
                    assert!(
                        full.conflicts
                            .iter()
                            .filter_map(gix_merge::tree::Conflict::content_merge)
                            .any(|merge| {
                                merge.merged_blob_id == *merged_blob_id
                                    && merge.resolution == gix_merge::blob::Resolution::Conflict
                            }),
                        "{context}: the retained conflict metadata describes the blob in the selected tree"
                    );
                } else {
                    assert_eq!(
                        full_tree_id,
                        if choice == ResolveWith::Ancestor {
                            base_tree_id
                        } else {
                            current
                        },
                        "{context}: choosing another entry discards the source premerge"
                    );
                    assert!(
                        full.conflicts.iter().all(|conflict| conflict.content_merge().is_none()),
                        "{context}: discarded premerges leave no content conflict metadata"
                    );
                }

                options.fail_on_conflict = Some(TreatAsUnresolved::git());
                let mut early = repo.merge((base_tree_id, current, other), options)?;
                assert_eq!(
                    early.tree.write(|tree| repo.objects.write(tree))?,
                    full_tree_id,
                    "{context}: early stopping must evaluate the final selected entry"
                );
                assert_eq!(
                    early.failed_on_first_unresolved_conflict, retained,
                    "{context}: only retained premerge markers stop the merge"
                );
                assert_eq!(
                    early.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    retained,
                    "{context}: early and full merges agree whether selected content is unresolved"
                );
                for (mode, outcome) in [("full", &full), ("early", &early)] {
                    let index = outcome
                        .index
                        .to_index(TreatAsUnresolved::git(), &repo.objects, Default::default())?;
                    assert_eq!(
                        outcome.index.has_unmerged_entries(TreatAsUnresolved::git()),
                        retained,
                        "{context}, {mode}: retained content must replace the stage-zero marker file with conflict stages: {:#?}",
                        super::baseline::debug_entries(&index),
                    );
                    if retained {
                        let entries: Vec<_> = index
                            .entries()
                            .iter()
                            .map(|entry| (entry.path(&index), entry.stage(), entry.mode, entry.id))
                            .collect();
                        assert_eq!(
                            entries,
                            [
                                (Stage::Base, base_blob_id),
                                (Stage::Ours, our_blob_id),
                                (Stage::Theirs, their_blob_id),
                            ]
                            .map(|(stage, blob_id)| (
                                bstr::BStr::new(b"bar"),
                                stage,
                                Mode::FILE,
                                blob_id
                            )),
                            "{context}, {mode}: source premerge inputs are staged at the selected output path"
                        );
                    } else {
                        let selected_index =
                            gix_index::State::from_tree(&full_tree_id, &repo.objects, Default::default())?;
                        assert_eq!(
                            super::baseline::debug_entries(&index),
                            super::baseline::debug_entries(&selected_index),
                            "{context}, {mode}: discarded premerges leave the selected entries at stage zero"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn forced_directories_restore_sources_of_crossing_renames() -> TestResult {
    let repo = Repo::new()?;
    let original_blob_id = repo.blob("original\0")?;
    let replacement_blob_id = repo.blob("replacement\0")?;
    let target_blob_id = repo.blob("target")?;
    let gitlink_commit_id =
        gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"submodule")?;

    // The other side moves h/b/a to a while replacing its source directory h with a file.
    let direct = (
        repo.tree_from_paths(&[("h/b/a", EntryKind::BlobExecutable, original_blob_id)])?,
        repo.tree_from_paths(&[
            ("h/b/a", EntryKind::BlobExecutable, original_blob_id),
            ("a/a", EntryKind::Link, target_blob_id),
        ])?,
        repo.tree_from_paths(&[
            ("a", EntryKind::BlobExecutable, original_blob_id),
            ("h", EntryKind::BlobExecutable, replacement_blob_id),
        ])?,
    );
    // Here the replacement of b comes from another rename, so rejecting b/b/b -> a
    // must also restore h after rejecting h -> b.
    let chained = (
        repo.tree_from_paths(&[
            ("b/b/b", EntryKind::Blob, original_blob_id),
            ("h", EntryKind::Commit, gitlink_commit_id),
        ])?,
        repo.tree_from_paths(&[
            ("b/b/b", EntryKind::Blob, original_blob_id),
            ("h", EntryKind::Commit, gitlink_commit_id),
            ("a/a", EntryKind::Link, target_blob_id),
        ])?,
        repo.tree_from_paths(&[
            ("a", EntryKind::BlobExecutable, original_blob_id),
            ("b", EntryKind::Commit, gitlink_commit_id),
        ])?,
    );
    for (base_tree_id, our_tree_id, their_tree_id) in [direct, chained] {
        for (current_tree_id, other_tree_id) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
            for (choice, expected_tree_id) in [
                (gix_merge::tree::ResolveWith::Ours, current_tree_id),
                (gix_merge::tree::ResolveWith::Ancestor, base_tree_id),
            ] {
                let mut options = basic_merge_options().tree_merge;
                options.tree_conflicts = Some(choice);
                options
                    .rewrites
                    .as_mut()
                    .expect("the test enables rename detection")
                    .percentage = Some(1.0);
                let mut actual = repo.merge((base_tree_id, current_tree_id, other_tree_id), options)?;
                assert_eq!(
                    actual.tree.write(|tree| repo.objects.write(tree))?,
                    expected_tree_id,
                    "choosing {choice:?} directories restores rejected rename sources and their parent topology"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn forced_tree_choices_discard_content_from_replaced_directories() -> TestResult {
    use gix_merge::tree::{DirectoryRenames, ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let blob_ids: Vec<_> = (0..8)
        .map(|index| repo.blob(&format!("binary payload {index}\0\n")))
        .collect::<TestResult<_>>()?;
    // Decoded from crash-e15731a4664b3c6e8368ca3a23dc33b12ee35c68, configuration 32.
    let base_tree_id = repo.tree_from_paths(&[
        ("a", EntryKind::BlobExecutable, blob_ids[7]),
        ("c/a", EntryKind::Blob, blob_ids[0]),
        ("e/e/e", EntryKind::Blob, blob_ids[7]),
        ("e/e/f", EntryKind::Blob, blob_ids[4]),
    ])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("a/a", EntryKind::Blob, blob_ids[4]),
        ("c/a", EntryKind::Blob, blob_ids[0]),
        ("e/e", EntryKind::Blob, blob_ids[1]),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("a", EntryKind::Blob, blob_ids[0]),
        ("e/b/e", EntryKind::Blob, blob_ids[4]),
        ("e/e/a", EntryKind::Blob, blob_ids[0]),
        ("e/e/e", EntryKind::Blob, blob_ids[0]),
    ])?;
    let mut options = basic_merge_options().tree_merge;
    options.directory_renames = DirectoryRenames::Conflict;
    options.tree_conflicts = Some(ResolveWith::Ours);
    options
        .rewrites
        .as_mut()
        .expect("the test enables rename tracking")
        .percentage = Some(1.0);
    for (current_tree_id, other_tree_id) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
        let mut outcome = repo.merge((base_tree_id, current_tree_id, other_tree_id), options.clone())?;
        for how in [
            TreatAsUnresolved::git(),
            TreatAsUnresolved {
                content_merge: gix_merge::tree::treat_as_unresolved::ContentMerge::ForcedResolution,
                ..TreatAsUnresolved::git()
            },
        ] {
            let index = outcome.index.to_index(how, &repo.objects, Default::default())?;
            assert_eq!(
                outcome.has_unresolved_conflicts(how),
                outcome.index.has_unmerged_entries(how),
                "only retained content can remain unresolved after forcing tree conflicts: {:#?}\nindex: {:#?}",
                outcome.conflicts,
                super::baseline::debug_entries(&index),
            );
        }
        if !outcome.index.has_unmerged_entries(TreatAsUnresolved::git()) {
            assert!(
                outcome
                    .conflicts
                    .iter()
                    .all(|conflict| conflict.content_merge().is_none()),
                "restoring an input with the binary merge's ID does not retain its content conflict"
            );
        }
        outcome.conflicts.clear();
        assert_eq!(
            outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
            outcome.index.has_unmerged_entries(TreatAsUnresolved::git()),
            "discarding diagnostics cannot change the final conflict verdict"
        );
    }
    Ok(())
}

#[test]
fn forced_directory_cancellation_discards_completed_content_conflicts() -> TestResult {
    use gix_merge::tree::{ResolveWith, TreatAsUnresolved};

    let repo = Repo::new()?;
    let original_blob_id = repo.blob("original\0")?;
    let target_blob_id = repo.blob("target")?;
    let content = |first: &str| {
        format!(
            "{first}
one
two
three
four
five
six
seven
eight
"
        )
    };
    let base_blob_id = repo.blob(&content("base"))?;
    let our_blob_id = repo.blob(&content("ours"))?;
    let their_blob_id = repo.blob(&content("theirs"))?;
    let base_tree_id = repo.tree_from_paths(&[
        ("h/b/a", EntryKind::BlobExecutable, original_blob_id),
        ("s", EntryKind::Blob, base_blob_id),
    ])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("h/b/a", EntryKind::BlobExecutable, original_blob_id),
        ("s", EntryKind::Blob, our_blob_id),
        ("a/a", EntryKind::Link, target_blob_id),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("a", EntryKind::BlobExecutable, original_blob_id),
        ("h", EntryKind::Blob, their_blob_id),
    ])?;
    let mut options = basic_merge_options().tree_merge;
    options.tree_conflicts = Some(ResolveWith::Ours);

    // h receives the conflicting s -> h content before a's directory choice
    // cancels h/b/a -> a and, through that restored source, also cancels s -> h.
    let mut full = repo.merge((base_tree_id, our_tree_id, their_tree_id), options.clone())?;
    let full_tree_id = full.tree.write(|tree| repo.objects.write(tree))?;
    assert_eq!(
        full_tree_id, our_tree_id,
        "the completed cancellation restores the current directory and its original source entry"
    );
    let full_index = full
        .index
        .to_index(TreatAsUnresolved::git(), &repo.objects, Default::default())?;
    let selected_index = gix_index::State::from_tree(&full_tree_id, &repo.objects, Default::default())?;

    // Early stopping occurs before that later topology choice. Its partial tree
    // still owns h's marker blob, so retaining those source stages is correct.
    options.fail_on_conflict = Some(TreatAsUnresolved::git());
    let mut early = repo.merge((base_tree_id, our_tree_id, their_tree_id), options)?;
    let early_tree_id = early.tree.write(|tree| repo.objects.write(tree))?;
    let early_content = early
        .conflicts
        .iter()
        .find_map(gix_merge::tree::Conflict::content_merge)
        .expect("early stopping retains the first content conflict");
    let expected_early_tree_id = repo.tree("h", EntryKind::Blob.into(), early_content.merged_blob_id)?;
    assert_eq!(
        early_tree_id, expected_early_tree_id,
        "the partial merge stops at h before processing the later directory cancellation"
    );
    assert!(
        early.failed_on_first_unresolved_conflict && early.has_unresolved_conflicts(TreatAsUnresolved::git()),
        "the partial tree still contains the content conflict"
    );
    let early_index = early
        .index
        .to_index(TreatAsUnresolved::git(), &repo.objects, Default::default())?;
    assert!(
        early.index.has_unmerged_entries(TreatAsUnresolved::git()),
        "the retained marker blob receives unresolved source stages"
    );
    let early_entries: Vec<_> = early_index
        .entries()
        .iter()
        .map(|entry| (entry.path(&early_index), entry.stage(), entry.id))
        .collect();
    assert_eq!(
        early_entries,
        [
            (gix_index::entry::Stage::Base, base_blob_id),
            (gix_index::entry::Stage::Ours, our_blob_id),
            (gix_index::entry::Stage::Theirs, their_blob_id),
        ]
        .map(|(stage, blob_id)| (bstr::BStr::new(b"h"), stage, blob_id)),
        "only the still-retained early content conflict receives source stages"
    );
    assert!(
        !full.has_unresolved_conflicts(TreatAsUnresolved::git())
            && !full.index.has_unmerged_entries(TreatAsUnresolved::git()),
        "discarded h content cannot conflict with its restored directory: {:#?}\nindex: {:#?}",
        full.conflicts,
        super::baseline::debug_entries(&full_index),
    );
    assert_eq!(
        super::baseline::debug_entries(&full_index),
        super::baseline::debug_entries(&selected_index),
        "a completed cancellation leaves the selected tree at stage zero"
    );
    Ok(())
}

#[test]
fn index_plans_are_repeatable_owned_snapshots() -> TestResult {
    use gix_index::entry::{Mode, Stage};
    use gix_merge::tree::{ResolutionFailure, TreatAsUnresolved, treat_as_unresolved::ContentMerge};

    let repo = Repo::new()?;
    let base_blob_id = repo.blob("base\n")?;
    let our_blob_id = repo.blob("ours\n")?;
    let their_blob_id = repo.blob("theirs\n")?;
    let kept_blob_id = repo.blob("unchanged\n")?;
    let added_blob_id = repo.blob("added\n")?;
    let base_tree_id = repo.tree_from_paths(&[
        ("conflict", EntryKind::Blob, base_blob_id),
        ("kept", EntryKind::Blob, kept_blob_id),
    ])?;
    let our_tree_id = repo.tree_from_paths(&[
        ("conflict", EntryKind::Blob, our_blob_id),
        ("kept", EntryKind::Blob, kept_blob_id),
        ("new", EntryKind::Blob, added_blob_id),
    ])?;
    let their_tree_id = repo.tree_from_paths(&[
        ("conflict", EntryKind::Blob, their_blob_id),
        ("kept", EntryKind::Blob, kept_blob_id),
    ])?;

    for favor_ours in [false, true] {
        for how in [
            TreatAsUnresolved::git(),
            TreatAsUnresolved::forced_resolution(),
            TreatAsUnresolved::undecidable(),
        ] {
            let mut options = basic_merge_options().tree_merge;
            if favor_ours {
                options.blob_merge.text.conflict = gix_merge::blob::builtin_driver::text::Conflict::ResolveWithOurs;
            }
            let mut outcome = repo.merge((base_tree_id, our_tree_id, their_tree_id), options)?;
            let unmerged = !favor_ours || how.content_merge == ContentMerge::ForcedResolution;
            let first = outcome.index.to_index(how, &repo.objects, Default::default())?;
            let entries: Vec<_> = first
                .entries()
                .iter()
                .map(|entry| (entry.path(&first), entry.stage(), entry.mode, entry.id))
                .collect();
            let path = bstr::BStr::new(b"conflict");
            let mut expected = if unmerged {
                vec![
                    (path, Stage::Base, Mode::FILE, base_blob_id),
                    (path, Stage::Ours, Mode::FILE, our_blob_id),
                    (path, Stage::Theirs, Mode::FILE, their_blob_id),
                ]
            } else {
                vec![(path, Stage::Unconflicted, Mode::FILE, our_blob_id)]
            };
            expected.extend([
                (bstr::BStr::new(b"kept"), Stage::Unconflicted, Mode::FILE, kept_blob_id),
                (bstr::BStr::new(b"new"), Stage::Unconflicted, Mode::FILE, added_blob_id),
            ]);
            assert_eq!(
                entries, expected,
                "the plan includes unchanged, added, and conflicted paths"
            );
            first.verify_entries()?;
            assert!(
                first
                    .entries()
                    .iter()
                    .all(|entry| !entry.flags.contains(gix_index::entry::Flags::REMOVE)),
                "fresh indexes do not retain marked-for-removal entries"
            );
            let repeated = outcome.index.to_index(how, &repo.objects, Default::default())?;
            assert_eq!(
                super::baseline::debug_entries(&first),
                super::baseline::debug_entries(&repeated),
                "repeated construction cannot append duplicate conflict stages"
            );

            outcome
                .conflicts
                .first_mut()
                .expect("the content merge is reported")
                .resolution = Err(ResolutionFailure::Unknown);
            assert_eq!(
                outcome.has_unresolved_conflicts(how),
                unmerged,
                "changing diagnostic resolution cannot change the merge verdict"
            );
            outcome.conflicts.clear();
            outcome.tree.upsert(["conflict"], EntryKind::Link, added_blob_id)?;
            outcome.tree.upsert(["extra"], EntryKind::Blob, added_blob_id)?;
            assert_eq!(
                outcome.has_unresolved_conflicts(how),
                unmerged,
                "filtering diagnostics and editing the tree cannot change the completed merge"
            );
            let plan = outcome.index.clone();
            drop(outcome);
            assert_eq!(plan.has_unmerged_entries(how), unmerged, "stage presence is immutable");
            assert_eq!(
                plan.has_unresolved_conflicts(how),
                unmerged,
                "the review verdict is immutable"
            );
            let detached = plan.to_index(how, &repo.objects, Default::default())?;
            assert_eq!(
                super::baseline::debug_entries(&first),
                super::baseline::debug_entries(&detached),
                "the owned plan preserves its index after diagnostics and tree edits, even after dropping the outcome"
            );
        }
    }
    Ok(())
}

#[test]
fn index_plans_validate_final_paths_with_their_modes() -> TestResult {
    let repo = Repo::new()?;
    let base_tree_id = repo.objects.write(&gix_object::Tree::empty())?;
    let blob_id = repo.blob("target")?;
    let their_tree_id = repo.tree("unrelated", EntryKind::Blob.into(), blob_id)?;
    for path in [".gitmodules", "nested/.gitmodules"] {
        for kind in [EntryKind::Link, EntryKind::Blob] {
            let our_tree_id = repo.tree_from_paths(&[(path, kind, blob_id)])?;
            let outcome = repo.merge(
                (base_tree_id, our_tree_id, their_tree_id),
                basic_merge_options().tree_merge,
            )?;
            let index = outcome.index.to_index(
                gix_merge::tree::TreatAsUnresolved::git(),
                &repo.objects,
                Default::default(),
            );
            if kind == EntryKind::Link {
                let error = index.expect_err("a newly added .gitmodules symlink is rejected");
                assert!(
                    matches!(
                        error.downcast_any_ref::<gix_validate::path::component::Error>(),
                        Some(gix_validate::path::component::Error::SymlinkedGitModules)
                    ),
                    "{path}: mode-specific validation applies to merge additions: {error:?}"
                );
            } else {
                assert_eq!(index?.entries().len(), 2, "a regular .gitmodules file remains valid");
            }
        }

        let invalid_base_tree_id = repo.tree_from_paths(&[(path, EntryKind::Link, blob_id)])?;
        let other_tree_id = repo.tree_from_paths(&[
            (path, EntryKind::Link, blob_id),
            ("unrelated", EntryKind::Blob, blob_id),
        ])?;
        for replace in [false, true] {
            let mut entries = Vec::new();
            if replace {
                entries.push((path, EntryKind::Blob, blob_id));
            }
            let selected_tree_id = repo.tree_from_paths(&entries)?;
            entries.push(("unrelated", EntryKind::Blob, blob_id));
            let expected_tree_id = repo.tree_from_paths(&entries)?;
            let expected = gix_index::State::from_tree(&expected_tree_id, &repo.objects, Default::default())?;
            for (current, other) in [(selected_tree_id, other_tree_id), (other_tree_id, selected_tree_id)] {
                let outcome = repo.merge((invalid_base_tree_id, current, other), basic_merge_options().tree_merge)?;
                let index = outcome.index.to_index(
                    gix_merge::tree::TreatAsUnresolved::git(),
                    &repo.objects,
                    Default::default(),
                )?;
                assert_eq!(
                    super::baseline::debug_entries(&index),
                    super::baseline::debug_entries(&expected),
                    "{path}, replace={replace}: a discarded unsafe base entry cannot invalidate the safe final index"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn index_plans_reject_duplicate_and_overlapping_paths() -> TestResult {
    let repo = Repo::new()?;
    let blob_id = repo.blob("data\n")?;
    let child_tree_id = repo.tree("x", EntryKind::Blob.into(), blob_id)?;
    let entry = |filename: &str, kind: EntryKind, oid| gix_object::tree::Entry {
        mode: kind.into(),
        filename: filename.into(),
        oid,
    };
    let mut accepted = Vec::new();
    for (case, entries) in [
        (
            "duplicate",
            vec![
                entry("a", EntryKind::Blob, blob_id),
                entry("a", EntryKind::Blob, blob_id),
            ],
        ),
        (
            "file with descendants",
            vec![
                entry("a", EntryKind::Blob, blob_id),
                entry("a.", EntryKind::Blob, blob_id),
                entry("a", EntryKind::Tree, child_tree_id),
            ],
        ),
        ("invalid leaf name", vec![entry("a/b", EntryKind::Blob, blob_id)]),
        ("empty directory name", vec![entry("", EntryKind::Tree, child_tree_id)]),
        (
            "invalid directory name",
            vec![entry("a/b", EntryKind::Tree, child_tree_id)],
        ),
        (
            "null object ID",
            vec![entry("a", EntryKind::Blob, gix_testtools::object_hash().null())],
        ),
    ] {
        let tree_id = repo.objects.write(&gix_object::Tree { entries })?;
        let outcome = repo.merge((tree_id, tree_id, tree_id), basic_merge_options().tree_merge)?;
        let index = outcome.index.to_index(
            gix_merge::tree::TreatAsUnresolved::git(),
            &repo.objects,
            gix_validate::path::component::Options {
                // `a.` is valid here; the overlapping paths must cause the rejection.
                protect_windows: false,
                ..Default::default()
            },
        );
        if index.is_ok() {
            accepted.push(case);
        }
    }
    assert!(
        accepted.is_empty(),
        "the fresh index must reject invalid object IDs and malformed paths even when unrelated entries separate them: {accepted:?}"
    );
    Ok(())
}
