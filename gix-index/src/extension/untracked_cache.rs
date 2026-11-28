use bstr::BString;
use gix_hash::ObjectId;

use crate::{
    entry,
    extension::{Signature, UntrackedCache},
    util::{read_u32, split_at_byte_exclusive, var_int},
};

impl UntrackedCache {
    pub(crate) fn invalidate_path(&mut self, path: &bstr::BStr) {
        if self.directories.is_empty() {
            return;
        }
        // Git's DIR_SHOW_OTHER_DIRECTORIES allows changed leaves to affect collapsed ancestor listings.
        let invalidate_ancestors = self.dir_flags & (1 << 1) != 0;
        let mut directory = 0;
        let mut components = path.split(|byte| *byte == b'/').peekable();
        while let Some(component) = components.next() {
            let last = components.peek().is_none();
            if invalidate_ancestors || last {
                self.invalidate_directory(directory, last && component == self.exclude_filename_per_dir.as_slice());
            }
            let child = self.directories[directory]
                .sub_directories
                .iter()
                .copied()
                .find(|child| self.directories[*child].name.as_slice() == component);
            match child {
                Some(child) if !last => directory = child,
                Some(child) => {
                    self.invalidate_directory(child, true);
                    break;
                }
                None => {
                    self.invalidate_directory(directory, false);
                    break;
                }
            }
        }
    }

    fn invalidate_directory(&mut self, index: usize, recursively: bool) {
        let directory = &mut self.directories[index];
        directory.stat = None;
        directory.untracked_entries.clear();
        directory.check_only = false;
        if recursively {
            for position in 0..self.directories[index].sub_directories.len() {
                let child = self.directories[index].sub_directories[position];
                self.invalidate_directory(child, true);
            }
        }
    }

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

    /// Serialize the `UNTR` extension to `out`, using `object_hash` for absent ignore-file IDs.
    /// If `invalidate` is set, omit cached directory stats and listings without changing this instance.
    pub(crate) fn write_to(
        &self,
        mut out: impl std::io::Write,
        object_hash: gix_hash::Kind,
        invalidate: bool,
    ) -> std::io::Result<()> {
        use gix_error::{ResultExt, corruption};

        let mut data = Vec::new();
        write_varint(&mut data, self.identifier.len());
        data.extend_from_slice(&self.identifier);
        for stat in [&self.info_exclude, &self.excludes_file] {
            write_stat(
                &mut data,
                stat.as_ref().map_or(entry::Stat::default(), |stat| stat.stat),
            );
        }
        data.extend_from_slice(&self.dir_flags.to_be_bytes());
        for stat in [&self.info_exclude, &self.excludes_file] {
            data.extend_from_slice(stat.as_ref().map_or(object_hash.null(), |stat| stat.id).as_bytes());
        }
        data.extend_from_slice(&self.exclude_filename_per_dir);
        data.push(0);
        write_varint(&mut data, self.directories.len());
        for directory in &self.directories {
            let untracked = if !invalidate && directory.stat.is_some() {
                directory.untracked_entries.as_slice()
            } else {
                &[]
            };
            write_varint(&mut data, untracked.len());
            write_varint(&mut data, directory.sub_directories.len());
            data.extend_from_slice(&directory.name);
            data.push(0);
            for name in untracked {
                data.extend_from_slice(name);
                data.push(0);
            }
        }
        if !self.directories.is_empty() {
            let predicates: [&dyn Fn(&Directory) -> bool; 3] = [
                &|directory| !invalidate && directory.stat.is_some(),
                &|directory| !invalidate && directory.stat.is_some() && directory.check_only,
                &|directory| directory.exclude_file_oid.is_some(),
            ];
            for is_set in predicates {
                let bits: Vec<_> = self.directories.iter().map(is_set).collect();
                gix_bitmap::ewah::Vec::from_bits(&bits)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "UNTR has too many directories")
                    })?
                    .write_to(&mut data)?;
            }
            for directory in &self.directories {
                if !invalidate && let Some(stat) = directory.stat {
                    write_stat(&mut data, stat);
                }
            }
            for directory in &self.directories {
                if let Some(id) = directory.exclude_file_oid {
                    data.extend_from_slice(id.as_bytes());
                }
            }
            data.push(0);
        }
        out.write_all(&SIGNATURE)?;
        out.write_all(
            &u32::try_from(data.len())
                .or_raise(|| corruption("untracked-cache extension exceeds 4GB"))
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?
                .to_be_bytes(),
        )?;
        out.write_all(&data)
    }
}

fn write_varint(out: &mut Vec<u8>, mut value: usize) {
    let mut buffer = [0u8; 10];
    let mut position = buffer.len() - 1;
    buffer[position] = (value & 0x7f) as u8;
    while value > 0x7f {
        value = (value >> 7) - 1;
        position -= 1;
        buffer[position] = 0x80 | (value & 0x7f) as u8;
    }
    out.extend_from_slice(&buffer[position..]);
}

fn write_stat(out: &mut Vec<u8>, stat: entry::Stat) {
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
    /// The directory name, or an empty string if this is the root directory.
    /// `/` is always used as path-separator.
    pub fn name(&self) -> &bstr::BStr {
        self.name.as_ref()
    }

    /// Untracked files and directory names.
    pub fn untracked_entries(&self) -> &[BString] {
        &self.untracked_entries
    }

    /// Indices for sub-directories similar to this one.
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
pub fn decode(data: &[u8], object_hash: gix_hash::Kind) -> Option<UntrackedCache> {
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
    let directories = &mut res.directories;
    directories.try_reserve(num_directory_blocks).ok()?;

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

fn decode_directory_block<'a>(data: &'a [u8], directories: &mut Vec<Directory>) -> Option<&'a [u8]> {
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
        let rest = decode_directory_block(data, directories)?;
        data = rest;
        directories[index].sub_directories.push(subdir_index);
    }

    data.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn directory_invalidation_respects_collapsed_listings_and_unknown_children() -> gix_error::TestResult {
        let directory = |name: &str, sub_directories| super::Directory {
            name: name.into(),
            untracked_entries: vec!["untracked".into()],
            sub_directories,
            stat: Some(Default::default()),
            exclude_file_oid: None,
            check_only: true,
        };
        for dir_flags in [0, 6] {
            let mut cache = super::UntrackedCache {
                identifier: Default::default(),
                info_exclude: None,
                excludes_file: None,
                exclude_filename_per_dir: ".gitignore".into(),
                dir_flags,
                directories: vec![
                    directory("", vec![1, 2]),
                    directory("affected", Vec::new()),
                    directory("sibling", Vec::new()),
                ],
            };
            cache.invalidate_path("affected/file".into());
            assert_eq!(
                cache.directories[0].stat.is_some(),
                dir_flags == 0,
                "only collapsed listings require invalidating known ancestors"
            );
            assert!(
                cache.directories[1].stat.is_none()
                    && cache.directories[1].untracked_entries.is_empty()
                    && !cache.directories[1].check_only,
                "the changed directory is invalidated"
            );
            assert!(
                cache.directories[2].stat.is_some()
                    && !cache.directories[2].untracked_entries.is_empty()
                    && cache.directories[2].check_only,
                "unaffected sibling caches are retained"
            );
            cache.invalidate_path("new/file".into());
            assert!(
                cache.directories[0].stat.is_none(),
                "an unknown directory invalidates its deepest cached ancestor"
            );
            assert!(
                cache.directories[2].stat.is_some(),
                "unknown children do not discard unaffected siblings"
            );
        }
        Ok(())
    }

    #[test]
    fn varints_cover_single_byte_and_multi_byte_boundaries() -> gix_error::TestResult {
        for (value, expected) in [
            (0, &[0][..]),
            (127, &[127][..]),
            (128, &[0x80, 0][..]),
            (255, &[0x80, 127][..]),
            (256, &[0x81, 0][..]),
        ] {
            let mut data = Vec::new();
            super::write_varint(&mut data, value);
            assert_eq!(data, expected, "UNTR uses Git's offset-style varint encoding");
        }
        for value in [16383, 16384, u32::MAX as usize, usize::MAX] {
            let mut data = Vec::new();
            super::write_varint(&mut data, value);
            let (decoded, rest) = crate::util::var_int(&data).expect("the encoded value fits in a u64");
            assert_eq!(decoded, value as u64, "larger counts retain every bit");
            assert!(rest.is_empty(), "the entire encoded count is consumed");
        }
        Ok(())
    }
}
