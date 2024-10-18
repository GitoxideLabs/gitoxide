//! An abstraction over [fetching][fetch()] a pack from the server.
//!
//! This implementation hides the transport layer, statefulness and the protocol version to the [fetch delegate][fetch::Delegate],
//! the actual client implementation.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

/// A selector for V2 commands to invoke on the server for purpose of pre-invocation validation.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum Command {
    /// List references.
    LsRefs,
    /// Fetch a pack.
    Fetch,
}
pub mod command;

#[cfg(feature = "async-trait")]
pub use async_trait;
#[cfg(feature = "futures-io")]
pub use futures_io;
#[cfg(feature = "futures-lite")]
pub use futures_lite;
pub use gix_credentials as credentials;
/// A convenience export allowing users of gix-protocol to use the transport layer without their own cargo dependency.
pub use gix_transport as transport;
pub use maybe_async;

///
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub mod fetch;

mod remote_progress;
pub use remote_progress::RemoteProgress;

#[cfg(all(feature = "blocking-client", feature = "async-client"))]
compile_error!("Cannot set both 'blocking-client' and 'async-client' features as they are mutually exclusive");

///
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub mod handshake;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use handshake::function::handshake;

///
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub mod ls_refs;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use ls_refs::function::ls_refs;

mod util;
pub use util::agent;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use util::indicate_end_of_interaction;
