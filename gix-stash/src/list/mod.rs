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
pub enum Error {
    /// The reflog file for `refs/stash` could not be read or seeked.
    ///
    /// This is extracted from [`gix_ref::file::log::Error`] since I/O is
    /// the only variant that can occur when a hard-coded, valid ref name is used.
    #[error("failed to perform I/O while reading the refs/stash reflog")]
    Io(#[from] std::io::Error),
    /// An individual reflog line failed to decode.
    #[error("failed to decode a reflog line for refs/stash")]
    DecodeReflog(#[from] gix_ref::file::log::iter::reverse::Error),
}

pub(crate) mod function {
    use gix_ref::FullName;

    use super::{Entry, Error, Outcome};

    /// Walk the reflog of `refs/stash` and return every stash entry, newest
    /// first.
    ///
    /// Returns an empty [`Outcome`] when `refs/stash` is unborn — matching
    /// `git stash list` which prints nothing in that case.
    ///
    /// `refs` is the file-based ref store for the repository (typically the
    /// `.git/` directory or the common-dir for linked worktrees).
    pub fn list(refs: &gix_ref::file::Store) -> Result<Outcome, Error> {
        // Parse the well-known ref name once.  FullName validates at
        // creation time so the expect is safe for this hard-coded literal.
        let stash_name: FullName = "refs/stash".try_into().expect("refs/stash is a valid ref name");

        // 4 KiB sliding window for the reverse-reflog iterator.
        let mut buf = vec![0u8; 4 * 1024];

        let iter = match refs
            .reflog_iter_rev(stash_name.as_ref(), &mut buf)
            .map_err(|e| match e {
                gix_ref::file::log::Error::Io(io) => Error::Io(io),
                // Cannot happen: we pass a hard-coded valid ref name.
                gix_ref::file::log::Error::RefnameValidation(_) => {
                    unreachable!("refs/stash is always a valid ref name")
                }
            })? {
            // refs/stash has never been written — no stash entries exist.
            None => return Ok(Outcome::default()),
            Some(iter) => iter,
        };

        let mut entries: Vec<Entry> = Vec::new();

        for line_result in iter {
            let line = line_result?;

            let commit = line.new_oid;
            let message = line.message.clone();
            // `gix_actor::Signature` carries a parsed `gix_date::Time` field
            // whose `seconds` is `i64`; stash entries cannot predate the epoch.
            let time_seconds = line.signature.time.seconds.max(0) as u64;

            entries.push(Entry {
                // entries are read newest-first, so index 0 = newest.
                index: entries.len(),
                commit,
                message,
                time_seconds,
            });
        }

        Ok(Outcome { entries })
    }
}
