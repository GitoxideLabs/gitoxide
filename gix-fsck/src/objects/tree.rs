use std::collections::HashSet;

use bstr::ByteSlice;

use super::{MessageId as M, Options, Reporter};

pub(super) fn validate(data: &[u8], hash: gix_hash::Kind, options: &Options, out: &mut Reporter<'_>) {
    let mut previous: Option<(&[u8], bool)> = None;
    let mut names = HashSet::new();
    for entry in Entries::new(data, hash) {
        let Ok(RawEntry {
            mode,
            name,
            object_id,
            zero_padded_mode,
        }) = entry
        else {
            out.report(M::BadTree);
            return;
        };
        if object_id.is_null() {
            out.report(M::NullSha1);
        }
        if name.contains(&b'/') {
            out.report(M::FullPathname);
        }
        if name.is_empty() {
            out.report(M::EmptyName);
        }
        if name == b"." {
            out.report(M::HasDot);
        }
        if name == b".." {
            out.report(M::HasDotdot);
        }
        if name.split(|byte| *byte == b'\\').any(is_dot_git) {
            out.report(M::HasDotgit);
        }
        if name.len() > options.max_tree_entry_length {
            out.report(M::LargePathname);
        }
        if zero_padded_mode {
            out.report(M::ZeroPaddedFilemode);
        }
        if !matches!(mode, 0o100644 | 0o100755 | 0o120000 | 0o40000 | 0o160000) && (mode != 0o100664 || options.strict)
        {
            out.report(M::BadFilemode);
        }
        let is_tree = mode & 0o170000 == 0o40000;
        let duplicate = !names.insert(name);
        if duplicate {
            out.report(M::DuplicateEntries);
        }
        if let Some((previous, previous_is_tree)) = previous
            && !duplicate
            && previous
                .iter()
                .copied()
                .chain([if previous_is_tree { b'/' } else { 0 }])
                .cmp(name.iter().copied().chain([if is_tree { b'/' } else { 0 }]))
                .is_gt()
        {
            out.report(M::TreeNotSorted);
        }
        previous = Some((name, is_tree));
    }
}

fn is_dot_git(name: &[u8]) -> bool {
    gix_validate::path::SpecialComponent::Git.matches(name.as_bstr())
}

/// A raw tree entry retains nonstandard modes for configurable fsck diagnostics.
pub(super) struct RawEntry<'a> {
    pub mode: u32,
    pub name: &'a [u8],
    pub object_id: &'a gix_hash::oid,
    pub zero_padded_mode: bool,
}

pub(super) struct Entries<'a> {
    data: &'a [u8],
    hash: gix_hash::Kind,
}

impl<'a> Entries<'a> {
    pub fn new(data: &'a [u8], hash: gix_hash::Kind) -> Self {
        Self { data, hash }
    }
}

impl<'a> Iterator for Entries<'a> {
    type Item = Result<RawEntry<'a>, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match raw_entry(self.data, self.hash.len_in_bytes()) {
            Some((entry, rest)) => {
                self.data = rest;
                Some(Ok(entry))
            }
            None => {
                self.data = &[];
                Some(Err(()))
            }
        }
    }
}

fn raw_entry(data: &[u8], hash_len: usize) -> Option<(RawEntry<'_>, &[u8])> {
    let space = data.find_byte(b' ')?;
    let mode_bytes = &data[..space];
    if mode_bytes.is_empty() || mode_bytes.iter().any(|byte| !(b'0'..=b'7').contains(byte)) {
        return None;
    }
    let mode = mode_bytes.iter().try_fold(0u32, |value, byte| {
        value.checked_mul(8)?.checked_add(u32::from(byte - b'0'))
    })?;
    let data = &data[space + 1..];
    let nul = data.find_byte(0)?;
    let name = &data[..nul];
    if name.is_empty() {
        return None;
    }
    let data = &data[nul + 1..];
    if data.len() < hash_len {
        return None;
    }
    let (object_id, rest) = data.split_at(hash_len);
    Some((
        RawEntry {
            mode,
            name,
            object_id: gix_hash::oid::try_from_bytes(object_id).ok()?,
            zero_padded_mode: mode_bytes[0] == b'0',
        },
        rest,
    ))
}
