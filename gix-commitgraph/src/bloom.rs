//! Query support for changed-path Bloom filters stored in commit-graph files.

use std::io::Cursor;

use bstr::BStr;

use crate::{file, from_be_u32, BloomFilterSettings, File, Graph, Position};

const SEED0: u32 = 0x293a_e76f;
const SEED1: u32 = 0x7e64_6e2c;
const BITS_PER_WORD: u64 = 8;

/// Precomputed hash positions for a path using Bloom filter settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BloomKey {
    h0: u32,
    h1: u32,
    num_hashes: u32,
}

impl BloomKey {
    /// Build a key for `path`.
    ///
    /// `path` must use `/` as separator, matching Git's changed-path Bloom filter expectations.
    pub fn from_path(path: &BStr, settings: BloomFilterSettings) -> Self {
        Self::from_bytes(path.as_ref(), settings)
    }

    /// Build keys for `path` and each directory prefix.
    ///
    /// For `a/b/c`, this yields keys for `a/b/c`, `a/b`, and `a`.
    /// `path` must use `/` as separator.
    pub fn from_path_with_prefixes(path: &BStr, settings: BloomFilterSettings) -> Vec<Self> {
        let bytes = path.as_ref();
        let mut out = vec![Self::from_bytes(bytes, settings)];

        let mut idx = bytes.len();
        while idx > 0 {
            idx -= 1;
            if bytes[idx] == b'/' {
                out.push(Self::from_bytes(&bytes[..idx], settings));
            }
        }
        out
    }

    fn from_bytes(path: &[u8], settings: BloomFilterSettings) -> Self {
        Self {
            h0: murmur3_v2(SEED0, path),
            h1: murmur3_v2(SEED1, path),
            num_hashes: settings.num_hashes,
        }
    }

    /// Query whether this key may be contained in `filter_data`.
    ///
    /// Returns `None` if the filter is unusable (empty data), `Some(false)` on a definite miss,
    /// and `Some(true)` on a possible hit.
    pub fn contains(&self, filter_data: &[u8]) -> Option<bool> {
        let modulo = (filter_data.len() as u64) * BITS_PER_WORD;
        if modulo == 0 {
            return None;
        }

        for i in 0..self.num_hashes {
            let hash = self.h0.wrapping_add(i.wrapping_mul(self.h1));
            let bit_pos = u64::from(hash) % modulo;
            let byte_pos = (bit_pos / BITS_PER_WORD) as usize;
            let mask = 1u8 << (bit_pos % BITS_PER_WORD);
            if filter_data[byte_pos] & mask == 0 {
                return Some(false);
            }
        }
        Some(true)
    }
}

impl File {
    /// Query if `path` may be present in the changed-path Bloom filter for commit `pos`.
    ///
    /// Checks the full path and every directory prefix against the filter,
    /// matching Git's `bloom_filter_contains_vec()` behavior for reduced false positives.
    pub fn maybe_contains_path(&self, pos: file::Position, path: &BStr) -> Option<bool> {
        let (data, settings) = self.bloom_filter_at(pos)?;
        let keys = BloomKey::from_path_with_prefixes(path, settings);
        for key in &keys {
            match key.contains(data) {
                Some(false) => return Some(false),
                None => return None,
                Some(true) => {}
            }
        }
        Some(true)
    }

    /// Query if all `keys` may be present in the changed-path Bloom filter for commit `pos`.
    ///
    /// This corresponds to Git's `bloom_filter_contains_vec()` behavior.
    pub fn maybe_contains_all_keys(&self, pos: file::Position, keys: &[BloomKey]) -> Option<bool> {
        let (data, _settings) = self.bloom_filter_at(pos)?;
        if keys.iter().all(|key| key.contains(data) == Some(true)) {
            Some(true)
        } else {
            Some(false)
        }
    }

    fn bloom_filter_at(&self, pos: file::Position) -> Option<(&[u8], BloomFilterSettings)> {
        let settings = self.bloom_filter_settings?;
        let index_offset = self.bloom_filter_index_offset?;
        let data_offset = self.bloom_filter_data_offset?;
        if pos.0 >= self.num_commits() {
            return None;
        }

        let lex = pos.0 as usize;
        let end = from_be_u32(&self.data[index_offset + lex * 4..][..4]);
        let start = if lex == 0 {
            0
        } else {
            from_be_u32(&self.data[index_offset + (lex - 1) * 4..][..4])
        };
        let start = start as usize;
        let end = end as usize;
        if start > end || end > self.bloom_filter_data_len {
            return None;
        }
        let start = data_offset.checked_add(start)?;
        let end = data_offset.checked_add(end)?;
        self.data.get(start..end).map(|data| (data, settings))
    }
}

impl Graph {
    /// Query by commit id if `path` may be present in changed-path Bloom filters.
    pub fn maybe_contains_path_by_id(&self, id: impl AsRef<gix_hash::oid>, path: &BStr) -> Option<bool> {
        let pos = self.lookup(id)?;
        self.maybe_contains_path(pos, path)
    }

    /// Query by graph position if `path` may be present in changed-path Bloom filters.
    pub fn maybe_contains_path(&self, pos: Position, path: &BStr) -> Option<bool> {
        self.commit_at(pos).maybe_contains_path(path)
    }
}

pub(crate) fn murmur3_v2(seed: u32, data: &[u8]) -> u32 {
    let mut reader = Cursor::new(data);
    murmur3::murmur3_32(&mut reader, seed).expect("reading from memory does not fail")
}
#[cfg(test)]
mod tests {
    use super::{murmur3_v2, BloomKey};
    use crate::BloomFilterSettings;
    use bstr::BStr;

    #[test]
    fn murmur3_known_vectors_match_git_and_reference_values() {
        assert_eq!(murmur3_v2(0, b""), 0x0000_0000);
        assert_eq!(murmur3_v2(0, b"Hello world!"), 0x627b_0c2c);
        assert_eq!(
            murmur3_v2(0, b"The quick brown fox jumps over the lazy dog"),
            0x2e4f_f723
        );
    }

    #[test]
    fn bloom_key_for_empty_path_matches_git_vector() {
        let settings = BloomFilterSettings {
            hash_version: 2,
            num_hashes: 7,
            bits_per_entry: 10,
        };
        let key = BloomKey::from_path(BStr::new(b""), settings);
        assert_eq!(
            (0..key.num_hashes)
                .map(|i| key.h0.wrapping_add(i.wrapping_mul(key.h1)))
                .collect::<Vec<_>>(),
            &[
                0x5615_800c,
                0x5b96_6560,
                0x6117_4ab4,
                0x6698_3008,
                0x6c19_155c,
                0x7199_fab0,
                0x771a_e004
            ]
        );
    }
}
