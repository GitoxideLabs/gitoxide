//! A crate for handling a git-style directory walk.
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

use bstr::{BStr, BString};
use std::borrow::Cow;

/// A directory entry, typically obtained using [`walk()`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct EntryRef<'a> {
    /// The repository-relative path at which the file or directory could be found, with unix-style component separators.
    ///
    /// To obtain the respective file, join it with the `worktree_root` passed to [`walk()`].
    /// The rationale here is that this is a compressed and normalized version compared to the paths we would otherwise get,
    /// which is preferable especially when converted to [`Entry`] due to lower memory requirements.
    ///
    /// This also means that the original path to be presented to the user needs to be computed separately, as it's also relative
    /// to their prefix, i.e. their current working directory within the repository.
    ///
    /// Note that this value can be empty if information about the `worktree_root` is provided, which is fine as
    /// [joining](std::path::Path::join) with an empty string is a no-op.
    ///
    /// Note that depending on the way entries are emitted, even refs might already contain an owned `rela_path`, for use with
    /// [into_owned()](EntryRef::into_owned())
    ///
    pub rela_path: Cow<'a, BStr>,
    /// The status of entry, most closely related to what we know from `git status`, but not the same.
    ///
    /// Note that many entries with status `Pruned` will not show up as their kind hasn't yet been determined when they were
    /// pruned very early on.
    pub status: entry::Status,
    /// Additional properties of the entry.
    pub property: Option<entry::Property>,
    /// Further specify what the entry is on disk, similar to a file mode.
    pub disk_kind: entry::Kind,
    /// The kind of entry according to the index, if tracked. *Usually* the same as `disk_kind`.
    pub index_kind: Option<entry::Kind>,
    /// Determines how the pathspec matched.
    /// Note that it can also be `Some(PathspecMatch::Excluded)` if a negative pathspec matched.
    pub pathspec_match: Option<entry::PathspecMatch>,
}

/// Just like [`EntryRef`], but with all fields owned (and thus without a lifetime to consider).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Entry {
    /// See [EntryRef::rela_path] for details.
    pub rela_path: BString,
    /// The status of entry, most closely related to what we know from `git status`, but not the same.
    pub status: entry::Status,
    /// Additional flags that further clarify properties of the entry.
    pub property: Option<entry::Property>,
    /// Further specify what the entry is on disk, similar to a file mode.
    pub disk_kind: entry::Kind,
    /// The kind of entry according to the index, if tracked. *Usually* the same as `disk_kind`.
    /// Note that even if tracked, this might be `None` which indicates this is a worktree placed
    /// within the parent repository.
    pub index_kind: Option<entry::Kind>,
    /// Indicate how the pathspec matches the entry. See more in [`EntryRef::pathspec_match`].
    pub pathspec_match: Option<entry::PathspecMatch>,
}

///
pub mod entry;

///
pub mod walk;
pub use walk::function::walk;
