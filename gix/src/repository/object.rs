#![allow(clippy::result_large_err)]
use std::ops::DerefMut;

use gix_hash::ObjectId;
use gix_object::{Exists, Find, FindExt, Write};
use gix_odb::{Header, HeaderExt};
use gix_ref::{
    transaction::{LogChange, PreviousValue, RefLog},
    FullName,
};
use smallvec::SmallVec;

use crate::{commit, ext::ObjectIdExt, object, tag, Blob, Commit, Id, Object, Reference, Tag, Tree};

/// Tree editing
#[cfg(feature = "tree-editor")]
impl crate::Repository {
    /// Return an editor for adjusting the tree at `id`.
    ///
    /// This can be the [empty tree id](ObjectId::empty_tree) to build a tree from scratch.
    #[doc(alias = "treebuilder", alias = "git2")]
    pub fn edit_tree(
        &self,
        id: impl Into<ObjectId>,
    ) -> Result<object::tree::Editor<'_>, crate::repository::edit_tree::Error> {
        let tree = self.find_tree(id)?;
        Ok(tree.edit()?)
    }
}

/// Find objects of various kins
impl crate::Repository {
    /// Find the object with `id` in the object database or return an error if it could not be found.
    ///
    /// There are various legitimate reasons for an object to not be present, which is why
    /// [`try_find_object(…)`][crate::Repository::try_find_object()] might be preferable instead.
    ///
    /// # Performance Note
    ///
    /// In order to get the kind of the object, is must be fully decoded from storage if it is packed with deltas.
    /// Loose object could be partially decoded, even though that's not implemented.
    pub fn find_object(&self, id: impl Into<ObjectId>) -> Result<Object<'_>, object::find::existing::Error> {
        let id = id.into();
        if id == ObjectId::empty_tree(self.object_hash()) {
            return Ok(Object {
                id,
                kind: gix_object::Kind::Tree,
                data: Vec::new(),
                repo: self,
            });
        }
        if id == ObjectId::empty_blob(self.object_hash()) {
            return Ok(Object {
                id,
                kind: gix_object::Kind::Blob,
                data: Vec::new(),
                repo: self,
            });
        }
        let mut buf = self.free_buf();
        let kind = self.objects.find(&id, &mut buf)?.kind;
        Ok(Object::from_data(id, kind, buf, self))
    }

    /// Find a commit with `id` or fail if there was no object or the object wasn't a commit.
    pub fn find_commit(
        &self,
        id: impl Into<ObjectId>,
    ) -> Result<Commit<'_>, object::find::existing::with_conversion::Error> {
        Ok(self.find_object(id)?.try_into_commit()?)
    }

    /// Find a tree with `id` or fail if there was no object or the object wasn't a tree.
    pub fn find_tree(
        &self,
        id: impl Into<ObjectId>,
    ) -> Result<Tree<'_>, object::find::existing::with_conversion::Error> {
        Ok(self.find_object(id)?.try_into_tree()?)
    }

    /// Find an annotated tag with `id` or fail if there was no object or the object wasn't a tag.
    pub fn find_tag(&self, id: impl Into<ObjectId>) -> Result<Tag<'_>, object::find::existing::with_conversion::Error> {
        Ok(self.find_object(id)?.try_into_tag()?)
    }

    /// Find a blob with `id` or fail if there was no object or the object wasn't a blob.
    pub fn find_blob(
        &self,
        id: impl Into<ObjectId>,
    ) -> Result<Blob<'_>, object::find::existing::with_conversion::Error> {
        Ok(self.find_object(id)?.try_into_blob()?)
    }

    /// Obtain information about an object without fully decoding it, or fail if the object doesn't exist.
    ///
    /// Note that despite being cheaper than [`Self::find_object()`], there is still some effort traversing delta-chains.
    #[doc(alias = "read_header", alias = "git2")]
    pub fn find_header(&self, id: impl Into<ObjectId>) -> Result<gix_odb::find::Header, object::find::existing::Error> {
        let id = id.into();
        if id == ObjectId::empty_tree(self.object_hash()) {
            return Ok(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Tree,
                size: 0,
            });
        }
        if id == ObjectId::empty_blob(self.object_hash()) {
            return Ok(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Blob,
                size: 0,
            });
        }
        self.objects.header(id)
    }

    /// Return `true` if `id` exists in the object database.
    ///
    /// # Performance
    ///
    /// This method can be slow if the underlying [object database](crate::Repository::objects) has
    /// an unsuitable [RefreshMode](gix_odb::store::RefreshMode) and `id` is not likely to exist.
    /// Use [`repo.objects.refresh_never()`](gix_odb::store::Handle::refresh_never) to avoid expensive
    /// IO-bound refreshes if an object wasn't found.
    #[doc(alias = "exists", alias = "git2")]
    pub fn has_object(&self, id: impl AsRef<gix_hash::oid>) -> bool {
        let id = id.as_ref();
        if id.to_owned().is_empty_tree() || id.to_owned().is_empty_blob() {
            true
        } else {
            self.objects.exists(id)
        }
    }

    /// Obtain information about an object without fully decoding it, or `None` if the object doesn't exist.
    ///
    /// Note that despite being cheaper than [`Self::try_find_object()`], there is still some effort traversing delta-chains.
    pub fn try_find_header(
        &self,
        id: impl Into<ObjectId>,
    ) -> Result<Option<gix_odb::find::Header>, object::find::Error> {
        let id = id.into();
        if id == ObjectId::empty_tree(self.object_hash()) {
            return Ok(Some(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Tree,
                size: 0,
            }));
        }
        if id == ObjectId::empty_blob(self.object_hash()) {
            return Ok(Some(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Blob,
                size: 0,
            }));
        }
        self.objects.try_header(&id).map_err(Into::into)
    }

    /// Try to find the object with `id` or return `None` if it wasn't found.
    pub fn try_find_object(&self, id: impl Into<ObjectId>) -> Result<Option<Object<'_>>, object::find::Error> {
        let id = id.into();
        if id == ObjectId::empty_tree(self.object_hash()) {
            return Ok(Some(Object {
                id,
                kind: gix_object::Kind::Tree,
                data: Vec::new(),
                repo: self,
            }));
        }
        if id == ObjectId::empty_blob(self.object_hash()) {
            return Ok(Some(Object {
                id,
                kind: gix_object::Kind::Blob,
                data: Vec::new(),
                repo: self,
            }));
        }

        let mut buf = self.free_buf();
        match self.objects.try_find(&id, &mut buf)? {
            Some(obj) => {
                let kind = obj.kind;
                Ok(Some(Object::from_data(id, kind, buf, self)))
            }
            None => Ok(None),
        }
    }
}

/// Write objects of any type.
impl crate::Repository {
    /// Write the given object into the object database and return its object id.
    ///
    /// Note that we hash the object in memory to avoid storing objects that are already present. That way,
    /// we avoid writing duplicate objects using slow disks that will eventually have to be garbage collected.
    pub fn write_object(&self, object: impl gix_object::WriteTo) -> Result<Id<'_>, object::write::Error> {
        let mut buf = self.empty_reusable_buffer();
        object
            .write_to(buf.deref_mut())
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>)?;

        self.write_object_inner(&buf, object.kind())
    }

    fn write_object_inner(&self, buf: &[u8], kind: gix_object::Kind) -> Result<Id<'_>, object::write::Error> {
        let oid = gix_object::compute_hash(self.object_hash(), kind, buf)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
        if self.objects.exists(&oid) {
            return Ok(oid.attach(self));
        }

        self.objects
            .write_buf(kind, buf)
            .map(|oid| oid.attach(self))
            .map_err(Into::into)
    }

    /// Write a blob from the given `bytes`.
    ///
    /// We avoid writing duplicate objects to slow disks that will eventually have to be garbage collected by
    /// pre-hashing the data, and checking if the object is already present.
    pub fn write_blob(&self, bytes: impl AsRef<[u8]>) -> Result<Id<'_>, object::write::Error> {
        let bytes = bytes.as_ref();
        let oid = gix_object::compute_hash(self.object_hash(), gix_object::Kind::Blob, bytes)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
        if self.objects.exists(&oid) {
            return Ok(oid.attach(self));
        }
        self.objects
            .write_buf(gix_object::Kind::Blob, bytes)
            .map_err(Into::into)
            .map(|oid| oid.attach(self))
    }

    /// Write a blob from the given `Read` implementation.
    ///
    /// Note that we hash the object in memory to avoid storing objects that are already present. That way,
    /// we avoid writing duplicate objects using slow disks that will eventually have to be garbage collected.
    ///
    /// If that is prohibitive, use the object database directly.
    pub fn write_blob_stream(&self, mut bytes: impl std::io::Read) -> Result<Id<'_>, object::write::Error> {
        let mut buf = self.empty_reusable_buffer();
        std::io::copy(&mut bytes, buf.deref_mut())
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;

        self.write_blob_stream_inner(&buf)
    }

    fn write_blob_stream_inner(&self, buf: &[u8]) -> Result<Id<'_>, object::write::Error> {
        let oid = gix_object::compute_hash(self.object_hash(), gix_object::Kind::Blob, buf)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
        if self.objects.exists(&oid) {
            return Ok(oid.attach(self));
        }

        self.objects
            .write_buf(gix_object::Kind::Blob, buf)
            .map_err(Into::into)
            .map(|oid| oid.attach(self))
    }
}

/// Create commits and tags
impl crate::Repository {
    /// Create a tag reference named `name` (without `refs/tags/` prefix) pointing to a newly created tag object
    /// which in turn points to `target` and return the newly created reference.
    ///
    /// It will be created with `constraint` which is most commonly to [only create it][PreviousValue::MustNotExist]
    /// or to [force overwriting a possibly existing tag](PreviousValue::Any).
    pub fn tag(
        &self,
        name: impl AsRef<str>,
        target: impl AsRef<gix_hash::oid>,
        target_kind: gix_object::Kind,
        tagger: Option<gix_actor::SignatureRef<'_>>,
        message: impl AsRef<str>,
        constraint: PreviousValue,
    ) -> Result<Reference<'_>, tag::Error> {
        let tag = gix_object::Tag {
            target: target.as_ref().into(),
            target_kind,
            name: name.as_ref().into(),
            tagger: tagger.map(|t| t.to_owned()).transpose()?,
            message: message.as_ref().into(),
            pgp_signature: None,
        };
        let tag_id = self.write_object(&tag)?;
        self.tag_reference(name, tag_id, constraint).map_err(Into::into)
    }

    /// Similar to [`commit(…)`](crate::Repository::commit()), but allows to create the commit with `committer` and `author` specified.
    ///
    /// This forces setting the commit time and author time by hand. Note that typically, committer and author are the same.
    pub fn commit_as<'a, 'c, Name, E>(
        &self,
        committer: impl Into<gix_actor::SignatureRef<'c>>,
        author: impl Into<gix_actor::SignatureRef<'a>>,
        reference: Name,
        message: impl AsRef<str>,
        tree: impl Into<ObjectId>,
        parents: impl IntoIterator<Item = impl Into<ObjectId>>,
    ) -> Result<Id<'_>, commit::Error>
    where
        Name: TryInto<FullName, Error = E>,
        commit::Error: From<E>,
    {
        self.commit_as_inner(
            committer.into(),
            author.into(),
            reference.try_into()?,
            message.as_ref(),
            tree.into(),
            parents.into_iter().map(Into::into).collect(),
        )
    }

    fn commit_as_inner(
        &self,
        committer: gix_actor::SignatureRef<'_>,
        author: gix_actor::SignatureRef<'_>,
        reference: FullName,
        message: &str,
        tree: ObjectId,
        parents: SmallVec<[ObjectId; 1]>,
    ) -> Result<Id<'_>, commit::Error> {
        use gix_ref::{
            transaction::{Change, RefEdit},
            Target,
        };

        // TODO: possibly use CommitRef to save a few allocations (but will have to allocate for object ids anyway.
        //       This can be made vastly more efficient though if we wanted to, so we lie in the API
        let commit = gix_object::Commit {
            message: message.into(),
            tree,
            author: author.into(),
            committer: committer.into(),
            encoding: None,
            parents,
            extra_headers: Default::default(),
        };

        let commit_id = self.write_object(&commit)?;
        self.edit_references_as(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: crate::reference::log::message(
                            "commit",
                            commit.message.as_ref(),
                            commit.parents.len(),
                        ),
                    },
                    expected: match commit.parents.first().map(|p| Target::Object(*p)) {
                        Some(previous) => {
                            if reference.as_bstr() == "HEAD" {
                                PreviousValue::MustExistAndMatch(previous)
                            } else {
                                PreviousValue::ExistingMustMatch(previous)
                            }
                        }
                        None => PreviousValue::MustNotExist,
                    },
                    new: Target::Object(commit_id.inner),
                },
                name: reference,
                deref: true,
            }),
            Some(committer),
        )?;
        Ok(commit_id)
    }

    /// Create a new commit object with `message` referring to `tree` with `parents`, and point `reference`
    /// to it. The commit is written without message encoding field, which can be assumed to be UTF-8.
    /// `author` and `committer` fields are pre-set from the configuration, which can be altered
    /// [temporarily](crate::Repository::config_snapshot_mut()) before the call if required.
    ///
    /// `reference` will be created if it doesn't exist, and can be `"HEAD"` to automatically write-through to the symbolic reference
    /// that `HEAD` points to if it is not detached. For this reason, detached head states cannot be created unless the `HEAD` is detached
    /// already. The reflog will be written as canonical git would do, like `<operation> (<detail>): <summary>`.
    ///
    /// The first parent id in `parents` is expected to be the current target of `reference` and the operation will fail if it is not.
    /// If there is no parent, the `reference` is expected to not exist yet.
    ///
    /// The method fails immediately if a `reference` lock can't be acquired.
    ///
    /// ### Writing a commit without `reference` update
    ///
    /// If the reference shouldn't be updated, use [`Self::write_object()`] along with a newly created [`crate::objs::Object`] whose fields
    /// can be fully defined.
    pub fn commit<Name, E>(
        &self,
        reference: Name,
        message: impl AsRef<str>,
        tree: impl Into<ObjectId>,
        parents: impl IntoIterator<Item = impl Into<ObjectId>>,
    ) -> Result<Id<'_>, commit::Error>
    where
        Name: TryInto<FullName, Error = E>,
        commit::Error: From<E>,
    {
        let author = self.author().ok_or(commit::Error::AuthorMissing)??;
        let committer = self.committer().ok_or(commit::Error::CommitterMissing)??;
        self.commit_as(committer, author, reference, message, tree, parents)
    }

    /// Return an empty tree object, suitable for [getting changes](Tree::changes()).
    ///
    /// Note that the returned object is special and doesn't necessarily physically exist in the object database.
    /// This means that this object can be used in an uninitialized, empty repository which would report to have no objects at all.
    pub fn empty_tree(&self) -> Tree<'_> {
        self.find_object(ObjectId::empty_tree(self.object_hash()))
            .expect("always present")
            .into_tree()
    }

    /// Return an empty blob object.
    ///
    /// Note that the returned object is special and doesn't necessarily physically exist in the object database.
    /// This means that this object can be used in an uninitialized, empty repository which would report to have no objects at all.
    pub fn empty_blob(&self) -> Blob<'_> {
        Blob {
            id: gix_hash::ObjectId::empty_blob(self.object_hash()),
            data: Vec::new(),
            repo: self,
        }
    }
}
