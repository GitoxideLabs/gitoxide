use crate::{
    config::{cache::util::ApplyLeniencyDefault, tree::Index},
    worktree,
    worktree::IndexPersistedOrInMemory,
};

/// Index access
impl crate::Repository {
    /// Open a new copy of the index file and decode it entirely.
    ///
    /// It will use the `index.threads` configuration key to learn how many threads to use.
    /// Note that it may fail if there is no index.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let index = repo.open_index()?;
    ///
    /// assert_eq!(index.entries().len(), 1);
    /// # Ok(()) }
    /// ```
    pub fn open_index(&self) -> Result<gix_index::File, worktree::open_index::Error> {
        let thread_limit = self
            .config
            .resolved
            .string(Index::THREADS)
            .map(|value| crate::config::tree::Index::THREADS.try_into_index_threads(value))
            .transpose()
            .with_lenient_default(self.config.lenient_config)?;
        let skip_hash = self
            .config
            .resolved
            .boolean(Index::SKIP_HASH)
            .map(|res| crate::config::tree::Index::SKIP_HASH.enrich_error(res))
            .transpose()
            .with_lenient_default(self.config.lenient_config)?
            .unwrap_or_default();

        let index = gix_index::File::at(
            self.index_path(),
            self.object_hash(),
            skip_hash,
            gix_index::decode::Options {
                thread_limit,
                min_extension_block_in_bytes_for_threading: 0,
                expected_checksum: None,
                alloc_limit_bytes: self.config.alloc_limit_bytes,
            },
        )?;

        Ok(index)
    }

    /// Return a shared worktree index which is updated automatically if the in-memory snapshot has become stale as the underlying file
    /// on disk has changed.
    ///
    /// ### Notes
    ///
    /// * This will fail if the file doesn't exist, like in a newly initialized repository. If that is the case, use
    ///   [index_or_empty()](Self::index_or_empty) or [try_index()](Self::try_index) instead.
    ///
    /// The index file is shared across all clones of this repository.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let index = repo.index()?;
    ///
    /// assert_eq!(index.entries().len(), 1);
    /// # Ok(()) }
    /// ```
    pub fn index(&self) -> Result<worktree::Index, worktree::open_index::Error> {
        self.try_index().and_then(|opt| match opt {
            Some(index) => Ok(index),
            None => Err(worktree::open_index::Error::IndexFile(
                gix_index::file::init::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "Could not find index file at '{index_path}' for opening.",
                        index_path = self.index_path().display()
                    ),
                )),
            )),
        })
    }

    /// Return the shared worktree index if present, or return a new empty one which has an association to the place where the index would be.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_subrepo_dir("unborn")?)?;
    /// let index = repo.index_or_empty()?;
    ///
    /// assert!(repo.try_index()?.is_none());
    /// assert!(index.entries().is_empty());
    /// # Ok(()) }
    /// ```
    pub fn index_or_empty(&self) -> Result<worktree::Index, worktree::open_index::Error> {
        Ok(self.try_index()?.unwrap_or_else(|| {
            worktree::Index::new(gix_fs::FileSnapshot::new(gix_index::File::from_state(
                gix_index::State::new(self.object_hash()),
                self.index_path(),
            )))
        }))
    }

    /// Return a shared worktree index which is updated automatically if the in-memory snapshot has become stale as the underlying file
    /// on disk has changed, or `None` if no such file exists.
    ///
    /// The index file is shared across all clones of this repository.
    pub fn try_index(&self) -> Result<Option<worktree::Index>, worktree::open_index::Error> {
        self.index.recent_snapshot(
            || self.index_path().metadata().and_then(|m| m.modified()).ok(),
            || {
                self.open_index().map(Some).or_else(|err| match err {
                    worktree::open_index::Error::IndexFile(gix_index::file::init::Error::Io(err))
                        if err.kind() == std::io::ErrorKind::NotFound =>
                    {
                        Ok(None)
                    }
                    err => Err(err),
                })
            },
        )
    }

    /// Open the persisted worktree index or generate it from the current `HEAD^{tree}` to live in-memory only.
    ///
    /// Use this method to get an index in any repository, even bare ones that don't have one naturally.
    ///
    /// ### Note
    ///
    /// * The locally stored index is not guaranteed to represent `HEAD^{tree}` if this repository is bare - bare repos
    ///   don't naturally have an index and if an index is present it must have been generated by hand.
    /// * This method will fail on unborn repositories as `HEAD` doesn't point to a reference yet, which is needed to resolve
    ///   the revspec. If that is a concern, use [`Self::index_or_load_from_head_or_empty()`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_subrepo_dir("non-bare-repo-without-index")?)?;
    /// let index = repo.index_or_load_from_head()?;
    ///
    /// assert!(repo.try_index()?.is_none());
    /// assert_eq!(index.entries().len(), 1);
    /// # Ok(()) }
    /// ```
    pub fn index_or_load_from_head(
        &self,
    ) -> Result<IndexPersistedOrInMemory, crate::repository::index_or_load_from_head::Error> {
        Ok(match self.try_index()? {
            Some(index) => IndexPersistedOrInMemory::Persisted(index),
            None => {
                let tree = self.head_commit()?.tree_id()?;
                IndexPersistedOrInMemory::InMemory(self.index_from_tree(&tree)?)
            }
        })
    }

    /// Open the persisted worktree index or generate it from the current `HEAD^{tree}` to live in-memory only,
    /// or resort to an empty index if `HEAD` is unborn.
    ///
    /// Use this method to get an index in any repository, even bare ones that don't have one naturally, or those
    /// that are in a state where `HEAD` is invalid or points to an unborn reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_subrepo_dir("unborn")?)?;
    /// let index = repo.index_or_load_from_head_or_empty()?;
    ///
    /// assert!(repo.head()?.is_unborn());
    /// assert!(index.entries().is_empty());
    /// # Ok(()) }
    /// ```
    pub fn index_or_load_from_head_or_empty(
        &self,
    ) -> Result<IndexPersistedOrInMemory, crate::repository::index_or_load_from_head_or_empty::Error> {
        Ok(match self.try_index()? {
            Some(index) => IndexPersistedOrInMemory::Persisted(index),
            None => match self.head()?.id() {
                Some(id) => {
                    let head_tree_id = id.object()?.peel_to_commit()?.tree_id()?;
                    IndexPersistedOrInMemory::InMemory(self.index_from_tree(&head_tree_id)?)
                }
                None => IndexPersistedOrInMemory::InMemory(gix_index::File::from_state(
                    gix_index::State::new(self.object_hash()),
                    self.index_path(),
                )),
            },
        })
    }

    /// Create new index-file, which would live at the correct location, in memory from the given `tree`.
    ///
    /// Note that this is an expensive operation as it requires recursively traversing the entire tree to unpack it into the index.
    pub fn index_from_tree(&self, tree: &gix_hash::oid) -> Result<gix_index::File, super::index_from_tree::Error> {
        Ok(gix_index::File::from_state(
            gix_index::State::from_tree(tree, self, self.config.protect_options()?).map_err(|err| {
                super::index_from_tree::Error::IndexFromTree {
                    id: tree.into(),
                    source: err,
                }
            })?,
            self.git_dir().join("index"),
        ))
    }
}

impl std::ops::Deref for IndexPersistedOrInMemory {
    type Target = gix_index::File;

    fn deref(&self) -> &Self::Target {
        match self {
            IndexPersistedOrInMemory::Persisted(i) => i,
            IndexPersistedOrInMemory::InMemory(i) => i,
        }
    }
}

impl IndexPersistedOrInMemory {
    /// Consume this instance and turn it into an owned index file.
    ///
    /// Note that this will cause the persisted index to be cloned, which would happen whenever the repository has a worktree.
    pub fn into_owned(self) -> gix_index::File {
        match self {
            IndexPersistedOrInMemory::Persisted(i) => gix_index::File::clone(&i),
            IndexPersistedOrInMemory::InMemory(i) => i,
        }
    }
}
