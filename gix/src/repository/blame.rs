use gix_hash::ObjectId;
use gix_ref::bstr::BStr;

use crate::{repository::blame_file, Repository};

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
        options: gix_blame::Options,
    ) -> Result<gix_blame::Outcome, blame_file::Error> {
        let cache: Option<gix_commitgraph::Graph> = self.commit_graph_if_enabled()?;
        let mut resource_cache = self.diff_resource_cache_for_tree_diff()?;

        let outcome = gix_blame::file(
            &self.objects,
            suspect.into(),
            cache,
            &mut resource_cache,
            file_path,
            options,
        )?;

        Ok(outcome)
    }
}
