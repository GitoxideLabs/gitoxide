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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parent {
    /// The object ID of the parent commit.
    pub commit_id: gix_hash::ObjectId,
    /// The parent's generation number, or [`Generation::INFINITY`] if unavailable.
    pub generation: Generation,
    /// The parent's committer timestamp, in seconds since the Unix epoch.
    pub commit_time: i64,
}

/// Instantiate a commit graph from an `.git/objects/info` directory, or one of the various commit-graph files.
///
/// Chain entries and graph files must use `object_hash`, typically the repository's hash kind.
pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Graph> {
    Graph::at(path.as_ref(), object_hash)
}

mod access;
pub mod file;
///
pub mod init;
pub mod verify;

/// A topological (version 1) generation number from a commit-graph.
///
/// A root has generation 1. Other commits have one more than their highest parent generation,
/// capped at [`Self::MAX`]. Raw values are retained even when they cannot be used
/// for ancestry cutoffs; use [`Self::is_reliable`] to filter them.
///
/// [`Display`](std::fmt::Display) shows the number, or `uncomputed`, `capped`, or `unavailable`
/// for the special values. Use [`Self::get`] when the raw number is needed.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize), serde(transparent))]
pub struct Generation(u32);

impl Generation {
    /// A commit-graph entry whose generation was not computed.
    pub const UNCOMPUTED: Self = Self(0);
    /// The largest generation that can be stored in a commit-graph. Higher generations are capped at this value.
    pub const MAX: Self = Self(0x3fff_ffff);
    /// A traversal sentinel for commits outside the commit-graph. It is never stored on disk.
    pub const INFINITY: Self = Self(u32::MAX);

    /// Wrap a raw generation number without checking whether it can be used for ancestry cutoffs.
    const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the raw generation number, including the numeric encoding of special values.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Return whether this is a computed, unsaturated generation. In a valid commit-graph,
    /// the parents of such a commit have strictly lower generations.
    #[inline]
    pub const fn is_reliable(&self) -> bool {
        self.0 > Self::UNCOMPUTED.0 && self.0 < Self::MAX.0
    }
}

impl std::fmt::Display for Generation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::UNCOMPUTED => f.pad("uncomputed"),
            Self::MAX => f.pad("capped"),
            Self::INFINITY => f.pad("unavailable"),
            Self(raw) => raw.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Generation;

    #[test]
    fn generation_just_above_the_on_disk_limit_is_unreliable() {
        let generation = Generation::new(Generation::MAX.0 + 1);
        assert!(!generation.is_reliable(), "only generations below the cap are reliable");
    }
}

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
