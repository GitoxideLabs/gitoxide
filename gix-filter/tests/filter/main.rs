pub(crate) mod driver;
pub(crate) mod eol;
mod ident;
mod pipeline;
/// Only Unix keeps unreaped children in its process table, so only there can their absence be asserted.
#[cfg(unix)]
pub(crate) mod reap;
mod worktree;

pub use gix_testtools::Result;
