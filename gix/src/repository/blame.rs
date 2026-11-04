use gix_blame::Start;
use gix_hash::ObjectId;
use gix_ref::bstr::BStr;

use crate::{Repository, Result, repository::blame_file};

impl Repository {
    /// Produce a list of consecutive [`gix_blame::BlameEntry`] instances. Each `BlameEntry`
    /// corresponds to a hunk of consecutive lines of the file at `suspect:<file_path>` that got
    /// introduced by a specific commit.
    ///
    /// For details, see the documentation of [`gix_blame::file()`].
    pub fn blame_file(
        &self,
        file_path: &BStr,
        suspect: impl Into<ObjectId>,
        options: blame_file::Options,
    ) -> Result<gix_blame::Outcome> {
        self.blame_file_from(file_path, Start::Commit(suspect.into()), options)
    }

    /// Attribute lines in a commit or in supplied contents to their originating commits.
    ///
    /// [`Start::Contents`] can represent uncommitted worktree changes; its contents must already
    /// have passed through the configured worktree filters and, when enabled, textconv drivers.
    /// Newly introduced lines carry the null id.
    /// By default, revisions in `blame.ignoreRevsFile` are excluded using line similarity.
    /// Relative file names resolve from the repository worktree, or the Git directory for bare
    /// repositories. Empty configuration values clear earlier exclusions. Unavailable objects
    /// and non-commits in these files are skipped; unreadable files and malformed IDs are errors.
    /// [`blame_file::Options::ignored_revisions`] can override the configured files.
    /// No objects or references are written. See [`gix_blame::file()`] for details and errors.
    pub fn blame_file_from(
        &self,
        file_path: &BStr,
        start: Start<'_>,
        options: blame_file::Options,
    ) -> Result<gix_blame::Outcome> {
        let cache = self.commit_graph_if_enabled()?;
        let mut resource_cache = if options.textconv {
            let index = self.index_or_load_from_head_or_empty()?;
            let source = if self.workdir().is_some() {
                gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping
            } else {
                gix_worktree::stack::state::attributes::Source::IdMapping
            };
            crate::diff::resource_cache(
                self,
                gix_diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
                self.attributes_only(&index, source)?.inner,
                Default::default(),
            )?
        } else {
            self.diff_resource_cache_for_tree_diff()?
        };

        let blame_file::Options {
            diff_algorithm,
            ranges,
            since,
            rewrites,
            contents_mode,
            textconv: _,
            ignored_revisions,
        } = options;
        let diff_algorithm = match diff_algorithm {
            Some(diff_algorithm) => diff_algorithm,
            None => self.diff_algorithm()?,
        };

        let options = gix_blame::Options {
            diff_algorithm,
            ranges,
            since,
            rewrites,
            debug_track_path: false,
            contents_mode,
            ignored_revisions: super::blame_ignore::revisions(self, ignored_revisions)?,
            boundary_commits: self
                .shallow_commits()?
                .map(|commits| commits.iter().copied().collect())
                .unwrap_or_default(),
        };

        let outcome = gix_blame::file(&self.objects, start, cache, &mut resource_cache, file_path, options)?;

        Ok(outcome)
    }
}
