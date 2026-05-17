//! Implementation of [`stash pop`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-pop).

use gix_hash::ObjectId;

/// Result of a successful [`function::pop`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The id of the stash commit that was applied + dropped.
    pub applied: ObjectId,

    /// The new value of `refs/stash` after dropping the applied entry — `None`
    /// when no older stash entries remain.
    pub new_top: Option<ObjectId>,

    /// Whether the apply step produced merge conflicts.
    ///
    /// When `true` the merged result (including conflict markers) has been
    /// written to the working tree, but `refs/stash` has **not** been dropped
    /// so that the stash entry can be re-applied after manual resolution;
    /// [`Outcome::had_conflicts`] is set to `true`.
    pub had_conflicts: bool,
}

/// Errors returned by [`function::pop`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `refs/stash` is unborn — no stash entries exist.
    #[error("no stash entries to pop (refs/stash is unborn)")]
    NoStash,

    /// Looking up `refs/stash` in the ref store failed.
    #[error("failed to read refs/stash from the ref store")]
    FindRef(#[from] gix_ref::file::find::Error),

    /// An object could not be found in the database.
    #[error("required object was not found in the object database")]
    FindObject(#[from] gix_object::find::existing_object::Error),

    /// The reflog for `refs/stash` could not be read.
    #[error("failed to read the refs/stash reflog")]
    Io(#[from] std::io::Error),

    /// A reflog line for `refs/stash` failed to decode.
    #[error("failed to decode a reflog line for refs/stash")]
    DecodeReflog(#[from] gix_ref::file::log::iter::reverse::Error),

    /// Preparing the ref transaction failed.
    #[error("failed to prepare the refs/stash ref transaction")]
    PrepareTransaction(#[from] gix_ref::file::transaction::prepare::Error),

    /// Committing the ref transaction failed.
    #[error("failed to commit the refs/stash ref transaction")]
    CommitTransaction(#[from] gix_ref::file::transaction::commit::Error),

    /// The 3-way tree merge failed.
    #[error("failed to merge stash tree into the working tree")]
    Merge(#[from] gix_merge::tree::Error),

    /// Writing the merged tree to the object database failed.
    #[error("failed to write merged tree to the object database")]
    WriteTree(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// Constructing the merge-result index for the worktree checkout failed.
    #[error("failed to construct index from merged tree")]
    IndexFromTree(#[from] gix_index::init::from_tree::Error),

    /// Writing the merge result to the working tree failed.
    #[error("failed to write merge result to the working tree")]
    Checkout(#[from] gix_worktree_state::checkout::Error),

    /// Reading a blob to restore an untracked file failed.
    #[error("failed to read untracked blob for restore at {path:?}")]
    RestoreUntracked {
        /// The path where the untracked file was to be written.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Repository-level plumbing handles required by [`function::pop`].
///
/// `Objects` must implement [`gix_object::Find`], [`gix_object::FindHeader`],
/// and [`gix_object::Write`] — all of which are satisfied by the typical
/// `gix_odb::Handle` / `gix::Repository` object store.
pub struct Context<'a, Objects> {
    /// The file-based ref store for the repository.
    pub refs: &'a gix_ref::file::Store,
    /// A combined readable + writable ODB handle.
    pub objects: &'a Objects,
    /// Identity and timestamp to use for the ref-transaction committer line.
    pub committer: gix_actor::SignatureRef<'a>,
    /// Absolute path to the working-tree root.
    pub worktree: &'a std::path::Path,
    /// Pre-configured blob merge platform for 3-way content merges.
    pub blob_merge: &'a mut gix_merge::blob::Platform,
    /// Pre-configured diff resource cache for rename tracking during tree merge.
    pub diff_cache: &'a mut gix_diff::blob::Platform,
    /// Options controlling the worktree checkout after a successful merge.
    pub checkout_options: gix_worktree_state::checkout::Options,
}

pub(crate) mod function {
    use std::path::Path;

    use bstr::ByteSlice;
    use gix_hash::ObjectId;
    use gix_object::FindExt;
    use gix_ref::{
        FullName,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    };

    use super::{Context, Error, Outcome};

    /// Apply the latest stash entry to the working tree and drop it from
    /// `refs/stash`.
    ///
    /// # Parameters
    ///
    /// * `head_tree` — OID of the root tree of the current `HEAD` commit.
    ///   This is the "ours" side of the 3-way merge.
    ///
    /// # Merge semantics
    ///
    /// Performs a 3-way merge of:
    /// * **base** — the tree of `parent[0]` of the stash commit (HEAD at stash
    ///   time)
    /// * **ours** — `head_tree` (current HEAD tree)
    /// * **theirs** — the stash commit's own tree (working-tree state at stash
    ///   time)
    ///
    /// If the merge is clean the result is checked out into the working tree
    /// and `refs/stash` is dropped.  On conflict, the working tree receives
    /// conflict markers and `refs/stash` is left in place so the entry can be
    /// re-applied after manual resolution; [`Outcome::had_conflicts`] is set
    /// to `true` in that case.
    ///
    /// If the stash commit has a third parent (`parent[2]`), its tree is
    /// treated as the untracked-files snapshot and those files are restored to
    /// the working tree after a clean merge.
    pub fn pop<Objects>(ctx: Context<'_, Objects>, head_tree: ObjectId) -> Result<Outcome, Error>
    where
        Objects: gix_object::Find + gix_object::FindHeader + gix_object::Write + Send + Clone,
    {
        let Context {
            refs,
            objects,
            committer,
            worktree,
            blob_merge,
            diff_cache,
            checkout_options,
        } = ctx;

        let stash_ref: FullName = "refs/stash".try_into().expect("refs/stash is a valid ref name");

        // ------------------------------------------------------------------ //
        // Read the current tip of refs/stash.
        // ------------------------------------------------------------------ //
        let stash_oid = refs
            .try_find(stash_ref.as_ref())?
            .ok_or(Error::NoStash)?
            .target
            .try_id()
            .map(ToOwned::to_owned)
            .ok_or(Error::NoStash)?;

        // ------------------------------------------------------------------ //
        // Decode the stash commit.
        // ------------------------------------------------------------------ //
        let mut commit_buf = Vec::new();
        let stash_commit = objects.find_commit(&stash_oid, &mut commit_buf)?;

        // The stash commit's own tree is the WIP working-tree state.
        let stash_tree = stash_commit.tree();

        // parent[0] is the original HEAD at stash time — the merge base.
        let base_commit = stash_commit.parents().next().ok_or(Error::NoStash)?;

        // parent[2] (optional) is the untracked-files commit.
        let untracked_commit: Option<ObjectId> = stash_commit.parents().nth(2);

        drop(stash_commit);

        // Resolve the base tree from parent[0].
        let mut base_buf = Vec::new();
        let base_commit_obj = objects.find_commit(&base_commit, &mut base_buf)?;
        let base_tree = base_commit_obj.tree();
        drop(base_commit_obj);

        // ------------------------------------------------------------------ //
        // 3-way tree merge:
        //   base  = HEAD tree at stash time   (stash parent[0]'s tree)
        //   ours  = current HEAD tree
        //   theirs = stash WIP tree
        // ------------------------------------------------------------------ //
        let mut diff_state = gix_diff::tree::State::default();
        let labels = gix_merge::blob::builtin_driver::text::Labels::default();

        let merge_outcome = gix_merge::tree(
            &base_tree,
            &head_tree,
            &stash_tree,
            labels,
            objects,
            |buf: &[u8]| objects.write_buf(gix_object::Kind::Blob, buf),
            &mut diff_state,
            diff_cache,
            blob_merge,
            gix_merge::tree::Options::default(),
        )?;

        let had_conflicts = merge_outcome.has_unresolved_conflicts(gix_merge::tree::TreatAsUnresolved::git());

        // Write the merged tree to the ODB.
        // `tree` is an Editor; we write it by consuming it via the write() method.
        let mut merge_tree_editor = merge_outcome.tree;
        let merged_tree_oid = merge_tree_editor.write(|tree| objects.write(tree).map_err(Error::WriteTree))?;

        // ------------------------------------------------------------------ //
        // Checkout merged tree into the working tree.
        // ------------------------------------------------------------------ //
        let mut merged_index = gix_index::State::from_tree(
            &merged_tree_oid,
            objects,
            gix_validate::path::component::Options::default(),
        )?;
        let should_interrupt = std::sync::atomic::AtomicBool::new(false);
        gix_worktree_state::checkout(
            &mut merged_index,
            worktree,
            objects.clone(),
            &gix_features::progress::Discard,
            &gix_features::progress::Discard,
            &should_interrupt,
            checkout_options,
        )?;

        // ------------------------------------------------------------------ //
        // Restore untracked files if the stash had a parent[2] and merge clean.
        // ------------------------------------------------------------------ //
        if !had_conflicts {
            if let Some(untracked_commit_oid) = untracked_commit {
                let mut uc_buf = Vec::new();
                let uc_commit = objects.find_commit(&untracked_commit_oid, &mut uc_buf)?;
                let untracked_tree_oid = uc_commit.tree();
                drop(uc_commit);
                restore_tree_to_worktree(&untracked_tree_oid, worktree, objects)?;
            }
        }

        // ------------------------------------------------------------------ //
        // Look up the second-newest entry so we know what to set refs/stash to
        // after dropping the top.
        // ------------------------------------------------------------------ //
        let mut reflog_buf = vec![0u8; 4 * 1024];
        let new_top: Option<ObjectId> = {
            let mut iter = refs
                .reflog_iter_rev(stash_ref.as_ref(), &mut reflog_buf)
                .map_err(|e| match e {
                    gix_ref::file::log::Error::Io(io) => Error::Io(io),
                    gix_ref::file::log::Error::RefnameValidation(_) => {
                        unreachable!("refs/stash is always a valid ref name")
                    }
                })?
                .ok_or(Error::NoStash)?;

            // Index 0 = current tip (what we are popping); index 1 = the one before.
            iter.nth(1).transpose()?.map(|line| line.new_oid)
        };

        // ------------------------------------------------------------------ //
        // Drop or update refs/stash — only if the merge was clean.
        // ------------------------------------------------------------------ //
        if !had_conflicts {
            let edit = if let Some(next_oid) = new_top {
                RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: true,
                            message: "drop stash".into(),
                        },
                        expected: PreviousValue::MustExistAndMatch(gix_ref::Target::Object(stash_oid)),
                        new: gix_ref::Target::Object(next_oid),
                    },
                    name: stash_ref,
                    deref: false,
                }
            } else {
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::MustExistAndMatch(gix_ref::Target::Object(stash_oid)),
                        log: RefLog::AndReference,
                    },
                    name: stash_ref,
                    deref: false,
                }
            };

            let committer_owned: gix_actor::Signature = committer.into();
            let mut time_buf = gix_date::parse::TimeBuf::default();
            refs.transaction()
                .prepare(
                    std::iter::once(edit),
                    gix_lock::acquire::Fail::Immediately,
                    gix_lock::acquire::Fail::Immediately,
                )?
                .commit(committer_owned.to_ref(&mut time_buf))?;
        }

        Ok(Outcome {
            applied: stash_oid,
            new_top,
            had_conflicts,
        })
    }

    /// Walk `tree_oid` recursively and write every blob to its corresponding
    /// path under `dir`.  Used to restore untracked files from `parent[2]`.
    fn restore_tree_to_worktree(
        tree_oid: &gix_hash::oid,
        dir: &Path,
        find: &impl gix_object::FindExt,
    ) -> Result<(), super::Error> {
        let mut buf = Vec::new();
        restore_tree_recursive(tree_oid, dir, find, &mut buf)
    }

    fn restore_tree_recursive(
        tree_oid: &gix_hash::oid,
        dir: &Path,
        find: &impl gix_object::FindExt,
        buf: &mut Vec<u8>,
    ) -> Result<(), super::Error> {
        use gix_object::tree::EntryKind;

        buf.clear();
        let tree = find.find_tree(tree_oid, buf)?.to_owned();

        for entry in tree.entries {
            let name_bytes: &bstr::BStr = entry.filename.as_ref();
            let entry_path = dir.join(gix_path::from_bstr(name_bytes));

            match entry.mode.kind() {
                EntryKind::Tree => {
                    std::fs::create_dir_all(&entry_path).map_err(|e| super::Error::RestoreUntracked {
                        path: entry_path.clone(),
                        source: e,
                    })?;
                    let mut sub_buf = Vec::new();
                    restore_tree_recursive(&entry.oid, &entry_path, find, &mut sub_buf)?;
                }
                EntryKind::Blob | EntryKind::BlobExecutable => {
                    let mut blob_buf = Vec::new();
                    let blob = find.find_blob(&entry.oid, &mut blob_buf)?;
                    if let Some(parent) = entry_path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| super::Error::RestoreUntracked {
                            path: entry_path.clone(),
                            source: e,
                        })?;
                    }
                    std::fs::write(&entry_path, blob.data).map_err(|e| super::Error::RestoreUntracked {
                        path: entry_path,
                        source: e,
                    })?;
                }
                EntryKind::Link => {
                    let mut blob_buf = Vec::new();
                    let blob = find.find_blob(&entry.oid, &mut blob_buf)?;
                    let target = gix_path::from_bstr(blob.data.as_bstr());
                    if let Some(parent) = entry_path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| super::Error::RestoreUntracked {
                            path: entry_path.clone(),
                            source: e,
                        })?;
                    }
                    // Remove any existing entry before creating the symlink.
                    let _ = std::fs::remove_file(&entry_path);
                    std::os::unix::fs::symlink(&target, &entry_path).map_err(|e| super::Error::RestoreUntracked {
                        path: entry_path,
                        source: e,
                    })?;
                }
                EntryKind::Commit => {
                    // Submodule — skip.
                }
            }
        }
        Ok(())
    }
}
