#[cfg(feature = "pack-cache-lru-dynamic")]
mod memory {
    use std::num::NonZeroUsize;

    use clru::WeightScale;

    use crate::cache::{self, DecodeEntry, set_vec_to_slice};

    struct Entry {
        data: Vec<u8>,
        kind: gix_object::Kind,
        compressed_size: usize,
    }

    type Key = (cache::Id, u64);
    struct CustomScale;

    impl WeightScale<Key, Entry> for CustomScale {
        fn weight(&self, _key: &Key, value: &Entry) -> usize {
            value.data.len()
        }
    }

    /// An LRU cache with hash map backing and an eviction rule based on the memory usage for object data in bytes.
    pub struct MemoryCappedHashmap {
        inner: clru::CLruCache<Key, Entry, std::collections::hash_map::RandomState, CustomScale>,
        free_list: Vec<Vec<u8>>,
        debug: gix_utils::cache::Debug,
    }

    impl MemoryCappedHashmap {
        /// Return a new instance which evicts least recently used items if it uses more than `memory_cap_in_bytes`
        /// object data.
        pub fn new(memory_cap_in_bytes: usize) -> MemoryCappedHashmap {
            MemoryCappedHashmap {
                inner: clru::CLruCache::with_config(
                    clru::CLruCacheConfig::new(NonZeroUsize::new(memory_cap_in_bytes).expect("non zero"))
                        .with_scale(CustomScale),
                ),
                free_list: Vec::new(),
                debug: gix_utils::cache::Debug::new(format!("MemoryCappedHashmap({memory_cap_in_bytes}B)")),
            }
        }
    }

    impl DecodeEntry for MemoryCappedHashmap {
        fn put(
            &mut self,
            pack_id: cache::Id,
            offset: u64,
            data: &[u8],
            kind: gix_object::Kind,
            compressed_size: usize,
        ) {
            self.debug.put();
            let Some(data) = set_vec_to_slice(self.free_list.pop().unwrap_or_default(), data) else {
                return;
            };
            let res = self.inner.put_with_weight(
                (pack_id, offset),
                Entry {
                    data,
                    kind,
                    compressed_size,
                },
            );
            match res {
                Ok(Some(previous_entry)) => self.free_list.push(previous_entry.data),
                Ok(None) => {}
                Err((_key, value)) => self.free_list.push(value.data),
            }
        }

        fn get(&mut self, pack_id: cache::Id, offset: u64, out: &mut Vec<u8>) -> Option<(gix_object::Kind, usize)> {
            let res = self.inner.get(&(pack_id, offset)).and_then(|e| {
                set_vec_to_slice(out, &e.data)?;
                Some((e.kind, e.compressed_size))
            });
            if res.is_some() {
                self.debug.hit();
            } else {
                self.debug.miss();
            }
            res
        }
    }
}

#[cfg(feature = "pack-cache-lru-dynamic")]
pub use memory::MemoryCappedHashmap;

#[cfg(feature = "pack-cache-lru-static")]
mod _static {
    use crate::cache::{self, DecodeEntry, set_vec_to_slice};
    struct Entry {
        pack_id: cache::Id,
        offset: u64,
        data: Vec<u8>,
        kind: gix_object::Kind,
        compressed_size: usize,
    }

    /// A cache using a least-recently-used implementation capable of storing the `SIZE` most recent objects.
    /// The cache must be small as the search is 'naive' and the underlying data structure is a linked list.
    /// Values of 64 seem to improve performance.
    pub struct StaticLinkedList<const SIZE: usize> {
        inner: uluru::LRUCache<Entry, SIZE>,
        last_evicted: Vec<u8>,
        debug: gix_utils::cache::Debug,
        /// the amount of bytes we are currently holding, taking into account the capacities of all Vecs we keep.
        mem_used: usize,
        /// The total amount of memory we should be able to hold with all entries combined.
        mem_limit: usize,
    }

    impl<const SIZE: usize> StaticLinkedList<SIZE> {
        /// Create a new list with a memory limit of `mem_limit` in bytes. If 0, there is no memory limit.
        pub fn new(mem_limit: usize) -> Self {
            StaticLinkedList {
                inner: Default::default(),
                last_evicted: Vec::new(),
                debug: gix_utils::cache::Debug::new(format!("StaticLinkedList<{SIZE}>")),
                mem_used: 0,
                mem_limit: if mem_limit == 0 { usize::MAX } else { mem_limit },
            }
        }
    }

    impl<const SIZE: usize> Default for StaticLinkedList<SIZE> {
        fn default() -> Self {
            Self::new(96 * 1024 * 1024)
        }
    }

    impl<const SIZE: usize> DecodeEntry for StaticLinkedList<SIZE> {
        fn put(
            &mut self,
            pack_id: cache::Id,
            offset: u64,
            data: &[u8],
            kind: gix_object::Kind,
            compressed_size: usize,
        ) {
            // We cannot possibly hold this much.
            if data.len() > self.mem_limit {
                return;
            }
            // If we could hold it but are at limit, all we can do is make space.
            let mem_free = self.mem_limit - self.mem_used;
            if data.len() > mem_free {
                // prefer freeing free-lists instead of clearing our cache
                let free_list_cap = self.last_evicted.len();
                self.last_evicted = Vec::new();
                // still not enough? clear everything
                if data.len() > mem_free + free_list_cap {
                    self.inner.clear();
                    self.mem_used = 0;
                } else {
                    self.mem_used -= free_list_cap;
                }
            }
            self.debug.put();
            let mut v = std::mem::take(&mut self.last_evicted);
            self.mem_used -= v.capacity();
            if set_vec_to_slice(&mut v, data).is_none() {
                return;
            }
            self.mem_used += v.capacity();
            if let Some(previous) = self.inner.insert(Entry {
                offset,
                pack_id,
                data: v,
                kind,
                compressed_size,
            }) {
                // No need to adjust capacity as we already counted it.
                self.last_evicted = previous.data;
            }
        }

        fn get(&mut self, pack_id: cache::Id, offset: u64, out: &mut Vec<u8>) -> Option<(gix_object::Kind, usize)> {
            let res = self.inner.lookup(|e: &mut Entry| {
                if e.pack_id == pack_id && e.offset == offset {
                    set_vec_to_slice(&mut *out, &e.data)?;
                    Some((e.kind, e.compressed_size))
                } else {
                    None
                }
            });
            if res.is_some() {
                self.debug.hit();
            } else {
                self.debug.miss();
            }
            res
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn no_limit() {
            let c = StaticLinkedList::<10>::new(0);
            assert_eq!(
                c.mem_limit,
                usize::MAX,
                "zero is automatically turned into a large limit that is equivalent to unlimited"
            );
        }

        #[test]
        fn journey() {
            let mut c = StaticLinkedList::<10>::new(100);
            assert_eq!(c.mem_limit, 100);
            assert_eq!(c.mem_used, 0);

            let pack_id = cache::Id(0);
            // enough memory for normal operation
            let mut last_mem_used = 0;
            for _ in 0..10 {
                c.put(pack_id, 0, &[0], gix_object::Kind::Blob, 1);
                assert!(c.mem_used > last_mem_used);
                last_mem_used = c.mem_used;
            }
            assert_eq!(c.mem_used, 80, "there is a minimal vec size");
            assert_eq!(c.inner.len(), 10);
            assert_eq!(c.last_evicted.len(), 0);

            c.put(pack_id, 0, &(0..20).collect::<Vec<_>>(), gix_object::Kind::Blob, 1);
            assert_eq!(c.inner.len(), 10);
            assert_eq!(c.mem_used, 80 + 20);
            assert_eq!(c.last_evicted.len(), 1);

            c.put(pack_id, 0, &(0..50).collect::<Vec<_>>(), gix_object::Kind::Blob, 1);
            assert_eq!(c.inner.len(), 1, "cache clearance wasn't necessary");
            assert_eq!(c.last_evicted.len(), 0, "the free list was cleared");
            assert_eq!(c.mem_used, 50);

            c.put(pack_id, 0, &(0..101).collect::<Vec<_>>(), gix_object::Kind::Blob, 1);
            assert_eq!(
                c.inner.len(),
                1,
                "objects that won't ever fit within the memory limit are ignored"
            );
        }
    }
}

#[cfg(feature = "pack-cache-lru-static")]
pub use _static::StaticLinkedList;
