//! Synchronous status collection that finishes all workers before returning.

use std::sync::atomic::{AtomicBool, Ordering};

use gix_error::{ErrorExt, ResultExt, message};
use gix_status::index_as_worktree::EntryStatus;

use crate::{
    bstr::BString,
    status::{Item, Platform, index_worktree},
    worktree::IndexPersistedOrInMemory,
};

/// The error returned by [`Platform::into_vec()`].
pub type Error = gix_error::Exn<gix_error::Message>;

impl<P: gix_features::progress::Progress> Platform<'_, P> {
    /// Collect all status changes synchronously, optionally restricted to `patterns`.
    ///
    /// All scoped workers, including nested submodule status computations, finish before this method
    /// returns. The result owns its changes and retains no repository handle. Ordering is unspecified.
    ///
    /// `should_interrupt` controls this computation instead of the iterator-specific interrupt options.
    /// Cancellation or failure discards partial results and returns an error.
    pub fn into_vec(
        self,
        patterns: impl IntoIterator<Item = BString>,
        should_interrupt: &AtomicBool,
    ) -> Result<Vec<Item>, Error> {
        self.collect_internal(patterns.into_iter().collect(), true, true, should_interrupt)
    }

    /// Collect owned changes, joining all scoped work before returning on success or failure.
    pub(crate) fn collect_internal(
        self,
        patterns: Vec<BString>,
        staged: bool,
        unstaged: bool,
        interrupt: &AtomicBool,
    ) -> Result<Vec<Item>, Error> {
        let interrupted = || {
            if interrupt.load(Ordering::Relaxed) {
                Err(message("status collection was interrupted").raise())
            } else {
                Ok(())
            }
        };
        interrupted()?;
        let index = match self.index {
            Some(index) => index,
            None => IndexPersistedOrInMemory::Persisted(
                self.repo
                    .index_or_empty()
                    .or_raise(|| message("could not read index for status"))?,
            ),
        };
        let mut items = Vec::new();
        if staged && let Some(tree_id) = self.head_tree {
            let tree_id = match tree_id {
                Some(tree_id) => tree_id,
                None => self
                    .repo
                    .head_tree_id_or_empty()
                    .or_raise(|| message("could not resolve HEAD tree for status"))?
                    .into(),
            };
            let mut pathspec = self
                .repo
                .index_worktree_status_pathspec::<index_worktree::Error>(
                    &patterns,
                    &index,
                    self.index_worktree_options.dirwalk_options.as_ref(),
                )
                .or_raise(|| message("could not prepare staged status pathspec"))?;
            interrupted()?;
            self.repo
                .tree_index_status(
                    &tree_id,
                    &index,
                    Some(&mut pathspec),
                    self.tree_index_renames,
                    |change, _, _| {
                        let action = if interrupt.load(Ordering::Relaxed) {
                            std::ops::ControlFlow::Break(())
                        } else {
                            items.push(Item::TreeIndex(change.into_owned()));
                            std::ops::ControlFlow::Continue(())
                        };
                        Ok::<_, std::convert::Infallible>(action)
                    },
                )
                .or_raise(|| message("could not collect staged status"))?;
            interrupted()?;
        }
        if unstaged {
            let submodules =
                index_worktree::BuiltinSubmoduleStatus::new(self.repo.clone().into_sync(), self.submodules)
                    .or_raise(|| message("could not prepare synchronous submodule status"))?
                    .synchronous(interrupt);
            interrupted()?;
            let mut recorder = gix_status::index_as_worktree_with_renames::Recorder::default();
            self.repo
                .index_worktree_status(
                    &index,
                    patterns,
                    &mut recorder,
                    gix_status::index_as_worktree::traits::FastEq,
                    submodules,
                    &mut { self.progress },
                    interrupt,
                    self.index_worktree_options,
                )
                .or_raise(|| message("could not collect worktree status"))?;
            interrupted()?;
            items.extend(
                recorder
                    .records
                    .into_iter()
                    .map(index_worktree::Item::from)
                    .filter(|item| {
                        !matches!(
                            item,
                            index_worktree::Item::Modification {
                                status: EntryStatus::NeedsUpdate(_),
                                ..
                            }
                        )
                    })
                    .map(Item::IndexWorktree),
            );
        }
        interrupted()?;
        Ok(items)
    }
}

#[cfg(test)]
mod tests;
