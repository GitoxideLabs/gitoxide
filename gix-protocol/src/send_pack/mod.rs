//! Client implementation of git's send-pack protocol (the push direction).
//!
//! Mirrors the layout of [`crate::fetch`]:
//!
//! * [`command_list`] — serialize the command-list pkt-lines
//!   (`<old-oid> <new-oid> <refname>\0<capabilities>` for the first, then
//!   `<old-oid> <new-oid> <refname>` for each subsequent).
//! * [`pack_writer`] — stream a pack to the wire after the command list.
//! * [`report`] — parse `unpack <status>` plus `ok <ref>` / `ng <ref> <reason>`.
//! * [`function`] — orchestrator; ties the three together over a [`Transport`].
//! * [`arguments`] / [`response`] — I/O-shaped pairs analogous to
//!   [`crate::fetch::Arguments`] and [`crate::fetch::Response`].
//!
//! See `vendor/git/send-pack.c` for the reference implementation. The protocol
//! shape does not version by V1/V2 the way fetch does — `git-receive-pack`
//! speaks the same command-list dialect in both. However the *transport*
//! handshake uses whichever protocol the caller negotiated; send-pack takes a
//! post-handshake transport.
#![allow(missing_docs)] // until the API settles

pub mod arguments;
pub mod command_list;
pub mod function;
pub mod pack_writer;
pub mod report;
pub mod response;

mod types;
pub use types::{Command, Error, Options, Outcome, RefStatus, Report, Request};

#[cfg(feature = "blocking-client")]
pub use function::send_pack;
