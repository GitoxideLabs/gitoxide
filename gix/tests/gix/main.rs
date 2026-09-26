#![allow(clippy::result_large_err, clippy::unnecessary_debug_formatting)]
mod util;
use util::*;

mod clone;
mod commit;
mod config;
#[cfg(feature = "blob-diff")]
mod diff;
mod error;
mod head;
mod id;
mod init;
mod object;
mod reference;
mod remote;
mod repository;
#[cfg(feature = "revision")]
mod revision;
#[cfg(feature = "status")]
mod status;
#[cfg(feature = "attributes")]
mod submodule;
