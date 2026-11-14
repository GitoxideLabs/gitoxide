//! Tree merges whose inputs Git's porcelain cannot produce, like gitlinks replaced by symlinks or noncanonical modes.
//! Trees are written to an in-memory object database so each case states its entries directly.

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

    fn blob_data(&self, blob_id: ObjectId) -> TestResult<String> {
        let mut buf = Vec::new();
        let blob = self.objects.find_blob(&blob_id, &mut buf)?;
        Ok(String::from_utf8(blob.data.to_vec())?)
    }

    /// Merge `other` into `current`.
    fn merge(
        &self,
        (base, current, other): (ObjectId, ObjectId, ObjectId),
        options: gix_merge::tree::Options,
    ) -> TestResult<gix_merge::tree::Outcome<'_>> {
        Ok(gix_merge::tree(
            &base,
            &current,
            &other,
            Default::default(),
            &self.objects,
            |buf| self.objects.write_buf(gix_object::Kind::Blob, buf),
            &mut gix_diff::tree::State::default(),
            &mut new_diff_resource_cache(self.root.path()),
            &mut new_blob_merge_platform(self.root.path(), None),
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
fn symlink_ancestor_resolution_restores_the_whole_ancestor_entry() -> TestResult {
    use gix_merge::tree::{TreatAsUnresolved, apply_index_entries::RemovalMode, treat_as_unresolved};

    let repo = Repo::new()?;
    let commit_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"submodule")?;
    let ours = repo.tree("a", EntryKind::Link.into(), repo.blob("ours")?)?;
    let theirs = repo.tree("a", EntryKind::Link.into(), repo.blob("theirs")?)?;

    for (kind, ancestor_id) in [
        (EntryKind::Commit, commit_id),
        (EntryKind::Blob, repo.blob("target")?),
        (EntryKind::Link, repo.blob("target")?),
    ] {
        let base = repo.tree("a", kind.into(), ancestor_id)?;
        for (current, other) in [(ours, theirs), (theirs, ours)] {
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
            let base_entry = conflict.entries()[0].expect("the conflict has an ancestor");
            assert_eq!(
                (base_entry.mode, base_entry.id),
                (kind.into(), ancestor_id),
                "the original mode identifies the restored object kind"
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
                    let mut index = gix_index::State::from_tree(&base, &repo.objects, Default::default())?;
                    assert_eq!(
                        outcome.index_changed_after_applying_conflicts(&mut index, how, RemovalMode::Prune),
                        unresolved,
                        "only a policy that considers this unresolved should add conflict stages"
                    );
                    let actual: Vec<_> = index
                        .entries()
                        .iter()
                        .map(|entry| (entry.stage_raw(), entry.mode, entry.id))
                        .collect();
                    let expected = if unresolved {
                        conflict
                            .entries()
                            .into_iter()
                            .enumerate()
                            .map(|(idx, entry)| {
                                let entry = entry.expect("all three conflict stages exist");
                                ((idx + 1) as u32, entry.mode.into(), entry.id)
                            })
                            .collect::<Vec<_>>()
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
fn forced_content_resolution_preserves_a_mode_conflict() -> TestResult {
    use gix_merge::tree::{ResolutionFailure, TreatAsUnresolved};

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
    }
    Ok(())
}
