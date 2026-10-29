use bstr::BString;
use gix_index::entry::stat;

/// One explicitly selected worktree change for [`apply_changes()`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Change {
    /// Write a stage-zero entry from the supplied index, unless it is marked skip-worktree.
    Write {
        /// The entry's position in the index's entries array.
        entry_index: usize,
    },
    /// Remove a file, symlink, or empty directory at a repository-relative Git path.
    Remove {
        /// The path to remove, without traversing symlinked parent directories.
        path: BString,
    },
}

/// Information about a path that failed to checkout as something else was already present.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Collision {
    /// the path that collided with something already present on disk.
    pub path: BString,
    /// The io error we encountered when checking out `path`.
    pub error_kind: std::io::ErrorKind,
}

/// A path that encountered an error.
#[derive(Debug)]
pub struct ErrorRecord {
    /// the path that encountered the error.
    pub path: BString,
    /// The error.
    pub error: gix_error::Error,
}

/// The outcome of checking out an entire index.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The amount of files updated, or created.
    pub files_updated: usize,
    /// The amount of bytes written to disk,
    pub bytes_written: u64,
    /// The encountered collisions, which can happen on a case-insensitive filesystem.
    pub collisions: Vec<Collision>,
    /// Other errors that happened during checkout.
    pub errors: Vec<ErrorRecord>,
    /// Relative paths that the process listed as 'delayed' even though we never passed them.
    pub delayed_paths_unknown: Vec<BString>,
    /// All paths that were left unprocessed, because they were never listed by the process even though we passed them.
    pub delayed_paths_unprocessed: Vec<BString>,
}

/// Options to further configure the checkout operation.
///
/// Create these with [`Options::new()`] to supply a filter pipeline with the repository's object hash.
#[derive(Clone)]
pub struct Options {
    /// capabilities of the file system
    pub fs: gix_fs::Capabilities,
    /// Options to configure how to validate path components.
    pub validate: gix_worktree::validate::path::component::Options,
    /// The maximum number of worker threads. `None` and `Some(0)` use the available logical cores,
    /// capped at four on macOS to limit APFS creation contention. Positive limits override this tuning.
    /// Without `gix-parallel/parallel`, checkout runs on the calling thread.
    pub thread_limit: Option<usize>,
    /// If true, we assume no file to exist in the target directory, and want exclusive access to it.
    /// This should be enabled when cloning to avoid checks for freshness of files. This also enables
    /// detection of collisions based on whether or not exclusive file creation succeeds or fails.
    pub destination_is_initially_empty: bool,
    /// If true, default false, worktree entries on disk will be overwritten with content from the index
    /// even if they appear to be changed. When creating directories that clash with existing worktree entries,
    /// these will try to delete the existing entry.
    /// This is similar in behaviour as `git checkout --force`.
    ///
    /// Note that when `destination_is_initially_empty` is `false`, existing files may still have their
    /// executable bit updated to match the index. This option prevents overwriting file contents, but
    /// does not necessarily prevent metadata updates.
    pub overwrite_existing: bool,
    /// If true, default false, try to checkout as much as possible and don't abort on first error which isn't
    /// due to a conflict.
    /// The checkout operation will never fail, but count the encountered errors instead along with their paths.
    pub keep_going: bool,
    /// Control how stat comparisons are made when checking if a file is fresh.
    pub stat_options: stat::Options,
    /// A stack of attributes to use with the filesystem cache to use as driver for filters.
    pub attributes: gix_worktree::stack::state::Attributes,
    /// The filter pipeline to use for applying mandatory filters before writing to the worktree.
    pub filters: gix_filter::Pipeline,
    /// Control how long-running processes may use the 'delay' capability.
    pub filter_process_delay: gix_filter::driver::apply::Delay,
}

impl Options {
    fn thread_limit(&self) -> Option<usize> {
        #[cfg(target_os = "macos")]
        if self.thread_limit.is_none_or(|threads| threads == 0) {
            return Some(gix_parallel::num_threads(None).min(4));
        }
        self.thread_limit
    }

    /// Create options with the given `filters` and default settings for the checkout operation.
    pub fn new(filters: gix_filter::Pipeline) -> Self {
        Self {
            fs: Default::default(),
            validate: Default::default(),
            thread_limit: None,
            destination_is_initially_empty: false,
            overwrite_existing: false,
            keep_going: false,
            stat_options: Default::default(),
            attributes: Default::default(),
            filters,
            filter_process_delay: Default::default(),
        }
    }
}

mod changes;
mod chunk;
mod entry;
pub(crate) mod function;
pub use changes::apply_changes;
