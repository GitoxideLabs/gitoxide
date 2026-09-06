use crate::object::tree::EntryRef;
use crate::{bstr::BStr, ext::ObjectIdExt, object::tree::Entry};

/// Access
impl<'repo> Entry<'repo> {
    /// The kind of object to which `oid` is pointing to.
    pub fn mode(&self) -> gix_object::tree::EntryMode {
        self.inner.mode
    }

    /// The name of the file in the parent tree.
    pub fn filename(&self) -> &BStr {
        self.inner.filename.as_ref()
    }

    /// Return the object id of the entry.
    pub fn id(&self) -> crate::Id<'repo> {
        self.inner.oid.attach(self.repo)
    }

    /// Return the object this entry points to.
    pub fn object(&self) -> Result<crate::Object<'repo>, crate::Error> {
        self.id().object()
    }

    /// Return the plain object id of this entry, without access to the repository.
    pub fn oid(&self) -> &gix_hash::oid {
        &self.inner.oid
    }

    /// Return the plain object id of this entry, without access to the repository.
    pub fn object_id(&self) -> gix_hash::ObjectId {
        self.inner.oid
    }
}

/// Consuming
impl Entry<'_> {
    /// Return the contained object.
    pub fn detach(self) -> gix_object::tree::Entry {
        self.inner
    }
}

impl<'repo, 'a> EntryRef<'repo, 'a> {
    /// The kind of object to which [`id()`][Self::id()] is pointing.
    pub fn mode(&self) -> gix_object::tree::EntryMode {
        self.inner.mode
    }

    /// The kind of object to which [`id()`][Self::id()] is pointing, as shortcut to [self.mode().kind()](Self::mode()).
    pub fn kind(&self) -> gix_object::tree::EntryKind {
        self.inner.mode.kind()
    }

    /// The name of the file in the parent tree.
    pub fn filename(&self) -> &gix_object::bstr::BStr {
        self.inner.filename
    }

    /// Return the entries id, connected to the underlying repository.
    pub fn id(&self) -> crate::Id<'repo> {
        crate::Id::from_id(self.inner.oid, self.repo)
    }

    /// Return the plain object id of this entry, without access to the repository.
    pub fn oid(&self) -> &gix_hash::oid {
        self.inner.oid
    }

    /// Return the object this entry points to.
    pub fn object(&self) -> Result<crate::Object<'repo>, crate::Error> {
        self.id().object()
    }

    /// Return the plain object id of this entry, without access to the repository.
    pub fn object_id(&self) -> gix_hash::ObjectId {
        self.inner.oid.to_owned()
    }

    /// Detach the repository from this instance.
    pub fn detach(&self) -> gix_object::tree::EntryRef<'a> {
        self.inner
    }

    /// Create an instance that doesn't bind to a buffer anymore (but that still contains a repository reference).
    pub fn to_owned(&self) -> Entry<'repo> {
        Entry {
            inner: self.inner.into(),
            repo: self.repo,
        }
    }
}

impl std::fmt::Display for EntryRef<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>6o} {:>6} {}\t{}",
            self.mode(),
            self.mode().as_str(),
            self.id().shorten_or_id(),
            self.filename()
        )
    }
}
