use std::collections::BTreeSet;

use gix_error::{ErrorExt, ExnResult, Message, ResultExt, corruption, not_found};
use gix_hash::ObjectId;

use crate::{
    Target, packed,
    raw::Reference,
    store_impl::{file, file::log},
};

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
    #[deprecated = "Use `peel_to_id()` instead"]
    fn peel_to_id_in_place(&mut self, store: &file::Store, objects: &dyn gix_object::Find) -> ExnResult<ObjectId>;

    /// Follow all symbolic targets this reference might point to and peel the underlying object
    /// to the end of the tag-chain, returning the first non-tag object the annotated tag points to,
    /// using `objects` to access them and `store` to lookup symbolic references.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following all symbolic
    /// refs and all annotated tags to the first non-tag object.
    ///
    /// Note that this method mutates `self` in place if it does not already point to a
    /// non-symbolic object.
    fn peel_to_id(&mut self, store: &file::Store, objects: &dyn gix_object::Find) -> ExnResult<ObjectId>;

    /// Like [`ReferenceExt::peel_to_id_in_place()`], but with support for a known stable `packed` buffer
    /// to use for resolving symbolic links.
    #[deprecated = "Use `peel_to_id_packed()` instead"]
    fn peel_to_id_in_place_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<ObjectId>;

    /// Like [`ReferenceExt::peel_to_id()`], but with support for a known stable `packed` buffer to
    /// use for resolving symbolic links.
    /// Object lookup failures include [metadata](gix_error::Exn::metadata()) `object_id` (hex text) and `reference`
    /// (name bytes).
    /// Missing objects are classified as not found; lookup errors retain their own classifications.
    fn peel_to_id_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<ObjectId>;

    /// Like [`ReferenceExt::follow()`], but follows all symbolic references while gracefully handling loops,
    /// altering this instance in place.
    #[deprecated = "Use `follow_to_object_packed()` instead"]
    fn follow_to_object_in_place_packed(
        &mut self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<ObjectId>;

    /// Like [`ReferenceExt::follow()`], but follows all symbolic references while gracefully handling loops,
    /// altering this instance in place.
    /// Cycle failures include [metadata](gix_error::Exn::metadata()) `path` (native path); depth-limit failures include
    /// `max_depth` (unsigned integer).
    fn follow_to_object_packed(&mut self, store: &file::Store, packed: Option<&packed::Buffer>) -> ExnResult<ObjectId>;

    /// Follow this symbolic reference one level and return the ref it refers to.
    ///
    /// Returns `None` if this is not a symbolic reference, hence the leaf of the chain.
    fn follow(&self, store: &file::Store) -> Option<ExnResult<Reference>>;

    /// Follow this symbolic reference one level and return the ref it refers to,
    /// possibly providing access to `packed` references for lookup if it contains the referent.
    ///
    /// Returns `None` if this is not a symbolic reference, hence the leaf of the chain.
    fn follow_packed(&self, store: &file::Store, packed: Option<&packed::Buffer>) -> Option<ExnResult<Reference>>;
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

    fn peel_to_id_in_place(&mut self, store: &file::Store, objects: &dyn gix_object::Find) -> ExnResult<ObjectId> {
        self.peel_to_id(store, objects)
    }

    fn peel_to_id(&mut self, store: &file::Store, objects: &dyn gix_object::Find) -> ExnResult<ObjectId> {
        let packed = store.assure_packed_refs_uptodate()?;
        self.peel_to_id_packed(store, objects, packed.as_ref().map(|b| &***b))
    }

    fn peel_to_id_in_place_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<ObjectId> {
        self.peel_to_id_packed(store, objects, packed)
    }

    /// Object lookup failures include [metadata](gix_error::Exn::metadata()) `object_id` (hex text) and `reference`
    /// (name bytes).
    fn peel_to_id_packed(
        &mut self,
        store: &file::Store,
        objects: &dyn gix_object::Find,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<ObjectId> {
        match self.peeled {
            Some(peeled) => {
                self.target = Target::Object(peeled.to_owned());
                Ok(peeled)
            }
            None => {
                let mut object_id = self.follow_to_object_packed(store, packed)?;
                let mut buf = Vec::new();
                let peeled_id = loop {
                    let gix_object::Data {
                        kind,
                        data,
                        object_hash: hash_kind,
                    } = objects
                        .try_find(&object_id, &mut buf)
                        .or_raise_erased(|| {
                            Message::new("Could not peel reference to an object")
                                .with("object_id", object_id.to_string())
                                .with("reference", self.name.as_bstr())
                        })?
                        .ok_or_else(|| {
                            not_found("Could not peel reference to an object: object could not be found")
                                .with("object_id", object_id.to_string())
                                .with("reference", self.name.as_bstr())
                                .raise_erased()
                        })?;
                    match kind {
                        gix_object::Kind::Tag => {
                            object_id = gix_object::TagRefIter::from_bytes(data, hash_kind)
                                .target_id()
                                .or_raise(|| {
                                    corruption(format!(
                                        "Could not decode tag {object_id} as referred to by {:?}",
                                        self.name.0
                                    ))
                                })
                                .or_erased()?;
                        }
                        _ => break object_id,
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
    ) -> ExnResult<ObjectId> {
        self.follow_to_object_packed(store, packed)
    }

    /// Cycle failures include [metadata](gix_error::Exn::metadata()) `path` (native path); depth-limit failures include
    /// `max_depth` (unsigned integer).
    fn follow_to_object_packed(&mut self, store: &file::Store, packed: Option<&packed::Buffer>) -> ExnResult<ObjectId> {
        match self.target {
            Target::Object(id) => Ok(id),
            Target::Symbolic(_) => {
                let mut seen = BTreeSet::new();
                let cursor = &mut *self;
                while let Some(next) = cursor.follow_packed(store, packed) {
                    let next = next?;
                    if seen.contains(&next.name) {
                        return Err(corruption("Aborting symbolic reference cycle")
                            .with("path", store.reference_path(cursor.name.as_ref()))
                            .raise_erased());
                    }
                    *cursor = next;
                    seen.insert(cursor.name.clone());
                    const MAX_REF_DEPTH: usize = 5;
                    if seen.len() == MAX_REF_DEPTH {
                        return Err(Message::new("Symbolic reference depth limit exceeded")
                            .with("max_depth", MAX_REF_DEPTH)
                            .raise_erased());
                    }
                }
                let oid = self.target.try_id().expect("peeled ref").to_owned();
                Ok(oid)
            }
        }
    }

    fn follow(&self, store: &file::Store) -> Option<ExnResult<Reference>> {
        let packed = match store.assure_packed_refs_uptodate() {
            Ok(packed) => packed,
            Err(err) => return Some(Err(err)),
        };
        self.follow_packed(store, packed.as_ref().map(|b| &***b))
    }

    fn follow_packed(&self, store: &file::Store, packed: Option<&packed::Buffer>) -> Option<ExnResult<Reference>> {
        match &self.target {
            Target::Object(_) => None,
            Target::Symbolic(full_name) => match store.try_find_packed(full_name.as_ref(), packed) {
                Ok(Some(next)) => Some(Ok(next)),
                Ok(None) => Some(Err(file::find::NotFound {
                    name: full_name.to_path().to_owned(),
                }
                .raise_erased())),
                Err(err) => Some(Err(err)),
            },
        }
    }
}
