use crate::{Id, Reference, bstr::BStr, ext::ReferenceExt, revision::Spec};

///
pub mod parse;

mod impls {
    use std::ops::{Deref, DerefMut};

    use crate::revision::Spec;

    impl Deref for Spec<'_> {
        type Target = gix_revision::Spec;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl DerefMut for Spec<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl PartialEq for Spec<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.inner == other.inner
        }
    }

    impl Eq for Spec<'_> {}
}

/// Initialization
impl<'repo> Spec<'repo> {
    /// Create a single specification which points to `id`.
    pub fn from_id(id: Id<'repo>) -> Self {
        Spec {
            inner: gix_revision::Spec::Include(id.inner),
            path: None,
            repo: id.repo,
            first_ref: None,
            second_ref: None,
        }
    }
}

/// Access
impl<'repo> Spec<'repo> {
    /// Detach the `Repository` from this instance, leaving only plain data that can be moved freely and serialized.
    pub fn detach(self) -> gix_revision::Spec {
        self.inner
    }

    /// Some revision specifications leave information about references which are returned as `(from-ref, to-ref)` here, e.g.
    /// `HEAD@{-1}..main` might be `(Some(refs/heads/previous-branch), Some(refs/heads/main))`,
    /// or `@` returns `(Some(refs/heads/main), None)`.
    pub fn into_references(self) -> (Option<Reference<'repo>>, Option<Reference<'repo>>) {
        let repo = self.repo;
        (
            self.first_ref.map(|r| r.attach(repo)),
            self.second_ref.map(|r| r.attach(repo)),
        )
    }

    /// Return the path encountered in specs like `@:<path>` or `:<path>`, along with the kind of object it represents.
    ///
    /// Note that there can only be one as paths always terminates further revspec parsing.
    pub fn path_and_mode(&self) -> Option<(&BStr, gix_object::tree::EntryMode)> {
        self.path.as_ref().map(|(p, mode)| (p.as_ref(), *mode))
    }

    /// Return the name of the first reference we encountered while resolving the rev-spec, or `None` if a short hash
    /// was used. For example, `@` might yield `Some(HEAD)`, but `abcd` yields `None`.
    pub fn first_reference(&self) -> Option<&gix_ref::Reference> {
        self.first_ref.as_ref()
    }

    /// Return the name of the second reference we encountered while resolving the rev-spec, or `None` if a short hash
    /// was used or there was no second reference. For example, `..@` might yield `Some(HEAD)`, but `..abcd` or `@`
    /// yields `None`.
    pub fn second_reference(&self) -> Option<&gix_ref::Reference> {
        self.second_ref.as_ref()
    }

    /// Return the single included object represented by this instance, or `None` if it is a range of any kind.
    pub fn single(&self) -> Option<Id<'repo>> {
        match self.inner {
            gix_revision::Spec::Include(id) | gix_revision::Spec::ExcludeParents(id) => {
                Id::from_id(id, self.repo).into()
            }
            gix_revision::Spec::Exclude(_)
            | gix_revision::Spec::Range { .. }
            | gix_revision::Spec::Merge { .. }
            | gix_revision::Spec::IncludeOnlyParents { .. } => None,
        }
    }
}
