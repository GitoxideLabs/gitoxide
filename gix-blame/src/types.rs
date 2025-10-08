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
    /// Create from a single 0-based range.
    ///
    /// Note that the input range is 1-based inclusive, as used by git, and
    /// the output is a zero-based `BlameRanges` instance.
    pub fn from_one_based_inclusive_range(range: RangeInclusive<u32>) -> Result<Self, Error> {
        let zero_based_range = Self::inclusive_to_zero_based_exclusive(range)?;
        Ok(Self::PartialFile(vec![zero_based_range]))
    }

    /// Create from multiple 0-based ranges.
    ///
    /// Note that the input ranges are 1-based inclusive, as used by git, and
    /// the output is a zero-based `BlameRanges` instance.
    ///
    /// If the input vector is empty, the result will be `WholeFile`.
    pub fn from_one_based_inclusive_ranges(ranges: Vec<RangeInclusive<u32>>) -> Result<Self, Error> {
        if ranges.is_empty() {
            return Ok(Self::WholeFile);
        }

        let zero_based_ranges = ranges
            .into_iter()
            .map(Self::inclusive_to_zero_based_exclusive)
            .collect::<Vec<_>>();
        let mut result = Self::PartialFile(vec![]);
        for range in zero_based_ranges {
            result.merge_zero_based_exclusive_range(range?);
        }
        Ok(result)
    }

    /// Convert a 1-based inclusive range to a 0-based exclusive range.
    fn inclusive_to_zero_based_exclusive(range: RangeInclusive<u32>) -> Result<Range<u32>, Error> {
        if range.start() == &0 {
            return Err(Error::InvalidOneBasedLineRange);
        }
        let start = range.start() - 1;
        let end = *range.end();
        Ok(start..end)
    }
}

impl BlameRanges {
    /// Add a single range to blame.
    ///
    /// The new range will be merged with any overlapping existing ranges.
    pub fn add_one_based_inclusive_range(&mut self, new_range: RangeInclusive<u32>) -> Result<(), Error> {
        let zero_based_range = Self::inclusive_to_zero_based_exclusive(new_range)?;
        self.merge_zero_based_exclusive_range(zero_based_range);

        Ok(())
    }

    /// Adds a new ranges, merging it with any existing overlapping ranges.
    fn merge_zero_based_exclusive_range(&mut self, new_range: Range<u32>) {
        match self {
            Self::PartialFile(ref mut ranges) => {
                // Partition ranges into those that don't overlap and those that do.
                let (mut non_overlapping, overlapping): (Vec<_>, Vec<_>) = ranges
                    .drain(..)
                    .partition(|range| new_range.end < range.start || range.end < new_range.start);

                let merged_range = overlapping.into_iter().fold(new_range, |acc, range| {
                    acc.start.min(range.start)..acc.end.max(range.end)
                });

                non_overlapping.push(merged_range);

                *ranges = non_overlapping;
                ranges.sort_by(|a, b| a.start.cmp(&b.start));
            }
            Self::WholeFile => *self = Self::PartialFile(vec![new_range]),
        }
    }

    /// Gets zero-based exclusive ranges.
    pub fn to_zero_based_exclusive_ranges(&self, max_lines: u32) -> Vec<Range<u32>> {
        match self {
            Self::WholeFile => {
                let full_range = 0..max_lines;
                vec![full_range]
            }
            Self::PartialFile(ranges) => ranges
                .iter()
                .filter_map(|range| {
                    if range.end < max_lines {
                        return Some(range.clone());
                    }

                    if range.start < max_lines {
                        Some(range.start..max_lines)
                    } else {
                        None
                    }
                })
                .collect(),
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
    pub parent_index: usize,
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
    pub(crate) fn new(from_range_in_blamed_file: Range<u32>, suspect: ObjectId) -> Self {
        let range_start = from_range_in_blamed_file.start;
        let range_end = from_range_in_blamed_file.end;

        UnblamedHunk {
            range_in_blamed_file: range_start..range_end,
            suspects: [(suspect, range_start..range_end)].into(),
            source_file_name: None,
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
