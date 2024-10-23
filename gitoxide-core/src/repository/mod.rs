use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result};
use gix::bstr::BString;

pub fn init(directory: Option<PathBuf>) -> Result<gix::discover::repository::Path> {
    gix::create::into(
        directory.unwrap_or_default(),
        gix::create::Kind::WithWorktree,
        gix::create::Options::default(),
    )
    .with_context(|| "Repository initialization failed")
}

pub enum PathsOrPatterns {
    Paths(Box<dyn std::iter::Iterator<Item = BString>>),
    Patterns(Vec<BString>),
}

#[cfg(feature = "archive")]
pub mod archive;
pub mod cat;
pub use cat::function::cat;
pub mod commit;
pub mod config;
mod credential;
pub use credential::function as credential;
pub mod attributes;
#[cfg(feature = "clean")]
pub mod clean;
pub mod diff;
pub mod dirty;
#[cfg(feature = "clean")]
pub use clean::function::clean;
#[cfg(feature = "blocking-client")]
pub mod clone;
pub mod exclude;
#[cfg(feature = "blocking-client")]
pub mod fetch;
#[cfg(feature = "blocking-client")]
pub use clone::function::clone;
#[cfg(feature = "blocking-client")]
pub use fetch::function::fetch;

pub mod commitgraph;
mod fsck;
pub use fsck::function as fsck;
pub mod index;
pub mod log;
pub mod mailmap;
mod merge_base;
pub use merge_base::merge_base;
pub mod merge;
pub mod odb;
pub mod remote;
pub mod revision;
pub mod status;
pub mod submodule;
pub mod tree;
pub mod verify;
pub mod worktree;
