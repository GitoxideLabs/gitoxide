//! Implementation of [`stash pop`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-pop).

use gix_hash::ObjectId;

/// Result of a successful [`function::pop`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The id of the stash commit that was applied + dropped.
    pub applied: ObjectId,

    /// The new value of `refs/stash` after dropping the applied entry — `None`
    /// when no older stash entries remain.
    pub new_top: Option<ObjectId>,

    /// Whether the apply step produced merge conflicts.  When `true`, the
    /// working tree contains conflict markers and `refs/stash` is left
    /// untouched (matching `git stash pop` behaviour: only drop on clean
    /// apply).
    pub had_conflicts: bool,
}

/// Errors returned by [`function::pop`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("no stash entries to pop (refs/stash is unborn)")]
    NoStash,
    #[error("not implemented yet")]
    NotImplemented,
}

pub(crate) mod function {
    use super::{Error, Outcome};

    /// Apply the latest stash entry to the working tree (3-way merge against
    /// the stash commit's parents) and remove it from `refs/stash`.
    ///
    /// If the merge produces conflicts, the working tree is left in a
    /// conflicted state and the stash is **not** dropped — matching
    /// `git stash pop` semantics.
    ///
    /// Returns [`Outcome`] on success, [`Error::NotImplemented`] until the
    /// MVP lands.
    pub fn pop() -> Result<Outcome, Error> {
        Err(Error::NotImplemented)
    }
}
