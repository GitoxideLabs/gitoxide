//!
#![allow(clippy::empty_docs)]

use std::convert::Infallible;

/// An empty array of a type usable with the `gix::easy` API to help declaring no parents should be used
pub const NO_PARENT_IDS: [gix_hash::ObjectId; 0] = [];

/// The error returned by [`commit(…)`](crate::Repository::commit()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ParseTime(#[from] crate::config::time::Error),
    #[error("Committer identity is not configured")]
    CommitterMissing,
    #[error("Author identity is not configured")]
    AuthorMissing,
    #[error(transparent)]
    ReferenceNameValidation(#[from] gix_ref::name::Error),
    #[error(transparent)]
    WriteObject(#[from] crate::object::write::Error),
    #[error(transparent)]
    ReferenceEdit(#[from] crate::reference::edit::Error),
}

impl From<std::convert::Infallible> for Error {
    fn from(_value: Infallible) -> Self {
        unreachable!("cannot be invoked")
    }
}

#[cfg(feature = "revision")]
mod named_refs {
    use std::borrow::Cow;

    use crate::bstr::{BStr, BString, ByteSlice};
    use gix_hash::ObjectId;

    use crate::Repository;

    /// A selector to choose what kind of references should contribute to names.
    #[derive(Default, Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash)]
    pub enum SelectRef {
        /// Only use annotated tags for names.
        #[default]
        AnnotatedTags,
        /// Use all tags for names, annotated or plain reference.
        AllTags,
        /// Use all references, including local branch names.
        AllRefs,
    }

    pub(crate) struct Candidate {
        pub(crate) peeled_id: ObjectId,
        pub(crate) name: Cow<'static, BStr>,
        pub(crate) name_rev_name: Cow<'static, BStr>,
        pub(crate) priority: u8,
        pub(crate) taggerdate: i64,
        pub(crate) from_tag: bool,
        pub(crate) deref: bool,
    }

    pub(crate) struct Filters<'a> {
        pub(crate) include: &'a [BString],
        pub(crate) exclude: &'a [BString],
    }

    impl Filters<'_> {
        pub(crate) const NONE: Filters<'static> = Filters {
            include: &[],
            exclude: &[],
        };

        fn allows(&self, name: &BStr) -> Option<bool> {
            if self
                .exclude
                .iter()
                .any(|pattern| subpath_match(name, pattern.as_bstr()).is_some())
            {
                return None;
            }

            if self.include.is_empty() {
                return Some(false);
            }

            let mut matched = false;
            let mut abbreviate = false;
            for pattern in self.include {
                if let Some(is_subpath_match) = subpath_match(name, pattern.as_bstr()) {
                    matched = true;
                    abbreviate |= is_subpath_match;
                }
            }
            matched.then_some(abbreviate)
        }
    }

    pub(crate) fn collect<E>(repo: &Repository, select: SelectRef, filters: Filters<'_>) -> Result<Vec<Candidate>, E>
    where
        E: From<crate::reference::iter::Error> + From<crate::reference::iter::init::Error>,
    {
        let platform = repo.references()?;
        let refs = match select {
            SelectRef::AllRefs => platform.all()?,
            SelectRef::AllTags | SelectRef::AnnotatedTags => platform.tags()?,
        };

        let mut out = Vec::new();
        for reference in refs.filter_map(Result::ok) {
            let Some(abbreviate_name_rev) = filters.allows(reference.inner.name.as_bstr()) else {
                continue;
            };

            match select {
                SelectRef::AnnotatedTags => {
                    if let Some(candidate) = annotated_tag_candidate(repo, reference, abbreviate_name_rev) {
                        out.push(candidate);
                    }
                }
                SelectRef::AllTags | SelectRef::AllRefs => {
                    if let Some(candidate) = any_ref_candidate(repo, reference, abbreviate_name_rev) {
                        out.push(candidate);
                    }
                }
            }
        }

        Ok(out)
    }

    fn any_ref_candidate(
        repo: &Repository,
        mut reference: crate::Reference<'_>,
        abbreviate_name_rev: bool,
    ) -> Option<Candidate> {
        let full_name = reference.inner.name.as_bstr().to_owned();
        let name = Cow::from(reference.inner.name.shorten().to_owned());
        let name_rev_name = if abbreviate_name_rev {
            name.clone()
        } else {
            Cow::from(name_rev_name(full_name.as_bstr()))
        };
        let target_id = reference.target().try_id().map(ToOwned::to_owned);
        let peeled_id = reference.peel_to_id().ok()?;
        let from_tag = full_name.starts_with_str("refs/tags/");
        let mut priority = 0;
        let mut taggerdate = 0;
        let deref = match target_id {
            Some(target_id) if peeled_id != *target_id => {
                let tag = repo.find_object(target_id).ok()?.try_into_tag().ok()?;
                taggerdate = tag.tagger().ok().and_then(|s| s.map(|s| s.seconds())).unwrap_or(0);
                priority = 1;
                true
            }
            _ => false,
        };

        Some(Candidate {
            peeled_id: peeled_id.inner,
            name,
            name_rev_name,
            priority,
            taggerdate,
            from_tag,
            deref,
        })
    }

    fn annotated_tag_candidate(
        _repo: &Repository,
        reference: crate::Reference<'_>,
        abbreviate_name_rev: bool,
    ) -> Option<Candidate> {
        // TODO: we assume direct refs for tags, which is the common case, but it doesn't have to be
        //       so rather follow symrefs till the first object and then peel tags after the first object was found.
        let name = Cow::from(reference.name().shorten().to_owned());
        let name_rev_name = if abbreviate_name_rev {
            name.clone()
        } else {
            Cow::from(name_rev_name(reference.name().as_bstr()))
        };
        let tag = reference.try_id()?.object().ok()?.try_into_tag().ok()?;
        let taggerdate = tag.tagger().ok().and_then(|s| s.map(|s| s.seconds())).unwrap_or(0);
        let commit_id = tag.target_id().ok()?.object().ok()?.try_into_commit().ok()?.id;
        Some(Candidate {
            peeled_id: commit_id,
            name,
            name_rev_name,
            priority: 1,
            taggerdate,
            from_tag: true,
            deref: true,
        })
    }

    pub(crate) fn commit_time(repo: &Repository, id: ObjectId) -> Option<i64> {
        repo.find_object(id)
            .ok()?
            .try_into_commit()
            .ok()?
            .committer()
            .ok()
            .map(|committer| committer.seconds())
    }

    fn name_rev_name(name: &BStr) -> BString {
        name.strip_prefix(b"refs/heads/")
            .or_else(|| name.strip_prefix(b"refs/"))
            .unwrap_or(name)
            .to_owned()
            .into()
    }

    fn subpath_match(name: &BStr, pattern: &BStr) -> Option<bool> {
        let mut subpath = name;
        let mut is_subpath = false;
        loop {
            if gix_glob::wildmatch(pattern, subpath, gix_glob::wildmatch::Mode::empty()) {
                return Some(is_subpath);
            }
            match subpath.find_byte(b'/') {
                Some(pos) => {
                    subpath = subpath[pos + 1..].as_bstr();
                    is_subpath = true;
                }
                None => return None,
            }
        }
    }
}

///
#[cfg(feature = "revision")]
pub mod describe {
    use gix_error::Exn;
    use gix_hash::ObjectId;
    use gix_hashtable::HashMap;
    use std::borrow::Cow;

    use crate::{Repository, bstr::BStr, ext::ObjectIdExt};

    pub use super::named_refs::SelectRef;

    /// The result of [`try_resolve()`][Platform::try_resolve()].
    pub struct Resolution<'repo> {
        /// The outcome of the describe operation.
        pub outcome: gix_revision::describe::Outcome<'static>,
        /// The id to describe.
        pub id: crate::Id<'repo>,
    }

    impl Resolution<'_> {
        /// Turn this instance into something displayable.
        pub fn format(self) -> Result<gix_revision::describe::Format<'static>, Error> {
            let prefix = self.id.shorten()?;
            Ok(self.outcome.into_format(prefix.hex_len()))
        }

        /// Turn this instance into something displayable, possibly with dirty-suffix.
        ///
        /// If `dirty_suffix` is `Some(suffix)`, a possibly expensive [dirty check](crate::Repository::is_dirty()) will be
        /// performed so that the `suffix` is appended to the output. If it is `None`, no check will be performed and
        /// there will be no suffix.
        /// Note that obtaining the dirty-state of the repository can be expensive.
        #[cfg(feature = "status")]
        pub fn format_with_dirty_suffix(
            self,
            dirty_suffix: impl Into<Option<String>>,
        ) -> Result<gix_revision::describe::Format<'static>, Error> {
            let prefix = self.id.shorten()?;
            let mut dirty_suffix = dirty_suffix.into();
            if dirty_suffix.is_some() && !self.id.repo.is_dirty()? {
                dirty_suffix.take();
            }
            let mut format = self.outcome.into_format(prefix.hex_len());
            format.dirty_suffix = dirty_suffix;
            Ok(format)
        }
    }

    /// The error returned by [`try_format()`][Platform::try_format()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCache(#[from] crate::repository::commit_graph_if_enabled::Error),
        #[error(transparent)]
        Describe(#[from] gix_revision::describe::Error),
        #[error("Could not produce an unambiguous shortened id for formatting.")]
        ShortId(#[from] crate::id::shorten::Error),
        #[error(transparent)]
        RefIter(#[from] crate::reference::iter::Error),
        #[error(transparent)]
        RefIterInit(#[from] crate::reference::iter::init::Error),
        #[error(transparent)]
        #[cfg(feature = "status")]
        DetermineIsDirty(#[from] crate::status::is_dirty::Error),
    }

    impl SelectRef {
        fn names(&self, repo: &Repository) -> Result<HashMap<ObjectId, Cow<'static, BStr>>, Error> {
            Ok(match self {
                SelectRef::AllTags | SelectRef::AllRefs => {
                    let mut refs = super::named_refs::collect::<Error>(repo, *self, super::named_refs::Filters::NONE)?;
                    // By priority, then by time ascending, then lexicographically.
                    // More recent entries overwrite older ones due to collection into hashmap.
                    refs.sort_by(|a, b| {
                        a.priority
                            .cmp(&b.priority)
                            .then_with(|| a.taggerdate.cmp(&b.taggerdate))
                            .then_with(|| b.name.cmp(&a.name))
                    });
                    refs.into_iter()
                        .map(|candidate| (candidate.peeled_id, candidate.name))
                        .collect()
                }
                SelectRef::AnnotatedTags => {
                    let mut refs = super::named_refs::collect::<Error>(repo, *self, super::named_refs::Filters::NONE)?;
                    // Sort by time ascending, then lexicographically.
                    // More recent entries overwrite older ones due to collection into hashmap.
                    refs.sort_by(|a, b| a.taggerdate.cmp(&b.taggerdate).then_with(|| b.name.cmp(&a.name)));
                    refs.into_iter()
                        .map(|candidate| (candidate.peeled_id, candidate.name))
                        .collect()
                }
            })
        }
    }

    /// A support type to allow configuring a `git describe` operation
    pub struct Platform<'repo> {
        pub(crate) id: gix_hash::ObjectId,
        /// The owning repository.
        pub repo: &'repo crate::Repository,
        pub(crate) select: SelectRef,
        pub(crate) first_parent: bool,
        pub(crate) id_as_fallback: bool,
        pub(crate) max_candidates: usize,
    }

    impl<'repo> Platform<'repo> {
        /// Configure which names to `select` from which describe can chose.
        pub fn names(mut self, select: SelectRef) -> Self {
            self.select = select;
            self
        }

        /// If true, shorten the graph traversal time by just traversing the first parent of merge commits.
        pub fn traverse_first_parent(mut self, first_parent: bool) -> Self {
            self.first_parent = first_parent;
            self
        }

        /// Only consider the given number of candidates, instead of the default of 10.
        pub fn max_candidates(mut self, candidates: usize) -> Self {
            self.max_candidates = candidates;
            self
        }

        /// If true, even if no candidate is available a format will always be produced.
        pub fn id_as_fallback(mut self, use_fallback: bool) -> Self {
            self.id_as_fallback = use_fallback;
            self
        }

        /// Try to find a name for the configured commit id using all prior configuration, returning `Some(describe::Format)`
        /// if one was found, or `None` if that wasn't the case.
        pub fn try_format(&self) -> Result<Option<gix_revision::describe::Format<'static>>, Error> {
            self.try_resolve()?.map(Resolution::format).transpose()
        }

        /// Try to find a name for the configured commit id using all prior configuration, returning `Some(Outcome)`
        /// if one was found.
        ///
        /// The outcome provides additional information, but leaves the caller with the burden
        ///
        /// # Performance
        ///
        /// It is greatly recommended to [assure an object cache is set](crate::Repository::object_cache_size_if_unset())
        /// to save ~40% of time.
        pub fn try_resolve_with_cache(
            &self,
            cache: Option<&'_ gix_commitgraph::Graph>,
        ) -> Result<Option<Resolution<'repo>>, Error> {
            let mut graph = self.repo.revision_graph(cache);
            let outcome = gix_revision::describe(
                &self.id,
                &mut graph,
                gix_revision::describe::Options {
                    name_by_oid: self.select.names(self.repo)?,
                    fallback_to_oid: self.id_as_fallback,
                    first_parent: self.first_parent,
                    max_candidates: self.max_candidates,
                },
            )
            .map_err(Exn::into_inner)?;

            Ok(outcome.map(|outcome| Resolution {
                outcome,
                id: self.id.attach(self.repo),
            }))
        }

        /// Like [`Self::try_resolve_with_cache()`], but obtains the commitgraph-cache internally for a single use.
        ///
        /// # Performance
        ///
        /// Prefer to use the [`Self::try_resolve_with_cache()`] method when processing more than one commit at a time.
        pub fn try_resolve(&self) -> Result<Option<Resolution<'repo>>, Error> {
            let cache = self.repo.commit_graph_if_enabled()?;
            self.try_resolve_with_cache(cache.as_ref())
        }

        /// Like [`try_format()`](Self::try_format()), but turns `id_as_fallback()` on to always produce a format.
        pub fn format(&mut self) -> Result<gix_revision::describe::Format<'static>, Error> {
            self.id_as_fallback = true;
            Ok(self.try_format()?.expect("BUG: fallback must always produce a format"))
        }
    }
}

///
#[cfg(feature = "revision")]
pub mod name_rev {
    use crate::bstr::BString;
    use gix_error::Exn;

    use crate::ext::ObjectIdExt;

    pub use super::named_refs::SelectRef;

    /// The result of [`try_resolve()`][Platform::try_resolve()].
    pub struct Resolution<'repo> {
        /// The outcome of the name-rev operation.
        pub outcome: gix_revision::name_rev::Outcome<'static>,
        /// The id to name.
        pub id: crate::Id<'repo>,
    }

    impl Resolution<'_> {
        /// Turn this instance into something displayable.
        pub fn format(self) -> Result<gix_revision::name_rev::Format<'static>, Error> {
            let prefix = self.id.shorten()?;
            Ok(self.outcome.into_format(prefix.hex_len()))
        }
    }

    /// The error returned by [`try_format()`][Platform::try_format()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCache(#[from] crate::repository::commit_graph_if_enabled::Error),
        #[error(transparent)]
        NameRev(#[from] gix_revision::name_rev::Error),
        #[error("Could not produce an unambiguous shortened id for formatting.")]
        ShortId(#[from] crate::id::shorten::Error),
        #[error(transparent)]
        RefIter(#[from] crate::reference::iter::Error),
        #[error(transparent)]
        RefIterInit(#[from] crate::reference::iter::init::Error),
    }

    /// A support type to allow configuring a `git name-rev` operation.
    pub struct Platform<'repo> {
        pub(crate) id: gix_hash::ObjectId,
        /// The owning repository.
        pub repo: &'repo crate::Repository,
        pub(crate) select: SelectRef,
        pub(crate) include_refs: Vec<BString>,
        pub(crate) exclude_refs: Vec<BString>,
        pub(crate) id_as_fallback: bool,
    }

    impl<'repo> Platform<'repo> {
        /// Configure which references to use for names.
        pub fn names(mut self, select: SelectRef) -> Self {
            self.select = select;
            self
        }

        /// Only use references matching `pattern`.
        pub fn include_ref(mut self, pattern: impl Into<BString>) -> Self {
            self.include_refs.push(pattern.into());
            self
        }

        /// Ignore references matching `pattern`.
        pub fn exclude_ref(mut self, pattern: impl Into<BString>) -> Self {
            self.exclude_refs.push(pattern.into());
            self
        }

        /// If true, even if no name is available a format will always be produced.
        pub fn id_as_fallback(mut self, use_fallback: bool) -> Self {
            self.id_as_fallback = use_fallback;
            self
        }

        /// Try to find a name for the configured commit id using all prior configuration.
        pub fn try_format(&self) -> Result<Option<gix_revision::name_rev::Format<'static>>, Error> {
            self.try_resolve()?.map(Resolution::format).transpose()
        }

        /// Try to find a name for the configured commit id using all prior configuration, returning `Some(Outcome)`
        /// if one was found.
        ///
        /// # Performance
        ///
        /// Prefer to use [`Self::try_resolve_with_cache()`] when processing more than one commit at a time.
        pub fn try_resolve_with_cache(
            &self,
            cache: Option<&'_ gix_commitgraph::Graph>,
        ) -> Result<Option<Resolution<'repo>>, Error> {
            let mut graph = self.repo.revision_graph(cache);
            let tips = super::named_refs::collect::<Error>(
                self.repo,
                self.select,
                super::named_refs::Filters {
                    include: &self.include_refs,
                    exclude: &self.exclude_refs,
                },
            )?
            .into_iter()
            .filter_map(|candidate| {
                let taggerdate = if candidate.deref {
                    candidate.taggerdate
                } else {
                    super::named_refs::commit_time(self.repo, candidate.peeled_id)?
                };
                Some(gix_revision::name_rev::Tip {
                    id: candidate.peeled_id,
                    name: candidate.name_rev_name,
                    taggerdate,
                    from_tag: candidate.from_tag,
                    deref: candidate.deref,
                })
            })
            .collect();

            let outcome = gix_revision::name_rev(
                &self.id,
                &mut graph,
                gix_revision::name_rev::Options {
                    tips,
                    fallback_to_oid: self.id_as_fallback,
                },
            )
            .map_err(Exn::into_inner)?;

            Ok(outcome.map(|outcome| Resolution {
                outcome,
                id: self.id.attach(self.repo),
            }))
        }

        /// Like [`Self::try_resolve_with_cache()`], but obtains the commitgraph-cache internally for a single use.
        pub fn try_resolve(&self) -> Result<Option<Resolution<'repo>>, Error> {
            let cache = self.repo.commit_graph_if_enabled()?;
            self.try_resolve_with_cache(cache.as_ref())
        }

        /// Like [`try_format()`](Self::try_format()), but turns `id_as_fallback()` on to always produce a format.
        pub fn format(&mut self) -> Result<gix_revision::name_rev::Format<'static>, Error> {
            self.id_as_fallback = true;
            Ok(self.try_format()?.expect("BUG: fallback must always produce a format"))
        }
    }
}
