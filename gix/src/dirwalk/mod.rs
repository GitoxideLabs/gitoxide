use gix_dir::walk::{CollapsedEntriesEmissionMode, EmissionMode, ForDeletionMode};

use crate::{config, AttributeStack, Pathspec};
use std::path::PathBuf;

mod options;

///
pub mod iter;

/// An iterator for entries in a directory walk.
///
/// ### Parallel Operation
///
/// Note that without the `parallel` feature, the iterator becomes 'serial', which means that all entries will be traversed
/// in advance and it cannot be interrupted unless the interrupt flag is set from another thread.
///
/// It's a crutch that is just there to make single-threaded applications possible at all, as it's not really an iterator
/// anymore. If this matters, better run [Repository::dirwalk()](crate::Repository::dirwalk) by hand as it provides all
/// control one would need, just not as an iterator.
///
/// Also, even with `parallel` set, the first call to `next()` will block until there is an item available, without a chance
/// to interrupt unless the interrupt flag is set from another thread.
pub struct Iter {
    #[cfg(feature = "parallel")]
    #[allow(clippy::type_complexity)]
    rx_and_join: Option<(
        std::sync::mpsc::Receiver<iter::Item>,
        std::thread::JoinHandle<Result<iter::Outcome, Error>>,
    )>,
    #[cfg(feature = "parallel")]
    should_interrupt: crate::util::OwnedOrStaticAtomicBool,
    /// Without parallelization, the iterator has to buffer all changes in advance.
    #[cfg(not(feature = "parallel"))]
    items: std::vec::IntoIter<iter::Item>,
    /// The outcome of the operation, only available once the operation has ended.
    out: Option<iter::Outcome>,
}

/// The error returned by [dirwalk()](crate::Repository::dirwalk()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Walk(#[from] gix_dir::walk::Error),
    #[error("A working tree is required to perform a directory walk")]
    MissingWorkDir,
    #[error(transparent)]
    Excludes(#[from] config::exclude_stack::Error),
    #[error(transparent)]
    Pathspec(#[from] crate::pathspec::init::Error),
    #[error(transparent)]
    Prefix(#[from] gix_path::realpath::Error),
    #[error(transparent)]
    FilesystemOptions(#[from] config::boolean::Error),
    #[error("Could not list worktrees to assure they are no candidates for deletion")]
    ListWorktrees(#[from] std::io::Error),
}

/// The outcome of the [dirwalk()](crate::Repository::dirwalk).
pub struct Outcome<'repo> {
    /// The excludes stack used for the dirwalk, for access of `.gitignore` information.
    pub excludes: AttributeStack<'repo>,
    /// The pathspecs used to guide the operation,
    pub pathspec: Pathspec<'repo>,
    /// The root actually being used for the traversal, and useful to transform the paths returned for the user.
    /// It's always within the [`work-dir`](crate::Repository::workdir).
    pub traversal_root: PathBuf,
    /// The actual result of the dirwalk.
    pub dirwalk: gix_dir::walk::Outcome,
}

/// Options for use in the [`Repository::dirwalk()`](crate::Repository::dirwalk()) function.
///
/// Note that all values start out disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Options {
    precompose_unicode: bool,
    ignore_case: bool,

    recurse_repositories: bool,
    emit_pruned: bool,
    emit_ignored: Option<EmissionMode>,
    for_deletion: Option<ForDeletionMode>,
    emit_tracked: bool,
    emit_untracked: EmissionMode,
    emit_empty_directories: bool,
    classify_untracked_bare_repositories: bool,
    emit_collapsed: Option<CollapsedEntriesEmissionMode>,
    symlinks_to_directories_are_ignored_like_directories: bool,
    pub(crate) empty_patterns_match_prefix: bool,
}
