// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/myers/slice.rs

use std::mem::take;
use std::ops::RangeBounds;

use crate::intern::Token;
use crate::myers::preprocess::PreprocessedFile;
use crate::util::common_edges;

/// A slice of a preprocessed file used during the Myers algorithm's divide-and-conquer.
///
/// This structure allows the algorithm to work on subproblems while maintaining
/// references to the original file's token indices and change tracking.
#[derive(Default)]
pub struct FileSlice<'a> {
    /// The tokens in this slice.
    pub tokens: &'a [Token],
    /// Maps from slice positions back to original file positions.
    indices: &'a [u32],
    /// Tracks which tokens in the original file have changed.
    changed: &'a mut [bool],
}

impl<'a> FileSlice<'a> {
    pub fn new(file: &'a PreprocessedFile, changed: &'a mut [bool]) -> Self {
        Self {
            tokens: &file.tokens,
            indices: &file.indices,
            changed,
        }
    }

    pub fn mark_changed(&mut self) {
        for &i in self.indices {
            self.changed[i as usize] = true;
        }
    }

    pub fn borrow(&mut self) -> FileSlice<'_> {
        FileSlice {
            tokens: self.tokens,
            changed: self.changed,
            indices: self.indices,
        }
    }

    pub fn slice<R: RangeBounds<u32>>(self, range: R) -> Self {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&start) => start,
            std::ops::Bound::Excluded(&start) => start + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(&end) => end + 1,
            std::ops::Bound::Excluded(&end) => end,
            std::ops::Bound::Unbounded => self.len(),
        };

        Self {
            tokens: &self.tokens[start as usize..end as usize],
            changed: self.changed,
            indices: &self.indices[start as usize..end as usize],
        }
    }

    pub fn strip_common(&mut self, other: &mut Self) {
        let (start, common_postfix) = common_edges(self.tokens, other.tokens);
        let end = self.len() - common_postfix;
        *self = take(self).slice(start..end);
        let end = other.len() - common_postfix;
        *other = take(other).slice(start..end);
    }

    pub fn len(&self) -> u32 {
        self.tokens.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
