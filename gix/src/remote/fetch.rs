///
pub mod negotiate {
    #[cfg(feature = "credentials")]
    pub use gix_negotiate::Algorithm;
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub use super::connection::fetch::{Outcome, Prepare, ProgressId, RefLogMessage, Status, outcome, prune, refs};

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) use super::connection::fetch::PrepareDetached;
#[cfg(feature = "blocking-network-client")]
pub(crate) use super::connection::fetch::PrepareLocal;

/// If `Yes`, don't really make changes but do as much as possible to get an idea of what would be done.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub(crate) enum DryRun {
    /// Enable dry-run mode and don't actually change the underlying repository in any way.
    Yes,
    /// Run the operation like normal, making changes to the underlying repository.
    No,
}

/// How to deal with refs when cloning or fetching.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub(crate) enum WritePackedRefs {
    /// Normal operation, i.e. don't use packed-refs at all for writing.
    Never,
    /// Put ref updates straight into the `packed-refs` file, without creating loose refs first or dealing with them in any way.
    Only,
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub use gix_protocol::fetch::{RefMap, refmap};
pub use gix_protocol::fetch::{Shallow, Tags};

/// Options for [`crate::Remote::fetch()`].
#[cfg(feature = "blocking-network-client")]
#[derive(Default)]
pub struct Options {
    /// Additional ref mapping and handshake options.
    pub ref_map: crate::remote::ref_map::Options,
    /// Report proposed changes without writing objects, refs, or shallow state.
    pub dry_run: bool,
    /// Delete stale destinations covered by explicit refspecs before updating fetched refs.
    pub prune: bool,
    /// Replace the worktree's `FETCH_HEAD` with the refs received by this fetch.
    ///
    /// Explicit refspecs are merge candidates, as with `git fetch <remote> <refspec>…`;
    /// implicit tags and non-commit targets are not for merging. The file is locked before
    /// fetching and atomically replaced afterwards. Unlike Git, errors leave the old file intact.
    /// Disabled by default and always ignored during a dry run.
    pub write_fetch_head: bool,
    /// Allow a selection whose exact source refs no longer exist.
    pub missing_refs_allowed: bool,
    /// Write reference updates directly to packed-refs.
    pub write_packed_refs_only: bool,
    /// Override the reflog message; by default it describes a fetch.
    pub reflog_message: Option<RefLogMessage>,
    /// How to adjust the shallow boundary.
    pub shallow: Shallow,
}
