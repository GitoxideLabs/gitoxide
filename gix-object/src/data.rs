//! Contains a borrowed Object bound to a buffer holding its decompressed data.

use crate::{BlobRef, CommitRef, CommitRefIter, Data, Kind, ObjectRef, TagRef, TagRefIter, TreeRef, TreeRefIter};

impl<'a> Data<'a> {
    /// Constructs a new data object from `data`, `kind` and `object_hash`.
    pub fn new(data: &'a [u8], kind: Kind, hash_kind: gix_hash::Kind) -> Data<'a> {
        Data {
            kind,
            object_hash: hash_kind,
            data,
        }
    }
    /// Decodes the data in the backing slice into a [`ObjectRef`], allowing to access all of its data
    /// conveniently. The cost of parsing an object is negligible.
    ///
    /// **Note** that [mutable, decoded objects][crate::Object] can be created from [`Data`]
    /// using [`crate::ObjectRef::into_owned()`].
    pub fn decode(&self) -> Result<ObjectRef<'a>, crate::decode::Error> {
        Ok(match self.kind {
            Kind::Tree => ObjectRef::Tree(TreeRef::from_bytes(self.data, self.object_hash)?),
            Kind::Blob => ObjectRef::Blob(BlobRef { data: self.data }),
            Kind::Commit => ObjectRef::Commit(CommitRef::from_bytes(self.data, self.object_hash)?),
            Kind::Tag => ObjectRef::Tag(TagRef::from_bytes(self.data, self.object_hash)?),
        })
    }

    /// Returns this object as tree iterator to parse entries one at a time to avoid allocations, or
    /// `None` if this is not a tree object.
    pub fn try_into_tree_iter(self) -> Option<TreeRefIter<'a>> {
        match self.kind {
            Kind::Tree => Some(TreeRefIter::from_bytes(self.data, self.object_hash)),
            _ => None,
        }
    }

    /// Returns this object as commit iterator to parse tokens one at a time to avoid allocations, or
    /// `None` if this is not a commit object.
    pub fn try_into_commit_iter(self) -> Option<CommitRefIter<'a>> {
        match self.kind {
            Kind::Commit => Some(CommitRefIter::from_bytes(self.data, self.object_hash)),
            _ => None,
        }
    }

    /// Returns this object as tag iterator to parse tokens one at a time to avoid allocations, or
    /// `None` if this is not a tag object.
    pub fn try_into_tag_iter(self) -> Option<TagRefIter<'a>> {
        match self.kind {
            Kind::Tag => Some(TagRefIter::from_bytes(self.data, self.object_hash)),
            _ => None,
        }
    }
}

/// Types supporting object hash verification
pub mod verify {
    /// Returned by [`crate::Data::verify_checksum()`]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Failed to hash object")]
        Hasher(#[from] gix_hash::hasher::Error),
        #[error(transparent)]
        Verify(#[from] gix_hash::verify::Error),
    }

    impl crate::Data<'_> {
        /// Compute the checksum of `self` and compare it with the `expected` hash.
        /// If the hashes do not match, an [`Error`] is returned, containing the actual
        /// hash of `self`.
        pub fn verify_checksum(&self, expected: &gix_hash::oid) -> Result<gix_hash::ObjectId, Error> {
            let actual = crate::compute_hash(expected.kind(), self.kind, self.data)?;
            actual.verify(expected)?;
            Ok(actual)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_object() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(std::mem::size_of::<Data<'_>>(), 24, "this shouldn't change unnoticed");
        #[cfg(target_pointer_width = "32")]
        assert_eq!(std::mem::size_of::<Data<'_>>(), 12, "this shouldn't change unnoticed");
    }
}
