use std::collections::HashMap;

pub(crate) const MAX_SCORE: u16 = super::Similarity::MAX_SCORE;

/// Git's `diffcore-delta.c` signature: byte counts for hashed, newline-delimited spans of at most 64 bytes.
#[derive(Debug)]
pub(crate) struct Signature {
    size: usize,
    spans: Vec<(u32, usize)>,
}

impl Signature {
    pub(crate) fn new(data: &[u8], is_binary: bool) -> Self {
        let mut counts = HashMap::<u32, usize>::new();
        let mut add_span = |first: u32, second: u32, len| {
            let hash = first.wrapping_add(second.wrapping_mul(0x61)) % 107_927;
            *counts.entry(hash).or_default() += len;
        };
        let (mut first, mut second) = (0u32, 0u32);
        let mut len = 0;
        for (idx, &byte) in data.iter().enumerate() {
            if !is_binary && byte == b'\r' && data.get(idx + 1) == Some(&b'\n') {
                continue;
            }
            let previous = first;
            first = ((first << 7) ^ (second >> 25)).wrapping_add(u32::from(byte));
            second = (second << 7) ^ (previous >> 25);
            len += 1;
            if len == 64 || byte == b'\n' {
                add_span(first, second, len);
                len = 0;
                first = 0;
                second = 0;
            }
        }
        if len != 0 {
            add_span(first, second, len);
        }
        let mut spans: Vec<_> = counts.into_iter().collect();
        spans.sort_unstable_by_key(|&(hash, _)| hash);
        Signature {
            size: data.len(),
            spans,
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn similarity(&self, other: &Self) -> u16 {
        let max_size = self.size.max(other.size);
        if max_size == 0 {
            return 0;
        }
        let mut destinations = other.spans.iter();
        let mut destination = destinations.next();
        let mut copied = 0;
        for &(src_hash, src_count) in &self.spans {
            while let Some(&(dst_hash, _)) = destination {
                if dst_hash >= src_hash {
                    break;
                }
                destination = destinations.next();
            }
            let Some(&(dst_hash, dst_count)) = destination else {
                break;
            };
            if src_hash == dst_hash {
                copied += src_count.min(dst_count);
                destination = destinations.next();
            }
        }
        match (copied as u64).checked_mul(u64::from(MAX_SCORE)) {
            Some(scaled) => (scaled / max_size as u64) as u16,
            None => ((copied as u128 * u128::from(MAX_SCORE)) / max_size as u128) as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SCORE, Signature};

    #[test]
    fn span_order_does_not_affect_similarity() -> gix_error::TestResult {
        let src = Signature::new(
            b"alpha
beta
tail",
            false,
        );
        let dst = Signature::new(
            b"beta
alpha
tail",
            false,
        );
        assert_eq!(
            src.similarity(&dst),
            MAX_SCORE,
            "the final partial span also contributes"
        );
        Ok(())
    }

    #[test]
    fn repeated_spans_only_count_the_bytes_present_in_both_files() -> gix_error::TestResult {
        let src = Signature::new(
            b"same
same
other
",
            false,
        );
        let dst = Signature::new(
            b"same
other
other
",
            false,
        );
        assert_eq!(
            src.similarity(&dst),
            38_823,
            "eleven common bytes out of seventeen, rounded down"
        );
        assert_eq!(dst.similarity(&src), src.similarity(&dst), "similarity is symmetric");
        Ok(())
    }

    #[test]
    fn crlf_is_normalized_only_when_hashing_text() -> gix_error::TestResult {
        let text = Signature::new(b"abc\r\n", false);
        let binary = Signature::new(b"abc\r\n", true);
        let lf = Signature::new(b"abc\n", false);
        assert_eq!(
            text.similarity(&lf),
            48_000,
            "CR is omitted from counts, but the denominator uses raw bytes"
        );
        assert_eq!(binary.similarity(&lf), 0, "binary CRLF remains part of the hashed span");
        assert_eq!(text.size(), 5, "raw size is available for the size-delta precheck");
        Ok(())
    }

    #[test]
    fn long_lines_and_binary_data_are_split_at_64_bytes() -> gix_error::TestResult {
        let mut src = vec![b'x'; 64];
        let mut dst = src.clone();
        src.extend_from_slice(b"old");
        dst.extend_from_slice(b"new");
        assert_eq!(
            Signature::new(&src, true).similarity(&Signature::new(&dst, true)),
            57_313,
            "the unchanged 64-byte span contributes even without a newline"
        );
        assert_eq!(
            Signature::new(&[0xff; 64], true).spans,
            [(12_233, 64)],
            "span hashes use 32-bit wrapping arithmetic"
        );
        Ok(())
    }

    #[test]
    fn hash_collisions_count_as_copied_bytes() -> gix_error::TestResult {
        let src = Signature::new(b"   ", false);
        let dst = Signature::new(b"&k7", false);
        assert_eq!(src.spans, [(96_708, 3)], "Git reduces span hashes modulo 107927");
        assert_eq!(
            src.similarity(&dst),
            MAX_SCORE,
            "Git's similarity estimate intentionally permits hash collisions"
        );
        assert_eq!(
            Signature::new(b"", false).similarity(&src),
            0,
            "empty content cannot provide copied bytes"
        );
        Ok(())
    }

    #[test]
    fn span_subsets_match_a_direct_intersection() -> gix_error::TestResult {
        let signature = |spans: &[(u32, usize)], mask| {
            let spans: Vec<_> = spans
                .iter()
                .enumerate()
                .filter_map(|(idx, &span)| (mask & (1 << idx) != 0).then_some(span))
                .collect();
            Signature {
                size: spans.iter().map(|&(_, count)| count).sum(),
                spans,
            }
        };
        for source_mask in 0..16 {
            let source = signature(&[(1, 3), (3, 7), (7, 11), (9, 19)], source_mask);
            for destination_mask in 0..16 {
                let destination = signature(&[(1, 5), (3, 3), (7, 17), (9, 13)], destination_mask);
                let copied: usize = source
                    .spans
                    .iter()
                    .filter_map(|&(hash, count)| {
                        destination
                            .spans
                            .iter()
                            .find(|&&(other, _)| hash == other)
                            .map(|&(_, other_count)| count.min(other_count))
                    })
                    .sum();
                let size = source.size.max(destination.size);
                let expected = if size == 0 {
                    0
                } else {
                    (copied as u128 * u128::from(MAX_SCORE) / size as u128) as u16
                };
                assert_eq!(
                    (source.similarity(&destination), destination.similarity(&source)),
                    (expected, expected),
                    "all empty, equal, interleaved and disjoint subsets agree for {source_mask}/{destination_mask}"
                );
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn large_counts_keep_integer_precision_and_do_not_overflow() -> gix_error::TestResult {
        let boundary = usize::MAX / usize::from(MAX_SCORE);
        for copied in [1, boundary - 1, boundary, boundary + 1, usize::MAX - 1, usize::MAX] {
            let source = Signature {
                size: copied,
                spans: vec![(1, copied)],
            };
            for size in [copied, copied.saturating_add(1), usize::MAX] {
                let mut destination = Signature {
                    size,
                    spans: vec![(1, copied)],
                };
                if size > copied {
                    destination.spans.push((2, size - copied));
                }
                let expected = (copied as u128 * u128::from(MAX_SCORE) / size as u128) as u16;
                assert_eq!(
                    source.similarity(&destination),
                    expected,
                    "scoring {copied} copied bytes out of {size} uses exact integer arithmetic"
                );
            }
        }
        Ok(())
    }
}
