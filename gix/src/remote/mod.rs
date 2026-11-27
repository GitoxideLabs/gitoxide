use std::collections::BTreeSet;

use crate::bstr::BString;

/// The direction of an operation carried out (or to be carried out) through a remote.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum Direction {
    /// Push local changes to the remote.
    Push,
    /// Fetch changes from the remote to the local repository.
    Fetch,
}

impl Direction {
    /// Return ourselves as string suitable for use as verb in an english sentence.
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Push => "push",
            Direction::Fetch => "fetch",
        }
    }
}

pub(crate) use name::remote_tracking_ref;
pub use name::{ConfiguredName, Name};

/// A type-definition for a sorted list of unvalidated remote names - they have been read straight from the configuration.
pub type Names = BTreeSet<BString>;

mod name;

mod build;

///
pub mod init;

///
pub mod fetch;

///
#[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
pub mod connect;

#[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
mod connection;
#[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
pub use connection::{AuthenticateFn, Connection, ref_map};

///
pub mod save;

mod access;
///
pub mod url;
