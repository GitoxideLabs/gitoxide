//! Test which commit tips contain a target, as in `git for-each-ref --contains`.
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
//! # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;

//! let mut graph = repo.revision_graph_with_cache()?;
//! let mut target = repo.contains_target(repo.head_id()?, &mut graph)?;
//! let references = repo.references()?;
//! let mut matching = Vec::new();
//! for reference in references.prefixed("refs/heads/")? {
//!     let mut reference = reference?;
//!     if target.is_reachable_from(reference.peel_to_id()?)? {
//!         matching.push(reference.name().to_owned());
//!     }
//! }
//! assert!(matching.iter().any(|name| name.as_bstr() == "refs/heads/main"));
//! # Ok(()) }
//! ```

use gix_hash::ObjectId;
use gix_revwalk::{Graph, graph::Commit};

use crate::Result;

/// A containment query for one target commit, with answers shared across tips.
/// Create one with [`crate::Repository::contains_target()`].
pub struct TargetQuery<'query, 'repo, 'cache> {
    inner: gix_revision::contains::Query<'query, 'repo, 'cache>,
}

impl<'query, 'repo, 'cache> TargetQuery<'query, 'repo, 'cache> {
    pub(crate) fn new(
        target_commit_id: impl Into<ObjectId>,
        graph: &'query mut Graph<'repo, 'cache, Commit<()>>,
    ) -> Result<Self> {
        Ok(Self {
            inner: gix_revision::contains::Query::new(target_commit_id, graph)?,
        })
    }

    /// Return whether the target is reachable from `tip_commit_id` by following *zero* or more parent links.
    pub fn is_reachable_from(&mut self, tip_commit_id: impl Into<ObjectId>) -> Result<bool> {
        self.inner.contains(tip_commit_id)
    }
}
