//! Facilities to produce git-formatted diffs.

use std::cmp::Ordering;
use std::ops::Range;

use crate::blob::GitDiff;
use imara_diff::intern::{InternedInput, Interner, Token};
use imara_diff::Sink;

// Explanation for the following numbers can be found here:
// https://github.com/git/git/blob/324fbaab88126196bd42e7fa383ee94e165d61b5/xdiff/xdiffi.c#L535
const MAX_INDENT: u8 = 200;
const MAX_BLANKS: i16 = 20;
const INDENT_WEIGHT: i16 = 60;
const INDENT_HEURISTIC_MAX_SLIDING: usize = 100;

const START_OF_FILE_PENALTY: i16 = 1;
const END_OF_FILE_PENALTY: i16 = 21;
const TOTAL_BLANK_WEIGHT: i16 = -30;
const POST_BLANK_WEIGHT: i16 = 6;
const RELATIVE_INDENT_PENALTY: i16 = -4;
const RELATIVE_INDENT_WITH_BLANK_PENALTY: i16 = 10;
const RELATIVE_OUTDENT_PENALTY: i16 = 24;
const RELATIVE_OUTDENT_WITH_BLANK_PENALTY: i16 = 17;
const RELATIVE_DEDENT_PENALTY: i16 = 23;
const RELATIVE_DEDENT_WITH_BLANK_PENALTY: i16 = 17;

pub(super) mod types {
    use crate::blob::git_diff::ChangeGroup;

    /// A [`Sink`](imara_diff::Sink) that creates a diff like git would.
    pub struct GitDiff<'a, T>
    where
        T: std::fmt::Display,
    {
        pub(crate) after: &'a [imara_diff::intern::Token],
        pub(crate) interner: &'a imara_diff::intern::Interner<T>,
        pub(crate) changes: Vec<ChangeGroup>,
    }
}

/// An enum indicating the kind of change that occurred.
#[derive(PartialEq, Debug)]
pub enum ChangeKind {
    /// Indicates that a change introduced new lines.
    Added,
    /// Indicates that a change removed lines before the starting line of the change.
    RemovedAbove,
    /// Indicates that a change removed lines after the ending line of the change.
    RemovedBelow,
    /// Indicates that the change modified lines.
    Modified,
}

#[derive(PartialEq)]
struct Score {
    effective_indent: i16,
    penalty: i16,
}

impl PartialOrd for Score {
    // A score is considered "Greater" if it is equal or less than 0
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let indent_penalty = match self.effective_indent.cmp(&other.effective_indent) {
            Ordering::Greater => INDENT_WEIGHT,
            Ordering::Less => -INDENT_WEIGHT,
            Ordering::Equal => 0,
        };

        Some((indent_penalty + (self.penalty - other.penalty)).cmp(&0).reverse())
    }
}

/// A [`ChangeGroup`] represents a block of changed lines.
#[derive(PartialEq, Debug)]
pub struct ChangeGroup {
    /// Range indicating the lines of the previous block.
    /// To actually see how the previous block looked like, you need to combine this range with
    /// the [`InternedInput`].
    pub before: Range<usize>,
    /// Range indicating the lines of the new block
    /// To actually see how the current block looks like, you need to combine this range with
    /// the [`InternedInput`].
    pub after: Range<usize>,
    /// Further specify what kind of change is denoted by the ranges above.
    pub change_kind: ChangeKind,
}

// Calculate the indentation of a single line
fn get_indent(s: String) -> Option<u8> {
    let mut indent = 0;

    for char in s.chars() {
        if !char.is_whitespace() {
            return Some(indent);
        } else if char == ' ' {
            indent += 1;
        } else if char == '\t' {
            indent += 8 - indent % 8;
        }

        if indent >= MAX_INDENT {
            return Some(MAX_INDENT);
        }
    }

    None
}

fn measure_and_score_change<T: std::fmt::Display>(
    lines: &[Token],
    split: usize,
    interner: &Interner<T>,
    score: &mut Score,
) {
    // Gather information about the surroundings of the change
    let end_of_file = split >= lines.len();
    let mut indent: Option<u8> = if split >= lines.len() {
        None
    } else {
        get_indent(interner[lines[split]].to_string())
    };
    let mut pre_blank = 0;
    let mut pre_indent: Option<u8> = None;
    let mut post_blank = 0;
    let mut post_indent: Option<u8> = None;

    for line in (0..=split.saturating_sub(1)).rev() {
        pre_indent = get_indent(interner[lines[line]].to_string());
        if pre_indent.is_none() {
            pre_blank += 1;
            if pre_blank == MAX_BLANKS {
                pre_indent = Some(0);
                break;
            }
        }
    }
    for line in split + 1..lines.len() {
        post_indent = get_indent(interner[lines[line]].to_string());
        if post_indent.is_none() {
            post_blank += 1;
            if post_blank == MAX_BLANKS {
                post_indent = Some(0);
                break;
            }
        }
    }

    // Calculate score of the currently applied split
    post_blank = if indent.is_none() { 1 + post_blank } else { 0 };
    let total_blank = pre_blank + post_blank;
    if indent.is_none() {
        indent = post_indent;
    }
    let any_blanks = total_blank != 0;

    if pre_indent.is_none() && pre_blank == 0 {
        score.penalty += START_OF_FILE_PENALTY;
    }

    if end_of_file {
        score.penalty += END_OF_FILE_PENALTY;
    }

    score.penalty += TOTAL_BLANK_WEIGHT * total_blank;
    score.penalty += POST_BLANK_WEIGHT * post_blank;

    score.effective_indent += if let Some(indent) = indent { indent as i16 } else { -1 };

    if indent.is_none() || pre_indent.is_none() || indent == pre_indent {
    } else if indent > pre_indent {
        score.penalty += if any_blanks {
            RELATIVE_INDENT_WITH_BLANK_PENALTY
        } else {
            RELATIVE_INDENT_PENALTY
        };
    } else if post_indent.is_some() && post_indent > indent {
        score.penalty += if any_blanks {
            RELATIVE_OUTDENT_WITH_BLANK_PENALTY
        } else {
            RELATIVE_OUTDENT_PENALTY
        };
    } else {
        score.penalty += if any_blanks {
            RELATIVE_DEDENT_WITH_BLANK_PENALTY
        } else {
            RELATIVE_DEDENT_PENALTY
        };
    }
}

impl<'a, T> GitDiff<'a, T>
where
    T: std::fmt::Display,
{
    /// Create a new instance of [`GitDiff`] that can then be passed to [`imara_diff::diff`]
    /// and generate a more human-readable diff.
    pub fn new(input: &'a InternedInput<T>) -> Self {
        Self {
            after: &input.after,
            interner: &input.interner,
            changes: Vec::new(),
        }
    }
}

impl<T> Sink for GitDiff<'_, T>
where
    T: std::fmt::Display,
{
    type Out = Vec<ChangeGroup>;

    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        let change_kind = match (before.is_empty(), after.is_empty()) {
            (true, false) => ChangeKind::Added,
            (false, true) => {
                if after.start == 0 {
                    ChangeKind::RemovedAbove
                } else {
                    ChangeKind::RemovedBelow
                }
            }
            _ => ChangeKind::Modified,
        };
        self.changes.push(ChangeGroup {
            before: before.start as usize..before.end as usize,
            after: after.start as usize..after.end as usize,
            change_kind,
        });
    }

    fn finish(mut self) -> Self::Out {
        if self.changes.is_empty() {
            return self.changes;
        }

        let mut shift: usize;
        for change in &mut self.changes {
            // Skip one-liner changes
            if change.after.is_empty() {
                continue;
            }

            // Move this change up by one line if the line before the change and the last line in
            // the change are equal
            loop {
                if change.after.start > 0 && self.after[change.after.start - 1] == self.after[change.after.end - 1] {
                    change.after.start -= 1;
                    change.after.end -= 1;
                } else {
                    break;
                }
            }

            shift = change.after.end;

            // Move this change down by one line if the first line in the change the line after the
            // change are equal
            loop {
                if change.after.end < self.after.len() && self.after[change.after.start] == self.after[change.after.end]
                {
                    change.after.start += 1;
                    change.after.end += 1;
                } else {
                    break;
                }
            }

            let mut best_shift: Option<usize> = None;
            let mut best_score = Score {
                effective_indent: 0,
                penalty: 0,
            };

            if change.after.end.saturating_sub(change.after.len()) > shift {
                shift = change.after.end - change.after.len();
            }

            if change.after.end.saturating_sub(INDENT_HEURISTIC_MAX_SLIDING) > shift {
                shift = change.after.end - INDENT_HEURISTIC_MAX_SLIDING;
            }

            while shift <= change.after.end {
                let mut score = Score {
                    effective_indent: 0,
                    penalty: 0,
                };

                measure_and_score_change(self.after, shift, self.interner, &mut score);
                measure_and_score_change(self.after, shift - change.after.len(), self.interner, &mut score);

                if best_shift.is_none() || score > best_score {
                    best_score = score;
                    best_shift = Some(shift);
                }
                shift += 1;
            }

            if let Some(best_shift) = best_shift {
                while change.after.end > best_shift {
                    loop {
                        if change.after.start > 0
                            && self.after[change.after.start - 1] == self.after[change.after.end - 1]
                        {
                            change.after.start -= 1;
                            change.after.end -= 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        self.changes
    }
}
