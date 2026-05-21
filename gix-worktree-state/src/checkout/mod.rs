use bstr::BString;
use gix_index::entry::stat;

/// Information about a path that failed to checkout as something else was already present.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Collision {
    /// the path that collided with something already present on disk.
    pub path: BString,
    /// The io error we encountered when checking out `path`.
    pub error_kind: std::io::ErrorKind,
}

/// A path that encountered an IO error.
#[derive(Debug)]
pub struct ErrorRecord {
    /// the path that encountered the error.
    pub path: BString,
    /// The error
    pub error: Box<dyn std::error::Error + Send + Sync + 'static>,
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
#[derive(Clone, Default)]
pub struct Options {
    /// capabilities of the file system
    pub fs: gix_fs::Capabilities,
    /// Options to configure how to validate path components.
    pub validate: gix_worktree::validate::path::component::Options,
    /// If set, don't use more than this amount of threads.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
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

/// The error returned by the [checkout()][crate::checkout()] function.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not convert path to UTF8: {}", .path)]
    IllformedUtf8 { path: BString },
    #[error("The clock was off when reading file related metadata after updating a file on disk")]
    Time(#[from] std::time::SystemTimeError),
    #[error("IO error while writing blob or reading file metadata or changing filetype")]
    Io(#[from] std::io::Error),
    #[error("object for checkout at {} could not be retrieved from object database", .path.display())]
    Find {
        #[source]
        err: gix_object::find::existing_object::Error,
        path: std::path::PathBuf,
    },
    #[error(transparent)]
    Filter(#[from] gix_filter::pipeline::convert::to_worktree::Error),
    #[error(transparent)]
    FilterListDelayed(#[from] gix_filter::driver::delayed::list::Error),
    #[error(transparent)]
    FilterFetchDelayed(#[from] gix_filter::driver::delayed::fetch::Error),
    #[error("The entry at path '{rela_path}' was listed as delayed by the filter process, but we never passed it")]
    FilterPathUnknown { rela_path: BString },
    #[error("The following paths were delayed and apparently forgotten to be processed by the filter driver: ")]
    FilterPathsUnprocessed { rela_paths: Vec<BString> },
}

mod chunk;
mod entry;
pub(crate) mod function;
