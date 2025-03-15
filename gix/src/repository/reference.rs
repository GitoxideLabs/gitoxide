use gix_hash::ObjectId;
use gix_ref::{
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    FullName, PartialNameRef, Target,
};

use crate::{bstr::BString, ext::ReferenceExt, reference, Reference};

/// Obtain and alter references comfortably
impl crate::Repository {
    /// Create a lightweight tag with given `name` (and without `refs/tags/` prefix) pointing to the given `target`, and return it as reference.
    ///
    /// It will be created with `constraint` which is most commonly to [only create it](PreviousValue::MustNotExist)
    /// or to [force overwriting a possibly existing tag](PreviousValue::Any).
    pub fn tag_reference(
        &self,
        name: impl AsRef<str>,
        target: impl Into<ObjectId>,
        constraint: PreviousValue,
    ) -> Result<Reference<'_>, reference::edit::Error> {
        let id = target.into();
        let mut edits = self.edit_reference(RefEdit {
            change: Change::Update {
                log: Default::default(),
                expected: constraint,
                new: Target::Object(id),
            },
            name: format!("refs/tags/{}", name.as_ref()).try_into()?,
            deref: false,
        })?;
        assert_eq!(edits.len(), 1, "reference splits should ever happen");
        let edit = edits.pop().expect("exactly one item");
        Ok(Reference {
            inner: gix_ref::Reference {
                name: edit.name,
                target: id.into(),
                peeled: None,
            },
            repo: self,
        })
    }

    /// Returns the currently set namespace for references, or `None` if it is not set.
    ///
    /// Namespaces allow to partition references, and is configured per `Easy`.
    pub fn namespace(&self) -> Option<&gix_ref::Namespace> {
        self.refs.namespace.as_ref()
    }

    /// Remove the currently set reference namespace and return it, affecting only this `Easy`.
    pub fn clear_namespace(&mut self) -> Option<gix_ref::Namespace> {
        self.refs.namespace.take()
    }

    /// Set the reference namespace to the given value, like `"foo"` or `"foo/bar"`.
    ///
    /// Note that this value is shared across all `Easy…` instances as the value is stored in the shared `Repository`.
    pub fn set_namespace<'a, Name, E>(
        &mut self,
        namespace: Name,
    ) -> Result<Option<gix_ref::Namespace>, gix_validate::reference::name::Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        gix_validate::reference::name::Error: From<E>,
    {
        let namespace = gix_ref::namespace::expand(namespace)?;
        Ok(self.refs.namespace.replace(namespace))
    }

    // TODO: more tests or usage
    /// Create a new reference with `name`, like `refs/heads/branch`, pointing to `target`, adhering to `constraint`
    /// during creation and writing `log_message` into the reflog. Note that a ref-log will be written even if `log_message` is empty.
    ///
    /// The newly created Reference is returned.
    pub fn reference<Name, E>(
        &self,
        name: Name,
        target: impl Into<ObjectId>,
        constraint: PreviousValue,
        log_message: impl Into<BString>,
    ) -> Result<Reference<'_>, reference::edit::Error>
    where
        Name: TryInto<FullName, Error = E>,
        gix_validate::reference::name::Error: From<E>,
    {
        self.reference_inner(
            name.try_into().map_err(gix_validate::reference::name::Error::from)?,
            target.into(),
            constraint,
            log_message.into(),
        )
    }

    fn reference_inner(
        &self,
        name: FullName,
        id: ObjectId,
        constraint: PreviousValue,
        log_message: BString,
    ) -> Result<Reference<'_>, reference::edit::Error> {
        let mut edits = self.edit_reference(RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: log_message,
                },
                expected: constraint,
                new: Target::Object(id),
            },
            name,
            deref: false,
        })?;
        assert_eq!(
            edits.len(),
            1,
            "only one reference can be created, splits aren't possible"
        );

        Ok(gix_ref::Reference {
            name: edits.pop().expect("exactly one edit").name,
            target: Target::Object(id),
            peeled: None,
        }
        .attach(self))
    }

    /// Edit a single reference as described in `edit`, and write reference logs as `log_committer`.
    ///
    /// One or more `RefEdit`s  are returned - symbolic reference splits can cause more edits to be performed. All edits have the previous
    /// reference values set to the ones encountered at rest after acquiring the respective reference's lock.
    pub fn edit_reference(&self, edit: RefEdit) -> Result<Vec<RefEdit>, reference::edit::Error> {
        self.edit_references(Some(edit))
    }

    /// Edit one or more references as described by their `edits`.
    /// Note that one can set the committer name for use in the ref-log by temporarily
    /// [overriding the git-config](crate::Repository::config_snapshot_mut()), or use
    /// [`edit_references_as(committer)`](Self::edit_references_as()) for convenience.
    ///
    /// Returns all reference edits, which might be more than where provided due the splitting of symbolic references, and
    /// whose previous (_old_) values are the ones seen on in storage after the reference was locked.
    pub fn edit_references(
        &self,
        edits: impl IntoIterator<Item = RefEdit>,
    ) -> Result<Vec<RefEdit>, reference::edit::Error> {
        self.edit_references_as(edits, self.committer().transpose()?)
    }

    /// A way to apply reference `edits` similar to [edit_references(…)](Self::edit_references()), but set a specific
    /// `commiter` for use in the reflog. It can be `None` if it's the purpose `edits` are configured to not update the
    /// reference log, or cause a failure otherwise.
    pub fn edit_references_as(
        &self,
        edits: impl IntoIterator<Item = RefEdit>,
        committer: Option<gix_actor::SignatureRef<'_>>,
    ) -> Result<Vec<RefEdit>, reference::edit::Error> {
        let (file_lock_fail, packed_refs_lock_fail) = self.config.lock_timeout()?;
        self.refs
            .transaction()
            .prepare(edits, file_lock_fail, packed_refs_lock_fail)?
            .commit(committer)
            .map_err(Into::into)
    }

    /// Return the repository head, an abstraction to help dealing with the `HEAD` reference.
    ///
    /// The `HEAD` reference can be in various states, for more information, the documentation of [`Head`](crate::Head).
    pub fn head(&self) -> Result<crate::Head<'_>, reference::find::existing::Error> {
        let head = self.find_reference("HEAD")?;
        Ok(match head.inner.target {
            Target::Symbolic(branch) => match self.find_reference(&branch) {
                Ok(r) => crate::head::Kind::Symbolic(r.detach()),
                Err(reference::find::existing::Error::NotFound { .. }) => crate::head::Kind::Unborn(branch),
                Err(err) => return Err(err),
            },
            Target::Object(target) => crate::head::Kind::Detached {
                target,
                peeled: head.inner.peeled,
            },
        }
        .attach(self))
    }

    /// Resolve the `HEAD` reference, follow and peel its target and obtain its object id,
    /// following symbolic references and tags until a commit is found.
    ///
    /// Note that this may fail for various reasons, most notably because the repository
    /// is freshly initialized and doesn't have any commits yet.
    ///
    /// Also note that the returned id is likely to point to a commit, but could also
    /// point to a tree or blob. It won't, however, point to a tag as these are always peeled.
    pub fn head_id(&self) -> Result<crate::Id<'_>, reference::head_id::Error> {
        Ok(self.head()?.into_peeled_id()?)
    }

    /// Return the name to the symbolic reference `HEAD` points to, or `None` if the head is detached.
    ///
    /// The difference to [`head_ref()`](Self::head_ref()) is that the latter requires the reference to exist,
    /// whereas here we merely return a the name of the possibly unborn reference.
    pub fn head_name(&self) -> Result<Option<FullName>, reference::find::existing::Error> {
        Ok(self.head()?.referent_name().map(std::borrow::ToOwned::to_owned))
    }

    /// Return the reference that `HEAD` points to, or `None` if the head is detached or unborn.
    pub fn head_ref(&self) -> Result<Option<Reference<'_>>, reference::find::existing::Error> {
        Ok(self.head()?.try_into_referent())
    }

    /// Return the commit object the `HEAD` reference currently points to after peeling it fully,
    /// following symbolic references and tags until a commit is found.
    ///
    /// Note that this may fail for various reasons, most notably because the repository
    /// is freshly initialized and doesn't have any commits yet. It could also fail if the
    /// head does not point to a commit.
    pub fn head_commit(&self) -> Result<crate::Commit<'_>, reference::head_commit::Error> {
        Ok(self.head()?.peel_to_commit_in_place()?)
    }

    /// Return the tree id the `HEAD` reference currently points to after peeling it fully,
    /// following symbolic references and tags until a commit is found.
    ///
    /// Note that this may fail for various reasons, most notably because the repository
    /// is freshly initialized and doesn't have any commits yet. It could also fail if the
    /// head does not point to a commit.
    pub fn head_tree_id(&self) -> Result<crate::Id<'_>, reference::head_tree_id::Error> {
        Ok(self.head_commit()?.tree_id()?)
    }

    /// Like [`Self::head_tree_id()`], but will return an empty tree hash if the repository HEAD is unborn.
    pub fn head_tree_id_or_empty(&self) -> Result<crate::Id<'_>, reference::head_tree_id::Error> {
        self.head_tree_id().or_else(|err| {
            if let reference::head_tree_id::Error::HeadCommit(reference::head_commit::Error::PeelToCommit(
                crate::head::peel::to_commit::Error::PeelToObject(crate::head::peel::to_object::Error::Unborn {
                    ..
                }),
            )) = err
            {
                Ok(self.empty_tree().id())
            } else {
                Err(err)
            }
        })
    }

    /// Return the tree object the `HEAD^{tree}` reference currently points to after peeling it fully,
    /// following symbolic references and tags until a tree is found.
    ///
    /// Note that this may fail for various reasons, most notably because the repository
    /// is freshly initialized and doesn't have any commits yet. It could also fail if the
    /// head does not point to a tree, unlikely but possible.
    pub fn head_tree(&self) -> Result<crate::Tree<'_>, reference::head_tree::Error> {
        Ok(self.head_commit()?.tree()?)
    }

    /// Find the reference with the given partial or full `name`, like `main`, `HEAD`, `heads/branch` or `origin/other`,
    /// or return an error if it wasn't found.
    ///
    /// Consider [`try_find_reference(…)`](crate::Repository::try_find_reference()) if the reference might not exist
    /// without that being considered an error.
    pub fn find_reference<'a, Name, E>(&self, name: Name) -> Result<Reference<'_>, reference::find::existing::Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E> + Clone,
        gix_ref::file::find::Error: From<E>,
    {
        // TODO: is there a way to just pass `partial_name` to `try_find_reference()`? Compiler freaks out then
        //       as it still wants to see `E` there, not `Infallible`.
        let partial_name = name
            .clone()
            .try_into()
            .map_err(|err| reference::find::Error::Find(gix_ref::file::find::Error::from(err)))?;
        self.try_find_reference(name)?
            .ok_or_else(|| reference::find::existing::Error::NotFound {
                name: partial_name.to_owned(),
            })
    }

    /// Return a platform for iterating references.
    ///
    /// Common kinds of iteration are [all](crate::reference::iter::Platform::all()) or [prefixed](crate::reference::iter::Platform::prefixed())
    /// references.
    pub fn references(&self) -> Result<reference::iter::Platform<'_>, reference::iter::Error> {
        Ok(reference::iter::Platform {
            platform: self.refs.iter()?,
            repo: self,
        })
    }

    /// Try to find the reference named `name`, like `main`, `heads/branch`, `HEAD` or `origin/other`, and return it.
    ///
    /// Otherwise return `None` if the reference wasn't found.
    /// If the reference is expected to exist, use [`find_reference()`](crate::Repository::find_reference()).
    pub fn try_find_reference<'a, Name, E>(&self, name: Name) -> Result<Option<Reference<'_>>, reference::find::Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        gix_ref::file::find::Error: From<E>,
    {
        match self.refs.try_find(name) {
            Ok(r) => match r {
                Some(r) => Ok(Some(Reference::from_ref(r, self))),
                None => Ok(None),
            },
            Err(err) => Err(err.into()),
        }
    }
}
