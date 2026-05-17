mod list;
mod pop;
mod push;

use std::path::{Path, PathBuf};

use gix_ref::store::WriteReflog;

/// Open a read-enabled `gix_ref::file::Store` pointed at `git_dir`.
pub(crate) fn open_ref_store(git_dir: &Path) -> gix_ref::file::Store {
    let object_hash = gix_testtools::object_hash();
    gix_ref::file::Store::at(
        git_dir.to_owned(),
        gix_ref::store::init::Options {
            write_reflog: WriteReflog::Normal,
            object_hash,
            ..Default::default()
        },
    )
}

/// Open a `gix_odb::HandleArc` pointed at `<git_dir>/objects`.
///
/// Using `HandleArc` (backed by `Arc<Store>`) ensures the handle is `Send + Clone`,
/// which is required by the `push` and `pop` generic bounds.
pub(crate) fn open_odb(git_dir: &Path) -> gix_testtools::Result<gix_odb::HandleArc> {
    let object_hash = gix_testtools::object_hash();
    let odb = gix_odb::at_opts(
        git_dir.join("objects"),
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash,
            ..Default::default()
        },
    )?
    .into_arc()?;
    Ok(odb)
}

/// Return a fixed `gix_actor::Signature` suitable for test commits.
pub(crate) fn test_committer() -> gix_actor::Signature {
    gix_actor::Signature {
        name: "Test User".into(),
        email: "test@example.com".into(),
        time: gix_date::Time::new(1_700_000_000, 0),
    }
}

/// Resolve the OID that `HEAD` points to in the repo rooted at `worktree_path`.
pub(crate) fn head_commit_oid(
    _worktree_path: &Path,
    refs: &gix_ref::file::Store,
    odb: &impl gix_object::FindExt,
) -> gix_testtools::Result<gix_hash::ObjectId> {
    use gix_ref::file::ReferenceExt;
    let mut reference = refs.find("HEAD")?;
    Ok(reference.peel_to_id(refs, odb)?)
}

/// Return the tree OID for the given commit OID.
pub(crate) fn commit_tree(
    odb: &impl gix_object::FindExt,
    commit_oid: gix_hash::ObjectId,
) -> gix_testtools::Result<gix_hash::ObjectId> {
    let mut buf = Vec::new();
    let commit = odb.find_commit(&commit_oid, &mut buf)?;
    Ok(commit.tree())
}

/// Read a blob from a tree by name and return its content.
///
/// Only works for top-level file names in the tree.
pub(crate) fn blob_content_in_tree(
    odb: &impl gix_object::FindExt,
    tree_oid: gix_hash::ObjectId,
    filename: &[u8],
) -> gix_testtools::Result<Vec<u8>> {
    use bstr::ByteSlice;
    let mut buf = Vec::new();
    let tree = odb.find_tree(&tree_oid, &mut buf)?;
    for entry in &tree.entries {
        if entry.filename.as_bstr() == filename {
            let mut blob_buf = Vec::new();
            let blob = odb.find_blob(entry.oid, &mut blob_buf)?;
            return Ok(blob.data.to_owned());
        }
    }
    Err(format!("file {filename:?} not found in tree {tree_oid}").into())
}

/// Build a minimal `gix_diff::blob::Platform` for use with pop's `Context`.
pub(crate) fn new_diff_cache(worktree: &Path) -> gix_diff::blob::Platform {
    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
        Default::default(),
        gix_worktree::Stack::new(
            worktree,
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}

/// Build a `gix_merge::blob::Platform` for use with pop's `Context`.
pub(crate) fn new_blob_merge_platform(worktree: &Path) -> gix_merge::blob::Platform {
    let attributes = gix_worktree::Stack::new(
        worktree,
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
        Default::default(),
        Vec::new(),
        Vec::new(),
    );
    let filter = gix_merge::blob::Pipeline::new(
        Default::default(),
        gix_filter::Pipeline::default(),
        gix_merge::blob::pipeline::Options {
            large_file_threshold_bytes: 0,
        },
    );
    gix_merge::blob::Platform::new(
        filter,
        gix_merge::blob::pipeline::Mode::ToGit,
        attributes,
        vec![],
        Default::default(),
    )
}

/// Return `.git` directory for a worktree path.
pub(crate) fn git_dir(worktree_path: &Path) -> PathBuf {
    worktree_path.join(".git")
}
