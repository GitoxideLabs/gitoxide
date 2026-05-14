//! Auxiliary types used in commit graph file verification methods.
use std::{
    cmp::{max, min},
    collections::HashMap,
    path::Path,
};

use gix_error::{ErrorExt, Exn, Message, ResultExt, message};

use crate::{File, GENERATION_NUMBER_INFINITY, GENERATION_NUMBER_MAX, file};

/// The positive result of [`File::traverse()`] providing some statistical information.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Outcome {
    /// The largest encountered [`file::Commit`] generation number.
    pub max_generation: u32,
    /// The smallest encountered [`file::Commit`] generation number.
    pub min_generation: u32,
    /// The largest number of parents in a single [`file::Commit`].
    pub max_parents: u32,
    /// The total number of [`commits`][file::Commit]s seen in the iteration.
    pub num_commits: u32,
    /// A mapping of `N -> number of commits with N parents`.
    pub parent_counts: HashMap<u32, u32>,
}

/// Verification
impl File {
    /// Returns the trailing checksum over the entire content of this file.
    pub fn checksum(&self) -> &gix_hash::oid {
        gix_hash::oid::from_bytes_unchecked(&self.data[self.data.len() - self.hash_len..])
    }

    /// Traverse all [commits][file::Commit] stored in this file and call `processor(commit) -> Result<(), Error>` on it.
    ///
    /// If the `processor` fails, the iteration will be stopped and the entire call results in the respective error.
    pub fn traverse<'a, Processor>(&'a self, mut processor: Processor) -> Result<Outcome, Exn<Message>>
    where
        Processor: FnMut(&file::Commit<'a>) -> Result<(), Exn>,
    {
        self.verify_checksum()?;
        verify_split_chain_filename_hash(&self.path, self.checksum())?;

        let null_id = self.object_hash().null_ref();

        let mut stats = Outcome {
            max_generation: 0,
            max_parents: 0,
            min_generation: GENERATION_NUMBER_INFINITY,
            num_commits: self.num_commits(),
            parent_counts: HashMap::new(),
        };

        // TODO: Verify self.fan values as we go.
        let mut prev_id: &gix_hash::oid = null_id;
        for commit in self.iter_commits() {
            if commit.id() <= prev_id {
                if commit.id() == null_id {
                    return Err(message!(
                        "commit at file position {} has invalid ID {}",
                        commit.position(),
                        commit.id()
                    )
                    .raise());
                }
                return Err(message!(
                    "commit at file position {} with ID {} is out of order relative to its predecessor with ID {prev_id}",
                    commit.position(),
                    commit.id()
                )
                .raise());
            }
            if commit.root_tree_id() == null_id {
                return Err(message!(
                    "commit {} has invalid root tree ID {}",
                    commit.id(),
                    commit.root_tree_id()
                )
                .raise());
            }
            if commit.generation() > GENERATION_NUMBER_MAX {
                return Err(message!("commit {} has invalid generation {}", commit.id(), commit.generation()).raise());
            }

            processor(&commit).or_raise(|| message!("processor failed on commit {}", commit.id()))?;

            stats.max_generation = max(stats.max_generation, commit.generation());
            stats.min_generation = min(stats.min_generation, commit.generation());
            let parent_count = commit.iter_parents().try_fold(0u32, |acc, pos| pos.map(|_| acc + 1))?;
            *stats.parent_counts.entry(parent_count).or_insert(0) += 1;
            prev_id = commit.id();
        }

        if stats.min_generation == GENERATION_NUMBER_INFINITY {
            stats.min_generation = 0;
        }

        Ok(stats)
    }

    /// Assure the [`checksum`][File::checksum()] matches the actual checksum over all content of this file, excluding the trailing
    /// checksum itself.
    ///
    /// Return the actual checksum on success or [`Exn<Message>`] if there is a mismatch.
    pub fn verify_checksum(&self) -> Result<gix_hash::ObjectId, Exn<Message>> {
        // Even though we could use gix_hash::bytes_of_file(…), this would require extending our
        // Error type to support io::Error. As we only gain progress, there probably isn't much value
        // as these files are usually small enough to process them in less than a second, even for the large ones.
        // But it's possible, once a progress instance is passed.
        let data_len_without_trailer = self.data.len() - self.hash_len;
        let mut hasher = gix_hash::hasher(self.object_hash());
        hasher.update(&self.data[..data_len_without_trailer]);
        let actual = hasher
            .try_finalize()
            .map_err(|e| message!("failed to hash commit graph file: {e}").raise())?;
        actual.verify(self.checksum()).map_err(|e| message!("{e}").raise())?;
        Ok(actual)
    }
}

/// If the given path's filename matches "graph-{hash}.graph", check that `hash` matches the
/// expected hash.
fn verify_split_chain_filename_hash(path: &Path, expected: &gix_hash::oid) -> Result<(), Exn<Message>> {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(|filename| filename.strip_suffix(".graph"))
        .and_then(|stem| stem.strip_prefix("graph-"))
        .map_or(Ok(()), |hex| match gix_hash::ObjectId::from_hex(hex.as_bytes()) {
            Ok(actual) if actual == expected => Ok(()),
            _ => Err(message!("commit-graph filename should be graph-{}.graph", expected.to_hex()).raise()),
        })
}
