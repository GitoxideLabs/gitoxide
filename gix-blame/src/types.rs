use gix_hash::ObjectId;
use gix_object::bstr::BString;
use smallvec::SmallVec;
use std::ops::RangeInclusive;
use std::{
    num::NonZeroU32,
    ops::{AddAssign, Range, SubAssign},
};

use crate::file::function::tokens_for_diffing;
use crate::Error;

/// A type to represent one or more line ranges to blame in a file.
///
/// It handles the conversion between git's 1-based inclusive ranges and the internal
/// 0-based exclusive ranges used by the blame algorithm.
///
/// # Examples
///
/// ```rust
/// use gix_blame::BlameRanges;
///
/// // Blame lines 20 through 40 (inclusive)
/// let range = BlameRanges::from_range(20..=40);
///
/// // Blame multiple ranges
/// let mut ranges = BlameRanges::new();
/// ranges.add_range(1..=4);   // Lines 1-4
/// ranges.add_range(10..=14); // Lines 10-14
/// ```
///
/// # Line Number Representation
///
/// This type uses 1-based inclusive ranges to mirror `git`'s behaviour:
/// - A range of `20..=40` represents 21 lines, spanning from line 20 up to and including line 40
/// - This will be converted to `19..40` internally as the algorithm uses 0-based ranges that are exclusive at the end
///
/// # Empty Ranges
///
/// An empty `BlameRanges` (created via `BlameRanges::new()` or `BlameRanges::default()`) means
/// to blame the entire file, similar to running `git blame` without line number arguments.
#[derive(Debug, Clone, Default)]
pub struct BlameRanges {
    /// The ranges to blame, stored as 1-based inclusive ranges
    /// An empty Vec means blame the entire file
    ranges: Vec<RangeInclusive<u32>>,
}

/// Lifecycle
impl BlameRanges {
    /// Create a new empty BlameRanges instance.
    ///
    /// An empty instance means to blame the entire file.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a single range.
    ///
    /// The range is 1-based, similar to git's line number format.
    pub fn from_range(range: RangeInclusive<u32>) -> Self {
        Self { ranges: vec![range] }
    }

    /// Create from multiple ranges.
    ///
    /// All ranges are 1-based.
    /// Overlapping or adjacent ranges will be merged.
    pub fn from_ranges(ranges: Vec<RangeInclusive<u32>>) -> Self {
        let mut result = Self::new();
        for range in ranges {
            result.merge_range(range);
        }
        result
    }
}

impl BlameRanges {
    /// Add a single range to blame.
    ///
    /// The range should be 1-based inclusive.
    /// If the new range overlaps with or is adjacent to an existing range,
    /// they will be merged into a single range.
    pub fn add_range(&mut self, new_range: RangeInclusive<u32>) {
        self.merge_range(new_range);
    }

    /// Attempts to merge the new range with any existing ranges.
    /// If no merge is possible, add it as a new range.
    fn merge_range(&mut self, new_range: RangeInclusive<u32>) {
        // Check if this range can be merged with any existing range
        for range in &mut self.ranges {
            // Check if ranges overlap or are adjacent
            if new_range.start() <= range.end() && range.start() <= new_range.end() {
                *range = *range.start().min(new_range.start())..=*range.end().max(new_range.end());
                return;
            }
        }
        // If no overlap found, add it as a new range
        self.ranges.push(new_range);
    }

    /// Convert the 1-based inclusive ranges to 0-based exclusive ranges.
    ///
    /// This is used internally by the blame algorithm to convert from git's line number format
    /// to the internal format used for processing.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidLineRange` if:
    /// - Any range starts at 0 (must be 1-based)
    /// - Any range extends beyond the file's length
    /// - Any range has the same start and end
    pub fn to_zero_based_exclusive(&self, max_lines: u32) -> Result<Vec<Range<u32>>, Error> {
        if self.ranges.is_empty() {
            let range = 0..max_lines;
            return Ok(vec![range]);
        }

        let mut result = Vec::with_capacity(self.ranges.len());
        for range in &self.ranges {
            if *range.start() == 0 {
                return Err(Error::InvalidLineRange);
            }
            let start = range.start() - 1;
            let end = *range.end();
            if start >= max_lines || end > max_lines || start == end {
                return Err(Error::InvalidLineRange);
            }
            result.push(start..end);
        }
        Ok(result)
    }

    /// Returns true if no specific ranges are set (meaning blame entire file)
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

/// Options to be passed to [`file()`](crate::file()).
#[derive(Default, Debug, Clone)]
pub struct Options {
    /// The algorithm to use for diffing.
    pub diff_algorithm: gix_diff::blob::Algorithm,
    /// The ranges to blame in the file.
    pub range: BlameRanges,
    /// Don't consider commits before the given date.
    pub since: Option<gix_date::Time>,
    /// Determine if rename tracking should be performed, and how.
    pub rewrites: Option<gix_diff::Rewrites>,
    /// Collect debug information whenever there's a diff or rename that affects the outcome of a
    /// blame.
    pub debug_track_path: bool,
}

/// Represents a change during history traversal for blame. It is supposed to capture enough
/// information to allow reconstruction of the way a blame was performed, i. e. the path the
/// history traversal, combined with repeated diffing of two subsequent states in this history, has
/// taken.
///
/// This is intended for debugging purposes.
#[derive(Clone, Debug)]
pub struct BlamePathEntry {
    /// The path to the *Source File* in the blob after the change.
    pub source_file_path: BString,
    /// The path to the *Source File* in the blob before the change. Allows
    /// detection of renames. `None` for root commits.
    pub previous_source_file_path: Option<BString>,
    /// The commit id associated with the state after the change.
    pub commit_id: ObjectId,
    /// The blob id associated with the state after the change.
    pub blob_id: ObjectId,
    /// The blob id associated with the state before the change.
    pub previous_blob_id: ObjectId,
    /// When there is more than one `BlamePathEntry` for a commit, this indicates to which parent
    /// commit the change is related.
    pub index: usize,
}

/// The outcome of [`file()`](crate::file()).
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    /// One entry in sequential order, to associate a hunk in the blamed file with the source commit (and its lines)
    /// that introduced it.
    pub entries: Vec<BlameEntry>,
    /// A buffer with the file content of the *Blamed File*, ready for tokenization.
    pub blob: Vec<u8>,
    /// Additional information about the amount of work performed to produce the blame.
    pub statistics: Statistics,
    /// Contains a log of all changes that affected the outcome of this blame.
    pub blame_path: Option<Vec<BlamePathEntry>>,
}

/// Additional information about the performed operations.
#[derive(Debug, Default, Copy, Clone)]
pub struct Statistics {
    /// The amount of commits it traversed until the blame was complete.
    pub commits_traversed: usize,
    /// The amount of trees that were decoded to find the entry of the file to blame.
    pub trees_decoded: usize,
    /// The amount of tree-diffs to see if the filepath was added, deleted or modified. These diffs
    /// are likely partial as they are cancelled as soon as a change to the blamed file is
    /// detected.
    pub trees_diffed: usize,
    /// The amount of tree-diffs to see if the file was moved (or rewritten, in git terminology).
    /// These diffs are likely partial as they are cancelled as soon as a change to the blamed file
    /// is detected.
    pub trees_diffed_with_rewrites: usize,
    /// The amount of blobs there were compared to each other to learn what changed between commits.
    /// Note that in order to diff a blob, one needs to load both versions from the database.
    pub blobs_diffed: usize,
}

impl Outcome {
    /// Return an iterator over each entry in [`Self::entries`], along with its lines, line by line.
    ///
    /// Note that [`Self::blob`] must be tokenized in exactly the same way as the tokenizer that was used
    /// to perform the diffs, which is what this method assures.
    pub fn entries_with_lines(&self) -> impl Iterator<Item = (BlameEntry, Vec<BString>)> + '_ {
        use gix_diff::blob::intern::TokenSource;
        let mut interner = gix_diff::blob::intern::Interner::new(self.blob.len() / 100);
        let lines_as_tokens: Vec<_> = tokens_for_diffing(&self.blob)
            .tokenize()
            .map(|token| interner.intern(token))
            .collect();
        self.entries.iter().map(move |e| {
            (
                e.clone(),
                lines_as_tokens[e.range_in_blamed_file()]
                    .iter()
                    .map(|token| BString::new(interner[*token].into()))
                    .collect(),
            )
        })
    }
}

/// Describes the offset of a particular hunk relative to the *Blamed File*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Offset {
    /// The amount of lines to add.
    Added(u32),
    /// The amount of lines to remove.
    Deleted(u32),
}

impl Offset {
    /// Shift the given `range` according to our offset.
    pub fn shifted_range(&self, range: &Range<u32>) -> Range<u32> {
        match self {
            Offset::Added(added) => {
                debug_assert!(range.start >= *added, "{self:?} {range:?}");
                Range {
                    start: range.start - added,
                    end: range.end - added,
                }
            }
            Offset::Deleted(deleted) => Range {
                start: range.start + deleted,
                end: range.end + deleted,
            },
        }
    }
}

impl AddAssign<u32> for Offset {
    fn add_assign(&mut self, rhs: u32) {
        match self {
            Self::Added(added) => *self = Self::Added(*added + rhs),
            Self::Deleted(deleted) => {
                if rhs > *deleted {
                    *self = Self::Added(rhs - *deleted);
                } else {
                    *self = Self::Deleted(*deleted - rhs);
                }
            }
        }
    }
}

impl SubAssign<u32> for Offset {
    fn sub_assign(&mut self, rhs: u32) {
        match self {
            Self::Added(added) => {
                if rhs > *added {
                    *self = Self::Deleted(rhs - *added);
                } else {
                    *self = Self::Added(*added - rhs);
                }
            }
            Self::Deleted(deleted) => *self = Self::Deleted(*deleted + rhs),
        }
    }
}

/// A mapping of a section of the *Blamed File* to the section in a *Source File* that introduced it.
///
/// Both ranges are of the same size, but may use different [starting points](Range::start). Naturally,
/// they have the same content, which is the reason they are in what is returned by [`file()`](crate::file()).
#[derive(Clone, Debug, PartialEq)]
pub struct BlameEntry {
    /// The index of the token in the *Blamed File* (typically lines) where this entry begins.
    pub start_in_blamed_file: u32,
    /// The index of the token in the *Source File* (typically lines) where this entry begins.
    ///
    /// This is possibly offset compared to `start_in_blamed_file`.
    pub start_in_source_file: u32,
    /// The amount of lines the hunk is spanning.
    pub len: NonZeroU32,
    /// The commit that introduced the section into the *Source File*.
    pub commit_id: ObjectId,
    /// The *Source File*'s name, in case it differs from *Blamed File*'s name.
    /// This happens when the file was renamed.
    pub source_file_name: Option<BString>,
}

impl BlameEntry {
    /// Create a new instance.
    pub fn new(
        range_in_blamed_file: Range<u32>,
        range_in_source_file: Range<u32>,
        commit_id: ObjectId,
        source_file_name: Option<BString>,
    ) -> Self {
        debug_assert!(
            range_in_blamed_file.end > range_in_blamed_file.start,
            "{range_in_blamed_file:?}"
        );
        debug_assert!(
            range_in_source_file.end > range_in_source_file.start,
            "{range_in_source_file:?}"
        );
        debug_assert_eq!(range_in_source_file.len(), range_in_blamed_file.len());

        Self {
            start_in_blamed_file: range_in_blamed_file.start,
            start_in_source_file: range_in_source_file.start,
            len: NonZeroU32::new(range_in_blamed_file.len() as u32).expect("BUG: hunks are never empty"),
            commit_id,
            source_file_name,
        }
    }
}

impl BlameEntry {
    /// Return the range of tokens this entry spans in the *Blamed File*.
    pub fn range_in_blamed_file(&self) -> Range<usize> {
        let start = self.start_in_blamed_file as usize;
        start..start + self.len.get() as usize
    }
    /// Return the range of tokens this entry spans in the *Source File*.
    pub fn range_in_source_file(&self) -> Range<usize> {
        let start = self.start_in_source_file as usize;
        start..start + self.len.get() as usize
    }
}

pub(crate) trait LineRange {
    fn shift_by(&self, offset: Offset) -> Self;
}

impl LineRange for Range<u32> {
    fn shift_by(&self, offset: Offset) -> Self {
        offset.shifted_range(self)
    }
}

/// Tracks the hunks in the *Blamed File* that are not yet associated with the commit that introduced them.
#[derive(Debug, PartialEq)]
pub struct UnblamedHunk {
    /// The range in the file that is being blamed that this hunk represents.
    pub range_in_blamed_file: Range<u32>,
    /// Maps a commit to the range in a source file (i.e. *Blamed File* at a revision) that is
    /// equal to `range_in_blamed_file`. Since `suspects` rarely contains more than 1 item, it can
    /// efficiently be stored as a `SmallVec`.
    pub suspects: SmallVec<[(ObjectId, Range<u32>); 1]>,
    /// The *Source File*'s name, in case it differs from *Blamed File*'s name.
    pub source_file_name: Option<BString>,
}

impl UnblamedHunk {
    pub(crate) fn has_suspect(&self, suspect: &ObjectId) -> bool {
        self.suspects.iter().any(|entry| entry.0 == *suspect)
    }

    pub(crate) fn get_range(&self, suspect: &ObjectId) -> Option<&Range<u32>> {
        self.suspects
            .iter()
            .find(|entry| entry.0 == *suspect)
            .map(|entry| &entry.1)
    }
}

#[derive(Debug)]
pub(crate) enum Either<T, U> {
    Left(T),
    Right(U),
}

/// A single change between two blobs, or an unchanged region.
///
/// Line numbers refer to the file that is referred to as `after` or `NewOrDestination`, depending
/// on the context.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// A range of tokens that wasn't changed.
    Unchanged(Range<u32>),
    /// `(added_line_range, num_deleted_in_before)`
    AddedOrReplaced(Range<u32>, u32),
    /// `(line_to_start_deletion_at, num_deleted_in_before)`
    Deleted(u32, u32),
}
