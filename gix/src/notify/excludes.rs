use std::path::Path;

use gix_error::{Result, ResultExt, message};

use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
};

pub(super) struct FileFilter {
    rules: gix_worktree::Stack,
    blobs: gix_odb::memory::Storage,
    /// Sorted Git paths, with sparse directories stored without a trailing slash.
    tracked: Vec<BString>,
    ignore_case: bool,
}

impl FileFilter {
    pub(super) fn new(repo: &Repository, root: &Path, index: &gix_index::State, ignore_case: bool) -> Result<Self> {
        let rules = repo.exclude_stack(
            root,
            index,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )?;
        let mut blobs = gix_odb::memory::Storage::default();
        for (_, id) in rules.id_mappings() {
            if let gix_hashtable::hash_map::Entry::Vacant(entry) = blobs.entry(*id) {
                entry.insert((gix_object::Kind::Blob, repo.find_blob(*id)?.take_data()));
            }
        }
        let mut tracked: Vec<_> = index
            .entries()
            .iter()
            .map(|entry| {
                let mut path = entry.path(index).to_owned();
                if ignore_case {
                    path.make_ascii_lowercase();
                }
                if entry.mode.is_sparse() && path.ends_with(b"/") {
                    path.pop();
                }
                path
            })
            .collect();
        if ignore_case || index.is_sparse() {
            tracked.sort_unstable();
        }
        tracked.dedup();
        Ok(Self {
            rules,
            blobs,
            tracked,
            ignore_case,
        })
    }

    pub(super) fn same_tracked_paths(&self, other: &Self) -> bool {
        self.tracked == other.tracked
    }

    pub(super) fn is_ignored(&mut self, path: &BStr) -> Result<bool> {
        if !self
            .rules
            .at_entry(path, Some(gix_index::entry::Mode::FILE), &self.blobs)
            .or_raise(|| message!("could not check ignore rules for {path}"))?
            .is_excluded()
        {
            return Ok(false);
        }
        let mut path = path.to_owned();
        if self.ignore_case {
            path.make_ascii_lowercase();
        }
        // An ancestor can be a sparse directory, or a tracked file recently replaced by a directory.
        let tracked = |name: &[u8]| {
            self.tracked
                .binary_search_by(|entry| entry.as_slice().cmp(name))
                .is_ok()
        };
        if tracked(path.as_slice()) || path.find_iter(b"/").any(|slash| tracked(&path[..slash])) {
            return Ok(false);
        }
        // A file replacing a tracked directory can invalidate all of its tracked descendants.
        path.push(b'/');
        let pos = self.tracked.partition_point(|entry| entry < &path);
        Ok(!self
            .tracked
            .get(pos)
            .is_some_and(|entry| entry.starts_with(path.as_slice())))
    }
}
