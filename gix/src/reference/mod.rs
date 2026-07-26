//!
#![allow(clippy::empty_docs)]

use gix_ref::file::ReferenceExt;

use crate::{Blob, Commit, Id, Object, Reference, Tag, Tree};

pub mod iter;
///
pub mod remote;

mod errors;
pub use errors::{edit, find, follow, head_commit, head_id, head_tree, head_tree_id, peel};

use crate::ext::ObjectIdExt;

pub mod log;

pub use gix_ref::{Category, Kind};

/// Access
impl<'repo> Reference<'repo> {
    /// Returns the attached id we point to, or `None` if this is a symbolic ref.
    pub fn try_id(&self) -> Option<Id<'repo>> {
        match self.inner.target {
            gix_ref::Target::Symbolic(_) => None,
            gix_ref::Target::Object(oid) => oid.to_owned().attach(self.repo).into(),
        }
    }

    /// Returns the attached id we point to, or panic if this is a symbolic ref.
    pub fn id(&self) -> Id<'repo> {
        self.try_id()
            .expect("BUG: tries to obtain object id from symbolic target")
    }

    /// Return the target to which this reference points to.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let branch = repo.find_reference("main")?;
    ///
    /// assert_eq!(branch.target().try_id().expect("direct target"), repo.head_id()?.as_ref());
    /// # Ok(()) }
    /// ```
    pub fn target(&self) -> gix_ref::TargetRef<'_> {
        self.inner.target.to_ref()
    }

    /// Return the reference's full name.
    pub fn name(&self) -> &gix_ref::FullNameRef {
        self.inner.name.as_ref()
    }

    /// Turn this instances into a stand-alone reference.
    pub fn detach(self) -> gix_ref::Reference {
        self.inner
    }
}

impl std::fmt::Debug for Reference<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
    }
}

impl<'repo> Reference<'repo> {
    pub(crate) fn from_ref(reference: gix_ref::Reference, repo: &'repo crate::Repository) -> Self {
        Reference { inner: reference, repo }
    }
}

/// Peeling
impl<'repo> Reference<'repo> {
    /// Follow all symbolic targets this reference might point to and peel all annotated tags
    /// to their first non-tag target, and return it.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following
    /// the chain of symbolic refs and annotated tags.
    #[deprecated = "Use `peel_to_id()` instead"]
    pub fn peel_to_id_in_place(&mut self) -> Result<Id<'repo>, peel::Error> {
        let oid = self.inner.peel_to_id(&self.repo.refs, &self.repo.objects)?;
        Ok(Id::from_id(oid, self.repo))
    }

    /// Follow all symbolic targets this reference might point to and peel all annotated tags
    /// to their first non-tag target, and return it.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following
    /// the chain of symbolic refs and annotated tags.
    ///
    /// Note that this method mutates `self` in place if it does not already point to a
    /// non-symbolic object.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("base")?)?;
    /// let mut tag = repo.find_reference("b-tag")?;
    /// let tag_object_id = tag.id();
    /// let peeled_id = tag.peel_to_id()?;
    ///
    /// assert!(repo.find_tag(tag_object_id).is_ok(), "the ref initially points to a tag object");
    /// assert_ne!(peeled_id, tag_object_id);
    /// assert_eq!(peeled_id, repo.find_reference("b")?.id());
    /// # Ok(()) }
    /// ```
    pub fn peel_to_id(&mut self) -> Result<Id<'repo>, peel::Error> {
        let oid = self.inner.peel_to_id(&self.repo.refs, &self.repo.objects)?;
        Ok(Id::from_id(oid, self.repo))
    }

    /// Follow all symbolic targets this reference might point to and peel all annotated tags
    /// to their first non-tag target, and return it, reusing the `packed` buffer if available.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following
    /// the chain of symbolic refs and annotated tags.
    #[deprecated = "Use `peel_to_id_packed()` instead"]
    pub fn peel_to_id_in_place_packed(
        &mut self,
        packed: Option<&gix_ref::packed::Buffer>,
    ) -> Result<Id<'repo>, peel::Error> {
        let oid = self
            .inner
            .peel_to_id_packed(&self.repo.refs, &self.repo.objects, packed)?;
        Ok(Id::from_id(oid, self.repo))
    }

    /// Follow all symbolic targets this reference might point to and peel all annotated tags
    /// to their first non-tag target, and return it, reusing the `packed` buffer if available.
    ///
    /// This is useful to learn where this reference is ultimately pointing to after following
    /// the chain of symbolic refs and annotated tags.
    ///
    /// Note that this method mutates `self` in place if it does not already point to a
    /// non-symbolic object.
    pub fn peel_to_id_packed(&mut self, packed: Option<&gix_ref::packed::Buffer>) -> Result<Id<'repo>, peel::Error> {
        let oid = self
            .inner
            .peel_to_id_packed(&self.repo.refs, &self.repo.objects, packed)?;
        Ok(Id::from_id(oid, self.repo))
    }

    /// Similar to [`peel_to_id()`](Reference::peel_to_id()), but consumes this instance.
    pub fn into_fully_peeled_id(mut self) -> Result<Id<'repo>, peel::Error> {
        self.peel_to_id()
    }

    /// Follow this reference's target until it points at an object directly, and peel that object until
    /// its type matches the given `kind`. It's an error to try to peel to a kind that this ref doesn't point to.
    ///
    /// Note that this ref will point to the first target object afterward, which may be a tag. This is different
    /// from [`peel_to_id()`](Self::peel_to_id()) where it will point to the first non-tag object.
    ///
    /// Note that `git2::Reference::peel` does not "peel in place", but returns a new object
    /// instead.
    #[doc(alias = "peel", alias = "git2")]
    pub fn peel_to_kind(&mut self, kind: gix_object::Kind) -> Result<Object<'repo>, peel::to_kind::Error> {
        let packed = self.repo.refs.cached_packed_buffer().map_err(|err| {
            gix_error::Error::from_error(gix_ref::peel::to_object::Error::Follow(
                file::find::existing::Error::Find(file::find::Error::PackedOpen(err)),
            ))
        })?;
        self.peel_to_kind_packed(kind, packed.as_ref().map(|p| &***p))
    }

    /// Peel this ref until the first commit.
    ///
    /// For details, see [`peel_to_kind`()](Self::peel_to_kind()).
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let mut branch = repo.find_reference("main")?;
    /// let commit = branch.peel_to_commit()?;
    ///
    /// assert_eq!(commit.message_raw()?, "c2\n");
    /// # Ok(()) }
    /// ```
    pub fn peel_to_commit(&mut self) -> Result<Commit<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Commit)?.into_commit())
    }

    /// Peel this ref until the first annotated tag.
    ///
    /// For details, see [`peel_to_kind`()](Self::peel_to_kind()).
    pub fn peel_to_tag(&mut self) -> Result<Tag<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Tag)?.into_tag())
    }

    /// Peel this ref until the first tree.
    ///
    /// For details, see [`peel_to_kind`()](Self::peel_to_kind()).
    pub fn peel_to_tree(&mut self) -> Result<Tree<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Tree)?.into_tree())
    }

    /// Peel this ref until it points to a blob. Note that this is highly uncommon to happen
    /// as it would require an annotated tag to point to a blob, instead of a commit.
    ///
    /// For details, see [`peel_to_kind`()](Self::peel_to_kind()).
    pub fn peel_to_blob(&mut self) -> Result<Blob<'repo>, peel::to_kind::Error> {
        Ok(self.peel_to_kind(gix_object::Kind::Blob)?.into_blob())
    }

    /// Like [`peel_to_kind()`](Self::peel_to_kind), but allows to provide `packed` for best possible performance
    /// when peeling many refs.
    pub fn peel_to_kind_packed(
        &mut self,
        kind: gix_object::Kind,
        packed: Option<&gix_ref::packed::Buffer>,
    ) -> Result<Object<'repo>, peel::to_kind::Error> {
        let target = self
            .inner
            .follow_to_object_packed(&self.repo.refs, packed)
            .map_err(gix_error::Error::from_error)?
            .attach(self.repo);
        target
            .object()
            .map_err(gix_error::Error::from_error)?
            .peel_to_kind(kind)
    }

    /// Follow all symbolic references we point to up to the first object, which is typically (but not always) a tag,
    /// returning its id.
    /// After this call, this ref will be pointing to an object directly, but may still not consider itself 'peeled' unless
    /// a symbolic target ref was looked up from packed-refs.
    #[doc(alias = "resolve", alias = "git2")]
    pub fn follow_to_object(&mut self) -> Result<Id<'repo>, follow::to_object::Error> {
        let packed = self.repo.refs.cached_packed_buffer().map_err(|err| {
            gix_error::Error::from_error(gix_ref::peel::to_object::Error::Follow(
                file::find::existing::Error::Find(file::find::Error::PackedOpen(err)),
            ))
        })?;
        self.follow_to_object_packed(packed.as_ref().map(|p| &***p))
    }

    /// Like [`follow_to_object`](Self::follow_to_object), but can be used for repeated calls as it won't
    /// look up `packed` each time, but can reuse it instead.
    #[doc(alias = "resolve", alias = "git2")]
    pub fn follow_to_object_packed(
        &mut self,
        packed: Option<&gix_ref::packed::Buffer>,
    ) -> Result<Id<'repo>, follow::to_object::Error> {
        Ok(self
            .inner
            .follow_to_object_packed(&self.repo.refs, packed)
            .map_err(gix_error::Error::from_error)?
            .attach(self.repo))
    }

    /// Follow this symbolic reference one level and return the ref it refers to.
    ///
    /// Returns `None` if this is not a symbolic reference, hence the leaf of the chain.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let head = repo.find_reference("HEAD")?;
    /// let branch = head.follow().expect("symbolic")?;
    ///
    /// assert_eq!(branch.name().as_bstr(), "refs/heads/main");
    /// # Ok(()) }
    /// ```
    pub fn follow(&self) -> Option<Result<Reference<'repo>, gix_ref::file::find::existing::Error>> {
        self.inner.follow(&self.repo.refs).map(|res| {
            res.map(|r| Reference {
                inner: r,
                repo: self.repo,
            })
        })
    }
}

mod edits;
pub use edits::{delete, set_target_id};
use gix_ref::file;
