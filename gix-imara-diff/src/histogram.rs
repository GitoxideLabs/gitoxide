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
    let mut histogram = Histogram::new(num_tokens);
    histogram.run(before, after, removed, added, true);
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
        mut cache_tail_after_split: bool,
    ) {
        let mut common_tail: Option<Box<CommonTail>> = None;
        loop {
            if before.is_empty() {
                added.fill(true);
                return;
            } else if after.is_empty() {
                removed.fill(true);
                return;
            }

            let tail = common_tail
                .as_deref()
                .map(|tail| tail.for_len(before.len().min(after.len())));
            let tail_len = tail.map_or(0, |tail| tail.len as usize);
            self.populate(&before[..before.len() - tail_len]);
            match find_lcs(before, after, tail, self) {
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
                        false, // The prefixes have different endpoints from the cached suffix.
                    );

                    // this is equivalent to (tail) recursion but implement as a loop for efficiency reasons
                    let before_end = lcs.before_start + lcs.len;
                    before = &before[before_end as usize..];
                    removed = &mut removed[before_end as usize..];

                    let after_end = lcs.after_start + lcs.len;
                    after = &after[after_end as usize..];
                    added = &mut added[after_end as usize..];

                    // A match may consume the common suffix in the first search.
                    if std::mem::take(&mut cache_tail_after_split) {
                        common_tail = CommonTail::find(before, after, self.token_occurrences.len()).map(Box::new);
                    }
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
    len: u32,
    /// Offsets into `positions`, with one extra entry for the end of the last token's list.
    offsets: Vec<u32>,
    /// The last 66 positions of each token, sorted by distance from the end.
    positions: Vec<u32>,
    /// The same distances grouped by each token's capped count in the full suffix.
    seeds_by_count: Vec<Vec<u32>>,
}

impl CommonTail {
    fn find(before: &[Token], after: &[Token], num_tokens: usize) -> Option<Self> {
        let min_len = OVERFLOW_COUNT as usize;
        if before.len().min(after.len()) < min_len || before[before.len() - min_len..] != after[after.len() - min_len..]
        {
            return None;
        }

        let mut len = 0;
        let mut counts = vec![0u8; num_tokens];
        for (&token, &other) in before.iter().rev().zip(after.iter().rev()) {
            if token != other {
                break;
            }
            len += 1;
            let count = &mut counts[token.0 as usize];
            *count = (*count + 1).min(OVERFLOW_COUNT as u8);
        }
        let mut offsets = Vec::with_capacity(num_tokens + 1);
        let mut bucket_sizes = vec![0usize; OVERFLOW_COUNT as usize];
        let mut total = 0;
        offsets.push(total);
        for &count in &counts {
            if count != 0 {
                bucket_sizes[count as usize - 1] += count as usize;
            }
            total += u32::from(count);
            offsets.push(total);
        }
        let mut positions = vec![0; total as usize];
        let mut seeds_by_count: Vec<_> = bucket_sizes.into_iter().map(Vec::with_capacity).collect();
        let suffix = &before[before.len() - len as usize..];
        let mut unfilled = total;
        for (distance, &token) in suffix.iter().rev().enumerate() {
            let id = token.0 as usize;
            let remaining = &mut counts[id];
            if *remaining != 0 {
                let end = offsets[id + 1];
                let count = end - offsets[id];
                positions[(end - u32::from(*remaining)) as usize] = distance as u32;
                seeds_by_count[count as usize - 1].push(distance as u32);
                *remaining -= 1;
                unfilled -= 1;
                if unfilled == 0 {
                    break;
                }
            }
        }
        Some(Self {
            len,
            offsets,
            positions,
            seeds_by_count,
        })
    }

    fn for_len(&self, available: usize) -> Tail<'_> {
        Tail {
            index: self,
            len: self.len.min(available as u32),
        }
    }
}

#[derive(Clone, Copy)]
struct Tail<'a> {
    index: &'a CommonTail,
    len: u32,
}

impl<'a> Tail<'a> {
    fn positions(self, token: Token) -> &'a [u32] {
        let id = token.0 as usize;
        let (start, end) = (self.index.offsets[id] as usize, self.index.offsets[id + 1] as usize);
        if start == end {
            return &[];
        }
        let positions = &self.index.positions[start..end];
        if self.len == self.index.len {
            return positions;
        }
        &positions[..positions.partition_point(|&distance| distance < self.len)]
    }

    fn seeds(self) -> TailSeeds<'a> {
        TailSeeds {
            tail: self,
            ends: [u32::MAX; OVERFLOW_COUNT as usize],
        }
    }
}

struct TailSeeds<'a> {
    tail: Tail<'a>,
    ends: [u32; OVERFLOW_COUNT as usize],
}

impl TailSeeds<'_> {
    /// Seeds are requested in forward order, so each bucket can keep its place across calls.
    fn next(&mut self, after_len: u32, pos: u32, cutoff: u32) -> Option<u32> {
        let remaining = (after_len - pos).min(self.tail.len);
        if remaining == 0 {
            return None;
        }
        // Each removed line can lower a token's count by at most one.
        let max_count = (cutoff + (self.tail.index.len - self.tail.len)).min(OVERFLOW_COUNT);
        let mut earliest = None;
        for (bucket, (positions, end)) in self.tail.index.seeds_by_count[..max_count as usize]
            .iter()
            .zip(&mut self.ends)
            .enumerate()
        {
            let mut positions = &positions[..positions.len().min(*end as usize)];
            if positions.last().is_some_and(|&distance| distance >= remaining) {
                positions = &positions[..positions.partition_point(|&distance| distance < remaining)];
            }
            *end = positions.len() as u32;
            if let Some(&distance) = positions.last()
                && earliest.is_none_or(|(earliest, _)| distance > earliest)
            {
                earliest = Some((distance, bucket));
            }
        }
        let (distance, bucket) = earliest?;
        self.ends[bucket] -= 1;
        Some(after_len - 1 - distance)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommonTail, OVERFLOW_COUNT, Token};

    #[test]
    fn suffix_seeds_use_the_cutoff_in_input_order() -> gix_testtools::TestResult {
        let mut tokens: Vec<_> = (0..40).flat_map(|id| [Token(id); 2]).collect();
        let tail = CommonTail::find(&tokens, &tokens, 41).expect("long enough to cache");
        assert_eq!(
            tail.for_len(tokens.len()).seeds().next(tokens.len() as u32, 0, 1),
            None,
            "a unique prefix anchor makes every suffix pair ineligible"
        );

        tokens.push(Token(40));
        let tail = CommonTail::find(&tokens, &tokens, 41).expect("long enough to cache");
        let tail = tail.for_len(tokens.len());
        let len = tokens.len() as u32;
        assert_eq!(
            tail.seeds().next(len, 0, 1),
            Some(len - 1),
            "retain the rarer seed at the end"
        );
        let mut seeds = tail.seeds();
        assert_eq!(
            seeds.next(len, 0, 2),
            Some(0),
            "an earlier seed at the cutoff precedes a rarer one"
        );
        assert_eq!(seeds.next(len, 1, 2), Some(1), "resume within the same bucket");
        assert_eq!(
            seeds.next(len, 20, 2),
            Some(20),
            "skip lines already covered by a match"
        );
        Ok(())
    }

    #[test]
    fn shortened_suffix_restores_positions_for_formerly_frequent_tokens() -> gix_testtools::TestResult {
        let overflow = OVERFLOW_COUNT as usize;
        let mut tokens = vec![Token(0); 2 * overflow];
        tokens.extend([Token(1), Token(1)]);
        let tail = CommonTail::find(&tokens, &tokens, 2).expect("the repeated token reaches the overflow count");
        let shortened_len = overflow + 1;
        let shortened = tail.for_len(shortened_len);
        assert_eq!(
            shortened.positions(Token(0)).len(),
            overflow - 1,
            "the remaining 65 occurrences of token 0 can seed a match"
        );
        assert_eq!(
            shortened.seeds().next(shortened_len as u32, 0, (overflow - 1) as u32),
            Some(0),
            "the earliest seed can belong to a token whose previous count overflowed"
        );
        Ok(())
    }
}
