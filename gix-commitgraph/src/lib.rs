//! Read, verify, and traverse git commit graphs.
//!
//! A [commit graph][Graph] is an index of commits in the git commit history.
//! The [Graph] stores commit data in a way that accelerates lookups considerably compared to
//! traversing the git history by usual means.
//!
//! As generating the full commit graph from scratch can take some time, git may write new commits
//! to separate [files][File] instead of overwriting the original file.
//! Eventually, git will merge these files together as the number of files grows.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

use gix_error::{Exn, Message};
use std::path::Path;

/// A single commit-graph file.
///
/// All operations on a `File` are local to that graph file. Since a commit graph can span multiple
/// files, all interesting graph operations belong on [`Graph`].
pub struct File {
    base_graph_count: u8,
    base_graphs_list_offset: Option<usize>,
    commit_data_offset: usize,
    data: memmap2::Mmap,
    extra_edges_list_range: Option<std::ops::Range<usize>>,
    fan: [u32; file::FAN_LEN],
    oid_lookup_offset: usize,
    path: std::path::PathBuf,
    hash_len: usize,
    object_hash: gix_hash::Kind,
}

/// A complete commit graph.
///
/// The data in the commit graph may come from a monolithic `objects/info/commit-graph` file, or it
/// may come from one or more `objects/info/commit-graphs/graph-*.graph` files. These files are
/// generated via `git commit-graph write ...` commands.
pub struct Graph {
    files: Vec<File>,
}

/// Instantiate a commit graph from an `.git/objects/info` directory, or one of the various commit-graph files.
pub fn at(path: impl AsRef<Path>) -> Result<Graph, Exn<Message>> {
    Graph::at(path.as_ref())
}

mod access;
pub mod file;
///
pub mod init;
pub mod verify;

/// The number of generations that are considered 'infinite' commit history.
pub const GENERATION_NUMBER_INFINITY: u32 = 0xffff_ffff;
/// The largest valid generation number.
///
/// If a commit's real generation number is larger than this, the commit graph will cap the value to
/// this number.
/// The largest distinct generation number is `GENERATION_NUMBER_MAX - 1`.
pub const GENERATION_NUMBER_MAX: u32 = 0x3fff_ffff;

/// The maximum number of commits that can be stored in a commit graph.
pub const MAX_COMMITS: u32 = (1 << 30) + (1 << 29) + (1 << 28) - 1;

/// A generalized position for use in [`Graph`].
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Position(pub u32);

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
