//!
#![allow(clippy::empty_docs)]
use gix_hash::ObjectId;
pub use gix_object::Kind;

use crate::{Blob, Commit, Id, Object, ObjectDetached, Tag, Tree};

mod errors;
#[cfg(feature = "object-cache-dynamic")]
pub(crate) mod cache {
    pub use gix_pack::cache::object::MemoryCappedHashmap;
}
pub use errors::{conversion, find, write};
///
pub mod blob;
///
pub mod commit;
mod impls;
pub mod peel;
mod tag;
///
pub mod tree;

///
pub mod try_into {
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    #[error("Object named {id} was supposed to be of kind {expected}, but was kind {actual}.")]
    pub struct Error {
        pub actual: gix_object::Kind,
        pub expected: gix_object::Kind,
        pub id: gix_hash::ObjectId,
    }
}

impl ObjectDetached {
    /// Infuse this owned object with `repo` access.
    pub fn attach(self, repo: &crate::Repository) -> Object<'_> {
        Object {
            id: self.id,
            kind: self.kind,
            data: self.data,
            repo,
        }
    }
}

impl std::fmt::Debug for ObjectDetached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use gix_object::Kind::*;
        let type_name = match self.kind {
            Blob => "Blob",
            Commit => "Commit",
            Tree => "Tree",
            Tag => "Tag",
        };
        write!(f, "{}({})", type_name, self.id)
    }
}

/// Consuming conversions to attached object kinds.
impl<'repo> Object<'repo> {
    pub(crate) fn from_data(
        id: impl Into<ObjectId>,
        kind: Kind,
        data: Vec<u8>,
        repo: &'repo crate::Repository,
    ) -> Self {
        Object {
            id: id.into(),
            kind,
            data,
            repo,
        }
    }

    /// Transform this object into a blob, or panic if it is none.
    pub fn into_blob(self) -> Blob<'repo> {
        match self.try_into() {
            Ok(blob) => blob,
            Err(this) => panic!("Tried to use {} as blob, but was {}", this.id, this.kind),
        }
    }

    /// Transform this object into a tree, or panic if it is none.
    pub fn into_tree(self) -> Tree<'repo> {
        match self.try_into() {
            Ok(tree) => tree,
            Err(this) => panic!("Tried to use {} as tree, but was {}", this.id, this.kind),
        }
    }

    /// Transform this object into a commit, or panic if it is none.
    pub fn into_commit(self) -> Commit<'repo> {
        match self.try_into() {
            Ok(commit) => commit,
            Err(this) => panic!("Tried to use {} as commit, but was {}", this.id, this.kind),
        }
    }

    /// Transform this object into a tag, or panic if it is none.
    pub fn into_tag(self) -> Tag<'repo> {
        match self.try_into() {
            Ok(tag) => tag,
            Err(this) => panic!("Tried to use {} as tag, but was {}", this.id, this.kind),
        }
    }

    /// Transform this object into a commit, or return it as part of the `Err` if it is no commit.
    pub fn try_into_commit(self) -> Result<Commit<'repo>, try_into::Error> {
        self.try_into().map_err(|this: Self| try_into::Error {
            id: this.id,
            actual: this.kind,
            expected: gix_object::Kind::Commit,
        })
    }

    /// Transform this object into a tag, or return it as part of the `Err` if it is no commit.
    pub fn try_into_tag(self) -> Result<Tag<'repo>, try_into::Error> {
        self.try_into().map_err(|this: Self| try_into::Error {
            id: this.id,
            actual: this.kind,
            expected: gix_object::Kind::Commit,
        })
    }

    /// Transform this object into a tree, or return it as part of the `Err` if it is no tree.
    pub fn try_into_tree(self) -> Result<Tree<'repo>, try_into::Error> {
        self.try_into().map_err(|this: Self| try_into::Error {
            id: this.id,
            actual: this.kind,
            expected: gix_object::Kind::Tree,
        })
    }

    /// Transform this object into a blob, or return it as part of the `Err` if it is no blob.
    pub fn try_into_blob(self) -> Result<Blob<'repo>, try_into::Error> {
        self.try_into().map_err(|this: Self| try_into::Error {
            id: this.id,
            actual: this.kind,
            expected: gix_object::Kind::Blob,
        })
    }
}

impl Object<'_> {
    /// Create an owned instance of this object, copying our data in the process.
    pub fn detached(&self) -> ObjectDetached {
        ObjectDetached {
            id: self.id,
            kind: self.kind,
            data: self.data.clone(),
        }
    }

    /// Sever the connection to the `Repository` and turn this instance into a standalone object.
    pub fn detach(self) -> ObjectDetached {
        self.into()
    }
}

/// Conversions to detached, lower-level object types.
impl<'repo> Object<'repo> {
    /// Obtain a fully parsed commit whose fields reference our data buffer,
    ///
    /// # Panic
    ///
    /// - this object is not a commit
    /// - the commit could not be decoded
    pub fn to_commit_ref(&self) -> gix_object::CommitRef<'_> {
        self.try_to_commit_ref().expect("BUG: need a commit")
    }

    /// Obtain a fully parsed commit whose fields reference our data buffer.
    pub fn try_to_commit_ref(&self) -> Result<gix_object::CommitRef<'_>, conversion::Error> {
        gix_object::Data::new(self.kind, &self.data)
            .decode()?
            .into_commit()
            .ok_or(conversion::Error::UnexpectedType {
                expected: gix_object::Kind::Commit,
                actual: self.kind,
            })
    }

    /// Obtain an iterator over commit tokens like in [`to_commit_iter()`][Object::try_to_commit_ref_iter()].
    ///
    /// # Panic
    ///
    /// - this object is not a commit
    pub fn to_commit_ref_iter(&self) -> gix_object::CommitRefIter<'_> {
        gix_object::Data::new(self.kind, &self.data)
            .try_into_commit_iter()
            .expect("BUG: This object must be a commit")
    }

    /// Obtain a commit token iterator from the data in this instance, if it is a commit.
    pub fn try_to_commit_ref_iter(&self) -> Option<gix_object::CommitRefIter<'_>> {
        gix_object::Data::new(self.kind, &self.data).try_into_commit_iter()
    }

    /// Obtain a tag token iterator from the data in this instance.
    ///
    /// # Panic
    ///
    /// - this object is not a tag
    pub fn to_tag_ref_iter(&self) -> gix_object::TagRefIter<'_> {
        gix_object::Data::new(self.kind, &self.data)
            .try_into_tag_iter()
            .expect("BUG: this object must be a tag")
    }

    /// Obtain a tag token iterator from the data in this instance.
    ///
    /// # Panic
    ///
    /// - this object is not a tag
    pub fn try_to_tag_ref_iter(&self) -> Option<gix_object::TagRefIter<'_>> {
        gix_object::Data::new(self.kind, &self.data).try_into_tag_iter()
    }

    /// Obtain a tag object from the data in this instance.
    ///
    /// # Panic
    ///
    /// - this object is not a tag
    /// - the tag could not be decoded
    pub fn to_tag_ref(&self) -> gix_object::TagRef<'_> {
        self.try_to_tag_ref().expect("BUG: need tag")
    }

    /// Obtain a fully parsed tag object whose fields reference our data buffer.
    pub fn try_to_tag_ref(&self) -> Result<gix_object::TagRef<'_>, conversion::Error> {
        gix_object::Data::new(self.kind, &self.data)
            .decode()?
            .into_tag()
            .ok_or(conversion::Error::UnexpectedType {
                expected: gix_object::Kind::Tag,
                actual: self.kind,
            })
    }

    /// Return the attached id of this object.
    pub fn id(&self) -> Id<'repo> {
        Id::from_id(self.id, self.repo)
    }
}
