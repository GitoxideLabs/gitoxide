use crate::{
    bstr::BStr,
    status::{index_worktree, tree_index},
    worktree::IndexPersistedOrInMemory,
};

/// An iterator for changes between the index and the worktree and the head-tree and the index.
///
/// Note that depending on the underlying configuration, there might be a significant delay until the first
/// item is received due to the buffering necessary to perform rename tracking and/or sorting.
///
/// ### Submodules
///
/// Note that submodules can be set to 'inactive', which will not exclude them from the status operation, similar to
/// how `git status` includes them.
///
/// ### Index Changes
///
/// Changes to the index are collected, and it's possible to write the index back using [Outcome::write_changes()].
/// Note that these changes are not observable, they will always be kept.
///
/// ### Parallel Operation
///
/// Note that without the `parallel` feature, the iterator becomes 'serial', which means all status will be computed in advance,
/// and it's non-interruptible, yielding worse performance for is-dirty checks for instance as interruptions won't happen.
/// It's a crutch that is just there to make single-threaded applications possible at all, as it's not really an iterator
/// anymore. If this matters, better run [Repository::index_worktree_status()](crate::Repository::index_worktree_status) by hand
/// as it provides all control one would need, just not as an iterator.
///
/// Also, even with `parallel` set, the first call to `next()` will block until there is an item available, without a chance
/// to interrupt unless [`status::Platform::should_interrupt_*()`](crate::status::Platform::should_interrupt_shared()) was
/// configured.
pub struct Iter {
    #[cfg(feature = "parallel")]
    #[allow(clippy::type_complexity)]
    pub(super) rx_and_join: Option<(
        std::sync::mpsc::Receiver<Item>,
        std::thread::JoinHandle<Result<Outcome, index_worktree::Error>>,
        Option<std::thread::JoinHandle<Result<tree_index::Outcome, tree_index::Error>>>,
    )>,
    #[cfg(feature = "parallel")]
    pub(super) should_interrupt: crate::status::OwnedOrStaticAtomicBool,
    /// Without parallelization, the iterator has to buffer all changes in advance.
    #[cfg(not(feature = "parallel"))]
    pub(super) items: std::vec::IntoIter<Item>,
    /// The outcome of the operation, only available once the operation has ended.
    pub(in crate::status) out: Option<Outcome>,
    /// The set of `(entry_index, change)` we extracted in order to potentially write back the worktree index with the changes applied.
    pub(super) index_changes: Vec<(usize, ApplyChange)>,
}

/// The item produced by the [iterator](Iter).
#[derive(Clone, PartialEq, Debug)]
pub enum Item {
    /// A change between the index and the worktree.
    ///
    /// Note that untracked changes are also collected here.
    IndexWorktree(index_worktree::Item),
    /// A change between the three of `HEAD` and the index.
    TreeIndex(gix_diff::index::Change),
}

/// The data the thread sends over to the receiving iterator.
pub struct Outcome {
    /// The outcome of the index-to-worktree comparison operation.
    pub index_worktree: gix_status::index_as_worktree_with_renames::Outcome,
    /// The outcome of the diff between `HEAD^{tree}` and the index, or `None` if this outcome
    /// was produced with the [`into_index_worktree_iter()`](crate::status::Platform::into_index_worktree_iter()).
    pub tree_index: Option<tree_index::Outcome>,
    /// The worktree index that was used for the operation.
    pub worktree_index: IndexPersistedOrInMemory,
    pub(super) skip_hash: bool,
    pub(super) changes: Option<Vec<(usize, ApplyChange)>>,
}

impl Outcome {
    /// Returns `true` if the index has received currently unapplied changes that *should* be written back.
    ///
    /// If they are not written back, subsequent `status` operations will take longer to complete, whereas the
    /// additional work can be prevented by writing the changes back to the index.
    pub fn has_changes(&self) -> bool {
        self.changes.as_ref().is_some_and(|changes| !changes.is_empty())
    }

    /// Write the changes if there are any back to the index file.
    /// This can only be done once as the changes are consumed in the process, if there were any.
    pub fn write_changes(&mut self) -> Option<Result<(), gix_index::file::write::Error>> {
        let _span = gix_features::trace::coarse!("gix::status::index_worktree::Outcome::write_changes()");
        let changes = self.changes.take()?;
        let mut index = match &self.worktree_index {
            IndexPersistedOrInMemory::Persisted(persisted) => (***persisted).clone(),
            IndexPersistedOrInMemory::InMemory(index) => index.clone(),
        };

        let entries = index.entries_mut_keep_tree_cache();
        for (entry_index, change) in changes {
            let entry = &mut entries[entry_index];
            match change {
                ApplyChange::SetSizeToZero => {
                    entry.stat.size = 0;
                }
                ApplyChange::NewStat(new_stat) => {
                    entry.stat = new_stat;
                }
            }
        }

        Some(index.write(crate::index::write::Options {
            extensions: Default::default(),
            skip_hash: self.skip_hash,
        }))
    }
}

pub(super) enum ApplyChange {
    SetSizeToZero,
    NewStat(crate::index::entry::Stat),
}

impl From<index_worktree::Item> for Item {
    fn from(value: index_worktree::Item) -> Self {
        Item::IndexWorktree(value)
    }
}

impl From<gix_diff::index::Change> for Item {
    fn from(value: gix_diff::index::Change) -> Self {
        Item::TreeIndex(value)
    }
}

/// Access
impl Item {
    /// Return the relative path at which the item can currently be found in the working tree or index.
    pub fn location(&self) -> &BStr {
        match self {
            Item::IndexWorktree(change) => change.rela_path(),
            Item::TreeIndex(change) => change.location(),
        }
    }
}
