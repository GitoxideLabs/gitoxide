use gix_features::fs;

use crate::store_impls::loose;

/// Returned by [`loose::Store::iter()`]
pub type Error = gix_features::fs::walkdir::Error;

impl loose::Iter {
    fn path_to_id(
        &self,
        res: Result<fs::walkdir::DirEntry, fs::walkdir::Error>,
    ) -> Option<Result<gix_hash::ObjectId, Error>> {
        use std::path::Component::Normal;

        match res {
            Ok(e) => {
                let p = e.path();
                let mut ci = p.components();
                let (c2, c1) = (ci.next_back(), ci.next_back());
                if let (Some(Normal(c1)), Some(Normal(c2))) = (c1, c2) {
                    if c1.len() == 2 && c2.len() == self.hash_hex_len - 2 {
                        if let (Some(c1), Some(c2)) = (c1.to_str(), c2.to_str()) {
                            let mut buf = gix_hash::Kind::hex_buf();
                            {
                                let (first_byte, rest) = buf[..self.hash_hex_len].split_at_mut(2);
                                first_byte.copy_from_slice(c1.as_bytes());
                                rest.copy_from_slice(c2.as_bytes());
                            }
                            if let Ok(b) = gix_hash::ObjectId::from_hex(&buf[..self.hash_hex_len]) {
                                return Some(Ok(b));
                            }
                        }
                    }
                }
            }
            Err(err) => return Some(Err(err)),
        }
        None
    }
}

impl Iterator for loose::Iter {
    type Item = Result<gix_hash::ObjectId, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(res) = self.inner.next() {
            if let Some(res) = self.path_to_id(res) {
                return Some(res);
            }
        }
        None
    }
}

/// Iteration and traversal
impl loose::Store {
    /// Return an iterator over all objects contained in the database.
    ///
    /// The [`Id`][gix_hash::ObjectId]s returned by the iterator can typically be used in the [`locate(…)`][loose::Store::try_find()] method.
    /// _Note_ that the result is not sorted or stable, thus ordering can change between runs.
    ///
    /// # Notes
    ///
    /// [`loose::Iter`] is used instead of `impl Iterator<…>` to allow using this iterator in struct fields, as is currently
    /// needed if iterators need to be implemented by hand in the absence of generators.
    pub fn iter(&self) -> loose::Iter {
        loose::Iter {
            inner: fs::walkdir_new(
                &self.path,
                fs::walkdir::Parallelism::ThreadPoolPerTraversal {
                    thread_name: "gix_odb::loose::Store::iter: fs-walk",
                },
                false,
            )
            .min_depth(2)
            .max_depth(3)
            .follow_links(false)
            .into_iter(),
            hash_hex_len: self.object_hash.len_in_hex(),
        }
    }
}
