use std::{
    ffi::OsString,
    fs::FileType,
    path::Path,
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use bstr::{BStr, ByteSlice};

pub(super) type Listing = Vec<(OsString, Option<FileType>)>;

pub(super) struct Prefetch<'index> {
    directories: Vec<&'index BStr>,
    listings: Vec<OnceLock<Option<Listing>>>,
    next: AtomicUsize,
    done: AtomicBool,
}

impl<'index> Prefetch<'index> {
    pub(super) fn new(index: &'index gix_index::State, root: &BStr) -> Option<Self> {
        let path_backing = index.path_backing();
        let mut directories = Vec::new();
        for entry in index.entries() {
            if entry.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE) {
                continue;
            }
            let path = entry.path_in(path_backing);
            let Some(pos) = path.rfind_byte(b'/') else {
                continue;
            };
            let mut dir = path[..pos].as_bstr();
            if directories.last() == Some(&dir) {
                continue;
            }
            while is_within(dir, root) {
                directories.push(dir);
                match dir.rfind_byte(b'/') {
                    Some(pos) => dir = dir[..pos].as_bstr(),
                    None => break,
                }
            }
        }
        if directories.is_empty() {
            return None;
        }
        directories.sort_unstable();
        directories.dedup();
        Some(Prefetch {
            listings: directories.iter().map(|_| OnceLock::new()).collect(),
            directories,
            next: AtomicUsize::new(0),
            done: AtomicBool::new(false),
        })
    }

    pub(super) fn get(&self, rela_path: &BStr, path: &Path, precompose_unicode: bool) -> Option<&Listing> {
        let idx = self.directories.binary_search(&rela_path).ok()?;
        self.listings[idx]
            .get_or_init(|| read(path, precompose_unicode))
            .as_ref()
    }

    pub(super) fn run(&self, worktree_root: &Path, precompose_unicode: bool, should_interrupt: Option<&AtomicBool>) {
        while !self.done.load(Ordering::Relaxed) && !should_interrupt.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            let idx = self.next.fetch_add(1, Ordering::Relaxed);
            let (Some(rela_path), Some(listing)) = (self.directories.get(idx), self.listings.get(idx)) else {
                break;
            };
            if listing.get().is_some() {
                continue;
            }
            let Ok(rela_path) = gix_path::from_bstr(*rela_path) else {
                continue;
            };
            let path = worktree_root.join(rela_path);
            listing.get_or_init(|| read(&path, precompose_unicode));
        }
    }

    pub(super) fn finish(&self) {
        self.done.store(true, Ordering::Relaxed);
    }
}

fn is_within(dir: &BStr, root: &BStr) -> bool {
    root.is_empty() || (dir.starts_with(root) && (dir.len() == root.len() || dir[root.len()] == b'/'))
}

fn read(path: &Path, precompose_unicode: bool) -> Option<Listing> {
    gix_fs::read_dir(path, precompose_unicode)
        .ok()?
        .map(|entry| entry.map(|entry| (entry.file_name().into_owned(), entry.file_type().ok())))
        .collect::<std::io::Result<_>>()
        .ok()
}
