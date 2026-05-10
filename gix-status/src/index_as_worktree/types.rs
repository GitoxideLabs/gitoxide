use std::sync::atomic::AtomicBool;

use bstr::{BStr, BString};
use gix_index::entry;

#[cfg(windows)]
use crate::metadata_cache::MetadataCache;

/// The error returned by [index_as_worktree()`](crate::index_as_worktree()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not convert path to UTF8")]
    IllformedUtf8,
    #[error("The clock was off when reading file related metadata after updating a file on disk")]
    Time(#[from] std::time::SystemTimeError),
    #[error("IO error while writing blob or reading file metadata or changing filetype")]
    Io(#[from] gix_hash::io::Error),
    #[error("Failed to obtain blob from object database")]
    Find(#[from] gix_object::find::existing_object::Error),
    #[error("Could not determine status for submodule at '{rela_path}'")]
    SubmoduleStatus {
        rela_path: BString,
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}

/// Options that control how the index status with a worktree is computed.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct Options {
    /// Capabilities of the file system which affect the status computation.
    pub fs: gix_fs::Capabilities,
    /// If set, don't use more than this amount of threads.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
    pub thread_limit: Option<usize>,
    /// Options that control how stat comparisons are made when checking if a file is fresh.
    pub stat: gix_index::entry::stat::Options,
}

/// The context for [index_as_worktree()`](crate::index_as_worktree()).
#[derive(Clone)]
pub struct Context<'a> {
    /// The pathspec to limit the amount of paths that are checked. Can be empty to allow all paths.
    ///
    /// Note that these are expected to have a [common_prefix()](gix_pathspec::Search::common_prefix()) according
    /// to the prefix of the repository to efficiently limit the scope of the paths we process.
    pub pathspec: gix_pathspec::Search,
    /// A stack pre-configured to allow accessing attributes for each entry, as required for `filter`
    /// and possibly pathspecs.
    pub stack: gix_worktree::Stack,
    /// A filter to be able to perform conversions from and to the worktree format.
    ///
    /// It is needed to potentially refresh the index with data read from the worktree, which needs to be converted back
    /// to the form stored in Git.
    ///
    /// Note that for this to be correct, the attribute `stack` must be configured correctly as well.
    pub filter: gix_filter::Pipeline,
    /// A flag to query to learn if cancellation is requested.
    pub should_interrupt: &'a AtomicBool,
    /// Windows-only pre-populated metadata cache. See [`crate::metadata_cache`].
    /// Transparent: `None`/empty/partial are all correct.
    #[cfg(windows)]
    pub metadata_cache: Option<&'a MetadataCache>,
}

/// Provide additional information collected during the runtime of [`index_as_worktree()`](crate::index_as_worktree()).
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Outcome {
    /// The total amount of entries that is to be processed.
    pub entries_to_process: usize,
    /// The amount of entries we actually processed. If this isn't the entire set, the operation was interrupted.
    pub entries_processed: usize,
    /// The amount of entries we didn't even traverse (and thus update with stat) due to a common prefix in pathspecs.
    /// This is similar to the current working directory.
    pub entries_skipped_by_common_prefix: usize,
    /// The amount of entries that were skipped due to exclusion by *pathspecs*.
    pub entries_skipped_by_pathspec: usize,
    /// The amount of entries that were skipped as the entry flag indicated this.
    pub entries_skipped_by_entry_flags: usize,
    /// The amount of times we queried symlink-metadata for a file on disk.
    pub symlink_metadata_calls: usize,
    /// The amount of entries whose stats would need to be updated as its modification couldn't be determined without
    /// an expensive calculation.
    ///
    /// With these updates, this calculation will be avoided next time the status runs.
    /// Note that the stat updates are delegated to the caller.
    pub entries_to_update: usize,
    /// The amount of entries that were considered racy-clean - they will need thorough checking to see if they are truly clean,
    /// i.e. didn't change.
    pub racy_clean: usize,

    /// The amount of bytes read from the worktree in order to determine if an entry changed, across all files.
    pub worktree_bytes: u64,
    /// The amount of files read in full from the worktree (and into memory).
    pub worktree_files_read: usize,
    /// The amount of bytes read from the object database in order to determine if an entry changed, across all objects.
    pub odb_bytes: u64,
    /// The amount of objects read from the object database.
    pub odb_objects_read: usize,
}

impl Outcome {
    /// The total amount of skipped entries, i.e. those that weren't processed at all.
    pub fn skipped(&self) -> usize {
        self.entries_skipped_by_common_prefix + self.entries_skipped_by_pathspec + self.entries_skipped_by_entry_flags
    }
}

/// How an index entry needs to be changed to obtain the destination worktree state, i.e. `entry.apply(this_change) == worktree-entry`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Change<T = (), U = ()> {
    /// This corresponding file does not exist in the worktree anymore.
    Removed,
    /// The type of file changed compared to the worktree.
    ///
    /// Examples include when a symlink is now a regular file, or a regular file was replaced with a named pipe.
    ///
    /// ### Deviation
    ///
    /// A change to a non-file is marked as `modification` in Git, but that's related to the content which we can't evaluate.
    /// Hence, a type-change is considered more appropriate.
    Type {
        /// The mode the worktree file would have if it was added to the index, and the mode that differs compared
        /// to what's currently stored in the index.
        worktree_mode: gix_index::entry::Mode,
    },
    /// This worktree file was modified in some form, like a permission change or content change or both,
    /// as compared to this entry.
    Modification {
        /// Indicates that one of the stat changes was an executable bit change
        /// which is a significant change itself.
        executable_bit_changed: bool,
        /// The output of the [`CompareBlobs`](crate::index_as_worktree::traits::CompareBlobs) run on this entry.
        /// If there is no content change and only the executable bit
        /// changed then this is `None`.
        content_change: Option<T>,
        /// If true, the caller is expected to set [entry.stat.size = 0](gix_index::entry::Stat::size) to assure this
        /// otherwise racily clean entry can still be detected as dirty next time this is called, but this time without
        /// reading it from disk to hash it. It's a performance optimization and not doing so won't change the correctness
        /// of the operation.
        set_entry_stat_size_zero: bool,
    },
    /// A submodule is initialized and checked out, and there was modification to either:
    ///
    /// * the `HEAD` as compared to the superproject's desired commit for `HEAD`
    /// * the worktree has at least one modified file
    /// * there is at least one untracked file
    ///
    /// The exact nature of the modification is handled by the caller which may retain information per submodule or
    /// re-compute details as needed when seeing this variant.
    SubmoduleModification(U),
}

/// Like [`gix_index::Entry`], but without disk-metadata.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConflictIndexEntry {
    /// The object id for this entry's ODB representation (assuming it's up-to-date with it).
    pub id: gix_hash::ObjectId,
    /// Additional flags for use in algorithms and for efficiently storing stage information, primarily
    /// to obtain the [stage](entry::Flags::stage()).
    pub flags: entry::Flags,
    /// The kind of item this entry represents - it's not all blobs in the index anymore.
    pub mode: entry::Mode,
}

impl From<&gix_index::Entry> for ConflictIndexEntry {
    fn from(
        gix_index::Entry {
            stat: _,
            id,
            flags,
            mode,
            ..
        }: &gix_index::Entry,
    ) -> Self {
        ConflictIndexEntry {
            id: *id,
            flags: *flags,
            mode: *mode,
        }
    }
}

/// Information about an entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryStatus<T = (), U = ()> {
    /// The entry is in a conflicting state, and we provide all related entries along with a summary.
    Conflict {
        /// An analysis on the conflict itself based on the observed index entries.
        summary: Conflict,
        /// The entries from stage 1 to stage 3, where stage 1 is at index 0 and stage 3 at index 2.
        /// Note that when there are conflicts, there is no stage 0.
        /// Further, all entries are looking at the same path.
        entries: Box<[Option<ConflictIndexEntry>; 3]>,
    },
    /// There is no conflict and a change was discovered.
    Change(Change<T, U>),
    /// The entry didn't change, but its state caused extra work that can be avoided next time if its stats would be updated to the
    /// given stat.
    NeedsUpdate(
        /// The new stats which represent what's currently in the working tree. If these replace the current stats in the entry,
        /// next time this operation runs we can determine the actual state much faster.
        gix_index::entry::Stat,
    ),
    /// An index entry that corresponds to an untracked worktree file marked with `git add --intent-to-add`.
    ///
    /// This means it's not available in the object database yet even though now an entry exists that represents the worktree file.
    /// The entry represents the promise of adding a new file, no matter the actual stat or content.
    /// Effectively this means nothing changed.
    /// This also means the file is still present, and that no detailed change checks were performed.
    IntentToAdd,
}

impl<T, U> From<Change<T, U>> for EntryStatus<T, U> {
    fn from(value: Change<T, U>) -> Self {
        EntryStatus::Change(value)
    }
}

/// Describes a conflicting entry as comparison between 'our' version and 'their' version of it.
///
/// If one side isn't specified, it is assumed to have modified the entry. In general, there would be no conflict
/// if both parties ended up in the same state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Conflict {
    /// Both deleted a different version of the entry.
    BothDeleted,
    /// We added, they modified, ending up in different states.
    AddedByUs,
    /// They deleted the entry, we modified it.
    DeletedByThem,
    /// They added the entry, we modified it, ending up in different states.
    AddedByThem,
    /// We deleted the entry, they modified it, ending up in different states.
    DeletedByUs,
    /// Both added the entry in different states.
    BothAdded,
    /// Both modified the entry, ending up in different states.
    BothModified,
}

/// Observe the status of an entry by comparing an index entry to the worktree.
pub trait VisitEntry<'index> {
    /// Data generated by comparing an entry with a file.
    type ContentChange;
    /// Data obtained when checking the submodule status.
    type SubmoduleStatus;
    /// Observe the `status` of `entry` at the repository-relative `rela_path` at `entry_index`
    /// (for accessing `entry` and surrounding in the complete list of `entries`).
    fn visit_entry(
        &mut self,
        entries: &'index [gix_index::Entry],
        entry: &'index gix_index::Entry,
        entry_index: usize,
        rela_path: &'index BStr,
        status: EntryStatus<Self::ContentChange, Self::SubmoduleStatus>,
    );
}
