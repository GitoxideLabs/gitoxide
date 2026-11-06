use gix_hash::ObjectId;

#[expect(unnameable_types, reason = "sealed traits must remain unnameable")]
pub trait Sealed {}

pub type AncestorsIter<Find> = gix_traverse::commit::Simple<Find, fn(&gix_hash::oid) -> bool>;

/// An extension trait to add functionality to [`ObjectId`]s.
pub trait ObjectIdExt: Sealed {
    /// Create an iterator over the ancestry of the commits reachable from this id, which must be a commit.
    fn ancestors<Find>(self, find: Find) -> AncestorsIter<Find>
    where
        Find: gix_object::Find;

    /// Infuse this object id `repo` access.
    fn attach(self, repo: &crate::Repository) -> crate::Id<'_>;
}

impl Sealed for ObjectId {}

impl ObjectIdExt for ObjectId {
    fn ancestors<Find>(self, find: Find) -> AncestorsIter<Find>
    where
        Find: gix_object::Find,
    {
        gix_traverse::commit::Simple::new(Some(self), find)
    }

    fn attach(self, repo: &crate::Repository) -> crate::Id<'_> {
        crate::Id::from_id(self, repo)
    }
}
