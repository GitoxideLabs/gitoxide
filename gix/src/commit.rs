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

///
#[cfg(feature = "revision")]
pub mod describe {
    use gix_error::Exn;
    use gix_hash::ObjectId;
    use gix_hashtable::HashMap;
    use std::borrow::Cow;

    use crate::{bstr::BStr, ext::ObjectIdExt, Repository};

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
        Describe(#[from] Box<gix_revision::describe::Error>),
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

    impl SelectRef {
        fn names(&self, repo: &Repository) -> Result<HashMap<ObjectId, Cow<'static, BStr>>, Error> {
            let platform = repo.references()?;

            Ok(match self {
                SelectRef::AllTags | SelectRef::AllRefs => {
                    let mut refs: Vec<_> = match self {
                        SelectRef::AllRefs => platform.all()?,
                        SelectRef::AllTags => platform.tags()?,
                        _ => unreachable!(),
                    }
                    .filter_map(Result::ok)
                    .filter_map(|mut r: crate::Reference<'_>| {
                        let target_id = r.target().try_id().map(ToOwned::to_owned);
                        let peeled_id = r.peel_to_id().ok()?;
                        let (prio, tag_time) = match target_id {
                            Some(target_id) if peeled_id != *target_id => {
                                let tag = repo.find_object(target_id).ok()?.try_into_tag().ok()?;
                                let tag_time = tag.tagger().ok().and_then(|s| s.map(|s| s.seconds())).unwrap_or(0);
                                (1, tag_time)
                            }
                            _ => (0, 0),
                        };
                        (
                            peeled_id.inner,
                            prio,
                            tag_time,
                            Cow::from(r.inner.name.shorten().to_owned()),
                        )
                            .into()
                    })
                    .collect();
                    // By priority, then by time ascending, then lexicographically.
                    // More recent entries overwrite older ones due to collection into hashmap.
                    refs.sort_by(
                        |(_a_peeled_id, a_prio, a_time, a_name), (_b_peeled_id, b_prio, b_time, b_name)| {
                            a_prio
                                .cmp(b_prio)
                                .then_with(|| a_time.cmp(b_time))
                                .then_with(|| b_name.cmp(a_name))
                        },
                    );
                    refs.into_iter().map(|(a, _, _, b)| (a, b)).collect()
                }
                SelectRef::AnnotatedTags => {
                    let mut peeled_commits_and_tag_date: Vec<_> = platform
                        .tags()?
                        .filter_map(Result::ok)
                        .filter_map(|r: crate::Reference<'_>| {
                            // TODO: we assume direct refs for tags, which is the common case, but it doesn't have to be
                            //       so rather follow symrefs till the first object and then peel tags after the first object was found.
                            let tag = r.try_id()?.object().ok()?.try_into_tag().ok()?;
                            let tag_time = tag.tagger().ok().and_then(|s| s.map(|s| s.seconds())).unwrap_or(0);
                            let commit_id = tag.target_id().ok()?.object().ok()?.try_into_commit().ok()?.id;
                            Some((commit_id, tag_time, Cow::<BStr>::from(r.name().shorten().to_owned())))
                        })
                        .collect();
                    // Sort by time ascending, then lexicographically.
                    // More recent entries overwrite older ones due to collection into hashmap.
                    peeled_commits_and_tag_date.sort_by(|(_a_id, a_time, a_name), (_b_id, b_time, b_name)| {
                        a_time.cmp(b_time).then_with(|| b_name.cmp(a_name))
                    });
                    peeled_commits_and_tag_date
                        .into_iter()
                        .map(|(a, _, c)| (a, c))
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
            .map_err(Exn::into_box)?;

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
