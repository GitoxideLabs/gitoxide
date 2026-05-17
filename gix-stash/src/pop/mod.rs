//! Implementation of [`stash pop`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-pop).

use gix_hash::ObjectId;

/// Result of a successful [`function::pop`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The id of the stash commit that was applied + dropped.
    pub applied: ObjectId,

    /// The tree recorded in the stash commit — the working-tree state at stash
    /// time.  Callers **must** apply this tree to the index and on-disk working
    /// tree themselves; `pop` cannot do so without `gix-filter` / `gix-worktree`
    /// infrastructure that is not yet wired into `gix-stash`.
    ///
    /// TODO(gix-stash): perform the worktree application here once
    /// `gix_worktree_state::checkout` dependencies are in scope.
    pub stash_tree: ObjectId,

    /// OID of parent[0] of the stash commit — the commit `HEAD` pointed at
    /// when the stash was created.  Callers can use this together with
    /// [`stash_tree`](Outcome::stash_tree) to run a 3-way merge if desired.
    ///
    /// TODO(gix-stash): call `gix_merge::tree` here once `gix_diff::blob::Platform`
    /// and `gix_merge::blob::Platform` are available without adding `gix-filter` /
    /// `gix-worktree` as direct dependencies.
    pub base_commit: ObjectId,

    /// The new value of `refs/stash` after dropping the applied entry — `None`
    /// when no older stash entries remain.
    pub new_top: Option<ObjectId>,

    /// Whether the apply step produced merge conflicts.
    ///
    /// Always `false` in the current implementation — a real 3-way merge is not
    /// yet performed (see [`stash_tree`](Outcome::stash_tree) / [`base_commit`](Outcome::base_commit)).
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
}

/// Repository-level plumbing handles required by [`function::pop`].
pub struct Context<'a> {
    /// The file-based ref store for the repository.
    pub refs: &'a gix_ref::file::Store,
    /// A readable ODB handle — used to decode the stash commit.
    pub find: &'a dyn gix_object::FindExt,
    /// Identity and timestamp to use for the ref-transaction committer line.
    pub committer: gix_actor::SignatureRef<'a>,
}

pub(crate) mod function {
    use gix_hash::ObjectId;
    use gix_ref::{
        FullName,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    };

    use super::{Context, Error, Outcome};

    /// Drop the latest stash entry from `refs/stash` and return the stash
    /// commit's OID plus the tree it carries.
    ///
    /// # Limitations
    ///
    /// * The stash is **not merged into the working tree** — the caller must
    ///   apply [`Outcome::stash_tree`] themselves (e.g. via
    ///   `gix_worktree_state::checkout`).  This mirrors the limitation in
    ///   [`push`](crate::push()) where the worktree reset is also deferred to
    ///   the caller.
    ///
    ///   TODO(gix-stash): perform 3-way merge + worktree update in-crate once
    ///   `gix_diff::blob::Platform` and `gix_merge::blob::Platform` can be
    ///   constructed without `gix-filter` / `gix-worktree` direct deps.
    ///
    /// * [`Outcome::had_conflicts`] is always `false` until the merge is wired in.
    pub fn pop(ctx: Context<'_>) -> Result<Outcome, Error> {
        let Context { refs, find, committer } = ctx;

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
        // Decode the stash commit to extract tree + base commit (parent[0]).
        // ------------------------------------------------------------------ //
        let mut commit_buf = Vec::new();
        let stash_commit = find.find_commit(&stash_oid, &mut commit_buf)?;

        let stash_tree = stash_commit.tree();

        // parent[0] is the original HEAD at stash time.
        let base_commit = stash_commit.parents().next().ok_or(Error::NoStash)?; // malformed stash; treat as absent

        // ------------------------------------------------------------------ //
        // Look up the second-newest entry so we know what to set refs/stash to
        // after dropping the top.  We read the reflog in reverse (newest first);
        // index 0 = current tip (what we are popping), index 1 = the one before.
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
        // Drop the stash: update refs/stash to `new_top`, or delete it entirely.
        // ------------------------------------------------------------------ //
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

        Ok(Outcome {
            applied: stash_oid,
            stash_tree,
            base_commit,
            new_top,
            had_conflicts: false,
        })
    }
}
