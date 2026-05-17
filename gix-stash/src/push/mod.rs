//! Implementation of [`stash push`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-push).

use bstr::BString;
use gix_hash::ObjectId;

/// Options controlling [`function::push`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Include untracked (but not git-ignored) files in `parent[2]` of the
    /// stash commit and remove them from the working tree.
    ///
    /// Note that `.gitignore` rules are **not** consulted in the current
    /// implementation — all untracked files are included.  A future
    /// implementation will wire up `gix-worktree`'s exclude stack to provide
    /// full `.gitignore` support.
    ///
    /// TODO(gix-stash): respect .gitignore via `gix-worktree` exclude stack.
    pub include_untracked: bool,

    /// Also include ignored files when `include_untracked` is set.  Has no
    /// effect on its own.
    ///
    /// Not yet implemented; included for API completeness.
    pub include_ignored: bool,

    /// Keep the index state intact in the working tree after stashing — the
    /// stash still captures it, but the on-disk working tree continues to
    /// reflect what was staged.
    pub keep_index: bool,

    /// Optional explicit message — written to the stash commit subject and
    /// the reflog entry.  When `None`, the message defaults to
    /// `WIP on <branch>: <short-hash> <subject>`.
    pub message: Option<BString>,
}

/// Result of a successful [`function::push`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The id of the newly-created stash commit (now `refs/stash`).
    pub stash: ObjectId,

    /// The id of the index-state commit (`parent[1]` of the stash commit).
    pub index_commit: ObjectId,

    /// The id of the untracked-files commit (`parent[2]`), if one was created.
    pub untracked_commit: Option<ObjectId>,

    /// The previous value of `refs/stash`, now reachable only via reflog.
    pub previous: Option<ObjectId>,
}

/// Errors returned by [`function::push`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The repository has no commits yet — stash requires at least one.
    #[error("cannot stash in an empty repository (HEAD has no commits)")]
    EmptyRepository,

    /// There are no local changes to stash.
    #[error("no local changes to save")]
    NoLocalChanges,

    /// An index entry's mode could not be converted to a tree entry mode.
    #[error("index entry at path {path:?} has an unrecognised file mode ({mode:#o})")]
    InvalidIndexEntryMode {
        /// Repository-relative path of the offending entry.
        path: BString,
        /// The raw mode bits that could not be mapped.
        mode: u32,
    },

    /// A tree could not be written to the object database.
    #[error("failed to write tree object to the object database")]
    WriteTree(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// A blob could not be written to the object database.
    #[error("failed to write blob object to the object database")]
    WriteBlob(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// A commit could not be written to the object database.
    #[error("failed to write commit object to the object database")]
    WriteCommit(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// An object could not be found in the database.
    #[error("required object was not found in the object database")]
    FindObject(#[from] gix_object::find::existing_object::Error),

    /// Reading a worktree file failed while building the WIP tree or untracked-files tree.
    #[error("failed to read worktree file at {path:?}")]
    ReadFile {
        /// The path that failed.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Walking the worktree for untracked files failed.
    #[error("failed to walk the worktree directory")]
    WalkWorktree(#[source] std::io::Error),

    /// The tree editor encountered a problem assembling a tree.
    #[error("failed to assemble tree from index or worktree entries")]
    TreeEditor(#[from] gix_object::tree::editor::Error),

    /// Preparing the ref transaction failed.
    #[error("failed to prepare the refs/stash ref transaction")]
    PrepareTransaction(#[from] gix_ref::file::transaction::prepare::Error),

    /// Committing the ref transaction failed.
    #[error("failed to commit the refs/stash ref transaction")]
    CommitTransaction(#[from] gix_ref::file::transaction::commit::Error),

    /// Constructing the HEAD index for the worktree reset failed.
    #[error("failed to construct HEAD index for worktree reset")]
    IndexFromTree(#[from] gix_index::init::from_tree::Error),

    /// Resetting the working tree to HEAD after stashing failed.
    #[error("failed to reset working tree to HEAD after stashing")]
    Checkout(#[from] gix_worktree_state::checkout::Error),
}

/// Repository-level plumbing handles required by [`function::push`].
///
/// Grouping these together avoids crossing the "too many arguments" threshold
/// that clippy enforces.
///
/// `Objects` must implement [`gix_object::Find`], [`gix_object::FindHeader`],
/// and [`gix_object::Write`] — all of which are satisfied by the typical
/// `gix_odb::Handle` / `gix::Repository` object store.
pub struct Context<'a, Objects> {
    /// The file-based ref store for the repository.
    pub refs: &'a gix_ref::file::Store,
    /// A combined readable + writable ODB handle.
    pub objects: &'a Objects,
    /// The current in-memory index state.
    pub index: &'a gix_index::State,
    /// Absolute path to the working-tree root.
    pub worktree: &'a std::path::Path,
    /// Identity and timestamp to use for all created commits.
    pub committer: gix_actor::SignatureRef<'a>,
    /// Options controlling the worktree-reset checkout that runs after the
    /// stash commit is recorded.  The caller is responsible for populating
    /// `attributes` (`.gitattributes` from the index) and `filters`
    /// (a fully-configured `gix_filter::Pipeline`).  The remaining fields
    /// can be left at their defaults for a typical stash.
    pub checkout_options: gix_worktree_state::checkout::Options,
}

pub(crate) mod function {
    use std::path::Path;

    use bstr::{BString, ByteSlice};
    use gix_hash::ObjectId;
    use gix_object::{Tree, tree::EntryKind};
    use gix_ref::{
        FullName,
        transaction::{Change, LogChange, PreviousValue, RefEdit},
    };

    use super::{Context, Error, Options, Outcome};

    /// Capture the current working tree (+ index, + optional untracked files)
    /// as a new stash commit at `refs/stash`.
    ///
    /// All plumbing handles are passed via [`Context`].  The remaining
    /// parameters are:
    ///
    /// * `head_commit` — OID of the commit `HEAD` currently points at.
    /// * `head_tree` — OID of the root tree of `head_commit`.
    /// * `head_branch` — full name of the current branch (e.g.
    ///   `refs/heads/main`), or `None` when `HEAD` is detached.
    /// * `options` — behavioural flags.
    ///
    /// # Limitations
    ///
    /// * `.gitignore` rules are **not consulted** when `include_untracked` is
    ///   set — all non-tracked, non-`.git` files are included.
    ///
    ///   TODO(gix-stash): wire up `gix-worktree` exclude stack.
    pub fn push<Objects>(
        ctx: Context<'_, Objects>,
        head_commit: ObjectId,
        head_tree: ObjectId,
        head_branch: Option<&gix_ref::FullNameRef>,
        options: Options,
    ) -> Result<Outcome, Error>
    where
        Objects: gix_object::Find + gix_object::FindHeader + gix_object::Write + Send + Clone,
    {
        let Context {
            refs,
            objects,
            index,
            worktree,
            committer,
            checkout_options,
        } = ctx;
        // ------------------------------------------------------------------ //
        // Build all three trees before writing any commits so the
        // NoLocalChanges check can see the full picture.
        // ------------------------------------------------------------------ //
        let wip_tree_oid = write_wip_tree(index, objects, objects, head_tree, worktree)?;
        let index_tree_oid = write_tree_from_index(index, objects, objects, head_tree)?;

        // Collect untracked files (trees only, no commits yet) so we can
        // include them in the NoLocalChanges decision below.
        let (pending_untracked_tree, pending_untracked_paths) = if options.include_untracked {
            let (tree, paths) = write_untracked_tree(objects, objects, worktree, index)?;
            (Some(tree), paths)
        } else {
            (None, Vec::new())
        };

        // Guard: nothing to stash when all three trees are empty / identical
        // to HEAD.  Checking the untracked tree against the empty-tree OID
        // guards against the case where `include_untracked=true` but the
        // worktree has no untracked files.
        let has_wt_changes = wip_tree_oid != head_tree;
        let has_index_changes = index_tree_oid != head_tree;
        let empty_tree = ObjectId::empty_tree(head_commit.kind());
        let has_untracked = pending_untracked_tree.as_ref().is_some_and(|t| *t != empty_tree);
        if !has_wt_changes && !has_index_changes && !has_untracked {
            return Err(Error::NoLocalChanges);
        }

        // ------------------------------------------------------------------ //
        // Build common text fragments used in commit messages.
        // ------------------------------------------------------------------ //
        let head_subject = first_line_of_commit_message(objects, head_commit)?;
        let short_hash = short_id(head_commit);
        let branch_name: BString = head_branch.map_or_else(|| BString::from("HEAD"), |n| n.shorten().to_owned());

        let index_msg = format!(
            "index on {branch}: {short} {subj}",
            branch = branch_name.as_bstr(),
            short = short_hash.as_bstr(),
            subj = head_subject.as_bstr(),
        );
        let index_commit_oid = write_commit(
            objects,
            index_tree_oid,
            &[head_commit],
            committer,
            index_msg.as_bytes().as_bstr(),
        )?;

        // ------------------------------------------------------------------ //
        // parent[2] — untracked files commit (optional).
        // ------------------------------------------------------------------ //
        let (untracked_commit_oid, untracked_paths) = if let Some(untracked_tree) = pending_untracked_tree {
            if untracked_tree != empty_tree {
                let msg = format!(
                    "untracked files on {branch}: {short} {subj}",
                    branch = branch_name.as_bstr(),
                    short = short_hash.as_bstr(),
                    subj = head_subject.as_bstr(),
                );
                (
                    Some(write_commit(
                        objects,
                        untracked_tree,
                        &[],
                        committer,
                        msg.as_bytes().as_bstr(),
                    )?),
                    pending_untracked_paths,
                )
            } else {
                (None, Vec::new())
            }
        } else {
            (None, Vec::new())
        };

        // ------------------------------------------------------------------ //
        // Stash commit — WIP tree captures the *actual* working-tree state.
        // ------------------------------------------------------------------ //
        let stash_msg: BString = options.message.clone().unwrap_or_else(|| {
            format!(
                "WIP on {branch}: {short} {subj}",
                branch = branch_name.as_bstr(),
                short = short_hash.as_bstr(),
                subj = head_subject.as_bstr(),
            )
            .into()
        });

        let mut stash_parents: Vec<ObjectId> = vec![head_commit, index_commit_oid];
        if let Some(u) = untracked_commit_oid {
            stash_parents.push(u);
        }
        let stash_oid = write_commit(objects, wip_tree_oid, &stash_parents, committer, stash_msg.as_bstr())?;

        // ------------------------------------------------------------------ //
        // Update refs/stash via transaction.
        // ------------------------------------------------------------------ //
        let stash_ref_name: FullName = "refs/stash".try_into().expect("refs/stash is a valid ref name");

        let previous = refs
            .try_find(stash_ref_name.as_ref())
            .ok()
            .flatten()
            .and_then(|r| r.target.try_id().map(ToOwned::to_owned));

        let expected = match &previous {
            Some(prev_oid) => PreviousValue::ExistingMustMatch(gix_ref::Target::Object(*prev_oid)),
            None => PreviousValue::Any,
        };

        let edit = RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: gix_ref::transaction::RefLog::AndReference,
                    force_create_reflog: true,
                    message: stash_msg.clone(),
                },
                expected,
                new: gix_ref::Target::Object(stash_oid),
            },
            name: stash_ref_name,
            deref: false,
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

        // ------------------------------------------------------------------ //
        // Reset working tree — to HEAD, or to the index when keep_index=true.
        // ------------------------------------------------------------------ //
        // With keep_index=true the WT is reset to the *index* state (staged
        // changes are preserved on disk) rather than to HEAD.  We already
        // computed `index_tree_oid` above, so we just reuse it.
        let reset_tree = if options.keep_index { index_tree_oid } else { head_tree };
        let mut reset_index =
            gix_index::State::from_tree(&reset_tree, objects, gix_validate::path::component::Options::default())?;
        let should_interrupt = std::sync::atomic::AtomicBool::new(false);
        gix_worktree_state::checkout(
            &mut reset_index,
            worktree,
            objects.clone(),
            &gix_features::progress::Discard,
            &gix_features::progress::Discard,
            &should_interrupt,
            checkout_options,
        )?;

        // ------------------------------------------------------------------ //
        // Remove untracked files that were captured in parent[2].
        // ------------------------------------------------------------------ //
        for abs_path in &untracked_paths {
            // Best-effort: ignore errors (file may have already been removed).
            let _ = std::fs::remove_file(abs_path);
        }

        Ok(Outcome {
            stash: stash_oid,
            index_commit: index_commit_oid,
            untracked_commit: untracked_commit_oid,
            previous,
        })
    }

    // ======================================================================= //
    // Private helpers
    // ======================================================================= //

    /// Build a WIP tree that captures the **actual working-tree state** for
    /// every tracked entry, not just the index content.
    ///
    /// For each index entry:
    /// * Regular files and executables: read the file from disk, hash it as a
    ///   blob, and use the resulting OID in the tree.  This captures unstaged
    ///   modifications.  If the WT file is missing (a `git rm`-style change),
    ///   the index OID is reused — the file is still represented in the stash.
    /// * Symlinks: read the link target from disk and store it as a blob.
    /// * Submodules (`Commit` mode): reuse the index OID without recursing.
    ///
    /// The resulting tree therefore reflects the state an observer would see
    /// by reading every file from the worktree.
    fn write_wip_tree(
        index: &gix_index::State,
        find: &impl gix_object::FindExt,
        odb: &impl gix_object::Write,
        head_tree: ObjectId,
        worktree: &Path,
    ) -> Result<ObjectId, Error> {
        let object_hash = index.object_hash();

        // Seed the editor with HEAD's root tree so existing sub-tree objects
        // can be reused without being re-fetched.
        let mut buf = Vec::new();
        let root_tree = find.find_tree(&head_tree, &mut buf)?.to_owned();
        let mut editor = gix_object::tree::Editor::new(root_tree, find, object_hash);

        let paths = index.path_backing();
        for entry in index.entries() {
            // Skip sparse-checkout directory markers.
            if entry.mode.is_sparse() {
                continue;
            }
            let path = entry.path_in(paths);
            let entry_kind = entry
                .mode
                .to_tree_entry_mode()
                .ok_or_else(|| Error::InvalidIndexEntryMode {
                    path: path.to_owned(),
                    mode: entry.mode.bits(),
                })?
                .kind();

            let components: Vec<&bstr::BStr> = path.split(|b| *b == b'/').map(bstr::ByteSlice::as_bstr).collect();

            let blob_oid = match entry_kind {
                EntryKind::Blob | EntryKind::BlobExecutable => {
                    // Read the actual working-tree file so that unstaged
                    // modifications are captured.
                    let abs_path = worktree.join(gix_path::from_bstr(path).as_ref());
                    match std::fs::read(&abs_path) {
                        Ok(content) => odb
                            .write_buf(gix_object::Kind::Blob, &content)
                            .map_err(Error::WriteBlob)?,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // File deleted from WT but index still tracks it.
                            // Keep the index OID so the stash still records
                            // the last known content.
                            // TODO(gix-stash): represent deleted-from-WT files
                            // as a deletion in the WIP tree so pop can replay them.
                            entry.id
                        }
                        Err(e) => {
                            return Err(Error::ReadFile {
                                path: abs_path,
                                source: e,
                            });
                        }
                    }
                }
                EntryKind::Link => {
                    // Symlinks are stored as blobs containing the link target.
                    let abs_path = worktree.join(gix_path::from_bstr(path).as_ref());
                    match std::fs::read_link(&abs_path) {
                        Ok(target) => {
                            let target_bytes = gix_path::into_bstr(target);
                            odb.write_buf(gix_object::Kind::Blob, target_bytes.as_ref())
                                .map_err(Error::WriteBlob)?
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => entry.id,
                        Err(e) => {
                            return Err(Error::ReadFile {
                                path: abs_path,
                                source: e,
                            });
                        }
                    }
                }
                EntryKind::Commit => {
                    // Submodule — record the checked-out commit OID as-is.
                    entry.id
                }
                EntryKind::Tree => {
                    // Should not appear in index entries, but be defensive.
                    entry.id
                }
            };

            editor.upsert(components, entry_kind, blob_oid)?;
        }

        editor.write(|tree| odb.write(tree).map_err(Error::WriteTree))
    }

    /// Build a tree mirroring the current index state and write it to the ODB.
    fn write_tree_from_index(
        index: &gix_index::State,
        find: &impl gix_object::FindExt,
        odb: &impl gix_object::Write,
        head_tree: ObjectId,
    ) -> Result<ObjectId, Error> {
        let object_hash = index.object_hash();

        // Seed the editor with HEAD's root tree so existing sub-tree objects
        // can be reused without being re-fetched.
        let mut buf = Vec::new();
        let root_tree = find.find_tree(&head_tree, &mut buf)?.to_owned();
        let mut editor = gix_object::tree::Editor::new(root_tree, find, object_hash);

        let paths = index.path_backing();
        for entry in index.entries() {
            // Skip sparse-checkout directory markers.
            if entry.mode.is_sparse() {
                continue;
            }
            let path = entry.path_in(paths);
            let entry_kind = entry
                .mode
                .to_tree_entry_mode()
                .ok_or_else(|| Error::InvalidIndexEntryMode {
                    path: path.to_owned(),
                    mode: entry.mode.bits(),
                })?
                .kind();

            // Split the path on `/` to feed into the tree editor.
            let components: Vec<&bstr::BStr> = path.split(|b| *b == b'/').map(bstr::ByteSlice::as_bstr).collect();
            editor.upsert(components, entry_kind, entry.id)?;
        }

        editor.write(|tree| odb.write(tree).map_err(Error::WriteTree))
    }

    /// Walk the worktree recursively for files not in `index`, write them as
    /// blobs, and assemble them into a tree.
    ///
    /// Returns the tree OID **and** the list of absolute paths that were
    /// captured.  The paths list is used by the caller to remove those files
    /// from disk after the stash ref is committed.
    ///
    /// Uses `std::fs::read_dir` rather than `gix-dir` to avoid pulling in
    /// `gix-pathspec` as a direct dependency.  `.gitignore` rules are **not**
    /// respected.
    ///
    /// TODO(gix-stash): consult `.gitignore` via the `gix-worktree` exclude stack.
    fn write_untracked_tree(
        find: &impl gix_object::FindExt,
        odb: &impl gix_object::Write,
        worktree: &Path,
        index: &gix_index::State,
    ) -> Result<(ObjectId, Vec<std::path::PathBuf>), Error> {
        let object_hash = index.object_hash();
        let mut editor = gix_object::tree::Editor::new(Tree::empty(), find, object_hash);
        let mut abs_paths: Vec<std::path::PathBuf> = Vec::new();

        let paths_storage = index.path_backing();
        let tracked: std::collections::BTreeSet<BString> = index
            .entries()
            .iter()
            .map(|e| e.path_in(paths_storage).to_owned())
            .collect();

        collect_untracked(worktree, worktree, &tracked, odb, &mut editor, &mut abs_paths)?;
        let tree_oid = editor.write(|tree| odb.write(tree).map_err(Error::WriteTree))?;
        Ok((tree_oid, abs_paths))
    }

    /// Recursively walk `dir` and add untracked files to `editor`.
    fn collect_untracked(
        worktree: &Path,
        dir: &Path,
        tracked: &std::collections::BTreeSet<BString>,
        odb: &impl gix_object::Write,
        editor: &mut gix_object::tree::Editor<'_>,
        abs_paths: &mut Vec<std::path::PathBuf>,
    ) -> Result<(), Error> {
        let read_dir = std::fs::read_dir(dir).map_err(Error::WalkWorktree)?;

        for dir_entry_result in read_dir {
            let dir_entry = dir_entry_result.map_err(Error::WalkWorktree)?;
            let name = dir_entry.file_name();
            let name_bytes = name.as_encoded_bytes();

            // Never recurse into .git.
            if name_bytes == b".git" {
                continue;
            }

            let abs_path = dir_entry.path();
            let file_type = dir_entry.file_type().map_err(|e| Error::ReadFile {
                path: abs_path.clone(),
                source: e,
            })?;

            if file_type.is_dir() {
                collect_untracked(worktree, &abs_path, tracked, odb, editor, abs_paths)?;
            } else if file_type.is_file() || file_type.is_symlink() {
                let rela = rela_path(worktree, &abs_path);
                if tracked.contains(&rela) {
                    continue;
                }

                let (blob_content, kind) = if file_type.is_symlink() {
                    // Store the symlink target path as the blob, not the
                    // content of the file the link points to.
                    let target = std::fs::read_link(&abs_path).map_err(|e| Error::ReadFile {
                        path: abs_path.clone(),
                        source: e,
                    })?;
                    let target_bytes = gix_path::into_bstr(target);
                    (target_bytes.as_ref().to_vec(), EntryKind::Link)
                } else {
                    let content = std::fs::read(&abs_path).map_err(|e| Error::ReadFile {
                        path: abs_path.clone(),
                        source: e,
                    })?;
                    (content, EntryKind::Blob)
                };

                let blob_oid = odb
                    .write_buf(gix_object::Kind::Blob, &blob_content)
                    .map_err(Error::WriteBlob)?;

                let rela_bstr: &bstr::BStr = rela.as_bstr();
                let components: Vec<&bstr::BStr> =
                    rela_bstr.split(|b| *b == b'/').map(bstr::ByteSlice::as_bstr).collect();
                editor.upsert(components, kind, blob_oid)?;
                abs_paths.push(abs_path);
            }
            // Special files (sockets, devices, pipes) are silently skipped.
        }
        Ok(())
    }

    /// Compute a `/`-separated path relative to `worktree`.
    fn rela_path(worktree: &Path, abs: &Path) -> BString {
        let rel = abs
            .strip_prefix(worktree)
            .unwrap_or(abs)
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.as_encoded_bytes().to_vec()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(b"/" as &[u8]);
        BString::from(rel)
    }

    /// Write a commit object to the ODB and return its OID.
    fn write_commit(
        odb: &impl gix_object::Write,
        tree: ObjectId,
        parents: &[ObjectId],
        committer: gix_actor::SignatureRef<'_>,
        message: &bstr::BStr,
    ) -> Result<ObjectId, Error> {
        let sig: gix_actor::Signature = committer.into();
        let commit = gix_object::Commit {
            tree,
            parents: parents.iter().copied().collect(),
            author: sig.clone(),
            committer: sig,
            encoding: None,
            message: message.to_owned(),
            extra_headers: Vec::new(),
        };
        odb.write(&commit).map_err(Error::WriteCommit)
    }

    /// Return the first line (subject) of a commit's message.
    fn first_line_of_commit_message(find: &impl gix_object::FindExt, commit_oid: ObjectId) -> Result<BString, Error> {
        let mut buf = Vec::new();
        let commit = find.find_commit(&commit_oid, &mut buf)?;
        Ok(commit.message.lines().next().unwrap_or(b"").as_bstr().to_owned())
    }

    /// Return a 7-character hex prefix of the given OID.
    fn short_id(oid: ObjectId) -> BString {
        let s = oid.to_hex().to_string();
        BString::from(&s.as_bytes()[..7.min(s.len())])
    }
}
