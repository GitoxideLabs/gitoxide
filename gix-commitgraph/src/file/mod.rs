//! Operations on a single commit-graph file.

use std::fmt::{Display, Formatter};

pub use self::commit::Commit;

mod access;
pub mod commit;
mod init;
pub mod verify;

const COMMIT_DATA_ENTRY_SIZE_SANS_HASH: usize = 16;
pub(crate) const FAN_LEN: usize = 256;
const HEADER_LEN: usize = 8;

const SIGNATURE: &[u8] = b"CGPH";

type ChunkId = gix_chunk::Id;
const BASE_GRAPHS_LIST_CHUNK_ID: ChunkId = *b"BASE";
const BLOOM_FILTER_DATA_CHUNK_ID: ChunkId = *b"BDAT";
const BLOOM_FILTER_INDEX_CHUNK_ID: ChunkId = *b"BIDX";
const BLOOM_FILTER_HEADER_SIZE: usize = 3 * std::mem::size_of::<u32>();
const COMMIT_DATA_CHUNK_ID: ChunkId = *b"CDAT";
const EXTENDED_EDGES_LIST_CHUNK_ID: ChunkId = *b"EDGE";
const OID_FAN_CHUNK_ID: ChunkId = *b"OIDF";
const OID_LOOKUP_CHUNK_ID: ChunkId = *b"OIDL";

// Note that git's commit-graph-format.txt as of v2.28.0 gives an incorrect value 0x0700_0000 for
// NO_PARENT. Fixed in https://github.com/git/git/commit/4d515253afcef985e94400adbfed7044959f9121 .
const NO_PARENT: u32 = 0x7000_0000;
const EXTENDED_EDGES_MASK: u32 = 0x8000_0000;
const LAST_EXTENDED_EDGE_MASK: u32 = 0x8000_0000;

/// The position of a given commit within a graph file, starting at 0.
///
/// Commits within a graph file are sorted in lexicographical order by OID; a commit's lexicographical position
/// is its position in this ordering. If a commit graph spans multiple files, each file's commits
/// start at lexicographical position 0, so it is unique across a single file but is not unique across
/// the whole commit graph. Each commit also has a graph position ([`Position`][crate::Position]),
/// which is unique across the whole commit graph.
/// In order to avoid accidentally mixing lexicographical positions with graph positions, distinct types are used for each.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Position(pub u32);

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
