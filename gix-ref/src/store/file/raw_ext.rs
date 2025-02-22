use std::collections::BTreeSet;

use crate::{
    packed, peel,
    raw::Reference,
    store_impl::{file, file::log},
    Target,
};
use gix_hash::ObjectId;

pub trait Sealed {}
impl Sealed for crate::Reference {}

/// A trait to extend [Reference][crate::Reference] with functionality requiring a [file::Store].
pub trait ReferenceExt: Sealed {
    /// A step towards obtaining forward or reverse iterators on reference logs.
    fn log_iter<'a, 's>(&'a self, store: &'s file::Store) -> log::iter::Platform<'a, 's>;

    /// For details, see [`Reference::log_exists()`].
    fn log_exists(&self, store: &file::Store) -> bool;

    /// Follow all symbolic targets this reference might point to and peel the underlying object
    /// to the end of the tag-chain, returning the first non-tag object the annotated tag points to,
    /// using `objects` to access them and `store` to lookup symbolic references.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following all symbolic
    /// refs and all annotated tags to the first non-tag object.
    fn peel_to_id_in_place(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error>;

    /// Like [`ReferenceExt::peel_to_id_in_place()`], but with support for a known stable `packed` buffer
    /// to use for resolving symbolic links.
    fn peel_to_id_in_place_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> Result<ObjectId, peel::to_id::Error>;

    /// Like [`ReferenceExt::follow()`], but follows all symbolic references while gracefully handling loops,
    /// altering this instance in place.
    fn follow_to_object_in_place_packed(
        &mut self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
    ) -> Result<ObjectId, peel::to_object::Error>;

    /// Follow this symbolic reference one level and return the ref it refers to.
    ///
    /// Returns `None` if this is not a symbolic reference, hence the leaf of the chain.
    fn follow(&self, store: &file::Store) -> Option<Result<Reference, file::find::existing::Error>>;

    /// Follow this symbolic reference one level and return the ref it refers to,
    /// possibly providing access to `packed` references for lookup if it contains the referent.
    ///
    /// Returns `None` if this is not a symbolic reference, hence the leaf of the chain.
    fn follow_packed(
        &self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
    ) -> Option<Result<Reference, file::find::existing::Error>>;
}

impl ReferenceExt for Reference {
    fn log_iter<'a, 's>(&'a self, store: &'s file::Store) -> log::iter::Platform<'a, 's> {
        log::iter::Platform {
            store,
            name: self.name.as_ref(),
            buf: Vec::new(),
        }
    }

    fn log_exists(&self, store: &file::Store) -> bool {
        store
            .reflog_exists(self.name.as_ref())
            .expect("infallible name conversion")
    }

    fn peel_to_id_in_place(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error> {
        let packed = store.assure_packed_refs_uptodate().map_err(|err| {
            peel::to_id::Error::FollowToObject(peel::to_object::Error::Follow(file::find::existing::Error::Find(
                file::find::Error::PackedOpen(err),
            )))
        })?;
        self.peel_to_id_in_place_packed(store, objects, packed.as_ref().map(|b| &***b))
    }

    fn peel_to_id_in_place_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> Result<ObjectId, peel::to_id::Error> {
        match self.peeled {
            Some(peeled) => {
                self.target = Target::Object(peeled.to_owned());
                Ok(peeled)
            }
            None => {
                let mut oid = self.follow_to_object_in_place_packed(store, packed)?;
                let mut buf = Vec::new();
                let peeled_id = loop {
                    let gix_object::Data { kind, data } =
                        objects
                            .try_find(&oid, &mut buf)?
                            .ok_or_else(|| peel::to_id::Error::NotFound {
                                oid,
                                name: self.name.0.clone(),
                            })?;
                    match kind {
                        gix_object::Kind::Tag => {
                            oid = gix_object::TagRefIter::from_bytes(data).target_id().map_err(|_err| {
                                peel::to_id::Error::NotFound {
                                    oid,
                                    name: self.name.0.clone(),
                                }
                            })?;
                        }
                        _ => break oid,
                    }
                };
                self.peeled = Some(peeled_id);
                self.target = Target::Object(peeled_id);
                Ok(peeled_id)
            }
        }
    }

    fn follow_to_object_in_place_packed(
        &mut self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
    ) -> Result<ObjectId, peel::to_object::Error> {
        match self.target {
            Target::Object(id) => Ok(id),
            Target::Symbolic(_) => {
                let mut seen = BTreeSet::new();
                let cursor = &mut *self;
                while let Some(next) = cursor.follow_packed(store, packed) {
                    let next = next?;
                    if seen.contains(&next.name) {
                        return Err(peel::to_object::Error::Cycle {
                            start_absolute: store.reference_path(cursor.name.as_ref()),
                        });
                    }
                    *cursor = next;
                    seen.insert(cursor.name.clone());
                    const MAX_REF_DEPTH: usize = 5;
                    if seen.len() == MAX_REF_DEPTH {
                        return Err(peel::to_object::Error::DepthLimitExceeded {
                            max_depth: MAX_REF_DEPTH,
                        });
                    }
                }
                let oid = self.target.try_id().expect("peeled ref").to_owned();
                Ok(oid)
            }
        }
    }

    fn follow(&self, store: &file::Store) -> Option<Result<Reference, file::find::existing::Error>> {
        let packed = match store
            .assure_packed_refs_uptodate()
            .map_err(|err| file::find::existing::Error::Find(file::find::Error::PackedOpen(err)))
        {
            Ok(packed) => packed,
            Err(err) => return Some(Err(err)),
        };
        self.follow_packed(store, packed.as_ref().map(|b| &***b))
    }

    fn follow_packed(
        &self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
    ) -> Option<Result<Reference, file::find::existing::Error>> {
        match &self.target {
            Target::Object(_) => None,
            Target::Symbolic(full_name) => match store.try_find_packed(full_name.as_ref(), packed) {
                Ok(Some(next)) => Some(Ok(next)),
                Ok(None) => Some(Err(file::find::existing::Error::NotFound {
                    name: full_name.to_path().to_owned(),
                })),
                Err(err) => Some(Err(file::find::existing::Error::Find(err))),
            },
        }
    }
}
