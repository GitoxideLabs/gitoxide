//! Plumbing for [`git stash`](https://git-scm.com/docs/git-stash) workflows.
//!
//! This crate implements the `push` and `pop` operations as a starting MVP.
//! Additional operations (`apply`, `drop`, `list`, `show`, `branch`,
//! `autostash`) are tracked in [`crate-status.md`] and may follow.
//!
//! [`crate-status.md`]: https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md
//!
//! # Stash representation
//!
//! A stash entry is a merge commit with 2 or 3 parents stored at the single
//! ref `refs/stash`, with the reflog providing the stack of older entries:
//!
//! * `parent[0]` — the commit that `HEAD` pointed at when the stash was made
//! * `parent[1]` — a commit whose tree is the **index** at stash time
//! * `parent[2]` — *(optional, only when `--include-untracked` is used)* — a
//!   commit whose tree contains the **untracked** files at stash time
//!
//! The stash commit's own tree is the **working tree** at stash time.
//!
//! # API
//!
//! * [`push`] — capture working tree (+ index, + optional untracked) and reset
//!   to `HEAD`.
//! * [`pop`] — apply the latest stash to the working tree (3-way merge) and
//!   drop it from `refs/stash`.
//! * [`list`] — walk the `refs/stash` reflog and return every stash entry.
//!
//! All three operate on plumbing handles (index, ODB, ref store, worktree
//! path) rather than a high-level repository — the porcelain layer in `gix`
//! wraps them and provides `Repository::stash_push` / `Repository::stash_pop`
//! / `Repository::stash_list`.

#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod list;
pub mod pop;
pub mod push;

pub use list::{Entry as ListEntry, Outcome as ListOutcome, function::list};
pub use pop::{Outcome as PopOutcome, function::pop};
pub use push::{Context as PushContext, Options as PushOptions, Outcome as PushOutcome, function::push};
