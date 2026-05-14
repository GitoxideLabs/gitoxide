// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/slider_heuristic.rs

use std::cmp::Ordering;
use std::hash::Hash;
use std::ops::{Add, Range};

use crate::intern::Token;

/// A trait for heuristics that determine the best position for ambiguous diff hunks.
///
/// During postprocessing, some hunks can be moved up or down without changing the
/// minimal nature of the diff. This trait allows customizing the logic for choosing
/// the optimal position for such hunks.
pub trait SliderHeuristic {
    /// Determines the best ending position for a hunk that can be slid.
    ///
    /// # Parameters
    ///
    /// * `tokens` - The token sequence being diffed
    /// * `hunk` - The range representing the current hunk position
    /// * `earliest_end` - The earliest valid ending position for the hunk
    ///
    /// # Returns
    ///
    /// The preferred ending position for the hunk
    fn best_slider_end(&mut self, tokens: &[Token], hunk: Range<u32>, earliest_end: u32) -> u32;
}

impl<F> SliderHeuristic for F
where
    F: FnMut(&[Token], Range<u32>, u32) -> u32,
{
    fn best_slider_end(&mut self, tokens: &[Token], hunk: Range<u32>, earliest_end: u32) -> u32 {
        self(tokens, hunk, earliest_end)
    }
}

/// A slider heuristic that doesn't adjust hunk positions.
///
/// This heuristic always places hunks at their lowest possible position without
/// applying any additional logic.
pub struct NoSliderHeuristic;

impl SliderHeuristic for NoSliderHeuristic {
    fn best_slider_end(&mut self, _tokens: &[Token], hunk: Range<u32>, _earliest_end: u32) -> u32 {
        hunk.end
    }
}

/// A slider heuristic that uses indentation levels to determine the best hunk position.
///
/// This heuristic analyzes the indentation of lines surrounding potential hunk positions
/// and chooses the position that results in the most intuitive diff for human readers.
/// It's particularly effective for code and other indented text.
pub struct IndentHeuristic<IndentOfToken> {
    /// A function that computes the indentation level for a given token.
    indent_of_token: IndentOfToken,
}

impl<IndentOfToken> IndentHeuristic<IndentOfToken> {
    /// Creates a new `IndentHeuristic` with the given indentation function.
    ///
    /// # Parameters
    ///
    /// * `indent_of_token` - A function that takes a token and returns its indentation level
    pub fn new(indent_of_token: IndentOfToken) -> Self {
        Self { indent_of_token }
    }
}

impl<IndentOfToken: Fn(Token) -> IndentLevel> SliderHeuristic for IndentHeuristic<IndentOfToken> {
    fn best_slider_end(&mut self, tokens: &[Token], hunk: Range<u32>, earliest_end: u32) -> u32 {
        const MAX_SLIDING: u32 = 100;
        // This is a pure insertion that can be moved freely up and down.
        // To get more intuitive results, apply a heuristic.
        let mut top_slider_end = earliest_end;
        // TODO: why is this needed
        if top_slider_end < hunk.start - 1 {
            top_slider_end = hunk.start - 1;
        }
        if hunk.end > top_slider_end + MAX_SLIDING {
            top_slider_end = hunk.end - MAX_SLIDING;
        }
        let group_size = hunk.end - hunk.start;
        let mut best_score = Score::for_range(
            top_slider_end - group_size..top_slider_end,
            tokens,
            &self.indent_of_token,
        );
        let mut best_slider_end = top_slider_end;
        for slider_end in (top_slider_end + 1)..=hunk.end {
            let score = Score::for_range(slider_end - group_size..slider_end, tokens, &self.indent_of_token);
            if score.is_improvement_over(best_score) {
                best_score = score;
                best_slider_end = slider_end;
            }
        }
        best_slider_end
    }
}

/// Represents the indentation level of a line.
///
/// Indentation is measured in spaces, with tabs expanded according to a configurable tab width.
/// Special values are used to represent blank lines and maximum indentation.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd)]
pub struct IndentLevel(u8);

impl IndentLevel {
    /// Represents a line that is empty or contains only whitespace (or EOF).
    const BLANK: IndentLevel = IndentLevel(u8::MAX);
    /// The maximum trackable indentation level.
    const MAX: IndentLevel = IndentLevel(200);

    /// Computes the indentation level for an ASCII line.
    ///
    /// # Parameters
    ///
    /// * `src` - An iterator over the bytes of the line
    /// * `tab_width` - The number of spaces that a tab character represents (min is 1)
    ///
    /// # Returns
    ///
    /// The computed indentation level, or `BLANK` if the line contains only whitespace
    pub fn for_ascii_line(src: impl IntoIterator<Item = u8>, tab_width: u8) -> IndentLevel {
        let mut indent_level = IndentLevel(0);
        let tab_width = tab_width.max(1);
        for c in src {
            match c {
                b' ' => indent_level = indent_level.increased_by(1),
                b'\t' => indent_level = indent_level.increased_by(tab_width - indent_level.0 % tab_width),
                b'\r' | b'\n' | b'\x0C' => (),
                _ => return indent_level,
            }
            if indent_level >= Self::MAX {
                return Self::MAX;
            }
        }
        IndentLevel::BLANK
    }

    /// Computes the indentation level for a Unicode line.
    ///
    /// # Parameters
    ///
    /// * `src` - An iterator over the characters of the line
    /// * `tab_width` - The number of spaces that a tab character represents
    ///
    /// # Returns
    ///
    /// The computed indentation level, or `BLANK` if the line contains only whitespace
    pub fn for_line(src: impl IntoIterator<Item = char>, tab_width: u8) -> IndentLevel {
        let mut indent_level = IndentLevel(0);
        let tab_width = tab_width.max(1);
        for c in src {
            match c {
                ' ' => indent_level = indent_level.increased_by(1),
                '\t' => indent_level = indent_level.increased_by(tab_width - indent_level.0 % tab_width),
                '\r' | '\n' | '\x0C' => (),
                _ => return indent_level,
            }
            if indent_level >= Self::MAX {
                return Self::MAX;
            }
        }
        IndentLevel::BLANK
    }

    fn increased_by(self, amount: u8) -> Self {
        IndentLevel(self.0.saturating_add(amount).min(Self::MAX.0))
    }

    fn map_or<T>(self, default: T, f: impl FnOnce(u8) -> T) -> T {
        if self == Self::BLANK { default } else { f(self.0) }
    }

    fn or(self, default: Self) -> Self {
        if self == Self::BLANK { default } else { self }
    }
}

/// Captures indentation information for a token and its surrounding context.
///
/// This structure is used by the indent heuristic to evaluate different hunk positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indents {
    /// Indentation level of the current line/token.
    indent: IndentLevel,
    /// Indentation level of the previous non-blank line.
    prev_indent: IndentLevel,
    /// Indentation level of the next non-blank line.
    next_indent: IndentLevel,
    /// The number of consecutive blank lines above the current position.
    leading_blanks: u8,
    /// The number of blank lines after the line following the current position.
    trailing_blanks: u8,
}

/// Maximum number of consecutive blank lines to consider when computing indentation context.
const MAX_BLANKS: usize = 20;

impl Indents {
    fn at_token(tokens: &[Token], token_idx: usize, indent_of_token: impl Fn(Token) -> IndentLevel) -> Indents {
        let (leading_blank_lines, indent_previous_line) = tokens[..token_idx]
            .iter()
            .rev()
            .enumerate()
            .find_map(|(i, &token)| {
                if i == MAX_BLANKS {
                    Some((i, IndentLevel(0)))
                } else {
                    let level = indent_of_token(token);
                    if level == IndentLevel::BLANK {
                        None
                    } else {
                        Some((i, level))
                    }
                }
            })
            .unwrap_or((token_idx, IndentLevel::BLANK));
        let at_eof = token_idx == tokens.len();
        let (trailing_blank_lines, indent_next_line) = if at_eof {
            (0, IndentLevel::BLANK)
        } else {
            tokens[token_idx + 1..]
                .iter()
                .enumerate()
                .find_map(|(i, &token)| {
                    if i == MAX_BLANKS {
                        Some((i, IndentLevel(0)))
                    } else {
                        let level = indent_of_token(token);
                        if level == IndentLevel::BLANK {
                            None
                        } else {
                            Some((i, level))
                        }
                    }
                })
                .unwrap_or((token_idx, IndentLevel::BLANK))
        };
        let indent = tokens
            .get(token_idx)
            .map_or(IndentLevel::BLANK, |&token| indent_of_token(token));
        Indents {
            indent,
            prev_indent: indent_previous_line,
            next_indent: indent_next_line,
            leading_blanks: leading_blank_lines as u8,
            trailing_blanks: trailing_blank_lines as u8,
        }
    }

    fn score(&self) -> Score {
        let mut penalty = 0;
        if self.prev_indent == IndentLevel::BLANK && self.leading_blanks == 0 {
            penalty += START_OF_FILE_PENALTY;
        }
        if self.next_indent == IndentLevel::BLANK && self.trailing_blanks == 0 {
            penalty += END_OF_FILE_PENALTY;
        }

        let trailing_blank_lines = if self.indent == IndentLevel::BLANK {
            self.trailing_blanks as i32 + 1
        } else {
            0
        };
        let total_blank_lines = trailing_blank_lines + self.leading_blanks as i32;
        penalty += TOTAL_BLANK_LINE_WEIGHT * total_blank_lines + trailing_blank_lines * TRAILING_BLANK_LINES_WEIGHT;
        let indent = self.indent.or(self.next_indent);
        if indent != IndentLevel::BLANK && self.prev_indent != IndentLevel::BLANK {
            match indent.0.cmp(&self.prev_indent.0) {
                Ordering::Equal => {}
                // self.next_indent != IndentLevel::BLANK follows for free here
                // since indent != BLANK and therefore self.next_indent <= indent < BLANK
                Ordering::Less if self.next_indent.0 <= indent.0 => {
                    penalty += if total_blank_lines != 0 {
                        RELATIVE_DEDENT_WITH_BLANK_PENALTY
                    } else {
                        RELATIVE_DEDENT_PENALTY
                    }
                }
                Ordering::Less => {
                    penalty += if total_blank_lines != 0 {
                        RELATIVE_OUTDENT_WITH_BLANK_PENALTY
                    } else {
                        RELATIVE_OUTDENT_PENALTY
                    }
                }
                Ordering::Greater => {
                    penalty += if total_blank_lines != 0 {
                        RELATIVE_INDENT_WITH_BLANK_PENALTY
                    } else {
                        RELATIVE_INDENT_PENALTY
                    }
                }
            }
        }
        Score {
            indent: indent.map_or(-1, i32::from),
            penalty,
        }
    }
}

/// Penalty for placing a hunk at the start of a file.
const START_OF_FILE_PENALTY: i32 = 1;
/// Penalty for placing a hunk at the end of a file.
const END_OF_FILE_PENALTY: i32 = 21;
/// Weight applied to the total number of blank lines surrounding a hunk (negative means preferred).
const TOTAL_BLANK_LINE_WEIGHT: i32 = -30;
/// Additional weight for trailing blank lines.
const TRAILING_BLANK_LINES_WEIGHT: i32 = 6;

/// Penalty for placing a hunk where indentation increases (negative means preferred).
const RELATIVE_INDENT_PENALTY: i32 = -4;
/// Penalty for placing a hunk where indentation increases with blank lines present.
const RELATIVE_INDENT_WITH_BLANK_PENALTY: i32 = 10;

/// Penalty for placing a hunk where indentation decreases (outdent).
const RELATIVE_OUTDENT_PENALTY: i32 = 24;
/// Penalty for placing a hunk where indentation decreases with blank lines present.
const RELATIVE_OUTDENT_WITH_BLANK_PENALTY: i32 = 17;

/// Penalty for placing a hunk where indentation decreases but stays aligned (dedent).
const RELATIVE_DEDENT_PENALTY: i32 = 23;
/// Penalty for placing a hunk where indentation decreases but stays aligned with blank lines present.
const RELATIVE_DEDENT_WITH_BLANK_PENALTY: i32 = 17;

/// Weight factor for comparing indentation levels when scoring positions.
const INDENT_WEIGHT: i32 = 60;

/// A score for evaluating the quality of a hunk position.
///
/// Lower scores are better. The score considers both indentation level
/// and various penalties based on the surrounding context.
#[derive(PartialEq, Eq, Clone, Copy)]
struct Score {
    /// The combined indentation level at the hunk boundaries.
    indent: i32,
    /// The total penalty from various heuristics.
    penalty: i32,
}

impl Score {
    fn for_range(range: Range<u32>, tokens: &[Token], indent_of_token: impl Fn(Token) -> IndentLevel) -> Score {
        Indents::at_token(tokens, range.start as usize, &indent_of_token).score()
            + Indents::at_token(tokens, range.end as usize, &indent_of_token).score()
    }
}

impl Add for Score {
    type Output = Score;

    fn add(self, rhs: Self) -> Self::Output {
        Score {
            indent: self.indent + rhs.indent,
            penalty: self.penalty + rhs.penalty,
        }
    }
}

impl Score {
    fn is_improvement_over(self, prev_score: Self) -> bool {
        // smaller indentation level is preferred (with a weight)
        let indent_score = match prev_score.indent.cmp(&self.indent) {
            Ordering::Less => INDENT_WEIGHT,
            Ordering::Greater => -INDENT_WEIGHT,
            Ordering::Equal => 0,
        };
        (indent_score + self.penalty - prev_score.penalty) <= 0
    }
}

#[cfg(test)]
mod tests {
    use super::IndentLevel;

    #[test]
    fn ascii_indent_clamps_before_overflow() {
        assert_eq!(
            IndentLevel::for_ascii_line(std::iter::repeat_n(b' ', 255), 1),
            IndentLevel::MAX
        );
        assert_eq!(
            IndentLevel::for_ascii_line(std::iter::repeat_n(b'\t', 8), u8::MAX),
            IndentLevel::MAX
        );
    }

    #[test]
    fn unicode_indent_treats_zero_tab_width_as_one() {
        assert_eq!(IndentLevel::for_line(['\t', 'x'], 0), IndentLevel(1));
    }
}
