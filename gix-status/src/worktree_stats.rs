//! Windows-only worktree metadata preprocessing — see the gate on
//! `pub mod worktree_stats` in [`crate`] for why this is Windows-only.
//!
//! [`prepare`] runs a single parallel `GetFileInformationByHandleEx` walk
//! of the worktree (~30 ms / 90 k files) and returns a [`WorktreeStats`]
//! map keyed by worktree-relative path. `index_as_worktree` then looks up
//! each index entry there instead of issuing a per-file `lstat` (~1 s for
//! the same tree). The map is **not a long-lived cache**: it is built once
//! per status call and discarded with the iterator. Lookups are
//! transparent — empty, partial, or extra entries change speed only, never
//! correctness, since misses fall through to a live syscall.

use std::path::Path;

use bstr::BString;

/// Pre-computed file metadata produced by [`prepare`] for one worktree entry.
///
/// Carries enough information to determine file type, detect mode changes,
/// build a [`gix_index::entry::Stat`] for comparison, and short-circuit content
/// reads via file size.
///
/// Windows-only fields: this module is `#[cfg(windows)]`, and Windows batch
/// directory enumeration doesn't expose `dev`/`ino`/`uid`/`gid` or the
/// executable bit. The status pipeline's stat comparison on Windows compares
/// those `Stat` fields against matching zeros from
/// [`gix_index::entry::Stat::from_fs`]'s Windows branch, and git on Windows
/// defaults to `core.filemode=false`, so all five are simply omitted here.
#[derive(Debug, Clone, Default)]
pub struct WorktreeStat {
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether this is a symlink (or reparse point on Windows).
    pub is_symlink: bool,
    /// File size in bytes.
    pub size: u64,
    /// Modification time — seconds since Unix epoch.
    pub mtime_secs: u32,
    /// Modification time — nanoseconds component.
    pub mtime_nsecs: u32,
    /// Status/creation time — seconds since Unix epoch.
    ///
    /// On Windows this must be populated from the real `CreationTime`, not `mtime`:
    /// the stat comparison in the status pipeline compares `ctime.secs` by default
    /// (`trust_ctime=true`), and faking `ctime=mtime` causes spurious mismatches
    /// for any file whose creation-time and modification-time differ.
    pub ctime_secs: u32,
    /// Status/creation time — nanoseconds component.
    pub ctime_nsecs: u32,
}

impl WorktreeStat {
    /// Convert to gitoxide's [`Stat`](gix_index::entry::Stat) struct for index comparison.
    ///
    /// Truncates `size` from 64 to 32 bits — matching what
    /// [`gix_index::entry::stat::Stat::from_fs`] does on Unix, so both code
    /// paths compare the same quantities. `dev`/`ino`/`uid`/`gid` are zeroed
    /// here to match what `from_fs` produces on Windows.
    pub fn to_stat(&self) -> gix_index::entry::Stat {
        gix_index::entry::Stat {
            mtime: gix_index::entry::stat::Time {
                secs: self.mtime_secs,
                nsecs: self.mtime_nsecs,
            },
            ctime: gix_index::entry::stat::Time {
                secs: self.ctime_secs,
                nsecs: self.ctime_nsecs,
            },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: self.size as u32,
        }
    }
}

/// Map of worktree-relative paths (forward-slashed, in the exact case as
/// enumerated from disk) to their pre-computed [`WorktreeStat`].
///
/// Lookups are case-sensitive: callers must query with the same case the walker
/// emitted. On a case-insensitive worktree where the index path's case differs
/// from disk, the lookup misses and `index_as_worktree` falls back to a live
/// `lstat` — a few extra syscalls in a rare scenario. Folding cases together
/// would silently merge distinct files on case-sensitive volumes (Windows
/// per-directory case-sensitivity, NTFS POSIX mode), which would let the map
/// return one file's stat for a query about another and silently misreport
/// tracked-file status. That's strictly worse than a few cache misses.
pub type WorktreeStats = hashbrown::HashMap<BString, WorktreeStat>;

/// Either a live `lstat` result or a precomputed [`WorktreeStat`] from
/// [`prepare`]. Lets [`crate::index_as_worktree`] treat both shapes uniformly
/// without branching at every per-entry use site.
pub(crate) enum FileMetadata<'a> {
    Live(gix_index::fs::Metadata),
    Cached(&'a WorktreeStat),
}

impl FileMetadata<'_> {
    pub(crate) fn is_dir(&self) -> bool {
        match self {
            Self::Live(m) => m.is_dir(),
            Self::Cached(c) => c.is_dir,
        }
    }

    pub(crate) fn is_symlink(&self) -> bool {
        match self {
            Self::Live(m) => m.is_symlink(),
            Self::Cached(c) => c.is_symlink,
        }
    }

    pub(crate) fn len(&self) -> u64 {
        match self {
            Self::Live(m) => m.len(),
            Self::Cached(c) => c.size,
        }
    }

    pub(crate) fn to_stat(&self) -> Result<gix_index::entry::Stat, std::time::SystemTimeError> {
        match self {
            Self::Live(m) => gix_index::entry::Stat::from_fs(m),
            Self::Cached(c) => Ok(c.to_stat()),
        }
    }

    pub(crate) fn mode_change(
        &self,
        entry_mode: gix_index::entry::Mode,
        has_symlinks: bool,
        executable_bit: bool,
    ) -> Option<gix_index::entry::mode::Change> {
        match self {
            Self::Live(m) => entry_mode.change_to_match_fs(m, has_symlinks, executable_bit),
            // Windows batch enumeration doesn't expose the executable bit; pass `false`.
            // Git on Windows defaults to `core.filemode=false` so this is unused anyway.
            Self::Cached(c) => entry_mode.change_to_match_fs_with_values(
                !c.is_dir && !c.is_symlink, // is_file: regular file (not dir, not symlink)
                c.is_dir,
                c.is_symlink,
                false,
                has_symlinks,
                executable_bit,
            ),
        }
    }
}

/// Prepare a [`WorktreeStats`] map by walking the worktree in parallel using
/// `GetFileInformationByHandleEx` with `FileIdBothDirectoryInfo`, skipping
/// subtrees flagged by the per-thread predicate produced by `make_excludes`.
///
/// The returned map can be attached to the status pipeline via
/// [`Context::worktree_stats`](crate::index_as_worktree::Context::worktree_stats)
/// — hits skip per-file syscalls.
///
/// `thread_limit` caps parallelism. `None` uses all available cores; `Some(1)`
/// is single-threaded.
///
/// `make_excludes` is called once on each worker thread and returns a predicate
/// that owns thread-local state (e.g. a `gix_worktree::Stack`). Each time the
/// walker is about to descend into a subdirectory, it calls the predicate with
/// the worktree-relative path; returning `true` skips that subtree. Callers
/// that don't need gitignore pruning can pass `|| |_: &bstr::BStr| false`, but
/// for typical projects with fat ignored dirs (`node_modules`, `target`) the
/// wasted enumeration makes the preprocessing pass net-slower than plain
/// per-file stats.
pub fn prepare<F, E>(worktree: &Path, thread_limit: Option<usize>, make_excludes: F) -> std::io::Result<WorktreeStats>
where
    F: Fn() -> E + Sync,
    E: FnMut(&bstr::BStr) -> bool,
{
    windows::walk_worktree_parallel(worktree, thread_limit, make_excludes)
}

/// Windows-specific implementation using `GetFileInformationByHandleEx` /
/// `FileIdBothDirectoryInfo`. Work-stealing across threads via `thread::scope`.
#[allow(unsafe_code)]
mod windows {
    use super::*;
    use std::collections::VecDeque;
    use std::ffi::{OsString, c_void};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::sync::{Condvar, Mutex};
    use std::thread;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FileIdBothDirectoryInfo, GetFileInformationByHandleEx, OPEN_EXISTING, SYNCHRONIZE,
    };

    /// 64 KiB, u64-aligned — `FILE_ID_BOTH_DIR_INFO` contains LARGE_INTEGER fields that
    /// require 8-byte alignment, and `Vec<u64>` guarantees it. Hoisted to the worker so
    /// 6k+ directory walks reuse one allocation instead of allocating per call.
    const BUFFER_U64S: usize = 8 * 1024;

    /// Work item for the parallel walker: (null-terminated UTF-16 absolute path, relative prefix).
    ///
    /// The path is stored pre-encoded so `CreateFileW` on the child can reuse the parent's
    /// allocation without re-traversing `PathBuf`/`OsStr` each time.
    type WorkItem = (Vec<u16>, String);

    /// Convert FILE_ID_BOTH_DIR_INFO to a [`WorktreeStat`].
    fn stat_from_info(info: &FILE_ID_BOTH_DIR_INFO) -> WorktreeStat {
        let size = info.EndOfFile as u64;

        // FILETIME values are LARGE_INTEGER holding 100ns intervals since 1601-01-01 UTC.
        // `ctime` must come from `CreationTime` (not mtime): `gix_index::entry::stat::from_fs`
        // on Windows populates ctime from `Metadata::created()`, which is CreationTime. If we
        // faked ctime=mtime here, stat comparison would spuriously fail for any file where
        // creation-time and modification-time differ, forcing an unnecessary content hash.
        let (mtime_secs, mtime_nsecs) = filetime_to_unix(info.LastWriteTime as u64);
        let (ctime_secs, ctime_nsecs) = filetime_to_unix(info.CreationTime as u64);

        let is_dir = (info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
        let is_symlink = (info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

        WorktreeStat {
            is_dir,
            is_symlink,
            size,
            mtime_secs,
            mtime_nsecs,
            ctime_secs,
            ctime_nsecs,
        }
    }

    /// Convert a Windows FILETIME (100ns intervals since 1601-01-01 UTC) to Unix (secs, nsecs).
    fn filetime_to_unix(ft: u64) -> (u32, u32) {
        const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
        let unix_100ns = ft.saturating_sub(EPOCH_DIFF);
        let secs = (unix_100ns / 10_000_000) as u32;
        let nsecs = ((unix_100ns % 10_000_000) * 100) as u32;
        (secs, nsecs)
    }

    /// Build a null-terminated UTF-16 absolute path for `parent\name`.
    fn join_utf16(parent: &[u16], name: &[u16]) -> Vec<u16> {
        // Parent is null-terminated; drop the trailing NUL before joining.
        let parent = parent.strip_suffix(&[0u16]).unwrap_or(parent);
        let mut out = Vec::with_capacity(parent.len() + 1 + name.len() + 1);
        out.extend_from_slice(parent);
        if out.last().copied() != Some(b'\\' as u16) {
            out.push(b'\\' as u16);
        }
        out.extend_from_slice(name);
        out.push(0);
        out
    }

    /// Convert a filesystem path into a null-terminated UTF-16 buffer suitable for `CreateFileW`.
    fn utf16_null_terminated(path: &Path) -> Vec<u16> {
        let mut v: Vec<u16> = path.as_os_str().encode_wide().collect();
        v.push(0);
        v
    }

    /// Check if a UTF-16 name equals exactly ASCII ".git" (case-sensitive, matching the
    /// prior behaviour). This is intentional: on Windows a mis-cased `.Git` is the same
    /// file to the filesystem but conventionally never appears, and the preprocessing pass
    /// is look-through — a missed skip just means one extra entry that will be ignored
    /// by the status pipeline.
    fn name_is_dotgit(name: &[u16]) -> bool {
        name.len() == 4
            && name[0] == b'.' as u16
            && name[1] == b'g' as u16
            && name[2] == b'i' as u16
            && name[3] == b't' as u16
    }

    /// Result type for directory walking to simplify the return type.
    type WalkResult = (Vec<(BString, WorktreeStat)>, Vec<WorkItem>);

    /// Walk a single directory using `GetFileInformationByHandleEx` with
    /// `FileIdBothDirectoryInfo`.
    ///
    /// Returns (entries to record, subdirectories to recurse into). `buffer` is a
    /// reusable 64 KiB u64-aligned scratch buffer; reusing it across calls avoids
    /// a heap allocation per directory (6k+ per worktree on the Linux kernel).
    fn walk_directory(dir_path: &[u16], rel_prefix: &str, buffer: &mut [u64]) -> std::io::Result<WalkResult> {
        let mut files = Vec::new();
        let mut subdirs = Vec::new();

        let handle = unsafe {
            CreateFileW(
                dir_path.as_ptr(),
                FILE_LIST_DIRECTORY | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            // Directory doesn't exist or can't be read — not an error for a look-through preprocess.
            return Ok((files, subdirs));
        }

        let buffer_bytes = (buffer.len() * 8) as u32;

        loop {
            let success = unsafe {
                GetFileInformationByHandleEx(
                    handle,
                    FileIdBothDirectoryInfo,
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer_bytes,
                )
            };
            if success == 0 {
                // End of enumeration (ERROR_NO_MORE_FILES) or access denied / similar.
                // Either way, stop: the preprocess is best-effort and correctness falls back
                // to per-file syscalls in `index_as_worktree`.
                break;
            }

            let mut offset = 0usize;
            loop {
                let info_ptr = unsafe { buffer.as_ptr().cast::<u8>().add(offset).cast::<FILE_ID_BOTH_DIR_INFO>() };
                let info = unsafe { &*info_ptr };

                let name_len = (info.FileNameLength / 2) as usize;
                let name_slice = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };

                let is_dot = name_len == 1 && name_slice[0] == b'.' as u16;
                let is_dotdot = name_len == 2 && name_slice[0] == b'.' as u16 && name_slice[1] == b'.' as u16;

                if !is_dot && !is_dotdot && !name_is_dotgit(name_slice) {
                    let is_dir = (info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
                    let is_reparse = (info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

                    let name_str = OsString::from_wide(name_slice).to_string_lossy().into_owned();
                    let rel_path = if rel_prefix.is_empty() {
                        name_str
                    } else {
                        format!("{rel_prefix}/{name_str}")
                    };

                    let stat = stat_from_info(info);
                    if is_dir && !is_reparse {
                        let child = join_utf16(dir_path, name_slice);
                        subdirs.push((child, rel_path.clone()));
                    }
                    files.push((rel_path.into_bytes().into(), stat));
                }

                if info.NextEntryOffset == 0 {
                    break;
                }
                offset += info.NextEntryOffset as usize;
            }
        }

        unsafe { CloseHandle(handle) };
        Ok((files, subdirs))
    }

    /// A directory the walk hasn't descended into yet, plus a count of workers
    /// currently processing work so the last one out can tell the others to exit.
    struct WorkQueue {
        dirs: VecDeque<WorkItem>,
        active_workers: usize,
    }

    /// Walk the worktree using work-stealing parallelism.
    pub fn walk_worktree_parallel<F, E>(
        worktree: &Path,
        thread_limit: Option<usize>,
        make_excludes: F,
    ) -> std::io::Result<WorktreeStats>
    where
        F: Fn() -> E + Sync,
        E: FnMut(&bstr::BStr) -> bool,
    {
        let num_threads = thread_limit
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(std::num::NonZero::get)
                    .unwrap_or(4)
            })
            .max(1);

        if num_threads == 1 {
            return walk_worktree_single_threaded(worktree, make_excludes());
        }

        let queue_mutex = Mutex::new(WorkQueue {
            dirs: VecDeque::from([(utf16_null_terminated(worktree), String::new())]),
            active_workers: 0,
        });
        let cvar = Condvar::new();
        let shared = Mutex::new(WorktreeStats::default());

        thread::scope(|s| {
            for _ in 0..num_threads {
                let make_excludes = &make_excludes;
                s.spawn(|| worker(&queue_mutex, &cvar, &shared, make_excludes()));
            }
        });

        Ok(shared.into_inner().unwrap())
    }

    /// One worker of the parallel walker. Grabs batches of directories from the
    /// shared queue, walks them into a thread-local map, and pushes any discovered
    /// subdirectories back onto the queue. Exits when the queue is drained and no
    /// worker is still producing.
    ///
    /// `is_excluded` is a thread-local predicate that returns true for directories
    /// whose contents should be skipped (gitignored). The excluded directory's own
    /// metadata entry is still recorded; only recursion is avoided.
    fn worker<E: FnMut(&bstr::BStr) -> bool>(
        queue_mutex: &Mutex<WorkQueue>,
        cvar: &Condvar,
        shared: &Mutex<WorktreeStats>,
        mut is_excluded: E,
    ) {
        let mut local = WorktreeStats::default();
        let mut local_stack: Vec<WorkItem> = Vec::new();
        let mut buffer = vec![0u64; BUFFER_U64S];

        loop {
            // Claim work, or exit if the walk is done.
            {
                let mut queue = queue_mutex.lock().unwrap();
                loop {
                    // Steal up to half of the queue (capped) to reduce re-locking while
                    // still leaving work for other threads to pick up.
                    let take = queue.dirs.len().div_ceil(2).min(32);
                    if take > 0 {
                        local_stack.extend(queue.dirs.drain(..take));
                        queue.active_workers += 1;
                        break;
                    }
                    if queue.active_workers == 0 {
                        // Queue is empty and no one is producing more work: we're done.
                        cvar.notify_all();
                        shared.lock().unwrap().extend(local);
                        return;
                    }
                    queue = cvar.wait(queue).unwrap();
                }
            }

            // Process the claimed directories outside the lock.
            let mut new_dirs: Vec<WorkItem> = Vec::new();
            while let Some((dir, rel_prefix)) = local_stack.pop() {
                if let Ok((files, subdirs)) = walk_directory(&dir, &rel_prefix, &mut buffer) {
                    local.extend(files);
                    for (child_path, child_rel) in subdirs {
                        if !is_excluded(child_rel.as_bytes().into()) {
                            new_dirs.push((child_path, child_rel));
                        }
                    }
                }
            }

            // Return discovered subdirectories; wake anyone waiting.
            let mut queue = queue_mutex.lock().unwrap();
            queue.dirs.extend(new_dirs);
            queue.active_workers -= 1;
            cvar.notify_all();
        }
    }

    /// Simple single-threaded walk for thread_limit=1.
    fn walk_worktree_single_threaded<E: FnMut(&bstr::BStr) -> bool>(
        worktree: &Path,
        mut is_excluded: E,
    ) -> std::io::Result<WorktreeStats> {
        let mut stats = WorktreeStats::default();
        let mut dir_stack: Vec<WorkItem> = vec![(utf16_null_terminated(worktree), String::new())];
        let mut buffer = vec![0u64; BUFFER_U64S];

        while let Some((dir, rel_prefix)) = dir_stack.pop() {
            if let Ok((files, subdirs)) = walk_directory(&dir, &rel_prefix, &mut buffer) {
                stats.extend(files);
                for (child_path, child_rel) in subdirs {
                    if !is_excluded(child_rel.as_bytes().into()) {
                        dir_stack.push((child_path, child_rel));
                    }
                }
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_stat_to_stat() {
        let stat = WorktreeStat {
            is_dir: false,
            is_symlink: false,
            size: 1234,
            mtime_secs: 1700000000,
            mtime_nsecs: 500_000_000,
            ctime_secs: 1699999999,
            ctime_nsecs: 100_000_000,
        };
        let s = stat.to_stat();
        assert_eq!(s.size, 1234);
        assert_eq!(s.mtime.secs, 1700000000);
        assert_eq!(s.mtime.nsecs, 500_000_000);
        assert_eq!(s.ctime.secs, 1699999999);
        assert_eq!(s.ctime.nsecs, 100_000_000);
        // dev/ino/uid/gid are always zero on Windows — `Stat::from_fs` zeros them too.
        assert_eq!(s.dev, 0);
        assert_eq!(s.ino, 0);
        assert_eq!(s.uid, 0);
        assert_eq!(s.gid, 0);
    }

    #[test]
    fn lookup_is_case_sensitive() {
        // The map is keyed by the exact path bytes the walker emits.
        // Mixed-case lookups miss rather than silently aliasing onto the wrong
        // file — a case-insensitive worktree falls back to a live `lstat` on miss.
        let mut stats = WorktreeStats::default();
        let stat = WorktreeStat {
            size: 42,
            ..Default::default()
        };
        stats.insert(BString::from(b"src/foo.rs".as_slice()), stat.clone());

        assert!(stats.get(&b"src/foo.rs"[..]).is_some());
        assert!(stats.get(&b"SRC/Foo.rs"[..]).is_none());

        stats.insert(BString::from("ünïcode.txt".as_bytes()), stat);
        assert!(stats.get("ünïcode.txt".as_bytes()).is_some());
    }

    #[test]
    fn prepare_returns_stats() {
        // Use a unique temp directory to avoid walking other files.
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("gix_status_test_{timestamp}"));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("test.txt");
        std::fs::write(&test_file, b"hello").unwrap();

        let subdir = temp_dir.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let nested_file = subdir.join("nested.txt");
        std::fs::write(&nested_file, b"world").unwrap();

        let stats = prepare(&temp_dir, Some(1), || |_: &bstr::BStr| false).unwrap();

        assert!(!stats.is_empty());
        assert!(stats.contains_key(&b"test.txt"[..]));
        assert!(stats.contains_key(&b"subdir/nested.txt"[..]));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
