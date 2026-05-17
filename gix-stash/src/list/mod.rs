//! Implementation of [`stash list`](https://git-scm.com/docs/git-stash#Documentation/git-stash.txt-list).

use bstr::BString;
use gix_hash::ObjectId;

/// A single stash entry as found in the `refs/stash` reflog.
///
/// The newest entry is index `0` (the current value of `refs/stash`); older
/// entries follow in reverse-chronological order, matching what
/// `git stash list` prints (`stash@{0}`, `stash@{1}`, …).
#[derive(Debug, Clone)]
pub struct Entry {
    /// Stack position — `0` is the newest.
    pub index: usize,

    /// The stash commit's object id.
    pub commit: ObjectId,

    /// The reflog message (e.g. `WIP on main: abc1234 commit subject`).
    pub message: BString,

    /// Seconds since the Unix epoch, from the reflog committer line.
    pub time_seconds: u64,
}

/// Result of [`function::list`].
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// The stash entries, newest first.  Empty when `refs/stash` is unborn
    /// (no stashes have ever been created in this repo).
    pub entries: Vec<Entry>,
}

/// Errors returned by [`function::list`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("not implemented yet")]
    NotImplemented,
}

pub(crate) mod function {
    use super::{Error, Outcome};

    /// Walk the reflog of `refs/stash` and return every stash entry, newest
    /// first.
    ///
    /// Returns an empty [`Outcome`] when `refs/stash` is unborn — matching
    /// `git stash list` which prints nothing in that case.
    ///
    /// Returns [`Error::NotImplemented`] until the MVP lands.
    pub fn list() -> Result<Outcome, Error> {
        Err(Error::NotImplemented)
    }
}
