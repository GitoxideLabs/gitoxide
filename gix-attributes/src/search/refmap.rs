//! A utility to store objects by identity, which deduplicates them while avoiding lifetimes.
//!
//! We chose to use hashing/identity over pointers as it's possible that different objects end up in the same memory location,
//! which would create obscure bugs. The same could happen with hash collisions, but they these are designed to be less likely.
use std::{
    collections::{BTreeMap, btree_map::Entry, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

pub(crate) type RefMapKey = u64;
#[derive(Clone)]
pub(crate) struct RefMap<T>(BTreeMap<RefMapKey, T>);

impl<T> Default for RefMap<T> {
    fn default() -> Self {
        RefMap(Default::default())
    }
}

impl<T> RefMap<T>
where
    T: Hash + Clone,
{
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn insert(&mut self, value: &T) -> RefMapKey {
        let mut s = DefaultHasher::new();
        value.hash(&mut s);
        let key = s.finish();
        match self.0.entry(key) {
            Entry::Vacant(e) => {
                e.insert(value.clone());
                key
            }
            Entry::Occupied(_) => key,
        }
    }

    pub(crate) fn insert_owned(&mut self, value: T) -> RefMapKey {
        let mut s = DefaultHasher::new();
        value.hash(&mut s);
        let key = s.finish();
        match self.0.entry(key) {
            Entry::Vacant(e) => {
                e.insert(value);
                key
            }
            Entry::Occupied(_) => key,
        }
    }

    pub(crate) fn resolve(&self, key: RefMapKey) -> Option<&T> {
        self.0.get(&key)
    }
}
