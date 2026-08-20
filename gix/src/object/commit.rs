use crate::{Commit, ObjectDetached, Tree, bstr, bstr::BStr};
use gix_error::ResultExt;

mod error {
    /// The error returned by commit accessors.
    pub type Error = gix_error::Error;
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cmp::Ordering;
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let commit = repo.head_commit()?;
    /// let short_id = commit.short_id()?;
    ///
    /// assert_eq!(short_id.cmp_oid(&commit.id), Ordering::Equal);
    /// assert_eq!(short_id.to_string(), "3189cd3");
    /// # Ok(()) }
    /// ```
    pub fn short_id(&self) -> Result<gix_hash::Prefix, crate::id::shorten::Error> {
        use crate::ext::ObjectIdExt;
        self.id.attach(self.repo).shorten()
    }

    /// Parse the commits message into a [`MessageRef`][gix_object::commit::MessageRef]
    pub fn message(&self) -> Result<gix_object::commit::MessageRef<'_>, gix_object::decode::Error> {
        Ok(gix_object::commit::MessageRef::from_bytes(self.message_raw()?))
    }
    /// Decode the commit object until the message and return it.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let commit = repo.head_commit()?;
    ///
    /// assert_eq!(commit.message_raw()?, "c2\n");
    /// # Ok(()) }
    /// ```
    pub fn message_raw(&self) -> Result<&'_ BStr, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind()).message()
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
    /// For the time at which it was authored, refer to `.author()?.time()`.
    pub fn time(&self) -> Result<gix_date::Time, Error> {
        self.committer()
            .or_raise(|| gix_error::message("The commit could not be decoded fully or partially"))?
            .time()
            .or_raise(|| gix_error::message("The commit date could not be parsed"))
            .map_err(Into::into)
    }

    /// Decode the entire commit object and return it for accessing all commit information.
    ///
    /// It will allocate only if there are more than 2 parents.
    ///
    /// Note that the returned commit object does make lookup easy and should be
    /// used for successive calls to string-ish information to avoid decoding the object
    /// more than once.
    pub fn decode(&self) -> Result<gix_object::CommitRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRef::from_bytes(&self.data, self.id.kind())
    }

    /// Return an iterator over tokens, representing this commit piece by piece.
    pub fn iter(&self) -> gix_object::CommitRefIter<'_> {
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind())
    }

    /// Return the commits author, with surrounding whitespace trimmed.
    pub fn author(&self) -> Result<gix_actor::SignatureRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind())
            .author()
            .map(|s| s.trim())
    }

    /// Return the commits committer. with surrounding whitespace trimmed.
    pub fn committer(&self) -> Result<gix_actor::SignatureRef<'_>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind())
            .committer()
            .map(|s| s.trim())
    }

    /// Decode this commits parent ids on the fly without allocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let commit = repo.head_commit()?;
    /// let parent_ids: Vec<_> = commit.parent_ids().collect();
    ///
    /// #[cfg(feature = "revision")]
    /// assert_eq!(parent_ids, vec![repo.rev_parse_single("HEAD~1")?]);
    /// # Ok(()) }
    /// ```
    pub fn parent_ids(&self) -> impl Iterator<Item = crate::Id<'repo>> + '_ {
        use crate::ext::ObjectIdExt;
        let repo = self.repo;
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind())
            .parent_ids()
            .map(move |id| id.attach(repo))
    }

    /// Parse the commit and return the tree object it points to.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let commit = repo.head_commit()?;
    /// let tree = commit.tree()?;
    ///
    /// assert_eq!(tree.id, repo.head_tree_id()?);
    /// # Ok(()) }
    /// ```
    pub fn tree(&self) -> Result<Tree<'repo>, Error> {
        Ok(self.tree_id().or_erased()?.object()?.try_into_tree().or_erased()?)
    }

    /// Parse the commit and return the tree id it points to.
    pub fn tree_id(&self) -> Result<crate::Id<'repo>, gix_object::decode::Error> {
        gix_object::CommitRefIter::from_bytes(&self.data, self.id.kind())
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
    ) -> Result<Option<(std::borrow::Cow<'_, BStr>, gix_object::signature::SignedData<'_>)>, gix_object::decode::Error>
    {
        gix_object::CommitRefIter::signature(&self.data, self.id.kind())
    }

    /// Verify this commit's signature using Git-compatible configuration and external verification programs.
    ///
    /// Returns `Ok(None)` if the commit has no signature. If it is signed, the returned
    /// [`Outcome`](crate::commit::verify::Outcome) describes the signature's format, cryptographic status, trust,
    /// signer identity, and verifier output. A successful call does not necessarily mean that the signature is valid;
    /// use [`Outcome::is_valid()`](crate::commit::verify::Outcome::is_valid) to determine whether Git would accept it.
    #[cfg(feature = "command")]
    pub fn verify_signature(&self) -> Result<Option<crate::commit::verify::Outcome>, crate::commit::verify::Error> {
        crate::commit::verify::verify(self)
    }

    /// Write this commit with a Git-compatible signature added from repository configuration and return the attached commit,
    /// after writing it to the object database.
    ///
    /// An existing signature for the repository's object format is replaced.
    #[cfg(feature = "command")]
    pub fn signed(&self) -> Result<Commit<'repo>, crate::commit::sign::Error> {
        crate::commit::sign::sign(self)
    }
}

impl std::fmt::Debug for Commit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Commit({})", self.id)
    }
}
