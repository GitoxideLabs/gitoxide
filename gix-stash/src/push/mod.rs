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

    /// Reading a worktree file failed while building the untracked-files tree.
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
}

/// Repository-level plumbing handles required by [`function::push`].
///
/// Grouping these together avoids crossing the "too many arguments" threshold
/// that clippy enforces.
pub struct Context<'a> {
    /// The file-based ref store for the repository.
    pub refs: &'a gix_ref::file::Store,
    /// A writable ODB handle.
    pub odb: &'a dyn gix_object::Write,
    /// A readable ODB handle.
    pub find: &'a dyn gix_object::FindExt,
    /// The current in-memory index state.
    pub index: &'a gix_index::State,
    /// Absolute path to the working-tree root.
    pub worktree: &'a std::path::Path,
    /// Identity and timestamp to use for all created commits.
    pub committer: gix_actor::SignatureRef<'a>,
}

pub(crate) mod function {
    use std::path::Path;

    use bstr::{BStr, BString, ByteSlice};
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
    /// * The **on-disk working tree is not reset** to HEAD after stashing.
    ///   This requires `gix-worktree-state::checkout`, which in turn needs
    ///   `gix-filter`, `gix-fs`, and `gix-worktree` types not yet present in
    ///   `gix-stash`'s dependency graph.  Callers that need the full
    ///   `git stash push` experience must call `gix_worktree_state::checkout`
    ///   themselves after this function returns.
    ///
    ///   TODO(gix-stash): wire up `gix_worktree_state::checkout`.
    ///
    /// * The **WIP tree** currently captures the index state rather than the
    ///   true working-tree state for tracked files.  Unstaged modifications
    ///   are therefore not recorded in the stash commit's tree.
    ///
    ///   TODO(gix-stash): capture working-tree content for modified-but-not-staged files.
    ///
    /// * `.gitignore` rules are **not consulted** when `include_untracked` is
    ///   set — all non-tracked, non-`.git` files are included.
    ///
    ///   TODO(gix-stash): wire up `gix-worktree` exclude stack.
    pub fn push(
        ctx: Context<'_>,
        head_commit: ObjectId,
        head_tree: ObjectId,
        head_branch: Option<&gix_ref::FullNameRef>,
        options: Options,
    ) -> Result<Outcome, Error> {
        let Context {
            refs,
            odb,
            find,
            index,
            worktree,
            committer,
        } = ctx;
        // ------------------------------------------------------------------ //
        // Guard: make sure there is something to stash.
        // ------------------------------------------------------------------ //
        if index.entries().is_empty() && !options.include_untracked {
            return Err(Error::NoLocalChanges);
        }

        // ------------------------------------------------------------------ //
        // Build common text fragments used in commit messages.
        // ------------------------------------------------------------------ //
        let head_subject = first_line_of_commit_message(find, head_commit)?;
        let short_hash = short_id(head_commit);
        let branch_name: BString = head_branch.map_or_else(|| BString::from("HEAD"), |n| n.shorten().to_owned());

        // ------------------------------------------------------------------ //
        // parent[1] — "index on <branch>: …"
        // ------------------------------------------------------------------ //
        let index_tree_oid = write_tree_from_index(index, find, odb, head_tree)?;

        let index_msg = format!(
            "index on {branch}: {short} {subj}",
            branch = branch_name.as_bstr(),
            short = short_hash.as_bstr(),
            subj = head_subject.as_bstr(),
        );
        let index_commit_oid = write_commit(
            odb,
            index_tree_oid,
            &[head_commit],
            committer,
            index_msg.as_bytes().as_bstr(),
        )?;

        // ------------------------------------------------------------------ //
        // parent[2] — untracked files commit (optional).
        // ------------------------------------------------------------------ //
        let untracked_commit_oid = if options.include_untracked {
            let empty_tree = ObjectId::empty_tree(head_commit.kind());
            let untracked_tree = write_untracked_tree(find, odb, worktree, index)?;
            if untracked_tree != empty_tree {
                let msg = format!(
                    "untracked files on {branch}: {short} {subj}",
                    branch = branch_name.as_bstr(),
                    short = short_hash.as_bstr(),
                    subj = head_subject.as_bstr(),
                );
                Some(write_commit(
                    odb,
                    untracked_tree,
                    &[],
                    committer,
                    msg.as_bytes().as_bstr(),
                )?)
            } else {
                None
            }
        } else {
            None
        };

        // ------------------------------------------------------------------ //
        // Stash commit — WIP (uses index tree as proxy for the WT tree).
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
        let stash_oid = write_commit(odb, index_tree_oid, &stash_parents, committer, stash_msg.as_bstr())?;

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

    /// Build a tree mirroring the current index state and write it to the ODB.
    fn write_tree_from_index(
        index: &gix_index::State,
        find: &dyn gix_object::FindExt,
        odb: &dyn gix_object::Write,
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
            let components: Vec<&BStr> = path.split(|b| *b == b'/').map(bstr::ByteSlice::as_bstr).collect();
            editor.upsert(components, entry_kind, entry.id)?;
        }

        editor.write(|tree| odb.write(tree).map_err(Error::WriteTree))
    }

    /// Walk the worktree recursively for files not in `index`, write them as
    /// blobs, and assemble them into a tree.
    ///
    /// Uses `std::fs::read_dir` rather than `gix-dir` to avoid pulling in
    /// `gix-pathspec` / `gix-worktree` as direct dependencies.  `.gitignore`
    /// rules are **not** respected.
    ///
    /// TODO(gix-stash): consult `.gitignore` via the `gix-worktree` exclude stack.
    fn write_untracked_tree(
        find: &dyn gix_object::FindExt,
        odb: &dyn gix_object::Write,
        worktree: &Path,
        index: &gix_index::State,
    ) -> Result<ObjectId, Error> {
        let object_hash = index.object_hash();
        let mut editor = gix_object::tree::Editor::new(Tree::empty(), find, object_hash);

        let paths_storage = index.path_backing();
        let tracked: std::collections::BTreeSet<BString> = index
            .entries()
            .iter()
            .map(|e| e.path_in(paths_storage).to_owned())
            .collect();

        collect_untracked(worktree, worktree, &tracked, odb, &mut editor)?;
        editor.write(|tree| odb.write(tree).map_err(Error::WriteTree))
    }

    /// Recursively walk `dir` and add untracked files to `editor`.
    fn collect_untracked(
        worktree: &Path,
        dir: &Path,
        tracked: &std::collections::BTreeSet<BString>,
        odb: &dyn gix_object::Write,
        editor: &mut gix_object::tree::Editor<'_>,
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
                collect_untracked(worktree, &abs_path, tracked, odb, editor)?;
            } else if file_type.is_file() || file_type.is_symlink() {
                let rela = rela_path(worktree, &abs_path);
                if tracked.contains(&rela) {
                    continue;
                }

                let content = std::fs::read(&abs_path).map_err(|e| Error::ReadFile {
                    path: abs_path.clone(),
                    source: e,
                })?;
                let blob_oid = odb
                    .write_buf(gix_object::Kind::Blob, &content)
                    .map_err(Error::WriteBlob)?;

                let kind = if file_type.is_symlink() {
                    EntryKind::Link
                } else {
                    EntryKind::Blob
                };

                let rela_bstr: &BStr = rela.as_bstr();
                let components: Vec<&BStr> = rela_bstr.split(|b| *b == b'/').map(bstr::ByteSlice::as_bstr).collect();
                editor.upsert(components, kind, blob_oid)?;
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
        odb: &dyn gix_object::Write,
        tree: ObjectId,
        parents: &[ObjectId],
        committer: gix_actor::SignatureRef<'_>,
        message: &BStr,
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
    fn first_line_of_commit_message(find: &dyn gix_object::FindExt, commit_oid: ObjectId) -> Result<BString, Error> {
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
