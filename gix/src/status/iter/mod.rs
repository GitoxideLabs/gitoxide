use crate::bstr::BString;
use crate::config::cache::util::ApplyLeniencyDefault;
use crate::status::index_worktree::BuiltinSubmoduleStatus;
use crate::status::{index_worktree, tree_index, Platform};
use crate::worktree::IndexPersistedOrInMemory;
use gix_status::index_as_worktree::{Change, EntryStatus};
use std::sync::atomic::Ordering;

pub(super) mod types;
use types::{ApplyChange, Item, Iter, Outcome};

/// Lifecycle
impl<Progress> Platform<'_, Progress>
where
    Progress: gix_features::progress::Progress,
{
    /// Turn the platform into an iterator for changes between the head-tree and the index, and the index and the working tree,
    /// while optionally listing untracked and/or ignored files.
    ///
    /// * `patterns`
    ///     - Optional patterns to use to limit the paths to look at. If empty, all paths are considered.
    ///
    /// ### Important: Undefined ordering
    ///
    /// When compiled with the `parallel` feature, three operations are run at once:
    ///
    /// * a dirwalk to find untracked and possibly ignored files
    /// * an entry-by-entry check to see which of the tracked files changed, and how
    /// * a tree-index comparison
    ///
    /// All of these generate distinct events which may now happen in any order, so consumers
    /// that are ordering dependent have to restore their desired order.
    ///
    /// This isn't feasible to do here as it would mean that returned items would have to be delayed,
    /// degrading performance for everyone who isn't order-dependent.
    #[doc(alias = "diff_index_to_workdir", alias = "git2")]
    pub fn into_iter(
        self,
        patterns: impl IntoIterator<Item = BString>,
    ) -> Result<Iter, crate::status::into_iter::Error> {
        let index = match self.index {
            None => IndexPersistedOrInMemory::Persisted(self.repo.index_or_empty()?),
            Some(index) => index,
        };

        let obtain_tree_id = || -> Result<Option<gix_hash::ObjectId>, crate::status::into_iter::Error> {
            Ok(match self.head_tree {
                Some(None) => match self.repo.head_tree_id() {
                    Ok(id) => Some(id.into()),
                    Err(crate::reference::head_tree_id::Error::HeadCommit(
                        crate::reference::head_commit::Error::PeelToCommit(
                            crate::head::peel::to_commit::Error::PeelToObject(
                                crate::head::peel::to_object::Error::Unborn { .. },
                            ),
                        ),
                    )) => Some(gix_hash::ObjectId::empty_tree(self.repo.object_hash())),
                    Err(err) => return Err(err.into()),
                },
                Some(Some(tree_id)) => Some(tree_id),
                None => None,
            })
        };

        let skip_hash = self
            .repo
            .config
            .resolved
            .boolean(crate::config::tree::Index::SKIP_HASH)
            .map(|res| crate::config::tree::Index::SKIP_HASH.enrich_error(res))
            .transpose()
            .with_lenient_default(self.repo.config.lenient_config)?
            .unwrap_or_default();
        let should_interrupt = self.should_interrupt.clone().unwrap_or_default();
        let submodule = BuiltinSubmoduleStatus::new(self.repo.clone().into_sync(), self.submodules)?;
        #[cfg(feature = "parallel")]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let patterns: Vec<_> = patterns.into_iter().collect();
            let join_tree_index = if let Some(tree_id) = obtain_tree_id()? {
                std::thread::Builder::new()
                    .name("gix::status::tree_index::producer".into())
                    .spawn({
                        let repo = self.repo.clone().into_sync();
                        let should_interrupt = should_interrupt.clone();
                        let tx = tx.clone();
                        let tree_index_renames = self.tree_index_renames;
                        let index = index.clone();
                        let crate::Pathspec { repo: _, stack, search } = self
                            .repo
                            .index_worktree_status_pathspec::<crate::status::into_iter::Error>(
                                &patterns,
                                &index,
                                self.index_worktree_options.dirwalk_options.as_ref(),
                            )?;
                        move || -> Result<_, _> {
                            let repo = repo.to_thread_local();
                            let mut pathspec = crate::Pathspec {
                                repo: &repo,
                                stack,
                                search,
                            };
                            repo.tree_index_status(
                                &tree_id,
                                &index,
                                Some(&mut pathspec),
                                tree_index_renames,
                                |change, _, _| {
                                    let action = if tx.send(change.into_owned().into()).is_err()
                                        || should_interrupt.load(Ordering::Acquire)
                                    {
                                        gix_diff::index::Action::Cancel
                                    } else {
                                        gix_diff::index::Action::Continue
                                    };
                                    Ok::<_, std::convert::Infallible>(action)
                                },
                            )
                        }
                    })
                    .map_err(crate::status::into_iter::Error::SpawnThread)?
                    .into()
            } else {
                None
            };
            let mut collect = Collect { tx };
            let join_index_worktree = std::thread::Builder::new()
                .name("gix::status::index_worktree::producer".into())
                .spawn({
                    let repo = self.repo.clone().into_sync();
                    let options = self.index_worktree_options;
                    let should_interrupt = should_interrupt.clone();
                    let mut progress = self.progress;
                    move || -> Result<_, index_worktree::Error> {
                        let repo = repo.to_thread_local();
                        let out = repo.index_worktree_status(
                            &index,
                            patterns,
                            &mut collect,
                            gix_status::index_as_worktree::traits::FastEq,
                            submodule,
                            &mut progress,
                            &should_interrupt,
                            options,
                        )?;
                        Ok(Outcome {
                            index_worktree: out,
                            tree_index: None,
                            worktree_index: index,
                            changes: None,
                            skip_hash,
                        })
                    }
                })
                .map_err(crate::status::into_iter::Error::SpawnThread)?;

            Ok(Iter {
                rx_and_join: Some((rx, join_index_worktree, join_tree_index)),
                should_interrupt,
                index_changes: Vec::new(),
                out: None,
            })
        }
        #[cfg(not(feature = "parallel"))]
        {
            let mut collect = Collect { items: Vec::new() };

            let repo = self.repo;
            let options = self.index_worktree_options;
            let mut progress = self.progress;
            let patterns: Vec<BString> = patterns.into_iter().collect();
            let (mut items, tree_index) = match obtain_tree_id()? {
                Some(tree_id) => {
                    let mut pathspec = repo.index_worktree_status_pathspec::<crate::status::into_iter::Error>(
                        &patterns,
                        &index,
                        self.index_worktree_options.dirwalk_options.as_ref(),
                    )?;
                    let mut items = Vec::new();
                    let tree_index = self.repo.tree_index_status(
                        &tree_id,
                        &index,
                        Some(&mut pathspec),
                        self.tree_index_renames,
                        |change, _, _| {
                            items.push(change.into_owned().into());
                            let action = if should_interrupt.load(Ordering::Acquire) {
                                gix_diff::index::Action::Cancel
                            } else {
                                gix_diff::index::Action::Continue
                            };
                            Ok::<_, std::convert::Infallible>(action)
                        },
                    )?;
                    (items, Some(tree_index))
                }
                None => (Vec::new(), None),
            };
            let out = repo.index_worktree_status(
                &index,
                patterns,
                &mut collect,
                gix_status::index_as_worktree::traits::FastEq,
                submodule,
                &mut progress,
                &should_interrupt,
                options,
            )?;
            let mut iter = Iter {
                items: Vec::new().into_iter(),
                index_changes: Vec::new(),
                out: None,
            };
            let mut out = Outcome {
                index_worktree: out,
                worktree_index: index,
                tree_index,
                changes: None,
                skip_hash,
            };
            items.extend(
                collect
                    .items
                    .into_iter()
                    .filter_map(|item| iter.maybe_keep_index_change(item)),
            );
            out.changes = (!iter.index_changes.is_empty()).then(|| std::mem::take(&mut iter.index_changes));
            iter.items = items.into_iter();
            iter.out = Some(out);
            Ok(iter)
        }
    }
}

/// The error returned for each item returned by [`Iter`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    IndexWorktree(#[from] index_worktree::Error),
    #[error(transparent)]
    TreeIndex(#[from] tree_index::Error),
}

impl Iterator for Iter {
    type Item = Result<Item, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "parallel")]
        loop {
            let (rx, _join_worktree, _join_tree) = self.rx_and_join.as_ref()?;
            match rx.recv_timeout(std::time::Duration::from_millis(25)) {
                Ok(item) => {
                    if let Some(item) = self.maybe_keep_index_change(item) {
                        break Some(Ok(item));
                    }
                    continue;
                }
                // NOTE: this isn't necessary when index::from-tree also supports interrupts. As it stands,
                //       on big repositories it can go up to 500ms which aren't interruptible, so this is another
                //       way to not wait for this. Once it can be interrupted, this won't be needed anymore.
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if self.should_interrupt.load(Ordering::SeqCst) {
                        return None;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    let (_rx, worktree_handle, tree_handle) = self.rx_and_join.take()?;
                    let tree_index = if let Some(handle) = tree_handle {
                        match handle.join().expect("no panic") {
                            Ok(out) => Some(out),
                            Err(err) => break Some(Err(err.into())),
                        }
                    } else {
                        None
                    };
                    break match worktree_handle.join().expect("no panic") {
                        Ok(mut out) => {
                            out.changes = Some(std::mem::take(&mut self.index_changes));
                            out.tree_index = tree_index;
                            self.out = Some(out);
                            None
                        }
                        Err(err) => Some(Err(err.into())),
                    };
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.items.next().map(Ok)
    }
}

/// Access
impl Iter {
    /// Return the outcome of the iteration, or `None` if the iterator isn't fully consumed.
    pub fn outcome_mut(&mut self) -> Option<&mut Outcome> {
        self.out.as_mut()
    }

    /// Turn the iterator into the iteration outcome, which is `None` on error or if the iteration
    /// isn't complete.
    pub fn into_outcome(mut self) -> Option<Outcome> {
        self.out.take()
    }
}

impl Iter {
    fn maybe_keep_index_change(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::IndexWorktree(index_worktree::Item::Modification {
                status: EntryStatus::NeedsUpdate(stat),
                entry_index,
                ..
            }) => {
                self.index_changes.push((entry_index, ApplyChange::NewStat(stat)));
                return None;
            }
            Item::IndexWorktree(index_worktree::Item::Modification {
                status:
                    EntryStatus::Change(Change::Modification {
                        set_entry_stat_size_zero,
                        ..
                    }),
                entry_index,
                ..
            }) if set_entry_stat_size_zero => {
                self.index_changes.push((entry_index, ApplyChange::SetSizeToZero));
            }
            _ => {}
        }
        Some(item)
    }
}

#[cfg(feature = "parallel")]
impl Drop for Iter {
    fn drop(&mut self) {
        crate::util::parallel_iter_drop(self.rx_and_join.take(), &self.should_interrupt);
    }
}

struct Collect {
    #[cfg(feature = "parallel")]
    tx: std::sync::mpsc::Sender<Item>,
    #[cfg(not(feature = "parallel"))]
    items: Vec<Item>,
}

impl<'index> gix_status::index_as_worktree_with_renames::VisitEntry<'index> for Collect {
    type ContentChange =
        <gix_status::index_as_worktree::traits::FastEq as gix_status::index_as_worktree::traits::CompareBlobs>::Output;
    type SubmoduleStatus = <BuiltinSubmoduleStatus as gix_status::index_as_worktree::traits::SubmoduleStatus>::Output;

    fn visit_entry(
        &mut self,
        entry: gix_status::index_as_worktree_with_renames::Entry<'index, Self::ContentChange, Self::SubmoduleStatus>,
    ) {
        // NOTE: we assume that the receiver triggers interruption so the operation will stop if the receiver is down.
        let item = Item::IndexWorktree(entry.into());
        #[cfg(feature = "parallel")]
        self.tx.send(item).ok();
        #[cfg(not(feature = "parallel"))]
        self.items.push(item);
    }
}
