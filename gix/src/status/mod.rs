pub use gix_status as plumbing;

use crate::{Repository, config, config::cache::util::ApplyLeniencyDefault, util::OwnedOrStaticAtomicBool};

/// A structure to hold options configuring the status request, which can then be turned into an iterator.
pub struct Platform<'repo, Progress>
where
    Progress: gix_features::progress::Progress + 'static,
{
    repo: &'repo Repository,
    progress: Progress,
    index: Option<crate::worktree::IndexPersistedOrInMemory>,
    head_tree: Option<Option<gix_hash::ObjectId>>,
    submodules: Submodule,
    index_worktree_options: index_worktree::Options,
    tree_index_renames: tree_index::TrackRenames,
    should_interrupt: Option<OwnedOrStaticAtomicBool>,
    /// Windows-only: when `true` (default), run a single batched parallel
    /// directory walk before status to precompute worktree stats so that the
    /// per-entry pipeline can skip per-file `lstat`. Ignored on non-Windows.
    /// See [`crate::status::Platform::index_worktree_stats_preprocessing`].
    #[cfg(windows)]
    precompute_worktree_stats: bool,
}

/// How to obtain a submodule's status.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Submodule {
    /// Use the `diff.submoduleIgnore` configuration to determine, or if not set,
    /// use the submodule's own ['ignore' value](crate::Submodule::ignore) to determine
    /// which submodules participate in the status query, and to which extent.
    AsConfigured {
        /// If `true`, default `false`, the computation will stop once the first in a ladder operations
        /// ordered from cheap to expensive shows that the submodule is dirty.
        /// Thus, submodules that are clean will still impose the complete set of computation, as configured.
        check_dirty: bool,
    },
    /// Instead of the configuration, use the given ['ignore' value](crate::submodule::config::Ignore).
    /// This makes it possible to fine-tune the amount of work invested in this status, while allowing
    /// to turn off all submodule status information.
    Given {
        /// The portion of the submodule status to ignore.
        ignore: crate::submodule::config::Ignore,
        /// If `true`, default `false`, the computation will stop once the first in a ladder operations
        /// ordered from cheap to expensive shows that the submodule is dirty.
        /// Thus, submodules that are clean will still impose the complete set of computation, as given.
        check_dirty: bool,
    },
}

/// How untracked files should be handled.
#[derive(Default, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum UntrackedFiles {
    /// Do not show any untracked files.
    ///
    /// This can mean no directory walk is performed.
    None,
    /// If possible, collapse files into their parent folders to reduce the amount of
    /// emitted untracked files.
    #[default]
    Collapsed,
    /// Show each individual untracked file or directory (if empty directories are emitted) that the dirwalk encountered .
    Files,
}

impl Default for Submodule {
    fn default() -> Self {
        Submodule::AsConfigured { check_dirty: false }
    }
}

/// The error returned by [status()](Repository::status).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    DirwalkOptions(#[from] config::boolean::Error),
    #[error(transparent)]
    ConfigureUntrackedFiles(#[from] config::key::GenericErrorWithValue),
}

/// Status
impl Repository {
    /// Obtain a platform for configuring iterators for traversing git repository status information.
    ///
    /// By default, this is set to the fastest and most immediate way of obtaining a status,
    /// which is most similar to
    ///
    /// `git status --ignored=no`
    ///
    /// which implies that submodule information is provided by default.
    ///
    /// Note that `status.showUntrackedFiles` is respected, which leads to untracked files being
    /// collapsed by default. If that needs to be controlled,
    /// [configure the directory walk explicitly](Platform::dirwalk_options) or more [implicitly](Platform::untracked_files).
    ///
    /// Pass `progress` to receive progress information on file modifications on this repository.
    /// Use [`progress::Discard`](crate::progress::Discard) to discard all progress information.
    ///
    /// ### Deviation
    ///
    /// Whereas Git runs the index-modified check before the directory walk to set entries
    /// as up-to-date to (potentially) safe some disk-access, we run both in parallel which
    /// ultimately is much faster.
    pub fn status<P>(&self, progress: P) -> Result<Platform<'_, P>, Error>
    where
        P: gix_features::progress::Progress + 'static,
    {
        let platform = Platform {
            repo: self,
            progress,
            index: None,
            submodules: Submodule::default(),
            should_interrupt: None,
            head_tree: Some(None),
            tree_index_renames: Default::default(),
            index_worktree_options: index_worktree::Options {
                sorting: None,
                dirwalk_options: Some(self.dirwalk_options()?),
                rewrites: None,
                thread_limit: None,
            },
            #[cfg(windows)]
            precompute_worktree_stats: true,
        };

        let untracked = self
            .config
            .resolved
            .string(config::tree::Status::SHOW_UNTRACKED_FILES)
            .map(|value| {
                config::tree::Status::SHOW_UNTRACKED_FILES
                    .try_into_show_untracked_files(value)
                    .with_lenient_default(self.config.lenient_config)
            })
            .transpose()?
            .unwrap_or_default();
        Ok(platform.untracked_files(untracked))
    }
}

///
pub mod is_dirty {
    use std::convert::Infallible;

    use crate::Repository;

    /// The error returned by [Repository::is_dirty()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        StatusPlatform(#[from] crate::status::Error),
        #[error(transparent)]
        CreateStatusIterator(#[from] crate::status::into_iter::Error),
        #[error(transparent)]
        TreeIndexStatus(#[from] crate::status::tree_index::Error),
        #[error(transparent)]
        HeadTreeId(#[from] crate::reference::head_tree_id::Error),
        #[error(transparent)]
        OpenWorktreeIndex(#[from] crate::worktree::open_index::Error),
    }

    impl Repository {
        /// Returns `true` if the repository is dirty.
        /// This means it's changed in one of the following ways:
        ///
        /// * the index was changed in comparison to its working tree
        /// * the working tree was changed in comparison to the index
        /// * submodules are taken in consideration, along with their `ignore` and `isActive` configuration
        ///
        /// Note that *untracked files* do *not* affect this flag.
        // TODO(performance): this could be its very own implementation with parallelism and the special:
        //                    stop once there is a change flag, but without using the iterator for
        //                    optimal resource usage.
        pub fn is_dirty(&self) -> Result<bool, Error> {
            {
                let head_tree_id = self.head_tree_id_or_empty()?;
                let mut index_is_dirty = false;

                // Run this first as there is a high likelihood to find something, and it's very fast.
                self.tree_index_status(
                    &head_tree_id,
                    &*self.index_or_empty()?,
                    None,
                    crate::status::tree_index::TrackRenames::Disabled,
                    |_, _, _| {
                        index_is_dirty = true;
                        Ok::<_, Infallible>(std::ops::ControlFlow::Break(()))
                    },
                )?;
                if index_is_dirty {
                    return Ok(true);
                }
            }
            let is_dirty = self
                .status(gix_features::progress::Discard)?
                .index_worktree_rewrites(None)
                .index_worktree_submodules(crate::status::Submodule::AsConfigured { check_dirty: true })
                .index_worktree_options_mut(|opts| {
                    opts.dirwalk_options = None;
                })
                .into_index_worktree_iter(Vec::new())?
                .take_while(Result::is_ok)
                .next()
                .is_some();
            Ok(is_dirty)
        }
    }
}

///
pub mod into_iter {
    /// The error returned by [status::Platform::into_iter()](crate::status::Platform::into_iter()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Index(#[from] crate::worktree::open_index::Error),
        #[error("Failed to spawn producer thread")]
        #[cfg(feature = "parallel")]
        SpawnThread(#[source] std::io::Error),
        #[error(transparent)]
        #[cfg(not(feature = "parallel"))]
        IndexWorktreeStatus(#[from] crate::status::index_worktree::Error),
        #[error(transparent)]
        ConfigSkipHash(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        PrepareSubmodules(#[from] crate::submodule::modules::Error),
        #[error("Could not create an index for the head tree to compare with the worktree index")]
        HeadTreeIndex(#[from] crate::repository::index_from_tree::Error),
        #[error("Could not obtain the tree id pointed to by `HEAD`")]
        HeadTreeId(#[from] crate::reference::head_tree_id::Error),
        #[error(transparent)]
        AttributesAndExcludes(#[from] crate::repository::attributes::Error),
        #[error(transparent)]
        Pathspec(#[from] crate::pathspec::init::Error),
        #[error(transparent)]
        HeadTreeDiff(#[from] crate::status::tree_index::Error),
    }
}

/// Run the gitignore-aware Windows worktree stats preprocessing pass.
/// One internal helper, called from the iterator on Windows unless
/// preprocessing is disabled. Returns `None` when the repo lacks a workdir,
/// no index entry falls under the pathspec's `common_prefix` (then there is
/// nothing to accelerate), or the walk fails — callers fall through to live
/// `lstat` either way, so surfacing the failure as a hard error would be
/// unhelpful.
///
/// The walk is seeded at the deepest directory `common_prefix` pins down, so
/// a status restricted to a subtree only enumerates that subtree. When in
/// doubt — the prefix tail may be a partial file name — it walks the parent
/// instead: enumerating too much costs a few milliseconds, while every miss
/// is a per-file syscall.
#[cfg(windows)]
pub(crate) fn precomputed_worktree_stats(
    repo: &Repository,
    index: &gix_index::State,
    common_prefix: &crate::bstr::BStr,
    thread_limit: Option<usize>,
    should_interrupt: &std::sync::atomic::AtomicBool,
) -> Option<gix_status::worktree_stats::WorktreeStats> {
    let workdir = repo.workdir()?;
    let range = index.prefixed_entries_range(common_prefix)?;
    if range.is_empty() {
        return None;
    }
    let start_dir = {
        let prefix = common_prefix.strip_suffix(b"/").unwrap_or(common_prefix);
        // Walk `prefix` itself only if every entry to check lives beneath it as a
        // directory; entries are sorted, so its first and last suffice as witnesses.
        let entries = &index.entries()[range];
        let lives_below_prefix = |entry: &gix_index::Entry| {
            let path = entry.path_in(index.path_backing());
            path.starts_with(prefix) && path.get(prefix.len()) == Some(&b'/')
        };
        if !prefix.is_empty() && lives_below_prefix(&entries[0]) && lives_below_prefix(&entries[entries.len() - 1]) {
            prefix
        } else {
            &prefix[..prefix.iter().rposition(|&b| b == b'/').unwrap_or(0)]
        }
    };
    // Without the `parallel` feature, repository internals are `Rc`-based and the
    // per-thread excludes factory can't be `Sync` — build a single predicate on this
    // thread and walk single-threaded instead.
    #[cfg(feature = "parallel")]
    {
        let sync_repo = repo.clone().into_sync();

        let make_excludes = || -> Box<dyn FnMut(&crate::bstr::BStr) -> bool> {
            let thread_repo = sync_repo.to_thread_local();
            let Ok(stack) = thread_repo.excludes(
                index,
                None,
                gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            ) else {
                return Box::new(|_| false);
            };
            let mut stack = stack.detach();
            let objects = thread_repo.objects.clone();
            Box::new(move |path: &crate::bstr::BStr| -> bool {
                stack
                    .at_entry(path, Some(gix_index::entry::Mode::DIR), &objects)
                    .map(|p| p.is_excluded())
                    .unwrap_or(false)
            })
        };

        gix_status::worktree_stats::prepare(
            workdir,
            crate::bstr::BStr::new(start_dir),
            thread_limit,
            should_interrupt,
            make_excludes,
        )
        .ok()
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = thread_limit;
        let is_excluded: Box<dyn FnMut(&crate::bstr::BStr) -> bool> = match repo.excludes(
            index,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        ) {
            Ok(stack) => {
                let mut stack = stack.detach();
                let objects = repo.objects.clone();
                Box::new(move |path: &crate::bstr::BStr| -> bool {
                    stack
                        .at_entry(path, Some(gix_index::entry::Mode::DIR), &objects)
                        .map(|p| p.is_excluded())
                        .unwrap_or(false)
                })
            }
            Err(_) => Box::new(|_| false),
        };

        gix_status::worktree_stats::prepare_single_threaded(
            workdir,
            crate::bstr::BStr::new(start_dir),
            should_interrupt,
            is_excluded,
        )
        .ok()
    }
}

mod platform;

///
pub mod index_worktree;

///
pub mod tree_index;

///
pub mod iter;
pub use iter::types::{Item, Iter, Outcome};
