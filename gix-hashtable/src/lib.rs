//! Customized `HashMap` and Hasher implementation optimized for using `ObjectId`s as keys.
//!
//! The crate mirrors `std::collections` in layout for familiarity.
//!
//! ## Examples
//!
//! ```
//! use gix_hash::{Kind, ObjectId};
//!
//! let id = ObjectId::empty_blob(Kind::Sha1);
//!
//! let mut map = gix_hashtable::HashMap::default();
//! map.insert(id, "empty blob");
//! assert_eq!(map.get(&id), Some(&"empty blob"));
//!
//! let shared = gix_hashtable::sync::ObjectIdMap::default();
//! assert_eq!(shared.insert(id, 1), None);
//! assert_eq!(shared.insert(id, 2), Some(1));
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use gix_hash::ObjectId;
pub use hashbrown::{Equivalent, hash_map, hash_set, hash_table};

/// thread-safe types
pub mod sync {
    /// A map for associating data with object ids in a thread-safe fashion. It should scale well up to 256 threads.
    pub struct ObjectIdMap<V> {
        /// Sharing is done by the first byte of the incoming object id.
        shards: [parking_lot::Mutex<super::HashMap<gix_hash::ObjectId, V>>; 256],
    }

    impl<V> Default for ObjectIdMap<V> {
        fn default() -> Self {
            Self {
                shards: std::array::from_fn(|_| parking_lot::Mutex::new(super::HashMap::default())),
            }
        }
    }

    /// access and modifications - we only implement what's used within the `gix-*` ecosystem.
    impl<V> ObjectIdMap<V> {
        /// Insert `value` at `key` and return `None` if it's the first value at that location, or `Some(previous-value)`
        /// if `key` was already set.
        pub fn insert(&self, key: gix_hash::ObjectId, value: V) -> Option<V> {
            self.shards[key.as_slice()[0] as usize].lock().insert(key, value)
        }
    }
}

///
pub mod hash {
    /// A Hasher for usage with `HashMap` keys that are already robust hashes (like an `ObjectId`).
    /// The first `8` bytes of the hash are used as the `HashMap` hash
    #[derive(Default, Clone, Copy)]
    pub struct Hasher(u64);

    macro_rules! panic_other_writers {
        ($func:ident, $type:ty) => {
            #[cold]
            fn $func(&mut self, _i: $type) {
                panic!("This hasher only supports manually verified `Hash` implementations")
            }
        };
    }

    impl std::hash::Hasher for Hasher {
        fn finish(&self) -> u64 {
            self.0
        }

        #[inline(always)]
        fn write(&mut self, bytes: &[u8]) {
            self.0 = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
        }

        // Panic if someone tries to use this with a different function,
        // only manually verified types should be used with this hasher
        panic_other_writers!(write_u8, u8);
        panic_other_writers!(write_u16, u16);
        panic_other_writers!(write_u32, u32);
        panic_other_writers!(write_u64, u64);
        panic_other_writers!(write_u128, u128);
        panic_other_writers!(write_usize, usize);
        panic_other_writers!(write_i8, i8);
        panic_other_writers!(write_i16, i16);
        panic_other_writers!(write_i32, i32);
        panic_other_writers!(write_i64, i64);
        panic_other_writers!(write_i128, i128);
        panic_other_writers!(write_isize, isize);
    }

    /// A Hasher for usage with `HashMap` keys that are already robust hashes (like an `ObjectId`).
    /// The first `8` bytes of the hash are used as the `HashMap` hash
    #[derive(Default, Clone, Copy)]
    pub struct Builder;
    impl std::hash::BuildHasher for Builder {
        type Hasher = Hasher;

        fn build_hasher(&self) -> Self::Hasher {
            Hasher::default()
        }
    }
}

/// A `HashMap` for usage with keys that are already robust hashes (like an `ObjectId`).
/// The first `8` bytes of the hash are used as the `HashMap` hash
pub type HashMap<K, V> = hashbrown::HashMap<K, V, hash::Builder>;
/// A `HashSet` for usage with keys that are already robust hashes (like an `ObjectId`).
/// The first `8` bytes of the hash are used as the `HashMap` hash
pub type HashSet<T = ObjectId> = hashbrown::HashSet<T, hash::Builder>;
