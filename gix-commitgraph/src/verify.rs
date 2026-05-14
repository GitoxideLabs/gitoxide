//! Auxiliary types used by graph verification methods.
use std::{
    cmp::{max, min},
    collections::BTreeMap,
};

use gix_error::{ErrorExt, Exn, Message, ResultExt, message};

use crate::{
    GENERATION_NUMBER_MAX, Graph, Position,
    file::{self},
};

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
    ) -> Result<Outcome, Exn<Message>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if self.files.len() > 256 {
            // A file in a split chain can only have up to 255 base files.
            return Err(message!(
                "Commit-graph should be composed of at most 256 files but actually contains {} files",
                self.files.len()
            )
            .raise());
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
                return Err(message!(
                    "'{}' should have {} base graphs, but claims {} base graphs",
                    file.path().display(),
                    file_index,
                    file.base_graph_count()
                )
                .raise());
            }

            for (base_graph_index, (expected, actual)) in self
                .files
                .iter()
                .take(file_index)
                .map(crate::File::checksum)
                .zip(file.iter_base_graph_ids())
                .enumerate()
            {
                if actual != expected {
                    return Err(message!(
                        "'{}' base graph at index {} should have ID {} but is {}",
                        file.path().display(),
                        base_graph_index,
                        expected,
                        actual
                    )
                    .raise());
                }
            }

            let next_file_start_pos = Position(file_start_pos.0 + file.num_commits());
            let file_stats = file.traverse(|commit| {
                let mut max_parent_generation = 0u32;
                for parent_pos in commit.iter_parents() {
                    let parent_pos = parent_pos.map_err(|err| err.raise_erased())?;
                    if parent_pos >= next_file_start_pos {
                        return Err(message!(
                            "Commit {} has parent position {parent_pos} that is out of range (should be in range 0-{})",
                            commit.id(),
                            Position(next_file_start_pos.0 - 1)
                        )
                        .raise_erased());
                    }
                    let parent = self.commit_at(parent_pos);
                    max_parent_generation = max(max_parent_generation, parent.generation());
                }

                // If the max parent generation is GENERATION_NUMBER_MAX, then this commit's
                // generation should be GENERATION_NUMBER_MAX too.
                let expected_generation = min(max_parent_generation + 1, GENERATION_NUMBER_MAX);
                if commit.generation() != expected_generation {
                    return Err(message!(
                        "Commit {}'s generation should be {expected_generation} but is {}",
                        commit.id(),
                        commit.generation()
                    )
                    .raise_erased());
                }

                processor(commit).or_raise_erased(|| message!("processor failed on commit {id}", id = commit.id()))?;

                Ok(())
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
