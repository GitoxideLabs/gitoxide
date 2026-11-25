mod util;
use util::*;

mod clone;
mod clone_refs;
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
#[cfg(feature = "status-monitor")]
mod status_monitor;
#[cfg(feature = "attributes")]
mod submodule;
