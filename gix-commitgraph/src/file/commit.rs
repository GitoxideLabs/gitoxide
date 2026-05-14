//! Low-level operations on individual commits.
use crate::{
    File, Position,
    file::{self, EXTENDED_EDGES_MASK, LAST_EXTENDED_EDGE_MASK, NO_PARENT},
};
use gix_error::{Message, message};
use std::{
    fmt::{Debug, Formatter},
    slice::Chunks,
};

/// A commit as stored in a [`File`].
#[derive(Copy, Clone)]
pub struct Commit<'a> {
    file: &'a File,
    pos: file::Position,
    // We can parse the below fields lazily if needed.
    commit_timestamp: u64,
    generation: u32,
    parent1: ParentEdge,
    parent2: ParentEdge,
    root_tree_id: &'a gix_hash::oid,
}

#[inline]
fn read_u32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b.try_into().unwrap())
}

impl<'a> Commit<'a> {
    pub(crate) fn new(file: &'a File, pos: file::Position) -> Self {
        let bytes = file.commit_data_bytes(pos);
        Commit {
            file,
            pos,
            root_tree_id: gix_hash::oid::from_bytes_unchecked(&bytes[..file.hash_len]),
            parent1: ParentEdge::from_raw(read_u32(&bytes[file.hash_len..][..4])),
            parent2: ParentEdge::from_raw(read_u32(&bytes[file.hash_len + 4..][..4])),
            // TODO: Add support for corrected commit date offset overflow.
            //      See https://github.com/git/git/commit/e8b63005c48696a26f976f5f9b0ccaf1983e439d and
            //          https://github.com/git/git/commit/f90fca638e99a031dce8e3aca72427b2f9b4bb38 for more details and hints at a test.
            generation: read_u32(&bytes[file.hash_len + 8..][..4]) >> 2,
            commit_timestamp: u64::from_be_bytes(bytes[file.hash_len + 8..][..8].try_into().unwrap())
                & 0x0003_ffff_ffff,
        }
    }

    /// Returns the committer timestamp of this commit.
    ///
    /// The value is the number of seconds since 1970-01-01 00:00:00 UTC.
    pub fn committer_timestamp(&self) -> u64 {
        self.commit_timestamp
    }

    /// Returns the generation number of this commit.
    ///
    /// Commits without parents have generation number 1. Commits with parents have a generation
    /// number that is the max of their parents' generation numbers + 1.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns an iterator over the parent positions for lookup in the owning [Graph][crate::Graph].
    pub fn iter_parents(self) -> Parents<'a> {
        // I didn't find a combinator approach that a) was as strict as ParentIterator, b) supported
        // fuse-after-first-error behavior, and b) was significantly shorter or more understandable
        // than ParentIterator. So here we are.
        Parents {
            commit_data: self,
            state: ParentIteratorState::First,
        }
    }

    /// Returns the hash of this commit.
    pub fn id(&self) -> &'a gix_hash::oid {
        self.file.id_at(self.pos)
    }

    /// Returns the first parent of this commit.
    pub fn parent1(&self) -> Result<Option<Position>, Message> {
        self.iter_parents().next().transpose()
    }

    /// Returns the position at which this commit is stored in the parent [File].
    pub fn position(&self) -> file::Position {
        self.pos
    }

    /// Return the hash of the tree this commit points to.
    pub fn root_tree_id(&self) -> &gix_hash::oid {
        self.root_tree_id
    }
}

impl Debug for Commit<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commit {{ id: {}, lex_pos: {}, generation: {}, root_tree_id: {}, parent1: {:?}, parent2: {:?} }}",
            self.id(),
            self.pos,
            self.generation(),
            self.root_tree_id(),
            self.parent1,
            self.parent2,
        )
    }
}

impl Eq for Commit<'_> {}

impl PartialEq for Commit<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos
    }
}

/// An iterator over parents of a [`Commit`].
pub struct Parents<'a> {
    commit_data: Commit<'a>,
    state: ParentIteratorState<'a>,
}

impl Iterator for Parents<'_> {
    type Item = Result<Position, Message>;

    fn next(&mut self) -> Option<Self::Item> {
        let state = std::mem::replace(&mut self.state, ParentIteratorState::Exhausted);
        match state {
            ParentIteratorState::First => match self.commit_data.parent1 {
                ParentEdge::None => match self.commit_data.parent2 {
                    ParentEdge::None => None,
                    _ => Some(Err(message!(
                        "commit {} has a second parent but not a first parent",
                        self.commit_data.id()
                    ))),
                },
                ParentEdge::GraphPosition(pos) => {
                    self.state = ParentIteratorState::Second;
                    Some(Ok(pos))
                }
                ParentEdge::ExtraEdgeIndex(_) => Some(Err(message!(
                    "commit {}'s first parent is an extra edge index, which is invalid",
                    self.commit_data.id(),
                ))),
            },
            ParentIteratorState::Second => match self.commit_data.parent2 {
                ParentEdge::None => None,
                ParentEdge::GraphPosition(pos) => Some(Ok(pos)),
                ParentEdge::ExtraEdgeIndex(extra_edge_index) => {
                    if let Some(extra_edges_list) = self.commit_data.file.extra_edges_data() {
                        let start_offset: usize = extra_edge_index
                            .try_into()
                            .expect("an architecture able to hold 32 bits of integer");
                        let start_offset = start_offset
                            .checked_mul(4)
                            .expect("an extended edge index small enough to fit in usize");
                        if let Some(tail) = extra_edges_list.get(start_offset..) {
                            self.state = ParentIteratorState::Extra(tail.chunks(4));
                            // This recursive call is what blocks me from replacing ParentIterator
                            // with a std::iter::from_fn closure.
                            self.next()
                        } else {
                            Some(Err(message!(
                                "commit {}'s extra edges overflows the commit-graph file's extra edges list",
                                self.commit_data.id()
                            )))
                        }
                    } else {
                        Some(Err(message!(
                            "commit {} has extra edges, but commit-graph file has no extra edges list",
                            self.commit_data.id()
                        )))
                    }
                }
            },
            ParentIteratorState::Extra(mut chunks) => {
                if let Some(chunk) = chunks.next() {
                    let extra_edge = read_u32(chunk);
                    match ExtraEdge::from_raw(extra_edge) {
                        ExtraEdge::Internal(pos) => {
                            self.state = ParentIteratorState::Extra(chunks);
                            Some(Ok(pos))
                        }
                        ExtraEdge::Last(pos) => Some(Ok(pos)),
                    }
                } else {
                    Some(Err(message!(
                        "commit {}'s extra edges overflows the commit-graph file's extra edges list",
                        self.commit_data.id()
                    )))
                }
            }
            ParentIteratorState::Exhausted => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match (&self.state, self.commit_data.parent1, self.commit_data.parent2) {
            (ParentIteratorState::First, ParentEdge::None, ParentEdge::None) => (0, Some(0)),
            (ParentIteratorState::First, ParentEdge::None, _) => (1, Some(1)),
            (ParentIteratorState::First, ParentEdge::GraphPosition(_), ParentEdge::None) => (1, Some(1)),
            (ParentIteratorState::First, ParentEdge::GraphPosition(_), ParentEdge::GraphPosition(_)) => (2, Some(2)),
            (ParentIteratorState::First, ParentEdge::GraphPosition(_), ParentEdge::ExtraEdgeIndex(_)) => (3, None),
            (ParentIteratorState::First, ParentEdge::ExtraEdgeIndex(_), _) => (1, Some(1)),
            (ParentIteratorState::Second, _, ParentEdge::None) => (0, Some(0)),
            (ParentIteratorState::Second, _, ParentEdge::GraphPosition(_)) => (1, Some(1)),
            (ParentIteratorState::Second, _, ParentEdge::ExtraEdgeIndex(_)) => (2, None),
            (ParentIteratorState::Extra(_), _, _) => (1, None),
            (ParentIteratorState::Exhausted, _, _) => (0, Some(0)),
        }
    }
}

#[derive(Debug)]
enum ParentIteratorState<'a> {
    First,
    Second,
    Extra(Chunks<'a, u8>),
    Exhausted,
}

#[derive(Clone, Copy, Debug)]
enum ParentEdge {
    None,
    GraphPosition(Position),
    ExtraEdgeIndex(u32),
}

impl ParentEdge {
    pub fn from_raw(raw: u32) -> ParentEdge {
        if raw == NO_PARENT {
            return ParentEdge::None;
        }
        if raw & EXTENDED_EDGES_MASK != 0 {
            ParentEdge::ExtraEdgeIndex(raw & !EXTENDED_EDGES_MASK)
        } else {
            ParentEdge::GraphPosition(Position(raw))
        }
    }
}

enum ExtraEdge {
    Internal(Position),
    Last(Position),
}

impl ExtraEdge {
    pub fn from_raw(raw: u32) -> Self {
        if raw & LAST_EXTENDED_EDGE_MASK != 0 {
            Self::Last(Position(raw & !LAST_EXTENDED_EDGE_MASK))
        } else {
            Self::Internal(Position(raw))
        }
    }
}
