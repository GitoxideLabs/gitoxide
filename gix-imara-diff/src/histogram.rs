// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/histogram.rs

use crate::histogram::lcs::find_lcs;
use crate::histogram::list_pool::{ListHandle, ListPool};
use crate::intern::Token;
use crate::myers;

mod lcs;
mod list_pool;

/// Git's histogram chain limit. A seed may occur once more than this, but a match
/// must contain a token at or below the limit to avoid falling back to Myers.
const MAX_CHAIN_LEN: u32 = 64;
const MAX_SEED_OCCURRENCES: u32 = MAX_CHAIN_LEN + 1;
const OVERFLOW_COUNT: u32 = MAX_SEED_OCCURRENCES + 1;

/// State for computing histogram-based diffs.
struct Histogram {
    /// Tracks where each token appears in the "before" sequence.
    token_occurrences: Vec<ListHandle>,
    /// Memory pool for efficiently storing occurrence lists.
    pool: ListPool,
}

/// Computes a diff using the histogram algorithm.
///
/// # Parameters
///
/// * `before` - The token sequence from the first file, before changes.
/// * `after` - The token sequence from the second file, after changes.
/// * `removed` - Output array marking removed tokens
/// * `added` - Output array marking added tokens
/// * `num_tokens` - The total number of distinct tokens
pub fn diff(before: &[Token], after: &[Token], removed: &mut [bool], added: &mut [bool], num_tokens: u32) {
    let tail = CommonTail::find(before, after, num_tokens);
    let mut histogram = Histogram::new(num_tokens);
    histogram.run(before, after, removed, added, tail.as_ref());
}

impl Histogram {
    fn new(num_buckets: u32) -> Histogram {
        Histogram {
            token_occurrences: vec![ListHandle::default(); num_buckets as usize],
            pool: ListPool::new(2 * num_buckets),
        }
    }

    fn clear(&mut self) {
        self.pool.clear();
    }

    fn token_occurrences(&self, token: Token) -> &[u32] {
        self.token_occurrences[token.0 as usize].as_slice(&self.pool)
    }

    fn num_token_occurrences(&self, token: Token) -> u32 {
        self.token_occurrences[token.0 as usize].count(&self.pool)
    }

    fn populate(&mut self, file: &[Token]) {
        for (i, &token) in file.iter().enumerate() {
            self.token_occurrences[token.0 as usize].push(i as u32, &mut self.pool);
        }
    }

    fn run(
        &mut self,
        mut before: &[Token],
        mut after: &[Token],
        mut removed: &mut [bool],
        mut added: &mut [bool],
        tail: Option<&CommonTail>,
    ) {
        loop {
            if before.is_empty() {
                added.fill(true);
                return;
            } else if after.is_empty() {
                removed.fill(true);
                return;
            }

            let (tail_len, tail_positions, frequent_tokens) = tail
                .map(|tail| tail.for_len(before.len().min(after.len())))
                .unwrap_or_default();
            for &token in frequent_tokens {
                self.token_occurrences[token.0 as usize].mark_overflow(&self.pool);
            }
            self.populate(&before[..before.len() - tail_len]);
            for &distance in tail_positions.iter().rev() {
                let position = before.len() - 1 - distance as usize;
                let token = before[position];
                self.token_occurrences[token.0 as usize].push(position as u32, &mut self.pool);
            }
            match find_lcs(before, after, after.len() - tail_len, tail_positions, self) {
                // no lcs was found, that means that file1 and file2 two have nothing in common
                Some(lcs) if lcs.len == 0 => {
                    added.fill(true);
                    removed.fill(true);
                    return;
                }
                Some(lcs) => {
                    self.run(
                        &before[..lcs.before_start as usize],
                        &after[..lcs.after_start as usize],
                        &mut removed[..lcs.before_start as usize],
                        &mut added[..lcs.after_start as usize],
                        None, // The prefixes have different endpoints from the cached suffix.
                    );

                    // this is equivalent to (tail) recursion but implement as a loop for efficiency reasons
                    let before_end = lcs.before_start + lcs.len;
                    before = &before[before_end as usize..];
                    removed = &mut removed[before_end as usize..];

                    let after_end = lcs.after_start + lcs.len;
                    after = &after[after_end as usize..];
                    added = &mut added[after_end as usize..];
                }
                None => {
                    // we are diffing two extremely large repetitive files
                    // this is a worst case for histogram diff with O(N^2) performance
                    // fallback to myers to maintain linear time complexity
                    myers::diff(before, after, removed, added, false);
                    return;
                }
            }
        }
    }
}

/// The common suffix's occurrence data, indexed by distance from the end of either file.
struct CommonTail {
    len: usize,
    /// The last 66 positions of each token suffice to count it even when the suffix shortens.
    positions: Vec<u32>,
    /// Positions belonging to tokens that can still seed a match in the full suffix.
    seed_positions: Vec<u32>,
    frequent_tokens: Vec<Token>,
    /// The minimum suffix length that still contains 66 occurrences of every frequent token.
    min_overflow_len: usize,
}

impl CommonTail {
    fn find(before: &[Token], after: &[Token], num_tokens: u32) -> Option<Self> {
        let min_len = OVERFLOW_COUNT as usize;
        if before.len().min(after.len()) < min_len || before[before.len() - min_len..] != after[after.len() - min_len..]
        {
            return None;
        }

        let mut len = 0;
        let mut counts = vec![0u8; num_tokens as usize];
        let mut positions = Vec::new();
        let mut frequent_tokens = Vec::new();
        let mut min_overflow_len = 0;
        let overflow_count = OVERFLOW_COUNT as u8;
        for (&token, &other) in before.iter().rev().zip(after.iter().rev()) {
            if token != other {
                break;
            }
            let distance = len;
            len += 1;
            let count = &mut counts[token.0 as usize];
            if *count == overflow_count {
                continue;
            }
            *count += 1;
            if *count == overflow_count {
                // Defer saving positions until there is a token we can skip.
                if frequent_tokens.is_empty() {
                    positions.extend(0..distance as u32);
                }
                frequent_tokens.push(token);
                min_overflow_len = distance + 1;
            }
            if min_overflow_len != 0 {
                positions.push(distance as u32);
            }
        }
        if frequent_tokens.is_empty() {
            return None;
        }
        let seed_positions = positions
            .iter()
            .copied()
            .filter(|&distance| counts[before[before.len() - 1 - distance as usize].0 as usize] < overflow_count)
            .collect();
        Some(Self {
            len,
            positions,
            seed_positions,
            frequent_tokens,
            min_overflow_len,
        })
    }

    fn for_len(&self, available: usize) -> (usize, &[u32], &[Token]) {
        let len = available.min(self.len);
        let (positions, frequent_tokens) = if len >= self.min_overflow_len {
            (self.seed_positions.as_slice(), self.frequent_tokens.as_slice())
        } else {
            // A shortened suffix can make a previously frequent token eligible again.
            (self.positions.as_slice(), &[][..])
        };
        let end = positions.partition_point(|&distance| (distance as usize) < len);
        (len, &positions[..end], frequent_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommonTail, OVERFLOW_COUNT, Token};

    #[test]
    fn shortened_suffix_restores_positions_for_formerly_frequent_tokens() -> gix_testtools::TestResult {
        let overflow = OVERFLOW_COUNT as usize;
        let mut tokens = vec![Token(0); 2 * overflow];
        tokens.extend([Token(1), Token(1)]);
        let tail = CommonTail::find(&tokens, &tokens, 2).expect("the repeated token reaches the overflow count");
        assert_eq!(
            tail.for_len(tokens.len()),
            (tokens.len(), [0, 1].as_slice(), [Token(0)].as_slice()),
            "only the rare positions are needed while token 0 exceeds the seed limit"
        );
        let shortened_len = overflow + 1;
        let expected: Vec<_> = (0..shortened_len as u32).collect();
        assert_eq!(
            tail.for_len(shortened_len),
            (shortened_len, expected.as_slice(), &[][..]),
            "the remaining 65 occurrences of token 0 can seed a match"
        );
        Ok(())
    }
}
