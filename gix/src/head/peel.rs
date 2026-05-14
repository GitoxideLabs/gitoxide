use crate::{
    Head,
    ext::{ObjectIdExt, ReferenceExt},
    head::Kind,
};

mod error {
    use crate::{object, reference};

    /// The error returned by [`Head::peel_to_id()`](super::Head::try_peel_to_id()) and
    /// [`Head::into_fully_peeled_id()`](super::Head::try_into_peeled_id()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FindExistingObject(#[from] object::find::existing::Error),
        #[error(transparent)]
        PeelReference(#[from] reference::peel::Error),
    }
}

pub use error::Error;

///
pub mod into_id {
    use crate::object;

    /// The error returned by [`Head::into_peeled_id()`](super::Head::into_peeled_id()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Peel(#[from] super::Error),
        #[error("Branch '{name}' does not have any commits")]
        Unborn { name: gix_ref::FullName },
        #[error(transparent)]
        ObjectKind(#[from] object::try_into::Error),
    }
}

///
pub mod to_commit {
    use crate::object;

    /// The error returned by [`Head::peel_to_commit()`](super::Head::peel_to_commit()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        PeelToObject(#[from] super::to_object::Error),
        #[error(transparent)]
        ObjectKind(#[from] object::try_into::Error),
    }
}

///
pub mod to_object {
    /// The error returned by [`Head::peel_to_object()`](super::Head::peel_to_object()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Peel(#[from] super::Error),
        #[error("Branch '{name}' does not have any commits")]
        Unborn { name: gix_ref::FullName },
    }
}

impl<'repo> Head<'repo> {
    /// Peel this instance and consume it to make obtaining its final target id possible, while returning an error on unborn heads.
    ///
    /// The final target is obtained by following symbolic references and peeling tags to their final destination, which
    /// typically is a commit, but can be any object.
    pub fn into_peeled_id(mut self) -> Result<crate::Id<'repo>, into_id::Error> {
        self.try_peel_to_id()?;
        self.id().ok_or_else(|| match self.kind {
            Kind::Symbolic(gix_ref::Reference { name, .. }) | Kind::Unborn(name) => into_id::Error::Unborn { name },
            Kind::Detached { .. } => unreachable!("id can be returned after peeling"),
        })
    }

    /// Peel this instance and consume it to make obtaining its final target object possible, while returning an error on unborn heads.
    ///
    /// The final target is obtained by following symbolic references and peeling tags to their final destination, which
    /// typically is a commit, but can be any object as well.
    pub fn into_peeled_object(mut self) -> Result<crate::Object<'repo>, to_object::Error> {
        self.peel_to_object()
    }

    /// Consume this instance and transform it into the final object that it points to, or `Ok(None)` if the `HEAD`
    /// reference is yet to be born.
    ///
    /// The final target is obtained by following symbolic references and peeling tags to their final destination, which
    /// typically is a commit, but can be any object.
    pub fn try_into_peeled_id(mut self) -> Result<Option<crate::Id<'repo>>, Error> {
        self.try_peel_to_id()
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, and return that object id.
    ///
    /// Returns `Ok(None)` if the head is unborn.
    ///
    /// The final target is obtained by following symbolic references and peeling tags to their final destination, which
    /// typically is a commit, but can be any object.
    #[deprecated = "Use `try_peel_to_id()` instead"]
    pub fn try_peel_to_id_in_place(&mut self) -> Result<Option<crate::Id<'repo>>, Error> {
        self.try_peel_to_id()
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, and return that object id.
    ///
    /// Returns `Ok(None)` if the head is unborn.
    ///
    /// The final target is obtained by following symbolic references and peeling tags to their final destination, which
    /// typically is a commit, but can be any object.
    ///
    /// Note that this method mutates `self` in place.
    pub fn try_peel_to_id(&mut self) -> Result<Option<crate::Id<'repo>>, Error> {
        Ok(Some(match &mut self.kind {
            Kind::Unborn(_name) => return Ok(None),
            Kind::Detached {
                peeled: Some(peeled), ..
            } => (*peeled).attach(self.repo),
            Kind::Detached { peeled: None, target } => {
                let id = target.attach(self.repo);
                if id.header()?.kind() == gix_object::Kind::Commit {
                    id
                } else {
                    match id.object()?.peel_tags_to_end() {
                        Ok(obj) => {
                            self.kind = Kind::Detached {
                                peeled: Some(obj.id),
                                target: *target,
                            };
                            obj.id()
                        }
                        Err(err) => return Err(err.into()),
                    }
                }
            }
            Kind::Symbolic(r) => {
                let mut nr = r.clone().attach(self.repo);
                let peeled = nr.peel_to_id();
                *r = nr.detach();
                peeled?
            }
        }))
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, transform the id into a commit if possible and return that.
    ///
    /// Returns an error if the head is unborn or if it doesn't point to a commit.
    #[deprecated = "Use `peel_to_object()` instead"]
    pub fn peel_to_object_in_place(&mut self) -> Result<crate::Object<'repo>, to_object::Error> {
        self.peel_to_object()
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, transform the id into a commit if possible and return that.
    ///
    /// Returns an error if the head is unborn or if it doesn't point to a commit.
    ///
    /// Note that this method mutates `self` in place.
    pub fn peel_to_object(&mut self) -> Result<crate::Object<'repo>, to_object::Error> {
        let id = self.try_peel_to_id()?.ok_or_else(|| to_object::Error::Unborn {
            name: self.referent_name().expect("unborn").to_owned(),
        })?;
        id.object()
            .map_err(|err| to_object::Error::Peel(Error::FindExistingObject(err)))
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, transform the id into a commit if possible and return that.
    ///
    /// Returns an error if the head is unborn or if it doesn't point to a commit.
    #[deprecated = "Use `peel_to_commit()` instead"]
    pub fn peel_to_commit_in_place(&mut self) -> Result<crate::Commit<'repo>, to_commit::Error> {
        self.peel_to_commit()
    }

    /// Follow the symbolic reference of this head until its target object and peel it by following tag objects until there is no
    /// more object to follow, transform the id into a commit if possible and return that.
    ///
    /// Returns an error if the head is unborn or if it doesn't point to a commit.
    ///
    /// Note that this method mutates `self` in place.
    pub fn peel_to_commit(&mut self) -> Result<crate::Commit<'repo>, to_commit::Error> {
        Ok(self.peel_to_object()?.try_into_commit()?)
    }
}
