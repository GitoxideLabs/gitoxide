use std::{collections::BTreeSet, path::PathBuf, sync::atomic::AtomicBool};

use bstr::{BStr, BString};

use crate::{entry, EntryRef};

/// A type returned by the [`Delegate::emit()`] as passed to [`walk()`](function::walk()).
///
/// Use [`std::ops::ControlFlow::Continue`] to continue the traversal as normal.
/// Use [`std::ops::ControlFlow::Break`] to exit the traversal.
pub type Action = std::ops::ControlFlow<()>;

/// Ready-made delegate implementations.
pub mod delegate {
    use crate::{entry, walk, walk::Action, Entry, EntryRef};

    type Entries = Vec<(Entry, Option<entry::Status>)>;

    /// A [`Delegate`](walk::Delegate) implementation that collects all `entries` along with their directory status, if present.
    ///
    /// Note that this allocates for each entry.
    #[derive(Default)]
    pub struct Collect {
        /// All collected entries, in any order.
        pub unorded_entries: Entries,
    }

    impl Collect {
        /// Return the list of entries that were emitted, sorted ascending by their repository-relative tree path.
        pub fn into_entries_by_path(mut self) -> Entries {
            self.unorded_entries.sort_by(|a, b| a.0.rela_path.cmp(&b.0.rela_path));
            self.unorded_entries
        }
    }

    impl walk::Delegate for Collect {
        fn emit(&mut self, entry: EntryRef<'_>, dir_status: Option<entry::Status>) -> Action {
            self.unorded_entries.push((entry.to_owned(), dir_status));
            std::ops::ControlFlow::Continue(())
        }
    }
}

/// A way for the caller to control the traversal based on provided data.
pub trait Delegate {
    /// Called for each observed `entry` *inside* a directory, or the directory itself if the traversal is configured
    /// to simplify the result (i.e. if every file in a directory is ignored, emit the containing directory instead
    /// of each file), or if the root of the traversal passes through a directory that can't be traversed.
    ///
    /// It will also be called if the `root` in [`walk()`](crate::walk()) itself is matching a particular status,
    /// even if it is a file.
    ///
    /// Note that tracked entries will only be emitted if [`Options::emit_tracked`] is `true`.
    /// Further, not all pruned entries will be observable as they might be pruned so early that the kind of
    /// item isn't yet known. Pruned entries are also only emitted if [`Options::emit_pruned`] is `true`.
    ///
    /// `collapsed_directory_status` is `Some(dir_status)` if this entry was part of a directory with the given
    /// `dir_status` that wasn't the same as the one of `entry` and if [Options::emit_collapsed] was
    /// [CollapsedEntriesEmissionMode::OnStatusMismatch]. It will also be `Some(dir_status)` if that option
    /// was [CollapsedEntriesEmissionMode::All].
    fn emit(&mut self, entry: EntryRef<'_>, collapsed_directory_status: Option<entry::Status>) -> Action;

    /// Return `true` if the given entry can be recursed into. Will only be called if the entry is a physical directory.
    /// The base implementation will act like Git does by default in `git status` or `git clean`.
    ///
    /// Use `for_deletion` to specify if the seen entries should ultimately be deleted, which may affect the decision
    /// of whether to resource or not.
    ///
    /// If `worktree_root_is_repository` is `true`, then this status is part of the root of an iteration, and the corresponding
    /// worktree root is a repository itself. This typically happens for submodules. In this case, recursion rules are relaxed
    /// to allow traversing submodule worktrees.
    ///
    /// Note that this method will see all directories, even though not all of them may end up being [emitted](Self::emit()).
    /// If this method returns `false`, the `entry` will always be emitted.
    fn can_recurse(
        &mut self,
        entry: EntryRef<'_>,
        for_deletion: Option<ForDeletionMode>,
        worktree_root_is_repository: bool,
    ) -> bool {
        entry.status.can_recurse(
            entry.disk_kind,
            entry.pathspec_match,
            for_deletion,
            worktree_root_is_repository,
        )
    }
}

/// The way entries are emitted using the [Delegate].
///
/// The choice here controls if entries are emitted immediately, or have to be held back.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EmissionMode {
    /// Emit each entry as it matches exactly, without doing any kind of simplification.
    ///
    /// Emissions in this mode are happening as they occur, without any buffering or ordering.
    #[default]
    Matching,
    /// Emit only a containing directory if all of its entries are of the same type.
    ///
    /// Note that doing so is more expensive as it requires us to keep track of all entries in the directory structure
    /// until it's clear what to finally emit.
    CollapseDirectory,
}

/// The way entries that are contained in collapsed directories are emitted using the [Delegate].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CollapsedEntriesEmissionMode {
    /// Emit only entries if their status does not match the one of the parent directory that is
    /// going to be collapsed.
    ///
    /// E.g. if a directory is determined to be untracked, and the entries in question are ignored,
    /// they will be emitted.
    ///
    /// Entries that have the same status will essentially be 'merged' into the collapsing directory
    /// and won't be observable anymore.
    #[default]
    OnStatusMismatch,
    /// Emit all entries inside of a collapsed directory to make them observable.
    All,
}

/// When the walk is for deletion, assure that we don't collapse directories that have precious files in
/// them, and otherwise assure that no entries are observable that shouldn't be deleted.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ForDeletionMode {
    /// We will stop traversing into ignored directories which may save a lot of time, but also may include nested repositories
    /// which might end up being deleted.
    #[default]
    IgnoredDirectoriesCanHideNestedRepositories,
    /// Instead of skipping over ignored directories entirely, we will dive in and find ignored non-bare repositories
    /// so these are emitted separately and prevent collapsing. These are assumed to be a directory with `.git` inside.
    /// Only relevant when ignored entries are emitted.
    FindNonBareRepositoriesInIgnoredDirectories,
    /// This is a more expensive form of the above variant as it finds all repositories, bare or non-bare.
    FindRepositoriesInIgnoredDirectories,
}

/// Options for use in [`walk()`](function::walk()) function.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Options<'a> {
    /// If `true`, the filesystem will store paths as decomposed unicode, i.e. `ä` becomes `"a\u{308}"`, which means that
    /// we have to turn these forms back from decomposed to precomposed unicode before storing it in the index or generally
    /// using it. This also applies to input received from the command-line, so callers may have to be aware of this and
    /// perform conversions accordingly.
    /// If `false`, no conversions will be performed.
    pub precompose_unicode: bool,
    /// If true, the filesystem ignores the case of input, which makes `A` the same file as `a`.
    /// This is also called case-folding.
    /// Note that [pathspecs](Context::pathspec) must also be using the same defaults, which makes them match case-insensitive
    /// automatically.
    pub ignore_case: bool,
    /// If `true`, we will stop figuring out if any directory that is a candidate for recursion is also a nested repository,
    /// which saves time but leads to recurse into it. If `false`, nested repositories will not be traversed.
    pub recurse_repositories: bool,
    /// If `true`, entries that are pruned and whose [Kind](crate::entry::Kind) is known will be emitted.
    pub emit_pruned: bool,
    /// If `Some(mode)`, entries that are ignored will be emitted according to the given `mode`.
    /// If `None`, ignored entries will not be emitted at all.
    pub emit_ignored: Option<EmissionMode>,
    /// When the walk is for deletion, this must be `Some(_)` to assure we don't collapse directories that have precious files in
    /// them, and otherwise assure that no entries are observable that shouldn't be deleted.
    /// If `None`, precious files are treated like expendable files, which is usually what you want when displaying them
    /// for addition to the repository, and the collapse of folders can be more generous in relation to ignored files.
    pub for_deletion: Option<ForDeletionMode>,
    /// If `true`, we will not only find non-bare repositories in untracked directories, but also bare ones.
    ///
    /// Note that this is very costly, but without it, bare repositories will appear like untracked directories when collapsed,
    /// and they will be recursed into.
    pub classify_untracked_bare_repositories: bool,
    /// If `true`, we will also emit entries for tracked items. Otherwise these will remain 'hidden', even if a pathspec directly
    /// refers to it.
    pub emit_tracked: bool,
    /// Controls the way untracked files are emitted. By default, this is happening immediately and without any simplification.
    pub emit_untracked: EmissionMode,
    /// If `true`, emit empty directories as well. Note that a directory also counts as empty if it has any amount or depth of nested
    /// subdirectories, as long as none of them includes a file.
    /// Thus, this makes leaf-level empty directories visible, as those don't have any content.
    pub emit_empty_directories: bool,
    /// If `None`, no entries inside of collapsed directories are emitted. Otherwise, act as specified by `Some(mode)`.
    pub emit_collapsed: Option<CollapsedEntriesEmissionMode>,
    /// This is a `libgit2` compatibility flag, and if enabled, symlinks that point to directories will be considered a directory
    /// when checking for exclusion.
    ///
    /// This is relevant if `src2` points to `src`, and is excluded with `src2/`. If `false`, `src2` will not be excluded,
    /// if `true` it will be excluded as the symlink is considered a directory.
    ///
    /// In other words, for Git compatibility this flag should be `false`, the default, for `git2` compatibility it should be `true`.
    pub symlinks_to_directories_are_ignored_like_directories: bool,
    /// A set of all git worktree checkouts that are located within the main worktree directory.
    ///
    /// They will automatically be detected as 'tracked', but without providing index information (as there is no actual index entry).
    /// Note that the unicode composition must match the `precompose_unicode` field so that paths will match verbatim.
    pub worktree_relative_worktree_dirs: Option<&'a BTreeSet<BString>>,
}

/// All information that is required to perform a dirwalk, and classify paths properly.
pub struct Context<'a> {
    /// If not `None`, it will be checked before entering any directory to trigger early interruption.
    ///
    /// If this flag is `true` at any point in the iteration, it will abort with an error.
    pub should_interrupt: Option<&'a AtomicBool>,
    /// The `git_dir` of the parent repository, after a call to [`gix_path::realpath()`].
    ///
    /// It's used to help us differentiate our own `.git` directory from nested unrelated repositories,
    /// which is needed if `core.worktree` is used to nest the `.git` directory deeper within.
    pub git_dir_realpath: &'a std::path::Path,
    /// The current working directory as returned by `gix_fs::current_dir()` to assure it respects `core.precomposeUnicode`.
    /// It's used to produce the realpath of the git-dir of a repository candidate to assure it's not our own repository.
    ///
    /// It is also used to assure that when the walk is for deletion, that the current working dir will not be collapsed.
    pub current_dir: &'a std::path::Path,
    /// The index to quickly understand if a file or directory is tracked or not.
    ///
    /// ### Important
    ///
    /// The index must have been validated so that each entry that is considered up-to-date will have the [gix_index::entry::Flags::UPTODATE] flag
    /// set. Otherwise the index entry is not considered and a disk-access may occur which is costly.
    pub index: &'a gix_index::State,
    /// A utility to lookup index entries faster, and deal with ignore-case handling.
    ///
    /// Must be set if `ignore_case` is `true`, or else some entries won't be found if their case is different.
    ///
    /// ### Deviation
    ///
    /// Git uses a name-based hash (for looking up entries, not directories) even when operating
    /// in case-sensitive mode. It does, however, skip the directory hash creation (for looking
    /// up directories) unless `core.ignoreCase` is enabled.
    ///
    /// We only use the hashmap when available and when [`ignore_case`](Options::ignore_case) is enabled in the options.
    pub ignore_case_index_lookup: Option<&'a gix_index::AccelerateLookup<'a>>,
    /// A pathspec to use as filter - we only traverse into directories if it matches.
    /// Note that the `ignore_case` setting it uses should match our [Options::ignore_case].
    /// If no such filtering is desired, pass an empty `pathspec` which will match everything.
    pub pathspec: &'a mut gix_pathspec::Search,
    /// The `attributes` callback for use in [gix_pathspec::Search::pattern_matching_relative_path()], which happens when
    /// pathspecs use attributes for filtering.
    /// If `pathspec` isn't empty, this function may be called if pathspecs perform attribute lookups.
    pub pathspec_attributes: &'a mut dyn FnMut(
        &BStr,
        gix_pathspec::attributes::glob::pattern::Case,
        bool,
        &mut gix_pathspec::attributes::search::Outcome,
    ) -> bool,
    /// A way to query the `.gitignore` files to see if a directory or file is ignored.
    /// Set to `None` to not perform any work on checking for ignored, which turns previously ignored files into untracked ones, a useful
    /// operation when trying to add ignored files to a repository.
    pub excludes: Option<&'a mut gix_worktree::Stack>,
    /// Access to the object database for use with `excludes` - it's possible to access `.gitignore` files in the index if configured.
    pub objects: &'a dyn gix_object::Find,
    /// If not `None`, override the traversal root that is computed and use this one instead.
    ///
    /// This can be useful if the traversal root may be a file, in which case the traversal will
    /// still be returning possibly matching root entries.
    ///
    /// ### Panics
    ///
    /// If the `traversal_root` is not in the `worktree_root` passed to [walk()](crate::walk()).
    pub explicit_traversal_root: Option<&'a std::path::Path>,
}

/// Additional information collected as outcome of [`walk()`](function::walk()).
#[derive(Default, Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Outcome {
    /// The amount of calls to read the directory contents.
    pub read_dir_calls: u32,
    /// The amount of returned entries provided to the callback. This number can be lower than `seen_entries`.
    pub returned_entries: usize,
    /// The amount of entries, prior to pathspecs filtering them out or otherwise excluding them.
    pub seen_entries: u32,
}

/// The error returned by [`walk()`](function::walk()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Interrupted")]
    Interrupted,
    #[error("Worktree root at '{}' is not a directory", root.display())]
    WorktreeRootIsFile { root: PathBuf },
    #[error("Traversal root '{}' contains relative path components and could not be normalized", root.display())]
    NormalizeRoot { root: PathBuf },
    #[error("A symlink was found at component {component_index} of traversal root '{}' as seen from worktree root '{}'", root.display(), worktree_root.display())]
    SymlinkInRoot {
        root: PathBuf,
        worktree_root: PathBuf,
        /// This index starts at 0, with 0 being the first component.
        component_index: usize,
    },
    #[error("Failed to update the excludes stack to see if a path is excluded")]
    ExcludesAccess(std::io::Error),
    #[error("Failed to read the directory at '{}'", path.display())]
    ReadDir { path: PathBuf, source: std::io::Error },
    #[error("Could not obtain directory entry in root of '{}'", parent_directory.display())]
    DirEntry {
        parent_directory: PathBuf,
        source: std::io::Error,
    },
    #[error("Could not obtain filetype of directory entry '{}'", path.display())]
    DirEntryFileType { path: PathBuf, source: std::io::Error },
    #[error("Could not obtain symlink metadata on '{}'", path.display())]
    SymlinkMetadata { path: PathBuf, source: std::io::Error },
}

mod classify;
pub(crate) mod function;
mod readdir;
