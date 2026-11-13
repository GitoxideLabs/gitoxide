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
    root: std::path::PathBuf,
    objects: gix_odb::memory::Proxy<gix_odb::Handle>,
}

impl Repo {
    fn new() -> TestResult<Self> {
        let root = gix_testtools::scripted_fixture_read_only("make_blob_repo.sh")?;
        let objects = gix_odb::memory::Proxy::new(
            gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?,
            gix_testtools::object_hash(),
        );
        Ok(Repo { root, objects })
    }

    fn blob(&self, data: &str) -> TestResult<ObjectId> {
        Ok(self.objects.write_buf(gix_object::Kind::Blob, data.as_bytes())?)
    }

    /// Write a tree from `entries`, which must already be in Git's tree order.
    fn tree(&self, entries: &[(&str, EntryMode, ObjectId)]) -> TestResult<ObjectId> {
        let tree = gix_object::Tree {
            entries: entries
                .iter()
                .map(|(filename, mode, oid)| gix_object::tree::Entry {
                    mode: *mode,
                    filename: (*filename).into(),
                    oid: *oid,
                })
                .collect(),
        };
        Ok(self.objects.write(&tree)?)
    }

    fn merge(
        &self,
        (base_tree_id, our_tree_id, their_tree_id): (ObjectId, ObjectId, ObjectId),
        symlink_conflicts: Option<ResolveWith>,
    ) -> TestResult<gix_object::Tree> {
        let mut options = basic_merge_options().tree_merge;
        options.symlink_conflicts = symlink_conflicts;
        let mut diff_resource_cache = new_diff_resource_cache(&self.root);
        let mut blob_merge = new_blob_merge_platform(&self.root, None);
        let mut outcome = gix_merge::tree(
            &base_tree_id,
            &our_tree_id,
            &their_tree_id,
            Default::default(),
            &self.objects,
            |buf| self.objects.write_buf(gix_object::Kind::Blob, buf),
            &mut gix_diff::tree::State::default(),
            &mut diff_resource_cache,
            &mut blob_merge,
            options,
        )?;
        let merged_tree_id = outcome.tree.write(|tree| self.objects.write(tree))?;
        Ok(self.objects.find_tree(&merged_tree_id, &mut Vec::new())?.into())
    }
}

fn executable() -> EntryMode {
    EntryKind::BlobExecutable.into()
}

fn noncanonical_executable() -> EntryMode {
    let mode = EntryMode::from_bytes(b"100775").expect("valid octal mode");
    assert_eq!(
        mode.kind(),
        EntryKind::BlobExecutable,
        "group-writable modes are still executable blobs"
    );
    assert_ne!(
        mode,
        executable(),
        "the raw mode differs from the canonical executable mode"
    );
    mode
}

#[test]
fn symlink_ancestor_resolution_restores_a_gitlink_ancestor() -> TestResult {
    let repo = Repo::new()?;
    let commit_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Commit, b"submodule")?;
    let base = repo.tree(&[("a", EntryKind::Commit.into(), commit_id)])?;
    let ours = repo.tree(&[("a", EntryKind::Link.into(), repo.blob("ours")?)])?;
    let theirs = repo.tree(&[("a", EntryKind::Link.into(), repo.blob("theirs")?)])?;

    for (current, other) in [(ours, theirs), (theirs, ours)] {
        let merged = repo.merge((base, current, other), Some(ResolveWith::Ancestor))?;
        let entry = &merged.entries[0];
        assert_eq!(
            (entry.mode.kind(), entry.oid),
            (EntryKind::Commit, commit_id),
            "choosing the ancestor restores the whole gitlink entry instead of writing its commit id as a symlink target"
        );
    }
    Ok(())
}

#[test]
fn symlink_ancestor_resolution_restores_a_regular_file_ancestor() -> TestResult {
    let repo = Repo::new()?;
    let ancestor_blob_id = repo.blob("target")?;
    let base = repo.tree(&[("a", EntryKind::Blob.into(), ancestor_blob_id)])?;
    let ours = repo.tree(&[("a", EntryKind::Link.into(), repo.blob("ours")?)])?;
    let theirs = repo.tree(&[("a", EntryKind::Link.into(), repo.blob("theirs")?)])?;

    for (current, other) in [(ours, theirs), (theirs, ours)] {
        let merged = repo.merge((base, current, other), Some(ResolveWith::Ancestor))?;
        let entry = &merged.entries[0];
        assert_eq!(
            (entry.mode.kind(), entry.oid),
            (EntryKind::Blob, ancestor_blob_id),
            "choosing the ancestor restores the regular file instead of turning its content into a symlink target"
        );
    }
    Ok(())
}

#[test]
fn noncanonical_executable_mode_in_rename_edit_merge() -> TestResult {
    let repo = Repo::new()?;
    let base = repo.tree(&[("a", executable(), repo.blob("1\n2\n3\n4\n5\n")?)])?;
    let renamed = repo.tree(&[("b", noncanonical_executable(), repo.blob("1\n2\n3\n4\n5\n")?)])?;
    let edited_blob_id = repo.blob("1\n2\n3\n4\n5\n6\n")?;
    let edited = repo.tree(&[("a", executable(), edited_blob_id)])?;

    for (current, other) in [(renamed, edited), (edited, renamed)] {
        let merged = repo.merge((base, current, other), None)?;
        let entry = &merged.entries[0];
        assert_eq!(
            (entry.filename.as_slice(), entry.mode, entry.oid),
            (b"b".as_slice(), executable(), edited_blob_id),
            "two executable modes that differ only in permission bits merge to the canonical executable mode"
        );
    }
    Ok(())
}

#[test]
fn noncanonical_executable_mode_in_modify_modify_merge() -> TestResult {
    let repo = Repo::new()?;
    let base = repo.tree(&[("a", executable(), repo.blob("1\n2\n3\n4\n5\n")?)])?;
    let ours = repo.tree(&[("a", noncanonical_executable(), repo.blob("0\n1\n2\n3\n4\n5\n")?)])?;
    let theirs = repo.tree(&[("a", executable(), repo.blob("1\n2\n3\n4\n5\n6\n")?)])?;
    let merged_blob_id = repo.blob("0\n1\n2\n3\n4\n5\n6\n")?;

    for (current, other) in [(ours, theirs), (theirs, ours)] {
        let merged = repo.merge((base, current, other), None)?;
        let entry = &merged.entries[0];
        assert_eq!(
            (entry.mode, entry.oid),
            (executable(), merged_blob_id),
            "two executable modes that differ only in permission bits merge to the canonical executable mode"
        );
    }
    Ok(())
}

#[test]
fn noncanonical_regular_mode_in_modify_modify_merge() -> TestResult {
    let repo = Repo::new()?;
    let regular: EntryMode = EntryKind::Blob.into();
    let noncanonical_regular = EntryMode::from_bytes(b"100664").expect("valid octal mode");
    assert_eq!(
        noncanonical_regular.kind(),
        EntryKind::Blob,
        "group-writable modes are still regular blobs"
    );
    let base = repo.tree(&[("a", regular, repo.blob("1\n2\n3\n4\n5\n")?)])?;
    let ours = repo.tree(&[("a", noncanonical_regular, repo.blob("0\n1\n2\n3\n4\n5\n")?)])?;
    let theirs = repo.tree(&[("a", regular, repo.blob("1\n2\n3\n4\n5\n6\n")?)])?;
    let merged_blob_id = repo.blob("0\n1\n2\n3\n4\n5\n6\n")?;

    for (current, other) in [(ours, theirs), (theirs, ours)] {
        let merged = repo.merge((base, current, other), None)?;
        let entry = &merged.entries[0];
        assert_eq!(
            (entry.mode, entry.oid),
            (regular, merged_blob_id),
            "two regular modes that differ only in permission bits merge their content to the canonical regular mode"
        );
    }
    Ok(())
}
