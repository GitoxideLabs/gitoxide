use std::{borrow::Cow, collections::BTreeSet};

use crate::bstr::BStr;

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

/// The name of a remote, either interpreted as symbol like `origin` or as url as returned by [`Remote::name()`][crate::Remote::name()].
#[derive(Debug, PartialEq, Eq, Clone, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Name<'repo> {
    /// A symbolic name, like `origin`.
    /// Note that it has not necessarily been validated yet.
    Symbol(Cow<'repo, str>),
    /// A url pointing to the remote host directly.
    Url(Cow<'repo, BStr>),
}

/// A type-definition for a sorted list of unvalidated remote names - they have been read straight from the configuration.
pub type Names<'a> = BTreeSet<Cow<'a, BStr>>;

///
pub mod name;

mod build;

mod errors;
pub use errors::find;

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
