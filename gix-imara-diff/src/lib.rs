// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/lib.rs

#![deny(missing_docs)]
//! Imara-diff is a solid (imara in Swahili) diff library for Rust.
//! Solid refers to the fact that imara-diff provides very good runtime performance even
//! in pathological cases so that your application never appears to freeze while waiting on a diff.
//! The performance improvements are achieved using battle tested heuristics used in gnu-diff and git
//! that are known to yield fast runtime and performance.
//!
//! Imara-diff is also designed to be flexible so that it can be used with arbitrary collections and
//! not just lists and strings and even allows reusing large parts of the computation when
//! comparing the same file to multiple different files.
//!
//! Imara-diff provides two diff algorithms:
//!
//! * The linear-space variant of the well known [**Myers** algorithm](http://www.xmailserver.org/diff2.pdf)
//! * The **Histogram** algorithm which is a variant of the patience diff algorithm.
//!
//! Myers algorithm has been enhanced with preprocessing and multiple heuristics to ensure fast runtime in pathological
//! cases to avoid quadratic time complexity and closely matches the behavior of gnu-diff and git.
//! The Histogram algorithm was originally ported from git but has been heavily optimized.
//! The **Histogram algorithm outperforms Myers diff** by 10% - 100% across a **wide variety of workloads**.
//!
//! Imara-diffs algorithms have been benchmarked over a wide variety of real-world code.
//! For example, while comparing multiple different Linux kernel versions, it performs up to 30 times better than the `similar` crate.
//!
//! # API Overview
//!
//! ## Preparing the input
//! To compute a diff, an input sequence is required. `imara-diff` computes diffs on abstract
//! sequences represented as a slice of IDs/tokens: [`Token`]. To create
//! such a sequence from your input type (for example, text), the input needs to be interned.
//! For that `imara-diff` provides utilities in the form of the [`InternedInput`] struct and
//! the `TokenSource` trait to construct it. [`InternedInput`] contains the two sides of
//! the diff (used while computing the diff). As well as the interner that allows mapping
//! back tokens to their original data.
//!
//! The most common use case for diff is comparing text. `&str` implements `TokenSource`
//! by default to segment the text into lines. So creating an input for a text-based diff usually
//! looks something like the following:
//!
//! ```
//! # use gix_imara_diff::InternedInput;
//! #
//! let before = "abc\ndef";
//! let after = "abc\ndefg";
//! let input = InternedInput::new(before, after);
//! assert_eq!(input.interner[input.before[0]], "abc\n");
//! ```
//!
//! Note that interning inputs is optional, and you could choose a different strategy
//! for creating a sequence of tokens. Instead of using the [`Diff::compute`] function,
//! [`Diff::compute_with`] can be used to provide a list of tokens directly, entirely
//! bypassing the interning step.
//!
//! ## Computing the Diff
//!
//! A diff of two sequences is represented by the [`Diff`] struct and computed by
//! [`Diff::compute`] / [`Diff::compute_with`]. An algorithm can also be chosen here.
//! In most situations, [`Algorithm::Histogram`] is a good choice; refer to the docs
//! of [`Algorithm`] for more details.
//!
//! After the initial computation, the diff can be *postprocessed*. If the diff is shown
//! to a human in some way (even indirectly), you always want to use this.
//!
//! However, when only counting the number of changed tokens quickly, this can be skipped.
//! The postprocessing allows you to provide your own
//! heuristic for selecting a slider position. An indentation-based heuristic is provided,
//! which is a good fit for all text-based line diffs. The internals of the heuristic are
//! public, so a tweaked heuristic can be built on top.
//!
//! ```
//! # use gix_imara_diff::{InternedInput, Diff, Algorithm};
//! #
//! let before = "abc\ndef";
//! let after = "abc\ndefg";
//! let input = InternedInput::new(before, after);
//! let mut diff = Diff::compute(Algorithm::Histogram, &input);
//! diff.postprocess_lines(&input);
//! assert!(!diff.is_removed(0) && !diff.is_added(0));
//! assert!(diff.is_removed(1) && diff.is_added(1));
//! ```
//!
//! ## Accessing results
//!
//! [`Diff`] allows querying whether a particular position was removed/added on either
//! side of the diff with [`Diff::is_removed`] / [`Diff::is_added`]. The number
//! of additions/removals can be quickly counted with [`Diff::count_removals`] /
//! [`Diff::count_additions`]. The most powerful/useful interface is the hunk iterator
//! [`Diff::hunks`], which returns a list of additions/removals/modifications in the
//! order that they appear in the input.
//!
//! Finally, when built with the `unified_diff` feature, this crate also provides a
//! built-in unified diff/patch formatter similar to `git diff` or `diff -u`.
//! Note that while the formatter has a decent amount of flexibility, it is fairly
//! simplistic and not every formatting may be possible. It's meant to cover common
//! situations but not cover every advanced use case. Instead, if you need more advanced
//! printing, build your own printer on top of the [`Diff::hunks`] iterator; for that, you can
//! take inspiration from the built-in printer implementation.
//!
//! ```
//! # use gix_imara_diff::{InternedInput, Diff, Algorithm, BasicLineDiffPrinter, UnifiedDiffConfig};
//! #
//!
//! let before = r#"fn foo() -> Bar {
//!     let mut foo = 2;
//!     foo *= 50;
//!     println!("hello world")
//! }
//! "#;
//!
//! let after = r#"// lorem ipsum
//! fn foo() -> Bar {
//!     let mut foo = 2;
//!     foo *= 50;
//!     println!("hello world");
//!     println!("{foo}");
//! }
//! // foo
//! "#;
//! let input = InternedInput::new(before, after);
//! let mut diff = Diff::compute(Algorithm::Histogram, &input);
//! diff.postprocess_lines(&input);
//!
//! assert_eq!(
//!     diff.unified_diff(
//!         &BasicLineDiffPrinter(&input.interner),
//!         UnifiedDiffConfig::default(),
//!         &input,
//!     )
//!     .to_string(),
//!     r#"@@ -1,5 +1,8 @@
//! +// lorem ipsum
//!  fn foo() -> Bar {
//!      let mut foo = 2;
//!      foo *= 50;
//! -    println!("hello world")
//! +    println!("hello world");
//! +    println!("{foo}");
//!  }
//! +// foo
//! "#
//! );
//! ```

use std::ops::Range;
use std::slice;

use crate::{
    sources::words,
    util::{strip_common_postfix, strip_common_prefix},
};

pub use crate::slider_heuristic::{IndentHeuristic, IndentLevel, NoSliderHeuristic, SliderHeuristic};
pub use intern::{InternedInput, Interner, Token, TokenSource};
#[cfg(feature = "unified_diff")]
pub use unified_diff::{BasicLineDiffPrinter, EndsWithNewline, UnifiedDiff, UnifiedDiffConfig, UnifiedDiffPrinter};

mod histogram;
mod intern;
mod myers;
mod postprocess;
mod slider_heuristic;
pub mod sources;
#[cfg(test)]
mod tests;
#[cfg(feature = "unified_diff")]
mod unified_diff;
mod util;

/// `imara-diff` supports multiple different algorithms
/// for computing an edit sequence.
/// These algorithms have different performance and all produce different output.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum Algorithm {
    /// A variation of the [`patience` diff algorithm described by Bram Cohen's blog post](https://bramcohen.livejournal.com/73318.html)
    /// that uses a histogram to find the least common LCS.
    /// Just like the `patience` diff algorithm, this algorithm usually produces
    /// more human-readable output than Myers algorithm.
    /// However, compared to the `patience` diff algorithm (which is slower than Myers algorithm),
    /// the Histogram algorithm performs much better.
    ///
    /// The implementation here was originally ported from `git` but has been significantly
    /// modified to improve performance.
    /// As a result, it consistently **performs better than Myers algorithm** (5%-100%) over
    /// a wide variety of test data.
    ///
    /// For pathological subsequences that only contain highly repeating tokens (64+ occurrences)
    /// the algorithm falls back on Myers algorithm (with heuristics) to avoid quadratic behavior.
    ///
    /// Compared to Myers algorithm, the Histogram diff algorithm is more focused on providing
    /// human-readable diffs instead of minimal diffs. In practice, this means that the edit sequences
    /// produced by the histogram diff are often longer than those produced by Myers algorithm.
    ///
    /// The heuristic used by the histogram diff does not work well for inputs with small (often repeated)
    /// tokens. For example, **character diffs do not work well** as most (English) text is made up of
    /// a fairly small set of characters. The `Histogram` algorithm will automatically detect these cases and
    /// fall back to Myers algorithm. However, this detection has a nontrivial overhead, so
    /// if it's known upfront that the sort of tokens is very small, `Myers` algorithm should
    /// be used instead.
    #[default]
    Histogram,
    /// An implementation of the linear space variant of
    /// [Myers  `O((N+M)D)` algorithm](http://www.xmailserver.org/diff2.pdf).
    /// The algorithm is enhanced with preprocessing that removes
    /// tokens that don't occur in the other file at all.
    /// Furthermore, two heuristics for the middle snake search are implemented
    /// that ensure reasonable runtime (mostly linear time complexity) even for large files.
    ///
    /// Due to the divide-and-conquer nature of the algorithm,
    /// the edit sequences produced are still fairly small even when the middle snake
    /// search is aborted by a heuristic.
    /// However, the produced edit sequences are not guaranteed to be fully minimal.
    /// If that property is vital to you, use the `MyersMinimal` algorithm instead.
    ///
    /// The implementation (including the preprocessing) is mostly
    /// ported from `git` and `gnu-diff`, where Myers algorithm is used
    /// as the default diff algorithm.
    /// Therefore, the used heuristics have been heavily battle-tested and
    /// are known to behave well over a large variety of inputs.
    Myers,
    /// Same as `Myers` but the early abort heuristics are disabled to guarantee
    /// a minimal edit sequence.
    /// This can mean significant slowdown in pathological cases.
    MyersMinimal,
}

/// Represents the difference between two sequences of tokens.
///
/// A `Diff` stores which tokens were removed from the first sequence and which tokens were added to the second sequence.
#[derive(Default)]
pub struct Diff {
    /// Tracks which tokens were removed from the first sequence (`before`), with
    /// one entry for each one in the `before` sequence.
    removed: Vec<bool>,
    /// Tracks which tokens were added to the second sequence (`after`), with
    /// one entry for each one in the `after` sequence.
    added: Vec<bool>,
}

impl std::fmt::Debug for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.hunks()).finish()
    }
}

impl Diff {
    /// Computes an edit-script that transforms `input.before` into `input.after` using
    /// the specified `algorithm`
    pub fn compute<T>(algorithm: Algorithm, input: &InternedInput<T>) -> Diff {
        let mut diff = Diff::default();
        diff.compute_with(algorithm, &input.before, &input.after, input.interner.num_tokens());
        diff
    }

    /// Computes an edit-script that transforms `before` into `after` using
    /// the specified `algorithm`.
    pub fn compute_with(&mut self, algorithm: Algorithm, mut before: &[Token], mut after: &[Token], num_tokens: u32) {
        assert!(
            before.len() < i32::MAX as usize,
            "imara-diff only supports up to {} tokens",
            i32::MAX
        );
        assert!(
            after.len() < i32::MAX as usize,
            "imara-diff only supports up to {} tokens",
            i32::MAX
        );
        self.removed.clear();
        self.added.clear();
        self.removed.resize(before.len(), false);
        self.added.resize(after.len(), false);
        match algorithm {
            Algorithm::Histogram => {
                let common_prefix = strip_common_prefix(&mut before, &mut after) as usize;
                let common_postfix = strip_common_postfix(&mut before, &mut after);
                let range = common_prefix..self.removed.len() - common_postfix as usize;
                let removed = &mut self.removed[range];
                let range = common_prefix..self.added.len() - common_postfix as usize;
                let added = &mut self.added[range];
                histogram::diff(before, after, removed, added, num_tokens);
            }
            // Myers trims the common ends itself, as it needs to count the tokens of the whole input first.
            Algorithm::Myers => myers::diff(before, after, &mut self.removed, &mut self.added, false),
            Algorithm::MyersMinimal => myers::diff(before, after, &mut self.removed, &mut self.added, true),
        }
    }

    /// Returns the total number of tokens that were added in the second sequence.
    pub fn count_additions(&self) -> u32 {
        self.added.iter().map(|&added| u32::from(added)).sum()
    }

    /// Returns the total number of tokens that were removed from the first sequence (`before`).
    pub fn count_removals(&self) -> u32 {
        self.removed.iter().map(|&removed| u32::from(removed)).sum()
    }

    /// Returns `true` if the token at the given index was removed from the first sequence (`before`).
    ///
    /// # Panics
    ///
    /// Panics if `token_idx` is out of bounds for the first sequence.
    pub fn is_removed(&self, token_idx: u32) -> bool {
        self.removed[token_idx as usize]
    }

    /// Returns `true` if the token at the given index was added to the second sequence (`after`).
    ///
    /// # Panics
    ///
    /// Panics if `token_idx` is out of bounds for the second sequence (`after`).
    pub fn is_added(&self, token_idx: u32) -> bool {
        self.added[token_idx as usize]
    }

    /// Postprocesses the diff to make it more human-readable. Certain hunks
    /// have an ambiguous placement (even in a minimal diff) where they can move
    /// downward or upward by removing a token (line) at the start and adding
    /// one at the end (or the other way around). The postprocessing adjusts
    /// these hunks according to a couple of rules:
    ///
    /// * Always merge multiple hunks if possible.
    /// * Always try to create a single MODIFY hunk instead of multiple disjoint
    ///   ADDED/REMOVED hunks.
    /// * Move sliders as far down as possible.
    pub fn postprocess_no_heuristic<T>(&mut self, input: &InternedInput<T>) {
        self.postprocess_with_heuristic(input, NoSliderHeuristic)
    }

    /// Postprocesses the diff to make it more human-readable. Certain hunks
    /// have an ambiguous placement (even in a minimal diff) where they can move
    /// downward or upward by removing a token (line) at the start and adding
    /// one at the end (or the other way around). The postprocessing adjusts
    /// these hunks according to a couple of rules:
    ///
    /// * Always merge multiple hunks if possible.
    /// * Always try to create a single MODIFY hunk instead of multiple disjoint
    ///   ADDED/REMOVED hunks.
    /// * Based on a line's indentation level, heuristically compute the most
    ///   intuitive location to split lines.
    /// * Move sliders as far down as possible.
    pub fn postprocess_lines<T: AsRef<[u8]>>(&mut self, input: &InternedInput<T>) {
        self.postprocess_with_heuristic(
            input,
            IndentHeuristic::new(|token| {
                IndentLevel::for_ascii_line(input.interner[token].as_ref().iter().copied(), 8)
            }),
        )
    }

    /// Return an iterator that yields the changed hunks in this diff.
    pub fn hunks(&self) -> HunkIter<'_> {
        HunkIter {
            removed: self.removed.iter(),
            added: self.added.iter(),
            pos_before: 0,
            pos_after: 0,
        }
    }
}

/// A single change in a `Diff` that represents a range of tokens (`before`)
/// in the first sequence that were replaced by a different range of tokens
/// in the second sequence (`after`).
///
/// Each hunk identifies a contiguous region of change, where tokens from the `before` range
/// should be replaced with tokens from the `after` range.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Hunk {
    /// The range of token indices in the first sequence (`before`) that were removed.
    pub before: Range<u32>,
    /// The range of token indices in the second sequence (`after`) that were added.
    pub after: Range<u32>,
}

impl Hunk {
    /// Can be used instead of `Option::None` for better performance.
    /// Because `imara-diff` does not support more than `i32::MAX` there is an unused bit pattern that can be used.
    ///
    /// It has some nice properties where it usually is not necessary to check for `None` separately:
    /// Empty ranges fail contains checks and also fail smaller than checks.
    pub const NONE: Hunk = Hunk {
        before: u32::MAX..u32::MAX,
        after: u32::MAX..u32::MAX,
    };

    /// Inverts a hunk so that it represents a change
    /// that would undo this hunk.
    pub fn invert(&self) -> Hunk {
        Hunk {
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }

    /// Returns whether tokens are only inserted and not removed in this hunk.
    pub fn is_pure_insertion(&self) -> bool {
        self.before.is_empty()
    }

    /// Returns whether tokens are only removed and not inserted in this hunk.
    pub fn is_pure_removal(&self) -> bool {
        self.after.is_empty()
    }

    /// Performs a word-diff on this hunk.
    ///
    /// This requires passing the original [`input`](InternedInput) in order to look up
    /// the tokens of the current hunk, which typically are lines.
    /// Each token is split into words using the built-in [`words`] tokenizer.
    /// The resulting word tokens are stored in a second [`diff_input`](InternedInput),
    /// and a [`diff`](Diff) is computed on them, with basic post-processing applied.
    ///
    /// For performance reasons, this second [`diff_input`](InternedInput) as well as
    /// the computed [`diff`](Diff) need to be passed as parameters so that they can be
    /// re-used when iterating over hunks. Note that word tokens are always
    /// added but never removed from the interner. Consider clearing it if you expect
    /// your input to have a large vocabulary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gix_imara_diff::{InternedInput, Diff, Algorithm};
    /// // Compute diff normally
    /// let before = "before text";
    /// let after = "after text";
    /// let mut lines = InternedInput::new(before, after);
    /// let mut diff = Diff::compute(Algorithm::Histogram, &lines);
    /// diff.postprocess_lines(&lines);
    ///
    /// // Compute word-diff per hunk, reusing allocations across iterations
    /// let mut hunk_diff_input = InternedInput::default();
    /// let mut hunk_diff = Diff::default();
    /// for hunk in diff.hunks() {
    ///   hunk.latin_word_diff(&lines, &mut hunk_diff_input, &mut hunk_diff);
    ///   let added = hunk_diff.count_additions();
    ///   let removed = hunk_diff.count_removals();
    ///   println!("word-diff of this hunk has {added} additions and {removed} removals");
    ///   // optionally, clear the interner:
    ///   hunk_diff_input.clear();
    /// }
    /// ```
    pub fn latin_word_diff<'a>(
        &self,
        input: &InternedInput<&'a str>,
        word_tokens: &mut InternedInput<&'a str>,
        diff: &mut Diff,
    ) {
        let Hunk { before, after } = self.clone();
        word_tokens.update_before(
            before
                .map(|index| input.before[index as usize])
                .map(|token| input.interner[token])
                .flat_map(|line| words(line)),
        );
        word_tokens.update_after(
            after
                .map(|index| input.after[index as usize])
                .map(|token| input.interner[token])
                .flat_map(|line| words(line)),
        );
        diff.removed.clear();
        diff.removed.resize(word_tokens.before.len(), false);
        diff.added.clear();
        diff.added.resize(word_tokens.after.len(), false);
        if self.is_pure_removal() {
            diff.removed.fill(true);
        } else if self.is_pure_insertion() {
            diff.added.fill(true);
        } else {
            diff.compute_with(
                Algorithm::Myers,
                &word_tokens.before,
                &word_tokens.after,
                word_tokens.interner.num_tokens(),
            );
            diff.postprocess_no_heuristic(word_tokens);
        }
    }
}

/// Yields all [`Hunk`]s in a file in monotonically increasing order.
/// Monotonically increasing means here that the following holds for any two
/// consecutive [`Hunk`]s `x` and `y`:
///
/// ``` no_compile
/// assert!(x.before.end < y.before.start);
/// assert!(x.after.end < y.after.start);
/// ```
///
pub struct HunkIter<'diff> {
    removed: slice::Iter<'diff, bool>,
    added: slice::Iter<'diff, bool>,
    pos_before: u32,
    pos_after: u32,
}

impl Iterator for HunkIter<'_> {
    type Item = Hunk;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let removed = (&mut self.removed).take_while(|&&removed| removed).count() as u32;
            let added = (&mut self.added).take_while(|&&added| added).count() as u32;
            if removed != 0 || added != 0 {
                let start_before = self.pos_before;
                let start_after = self.pos_after;
                self.pos_before += removed;
                self.pos_after += added;
                let hunk = Hunk {
                    before: start_before..self.pos_before,
                    after: start_after..self.pos_after,
                };
                self.pos_before += 1;
                self.pos_after += 1;
                return Some(hunk);
            } else if self.removed.len() == 0 && self.added.len() == 0 {
                return None;
            } else {
                self.pos_before += 1;
                self.pos_after += 1;
            }
        }
    }
}
