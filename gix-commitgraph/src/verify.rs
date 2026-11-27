//! Auxiliary types used by graph verification methods.
use gix_error::Result;
use std::{cmp::max, collections::BTreeMap};

use gix_error::{ResultExt, bail};

use crate::{
    Generation, Graph,
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
    /// If the maximum generation is capped, the exact path length is unknown and this is [None].
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
        mut processor: impl FnMut(&file::Commit<'_>) -> std::result::Result<(), E>,
    ) -> Result<Outcome>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut stats = Outcome {
            longest_path_length: None,
            num_commits: 0,
            parent_counts: BTreeMap::new(),
        };
        let mut max_generation = Generation::UNCOMPUTED;

        // TODO: Detect duplicate commit IDs across different files. Not sure how to do this without
        //   a separate loop, e.g. self.iter_sorted_ids().

        for (file_index, file) in self.files.iter().enumerate() {
            if usize::from(file.base_graph_count()) != file_index {
                bail!(
                    "\"{}\" should have {} base graphs, but claims {} base graphs".corrupted(),
                    file.path().display(),
                    file_index,
                    file.base_graph_count()
                );
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
                    bail!(
                        "\"{}\" base graph at index {} should have ID {} but is {}".corrupted(),
                        file.path().display(),
                        base_graph_index,
                        expected,
                        actual
                    );
                }
            }

            let file_stats = file.traverse(|commit| {
                let mut max_parent_generation = Generation::UNCOMPUTED;
                let mut has_uncomputed_parent_generation = false;
                for parent_pos in commit.iter_parents() {
                    let parent_pos = parent_pos?;
                    let generation = self.commit_at(parent_pos).generation();
                    max_parent_generation = max(max_parent_generation, generation);
                    has_uncomputed_parent_generation |= generation == Generation::UNCOMPUTED;
                }

                // Zero denotes legacy, uncomputed generations, not corrupt data.
                let generation = commit.generation();
                if generation == Generation::UNCOMPUTED || has_uncomputed_parent_generation {
                    bail!(
                        "Cannot verify generation numbers for commit {} because it or a parent has an uncomputed generation".unsupported(),
                        commit.id()
                    );
                }

                let expected_generation = max_parent_generation.child_generation();
                if generation != expected_generation {
                    bail!(
                        "Commit {}'s generation should be {expected_generation} but is {generation}".corrupted(),
                        commit.id()
                    );
                }

                processor(commit).or_error()?;

                Ok(())
            })?;

            max_generation = max(max_generation, file_stats.max_generation);
            stats.num_commits += file_stats.num_commits;
            for (key, value) in file_stats.parent_counts.into_iter() {
                *stats.parent_counts.entry(key).or_insert(0) += value;
            }
        }

        stats.longest_path_length = max_generation.path_length();
        Ok(stats)
    }
}

impl Generation {
    /// The child's generation given its highest parent generation (zero if it has no parents).
    fn child_generation(self) -> Self {
        Self::new(self.0.saturating_add(1)).min(Self::MAX)
    }

    /// Convert the maximum generation to an edge count. Zero also represents an empty graph here.
    fn path_length(self) -> Option<u32> {
        (self < Self::MAX).then_some(self.0.saturating_sub(1))
    }
}
