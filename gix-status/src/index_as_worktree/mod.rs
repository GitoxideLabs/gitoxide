//! Changes between an index and a worktree.
///
mod types;
pub use types::{Change, Conflict, ConflictIndexEntry, Context, EntryStatus, Options, Outcome, VisitEntry};

mod recorder;
pub use recorder::{Record, Recorder};

pub(super) mod function;
///
pub mod traits;
