use std::cell::RefCell;

impl crate::Repository {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_refs_and_objects(
        refs: crate::RefStore,
        mut objects: crate::OdbHandle,
        work_tree: Option<std::path::PathBuf>,
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
}

#[cfg_attr(not(feature = "max-performance-safe"), allow(unused_variables, unused_mut))]
pub(crate) fn setup_objects(objects: &mut crate::OdbHandle, config: &crate::config::Cache) {
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
