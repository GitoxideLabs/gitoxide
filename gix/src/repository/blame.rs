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
    /// have passed through the configured worktree filters. Newly introduced lines carry the null id.
    /// No objects or references are written. See [`gix_blame::file()`] for details and errors.
    pub fn blame_file_from(
        &self,
        file_path: &BStr,
        start: Start<'_>,
        options: blame_file::Options,
    ) -> Result<gix_blame::Outcome> {
        let cache = self.commit_graph_if_enabled()?;
        let mut resource_cache = self.diff_resource_cache_for_tree_diff()?;

        let blame_file::Options {
            diff_algorithm,
            ranges,
            since,
            rewrites,
            contents_mode,
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
            boundary_commits: self
                .shallow_commits()?
                .map(|commits| commits.iter().copied().collect())
                .unwrap_or_default(),
        };

        let outcome = gix_blame::file(&self.objects, start, cache, &mut resource_cache, file_path, options)?;

        Ok(outcome)
    }
}
