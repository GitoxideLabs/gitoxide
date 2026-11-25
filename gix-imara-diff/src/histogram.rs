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
    if before == after {
        return;
    }
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

    fn token_occurrences(&self, token: Token, count: u32) -> &[u32] {
        self.token_occurrences[token.0 as usize].as_slice(count, &self.pool)
    }

    fn num_token_occurrences(&self, token: Token) -> u32 {
        self.token_occurrences[token.0 as usize].count(&self.pool)
    }

    fn populate(&mut self, file: &[Token], start: usize) {
        for (i, &token) in file.iter().enumerate() {
            self.token_occurrences[token.0 as usize].push((i + start) as u32, &mut self.pool);
        }
    }

    fn run(
        &mut self,
        mut before: &[Token],
        mut after: &[Token],
        mut removed: &mut [bool],
        mut added: &mut [bool],
        mut cache_after_split: bool,
    ) {
        let mut common_block: Option<Box<CommonBlock>> = None;
        loop {
            if before.is_empty() {
                added.fill(true);
                return;
            } else if after.is_empty() {
                removed.fill(true);
                return;
            }

            let block = common_block
                .as_deref()
                .and_then(|block| block.for_lengths(before.len(), after.len()));
            if let Some(block) = block {
                let end = before.len() - block.index.before_end_distance as usize;
                self.populate(&before[..end - block.len as usize], 0);
                self.populate(&before[end..], end);
            } else {
                self.populate(before, 0);
            }
            match find_lcs(before, after, block, self) {
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
                        false, // The cache uses the original file endpoints.
                    );

                    // this is equivalent to (tail) recursion but implement as a loop for efficiency reasons
                    let before_end = lcs.before_start + lcs.len;
                    before = &before[before_end as usize..];
                    removed = &mut removed[before_end as usize..];

                    let after_end = lcs.after_start + lcs.len;
                    after = &after[after_end as usize..];
                    added = &mut added[after_end as usize..];

                    // The first match may already consume a long common block.
                    if std::mem::take(&mut cache_after_split) {
                        common_block = CommonBlock::find(before, after, self.token_occurrences.len()).map(Box::new);
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

/// Occurrence data for a common block, indexed by distance from its end.
struct CommonBlock {
    before_end_distance: u32,
    after_end_distance: u32,
    len: u32,
    /// Offsets into `positions`, with one extra entry for the end of the last token's list.
    offsets: Vec<u32>,
    /// The last 66 positions of each token, sorted by distance from the end.
    positions: Vec<u32>,
    /// The same distances grouped by each token's capped count in the full block.
    seeds_by_count: Vec<Vec<u32>>,
    /// The smallest before range in which every eligible token also occurs outside the block.
    overlap_len: u32,
}

impl CommonBlock {
    fn find(before: &[Token], after: &[Token], num_tokens: usize) -> Option<Self> {
        let shortest = before.len().min(after.len());
        if shortest < OVERFLOW_COUNT as usize {
            return None;
        }

        // An insertion can shift a long match away from the alignment at the file ends.
        let (mut before_offset, mut after_offset) = (before.len() - shortest, after.len() - shortest);
        let mut longest = longest_aligned_match(&before[before_offset..], &after[after_offset..]);
        if before.len() != after.len() && longest.len() != shortest {
            let from_start = longest_aligned_match(&before[..shortest], &after[..shortest]);
            if from_start.len() > longest.len() {
                longest = from_start;
                (before_offset, after_offset) = (0, 0);
            }
        }
        if longest.len() < OVERFLOW_COUNT as usize {
            return None;
        }

        let block_start = before_offset + longest.start;
        let block_end = before_offset + longest.end;
        let block = &before[block_start..block_end];
        let mut counts = vec![0u8; num_tokens];
        for &token in block {
            let count = &mut counts[token.0 as usize];
            *count = (*count + 1).min(OVERFLOW_COUNT as u8);
        }
        let mut offsets = Vec::with_capacity(num_tokens + 1);
        let mut bucket_sizes = vec![0usize; OVERFLOW_COUNT as usize];
        let mut total = 0;
        let mut seed_tokens = 0;
        offsets.push(total);
        for &count in &counts {
            if count != 0 {
                bucket_sizes[count as usize - 1] += count as usize;
                seed_tokens += usize::from(u32::from(count) <= MAX_SEED_OCCURRENCES);
            }
            total += u32::from(count);
            offsets.push(total);
        }
        let mut positions = vec![0; total as usize];
        let mut seeds_by_count: Vec<_> = bucket_sizes.into_iter().map(Vec::with_capacity).collect();
        let mut unfilled = total;
        for (distance, &token) in block.iter().rev().enumerate() {
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

        let mut overlap_len = u32::MAX;
        if seed_tokens != 0 && before.len() - block.len() >= seed_tokens {
            let outside = before
                .iter()
                .enumerate()
                .skip(block_end)
                .rev()
                .chain(before[..block_start].iter().enumerate().rev());
            for (pos, &token) in outside {
                let id = token.0 as usize;
                let count = offsets[id + 1] - offsets[id];
                if counts[id] == 0 && (1..=MAX_SEED_OCCURRENCES).contains(&count) {
                    counts[id] = 1;
                    seed_tokens -= 1;
                    if seed_tokens == 0 {
                        overlap_len = (before.len() - pos) as u32;
                        break;
                    }
                }
            }
        }
        Some(Self {
            before_end_distance: (before.len() - block_end) as u32,
            after_end_distance: (after.len() - (after_offset + longest.end)) as u32,
            len: longest.len() as u32,
            offsets,
            positions,
            seeds_by_count,
            overlap_len,
        })
    }

    fn for_lengths(&self, before_len: usize, after_len: usize) -> Option<Block<'_>> {
        let before_available = (before_len as u32).saturating_sub(self.before_end_distance);
        let after_available = (after_len as u32).saturating_sub(self.after_end_distance);
        let len = self.len.min(before_available.min(after_available));
        // A small block costs less to scan than consulting two occurrence lists at each seed.
        (len != 0 && len as usize >= before_len.min(after_len).div_ceil(2)).then_some(Block { index: self, len })
    }
}

fn longest_aligned_match(before: &[Token], after: &[Token]) -> std::ops::Range<usize> {
    debug_assert_eq!(before.len(), after.len());
    let mut longest = 0..0;
    let mut start = 0;
    for (end, (left, right)) in before.iter().zip(after).enumerate() {
        if left != right {
            if end - start >= longest.len() {
                longest = start..end;
            }
            start = end + 1;
        }
    }
    if before.len() - start >= longest.len() {
        longest = start..before.len();
    }
    longest
}

#[derive(Clone, Copy)]
struct Block<'a> {
    index: &'a CommonBlock,
    len: u32,
}

impl<'a> Block<'a> {
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

    fn seeds(self, before_len: u32) -> BlockSeeds<'a> {
        BlockSeeds {
            block: self,
            all_seeds_overlap: self.len == self.index.len && before_len >= self.index.overlap_len,
            ends: [u32::MAX; OVERFLOW_COUNT as usize],
        }
    }
}

struct BlockSeeds<'a> {
    block: Block<'a>,
    all_seeds_overlap: bool,
    ends: [u32; OVERFLOW_COUNT as usize],
}

impl BlockSeeds<'_> {
    /// Seeds are requested in forward order, so each bucket can keep its place across calls.
    fn next(&mut self, end: u32, pos: u32, cutoff: u32) -> Option<u32> {
        let remaining = end.saturating_sub(pos).min(self.block.len);
        if remaining == 0 {
            return None;
        }
        // Each removed line can lower a token's count by at most one.
        let max_count = (cutoff + (self.block.index.len - self.block.len)).min(OVERFLOW_COUNT);
        // An occurrence outside the block raises each eligible token's count by at least one.
        let max_count = max_count.saturating_sub(u32::from(self.all_seeds_overlap));
        let mut earliest = None;
        for (bucket, (positions, end)) in self.block.index.seeds_by_count[..max_count as usize]
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
        Some(end - 1 - distance)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommonBlock, OVERFLOW_COUNT, Token};

    #[test]
    fn suffix_seeds_use_the_cutoff_in_input_order() -> gix_testtools::TestResult {
        let mut tokens: Vec<_> = (0..40).flat_map(|id| [Token(id); 2]).collect();
        let tail = CommonBlock::find(&tokens, &tokens, 41).expect("long enough to cache");
        assert_eq!(
            tail.for_lengths(tokens.len(), tokens.len())
                .expect("the entire block is in range")
                .seeds(tokens.len() as u32)
                .next(tokens.len() as u32, 0, 1),
            None,
            "a unique prefix anchor makes every suffix pair ineligible"
        );

        tokens.push(Token(40));
        let tail = CommonBlock::find(&tokens, &tokens, 41).expect("long enough to cache");
        let tail = tail
            .for_lengths(tokens.len(), tokens.len())
            .expect("the entire block is in range");
        let len = tokens.len() as u32;
        assert_eq!(
            tail.seeds(len).next(len, 0, 1),
            Some(len - 1),
            "retain the rarer seed at the end"
        );
        let mut seeds = tail.seeds(len);
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
        let tail = CommonBlock::find(&tokens, &tokens, 2).expect("the repeated token reaches the overflow count");
        let shortened_len = overflow + 1;
        let shortened = tail
            .for_lengths(shortened_len, shortened_len)
            .expect("part of the block is still in range");
        assert_eq!(
            shortened.positions(Token(0)).len(),
            overflow - 1,
            "the remaining 65 occurrences of token 0 can seed a match"
        );
        assert_eq!(
            shortened
                .seeds(shortened_len as u32)
                .next(shortened_len as u32, 0, (overflow - 1) as u32),
            Some(0),
            "the earliest seed can belong to a token whose previous count overflowed"
        );
        Ok(())
    }

    #[test]
    fn overlapping_seeds_cannot_use_their_count_in_the_block_alone() -> gix_testtools::TestResult {
        let after: Vec<_> = (0..70).map(Token).collect();
        let before = after.repeat(2);
        let index = CommonBlock::find(&before, &after, 70).expect("a long common block");
        let block = index
            .for_lengths(before.len(), after.len())
            .expect("the entire block is in range");
        assert_eq!(
            block.seeds(140).next(70, 0, 1),
            None,
            "every token in the block also occurs in the prefix"
        );
        assert_eq!(
            block.seeds(139).next(70, 0, 1),
            Some(0),
            "a token can seed again once its prefix occurrence is excluded"
        );
        assert_eq!(
            block.seeds(140).next(70, 0, 2),
            Some(0),
            "overlap does not exclude seeds within the cutoff"
        );
        Ok(())
    }
}
