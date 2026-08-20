use std::cell::RefCell;

impl crate::Repository {
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn from_refs_and_objects(
        refs: crate::RefStore,
        mut objects: crate::OdbHandle,
        work_tree: Option<std::path::PathBuf>,
        index_path: std::path::PathBuf,
        common_dir: Option<std::path::PathBuf>,
        config: crate::config::Cache,
        linked_worktree_options: crate::open::Options,
        #[cfg(feature = "index")] index: crate::worktree::IndexStorage,
        shallow_commits: crate::shallow::CommitsStorage,
        #[cfg(feature = "attributes")] modules: crate::submodule::ModulesFileStorage,
    ) -> Self {
        setup_objects(&mut objects, &config);
        crate::Repository {
            bufs: Some(RefCell::new(Vec::with_capacity(4))),
            work_tree,
            index_path,
            common_dir,
            objects,
            refs,
            config,
            options: linked_worktree_options,
            #[cfg(feature = "index")]
            index,
            shallow_commits,
            #[cfg(feature = "attributes")]
            modules,
        }
    }

    /// Convert this instance into a [`ThreadSafeRepository`][crate::ThreadSafeRepository] by dropping all thread-local data.
    pub fn into_sync(self) -> crate::ThreadSafeRepository {
        self.into()
    }

    /// Reopen this repository in place using the stored open options.
    /// Use this to forcefully refresh Git configuration, drop caches, and release system resources
    /// for opened object database resources (if this is the last instance to a repository).
    ///
    /// This discards in-memory-only configuration edits and any other transient repository state that is recreated
    /// during opening.
    ///
    /// # Notes on relative paths
    ///
    /// When the [`git_dir`](Self::git_dir()) is relative and the current working dir changed,
    /// then a reload will be performed on the joined path of both to make it succeed, which makes
    /// the reloaded repository git-dir absolute.
    pub fn reload(&mut self) -> Result<&mut Self, crate::open::Error> {
        let mut git_dir = self.git_dir().to_owned();
        let options = self.options.clone().open_path_as_is(true);
        if git_dir.is_relative() {
            if let Some((prev_cwd, cwd)) = options.current_dir.as_ref().zip(std::env::current_dir().ok()) {
                if *prev_cwd != cwd {
                    git_dir = prev_cwd.join(git_dir);
                }
            }
        }
        *self = crate::ThreadSafeRepository::open_opts(git_dir, options)?.to_thread_local();
        Ok(self)
    }
}

pub(crate) fn setup_objects(objects: &mut crate::OdbHandle, config: &crate::config::Cache) {
    objects.refresh = config.object_refresh_mode;
    #[cfg(feature = "max-performance-safe")]
    {
        match config.pack_cache_bytes {
            None => match config.static_pack_cache_limit_bytes {
                None => objects.set_pack_cache(|| Box::<gix_pack::cache::lru::StaticLinkedList<64>>::default()),
                Some(limit) => {
                    objects.set_pack_cache(move || Box::new(gix_pack::cache::lru::StaticLinkedList::<64>::new(limit)));
                }
            },
            Some(0) => objects.unset_pack_cache(),
            Some(bytes) => objects.set_pack_cache(move || -> Box<gix_odb::cache::PackCache> {
                Box::new(gix_pack::cache::lru::MemoryCappedHashmap::new(bytes))
            }),
        }
        if config.object_cache_bytes == 0 {
            objects.unset_object_cache();
        } else {
            let bytes = config.object_cache_bytes;
            objects.set_object_cache(move || Box::new(gix_pack::cache::object::MemoryCappedHashmap::new(bytes)));
        }
    }
}
