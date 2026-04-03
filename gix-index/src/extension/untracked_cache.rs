use bstr::BString;
use gix_hash::ObjectId;

use crate::{
    entry,
    extension::{Signature, UntrackedCache},
    util::{read_u32, split_at_byte_exclusive, var_int},
};

/// A structure to track filesystem stat information along with an object id, linking a worktree file with what's in our ODB.
#[derive(Clone)]
pub struct OidStat {
    /// The file system stat information
    pub stat: entry::Stat,
    /// The id of the file in our ODB.
    pub id: ObjectId,
}

impl OidStat {
    /// Return filesystem stat information for the tracked file.
    pub fn stat(&self) -> &entry::Stat {
        &self.stat
    }

    /// Return the object id associated with the tracked file contents.
    pub fn id(&self) -> &ObjectId {
        &self.id
    }
}

/// A directory with information about its untracked files, and its sub-directories
#[derive(Clone)]
pub struct Directory {
    /// The directories name, or an empty string if this is the root directory.
    pub name: BString,
    /// Untracked files and directory names
    pub untracked_entries: Vec<BString>,
    /// indices for sub-directories similar to this one.
    pub sub_directories: Vec<usize>,

    /// The directories stat data, if available or valid // TODO: or is it the exclude file?
    pub stat: Option<entry::Stat>,
    /// The oid of a .gitignore file, if it exists
    pub exclude_file_oid: Option<ObjectId>,
    /// TODO: figure out what this really does
    pub check_only: bool,
}

impl Directory {
    /// Return the directory name, or an empty string for the root directory.
    pub fn name(&self) -> &bstr::BStr {
        self.name.as_ref()
    }

    /// Return all cached untracked entries contained directly in this directory.
    pub fn untracked_entries(&self) -> &[BString] {
        &self.untracked_entries
    }

    /// Return indices pointing at cached child directories.
    pub fn sub_directories(&self) -> &[usize] {
        &self.sub_directories
    }

    /// Return the cached stat information for this directory, if available.
    pub fn stat(&self) -> Option<&entry::Stat> {
        self.stat.as_ref()
    }

    /// Return the cached object id of this directory's ignore file, if available.
    pub fn exclude_file_oid(&self) -> Option<&ObjectId> {
        self.exclude_file_oid.as_ref()
    }

    /// Return whether this directory was cached in `check_only` mode.
    pub fn check_only(&self) -> bool {
        self.check_only
    }
}

/// Only used as an indicator
pub const SIGNATURE: Signature = *b"UNTR";

// #[allow(unused)]
/// Decode an untracked cache extension from `data`, assuming object hashes are of type `object_hash`.
pub fn decode(data: &[u8], object_hash: gix_hash::Kind) -> Option<UntrackedCache> {
    if data.last().is_none_or(|b| *b != 0) {
        return None;
    }
    let (identifier_len, data) = var_int(data)?;
    let (identifier, data) = data.split_at_checked(identifier_len.try_into().ok()?)?;

    let hash_len = object_hash.len_in_bytes();
    let (info_exclude, data) = decode_oid_stat(data, hash_len)?;
    let (excludes_file, data) = decode_oid_stat(data, hash_len)?;
    let (dir_flags, data) = read_u32(data)?;
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

    let num_directory_blocks = num_directory_blocks.try_into().ok()?;
    let directories = &mut res.directories;
    directories.reserve(num_directory_blocks);

    let data = decode_directory_block(data, directories)?;
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
        directories[index].check_only = true;
        Some(())
    })?;
    valid.for_each_set_bit(|index| {
        let (stat, rest) = crate::decode::stat(data)?;
        directories[index].stat = stat.into();
        data = rest;
        Some(())
    });
    hash_valid.for_each_set_bit(|index| {
        let (hash, rest) = data.split_at_checked(hash_len)?;
        data = rest;
        directories[index].exclude_file_oid = ObjectId::from_bytes_or_panic(hash).into();
        Some(())
    });

    // null-byte checked in the beginning
    if data.len() != 1 {
        return None;
    }
    res.into()
}

fn decode_directory_block<'a>(data: &'a [u8], directories: &mut Vec<Directory>) -> Option<&'a [u8]> {
    let (num_untracked, data) = var_int(data)?;
    let (num_dirs, data) = var_int(data)?;
    let (name, mut data) = split_at_byte_exclusive(data, 0)?;
    let mut untracked_entries = Vec::<BString>::with_capacity(num_untracked.try_into().ok()?);
    for _ in 0..num_untracked {
        let (name, rest) = split_at_byte_exclusive(data, 0)?;
        data = rest;
        untracked_entries.push(name.into());
    }

    let index = directories.len();
    directories.push(Directory {
        name: name.into(),
        untracked_entries,
        sub_directories: Vec::with_capacity(num_dirs.try_into().ok()?),
        // the following are set later through their bitmaps
        stat: None,
        exclude_file_oid: None,
        check_only: false,
    });

    for _ in 0..num_dirs {
        let subdir_index = directories.len();
        let rest = decode_directory_block(data, directories)?;
        data = rest;
        directories[index].sub_directories.push(subdir_index);
    }

    data.into()
}

fn decode_oid_stat(data: &[u8], hash_len: usize) -> Option<(OidStat, &[u8])> {
    let (stat, data) = crate::decode::stat(data)?;
    let (hash, data) = data.split_at_checked(hash_len)?;
    Some((
        OidStat {
            stat,
            id: ObjectId::from_bytes_or_panic(hash),
        },
        data,
    ))
}
