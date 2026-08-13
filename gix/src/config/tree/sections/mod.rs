#![allow(clippy::unnecessary_literal_bound)]
#![allow(missing_docs)]

/// The `author` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Author;
mod author;

/// The `branch` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Branch;
pub mod branch;

/// The `checkout` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Checkout;
pub mod checkout;

/// The `clone` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Clone;
mod clone;

/// The `committer` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Committer;
mod committer;

/// The `core` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Core;
pub mod core;

/// The `credential` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Credential;
pub mod credential;

/// The `diff` top-level section.
#[derive(Copy, Clone, Default)]
#[cfg(feature = "blob-diff")]
pub struct Diff;
#[cfg(feature = "blob-diff")]
pub mod diff;

/// The `extension` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Extensions;
pub mod extensions;

/// The `fetch` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Fetch;
pub mod fetch;

/// The `gitoxide` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Gitoxide;
pub mod gitoxide;

/// The `gpg` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Gpg;
pub mod gpg;

/// The `http` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Http;
pub mod http;

/// The `index` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Index;
pub mod index;

/// The `init` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Init;
mod init;

#[derive(Copy, Clone, Default)]
pub struct Mailmap;
mod mailmap;

#[derive(Copy, Clone, Default)]
pub struct Merge;
mod merge;

/// The `pack` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Pack;
pub mod pack;

/// The `protocol` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Protocol;
pub mod protocol;

/// The `push` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Push;
pub mod push;

/// The `remote` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Remote;
pub mod remote;

/// The `safe` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Safe;
mod safe;

/// The `ssh` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Ssh;
pub mod ssh;

/// The `status` top-level section.
#[derive(Copy, Clone, Default)]
#[cfg(feature = "status")]
pub struct Status;
#[cfg(feature = "status")]
pub mod status;

/// The `user` top-level section.
#[derive(Copy, Clone, Default)]
pub struct User;
mod user;

/// The `url` top-level section.
#[derive(Copy, Clone, Default)]
pub struct Url;
mod url;
