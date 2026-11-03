use bstr::{BStr, BString};
use gix_hash::ObjectId;

use crate::{
    entry,
    extension::{Signature, UntrackedCache},
    util::{read_u32, split_at_byte_exclusive, var_int},
};

impl UntrackedCache {
    /// Something identifying the location and machine that this cache is for.
    pub fn identifier(&self) -> &bstr::BStr {
        self.identifier.as_ref()
    }

    /// Stat and object id for the `.git/info/exclude` file, if available.
    pub fn info_exclude(&self) -> Option<&OidStat> {
        self.info_exclude.as_ref()
    }

    /// Stat and object id for the `core.excludesfile`, if available.
    pub fn excludes_file(&self) -> Option<&OidStat> {
        self.excludes_file.as_ref()
    }

    /// Usually `.gitignore`.
    pub fn exclude_filename_per_dir(&self) -> &bstr::BStr {
        self.exclude_filename_per_dir.as_ref()
    }

    /// The directory flags Git used while populating the cache.
    pub fn dir_flags(&self) -> u32 {
        self.dir_flags
    }

    /// A list of directories and sub-directories, with `directories[0]` being the root.
    pub fn directories(&self) -> &[Directory] {
        &self.directories
    }

    /// Invalidate all cached directory listings while retaining their structure and exclusion metadata.
    pub fn invalidate_all(&mut self) {
        for directory in &mut self.directories {
            directory.invalidate();
        }
    }

    /// Invalidate the listing containing the repository-relative `path`, its ancestors, and any cached subtree at `path`.
    ///
    /// A trailing slash is optional. Invalid relative paths conservatively invalidate the entire cache.
    /// Changes to inherited ignore rules require invalidating every affected descendant as well.
    pub fn invalidate_path(&mut self, path: &BStr) {
        let path = path.strip_suffix(b"/").unwrap_or(path);
        if path.is_empty()
            || path
                .split(|byte| *byte == b'/')
                .any(|part| part.is_empty() || part == b"." || part == b"..")
        {
            self.invalidate_all();
            return;
        }
        let mut directory_index = 0;
        for component in path.split(|byte| *byte == b'/') {
            let Some(directory) = self.directories.get_mut(directory_index) else {
                return;
            };
            directory.invalidate();
            let Ok(next) = self.directories[directory_index]
                .sub_directories
                .binary_search_by(|&index| self.directories[index].name.as_slice().cmp(component))
            else {
                return;
            };
            directory_index = self.directories[directory_index].sub_directories[next];
        }
        let mut pending = vec![directory_index];
        while let Some(index) = pending.pop() {
            if let Some(directory) = self.directories.get_mut(index) {
                pending.extend_from_slice(&directory.sub_directories);
                directory.invalidate();
            }
        }
    }

    /// Serialize this extension, including its header, using `object_hash` for every stored object id.
    ///
    /// Only restore a cache to an index after invalidating listings affected by index or monitor changes.
    pub fn write_to(&self, mut out: impl std::io::Write, object_hash: gix_hash::Kind) -> std::io::Result<()> {
        let mut body = Vec::new();
        write_varint(&mut body, self.identifier.len());
        body.extend_from_slice(&self.identifier);
        write_stat(&mut body, self.info_exclude.as_ref().map(|value| value.stat));
        write_stat(&mut body, self.excludes_file.as_ref().map(|value| value.stat));
        body.extend_from_slice(&self.dir_flags.to_be_bytes());
        write_oid(&mut body, self.info_exclude.as_ref().map(|value| value.id), object_hash)?;
        write_oid(
            &mut body,
            self.excludes_file.as_ref().map(|value| value.id),
            object_hash,
        )?;
        body.extend_from_slice(&self.exclude_filename_per_dir);
        body.push(0);
        write_varint(&mut body, self.directories.len());
        if !self.directories.is_empty() {
            // Decoding stores directories in depth-first order; invalidation preserves that order.
            for directory in &self.directories {
                write_varint(&mut body, directory.untracked_entries.len());
                write_varint(&mut body, directory.sub_directories.len());
                body.extend_from_slice(&directory.name);
                body.push(0);
                for name in &directory.untracked_entries {
                    body.extend_from_slice(name);
                    body.push(0);
                }
            }
            for predicate in [
                |directory: &Directory| directory.stat.is_some(),
                |directory: &Directory| directory.check_only,
                |directory: &Directory| directory.exclude_file_oid.is_some(),
            ] {
                let bits: Vec<_> = self.directories.iter().map(predicate).collect();
                gix_bitmap::ewah::Vec::from_bits(&bits)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "too many cached directories")
                    })?
                    .write_to(&mut body)?;
            }
            for directory in &self.directories {
                if let Some(stat) = directory.stat {
                    write_stat(&mut body, Some(stat));
                }
            }
            for directory in &self.directories {
                if let Some(id) = directory.exclude_file_oid {
                    write_oid(&mut body, Some(id), object_hash)?;
                }
            }
            body.push(0);
        }
        let size = u32::try_from(body.len()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "untracked cache extension exceeds u32::MAX",
            )
        })?;
        out.write_all(&SIGNATURE)?;
        out.write_all(&size.to_be_bytes())?;
        out.write_all(&body)
    }
}

fn write_varint(out: &mut Vec<u8>, mut value: usize) {
    let mut bytes = [0; 10];
    let mut start = bytes.len() - 1;
    bytes[start] = (value & 0x7f) as u8;
    while value >= 0x80 {
        value = (value >> 7) - 1;
        start -= 1;
        bytes[start] = (value & 0x7f) as u8 | 0x80;
    }
    out.extend_from_slice(&bytes[start..]);
}

fn write_stat(out: &mut Vec<u8>, stat: Option<entry::Stat>) {
    let stat = stat.unwrap_or_default();
    for value in [
        stat.ctime.secs,
        stat.ctime.nsecs,
        stat.mtime.secs,
        stat.mtime.nsecs,
        stat.dev,
        stat.ino,
        stat.uid,
        stat.gid,
        stat.size,
    ] {
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn write_oid(out: &mut Vec<u8>, id: Option<ObjectId>, object_hash: gix_hash::Kind) -> std::io::Result<()> {
    let id = id.unwrap_or_else(|| object_hash.null());
    if id.kind() != object_hash {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "untracked cache object hash mismatch",
        ));
    }
    out.extend_from_slice(id.as_bytes());
    Ok(())
}

/// A structure to track filesystem stat information along with an object id, linking a worktree file with what's in our ODB.
#[derive(Clone, Debug)]
pub struct OidStat {
    /// The file system stat information
    pub stat: entry::Stat,
    /// The id of the file in our ODB.
    pub id: ObjectId,
}

impl OidStat {
    /// The file system stat information.
    pub fn stat(&self) -> &entry::Stat {
        &self.stat
    }

    /// The id of the file in our ODB.
    pub fn id(&self) -> ObjectId {
        self.id
    }
}

/// A directory with information about its untracked files, and its sub-directories
#[derive(Clone, Debug)]
pub struct Directory {
    /// The directories name, or an empty string if this is the root directory.
    pub name: BString,
    /// Untracked files and directory names
    pub untracked_entries: Vec<BString>,
    /// Indices for sub-directories, ordered strictly by their names.
    pub sub_directories: Vec<usize>,

    /// The directories stat data, if available or valid // TODO: or is it the exclude file?
    pub stat: Option<entry::Stat>,
    /// The oid of a .gitignore file, if it exists
    pub exclude_file_oid: Option<ObjectId>,
    /// TODO: figure out what this really does
    pub check_only: bool,
}

impl Directory {
    fn invalidate(&mut self) {
        self.stat = None;
        self.untracked_entries.clear();
    }

    /// The directory name, or an empty string if this is the root directory.
    /// `/` is always used as path-separator.
    pub fn name(&self) -> &bstr::BStr {
        self.name.as_ref()
    }

    /// Untracked files and directory names.
    pub fn untracked_entries(&self) -> &[BString] {
        &self.untracked_entries
    }

    /// Indices for sub-directories, ordered strictly by their names.
    pub fn sub_directories(&self) -> &[usize] {
        &self.sub_directories
    }

    /// The directory stat data, if available and valid.
    pub fn stat(&self) -> Option<&entry::Stat> {
        self.stat.as_ref()
    }

    /// The oid of a `.gitignore` file, if it exists.
    pub fn exclude_file_oid(&self) -> Option<ObjectId> {
        self.exclude_file_oid
    }

    /// Whether Git marked this directory as check-only.
    pub fn check_only(&self) -> bool {
        self.check_only
    }
}

/// Only used as an indicator
pub const SIGNATURE: Signature = *b"UNTR";

/// Decode an untracked cache extension from `data`, assuming object hashes are of type `object_hash`.
pub fn decode(data: &[u8], object_hash: gix_hash::Kind, alloc_limit_bytes: Option<usize>) -> Option<UntrackedCache> {
    if data.last().is_none_or(|b| *b != 0) {
        return None;
    }
    let (identifier_len, data) = var_int(data)?;
    let (identifier, data) = data.split_at_checked(identifier_len.try_into().ok()?)?;

    // The on-disk layout matches git's `ondisk_untracked_cache` struct
    // https://github.com/git/git/blob/2855562ca6a9c6b0e7bc780b050c1e83c9fcfbd0/dir.c#L3582-L3586
    // https://github.com/git/git/blob/2855562ca6a9c6b0e7bc780b050c1e83c9fcfbd0/dir.c#L3668-L3722
    //   info_exclude_stat  (36 bytes)
    //   excludes_file_stat (36 bytes)
    //   dir_flags          ( 4 bytes)
    //   info_exclude hash  (hash_len bytes)
    //   excludes_file hash (hash_len bytes)
    //   exclude_per_dir    (NUL-terminated)
    let hash_len = object_hash.len_in_bytes();
    let (info_exclude_stat, data) = crate::decode::stat(data)?;
    let (excludes_file_stat, data) = crate::decode::stat(data)?;
    let (dir_flags, data) = read_u32(data)?;
    let (info_exclude_hash, data) = data.split_at_checked(hash_len)?;
    let (excludes_file_hash, data) = data.split_at_checked(hash_len)?;
    let info_exclude = OidStat {
        stat: info_exclude_stat,
        id: ObjectId::from_bytes_or_panic(info_exclude_hash),
    };
    let excludes_file = OidStat {
        stat: excludes_file_stat,
        id: ObjectId::from_bytes_or_panic(excludes_file_hash),
    };
    let (exclude_filename_per_dir, data) = split_at_byte_exclusive(data, 0)?;

    let (num_directory_blocks, data) = var_int(data)?;

    let mut res = UntrackedCache {
        identifier: identifier.into(),
        info_exclude: (!info_exclude.id.is_null()).then_some(info_exclude),
        excludes_file: (!excludes_file.id.is_null()).then_some(excludes_file),
        exclude_filename_per_dir: exclude_filename_per_dir.into(),
        dir_flags,
        directories: Vec::new(),
    };
    if num_directory_blocks == 0 {
        return data.is_empty().then_some(res);
    }

    let num_directory_blocks: usize = num_directory_blocks.try_into().ok()?;
    if num_directory_blocks > data.len() {
        return None;
    }
    if alloc_limit_bytes
        .is_some_and(|limit| num_directory_blocks.saturating_mul(std::mem::size_of::<Directory>()) > limit)
    {
        return None;
    }
    let directories = &mut res.directories;
    directories.try_reserve(num_directory_blocks).ok()?;

    let data = decode_directory_block(data, directories, alloc_limit_bytes)?;
    if directories.len() != num_directory_blocks {
        return None;
    }
    let (valid, data) = gix_bitmap::ewah::decode(data).ok()?;
    let (check_only, data) = gix_bitmap::ewah::decode(data).ok()?;
    let (hash_valid, mut data) = gix_bitmap::ewah::decode(data).ok()?;

    if valid.num_bits() > num_directory_blocks
        || check_only.num_bits() > num_directory_blocks
        || hash_valid.num_bits() > num_directory_blocks
    {
        return None;
    }

    check_only.for_each_set_bit(|index| {
        let directory = directories.get_mut(index)?;
        directory.check_only = true;
        Some(())
    })?;
    valid.for_each_set_bit(|index| {
        let directory = directories.get_mut(index)?;
        let (stat, rest) = crate::decode::stat(data)?;
        directory.stat = stat.into();
        data = rest;
        Some(())
    })?;
    hash_valid.for_each_set_bit(|index| {
        let directory = directories.get_mut(index)?;
        let (hash, rest) = data.split_at_checked(hash_len)?;
        data = rest;
        directory.exclude_file_oid = ObjectId::from_bytes_or_panic(hash).into();
        Some(())
    })?;

    // null-byte checked in the beginning
    if data.len() != 1 {
        return None;
    }
    res.into()
}

fn decode_directory_block<'a>(
    data: &'a [u8],
    directories: &mut Vec<Directory>,
    alloc_limit_bytes: Option<usize>,
) -> Option<&'a [u8]> {
    let (num_untracked, data) = var_int(data)?;
    let (num_dirs, data) = var_int(data)?;
    let (name, mut data) = split_at_byte_exclusive(data, 0)?;
    // Untracked names are encoded as `name\0name\0...`, and we assume names are non-empty:
    // `a\0b\0` is 4 bytes for 2 entries, so each entry needs at least 2 bytes.
    let max_entries_from_remaining_data = data.len() / 2;
    let num_untracked: usize = num_untracked.try_into().ok()?;
    let num_dirs: usize = num_dirs.try_into().ok()?;
    if num_untracked > max_entries_from_remaining_data || num_dirs > max_entries_from_remaining_data {
        return None;
    }
    if alloc_limit_bytes.is_some_and(|limit| {
        num_untracked.saturating_mul(std::mem::size_of::<BString>()) > limit
            || num_dirs.saturating_mul(std::mem::size_of::<usize>()) > limit
    }) {
        return None;
    }
    let mut untracked_entries = Vec::<BString>::new();
    untracked_entries.try_reserve(num_untracked).ok()?;
    for _ in 0..num_untracked {
        let (name, rest) = split_at_byte_exclusive(data, 0)?;
        data = rest;
        untracked_entries.push(name.into());
    }

    let index = directories.len();
    directories.push(Directory {
        name: name.into(),
        untracked_entries,
        sub_directories: {
            let mut sub_directories = Vec::new();
            sub_directories.try_reserve(num_dirs).ok()?;
            sub_directories
        },
        // the following are set later through their bitmaps
        stat: None,
        exclude_file_oid: None,
        check_only: false,
    });

    for _ in 0..num_dirs {
        let subdir_index = directories.len();
        let rest = decode_directory_block(data, directories, alloc_limit_bytes)?;
        data = rest;
        if directories[index]
            .sub_directories
            .last()
            .is_some_and(|&previous| directories[previous].name >= directories[subdir_index].name)
        {
            return None;
        }
        directories[index].sub_directories.push(subdir_index);
    }

    data.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn variable_width_counts_roundtrip() {
        for value in [0, 127, 128, 16_383, 16_384, usize::MAX] {
            let mut encoded = Vec::new();
            super::write_varint(&mut encoded, value);
            assert_eq!(
                crate::util::var_int(&encoded),
                Some((value as u64, [].as_slice())),
                "counts retain every bit across variable-width boundaries"
            );
        }
    }
}
