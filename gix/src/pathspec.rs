//! Pathspec plumbing and abstractions
pub use gix_pathspec::*;

use crate::{bstr::BStr, AttributeStack, Pathspec, PathspecDetached, Repository};

///
pub mod init {
    /// The error returned by [`Pathspec::new()`](super::Pathspec::new()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        MakeAttributes(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
        #[error(transparent)]
        Defaults(#[from] crate::repository::pathspec_defaults_ignore_case::Error),
        #[error(transparent)]
        ParseSpec(#[from] gix_pathspec::parse::Error),
        #[error(
            "Could not obtain the repository prefix as the relative path of the CWD as seen from the working tree"
        )]
        NormalizeSpec(#[from] gix_pathspec::normalize::Error),
        #[error(transparent)]
        RepoPrefix(#[from] gix_path::realpath::Error),
    }
}

/// Lifecycle
impl<'repo> Pathspec<'repo> {
    /// Create a new instance by parsing `patterns` into [`Pathspecs`](Pattern) to make them usable for searches.
    /// `make_attribute` may be called if one of the patterns has a `(attr:a)` element which requires attribute matching. It should
    /// be used to control where attributes are coming from.
    /// If `inherit_ignore_case` is `true`, the pathspecs may have their ignore-case default overridden to be case-insensitive by default.
    /// This only works towards turning ignore-case for pathspecs on, but won't ever turn that setting off if.
    /// If `empty_patterns_match_prefix` is `true`, then even empty patterns will match only what's inside of the prefix. Otherwise
    /// they will match everything.
    ///
    /// ### Deviation
    ///
    /// Pathspecs can declare to be case-insensitive as part of their elements, which is a setting that is now respected for attribute
    /// queries as well.
    pub fn new(
        repo: &'repo Repository,
        empty_patterns_match_prefix: bool,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        inherit_ignore_case: bool,
        make_attributes: impl FnOnce() -> Result<gix_worktree::Stack, Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Result<Self, init::Error> {
        let defaults = repo.pathspec_defaults_inherit_ignore_case(inherit_ignore_case)?;
        let patterns = patterns
            .into_iter()
            .map(move |p| parse(p.as_ref(), defaults))
            .collect::<Result<Vec<_>, _>>()?;
        let needs_cache = patterns.iter().any(|p| !p.attributes.is_empty());
        let prefix = if patterns.is_empty() && !empty_patterns_match_prefix {
            None
        } else {
            repo.prefix()?
        };
        let search = Search::from_specs(
            patterns,
            prefix,
            &gix_path::realpath_opts(
                repo.workdir().unwrap_or_else(|| repo.git_dir()),
                repo.options.current_dir_or_empty(),
                gix_path::realpath::MAX_SYMLINKS,
            )?,
        )?;
        let cache = needs_cache.then(make_attributes).transpose()?;

        gix_trace::debug!(
            longest_prefix = ?search.longest_common_directory(),
            prefix_dir = ?search.prefix_directory(),
            patterns = ?search.patterns().map(gix_pathspec::Pattern::path).collect::<Vec<_>>()
        );

        Ok(Self {
            repo,
            search,
            stack: cache,
        })
    }
    /// Turn ourselves into the functional parts for direct usage.
    /// Note that the [`cache`](AttributeStack) is only set if one of the [`search` patterns](Search)
    /// is specifying attributes to match for.
    pub fn into_parts(self) -> (Search, Option<AttributeStack<'repo>>) {
        (
            self.search,
            self.stack.map(|stack| AttributeStack::new(stack, self.repo)),
        )
    }

    /// Turn ourselves into an implementation that works without a repository instance and that is rather minimal.
    pub fn detach(self) -> std::io::Result<PathspecDetached> {
        Ok(PathspecDetached {
            search: self.search,
            stack: self.stack,
            odb: self.repo.objects.clone().into_arc()?,
        })
    }
}

/// Access
impl<'repo> Pathspec<'repo> {
    /// Return the attributes cache which is used when matching attributes in pathspecs, or `None` if none of the pathspecs require that.
    pub fn attributes(&self) -> Option<&gix_worktree::Stack> {
        self.stack.as_ref()
    }

    /// Return the search itself which can be used for matching paths or accessing the actual patterns that will be used.
    pub fn search(&self) -> &gix_pathspec::Search {
        &self.search
    }

    /// Return the first [`Match`](search::Match) of `relative_path`, or `None`.
    /// Note that the match might [be excluded](search::Match::is_excluded()).
    /// `is_dir` is true if `relative_path` is a directory.
    #[doc(
        alias = "match_diff",
        alias = "match_tree",
        alias = "match_index",
        alias = "match_workdir",
        alias = "matches_path",
        alias = "git2"
    )]
    pub fn pattern_matching_relative_path<'a>(
        &mut self,
        relative_path: impl Into<&'a BStr>,
        is_dir: Option<bool>,
    ) -> Option<gix_pathspec::search::Match<'_>> {
        self.search.pattern_matching_relative_path(
            relative_path.into(),
            is_dir,
            &mut |relative_path, case, is_dir, out| {
                let stack = self.stack.as_mut().expect("initialized in advance");
                stack
                    .set_case(case)
                    .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), &self.repo.objects)
                    .is_ok_and(|platform| platform.matching_attributes(out))
            },
        )
    }

    /// The simplified version of [`pattern_matching_relative_path()`](Self::pattern_matching_relative_path()) which returns
    /// `true` if `relative_path` is included in the set of positive pathspecs, while not being excluded.
    pub fn is_included<'a>(&mut self, relative_path: impl Into<&'a BStr>, is_dir: Option<bool>) -> bool {
        self.pattern_matching_relative_path(relative_path, is_dir)
            .is_some_and(|m| !m.is_excluded())
    }

    /// Return an iterator over all entries along with their path if the path matches the pathspec, or `None` if the pathspec is
    /// known to match no entry.
    // TODO: tests
    pub fn index_entries_with_paths<'s: 'repo, 'a: 'repo>(
        &'s mut self,
        index: &'a gix_index::State,
    ) -> Option<impl Iterator<Item = (&'a BStr, &'a gix_index::Entry)> + 'repo + 's> {
        index.prefixed_entries(self.search.common_prefix()).map(|entries| {
            entries.iter().filter_map(move |entry| {
                let path = entry.path(index);
                self.is_included(path, Some(false)).then_some((path, entry))
            })
        })
    }
}

/// Access
impl PathspecDetached {
    /// Return the first [`Match`](search::Match) of `relative_path`, or `None`.
    /// Note that the match might [be excluded](search::Match::is_excluded()).
    /// `is_dir` is true if `relative_path` is a directory.
    #[doc(
        alias = "match_diff",
        alias = "match_tree",
        alias = "match_index",
        alias = "match_workdir",
        alias = "matches_path",
        alias = "git2"
    )]
    pub fn pattern_matching_relative_path<'a>(
        &mut self,
        relative_path: impl Into<&'a BStr>,
        is_dir: Option<bool>,
    ) -> Option<gix_pathspec::search::Match<'_>> {
        self.search.pattern_matching_relative_path(
            relative_path.into(),
            is_dir,
            &mut |relative_path, case, is_dir, out| {
                let stack = self.stack.as_mut().expect("initialized in advance");
                stack
                    .set_case(case)
                    .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), &self.odb)
                    .is_ok_and(|platform| platform.matching_attributes(out))
            },
        )
    }

    /// The simplified version of [`pattern_matching_relative_path()`](Self::pattern_matching_relative_path()) which returns
    /// `true` if `relative_path` is included in the set of positive pathspecs, while not being excluded.
    pub fn is_included<'a>(&mut self, relative_path: impl Into<&'a BStr>, is_dir: Option<bool>) -> bool {
        self.pattern_matching_relative_path(relative_path, is_dir)
            .is_some_and(|m| !m.is_excluded())
    }
}

pub(crate) fn is_dir_to_mode(is_dir: bool) -> gix_index::entry::Mode {
    if is_dir {
        gix_index::entry::Mode::DIR
    } else {
        gix_index::entry::Mode::FILE
    }
}
