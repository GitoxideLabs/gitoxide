use std::collections::BTreeSet;

use gix_hash::ObjectId;

use crate::{Reference, Target, store};

/// Logical operations on a [`Reference`] that require access to its owning [`crate::Store`].
pub trait ReferenceExt: private::Sealed {
    /// Return a platform for iterating this reference's reflog.
    fn log_iter<'store>(&self, store: &'store crate::Store) -> store::log::Platform<'store>;

    /// Return whether this reference has a reflog.
    fn log_exists(&self, store: &crate::Store) -> Result<bool, store::log::Error>;

    /// Follow one symbolic-reference level.
    fn follow(&self, store: &crate::Store) -> Option<Result<Reference, store::find::existing::Error>>;

    /// Follow one symbolic-reference level through an existing snapshot.
    fn follow_with_snapshot(
        &self,
        snapshot: &store::snapshot::Snapshot<'_>,
    ) -> Option<Result<Reference, store::find::existing::Error>>;

    /// Follow symbolic references until this reference points directly to an object.
    fn follow_to_object(&mut self, store: &crate::Store) -> Result<ObjectId, peel::to_object::Error>;

    /// Follow symbolic references through an existing snapshot.
    fn follow_to_object_with_snapshot(
        &mut self,
        snapshot: &store::snapshot::Snapshot<'_>,
    ) -> Result<ObjectId, peel::to_object::Error>;

    /// Follow symbolic references and annotated tags to the first non-tag object.
    fn peel_to_id(
        &mut self,
        store: &crate::Store,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error>;

    /// Peel through an existing reference snapshot.
    fn peel_to_id_with_snapshot(
        &mut self,
        snapshot: &store::snapshot::Snapshot<'_>,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error>;
}

impl ReferenceExt for Reference {
    fn log_iter<'store>(&self, store: &'store crate::Store) -> store::log::Platform<'store> {
        store
            .reflog_iter(self.name.clone())
            .expect("a reference always contains a validated full name")
    }

    fn log_exists(&self, store: &crate::Store) -> Result<bool, store::log::Error> {
        store.reflog_exists(self.name.clone())
    }

    fn follow(&self, store: &crate::Store) -> Option<Result<Reference, store::find::existing::Error>> {
        let snapshot = match store.snapshot() {
            Ok(snapshot) => snapshot,
            Err(err) => {
                return Some(Err(store::find::existing::Error::Find(store::find::Error::Snapshot(
                    err,
                ))));
            }
        };
        self.follow_with_snapshot(&snapshot)
    }

    fn follow_with_snapshot(
        &self,
        snapshot: &store::snapshot::Snapshot<'_>,
    ) -> Option<Result<Reference, store::find::existing::Error>> {
        let Target::Symbolic(full_name) = &self.target else {
            return None;
        };
        Some(match snapshot.try_find(full_name.as_ref()) {
            Ok(Some(reference)) => Ok(reference),
            Ok(None) => Err(store::find::existing::Error::NotFound {
                name: full_name
                    .0
                    .clone()
                    .try_into()
                    .expect("a full reference name is also a valid partial name"),
            }),
            Err(err) => Err(store::find::existing::Error::Find(err)),
        })
    }

    fn follow_to_object(&mut self, store: &crate::Store) -> Result<ObjectId, peel::to_object::Error> {
        let snapshot = store.snapshot().map_err(|err| {
            peel::to_object::Error::Follow(store::find::existing::Error::Find(store::find::Error::Snapshot(err)))
        })?;
        self.follow_to_object_with_snapshot(&snapshot)
    }

    fn follow_to_object_with_snapshot(
        &mut self,
        snapshot: &store::snapshot::Snapshot<'_>,
    ) -> Result<ObjectId, peel::to_object::Error> {
        if let Target::Object(object_id) = self.target {
            return Ok(object_id);
        }

        let mut seen = BTreeSet::new();
        while let Some(next) = self.follow_with_snapshot(snapshot) {
            let next = next?;
            if !seen.insert(next.name.clone()) {
                return Err(peel::to_object::Error::Cycle { reference: next.name });
            }
            *self = next;
            const MAX_REF_DEPTH: usize = 5;
            if seen.len() == MAX_REF_DEPTH {
                return Err(peel::to_object::Error::DepthLimitExceeded {
                    max_depth: MAX_REF_DEPTH,
                });
            }
        }
        Ok(self
            .target
            .try_id()
            .expect("following stops only at a direct reference")
            .to_owned())
    }

    fn peel_to_id(
        &mut self,
        store: &crate::Store,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error> {
        let snapshot = store.snapshot().map_err(|err| {
            peel::to_object::Error::Follow(store::find::existing::Error::Find(store::find::Error::Snapshot(err)))
        })?;
        self.peel_to_id_with_snapshot(&snapshot, objects)
    }

    fn peel_to_id_with_snapshot(
        &mut self,
        snapshot: &store::snapshot::Snapshot<'_>,
        objects: &dyn gix_object::Find,
    ) -> Result<ObjectId, peel::to_id::Error> {
        if let Some(peeled_id) = self.peeled {
            self.target = Target::Object(peeled_id);
            return Ok(peeled_id);
        }

        let mut object_id = self.follow_to_object_with_snapshot(snapshot)?;
        let mut buf = Vec::new();
        let peeled_id = loop {
            let gix_object::Data {
                kind,
                data,
                object_hash,
            } = objects
                .try_find(&object_id, &mut buf)?
                .ok_or_else(|| peel::to_id::Error::NotFound {
                    object_id,
                    name: self.name.0.clone(),
                })?;
            match kind {
                gix_object::Kind::Tag => {
                    object_id = gix_object::TagRefIter::from_bytes(data, object_hash)
                        .target_id()
                        .map_err(|_| peel::to_id::Error::NotFound {
                            object_id,
                            name: self.name.0.clone(),
                        })?;
                }
                _ => break object_id,
            }
        };
        self.peeled = Some(peeled_id);
        self.target = Target::Object(peeled_id);
        Ok(peeled_id)
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Reference {}
}

/// Errors produced while following and peeling references through [`crate::Store`].
pub mod peel {
    /// Errors produced while peeling a reference to its first non-tag object.
    pub mod to_id {
        use gix_object::bstr::BString;

        /// The error returned by [`super::super::ReferenceExt::peel_to_id()`].
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            FollowToObject(#[from] super::to_object::Error),
            #[error("An object needed while peeling the reference could not be read")]
            Find(#[from] gix_object::find::Error),
            #[error("Object {object_id} referred to by {name:?} could not be found")]
            NotFound {
                object_id: gix_hash::ObjectId,
                name: BString,
            },
        }
    }

    /// Errors produced while following symbolic references to a direct object.
    pub mod to_object {
        /// The error returned by [`super::super::ReferenceExt::follow_to_object()`].
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error {
            #[error("Could not follow a single level of a symbolic reference")]
            Follow(#[from] crate::store::find::existing::Error),
            #[error("A symbolic reference cycle repeats at {reference}")]
            Cycle { reference: crate::FullName },
            #[error("Refusing to follow more than {max_depth} levels of symbolic-reference indirection")]
            DepthLimitExceeded { max_depth: usize },
        }
    }
}
