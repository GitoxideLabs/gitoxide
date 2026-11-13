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

    /// Merge `other` into `current` and return the entries of the merged tree.
    fn merge(
        &self,
        (base, current, other): (ObjectId, ObjectId, ObjectId),
        symlink_conflicts: Option<ResolveWith>,
    ) -> TestResult<Vec<(String, EntryMode, ObjectId)>> {
        let mut options = basic_merge_options().tree_merge;
        options.symlink_conflicts = symlink_conflicts;
        let mut outcome = gix_merge::tree(
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
        )?;
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
            let actual = self.merge((base, current, other), symlink_conflicts)?;
            assert_eq!(actual, [(filename.to_owned(), mode, oid)], "{message}");
        }
        Ok(())
    }
}

#[test]
fn symlink_ancestor_resolution_restores_a_non_symlink_ancestor() -> TestResult {
    let repo = Repo::new()?;
    let commit_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"submodule")?;
    let ours = repo.tree("a", EntryKind::Link.into(), repo.blob("ours")?)?;
    let theirs = repo.tree("a", EntryKind::Link.into(), repo.blob("theirs")?)?;

    for (kind, ancestor_id) in [(EntryKind::Commit, commit_id), (EntryKind::Blob, repo.blob("target")?)] {
        let base = repo.tree("a", kind.into(), ancestor_id)?;
        repo.assert_merges_to(
            (base, ours, theirs),
            Some(ResolveWith::Ancestor),
            ("a", kind.into(), ancestor_id),
            "choosing the ancestor restores its whole entry instead of writing its id as a symlink target",
        )?;
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
        let entries = repo.merge((base, current, other), None)?;
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
