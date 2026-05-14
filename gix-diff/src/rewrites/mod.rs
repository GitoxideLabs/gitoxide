use std::collections::BTreeSet;

use crate::{Rewrites, tree::visit::ChangeId};

/// Types related to the rename tracker for renames, rewrites and copies.
pub mod tracker;

/// A type to retain state related to an ongoing tracking operation to retain sets of interesting changes
/// of which some are retained to at a later stage compute the ones that seem to be renames or copies.
pub struct Tracker<T> {
    /// The tracked items thus far, which will be used to determine renames/copies and rewrites later.
    items: Vec<tracker::Item<T>>,
    /// A place to store all paths in to reduce amount of allocations.
    path_backing: Vec<u8>,
    /// How to track copies and/or rewrites.
    rewrites: Rewrites,
    /// Previously emitted relation ids of rewrite pairs, with `(deleted source, added destination)`.
    child_renames: BTreeSet<(ChangeId, ChangeId)>,
}

/// Determine in which set of files to search for copies.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum CopySource {
    /// Find copies from the set of modified files only.
    #[default]
    FromSetOfModifiedFiles,
    /// Find copies from the set of modified files, as well as all files known to the source (i.e. previous state of the tree).
    ///
    /// This can be an expensive operation as it scales exponentially with the total amount of files in the set.
    FromSetOfModifiedFilesAndAllSources,
}

/// Under which circumstances we consider a file to be a copy.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Copies {
    /// The set of files to search when finding the source of copies.
    pub source: CopySource,
    /// Equivalent to [`Rewrites::percentage`], but used for copy tracking.
    ///
    /// Useful to have similarity-based rename tracking and cheaper copy tracking.
    pub percentage: Option<f32>,
}

impl Default for Copies {
    fn default() -> Self {
        Copies {
            source: CopySource::default(),
            percentage: Some(0.5),
        }
    }
}

/// Information collected while handling rewrites of files which may be tracked.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    /// The options used to guide the rewrite tracking. Either fully provided by the caller or retrieved from git configuration.
    pub options: Rewrites,
    /// The amount of similarity checks that have been conducted to find renamed files and potentially copies.
    pub num_similarity_checks: usize,
    /// Set to the amount of worst-case rename permutations we didn't search as our limit didn't allow it.
    pub num_similarity_checks_skipped_for_rename_tracking_due_to_limit: usize,
    /// Set to the amount of worst-case copy permutations we didn't search as our limit didn't allow it.
    pub num_similarity_checks_skipped_for_copy_tracking_due_to_limit: usize,
}

/// The default settings for rewrites according to the git configuration defaults.
impl Default for Rewrites {
    fn default() -> Self {
        Rewrites {
            copies: None,
            percentage: Some(0.5),
            limit: 1000,
            track_empty: false,
        }
    }
}
