use std::{collections::HashSet, ops::Deref};

use gix_object::Exists;

use crate::store::{Handle, load_index};

///
pub mod lookup {
    use crate::loose;

    /// Returned by [`Handle::lookup_prefix()`][crate::store::Handle::lookup_prefix()]
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("An error occurred looking up a prefix which requires iteration")]
        LooseWalkDir(#[from] loose::iter::Error),
        #[error(transparent)]
        LoadIndex(#[from] crate::store::load_index::Error),
    }

    /// A way to indicate if a lookup, despite successful, was ambiguous or yielded exactly
    /// one result in the particular index.
    pub type Outcome = Result<gix_hash::ObjectId, ()>;
}

///
pub mod disambiguate {
    /// A potentially ambiguous prefix for use with `Handle::disambiguate_prefix()`.
    #[derive(Debug, Copy, Clone)]
    pub struct Candidate {
        id: gix_hash::ObjectId,
        hex_len: usize,
    }

    impl Candidate {
        /// Create a new potentially ambiguous prefix from an `id` and the desired minimal `hex_len`.
        ///
        /// It is considered ambiguous until it's disambiguated by validating that there is only a single object
        /// matching this prefix.
        pub fn new(id: impl Into<gix_hash::ObjectId>, hex_len: usize) -> Result<Self, gix_hash::prefix::Error> {
            let id = id.into();
            gix_hash::Prefix::new(&id, hex_len)?;
            Ok(Candidate { id, hex_len })
        }

        /// Transform ourselves into a `Prefix` with our current hex lengths.
        pub fn to_prefix(&self) -> gix_hash::Prefix {
            gix_hash::Prefix::new(&self.id, self.hex_len).expect("our hex-len to always be in bounds")
        }

        pub(crate) fn inc_hex_len(&mut self) {
            self.hex_len += 1;
            assert!(self.hex_len <= self.id.kind().len_in_hex());
        }

        pub(crate) fn id(&self) -> &gix_hash::oid {
            &self.id
        }

        pub(crate) fn hex_len(&self) -> usize {
            self.hex_len
        }
    }

    /// Returned by [`Handle::disambiguate_prefix()`][crate::store::Handle::disambiguate_prefix()]
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("An error occurred while trying to determine if a full hash contained in the object database")]
        Contains(#[from] crate::store::find::Error),
        #[error(transparent)]
        Lookup(#[from] super::lookup::Error),
    }
}

impl<S> Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    /// Return the exact number of packed objects after loading all currently available indices
    /// as last seen on disk.
    pub fn packed_object_count(&self) -> Result<u64, load_index::Error> {
        let mut count = self.packed_object_count.borrow_mut();
        match *count {
            Some(count) => Ok(count),
            None => {
                let _span = gix_features::trace::detail!("gix_odb::Handle::packed_object_count()");
                let mut snapshot = self.snapshot.borrow_mut();
                *snapshot = self.store.load_all_indices()?;
                let mut obj_count = 0;
                for index in &snapshot.indices {
                    obj_count += u64::from(index.num_objects());
                }
                *count = Some(obj_count);
                Ok(obj_count)
            }
        }
    }

    /// Given a prefix `candidate` with an object id and an initial `hex_len`, check if it only matches a single
    /// object within the entire object database and increment its `hex_len` by one until it is unambiguous.
    /// Return `Ok(None)` if no object with that prefix exists.
    pub fn disambiguate_prefix(
        &self,
        mut candidate: disambiguate::Candidate,
    ) -> Result<Option<gix_hash::Prefix>, disambiguate::Error> {
        let max_hex_len = candidate.id().kind().len_in_hex();
        if candidate.hex_len() == max_hex_len {
            return Ok(self.exists(candidate.id()).then(|| candidate.to_prefix()));
        }

        while candidate.hex_len() != max_hex_len {
            let res = self.lookup_prefix(candidate.to_prefix(), None)?;
            match res {
                Some(Ok(_id)) => return Ok(Some(candidate.to_prefix())),
                Some(Err(())) => {
                    candidate.inc_hex_len();
                    continue;
                }
                None => return Ok(None),
            }
        }
        Ok(Some(candidate.to_prefix()))
    }

    /// Find the only object matching `prefix` and return it as `Ok(Some(Ok(<ObjectId>)))`, or return `Ok(Some(Err(()))`
    /// if multiple different objects with the same prefix were found.
    ///
    /// Return `Ok(None)` if no object matched the `prefix`.
    ///
    /// Pass `candidates` to obtain the set of all object ids matching `prefix`, with the same return value as
    /// one would have received if it remained `None`.
    ///
    /// ### Performance Note
    ///
    /// - Unless the handles refresh mode is set to `Never`, each lookup will trigger a refresh of the object databases files
    ///   on disk if the prefix doesn't lead to ambiguous results.
    /// - Since all objects need to be examined to assure non-ambiguous return values, after calling this method all indices will
    ///   be loaded.
    /// - If `candidates` is `Some(…)`, the traversal will continue to obtain all candidates, which takes more time
    ///   as there is no early abort.
    pub fn lookup_prefix(
        &self,
        prefix: gix_hash::Prefix,
        mut candidates: Option<&mut HashSet<gix_hash::ObjectId>>,
    ) -> Result<Option<lookup::Outcome>, lookup::Error> {
        let mut candidate: Option<gix_hash::ObjectId> = None;
        loop {
            let snapshot = self.snapshot.borrow();
            for index in &snapshot.indices {
                #[allow(clippy::needless_option_as_deref)] // needed as it's the equivalent of a reborrow.
                let lookup_result = index.lookup_prefix(prefix, candidates.as_deref_mut());
                if candidates.is_none() && !check_candidate(lookup_result, &mut candidate) {
                    return Ok(Some(Err(())));
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                #[allow(clippy::needless_option_as_deref)] // needed as it's the equivalent of a reborrow.
                let lookup_result = lodb.lookup_prefix(prefix, candidates.as_deref_mut())?;
                if candidates.is_none() && !check_candidate(lookup_result, &mut candidate) {
                    return Ok(Some(Err(())));
                }
            }

            match self.store.load_one_index(self.refresh, snapshot.marker)? {
                Some(new_snapshot) => {
                    drop(snapshot);
                    *self.snapshot.borrow_mut() = new_snapshot;
                }
                None => {
                    return match &candidates {
                        Some(candidates) => match candidates.len() {
                            0 => Ok(None),
                            1 => Ok(candidates.iter().copied().next().map(Ok)),
                            _ => Ok(Some(Err(()))),
                        },
                        None => Ok(candidate.map(Ok)),
                    };
                }
            }
        }

        fn check_candidate(lookup_result: Option<lookup::Outcome>, candidate: &mut Option<gix_hash::ObjectId>) -> bool {
            match (lookup_result, &*candidate) {
                (Some(Ok(oid)), Some(candidate)) if *candidate != oid => false,
                (Some(Ok(_)) | None, Some(_)) | (None, None) => true,
                (Some(Err(())), _) => false,
                (Some(Ok(oid)), None) => {
                    *candidate = Some(oid);
                    true
                }
            }
        }
    }
}
