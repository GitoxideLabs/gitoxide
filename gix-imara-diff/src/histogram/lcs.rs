// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/histogram/lcs.rs

use crate::histogram::{Histogram, MAX_CHAIN_LEN, MAX_SEED_OCCURRENCES, OVERFLOW_COUNT, Tail};
use crate::intern::Token;

/// Finds the longest common subsequence (LCS) using a histogram-based approach.
///
/// Returns `None` if the sequences are highly repetitive and should fall back to Myers.
pub(super) fn find_lcs(
    before: &[Token],
    after: &[Token],
    tail: Option<Tail<'_>>,
    histogram: &mut Histogram,
) -> Option<Lcs> {
    let mut search = LcsSearch {
        lcs: Lcs::default(),
        min_occurrences: MAX_SEED_OCCURRENCES,
        found_cs: tail.is_some(),
    };
    let occurrences = Occurrences {
        prefix: histogram,
        tail,
        before_len: before.len() as u32,
    };
    search.run(before, after, &occurrences);
    histogram.clear();
    if search.success() { Some(search.lcs) } else { None }
}

/// Occurrence data for the active before range, in prefix and suffix order.
struct Occurrences<'a> {
    prefix: &'a Histogram,
    tail: Option<Tail<'a>>,
    before_len: u32,
}

impl Occurrences<'_> {
    #[inline]
    fn count(&self, token: Token) -> u32 {
        let prefix = self.prefix.num_token_occurrences(token);
        let tail = self.tail.map_or(0, |tail| tail.positions(token).len() as u32);
        (prefix + tail).min(OVERFLOW_COUNT)
    }

    fn positions(&self, token: Token) -> impl Iterator<Item = u32> + '_ {
        let tail = self.tail.map_or(&[][..], |tail| tail.positions(token));
        self.prefix
            .token_occurrences(token)
            .iter()
            .copied()
            .chain(tail.iter().rev().map(|&distance| self.before_len - 1 - distance))
    }
}

/// Represents a longest common subsequence found by the histogram algorithm.
#[derive(Default, Debug)]
pub struct Lcs {
    /// Starting position in the "before" sequence.
    pub before_start: u32,
    /// Starting position in the "after" sequence.
    pub after_start: u32,
    /// Length of the common subsequence.
    pub len: u32,
}

/// State for searching for the longest common subsequence.
pub struct LcsSearch {
    /// The best LCS found so far.
    lcs: Lcs,
    /// The minimum occurrence count of tokens in the best LCS.
    min_occurrences: u32,
    /// Whether any common subsequence was found.
    found_cs: bool,
}

impl LcsSearch {
    fn run(&mut self, before: &[Token], after: &[Token], occurrences: &Occurrences<'_>) {
        let seed_end = after.len() - occurrences.tail.map_or(0, |tail| tail.len as usize);
        let seeds = &after[..seed_end];
        let mut pos = 0;
        while let Some(&token) = seeds.get(pos as usize) {
            let count = occurrences.count(token);
            if self.can_seed(count) {
                pos = self.update_lcs(pos, token, count, occurrences, before, after);
                continue;
            }
            pos += 1;
        }
        if let Some(tail) = occurrences.tail {
            let mut seeds = tail.seeds();
            let after_len = after.len() as u32;
            while let Some(seed) = seeds.next(after_len, pos, self.min_occurrences) {
                let token = after[seed as usize];
                let count = occurrences.count(token);
                pos = if self.can_seed(count) {
                    self.update_lcs(seed, token, count, occurrences, before, after)
                } else {
                    seed + 1
                };
            }
        }
    }

    fn can_seed(&mut self, occurrences: u32) -> bool {
        self.found_cs |= occurrences != 0;
        occurrences != 0 && occurrences <= self.min_occurrences
    }

    fn success(&mut self) -> bool {
        !self.found_cs || self.min_occurrences <= MAX_CHAIN_LEN
    }

    #[inline]
    fn update_lcs(
        &mut self,
        after_pos: u32,
        token: Token,
        count: u32,
        histogram: &Occurrences<'_>,
        before: &[Token],
        after: &[Token],
    ) -> u32 {
        let mut next_token_idx2 = after_pos + 1;
        let mut occurrences_iter = histogram.positions(token);
        let mut token_idx1 = occurrences_iter
            .next()
            .expect("an eligible seed occurs in the before range");

        'occurrences_iter: loop {
            let mut occurrences = count;
            let mut start1 = token_idx1;
            let mut start2 = after_pos;
            loop {
                if start1 == 0 || start2 == 0 {
                    break;
                }
                let token1 = before.get(start1 as usize - 1);
                let token2 = after.get(start2 as usize - 1);
                if matches!((token1, token2), (Some(token1), Some(token2)) if token1 == token2) {
                    start1 -= 1;
                    start2 -= 1;
                    // Once `occurrences` reaches 1, further lookups cannot change it, since
                    // every matched token occurs in `before` at least once. The matching region
                    // can still grow, though, so we can only skip count lookups.
                    if occurrences > 1 {
                        let new_occurrences = histogram.count(before[start1 as usize]);
                        occurrences = occurrences.min(new_occurrences);
                    }
                } else {
                    break;
                }
            }

            let mut end1 = token_idx1 + 1;
            let mut end2 = after_pos + 1;
            loop {
                let token1 = before.get(end1 as usize);
                let token2 = after.get(end2 as usize);
                if matches!((token1, token2), (Some(token1), Some(token2)) if token1 == token2) {
                    // See above. Once `occurrences` reaches 1, it cannot further decrease.
                    if occurrences > 1 {
                        let new_occurrences = histogram.count(before[end1 as usize]);
                        occurrences = occurrences.min(new_occurrences);
                    }
                    end1 += 1;
                    end2 += 1;
                } else {
                    break;
                }
            }

            if next_token_idx2 < end2 {
                next_token_idx2 = end2;
            }

            let len = end2 - start2;
            debug_assert_eq!(len, end1 - start1);
            if self.lcs.len < len || self.min_occurrences > occurrences {
                self.min_occurrences = occurrences;
                self.lcs = Lcs {
                    before_start: start1,
                    after_start: start2,
                    len,
                };
            }

            loop {
                if let Some(next_token_idx) = occurrences_iter.next() {
                    if next_token_idx >= end1 {
                        token_idx1 = next_token_idx;
                        break;
                    }
                } else {
                    break 'occurrences_iter;
                }
            }
        }

        next_token_idx2
    }
}
