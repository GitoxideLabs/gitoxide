use crate::walk::ForDeletionMode;
use crate::{Entry, EntryRef};
use std::borrow::Cow;
use std::fs::FileType;

/// A way of attaching additional information to an [Entry] .
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Property {
    /// The entry was named `.git`, matched according to the case-sensitivity rules of the repository.
    DotGit,
    /// The entry is a directory, and that directory is empty.
    EmptyDirectory,
    /// The entry is a directory, it is empty and the current working directory.
    ///
    /// The caller should pay special attention to this very special case, as it is indeed only possible to run into it
    /// while traversing the directory for deletion.
    /// Non-empty directory will never be collapsed, hence if they are working directories, they naturally become unobservable.
    EmptyDirectoryAndCWD,
    /// Always in conjunction with a directory on disk that is also known as cone-mode sparse-checkout exclude marker
    /// - i.e. a directory that is excluded, so its whole content is excluded and not checked out nor is part of the index.
    ///
    /// Note that evne if the directory is empty, it will only have this state, not `EmptyDirectory`.
    TrackedExcluded,
}

/// The kind of the entry, seated in their kinds available on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Kind {
    /// Something that is not a file, like a named pipe or character device.
    ///
    /// These can only exist in the filesystem.
    NonFile,
    /// The entry is a blob, executable or not.
    File,
    /// The entry is a symlink.
    Symlink,
    /// The entry is an ordinary directory.
    ///
    /// Note that since we don't check for bare repositories, this could in fact be a collapsed
    /// bare repository. To be sure, check it again with [`gix_discover::is_git()`] and act accordingly.
    Directory,
    /// The entry is a directory which *contains* a `.git` folder, or a submodule entry in the index.
    Repository,
}

/// The kind of entry as obtained from a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Status {
    /// The entry was removed from the walk due to its other properties, like [Property] or [PathspecMatch]
    ///
    /// Note that entries flagged as `DotGit` directory will always be considered `Pruned`, but if they are
    /// also ignored, in delete mode, they will be considered `Ignored` instead. This way, it's easier to remove them
    /// while they will not be available for any interactions in read-only mode.
    Pruned,
    /// The entry is tracked in Git.
    Tracked,
    /// The entry is ignored as per `.gitignore` files and their rules.
    ///
    /// If this is a directory, then its entire contents is ignored. Otherwise, possibly due to configuration, individual ignored files are listed.
    Ignored(gix_ignore::Kind),
    /// The entry is not tracked by git yet, it was not found in the [index](gix_index::State).
    ///
    /// If it's a directory, the entire directory contents is untracked.
    Untracked,
}

/// Describe how a pathspec pattern matched.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub enum PathspecMatch {
    /// The match happened because there wasn't any pattern, which matches all, or because there was a nil pattern or one with an empty path.
    /// Thus, this is not a match by merit.
    Always,
    /// A match happened, but the pattern excludes everything it matches, which means this entry was excluded.
    Excluded,
    /// The first part of a pathspec matches, like `dir/` that matches `dir/a`.
    Prefix,
    /// The whole pathspec matched and used a wildcard match, like `a/*` matching `a/file`.
    WildcardMatch,
    /// The entire pathspec matched, letter by letter, e.g. `a/file` matching `a/file`.
    Verbatim,
}

impl PathspecMatch {
    pub(crate) fn should_ignore(&self) -> bool {
        match self {
            PathspecMatch::Always | PathspecMatch::Excluded => true,
            PathspecMatch::Prefix | PathspecMatch::WildcardMatch | PathspecMatch::Verbatim => false,
        }
    }
}

impl From<gix_pathspec::search::MatchKind> for PathspecMatch {
    fn from(kind: gix_pathspec::search::MatchKind) -> Self {
        match kind {
            gix_pathspec::search::MatchKind::Always => Self::Always,
            gix_pathspec::search::MatchKind::Prefix => Self::Prefix,
            gix_pathspec::search::MatchKind::WildcardMatch => Self::WildcardMatch,
            gix_pathspec::search::MatchKind::Verbatim => Self::Verbatim,
        }
    }
}

impl From<gix_pathspec::search::Match<'_>> for PathspecMatch {
    fn from(m: gix_pathspec::search::Match<'_>) -> Self {
        if m.is_excluded() {
            PathspecMatch::Excluded
        } else {
            m.kind.into()
        }
    }
}

/// Conversion
impl EntryRef<'_> {
    /// Strip the lifetime to obtain a fully owned copy.
    pub fn to_owned(&self) -> Entry {
        Entry {
            rela_path: self.rela_path.clone().into_owned(),
            status: self.status,
            property: self.property,
            disk_kind: self.disk_kind,
            index_kind: self.index_kind,
            pathspec_match: self.pathspec_match,
        }
    }

    /// Turn this instance into a fully owned copy.
    pub fn into_owned(self) -> Entry {
        Entry {
            rela_path: self.rela_path.into_owned(),
            status: self.status,
            property: self.property,
            disk_kind: self.disk_kind,
            index_kind: self.index_kind,
            pathspec_match: self.pathspec_match,
        }
    }
}

/// Conversion
impl Entry {
    /// Obtain an [`EntryRef`] from this instance.
    pub fn to_ref(&self) -> EntryRef<'_> {
        EntryRef {
            rela_path: Cow::Borrowed(self.rela_path.as_ref()),
            status: self.status,
            property: self.property,
            disk_kind: self.disk_kind,
            index_kind: self.index_kind,
            pathspec_match: self.pathspec_match,
        }
    }
}

impl From<std::fs::FileType> for Kind {
    fn from(value: FileType) -> Self {
        if value.is_dir() {
            Kind::Directory
        } else if value.is_symlink() {
            Kind::Symlink
        } else if value.is_file() {
            Kind::File
        } else {
            Kind::NonFile
        }
    }
}

impl Status {
    /// Return true if this status is considered pruned. A pruned entry is typically hidden from view due to a pathspec.
    pub fn is_pruned(&self) -> bool {
        matches!(&self, Status::Pruned)
    }
    /// Return `true` if `file_type` is a directory on disk and isn't ignored, and is not a repository.
    /// This implements the default rules of `git status`, which is good for a minimal traversal through
    /// tracked and non-ignored portions of a worktree.
    /// `for_deletion` is used to determine if recursion into a directory is allowed even though it otherwise wouldn't be.
    /// If `worktree_root_is_repository` is `true`, then this status is part of the root of an iteration, and the corresponding
    /// worktree root is a repository itself. This typically happens for submodules. In this case, recursion rules are relaxed
    /// to allow traversing submodule worktrees.
    ///
    /// Use `pathspec_match` to determine if a pathspec matches in any way, affecting the decision to recurse.
    pub fn can_recurse(
        &self,
        file_type: Option<Kind>,
        pathspec_match: Option<PathspecMatch>,
        for_deletion: Option<ForDeletionMode>,
        worktree_root_is_repository: bool,
    ) -> bool {
        let is_dir_on_disk = file_type.map_or(false, |ft| {
            if worktree_root_is_repository {
                ft.is_dir()
            } else {
                ft.is_recursable_dir()
            }
        });
        if !is_dir_on_disk {
            return false;
        }
        match self {
            Status::Pruned => false,
            Status::Ignored(_) => {
                for_deletion.map_or(false, |fd| {
                    matches!(
                        fd,
                        ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories
                            | ForDeletionMode::FindRepositoriesInIgnoredDirectories
                    )
                }) || pathspec_match.map_or(false, |m| !m.should_ignore())
            }
            Status::Untracked | Status::Tracked => true,
        }
    }
}

impl Kind {
    pub(super) fn is_recursable_dir(&self) -> bool {
        matches!(self, Kind::Directory)
    }

    /// Return `true` if this is a directory on disk. Note that this is true for repositories as well.
    pub fn is_dir(&self) -> bool {
        matches!(self, Kind::Directory | Kind::Repository)
    }
}
