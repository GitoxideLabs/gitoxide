use std::path::PathBuf;

#[cfg(feature = "worktree-archive")]
pub use gix_archive as archive;
#[cfg(feature = "excludes")]
pub use gix_worktree::*;
#[cfg(feature = "worktree-mutation")]
pub use gix_worktree_state as state;
#[cfg(feature = "worktree-stream")]
pub use gix_worktree_stream as stream;

use crate::{
    Repository,
    bstr::{BStr, BString},
};

#[cfg(feature = "index")]
pub(crate) type IndexStorage = gix_features::threading::OwnShared<gix_fs::SharedFileSnapshotMut<gix_index::File>>;
/// A lazily loaded and auto-updated worktree index.
#[cfg(feature = "index")]
pub type Index = gix_fs::SharedFileSnapshot<gix_index::File>;

/// A type to represent an index which either was loaded from disk as it was persisted there, or created on the fly in memory.
#[cfg(feature = "index")]
#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum IndexPersistedOrInMemory {
    /// The index as loaded from disk, and shared across clones of the owning `Repository`.
    Persisted(Index),
    /// A temporary index as created from the `HEAD^{tree}`, with the file path set to the place where it would be stored naturally.
    ///
    /// Note that unless saved explicitly, it will not persist.
    InMemory(gix_index::File),
}

#[cfg(feature = "index")]
impl From<Index> for IndexPersistedOrInMemory {
    fn from(value: Index) -> Self {
        IndexPersistedOrInMemory::Persisted(value)
    }
}

#[cfg(feature = "index")]
impl From<gix_index::File> for IndexPersistedOrInMemory {
    fn from(value: gix_index::File) -> Self {
        IndexPersistedOrInMemory::InMemory(value)
    }
}

/// A stand-in to a worktree as result of a worktree iteration.
///
/// It provides access to typical worktree state, but may not actually point to a valid checkout as the latter has been moved or
/// deleted.
#[derive(Debug, Clone)]
pub struct Proxy<'repo> {
    pub(crate) parent: &'repo Repository,
    pub(crate) git_dir: PathBuf,
}

/// Access
impl<'repo> crate::Worktree<'repo> {
    /// Read the location of the checkout, the base of the work tree
    pub fn base(&self) -> &'repo std::path::Path {
        self.path
    }

    /// Return true if this worktree is the main worktree associated with a non-bare git repository.
    ///
    /// It cannot be removed.
    pub fn is_main(&self) -> bool {
        self.id().is_none()
    }

    /// Return true if this worktree cannot be pruned, moved or deleted, which is useful if it is located on an external storage device.
    ///
    /// Always false for the main worktree.
    pub fn is_locked(&self) -> bool {
        Proxy::new(self.parent, self.parent.git_dir()).is_locked()
    }

    /// Provide a reason for the locking of this worktree, if it is locked at all.
    ///
    /// Note that we squelch errors in case the file cannot be read in which case the
    /// reason is an empty string.
    pub fn lock_reason(&self) -> Option<BString> {
        Proxy::new(self.parent, self.parent.git_dir()).lock_reason()
    }

    /// Return the ID of the repository worktree, if it is a linked worktree, or `None` if it's a linked worktree.
    pub fn id(&self) -> Option<&BStr> {
        id(self.parent.git_dir(), self.parent.common_dir.is_some())
    }

    /// Returns true if the `.git` file or directory exists within the worktree.
    ///
    /// This is an indicator for the worktree to be checked out particularly if the parent repository is a submodule.
    pub fn dot_git_exists(&self) -> bool {
        self.path.join(gix_discover::DOT_GIT_DIR).exists()
    }
}

pub(crate) fn id(git_dir: &std::path::Path, has_common_dir: bool) -> Option<&BStr> {
    if !has_common_dir {
        return None;
    }
    let candidate = gix_path::os_str_into_bstr(git_dir.file_name().expect("at least one directory level"))
        .expect("no illformed UTF-8");
    let maybe_worktrees = git_dir.parent()?;
    (maybe_worktrees.file_name()?.to_str()? == "worktrees").then_some(candidate)
}

///
pub mod proxy;

///
#[cfg(feature = "index")]
pub mod open_index {
    /// The error returned by [`Worktree::open_index()`][crate::Worktree::open_index()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ConfigIndexThreads(#[from] crate::config::key::GenericErrorWithValue),
        #[error(transparent)]
        ConfigSkipHash(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        IndexFile(#[from] gix_index::file::init::Error),
        #[error(transparent)]
        IndexCorrupt(#[from] gix_index::file::verify::Error),
    }

    impl crate::Worktree<'_> {
        /// A shortcut to [`crate::Repository::open_index()`].
        pub fn open_index(&self) -> Result<gix_index::File, Error> {
            self.parent.open_index()
        }

        /// A shortcut to [`crate::Repository::index()`].
        pub fn index(&self) -> Result<crate::worktree::Index, Error> {
            self.parent.index()
        }
    }
}

///
#[cfg(feature = "excludes")]
pub mod excludes {
    use crate::AttributeStack;

    /// The error returned by [`Worktree::excludes()`][crate::Worktree::excludes()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
        #[error(transparent)]
        CreateCache(#[from] crate::config::exclude_stack::Error),
    }

    impl crate::Worktree<'_> {
        /// Configure a file-system cache checking if files below the repository are excluded.
        ///
        /// This takes into consideration all the usual repository configuration, namely:
        ///
        /// * `$XDG_CONFIG_HOME/…/ignore` if `core.excludesFile` is *not* set, otherwise use the configured file.
        /// * `$GIT_DIR/info/exclude` if present.
        ///
        /// When only excludes are desired, this is the most efficient way to obtain them. Otherwise use
        /// [`Worktree::attributes()`][crate::Worktree::attributes()] for accessing both attributes and excludes.
        pub fn excludes(&self, overrides: Option<gix_ignore::Search>) -> Result<AttributeStack<'_>, Error> {
            let index = self.index()?;
            Ok(self.parent.excludes(
                &index,
                overrides,
                gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            )?)
        }
    }
}

///
#[cfg(feature = "attributes")]
pub mod attributes {
    use crate::{AttributeStack, Worktree};

    /// The error returned by [`Worktree::attributes()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
        #[error(transparent)]
        CreateCache(#[from] crate::repository::attributes::Error),
    }

    impl<'repo> Worktree<'repo> {
        /// Configure a file-system cache checking if files below the repository are excluded or for querying their attributes.
        ///
        /// This takes into consideration all the usual repository configuration, namely:
        ///
        /// * `$XDG_CONFIG_HOME/…/ignore|attributes` if `core.excludesFile|attributesFile` is *not* set, otherwise use the configured file.
        /// * `$GIT_DIR/info/exclude|attributes` if present.
        pub fn attributes(&self, overrides: Option<gix_ignore::Search>) -> Result<AttributeStack<'repo>, Error> {
            let index = self.index()?;
            Ok(self.parent.attributes(
                &index,
                gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
                overrides,
            )?)
        }

        /// Like [attributes()][Self::attributes()], but without access to exclude/ignore information.
        pub fn attributes_only(&self) -> Result<AttributeStack<'repo>, Error> {
            let index = self.index()?;
            self.parent
                .attributes_only(
                    &index,
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                )
                .map_err(|err| Error::CreateCache(err.into()))
        }
    }
}

///
#[cfg(feature = "attributes")]
pub mod pathspec {
    use crate::{
        Worktree,
        bstr::BStr,
        config::{cache::util::ApplyLeniencyDefaultValue, tree::gitoxide},
    };

    /// The error returned by [`Worktree::pathspec()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Init(#[from] crate::pathspec::init::Error),
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
    }

    impl<'repo> Worktree<'repo> {
        /// Configure pathspecs `patterns` to be matched against, with pathspec attributes read from the worktree and then from the index
        /// if needed.
        ///
        /// Note that the `empty_patterns_match_prefix` flag of the [parent method](crate::Repository::pathspec()) defaults to `true`.
        ///
        /// ### Deviation
        ///
        /// Pathspec attributes match case-insensitively by default if the underlying filesystem is configured that way.
        pub fn pathspec(
            &self,
            patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        ) -> Result<crate::Pathspec<'repo>, Error> {
            let index = self.index()?;
            let inherit_ignore_case = self
                .parent
                .config
                .resolved
                .boolean("gitoxide.pathspec.inheritIgnoreCase")
                .map(|res| {
                    gitoxide::Pathspec::INHERIT_IGNORE_CASE
                        .enrich_error(res)
                        .with_lenient_default_value(
                            self.parent.config.lenient_config,
                            gitoxide::Pathspec::INHERIT_IGNORE_CASE_DEFAULT,
                        )
                })
                .transpose()
                .map_err(|err| Error::Init(crate::pathspec::init::Error::Defaults(err.into())))?
                .unwrap_or(gitoxide::Pathspec::INHERIT_IGNORE_CASE_DEFAULT);
            Ok(self.parent.pathspec(
                true, /* empty patterns match prefix */
                patterns,
                inherit_ignore_case,
                &index,
                gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            )?)
        }
    }
}
