/// Configure how caches are used to speed up various git repository operations
impl crate::Repository {
    /// Sets the amount of space used at most for caching most recently accessed fully decoded objects, to `Some(bytes)`,
    /// or `None` to deactivate it entirely.
    ///
    /// Note that it is unset by default but can be enabled once there is time for performance optimization.
    /// Well-chosen cache sizes can improve performance particularly if objects are accessed multiple times in a row.
    /// The cache is configured to grow gradually.
    ///
    /// Note that a cache on application level should be considered as well as the best object access is not doing one.
    pub fn object_cache_size(&mut self, bytes: impl Into<Option<usize>>) {
        let bytes = bytes.into();
        match bytes {
            Some(0) => self.objects.unset_object_cache(),
            Some(bytes) => {
                #[cfg(feature = "object-cache-dynamic")]
                self
                    .objects
                    .set_object_cache(move || Box::new(crate::object::cache::MemoryCappedHashmap::new(bytes)));
            },
            None => self.objects.unset_object_cache(),
        }
    }

    /// Set an object cache of size `bytes` if none is set.
    ///
    /// Use this method to avoid overwriting any existing value while assuring better performance in case no value is set.
    pub fn object_cache_size_if_unset(&mut self, bytes: usize) {
        if !self.objects.has_object_cache() {
            self.object_cache_size(bytes);
        }
    }

    /// Return the amount of bytes the object cache [should be set to](Self::object_cache_size_if_unset) to perform
    /// diffs between trees who are similar to `index` in a typical source code repository.
    ///
    /// Currently, this allocates about 10MB for every 10k files in `index`, and a minimum of 4KB.
    #[cfg(feature = "index")]
    pub fn compute_object_cache_size_for_tree_diffs(&self, index: &gix_index::State) -> usize {
        let num_tracked = index.entries().len();
        let ten_mb_for_every_10k_files = (num_tracked as f32 / 10_000.0) * (10 * 1024 * 1024) as f32;
        (ten_mb_for_every_10k_files as usize).max(4 * 1024)
    }
}

/// Handling of InMemory object writing
impl crate::Repository {
    /// When writing objects, keep them in memory instead of writing them to disk.
    /// This makes any change to the object database non-persisting, while keeping the view
    /// to the object database consistent for this instance.
    pub fn with_object_memory(mut self) -> Self {
        self.objects.enable_object_memory();
        self
    }
}
