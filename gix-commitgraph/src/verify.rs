//! Auxiliary types used by graph verification methods.
use std::{
    cmp::{max, min},
    collections::BTreeMap,
    path::PathBuf,
};

use crate::{
    file::{self, commit},
    Graph, Position, GENERATION_NUMBER_MAX,
};

/// The error used in [`verify_integrity()`][Graph::verify_integrity].
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error<E: std::error::Error + 'static> {
    #[error("'{}' should have {expected} base graphs, but claims {actual} base graphs", .path.display())]
    BaseGraphCount { actual: u8, expected: u8, path: PathBuf },
    #[error("'{}' base graph at index {index} should have ID {expected} but is {actual}", .path.display())]
    BaseGraphId {
        actual: gix_hash::ObjectId,
        expected: gix_hash::ObjectId,
        index: u8,
        path: PathBuf,
    },
    #[error(transparent)]
    Commit(#[from] commit::Error),
    #[error("{}: {err}", .path.display())]
    File {
        // Use zero-size error type. We will never return
        // `graph::verify::Error::File(file::verify::Error::Processor(...))`, because we are the
        // file's processor, and we convert`file::verify::Error::Processor<graph::verify::Error>`
        // variants into direct `graph::verify::Error` values.
        err: file::verify::Error<std::convert::Infallible>,
        path: PathBuf,
    },
    #[error("Commit {id}'s generation should be {expected} but is {actual}")]
    Generation {
        actual: u32,
        expected: u32,
        id: gix_hash::ObjectId,
    },
    #[error(
        "Commit {id} has parent position {parent_pos} that is out of range (should be in range 0-{max_valid_pos})"
    )]
    ParentOutOfRange {
        id: gix_hash::ObjectId,
        max_valid_pos: Position,
        parent_pos: Position,
    },
    #[error("{0}")]
    Processor(#[source] E),
    #[error("Commit-graph should be composed of at most 256 files but actually contains {0} files")]
    TooManyFiles(usize),
}

/// Statistics gathered while verifying the integrity of the graph as returned by [`Graph::verify_integrity()`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Outcome {
    /// The length of the longest path between any two commits in this graph.
    ///
    /// For example, this will be `Some(9)` for a commit graph containing 10 linear commits.
    /// This will be `Some(0)` for a commit graph containing 0 or 1 commits.
    /// If the longest path length is too large to fit in a [u32], then this will be [None].
    pub longest_path_length: Option<u32>,
    /// The total number of commits traversed.
    pub num_commits: u32,
    /// A mapping of `N -> number of commits with N parents`.
    pub parent_counts: BTreeMap<u32, u32>,
}

impl Graph {
    /// Traverse all commits in the graph and call `processor(&commit) -> Result<(), E>` on it while verifying checksums.
    ///
    /// When `processor` returns an error, the entire verification is stopped and the error returned.
    pub fn verify_integrity<E>(
        &self,
        mut processor: impl FnMut(&file::Commit<'_>) -> Result<(), E>,
    ) -> Result<Outcome, Error<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if self.files.len() > 256 {
            // A file in a split chain can only have up to 255 base files.
            return Err(Error::TooManyFiles(self.files.len()));
        }

        let mut stats = Outcome {
            longest_path_length: None,
            num_commits: 0,
            parent_counts: BTreeMap::new(),
        };
        let mut max_generation = 0u32;

        // TODO: Detect duplicate commit IDs across different files. Not sure how to do this without
        //   a separate loop, e.g. self.iter_sorted_ids().

        let mut file_start_pos = Position(0);
        for (file_index, file) in self.files.iter().enumerate() {
            if usize::from(file.base_graph_count()) != file_index {
                return Err(Error::BaseGraphCount {
                    actual: file.base_graph_count(),
                    expected: file_index
                        .try_into()
                        .expect("files.len() check to protect against this"),
                    path: file.path().to_owned(),
                });
            }

            for (base_graph_index, (expected, actual)) in self.files[..file_index]
                .iter()
                .map(crate::File::checksum)
                .zip(file.iter_base_graph_ids())
                .enumerate()
            {
                if actual != expected {
                    return Err(Error::BaseGraphId {
                        actual: actual.into(),
                        expected: expected.into(),
                        index: base_graph_index
                            .try_into()
                            .expect("files.len() check to protect against this"),
                        path: file.path().to_owned(),
                    });
                }
            }

            let next_file_start_pos = Position(file_start_pos.0 + file.num_commits());
            let file_stats = file
                .traverse(|commit| {
                    let mut max_parent_generation = 0u32;
                    for parent_pos in commit.iter_parents() {
                        let parent_pos = parent_pos.map_err(Error::Commit)?;
                        if parent_pos >= next_file_start_pos {
                            return Err(Error::ParentOutOfRange {
                                parent_pos,
                                id: commit.id().into(),
                                max_valid_pos: Position(next_file_start_pos.0 - 1),
                            });
                        }
                        let parent = self.commit_at(parent_pos);
                        max_parent_generation = max(max_parent_generation, parent.generation());
                    }

                    // If the max parent generation is GENERATION_NUMBER_MAX, then this commit's
                    // generation should be GENERATION_NUMBER_MAX too.
                    let expected_generation = min(max_parent_generation + 1, GENERATION_NUMBER_MAX);
                    if commit.generation() != expected_generation {
                        return Err(Error::Generation {
                            actual: commit.generation(),
                            expected: expected_generation,
                            id: commit.id().into(),
                        });
                    }

                    processor(commit).map_err(Error::Processor)?;

                    Ok(())
                })
                .map_err(|err| Error::File {
                    err: match err {
                        file::verify::Error::Processor(e) => return e,
                        file::verify::Error::RootTreeId { id, root_tree_id } => {
                            file::verify::Error::RootTreeId { id, root_tree_id }
                        }
                        file::verify::Error::Checksum(err) => file::verify::Error::Checksum(err),
                        file::verify::Error::Generation { generation, id } => {
                            file::verify::Error::Generation { generation, id }
                        }
                        file::verify::Error::Filename(expected) => file::verify::Error::Filename(expected),
                        file::verify::Error::Commit(err) => file::verify::Error::Commit(err),
                        file::verify::Error::CommitId { id, pos } => file::verify::Error::CommitId { id, pos },
                        file::verify::Error::CommitsOutOfOrder {
                            id,
                            pos,
                            predecessor_id,
                        } => file::verify::Error::CommitsOutOfOrder {
                            id,
                            pos,
                            predecessor_id,
                        },
                    },
                    path: file.path().to_owned(),
                })?;

            max_generation = max(max_generation, file_stats.max_generation);
            stats.num_commits += file_stats.num_commits;
            for (key, value) in file_stats.parent_counts.into_iter() {
                *stats.parent_counts.entry(key).or_insert(0) += value;
            }
            file_start_pos = next_file_start_pos;
        }

        stats.longest_path_length = if max_generation < GENERATION_NUMBER_MAX {
            Some(max_generation.saturating_sub(1))
        } else {
            None
        };
        Ok(stats)
    }
}
