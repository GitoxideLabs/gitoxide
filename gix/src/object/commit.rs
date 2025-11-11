use crate::{bstr, bstr::BStr, Commit, ObjectDetached, Tree};

mod error {
    use crate::object;

    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FindExistingObject(#[from] object::find::existing::Error),
        #[error("The commit could not be decoded fully or partially")]
        Decode(#[from] gix_object::decode::Error),
        #[error("The commit date could not be parsed")]
        ParseDate(#[from] gix_date::parse::Error),
        #[error("Expected object of type {}, but got {}", .expected, .actual)]
        ObjectKind {
            expected: gix_object::Kind,
            actual: gix_object::Kind,
        },
    }
}

pub use error::Error;

/// Remove Lifetime
impl Commit<'_> {
    /// Create an owned instance of this object, copying our data in the process.
    pub fn detached(&self) -> ObjectDetached {
        ObjectDetached {
            id: self.id,
            kind: gix_object::Kind::Commit,
            data: self.data.clone(),
        }
    }

    /// Sever the connection to the `Repository` and turn this instance into a standalone object.
    pub fn detach(self) -> ObjectDetached {
        self.into()
    }

    /// Retrieve this instance's encoded data, leaving its own data empty.
    ///
    /// This method works around the immovability of members of this type.
    pub fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data)
    }
}

impl<'repo> Commit<'repo> {
    /// Turn this objects id into a shortened id with a length in hex as configured by `core.abbrev`.
    pub fn short_id(&self) -> Result<gix_hash::Prefix, crate::id::shorten::Error> {
        use crate::ext::ObjectIdExt;
        self.id.attach(self.repo).shorten()
    }

    /// Parse the commits message into a [`MessageRef`][gix_object::commit::MessageRef]
    pub fn message(&self) -> Result<gix_object::commit::MessageRef<'_>, gix_object::decode::Error> {
        Ok(gix_object::commit::MessageRef::from_bytes(self.message_raw()?))
    }
    /// Decode the commit object until the message and return it.
    pub fn message_raw(&self) -> Result<&'_ BStr, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data).message()
    }
    /// Obtain the message by using intricate knowledge about the encoding, which is fastest and
    /// can't fail at the expense of error handling.
    pub fn message_raw_sloppy(&self) -> &BStr {
        use bstr::ByteSlice;
        self.data
            .find(b"\n\n")
            .map(|pos| &self.data[pos + 2..])
            .unwrap_or_default()
            .as_bstr()
    }

    /// Decode the commit and obtain the time at which the commit was created.
    ///
    /// For the time at which it was authored, refer to `.decode()?.author()?.time()`.
    pub fn time(&self) -> Result<gix_date::Time, Error> {
        Ok(self.committer()?.time()?)
    }

    /// Decode the entire commit object and return it for accessing all commit information.
    ///
    /// It will allocate only if there are more than 2 parents.
    ///
    /// Note that the returned commit object does make lookup easy and should be
    /// used for successive calls to string-ish information to avoid decoding the object
    /// more than once.
    pub fn decode(&self) -> Result<gix_object::CommitRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRef::from_bytes(&self.data)
    }

    /// Return an iterator over tokens, representing this commit piece by piece.
    pub fn iter(&self) -> gix_object::CommitRefIter<'_> {
        gix_object::CommitRefIter::from_bytes(&self.data)
    }

    /// Return the commits author, with surrounding whitespace trimmed.
    pub fn author(&self) -> Result<gix_actor::SignatureRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data)
            .author()
            .map(|s| s.trim())
    }

    /// Return the commits committer. with surrounding whitespace trimmed.
    pub fn committer(&self) -> Result<gix_actor::SignatureRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data)
            .committer()
            .map(|s| s.trim())
    }

    /// Decode this commits parent ids on the fly without allocating.
    // TODO: tests
    pub fn parent_ids(&self) -> impl Iterator<Item = crate::Id<'repo>> + '_ {
        use crate::ext::ObjectIdExt;
        let repo = self.repo;
        gix_object::CommitRefIter::from_bytes(&self.data)
            .parent_ids()
            .map(move |id| id.attach(repo))
    }

    /// Parse the commit and return the tree object it points to.
    pub fn tree(&self) -> Result<Tree<'repo>, Error> {
        match self.tree_id()?.object()?.try_into_tree() {
            Ok(tree) => Ok(tree),
            Err(crate::object::try_into::Error { actual, expected, .. }) => Err(Error::ObjectKind { actual, expected }),
        }
    }

    /// Parse the commit and return the tree id it points to.
    pub fn tree_id(&self) -> Result<crate::Id<'repo>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data)
            .tree_id()
            .map(|id| crate::Id::from_id(id, self.repo))
    }

    /// Return our id own id with connection to this repository.
    pub fn id(&self) -> crate::Id<'repo> {
        use crate::ext::ObjectIdExt;
        self.id.attach(self.repo)
    }

    /// Obtain a platform for traversing ancestors of this commit.
    pub fn ancestors(&self) -> crate::revision::walk::Platform<'repo> {
        self.id().ancestors()
    }

    /// Create a platform to further configure a `git describe` operation to find a name for this commit by looking
    /// at the closest annotated tags (by default) in its past.
    #[cfg(feature = "revision")]
    pub fn describe(&self) -> crate::commit::describe::Platform<'repo> {
        crate::commit::describe::Platform {
            id: self.id,
            repo: self.repo,
            select: Default::default(),
            first_parent: false,
            id_as_fallback: false,
            max_candidates: 10,
        }
    }

    /// Extracts the PGP signature and the data that was used to create the signature, or `None` if it wasn't signed.
    // TODO: make it possible to verify the signature, probably by wrapping `SignedData`. It's quite some work to do it properly.
    pub fn signature(
        &self,
    ) -> Result<Option<(std::borrow::Cow<'_, BStr>, gix_object::commit::SignedData<'_>)>, gix_object::decode::Error>
    {
        gix_object::CommitRefIter::signature(&self.data)
    }
}

impl std::fmt::Debug for Commit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Commit({})", self.id)
    }
}
