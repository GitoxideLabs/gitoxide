use std::ops::Range;

use crate::{
    decode::{self, header},
    entry,
    util::{read_u32, split_at_byte_exclusive, var_int},
    Entry, Version,
};

/// a guess directly from git sources
pub const AVERAGE_V4_DELTA_PATH_LEN_IN_BYTES: usize = 80;

pub struct Outcome {
    pub is_sparse: bool,
}

fn entries_block_size_in_bytes(
    on_disk_size: usize,
    offset_to_extensions: Option<usize>,
    object_hash: gix_hash::Kind,
) -> usize {
    offset_to_extensions
        .unwrap_or_else(|| on_disk_size.saturating_sub(object_hash.len_in_bytes()))
        .saturating_sub(header::SIZE)
}

const fn on_disk_entry_sans_path(object_hash: gix_hash::Kind) -> usize {
    8 + // ctime
    8 + // mtime
    (4 * 6) + // various stat fields
    2 + // flag, ignore extended flag as we'd rather overallocate a bit
    object_hash.len_in_bytes()
}

/// Return a lower bound for the number of on-disk bytes one entry must occupy,
/// based on `object_hash` and `version`.
///
/// This is used to reject impossible entry counts and to cap estimates derived from untrusted data.
/// For V2/V3, entries are padded to 8-byte boundaries, so the fixed payload is rounded up to the
/// next multiple of 8. For V4, paths are delta-encoded, but each entry still
/// needs at least a one-byte strip-length varint plus the trailing NUL of the path suffix, hence
/// `size + 2`.
const fn min_entry_size_in_bytes(object_hash: gix_hash::Kind, version: Version) -> usize {
    let size = on_disk_entry_sans_path(object_hash);
    match version {
        Version::V2 | Version::V3 => size.next_multiple_of(8),
        Version::V4 => size + 2,
    }
}

/// Compute an upper bound for how many entries can physically fit into the on-disk entries block.
///
/// The entries block is the index payload after the header and before extensions or, if there are no
/// extensions, before the trailing checksum. We divide that byte budget by [`min_entry_size_in_bytes()`]
/// to obtain the largest plausible entry count for the declared index version. This is intentionally a
/// coarse upper bound used to reject corrupt headers that claim more entries than the remaining bytes
/// could possibly encode.
pub fn max_entries_possible(
    on_disk_size: usize,
    offset_to_extensions: Option<usize>,
    object_hash: gix_hash::Kind,
    version: Version,
) -> usize {
    entries_block_size_in_bytes(on_disk_size, offset_to_extensions, object_hash)
        / min_entry_size_in_bytes(object_hash, version)
}
pub fn estimate_path_storage_requirements_in_bytes(
    num_entries: u32,
    on_disk_size: usize,
    offset_to_extensions: Option<usize>,
    object_hash: gix_hash::Kind,
    version: Version,
) -> usize {
    let size_of_entries_block = entries_block_size_in_bytes(on_disk_size, offset_to_extensions, object_hash);
    match version {
        Version::V3 | Version::V2 => {
            // V2/V3 store full paths in the entries block, so whatever remains after subtracting the fixed
            // entry header portion is an upper bound for path bytes we may copy into memory.
            size_of_entries_block.saturating_sub(num_entries as usize * on_disk_entry_sans_path(object_hash))
        }
        Version::V4 => size_of_entries_block
            // V4 stores delta-compressed paths on disk. Subtract the minimum per-entry non-path payload to
            // validate how many on-disk bytes could plausibly remain for path suffixes at all.
            .saturating_sub(num_entries as usize * (on_disk_entry_sans_path(object_hash) + 2))
            // The in-memory backing stores expanded paths, so the on-disk remainder is not a good estimate by
            // itself. Keep the historic per-entry heuristic, but clamp it to what the entries block can plausibly
            // contain so corrupt entry counts cannot drive an excessive preallocation.
            .min(num_entries as usize * AVERAGE_V4_DELTA_PATH_LEN_IN_BYTES),
    }
}

/// Note that `data` must point to the beginning of the entries, right past the header.
pub fn chunk<'a>(
    mut data: &'a [u8],
    entries: &mut Vec<Entry>,
    path_backing: &mut Vec<u8>,
    num_entries: u32,
    object_hash: gix_hash::Kind,
    version: Version,
) -> Result<(Outcome, &'a [u8]), decode::Error> {
    let mut is_sparse = false;
    let has_delta_paths = version == Version::V4;
    let mut prev_path = None;
    let mut delta_buf = Vec::<u8>::with_capacity(AVERAGE_V4_DELTA_PATH_LEN_IN_BYTES);

    for idx in 0..num_entries {
        let (entry, remaining) = load_one(
            data,
            path_backing,
            object_hash.len_in_bytes(),
            has_delta_paths,
            prev_path,
        )
        .ok_or(decode::Error::Entry { index: idx })?;

        data = remaining;
        is_sparse |= entry.mode.is_sparse();
        // TODO: entries are actually in an intrusive collection, with path as key. Could be set for us. This affects 'ignore_case' which we
        //       also don't yet handle but probably could, maybe even smartly with the collection.
        //       For now it's unclear to me how they access the index, they could iterate quickly, and have fast access by path.
        entries.push(entry);
        prev_path = entries.last().map(|e| (e.path.clone(), &mut delta_buf));
    }

    Ok((Outcome { is_sparse }, data))
}

/// Note that `prev_path` is only useful if the version is V4
fn load_one<'a>(
    data: &'a [u8],
    path_backing: &mut Vec<u8>,
    hash_len: usize,
    has_delta_paths: bool,
    prev_path_and_buf: Option<(Range<usize>, &mut Vec<u8>)>,
) -> Option<(Entry, &'a [u8])> {
    let first_byte_of_entry = data.as_ptr() as usize;
    let (ctime_secs, data) = read_u32(data)?;
    let (ctime_nsecs, data) = read_u32(data)?;
    let (mtime_secs, data) = read_u32(data)?;
    let (mtime_nsecs, data) = read_u32(data)?;
    let (dev, data) = read_u32(data)?;
    let (ino, data) = read_u32(data)?;
    let (mode, data) = read_u32(data)?;
    let (uid, data) = read_u32(data)?;
    let (gid, data) = read_u32(data)?;
    let (size, data) = read_u32(data)?;
    let (hash, data) = data.split_at_checked(hash_len)?;
    let (flags, data) = read_u16(data)?;
    let flags = entry::at_rest::Flags::from_bits_retain(flags);
    let (flags, data) = if flags.contains(entry::at_rest::Flags::EXTENDED) {
        let (extended_flags, data) = read_u16(data)?;
        let extended_flags = entry::at_rest::FlagsExtended::from_bits(extended_flags)?;
        let extended_flags = extended_flags.to_flags()?;
        (flags.to_memory() | extended_flags, data)
    } else {
        (flags.to_memory(), data)
    };

    let start = path_backing.len();
    let data = if has_delta_paths {
        let (strip_len, data) = var_int(data)?;
        if let Some((prev_path, buf)) = prev_path_and_buf {
            let end = prev_path.end.checked_sub(strip_len.try_into().ok()?)?;
            let copy_len = end.checked_sub(prev_path.start)?;
            if copy_len > 0 {
                buf.resize(copy_len, 0);
                buf.copy_from_slice(&path_backing[prev_path.start..end]);
                path_backing.extend_from_slice(buf);
            }
        }

        let (path, data) = split_at_byte_exclusive(data, 0)?;
        path_backing.extend_from_slice(path);

        data
    } else {
        let (path, data) = if flags.contains(entry::Flags::PATH_LEN) {
            split_at_byte_exclusive(data, 0)?
        } else {
            let path_len = (flags.bits() & entry::Flags::PATH_LEN.bits()) as usize;
            let (path, data) = data.split_at_checked(path_len)?;
            (path, skip_padding(data, first_byte_of_entry)?)
        };

        // TODO(perf): for some reason, this causes tremendous `memmove` time even though the backing
        //             has enough capacity most of the time.
        path_backing.extend_from_slice(path);
        data
    };
    let path_range = start..path_backing.len();

    Some((
        Entry {
            stat: entry::Stat {
                ctime: entry::stat::Time {
                    secs: ctime_secs,
                    nsecs: ctime_nsecs,
                },
                mtime: entry::stat::Time {
                    secs: mtime_secs,
                    nsecs: mtime_nsecs,
                },
                dev,
                ino,
                uid,
                gid,
                size,
            },
            id: gix_hash::ObjectId::from_bytes_or_panic(hash),
            flags: flags & !entry::Flags::PATH_LEN,
            // This forces us to add the bits we need before being able to use them.
            mode: entry::Mode::from_bits_truncate(mode),
            path: path_range,
        },
        data,
    ))
}

#[inline]
fn skip_padding(data: &[u8], first_byte_of_entry: usize) -> Option<&[u8]> {
    let current_offset = data.as_ptr() as usize;
    let c_padding = (current_offset - first_byte_of_entry + 8) & !7;
    let skip = (first_byte_of_entry + c_padding) - current_offset;

    data.get(skip..)
}

#[inline]
fn read_u16(data: &[u8]) -> Option<(u16, &[u8])> {
    data.split_at_checked(2)
        .map(|(num, data)| (u16::from_be_bytes(num.try_into().unwrap()), data))
}
