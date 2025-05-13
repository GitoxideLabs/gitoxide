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
/// let range = BlameRanges::from_one_based_inclusive_range(20..=40);
///
/// // Blame multiple ranges
/// let mut ranges = BlameRanges::from_one_based_inclusive_ranges(vec![
///     1..=4, // Lines 1-4
///    10..=14, // Lines 10-14
/// ]
/// );
/// ```
///
/// # Line Number Representation
///
/// This type uses 1-based inclusive ranges to mirror `git`'s behaviour:
/// - A range of `20..=40` represents 21 lines, spanning from line 20 up to and including line 40
/// - This will be converted to `19..40` internally as the algorithm uses 0-based ranges that are exclusive at the end
///
/// # Empty Ranges
/// You can blame the entire file by calling `BlameRanges::default()`, or by passing an empty vector to `from_one_based_inclusive_ranges`.
#[derive(Debug, Clone, Default)]
pub enum BlameRanges {
    /// Blame the entire file.
    #[default]
    WholeFile,
    /// Blame ranges in 0-based exclusive format.
    PartialFile(Vec<Range<u32>>),
}

/// Lifecycle
impl BlameRanges {
    /// Create from a single range.
    ///
    /// Note that the input range is 1-based inclusive, as used by git, and
    /// the output is zero-based `BlameRanges` instance.
    ///
    /// @param range: A 1-based inclusive range.
    /// @return: A `BlameRanges` instance representing the range.
    pub fn from_one_based_inclusive_range(range: RangeInclusive<u32>) -> Self {
        let zero_based_range = Self::inclusive_to_zero_based_exclusive(range);
        Self::PartialFile(vec![zero_based_range])
    }

    /// Create from multiple ranges.
    ///
    /// Note that the input ranges are 1-based inclusive, as used by git, and
    /// the output is zero-based `BlameRanges` instance.
    ///
    /// If the input vector is empty, the result will be `WholeFile`.
    ///
    /// @param ranges: A vec of 1-based inclusive range.
    /// @return: A `BlameRanges` instance representing the range.
    pub fn from_one_based_inclusive_ranges(ranges: Vec<RangeInclusive<u32>>) -> Self {
        if ranges.is_empty() {
            return Self::WholeFile;
        }

        let zero_based_ranges = ranges
            .into_iter()
            .map(Self::inclusive_to_zero_based_exclusive)
            .collect::<Vec<_>>();
        let mut result = Self::PartialFile(vec![]);
        for range in zero_based_ranges {
            let _ = result.merge_range(range);
        }
        result
    }

    /// Convert a 1-based inclusive range to a 0-based exclusive range.
    fn inclusive_to_zero_based_exclusive(range: RangeInclusive<u32>) -> Range<u32> {
        let start = range.start() - 1;
        let end = *range.end();
        start..end
    }
}

impl BlameRanges {
    /// Add a single range to blame.
    ///
    /// The range should be 1-based inclusive.
    /// If the new range overlaps with or is adjacent to an existing range,
    /// they will be merged into a single range.
    pub fn add_range(&mut self, new_range: RangeInclusive<u32>) -> Result<(), Error> {
        match self {
            Self::PartialFile(_) => {
                let zero_based_range = Self::inclusive_to_zero_based_exclusive(new_range);
                self.merge_range(zero_based_range)
            }
            _ => Err(Error::InvalidOneBasedLineRange),
        }
    }

    /// Attempts to merge the new range with any existing ranges.
    /// If no merge is possible, add it as a new range.
    fn merge_range(&mut self, new_range: Range<u32>) -> Result<(), Error> {
        match self {
            Self::PartialFile(ref mut ranges) => {
                // Check if this range can be merged with any existing range
                for range in &mut *ranges {
                    // Check if ranges overlap
                    if new_range.start <= range.end && range.start <= new_range.end {
                        *range = range.start.min(new_range.start)..range.end.max(new_range.end);
                        return Ok(());
                    }
                    // Check if ranges are adjacent
                    if new_range.start == range.end || range.start == new_range.end {
                        *range = range.start.min(new_range.start)..range.end.max(new_range.end);
                        return Ok(());
                    }
                }
                // If no overlap or adjacency found, add it as a new range
                ranges.push(new_range);
                Ok(())
            }
            _ => Err(Error::InvalidOneBasedLineRange),
        }
    }

    /// Convert the ranges to a vector of `Range<u32>`.
    pub fn to_ranges(&self, max_lines: u32) -> Vec<Range<u32>> {
        match self {
            Self::WholeFile => {
                let full_range = 0..max_lines;
                vec![full_range]
            }
            Self::PartialFile(ranges) => ranges.clone(),
        }
    }
}

/// Options to be passed to [`file()`](crate::file()).
#[derive(Default, Debug, Clone)]
pub struct Options {
    /// The algorithm to use for diffing.
    pub diff_algorithm: gix_diff::blob::Algorithm,
    /// The ranges to blame in the file.
    pub ranges: BlameRanges,
    /// Don't consider commits before the given date.
    pub since: Option<gix_date::Time>,
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
}

impl BlameEntry {
    /// Create a new instance.
    pub fn new(range_in_blamed_file: Range<u32>, range_in_source_file: Range<u32>, commit_id: ObjectId) -> Self {
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
}

impl UnblamedHunk {
    /// Create a new instance
    pub fn new(range: Range<u32>, suspect: ObjectId) -> Self {
        let range_start = range.start;
        let range_end = range.end;

        UnblamedHunk {
            range_in_blamed_file: range_start..range_end,
            suspects: [(suspect, range_start..range_end)].into(),
        }
    }

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
#[derive(Debug, PartialEq)]
pub enum Change {
    /// A range of tokens that wasn't changed.
    Unchanged(Range<u32>),
    /// `(added_line_range, num_deleted_in_before)`
    AddedOrReplaced(Range<u32>, u32),
    /// `(line_to_start_deletion_at, num_deleted_in_before)`
    Deleted(u32, u32),
}
