//! An implementation of the `git` transport layer, abstracting over all of its [versions][Protocol].
//!
//! Use `client::blocking_io::connect()` or `client::async_io::connect()` to establish a connection.
//!
//! All git transports are supported, including `ssh`, `git`, `http` and `https`, as well as local repository paths.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

#[cfg(feature = "async-trait")]
pub use async_trait;
pub use bstr;
#[cfg(feature = "futures-io")]
pub use futures_io;
pub use gix_packetline as packetline;

/// The version of the way client and server communicate.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Protocol {
    /// Version 0 is like V1, but doesn't show capabilities at all, at least when hosted without `git-daemon`.
    V0 = 0,
    /// Version 1 was the first one conceived, is stateful, and our implementation was seen to cause deadlocks. Prefer V2
    V1 = 1,
    /// A command-based and stateless protocol with clear semantics, and the one to use assuming the server isn't very old.
    /// This is the default.
    #[default]
    V2 = 2,
}

/// The kind of service to invoke on the client or the server side.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Service {
    /// The service sending packs from a server to the client. Used for fetching pack data.
    UploadPack,
    /// The service receiving packs produced by the client, who sends a pack to the server.
    ReceivePack,
}

impl Service {
    /// Render this instance as string recognized by the git transport layer.
    pub fn as_str(&self) -> &'static str {
        match self {
            Service::ReceivePack => "git-receive-pack",
            Service::UploadPack => "git-upload-pack",
        }
    }
}

mod traits {
    use std::convert::Infallible;

    /// An error which can tell whether it's worth retrying to maybe succeed next time.
    pub trait IsSpuriousError: std::error::Error {
        /// Return `true` if retrying might result in a different outcome due to IO working out differently.
        fn is_spurious(&self) -> bool {
            false
        }
    }

    impl IsSpuriousError for Infallible {}

    impl IsSpuriousError for std::io::Error {
        fn is_spurious(&self) -> bool {
            // TODO: also include the new special Kinds (currently unstable)
            use std::io::ErrorKind::*;
            match self.kind() {
                Unsupported | WriteZero | InvalidInput | InvalidData | WouldBlock | AlreadyExists
                | AddrNotAvailable | NotConnected | Other | PermissionDenied | NotFound => false,
                Interrupted | UnexpectedEof | OutOfMemory | TimedOut | BrokenPipe | AddrInUse | ConnectionAborted
                | ConnectionReset | ConnectionRefused => true,
                _ => false,
            }
        }
    }
}
pub use traits::IsSpuriousError;

mod error;
pub use error::Error;

///
pub mod client;
