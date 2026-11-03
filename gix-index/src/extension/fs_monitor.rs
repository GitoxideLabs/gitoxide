use bstr::BString;

use crate::{
    extension::{FsMonitor, Signature},
    util::{read_u32, read_u64, split_at_byte_exclusive},
};

/// A checkpoint identifying the filesystem changes already reflected by the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// A timestamp used by version 1 monitor hooks.
    V1 {
        /// Nanoseconds since the Unix epoch.
        nanos_since_1970: u64,
    },
    /// An opaque token used by version 2 hooks and the built-in monitor.
    V2 {
        /// A token without NUL bytes.
        token: BString,
    },
}

/// The index extension signature.
pub const SIGNATURE: Signature = *b"FSMN";

pub(crate) fn decode(data: &[u8]) -> Option<FsMonitor> {
    let (version, data) = read_u32(data)?;
    let (token, data) = match version {
        1 => {
            let (nanos_since_1970, data) = read_u64(data)?;
            (Token::V1 { nanos_since_1970 }, data)
        }
        2 => {
            let (token, data) = split_at_byte_exclusive(data, 0)?;
            (Token::V2 { token: token.into() }, data)
        }
        _ => return None,
    };

    let (ewah_size, data) = read_u32(data)?;
    let ((entry_dirty, extra), data) = data
        .split_at_checked(ewah_size as usize)
        .and_then(|(entry_dirty, data)| {
            gix_bitmap::ewah::decode(entry_dirty)
                .ok()
                .map(|entry_dirty| (entry_dirty, data))
        })?;
    if !extra.is_empty() {
        return None;
    }

    if !data.is_empty() {
        return None;
    }

    FsMonitor { token, entry_dirty }.into()
}

impl FsMonitor {
    /// Return the checkpoint to pass to the next filesystem monitor query.
    pub fn token(&self) -> &Token {
        &self.token
    }

    /// Decode the entries known to be unchanged at this checkpoint.
    ///
    /// Returns `None` for a malformed bitmap or one exceeding `num_entries`.
    /// The returned flags may only be trusted after querying the monitor and invalidating
    /// its reported paths. Entries beyond the bitmap are valid, as in Git's `FSMN` format.
    pub fn valid_entries(&self, num_entries: usize) -> Option<Vec<bool>> {
        if self.entry_dirty.num_bits() > num_entries {
            return None;
        }
        let mut valid = vec![true; num_entries];
        self.entry_dirty.for_each_set_bit(|index| {
            *valid.get_mut(index)? = false;
            Some(())
        })?;
        Some(valid)
    }

    pub(crate) fn from_entries(token: Token, entries: &[crate::Entry]) -> Self {
        let dirty: Vec<_> = entries.iter().map(entry_is_dirty).collect();
        Self {
            token,
            entry_dirty: gix_bitmap::ewah::Vec::from_bits(&dirty).expect("index entry counts fit into u32"),
        }
    }

    pub(crate) fn write_to(&self, out: &mut dyn std::io::Write, entries: &[crate::Entry]) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};

        let mut data = Vec::new();
        match &self.token {
            Token::V1 { nanos_since_1970 } => {
                data.extend_from_slice(&1u32.to_be_bytes());
                data.extend_from_slice(&nanos_since_1970.to_be_bytes());
            }
            Token::V2 { token } => {
                if token.contains(&0) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "filesystem monitor token contains NUL",
                    ));
                }
                data.extend_from_slice(&2u32.to_be_bytes());
                data.extend_from_slice(token);
                data.push(0);
            }
        }
        let dirty: Vec<_> = entries
            .iter()
            .filter(|entry| !entry.flags.contains(crate::entry::Flags::REMOVE))
            .map(entry_is_dirty)
            .collect();
        let bitmap = gix_bitmap::ewah::Vec::from_bits(&dirty).expect("index entry counts fit into u32");
        let size_offset = data.len();
        data.extend_from_slice(&[0; 4]);
        bitmap.write_to(&mut data)?;
        let bitmap_size = u32::try_from(data.len() - size_offset - 4)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "filesystem monitor bitmap exceeds u32::MAX"))?;
        data[size_offset..size_offset + 4].copy_from_slice(&bitmap_size.to_be_bytes());
        let size = u32::try_from(data.len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "filesystem monitor extension exceeds u32::MAX"))?;
        out.write_all(&SIGNATURE)?;
        out.write_all(&size.to_be_bytes())?;
        out.write_all(&data)
    }
}

fn entry_is_dirty(entry: &crate::Entry) -> bool {
    !entry.flags.contains(crate::entry::Flags::FSMONITOR_VALID) || !entry_is_eligible(entry)
}

pub(crate) fn entry_is_eligible(entry: &crate::Entry) -> bool {
    !entry
        .flags
        .intersects(crate::entry::Flags::INTENT_TO_ADD | crate::entry::Flags::REMOVE)
        && entry.stage_raw() == 0
        && !entry.mode.is_submodule()
        && !entry.mode.is_sparse()
}
