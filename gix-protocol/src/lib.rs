//! An abstraction over [fetching][fetch()] a pack from the server.
//!
//! Generally, there is the following order of operations.
//!
//! * create a `Transport`, either blocking or async
//! * perform a [`handshake()`]
//! * execute a [`Command`]
//!     - [list references](LsRefsCommand)
//!          - create a mapping between [refspecs and references](fetch::RefMap)
//!     - [receive a pack](fetch())
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

/// A function that performs a given credential action, trying to obtain credentials for an operation that needs it.
///
/// Useful for both `fetch` and `push`.
#[cfg(feature = "handshake")]
pub type AuthenticateFn<'a> = Box<dyn FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'a>;

/// A selector for V2 commands to invoke on the server for purpose of pre-invocation validation.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum Command {
    /// List references.
    LsRefs,
    /// Fetch a pack.
    Fetch,
}
pub mod command;

#[cfg(feature = "async-client")]
pub use async_trait;
#[cfg(feature = "async-client")]
pub use futures_io;
#[cfg(feature = "async-client")]
pub use futures_lite;
#[cfg(feature = "handshake")]
pub use gix_credentials as credentials;
/// A convenience export allowing users of gix-protocol to use the transport layer without their own cargo dependency.
pub use gix_transport as transport;
pub use maybe_async;

///
pub mod fetch;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use fetch::function::fetch;

mod remote_progress;
pub use remote_progress::RemoteProgress;

#[cfg(all(feature = "blocking-client", feature = "async-client"))]
compile_error!("Cannot set both 'blocking-client' and 'async-client' features as they are mutually exclusive");

///
pub mod handshake;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
#[cfg(feature = "handshake")]
pub use handshake::function::handshake;
#[cfg(feature = "handshake")]
pub use handshake::hero::Handshake;

///
pub mod ls_refs;
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use ls_refs::function::LsRefsCommand;

mod util;
pub use util::*;
