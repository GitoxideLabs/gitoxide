use crate::bstr::{BStr, BString};
use crate::{config, Repository};
use gix_status::index_as_worktree::traits::{CompareBlobs, SubmoduleStatus};
use std::sync::atomic::AtomicBool;

/// The error returned by [Repository::index_worktree_status()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("A working tree is required to perform a directory walk")]
    MissingWorkDir,
    #[error(transparent)]
    AttributesAndExcludes(#[from] crate::repository::attributes::Error),
    #[error(transparent)]
    Pathspec(#[from] crate::pathspec::init::Error),
    #[error(transparent)]
    Prefix(#[from] gix_path::realpath::Error),
    #[error(transparent)]
    FilesystemOptions(#[from] config::boolean::Error),
    #[error(transparent)]
    IndexAsWorktreeWithRenames(#[from] gix_status::index_as_worktree_with_renames::Error),
    #[error(transparent)]
    StatOptions(#[from] config::stat_options::Error),
    #[error(transparent)]
    ResourceCache(#[from] crate::diff::resource_cache::Error),
}

/// Options for use with [Repository::index_worktree_status()].
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// The way all output should be sorted.
    ///
    /// If `None`, and depending on the `rewrites` field, output will be immediate but the output order
    /// isn't determined, and may differ between two runs. `rewrites` also depend on the order of entries that
    /// are presented to it, hence for deterministic results, sorting needs to be enabled.
    ///
    /// If `Some(_)`, all entries are collected beforehand, so they can be sorted before outputting any of them
    /// to the user.
    ///
    /// If immediate output of entries in any order is desired, this should be `None`,
    /// along with `rewrites` being `None` as well.
    pub sorting: Option<gix_status::index_as_worktree_with_renames::Sorting>,
    /// If not `None`, the options to configure the directory walk, determining how its results will look like.
    ///
    /// If `None`, only modification checks are performed.
    ///
    /// Can be instantiated with [Repository::dirwalk_options()].
    pub dirwalk_options: Option<crate::dirwalk::Options>,
    /// If `Some(_)`, along with `Some(_)` in `dirwalk_options`, rewrite tracking will be performed between the
    /// index and the working tree.
    /// Note that there is no git-configuration specific to index-worktree rename tracking.
    /// When rewrite tracking is enabled, there will be a delay for some entries as they partake in the rename-analysis.
    pub rewrites: Option<gix_diff::Rewrites>,
    /// If set, don't use more than this amount of threads for the tracked modification check.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
    pub thread_limit: Option<usize>,
}

impl Repository {
    /// Obtain the status between the index and the worktree, involving modification checks
    /// for all tracked files along with information about untracked (and posisbly ignored) files (if configured).
    ///
    /// * `index`
    ///     - The index to use for modification checks, and to know which files are tacked when applying the dirwalk.
    /// * `patterns`
    ///     - Optional patterns to use to limit the paths to look at. If empty, all paths are considered.
    /// * `delegate`
    ///     - The sink for receiving all status data.
    /// * `compare`
    ///     - The implementations for fine-grained control over what happens if a hash must be recalculated.
    /// * `submodule`
    ///      - Control what kind of information to retrieve when a submodule is encountered while traversing the index.
    /// * `progress`
    ///     - A progress indication for index modification checks.
    /// * `should_interrupt`
    ///     - A flag to stop the whole operation.
    /// * `options`
    ///     - Additional configuration for all parts of the operation.
    ///
    /// ### Note
    ///
    /// This is a lower-level method, prefer the [`status`](Repository::status()) method for greater ease of use.
    #[allow(clippy::too_many_arguments)]
    pub fn index_worktree_status<'index, T, U, E>(
        &self,
        index: &'index gix_index::State,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        delegate: &mut impl gix_status::index_as_worktree_with_renames::VisitEntry<
            'index,
            ContentChange = T,
            SubmoduleStatus = U,
        >,
        compare: impl CompareBlobs<Output = T> + Send + Clone,
        submodule: impl SubmoduleStatus<Output = U, Error = E> + Send + Clone,
        progress: &mut dyn gix_features::progress::Progress,
        should_interrupt: &AtomicBool,
        options: Options,
    ) -> Result<gix_status::index_as_worktree_with_renames::Outcome, Error>
    where
        T: Send + Clone,
        U: Send + Clone,
        E: std::error::Error + Send + Sync + 'static,
    {
        let _span = gix_trace::coarse!("gix::index_worktree_status");
        let workdir = self.workdir().ok_or(Error::MissingWorkDir)?;
        let attrs_and_excludes = self.attributes(
            index,
            crate::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            crate::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            None,
        )?;
        let pathspec =
            self.index_worktree_status_pathspec::<Error>(patterns, index, options.dirwalk_options.as_ref())?;

        let cwd = self.current_dir();
        let git_dir_realpath = crate::path::realpath_opts(self.git_dir(), cwd, crate::path::realpath::MAX_SYMLINKS)?;
        let fs_caps = self.filesystem_options()?;
        let accelerate_lookup = fs_caps.ignore_case.then(|| index.prepare_icase_backing());
        let resource_cache = crate::diff::resource_cache(
            self,
            gix_diff::blob::pipeline::Mode::ToGit,
            attrs_and_excludes.inner,
            gix_diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: Some(workdir.to_owned()),
            },
        )?;

        let out = gix_status::index_as_worktree_with_renames(
            index,
            workdir,
            delegate,
            compare,
            submodule,
            self.objects.clone().into_arc().expect("arc conversion always works"),
            progress,
            gix_status::index_as_worktree_with_renames::Context {
                pathspec: pathspec.search,
                resource_cache,
                should_interrupt,
                dirwalk: gix_status::index_as_worktree_with_renames::DirwalkContext {
                    git_dir_realpath: git_dir_realpath.as_path(),
                    current_dir: cwd,
                    ignore_case_index_lookup: accelerate_lookup.as_ref(),
                },
            },
            gix_status::index_as_worktree_with_renames::Options {
                sorting: options.sorting,
                object_hash: self.object_hash(),
                tracked_file_modifications: gix_status::index_as_worktree::Options {
                    fs: fs_caps,
                    thread_limit: options.thread_limit,
                    stat: self.stat_options()?,
                },
                dirwalk: options.dirwalk_options.map(Into::into),
                rewrites: options.rewrites,
            },
        )?;
        Ok(out)
    }

    pub(super) fn index_worktree_status_pathspec<E>(
        &self,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        index: &gix_index::State,
        options: Option<&crate::dirwalk::Options>,
    ) -> Result<crate::Pathspec<'_>, E>
    where
        E: From<crate::repository::attributes::Error> + From<crate::pathspec::init::Error>,
    {
        let empty_patterns_match_prefix = options.is_some_and(|opts| opts.empty_patterns_match_prefix);
        let attrs_and_excludes = self.attributes(
            index,
            crate::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            crate::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            None,
        )?;
        Ok(crate::Pathspec::new(
            self,
            empty_patterns_match_prefix,
            patterns,
            true, /* inherit ignore case */
            move || Ok(attrs_and_excludes.inner),
        )?)
    }
}

/// An implementation of a trait to use with [`Repository::index_worktree_status()`] to compute the submodule status
/// using [Submodule::status()](crate::Submodule::status()).
#[derive(Clone)]
pub struct BuiltinSubmoduleStatus {
    mode: crate::status::Submodule,
    #[cfg(feature = "parallel")]
    repo: crate::ThreadSafeRepository,
    #[cfg(not(feature = "parallel"))]
    git_dir: std::path::PathBuf,
    submodule_paths: Vec<BString>,
}

///
mod submodule_status {
    use crate::bstr;
    use crate::bstr::BStr;
    use crate::status::index_worktree::BuiltinSubmoduleStatus;
    use crate::status::Submodule;
    use std::borrow::Cow;

    impl BuiltinSubmoduleStatus {
        /// Create a new instance from a `repo` and a `mode` to control how the submodule status will be obtained.
        pub fn new(
            repo: crate::ThreadSafeRepository,
            mode: Submodule,
        ) -> Result<Self, crate::submodule::modules::Error> {
            let local_repo = repo.to_thread_local();
            let submodule_paths = match local_repo.submodules() {
                Ok(Some(sm)) => {
                    let mut v: Vec<_> = sm.filter_map(|sm| sm.path().ok().map(Cow::into_owned)).collect();
                    v.sort();
                    v
                }
                Ok(None) => Vec::new(),
                Err(crate::submodule::modules::Error::FindHeadCommit(
                    crate::reference::head_commit::Error::PeelToCommit(
                        crate::head::peel::to_commit::Error::PeelToObject(
                            crate::head::peel::to_object::Error::Unborn { .. },
                        ),
                    ),
                )) => Vec::new(),
                Err(err) => return Err(err),
            };
            Ok(Self {
                mode,
                #[cfg(feature = "parallel")]
                repo,
                #[cfg(not(feature = "parallel"))]
                git_dir: local_repo.git_dir().to_owned(),
                submodule_paths,
            })
        }
    }

    /// The error returned submodule status checks.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        SubmoduleStatus(#[from] crate::submodule::status::Error),
        #[error(transparent)]
        IgnoreConfig(#[from] crate::submodule::config::Error),
    }

    impl gix_status::index_as_worktree::traits::SubmoduleStatus for BuiltinSubmoduleStatus {
        type Output = crate::submodule::Status;
        type Error = Error;

        fn status(&mut self, _entry: &gix_index::Entry, rela_path: &BStr) -> Result<Option<Self::Output>, Self::Error> {
            use bstr::ByteSlice;
            if self
                .submodule_paths
                .binary_search_by(|path| path.as_bstr().cmp(rela_path))
                .is_err()
            {
                return Ok(None);
            }
            #[cfg(feature = "parallel")]
            let repo = self.repo.to_thread_local();
            #[cfg(not(feature = "parallel"))]
            let Ok(repo) = crate::open(&self.git_dir) else {
                return Ok(None);
            };
            let Ok(Some(mut submodules)) = repo.submodules() else {
                return Ok(None);
            };
            let Some(sm) = submodules.find(|sm| sm.path().is_ok_and(|path| path == rela_path)) else {
                return Ok(None);
            };
            let (ignore, check_dirty) = match self.mode {
                Submodule::AsConfigured { check_dirty } => (sm.ignore()?.unwrap_or_default(), check_dirty),
                Submodule::Given { ignore, check_dirty } => (ignore, check_dirty),
            };
            let status = sm.status(ignore, check_dirty)?;
            Ok(status.is_dirty().and_then(|dirty| dirty.then_some(status)))
        }
    }
}

/// An iterator for changes between the index and the worktree.
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
/// Changes to the index are collected and it's possible to write the index back using [Outcome::write_changes()](crate::status::Outcome).
/// Note that these changes are not observable, they will always be kept.
///
/// ### Parallel Operation
///
/// Note that without the `parallel` feature, the iterator becomes 'serial', which means all status will be computed in advance
/// and it's non-interruptible, yielding worse performance for is-dirty checks for instance as interruptions won't happen.
/// It's a crutch that is just there to make single-threaded applications possible at all, as it's not really an iterator
/// anymore. If this matters, better run [Repository::index_worktree_status()] by hand as it provides all control one would need,
/// just not as an iterator.
///
/// Also, even with `parallel` set, the first call to `next()` will block until there is an item available, without a chance
/// to interrupt unless [`status::Platform::should_interrupt_*()`](crate::status::Platform::should_interrupt_shared()) was
/// configured.
pub struct Iter {
    inner: crate::status::Iter,
}

/// The item produced by the iterator
#[derive(Clone, PartialEq, Debug)]
pub enum Item {
    /// A tracked file was modified, and index-specific information is passed.
    Modification {
        /// The entry with modifications.
        entry: gix_index::Entry,
        /// The index of the `entry` for lookup in [`gix_index::State::entries()`] - useful to look at neighbors.
        entry_index: usize,
        /// The repository-relative path of the entry.
        rela_path: BString,
        /// The computed status of the entry.
        status: gix_status::index_as_worktree::EntryStatus<(), crate::submodule::Status>,
    },
    /// An entry returned by the directory walk, without any relation to the index.
    ///
    /// This can happen if ignored files are returned as well, or if rename-tracking is disabled.
    DirectoryContents {
        /// The entry found during the disk traversal.
        entry: gix_dir::Entry,
        /// `collapsed_directory_status` is `Some(dir_status)` if this `entry` was part of a directory with the given
        /// `dir_status` that wasn't the same as the one of `entry` and if [gix_dir::walk::Options::emit_collapsed] was
        /// [CollapsedEntriesEmissionMode::OnStatusMismatch](gix_dir::walk::CollapsedEntriesEmissionMode::OnStatusMismatch).
        /// It will also be `Some(dir_status)` if that option was [CollapsedEntriesEmissionMode::All](gix_dir::walk::CollapsedEntriesEmissionMode::All).
        collapsed_directory_status: Option<gix_dir::entry::Status>,
    },
    /// The rewrite tracking discovered a match between a deleted and added file, and considers them equal enough,
    /// depending on the tracker settings.
    ///
    /// Note that the source of the rewrite is always the index as it detects the absence of entries, something that
    /// can't be done during a directory walk.
    Rewrite {
        /// The source of the rewrite operation.
        source: RewriteSource,
        /// The untracked entry found during the disk traversal, the destination of the rewrite.
        ///
        /// Note that its [`rela_path`](gix_dir::EntryRef::rela_path) is the destination of the rewrite, and the current
        /// location of the entry.
        dirwalk_entry: gix_dir::Entry,
        /// `collapsed_directory_status` is `Some(dir_status)` if this `dirwalk_entry` was part of a directory with the given
        /// `dir_status` that wasn't the same as the one of `dirwalk_entry` and if [gix_dir::walk::Options::emit_collapsed] was
        /// [CollapsedEntriesEmissionMode::OnStatusMismatch](gix_dir::walk::CollapsedEntriesEmissionMode::OnStatusMismatch).
        /// It will also be `Some(dir_status)` if that option was [CollapsedEntriesEmissionMode::All](gix_dir::walk::CollapsedEntriesEmissionMode::All).
        dirwalk_entry_collapsed_directory_status: Option<gix_dir::entry::Status>,
        /// The object id after the rename, specifically hashed in order to determine equality.
        dirwalk_entry_id: gix_hash::ObjectId,
        /// It's `None` if the 'source.id' is equal to `dirwalk_entry_id`, as identity made an actual diff computation unnecessary.
        /// Otherwise, and if enabled, it's `Some(stats)` to indicate how similar both entries were.
        diff: Option<gix_diff::blob::DiffLineStats>,
        /// If true, this rewrite is created by copy, and 'source.id' is pointing to its source.
        /// Otherwise, it's a rename, and 'source.id' points to a deleted object,
        /// as renames are tracked as deletions and additions of the same or similar content.
        copy: bool,
    },
}

/// Either an index entry for renames or another directory entry in case of copies.
#[derive(Clone, PartialEq, Debug)]
pub enum RewriteSource {
    /// The source originates in the index and is detected as missing in the working tree.
    /// This can also happen for copies.
    RewriteFromIndex {
        /// The entry that is the source of the rewrite, which means it was removed on disk,
        /// equivalent to [Change::Removed](gix_status::index_as_worktree::Change::Removed).
        ///
        /// Note that the [entry-id](gix_index::Entry::id) is the content-id of the source of the rewrite.
        source_entry: gix_index::Entry,
        /// The index of the `source_entry` for lookup in [`gix_index::State::entries()`] - useful to look at neighbors.
        source_entry_index: usize,
        /// The repository-relative path of the `source_entry`.
        source_rela_path: BString,
        /// The computed status of the `source_entry`.
        source_status: gix_status::index_as_worktree::EntryStatus<(), crate::submodule::Status>,
    },
    /// This source originates in the directory tree and is always the source of copies.
    CopyFromDirectoryEntry {
        /// The source of the copy operation, which is also an entry of the directory walk.
        ///
        /// Note that its [`rela_path`](gix_dir::EntryRef::rela_path) is the source of the rewrite.
        source_dirwalk_entry: gix_dir::Entry,
        /// `collapsed_directory_status` is `Some(dir_status)` if this `source_dirwalk_entry` was part of a directory with the given
        /// `dir_status` that wasn't the same as the one of `source_dirwalk_entry` and
        /// if [gix_dir::walk::Options::emit_collapsed] was [CollapsedEntriesEmissionMode::OnStatusMismatch](gix_dir::walk::CollapsedEntriesEmissionMode::OnStatusMismatch).
        /// It will also be `Some(dir_status)` if that option was [CollapsedEntriesEmissionMode::All](gix_dir::walk::CollapsedEntriesEmissionMode::All).
        source_dirwalk_entry_collapsed_directory_status: Option<gix_dir::entry::Status>,
        /// The object id as it would appear if the entry was written to the object database.
        /// It's the same as [`dirwalk_entry_id`](Item::Rewrite), or `diff` is `Some(_)` to indicate that the copy
        /// was determined by similarity, not by content equality.
        source_dirwalk_entry_id: gix_hash::ObjectId,
    },
}

///
pub mod iter {
    use crate::bstr::{BStr, BString};
    use crate::status::{index_worktree, Platform};
    use gix_status::index_as_worktree::{Change, EntryStatus};

    use super::{Item, RewriteSource};
    pub use gix_status::index_as_worktree_with_renames::Summary;

    /// Access
    impl RewriteSource {
        /// The repository-relative path of this source.
        pub fn rela_path(&self) -> &BStr {
            match self {
                RewriteSource::RewriteFromIndex { source_rela_path, .. } => source_rela_path.as_ref(),
                RewriteSource::CopyFromDirectoryEntry {
                    source_dirwalk_entry, ..
                } => source_dirwalk_entry.rela_path.as_ref(),
            }
        }
    }

    impl<'index> From<gix_status::index_as_worktree_with_renames::RewriteSource<'index, (), SubmoduleStatus>>
        for RewriteSource
    {
        fn from(value: gix_status::index_as_worktree_with_renames::RewriteSource<'index, (), SubmoduleStatus>) -> Self {
            match value {
                gix_status::index_as_worktree_with_renames::RewriteSource::RewriteFromIndex {
                    index_entries: _,
                    source_entry,
                    source_entry_index,
                    source_rela_path,
                    source_status,
                } => RewriteSource::RewriteFromIndex {
                    source_entry: source_entry.clone(),
                    source_entry_index,
                    source_rela_path: source_rela_path.to_owned(),
                    source_status,
                },
                gix_status::index_as_worktree_with_renames::RewriteSource::CopyFromDirectoryEntry {
                    source_dirwalk_entry,
                    source_dirwalk_entry_collapsed_directory_status,
                    source_dirwalk_entry_id,
                } => RewriteSource::CopyFromDirectoryEntry {
                    source_dirwalk_entry,
                    source_dirwalk_entry_collapsed_directory_status,
                    source_dirwalk_entry_id,
                },
            }
        }
    }

    impl Item {
        /// Return a simplified summary of the item as digest of its status, or `None` if this item is
        /// created from the directory walk and is *not untracked*, or if it is merely to communicate
        /// a needed update to the index entry.
        pub fn summary(&self) -> Option<Summary> {
            use gix_status::index_as_worktree_with_renames::Summary::*;
            Some(match self {
                Item::Modification { status, .. } => match status {
                    EntryStatus::Conflict(_) => Conflict,
                    EntryStatus::Change(change) => match change {
                        Change::Removed => Removed,
                        Change::Type { .. } => TypeChange,
                        Change::Modification { .. } | Change::SubmoduleModification(_) => Modified,
                    },
                    EntryStatus::NeedsUpdate(_) => return None,
                    EntryStatus::IntentToAdd => IntentToAdd,
                },
                Item::DirectoryContents { entry, .. } => {
                    if matches!(entry.status, gix_dir::entry::Status::Untracked) {
                        Added
                    } else {
                        return None;
                    }
                }
                Item::Rewrite { copy, .. } => {
                    if *copy {
                        Copied
                    } else {
                        Renamed
                    }
                }
            })
        }

        /// The repository-relative path of the entry contained in this item.
        pub fn rela_path(&self) -> &BStr {
            match self {
                Item::Modification { rela_path, .. } => rela_path.as_ref(),
                Item::DirectoryContents { entry, .. } => entry.rela_path.as_ref(),
                Item::Rewrite { dirwalk_entry, .. } => dirwalk_entry.rela_path.as_ref(),
            }
        }
    }

    impl<'index> From<gix_status::index_as_worktree_with_renames::Entry<'index, (), SubmoduleStatus>> for Item {
        fn from(value: gix_status::index_as_worktree_with_renames::Entry<'index, (), SubmoduleStatus>) -> Self {
            match value {
                gix_status::index_as_worktree_with_renames::Entry::Modification {
                    entries: _,
                    entry,
                    entry_index,
                    rela_path,
                    status,
                } => Item::Modification {
                    entry: entry.clone(),
                    entry_index,
                    rela_path: rela_path.to_owned(),
                    status,
                },
                gix_status::index_as_worktree_with_renames::Entry::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                } => Item::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                },
                gix_status::index_as_worktree_with_renames::Entry::Rewrite {
                    source,
                    dirwalk_entry,
                    dirwalk_entry_collapsed_directory_status,
                    dirwalk_entry_id,
                    diff,
                    copy,
                } => Item::Rewrite {
                    source: source.into(),
                    dirwalk_entry,
                    dirwalk_entry_collapsed_directory_status,
                    dirwalk_entry_id,
                    diff,
                    copy,
                },
            }
        }
    }

    type SubmoduleStatus = crate::submodule::Status;

    /// Lifecycle
    impl<Progress> Platform<'_, Progress>
    where
        Progress: gix_features::progress::Progress,
    {
        /// Turn the platform into an iterator for changes between the index and the working tree.
        ///
        /// * `patterns`
        ///     - Optional patterns to use to limit the paths to look at. If empty, all paths are considered.
        #[doc(alias = "diff_index_to_workdir", alias = "git2")]
        pub fn into_index_worktree_iter(
            mut self,
            patterns: impl IntoIterator<Item = BString>,
        ) -> Result<index_worktree::Iter, crate::status::into_iter::Error> {
            // deactivate the tree-iteration
            self.head_tree = None;
            Ok(index_worktree::Iter {
                inner: self.into_iter(patterns)?,
            })
        }
    }

    impl Iterator for super::Iter {
        type Item = Result<Item, index_worktree::Error>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|res| {
                res.map(|item| match item {
                    crate::status::Item::IndexWorktree(item) => item,
                    crate::status::Item::TreeIndex(_) => unreachable!("BUG: we deactivated this kind of traversal"),
                })
                .map_err(|err| match err {
                    crate::status::iter::Error::IndexWorktree(err) => err,
                    crate::status::iter::Error::TreeIndex(_) => {
                        unreachable!("BUG: we deactivated this kind of traversal")
                    }
                })
            })
        }
    }

    /// Access
    impl super::Iter {
        /// Return the outcome of the iteration, or `None` if the iterator isn't fully consumed.
        pub fn outcome_mut(&mut self) -> Option<&mut crate::status::Outcome> {
            self.inner.out.as_mut()
        }

        /// Turn the iterator into the iteration outcome, which is `None` on error or if the iteration
        /// isn't complete.
        pub fn into_outcome(mut self) -> Option<crate::status::Outcome> {
            self.inner.out.take()
        }
    }
}
