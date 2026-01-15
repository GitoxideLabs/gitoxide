bitflags::bitflags! {
    /// The flags used in the graph for finding [merge bases](crate::merge_base()).
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct Flags: u8 {
        /// The commit belongs to the graph reachable by the first commit
        const COMMIT1 = 1 << 0;
        /// The commit belongs to the graph reachable by all other commits.
        const COMMIT2 = 1 << 1;

        /// Marks the commit as done, it's reachable by both COMMIT1 and COMMIT2.
        const STALE = 1 << 2;
        /// The commit was already put ontto the results list.
        const RESULT = 1 << 3;
    }
}

/// The error returned by the [`merge_base()`][function::merge_base()] function.
pub type Error = Simple;

/// A simple error type for merge base operations.
#[derive(Debug)]
pub struct Simple(pub &'static str);

impl std::fmt::Display for Simple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Simple {}

pub(crate) mod function;

mod octopus {
    use gix_hash::ObjectId;
    use gix_revwalk::{graph, Graph};

    use crate::merge_base::{Error, Flags};

    /// Given a commit at `first` id, traverse the commit `graph` and return *the best common ancestor* between it and `others`,
    /// sorted from best to worst. Returns `None` if there is no common merge-base as `first` and `others` don't *all* share history.
    /// If `others` is empty, `Some(first)` is returned.
    ///
    /// # Performance
    ///
    /// For repeated calls, be sure to re-use `graph` as its content will be kept and reused for a great speed-up. The contained flags
    /// will automatically be cleared.
    pub fn octopus(
        mut first: ObjectId,
        others: &[ObjectId],
        graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
    ) -> Result<Option<ObjectId>, Error> {
        for other in others {
            if let Some(next) =
                crate::merge_base(first, std::slice::from_ref(other), graph)?.and_then(|bases| bases.into_iter().next())
            {
                first = next;
            } else {
                return Ok(None);
            }
        }
        Ok(Some(first))
    }
}
pub use octopus::octopus;
