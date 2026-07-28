use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use gix_hash::ObjectId;
use gix_ref::transaction::PreviousValue;

use crate::{Progress, Repository, bstr::BString};

/// Options for [`Repository::reset_hard()`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// If `true`, fail unless the current `HEAD` commit is an ancestor of `target`
    /// (including equality). This is the `git reset --hard` equivalent of a
    /// fast-forward-only policy often used after a fetch.
    ///
    /// Requires the `revision` feature so merge-base can be computed; if that
    /// feature is off, setting this flag returns [`Error::FastForwardRequiresRevision`].
    pub require_fast_forward: bool,
    /// Message written into the reflog when updating the current branch or detached `HEAD`.
    /// Defaults to `"reset: moving to <id>"` when `None`.
    pub reflog_message: Option<BString>,
}

/// Outcome of a successful hard reset.
#[derive(Debug)]
pub struct Outcome {
    /// The commit `HEAD` now points at (after peeling tags).
    pub commit_id: ObjectId,
    /// The tree that was checked out into the index and worktree.
    pub tree_id: ObjectId,
    /// Details from the low-level worktree checkout.
    pub checkout: gix_worktree_state::checkout::Outcome,
}

///
pub mod error {
    use std::path::PathBuf;

    /// The error returned by [`Repository::reset_hard()`](crate::Repository::reset_hard).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("Repository at \"{}\" is bare and cannot be hard-reset", git_dir.display())]
        BareRepository { git_dir: PathBuf },
        #[error("HEAD is unborn; there is no commit to reset from")]
        UnbornHead,
        #[error("require_fast_forward needs the `revision` cargo feature (merge-base)")]
        FastForwardRequiresRevision,
        #[error("not a fast-forward: HEAD is not an ancestor of the reset target")]
        NotFastForward {
            head: gix_hash::ObjectId,
            target: gix_hash::ObjectId,
        },
        #[error(transparent)]
        HeadId(#[from] crate::reference::head_id::Error),
        #[error(transparent)]
        FindObject(#[from] crate::object::find::existing::Error),
        #[error(transparent)]
        PeelToKind(#[from] crate::object::peel::to_kind::Error),
        #[error(transparent)]
        DecodeObject(#[from] gix_object::decode::Error),
        #[error(transparent)]
        FindHead(#[from] crate::reference::find::existing::Error),
        #[error(transparent)]
        ReferenceEdit(#[from] crate::reference::edit::Error),
        #[error(transparent)]
        IndexFromTree(#[from] crate::repository::index_from_tree::Error),
        #[error(transparent)]
        CheckoutOptions(#[from] crate::config::checkout_options::Error),
        #[error(transparent)]
        IndexCheckout(#[from] gix_worktree_state::checkout::Error),
        #[error(transparent)]
        WriteIndex(#[from] gix_index::file::write::Error),
        #[error("Failed to reopen object database as Arc (only if thread-safety wasn't compiled in)")]
        OpenArcOdb(#[from] std::io::Error),
        #[cfg(feature = "revision")]
        #[error(transparent)]
        MergeBase(#[from] crate::repository::merge_base::Error),
    }
}
pub use error::Error;

/// Progress ids used in [`Repository::reset_hard()`].
///
/// Use this information to selectively extract the progress of interest when the parent
/// application has custom visualization.
#[derive(Debug, Copy, Clone)]
pub enum ProgressId {
    /// The amount of files checked out thus far.
    CheckoutFiles,
    /// The amount of bytes written in total.
    BytesWritten,
}

impl From<ProgressId> for gix_features::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::CheckoutFiles => *b"RSCF",
            ProgressId::BytesWritten => *b"RSCB",
        }
    }
}

/// Hard-reset of `HEAD`, index and worktree.
impl Repository {
    /// Hard-reset `HEAD`, the index and the worktree to the tree of `target`.
    ///
    /// This is the equivalent of `git reset --hard <target>`:
    ///
    /// 1. Peel `target` to a commit and its tree.
    /// 2. Optionally enforce that the current `HEAD` is an ancestor of `target`
    ///    ([`Options::require_fast_forward`]).
    /// 3. Move the current branch (or detached `HEAD`) to that commit.
    /// 4. Build a fresh index from the target tree.
    /// 5. Delete worktree paths that were tracked before but are absent from the new tree
    ///    (untracked files are left alone, matching Git).
    /// 6. Force-checkout the new index into the worktree (`overwrite_existing`).
    /// 7. Write the new index to disk.
    ///
    /// Hooks (`reset`, `post-checkout`) are **not** run yet.
    ///
    /// ### Notes
    ///
    /// * Soft and mixed modes are not implemented; open a follow-up if those are needed.
    /// * Sparse-checkout and submodules are not handled specially.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BareRepository`] if there is no worktree, and
    /// [`Error::NotFastForward`] when fast-forward is required but the history diverged.
    pub fn reset_hard<P>(
        &self,
        target: impl Into<ObjectId>,
        mut progress: P,
        should_interrupt: &AtomicBool,
        options: Options,
    ) -> Result<Outcome, Error>
    where
        P: gix_features::progress::NestedProgress,
        P::SubProgress: gix_features::progress::NestedProgress + 'static,
    {
        self.reset_hard_inner(target.into(), &mut progress, should_interrupt, options)
    }

    fn reset_hard_inner(
        &self,
        target: ObjectId,
        progress: &mut dyn gix_features::progress::DynNestedProgress,
        should_interrupt: &AtomicBool,
        options: Options,
    ) -> Result<Outcome, Error> {
        let _span = gix_trace::coarse!("gix::Repository::reset_hard()");
        let workdir = self.workdir().ok_or_else(|| Error::BareRepository {
            git_dir: self.git_dir().to_owned(),
        })?;

        let commit = self.find_object(target)?.peel_to_commit()?;
        let commit_id = commit.id;
        let tree_id = commit.tree_id()?.detach();

        if options.require_fast_forward {
            self.ensure_fast_forward(commit_id)?;
        }

        // Snapshot old index paths so we can delete tracked files that vanish.
        let old_paths: Vec<PathBuf> = match self.open_index() {
            Ok(idx) => idx
                .entries()
                .iter()
                .filter(|e| e.stage_raw() == 0)
                .filter_map(|e| {
                    let rela = e.path_in(idx.path_backing());
                    self.workdir_path(rela)
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        // 1) Move the current branch (or detached HEAD) to the target commit.
        let reflog = options
            .reflog_message
            .unwrap_or_else(|| format!("reset: moving to {commit_id}").into());
        update_head_to(self, commit_id, reflog)?;

        // 2) New index from the target tree.
        let mut index = self.index_from_tree(&tree_id)?;

        // 3) Remove worktree files that were tracked before and are gone now.
        let new_paths: HashSet<PathBuf> = index
            .entries()
            .iter()
            .filter(|e| e.stage_raw() == 0)
            .filter_map(|e| {
                let rela = e.path_in(index.path_backing());
                self.workdir_path(rela)
            })
            .collect();
        for path in old_paths {
            if !new_paths.contains(&path) {
                remove_path_and_empty_parents(&path, workdir);
            }
        }

        // 4) Force-checkout the new index into the worktree.
        let mut opts = self.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        opts.overwrite_existing = true;
        opts.destination_is_initially_empty = false;

        let mut files = progress.add_child_with_id("checkout".into(), ProgressId::CheckoutFiles.into());
        let mut bytes = progress.add_child_with_id("writing".into(), ProgressId::BytesWritten.into());
        files.init(Some(index.entries().len()), crate::progress::count("files"));
        bytes.init(None, crate::progress::bytes());

        let start = std::time::Instant::now();
        let checkout = gix_worktree_state::checkout(
            &mut index,
            workdir,
            self.objects.clone().into_arc()?,
            &files,
            &bytes,
            should_interrupt,
            opts,
        )?;
        files.show_throughput(start);
        bytes.show_throughput(start);

        // 5) Persist the index.
        index.write(Default::default())?;

        Ok(Outcome {
            commit_id,
            tree_id,
            checkout,
        })
    }

    fn ensure_fast_forward(&self, target: ObjectId) -> Result<(), Error> {
        #[cfg(feature = "revision")]
        {
            let head = self.head_id()?.detach();
            if head == target {
                return Ok(());
            }
            match self.merge_base(head, target) {
                Ok(base) if base.detach() == head => Ok(()),
                Ok(_) | Err(crate::repository::merge_base::Error::NotFound { .. }) => {
                    Err(Error::NotFastForward { head, target })
                }
                Err(err) => Err(err.into()),
            }
        }
        #[cfg(not(feature = "revision"))]
        {
            let _ = target;
            Err(Error::FastForwardRequiresRevision)
        }
    }
}

fn update_head_to(repo: &Repository, target: ObjectId, reflog_message: BString) -> Result<(), Error> {
    let head = repo.head()?;
    if head.is_unborn() {
        return Err(Error::UnbornHead);
    }

    if let Some(branch) = head.try_into_referent() {
        // Force-update the current branch (hard-reset always moves the tip).
        repo.reference(branch.name().to_owned(), target, PreviousValue::Any, reflog_message)?;
    } else {
        // Detached HEAD: update HEAD as a direct ref.
        repo.reference("HEAD", target, PreviousValue::Any, reflog_message)?;
    }
    Ok(())
}

fn remove_path_and_empty_parents(path: &Path, workdir: &Path) {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };
    let _ = if meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };

    let mut parent = path.parent();
    while let Some(dir) = parent {
        if dir == workdir {
            break;
        }
        match std::fs::remove_dir(dir) {
            Ok(()) => parent = dir.parent(),
            Err(_) => break,
        }
    }
}
