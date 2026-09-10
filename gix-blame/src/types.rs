use gix_hash::ObjectId;
use gix_object::bstr::BString;
use smallvec::SmallVec;
use std::ops::RangeInclusive;
use std::{
    num::NonZeroU32,
    ops::{AddAssign, Range, SubAssign},
};

use crate::Error;
use crate::file::function::tokens_for_diffing;

/// A type to represent one or more line ranges to blame in a file.
///
/// It handles the conversion between git's 1-based inclusive ranges and the internal
/// 0-based exclusive ranges used by the blame algorithm.
///
/// Values of this type are always valid: they either select the whole file, or a non-empty set of
/// non-empty, sorted and pairwise disjoint ranges. Construction therefore goes through the
/// constructors below, which validate and normalize their input.
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
/// let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![
///     1..=4,  // Lines 1-4
///    10..=14, // Lines 10-14
/// ]);
/// ```
///
/// # Line Number Representation
///
/// This type uses 1-based inclusive ranges to mirror `git`'s behaviour:
/// - A range of `20..=40` represents 21 lines, spanning from line 20 up to and including line 40
/// - This will be converted to `19..40` internally as the algorithm uses 0-based ranges that are exclusive at the end
///
/// Ranges are always non-empty, so `<start>` may never exceed `<end>`. This mirrors the `gix blame -L <start>,<end>`
/// command-line interface, but differs from `git blame -L <start>,<end>` which silently swaps reversed ranges.
/// That swapping is [explicitly documented as undocumented behaviour][swap] in `git`'s own test suite, so we
/// prefer to reject what we cannot unambiguously interpret.
///
/// # Blaming the Whole File
///
/// You can blame the entire file by calling [`BlameRanges::default()`], or by passing an empty vector to
/// [`BlameRanges::from_one_based_inclusive_ranges()`]. Note that this is about an empty collection of ranges;
/// an individual range may never be empty.
///
/// If you already hold 0-based exclusive ranges, convert each `start..end` to the 1-based inclusive
/// `(start + 1)..=end` before passing it in. That conversion is exact for every non-empty range.
///
/// [swap]: https://github.com/git/git/blob/3cb9185f65410273787f74333cc027d2ea5daada/t/annotate-tests.sh#L271-L273
#[derive(Debug, Clone, Default)]
pub struct BlameRanges(Selection);

#[derive(Debug, Clone, Default)]
enum Selection {
    /// Blame the entire file.
    #[default]
    WholeFile,
    /// Blame the given ranges, in 0-based exclusive format.
    ///
    /// Upheld invariants, all established by [`BlameRanges::merge_zero_based_exclusive_range()`]:
    ///
    /// * the `Vec` is never empty - that state is spelled [`Selection::WholeFile`],
    /// * every range is non-empty, so it can become a [`BlameEntry`] with a [`NonZeroU32`] length,
    /// * the ranges are sorted by `start` and pairwise disjoint and non-adjacent, so no line is
    ///   ever attributed twice.
    PartialFile(Vec<Range<u32>>),
}

/// Lifecycle
impl BlameRanges {
    /// Create from a single 1-based inclusive range.
    ///
    /// Note that the input range is 1-based inclusive, as used by git, and
    /// the output is a 0-based exclusive `BlameRanges` instance.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOneBasedLineRange`] if `range` starts at `0`, or if it is reversed,
    /// i.e. if its start exceeds its end.
    pub fn from_one_based_inclusive_range(range: RangeInclusive<u32>) -> Result<Self, Error> {
        let mut result = Self::default();
        result.merge_zero_based_exclusive_range(Self::inclusive_to_zero_based_exclusive(range)?);
        Ok(result)
    }

    /// Create from multiple 1-based inclusive ranges.
    ///
    /// Note that the input ranges are 1-based inclusive, as used by git, and
    /// the output is a 0-based exclusive `BlameRanges` instance.
    ///
    /// If the input vector is empty, the result selects the whole file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOneBasedLineRange`] if any range starts at `0`, or if any range is
    /// reversed, i.e. if its start exceeds its end.
    pub fn from_one_based_inclusive_ranges(ranges: Vec<RangeInclusive<u32>>) -> Result<Self, Error> {
        let mut result = Self::default();
        for range in ranges {
            result.merge_zero_based_exclusive_range(Self::inclusive_to_zero_based_exclusive(range)?);
        }
        Ok(result)
    }

    /// Convert a 1-based inclusive range to a 0-based exclusive range.
    ///
    /// Reversed ranges are rejected rather than turned into empty 0-based ranges, as the blame
    /// algorithm cannot represent a hunk without lines.
    fn inclusive_to_zero_based_exclusive(range: RangeInclusive<u32>) -> Result<Range<u32>, Error> {
        let (start, end) = (*range.start(), *range.end());
        // Not `RangeInclusive::is_empty()`, which is also `true` for a range iterated to exhaustion.
        if start == 0 || start > end {
            return Err(Error::InvalidOneBasedLineRange);
        }
        Ok(start - 1..end)
    }
}

/// Access
impl BlameRanges {
    /// Return `true` if the entire file is selected, which is the default.
    pub fn is_whole_file(&self) -> bool {
        matches!(self.0, Selection::WholeFile)
    }

    /// Return the selected 0-based exclusive ranges, or `None` if the whole file is selected.
    ///
    /// The ranges are non-empty, sorted and pairwise disjoint.
    pub fn selected_ranges(&self) -> Option<&[Range<u32>]> {
        match &self.0 {
            Selection::WholeFile => None,
            Selection::PartialFile(ranges) => Some(ranges),
        }
    }
}

impl BlameRanges {
    /// Add a single range to blame.
    ///
    /// The new range will be merged with any overlapping existing ranges.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOneBasedLineRange`] if `new_range` starts at `0`, or if it is
    /// reversed, i.e. if its start exceeds its end. The existing selection is left untouched in
    /// that case.
    pub fn add_one_based_inclusive_range(&mut self, new_range: RangeInclusive<u32>) -> Result<(), Error> {
        let zero_based_range = Self::inclusive_to_zero_based_exclusive(new_range)?;
        self.merge_zero_based_exclusive_range(zero_based_range);

        Ok(())
    }

    /// Add `new_range`, merging it with any existing overlapping or adjacent ranges.
    ///
    /// This is the only place that creates [`Selection::PartialFile`], and is what upholds its
    /// invariants. `new_range` must be non-empty.
    fn merge_zero_based_exclusive_range(&mut self, new_range: Range<u32>) {
        debug_assert!(!new_range.is_empty(), "BUG: an empty range must never be selected");
        match &mut self.0 {
            Selection::PartialFile(ranges) => {
                // Partition ranges into those that don't overlap and those that do.
                let (mut non_overlapping, overlapping): (Vec<_>, Vec<_>) = ranges
                    .drain(..)
                    .partition(|range| new_range.end < range.start || range.end < new_range.start);

                let merged_range = overlapping.into_iter().fold(new_range, |acc, range| {
                    acc.start.min(range.start)..acc.end.max(range.end)
                });

                non_overlapping.push(merged_range);

                *ranges = non_overlapping;
                ranges.sort_by_key(|a| a.start);
            }
            Selection::WholeFile => self.0 = Selection::PartialFile(vec![new_range]),
        }
    }

    /// Resolves the selection into 0-based exclusive ranges while taking into account `max_lines` -
    /// the number of lines in the content that will be blamed.
    ///
    /// Ranges that reach past `max_lines` are clamped to it, and ranges that start past `max_lines`
    /// are dropped. Consequently the result can be empty, if every selected range starts past
    /// `max_lines`, but every range it does contain is guaranteed to be non-empty.
    /// [`file()`](crate::file()) relies on that guarantee, as a hunk without lines cannot be turned
    /// into a [`BlameEntry`].
    ///
    /// Note that a file without lines cannot be expressed here, as `max_lines` is a [`NonZeroU32`].
    /// Such a file has nothing to attribute, and [`file()`](crate::file()) recognizes it before
    /// any range is resolved.
    pub(crate) fn to_zero_based_exclusive_ranges(&self, max_lines: NonZeroU32) -> Vec<Range<u32>> {
        let max_lines = max_lines.get();
        let ranges = match &self.0 {
            Selection::WholeFile => {
                // Kept as a binding to avoid `clippy::single_range_in_vec_init`.
                let full_range = 0..max_lines;
                vec![full_range]
            }
            Selection::PartialFile(ranges) => ranges
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
        };
        debug_assert!(
            ranges.iter().all(|range| !range.is_empty()),
            "BUG: resolved ranges must never be empty, or creating a `BlameEntry` from them will panic"
        );
        ranges
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

/// The starting point for [`file()`](crate::file()).
pub enum Start<'a> {
    /// Start from a specific commit.
    Commit(ObjectId),
    /// Start from `contents`, then continue from `first_suspect`.
    ///
    /// Lines that only exist in `contents` are attributed to the null id,
    /// i.e. "not committed yet".
    ///
    /// It is assumed that the data in `contents` is ready to be used for diffing, in particular
    /// that it has been run through the configured worktree filters.
    ///
    /// See [Pipeline::convert_to_diffable()](gix_diff::blob::Pipeline::convert_to_diffable) for
    /// how to obtain the contents of a worktree file by running them through the configured
    /// worktree filters.
    Contents {
        /// The commit to start from after it has been compared to `contents`.
        first_suspect: ObjectId,
        /// The contents to start the blame from, typically read from the worktree.
        // TODO(blame): add a type so rename tracking can avoid comparing blobs with symlinks.
        //              Blob is hard-coded in at least once place.
        contents: std::borrow::Cow<'a, [u8]>,
    },
}

impl std::fmt::Debug for Start<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Start::Commit(id) => f.debug_tuple("Commit").field(id).finish(),
            Start::Contents {
                first_suspect,
                contents,
            } => f
                .debug_struct("Contents")
                .field("first_suspect", first_suspect)
                .field("contents_len", &contents.len())
                .finish(),
        }
    }
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
        use gix_diff::blob::TokenSource;
        let mut interner = gix_diff::blob::Interner::new(self.blob.len() / 100);
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
