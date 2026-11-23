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
#![deny(missing_docs, unsafe_code)]

use gix_error::Result;
use std::path::Path;

/// A single commit-graph file.
///
/// All operations on a `File` are local to that graph file. Since a commit graph can span multiple
/// files, all interesting graph operations belong on [`Graph`].
pub struct File {
    /// Number of base graph files this file depends on, as declared in its header.
    base_graph_count: u8,
    /// Byte offset into `data` of the optional BASE chunk containing base graph checksums.
    base_graphs_list_offset: Option<usize>,
    /// Byte offset into `data` of the CDAT chunk containing fixed-size commit records.
    commit_data_offset: usize,
    /// Byte offset of corrected commit date offsets, one `u32` per commit.
    generation_data_offset: Option<usize>,
    /// Byte range of corrected commit date offsets that do not fit in 31 bits.
    generation_overflow_range: Option<std::ops::Range<usize>>,
    /// Read-only memory mapping of the entire commit-graph file, including its checksum trailer.
    data: memmap2::Mmap,
    /// Byte range within `data` of the optional EDGE chunk containing additional parent positions.
    extra_edges_list_range: Option<std::ops::Range<usize>>,
    /// Decoded OIDF fanout table: entry `i` counts commit IDs whose first byte is at most `i`.
    /// Entries are nondecreasing, and the last entry is the number of commits in this file.
    fan: [u32; file::FAN_LEN],
    /// Byte offset into `data` of the OIDL chunk containing lexicographically sorted commit IDs.
    oid_lookup_offset: usize,
    /// Path associated with the mapping, used for diagnostics and filename checksum verification.
    path: std::path::PathBuf,
    /// Size in bytes of an object ID or checksum using `object_hash`.
    hash_len: usize,
    /// Hash algorithm declared in the file header, used for object IDs and graph checksums.
    object_hash: gix_hash::Kind,
    /// Exclusive upper bound for graph-wide parent positions accepted during iteration.
    /// In a [`Graph`], this counts commits in this file and all preceding base files.
    /// A standalone file uses its commit count if it has no bases, or [`MAX_COMMITS`] otherwise.
    parent_position_limit: u32,
}

/// A complete commit graph.
///
/// The data in the commit graph may come from a monolithic `objects/info/commit-graph` file, or it
/// may come from one or more `objects/info/commit-graphs/graph-*.graph` files. These files are
/// generated via `git commit-graph write ...` commands.
pub struct Graph {
    files: nonempty::NonEmpty<File>,
    corrected_commit_dates_available: bool,
}

/// Owned parent commit information collected during traversal.
///
/// Shared with traversal buffers so allocation-limit estimates use the same record layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parent {
    /// The object ID of the parent commit.
    pub commit_id: gix_hash::ObjectId,
    /// The parent's generation number, or [`GENERATION_NUMBER_INFINITY`] if unavailable.
    pub generation: u32,
    /// The parent's committer timestamp, in seconds since the Unix epoch.
    pub commit_time: i64,
}

/// Instantiate a commit graph from an `.git/objects/info` directory, or one of the various commit-graph files.
///
/// Chain entries and graph files must use `object_hash`, typically the repository's hash kind.
/// `alloc_limit_bytes` limits parent-buffer capacity estimates as described in [`Graph::at()`].
/// `None` disables the limit.
pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind, alloc_limit_bytes: Option<usize>) -> Result<Graph> {
    Graph::at(path.as_ref(), object_hash, alloc_limit_bytes)
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
