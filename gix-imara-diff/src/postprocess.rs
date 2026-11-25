// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/postprocess.rs

use crate::intern::{InternedInput, Token};
use crate::slider_heuristic::SliderHeuristic;
use crate::util::{find_hunk_end, find_hunk_start};
use crate::{Diff, Hunk};

impl Diff {
    /// Postprocesses the diff using explicit token sequences and a custom heuristic.
    ///
    /// This is a lower-level method that works directly with token sequences rather than
    /// an `InternedInput`. Use [`postprocess_with_heuristic`](Self::postprocess_with_heuristic)
    /// for a more convenient API.
    ///
    /// # Parameters
    ///
    /// * `before` - The token sequence from the first file, before changes
    /// * `after` - The token sequence from the second file, after changes
    /// * `heuristic` - The slider heuristic to use for positioning hunks
    pub fn postprocess_with(&mut self, before: &[Token], after: &[Token], mut heuristic: impl SliderHeuristic) {
        let mut rediff = RediffScratch::default();
        // Compact removals first so ambiguous additions align with them, as in Git.
        Postprocessor {
            added: &mut self.removed,
            removed: &mut self.added,
            tokens: before,
            other_tokens: after,
            rediff_after_slide: self.rediff_after_slide,
            rediff: &mut rediff,
            hunk: Hunk {
                before: 0..0,
                after: 0..0,
            },
            heuristic: &mut heuristic,
        }
        .run();
        Postprocessor {
            added: &mut self.added,
            removed: &mut self.removed,
            tokens: after,
            other_tokens: before,
            rediff_after_slide: self.rediff_after_slide,
            rediff: &mut rediff,
            hunk: Hunk {
                before: 0..0,
                after: 0..0,
            },
            heuristic: &mut heuristic,
        }
        .run();
    }

    /// Postprocesses the diff using an `InternedInput` and a custom heuristic.
    ///
    /// This is a convenience wrapper around [`postprocess_with`](Self::postprocess_with)
    /// that extracts the token sequences from the input automatically.
    ///
    /// # Parameters
    ///
    /// * `input` - The interned input containing the token sequences
    /// * `heuristic` - The slider heuristic to use for positioning hunks
    pub fn postprocess_with_heuristic<T>(&mut self, input: &InternedInput<T>, heuristic: impl SliderHeuristic) {
        self.postprocess_with(&input.before, &input.after, heuristic);
    }
}

/// Internal state for postprocessing a diff to improve readability.
struct Postprocessor<'a, H> {
    /// The mutable array tracking which tokens were added.
    added: &'a mut [bool],
    /// The mutable array tracking which tokens were removed, changed only to diff a group again.
    removed: &'a mut [bool],
    /// The token sequence being processed.
    tokens: &'a [Token],
    /// The token sequence of the other file, to diff a group again after it slid.
    other_tokens: &'a [Token],
    /// Whether to diff a group again after it slid, which Git only does for histogram diffs.
    rediff_after_slide: bool,
    /// Buffers to diff a group again, shared by both passes.
    rediff: &'a mut RediffScratch,
    /// The current hunk being processed in the iteration.
    hunk: Hunk,
    /// The heuristic used to determine optimal hunk positions.
    heuristic: &'a mut H,
}

impl<H: SliderHeuristic> Postprocessor<'_, H> {
    fn run(mut self) {
        loop {
            // find a hunk
            if !self.hunk.next_hunk(self.removed, self.added) {
                break;
            }

            let range_before_sliding = self.hunk.after.clone();
            let mut earliest_end;
            let mut is_modification;
            loop {
                // move hunk up as far as possible to possibly merge it with other hunks
                // and discover if there are other possible positions
                while self.slide_up() {}
                earliest_end = self.hunk.after.end;
                is_modification = self.hunk.before.start != self.hunk.before.end;

                let hunk_size_unexpanded = self.hunk.after.len();
                // move hunk down as far as possible (and merge with other hunks it if
                // possible) sliding down is often the most preferred position
                while self.slide_down() {
                    is_modification |= self.hunk.before.start != self.hunk.before.end;
                }
                // if this hunk was merged with another hunk while sliding down we might
                // be able to slide up more otherwise we are done
                if hunk_size_unexpanded == self.hunk.after.len() {
                    break;
                }
            }

            if self.hunk.after.end == earliest_end {
                self.rediff(&range_before_sliding);
                continue;
            }
            if is_modification {
                // hunk can be moved and there is a removed hunk in the same region
                // move the hunk so it align with the other hunk to produce a single
                // MODIFIED hunk instead of two separate ADDED/REMOVED hunks
                while self.hunk.before.start == self.hunk.before.end {
                    let success = self.slide_up();
                    debug_assert!(success);
                }
            } else {
                let slider_end = self
                    .heuristic
                    .best_slider_end(self.tokens, self.hunk.after.clone(), earliest_end);
                for _ in slider_end..self.hunk.after.end {
                    let success = self.slide_up();
                    debug_assert!(success);
                }
            }
            self.rediff(&range_before_sliding);
        }
    }

    /// Diff the current group again if it changed while sliding, as sliding can merge groups
    /// so that the merged group has lines in common with the group in the other file.
    /// Like Git, only do this for histogram diffs and when the group in the other file isn't empty
    /// (`xdl_change_compact()` in `xdiff/xdiffi.c`).
    fn rediff(&mut self, range_before_sliding: &std::ops::Range<u32>) {
        let a = self.hunk.after.clone();
        let b = self.hunk.before.clone();
        if !self.rediff_after_slide || b.is_empty() || a == *range_before_sliding {
            return;
        }
        let (a0, a1, b0, b1) = (a.start as usize, a.end as usize, b.start as usize, b.end as usize);
        self.added[a0..a1].fill(false);
        self.removed[b0..b1].fill(false);
        let scratch = &mut *self.rediff;
        scratch.compact(&self.tokens[a0..a1], &self.other_tokens[b0..b1]);
        crate::myers::diff(
            &scratch.tokens,
            &scratch.other_tokens,
            &mut self.added[a0..a1],
            &mut self.removed[b0..b1],
            false,
        );
    }

    /// Slides a hunk down by one token/line, potentially merging it with a subsequent hunk.
    fn slide_down(&mut self) -> bool {
        let Some(&next_token) = self.tokens.get(self.hunk.after.end as usize) else {
            return false;
        };
        if self.tokens[self.hunk.after.start as usize] != next_token {
            return false;
        }
        self.added[self.hunk.after.start as usize] = false;
        self.added[self.hunk.after.end as usize] = true;
        self.hunk.after.start += 1;
        self.hunk.after.end = find_hunk_end(self.added, self.hunk.after.end);
        // move the end of the remove range one down to keep the unchanged lines aligned
        self.hunk.before.start = self.hunk.before.end + 1;
        self.hunk.before.end = find_hunk_end(self.removed, self.hunk.before.start);
        true
    }

    /// Slides a hunk up by one token/line, potentially merging it with a previous hunk.
    fn slide_up(&mut self) -> bool {
        if self.hunk.after.start == 0 {
            return false;
        }
        if self.tokens[self.hunk.after.start as usize - 1] != self.tokens[self.hunk.after.end as usize - 1] {
            return false;
        }
        self.added[self.hunk.after.start as usize - 1] = true;
        self.added[self.hunk.after.end as usize - 1] = false;
        self.hunk.after.end -= 1;
        self.hunk.after.start = find_hunk_start(self.added, self.hunk.after.start - 1);
        // move the start of the remove range one up to keep the unchanged lines aligned
        self.hunk.before.end = self.hunk.before.start - 1;
        self.hunk.before.start = find_hunk_start(self.removed, self.hunk.before.start - 1);
        true
    }
}

/// Buffers to diff a group again with tokens numbered from zero.
///
/// Myers sizes its occurrence counts by the largest token. With the tokens of the whole input, each
/// group would allocate and clear counts for the whole vocabulary. Like Git's `xdl_fall_back_diff()`,
/// which prepares the two groups as separate files, the counts only cover the tokens of the groups.
#[derive(Default)]
struct RediffScratch {
    /// The compact token of each input token in the current groups.
    compact_token: hashbrown::HashMap<Token, Token>,
    /// The group of this file, with compact tokens.
    tokens: Vec<Token>,
    /// The group of the other file, with compact tokens.
    other_tokens: Vec<Token>,
}

impl RediffScratch {
    /// Number the tokens of `tokens` and `other_tokens` from zero, in the order they first occur.
    fn compact(&mut self, tokens: &[Token], other_tokens: &[Token]) {
        self.compact_token.clear();
        self.tokens.clear();
        self.other_tokens.clear();
        for (input, output) in [(tokens, &mut self.tokens), (other_tokens, &mut self.other_tokens)] {
            for &token in input {
                let next = Token(self.compact_token.len() as u32);
                output.push(*self.compact_token.entry(token).or_insert(next));
            }
        }
    }
}
