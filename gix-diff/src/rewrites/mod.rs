use crate::Rewrites;

/// Types related to the rename tracker for renames, rewrites and copies.
pub mod tracker;

pub(crate) mod similarity;

/// Git's byte-span similarity score, independent of line ordering and line-diff algorithms.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Similarity(u16);

impl Similarity {
    /// The largest score, representing 100% similarity.
    pub const MAX_SCORE: u16 = 60_000;

    /// Return a similarity for a Git score in `0..=60000`, or `None` if it is out of range.
    pub const fn from_score(score: u16) -> Option<Self> {
        if score <= Self::MAX_SCORE {
            Some(Self(score))
        } else {
            None
        }
    }

    /// Return the exact Git score, in `0..=60000`.
    pub const fn score(self) -> u16 {
        self.0
    }

    /// Return the whole percentage, rounded down like Git's diff output.
    pub const fn percentage(self) -> u8 {
        (self.0 / (Self::MAX_SCORE / 100)) as u8
    }

    /// Return similarity as a fraction in `0.0..=1.0`, suitable for comparison with [`Rewrites::percentage`].
    pub fn fraction(self) -> f32 {
        f32::from(self.0) / f32::from(Self::MAX_SCORE)
    }
}

/// A type to retain state related to an ongoing tracking operation to retain sets of interesting changes
/// of which some are retained to at a later stage compute the ones that seem to be renames or copies.
pub struct Tracker<T> {
    /// The tracked items thus far, which will be used to determine renames/copies and rewrites later.
    items: Vec<tracker::Item<T>>,
    /// A place to store all paths in to reduce amount of allocations.
    path_backing: Vec<u8>,
    /// How to track copies and/or rewrites.
    rewrites: Rewrites,
}

/// Determine in which set of files to search for copies.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum CopySource {
    /// Find copies from modified files and deleted files, including sources already used by renames.
    #[default]
    FromSetOfModifiedFiles,
    /// Find copies from the set of modified files, as well as all files known to the source (i.e. previous state of the tree).
    ///
    /// This can be expensive: exhaustive matching scales with the product of sources and destinations.
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
