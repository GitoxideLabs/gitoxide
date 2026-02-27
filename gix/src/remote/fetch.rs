///
pub mod negotiate {
    #[cfg(feature = "credentials")]
    pub use gix_negotiate::Algorithm;
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
    pub use gix_protocol::fetch::negotiate::Error;
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub use super::connection::fetch::{
    outcome, prepare, refs, Error, Outcome, Prepare, ProgressId, RefLogMessage, Status,
};

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
pub use gix_protocol::fetch::{refmap, RefMap};
pub use gix_protocol::fetch::{Shallow, Tags};

mod object_filter;
pub use object_filter::{ObjectFilter, ParseError as ObjectFilterParseError};
