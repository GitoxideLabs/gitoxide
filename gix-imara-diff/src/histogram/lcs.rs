// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/histogram/lcs.rs

use crate::histogram::{Histogram, MAX_CHAIN_LEN, MAX_SEED_OCCURRENCES};
use crate::intern::Token;

/// Finds the longest common subsequence (LCS) using a histogram-based approach.
///
/// In the shared suffix at `seed_end`, consider seeds only at the given distances from the end.
/// Returns `None` if the sequences are highly repetitive and should fall back to Myers.
pub(super) fn find_lcs(
    before: &[Token],
    after: &[Token],
    seed_end: usize,
    tail_positions: &[u32],
    histogram: &mut Histogram,
) -> Option<Lcs> {
    let mut search = LcsSearch {
        lcs: Lcs::default(),
        min_occurrences: MAX_SEED_OCCURRENCES,
        found_cs: seed_end < after.len(),
    };
    search.run(before, after, seed_end, tail_positions, histogram);
    if search.success() { Some(search.lcs) } else { None }
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
    fn run(
        &mut self,
        before: &[Token],
        after: &[Token],
        seed_end: usize,
        tail_positions: &[u32],
        histogram: &mut Histogram,
    ) {
        let seeds = &after[..seed_end];
        let mut pos = 0;
        while let Some(&token) = seeds.get(pos as usize) {
            if self.can_seed(histogram.num_token_occurrences(token)) {
                pos = self.update_lcs(pos, token, histogram, before, after);
                continue;
            }
            pos += 1;
        }
        let after_len = after.len() as u32;
        for &distance in tail_positions.iter().rev() {
            if pos == after_len {
                break;
            }
            let seed = after_len - 1 - distance;
            if seed >= pos {
                let token = after[seed as usize];
                if self.can_seed(histogram.num_token_occurrences(token)) {
                    pos = self.update_lcs(seed, token, histogram, before, after);
                }
            }
        }

        histogram.clear();
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
        histogram: &Histogram,
        before: &[Token],
        after: &[Token],
    ) -> u32 {
        let mut next_token_idx2 = after_pos + 1;
        let mut occurrences_iter = histogram.token_occurrences(token).iter().copied();
        let mut token_idx1 = occurrences_iter.next().unwrap();

        'occurrences_iter: loop {
            let mut occurrences = histogram.num_token_occurrences(token);
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
                        let new_occurrences = histogram.num_token_occurrences(before[start1 as usize]);
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
                        let new_occurrences = histogram.num_token_occurrences(before[end1 as usize]);
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
