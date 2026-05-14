//!
#![allow(clippy::empty_docs)]
use crate::{
    Commit, Object, Tree, object,
    object::{Kind, peel},
};

///
pub mod to_kind {
    mod error {

        use crate::object;

        /// The error returned by [`Object::peel_to_kind()`][crate::Object::peel_to_kind()].
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            FindExistingObject(#[from] object::find::existing::Error),
            #[error("Last encountered object {oid} was {actual} while trying to peel to {expected}")]
            NotFound {
                oid: gix_hash::Prefix,
                actual: object::Kind,
                expected: object::Kind,
            },
        }
    }
    pub use error::Error;
}

impl<'repo> Object<'repo> {
    // TODO: tests
    /// Follow tags to their target and commits to trees until the given `kind` of object is encountered.
    ///
    /// Note that this object doesn't necessarily have to be the end of the chain.
    /// Typical values are [`Kind::Commit`] or [`Kind::Tree`].
    pub fn peel_to_kind(mut self, kind: Kind) -> Result<Self, peel::to_kind::Error> {
        loop {
            match self.kind {
                our_kind if kind == our_kind => {
                    return Ok(self);
                }
                Kind::Commit => {
                    let tree_id = self
                        .try_to_commit_ref_iter()
                        .expect("commit")
                        .tree_id()
                        .expect("valid commit");
                    let repo = self.repo;
                    drop(self);
                    self = repo.find_object(tree_id)?;
                }
                Kind::Tag => {
                    let target_id = self.to_tag_ref_iter().target_id().expect("valid tag");
                    let repo = self.repo;
                    drop(self);
                    self = repo.find_object(target_id)?;
                }
                Kind::Tree | Kind::Blob => {
                    return Err(peel::to_kind::Error::NotFound {
                        oid: self.id().shorten().unwrap_or_else(|_| self.id.into()),
                        actual: self.kind,
                        expected: kind,
                    });
                }
            }
        }
    }

    /// Peel this object into a tree and return it, if this is possible.
    ///
    /// This will follow tag objects and commits until their tree is reached.
    pub fn peel_to_tree(self) -> Result<Tree<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Tree)?.into_tree())
    }

    /// Peel this object into a commit and return it, if this is possible.
    ///
    /// This will follow tag objects until a commit is reached.
    pub fn peel_to_commit(self) -> Result<Commit<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Commit)?.into_commit())
    }

    // TODO: tests
    /// Follow all tag object targets until a commit, tree or blob is reached.
    ///
    /// Note that this method is different from [`peel_to_kind(…)`][Object::peel_to_kind()] as it won't
    /// peel commits to their tree, but handles tags only.
    pub fn peel_tags_to_end(mut self) -> Result<Self, object::find::existing::Error> {
        loop {
            match self.kind {
                Kind::Commit | Kind::Tree | Kind::Blob => break Ok(self),
                Kind::Tag => {
                    let target_id = self.to_tag_ref_iter().target_id().expect("valid tag");
                    let repo = self.repo;
                    drop(self);
                    self = repo.find_object(target_id)?;
                }
            }
        }
    }
}
