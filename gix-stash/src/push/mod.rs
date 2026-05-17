//! Implementation of [`stash push`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-push).

use bstr::BString;
use gix_hash::ObjectId;

/// Options controlling [`function::push`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Include untracked (but not git-ignored) files in `parent[2]` of the
    /// stash commit and remove them from the working tree.
    pub include_untracked: bool,

    /// Also include ignored files when `include_untracked` is set.  Has no
    /// effect on its own.
    pub include_ignored: bool,

    /// Keep the index state intact in the working tree after stashing — the
    /// stash still captures it, but the on-disk working tree continues to
    /// reflect what was staged.
    pub keep_index: bool,

    /// Optional explicit message — written to the stash commit subject and
    /// the reflog entry.  When `None`, the message defaults to
    /// `WIP on <branch>: <short-hash> <subject>`.
    pub message: Option<BString>,
}

/// Result of a successful [`function::push`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The id of the newly-created stash commit (now `refs/stash`).
    pub stash: ObjectId,

    /// The id of the index-state commit (`parent[1]` of the stash commit).
    pub index_commit: ObjectId,

    /// The id of the untracked-files commit (`parent[2]`), if one was created.
    pub untracked_commit: Option<ObjectId>,

    /// The previous value of `refs/stash`, now reachable only via reflog.
    pub previous: Option<ObjectId>,
}

/// Errors returned by [`function::push`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("not implemented yet")]
    NotImplemented,
}

pub(crate) mod function {
    use super::{Error, Options, Outcome};

    /// Capture the current working tree (+ index, + optional untracked) as a
    /// new stash commit at `refs/stash`, then reset the working tree and
    /// index to match `HEAD`.
    ///
    /// Returns [`Outcome`] on success, [`Error::NotImplemented`] until the
    /// MVP lands.
    pub fn push(_options: Options) -> Result<Outcome, Error> {
        Err(Error::NotImplemented)
    }
}
