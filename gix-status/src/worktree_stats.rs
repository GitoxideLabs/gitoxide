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
use std::sync::atomic::AtomicBool;

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
    /// Whether this is a directory, with the same semantics as
    /// `std::fs::symlink_metadata`: `false` for directory symlinks and junctions
    /// even though the filesystem also marks those with the directory attribute.
    pub is_dir: bool,
    /// Whether this is a symlink, with the same semantics as
    /// `std::fs::symlink_metadata`: `true` only for name-surrogate reparse points
    /// (symlinks and junctions/mount points). Other reparse points — OneDrive
    /// cloud placeholders, ProjFS, app-exec links — are regular files/dirs, as
    /// the live `lstat` fallback would report them.
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
/// `start_dir` seeds the walk at a worktree-relative directory (`/`-separated,
/// no leading slash, trailing slash tolerated; empty walks the entire
/// worktree). Use it to avoid walking unrelated subtrees when a pathspec
/// restricts status to a known prefix — keys in the returned map are full
/// worktree-relative paths either way. If `start_dir` doesn't name a directory
/// on disk the map comes back empty, which is still correct: lookups miss and
/// fall back to live syscalls.
///
/// `thread_limit` caps parallelism. `None` uses all available cores; `Some(1)`
/// is single-threaded.
///
/// `should_interrupt` stops the walk early at directory granularity. Whatever
/// was gathered until then is returned — a partial map is still a valid
/// look-through map.
///
/// The walk does not descend into nested repositories: a directory containing
/// a `.git` entry (other than the walk root) marks a submodule worktree or an
/// embedded repository whose contents can't be tracked by this index. The
/// directory itself still gets an entry, which is all submodule status needs.
///
/// `make_excludes` is called once on each worker thread and returns a predicate
/// that owns thread-local state (e.g. a `gix_worktree::Stack`). Each time the
/// walker is about to descend into a subdirectory, it calls the predicate with
/// the worktree-relative path; returning `true` skips that subtree. Callers
/// that don't need gitignore pruning can pass `|| |_: &bstr::BStr| false`, but
/// for typical projects with fat ignored dirs (`node_modules`, `target`) the
/// wasted enumeration makes the preprocessing pass net-slower than plain
/// per-file stats.
pub fn prepare<F, E>(
    worktree: &Path,
    start_dir: &bstr::BStr,
    thread_limit: Option<usize>,
    should_interrupt: &AtomicBool,
    make_excludes: F,
) -> std::io::Result<WorktreeStats>
where
    F: Fn() -> E + Sync,
    E: FnMut(&bstr::BStr) -> bool,
{
    windows::walk_worktree_parallel(worktree, start_dir, thread_limit, should_interrupt, make_excludes)
}

/// Like [`prepare`], but single-threaded and without the `Sync` requirement on
/// the excludes predicate — for callers whose gitignore machinery can't be
/// shared across threads, like `gix` built without its `parallel` feature.
pub fn prepare_single_threaded<E>(
    worktree: &Path,
    start_dir: &bstr::BStr,
    should_interrupt: &AtomicBool,
    is_excluded: E,
) -> std::io::Result<WorktreeStats>
where
    E: FnMut(&bstr::BStr) -> bool,
{
    windows::walk_worktree_single_threaded(
        windows::seed_work_item(worktree, start_dir),
        should_interrupt,
        is_excluded,
    )
}

/// Windows-specific implementation using `GetFileInformationByHandleEx` /
/// `FileIdBothDirectoryInfo`. Work-stealing across threads via `thread::scope`.
#[allow(unsafe_code)]
mod windows {
    use super::*;
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::Ordering;
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
    ///
    /// File-type flags must mirror what the live fallback
    /// (`gix_index::fs::Metadata`, i.e. `std::fs::symlink_metadata`) reports for
    /// the same path, or cache hits and misses would disagree on the type and
    /// misreport unchanged entries as type-changed or removed. std only calls a
    /// reparse point a symlink if its tag is a *name surrogate* (symlinks and
    /// junctions), and reports `is_dir=false` for those; non-surrogate reparse
    /// points (OneDrive cloud placeholders, ProjFS, app-exec links, WOF) are
    /// plain files/directories to std. Directory enumeration stores the reparse
    /// tag in `EaSize` when `FILE_ATTRIBUTE_REPARSE_POINT` is set (MS-FSCC
    /// 2.4.17, same convention as `WIN32_FIND_DATA::dwReserved0`), so we can
    /// replicate that logic without extra syscalls.
    fn stat_from_info(info: &FILE_ID_BOTH_DIR_INFO) -> WorktreeStat {
        let size = info.EndOfFile as u64;

        // FILETIME values are LARGE_INTEGER holding 100ns intervals since 1601-01-01 UTC.
        // `ctime` must come from `CreationTime` (not mtime): `gix_index::entry::stat::from_fs`
        // on Windows populates ctime from `Metadata::created()`, which is CreationTime. If we
        // faked ctime=mtime here, stat comparison would spuriously fail for any file where
        // creation-time and modification-time differ, forcing an unnecessary content hash.
        let (mtime_secs, mtime_nsecs) = filetime_to_unix(info.LastWriteTime as u64);
        let (ctime_secs, ctime_nsecs) = filetime_to_unix(info.CreationTime as u64);

        let (is_dir, is_symlink) = type_flags_from_attributes(info.FileAttributes, info.EaSize);

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

    /// Derive `(is_dir, is_symlink)` exactly like `std::fs::FileType` does for
    /// `symlink_metadata`, given raw directory-enumeration data. `reparse_tag`
    /// is `EaSize` reinterpreted, only meaningful when the reparse attribute is set.
    pub(super) fn type_flags_from_attributes(attributes: u32, reparse_tag: u32) -> (bool, bool) {
        /// `IsReparseTagNameSurrogate`: the tag names another filesystem object
        /// (symlink, junction/mount point), as opposed to tags that overlay data
        /// onto the file itself (cloud placeholders, WOF, ProjFS, ...).
        const NAME_SURROGATE_BIT: u32 = 0x2000_0000;
        let is_reparse = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        let is_symlink = is_reparse && (reparse_tag & NAME_SURROGATE_BIT) != 0;
        let is_dir = !is_symlink && (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
        (is_dir, is_symlink)
    }

    /// Read a record's UTF-16 `FileName` without assuming the record is aligned.
    ///
    /// `FILE_ID_BOTH_DIR_INFO` records can be returned at unaligned offsets by some
    /// filesystem drivers (see the call site and rust-lang/rust#104530), so forming a
    /// `&[u16]` over the buffer would be UB. Borrow in place in the common aligned case;
    /// copy element-by-element with unaligned reads otherwise. Mirrors std's
    /// `from_maybe_unaligned` helper.
    ///
    /// # Safety
    ///
    /// `p` must point at `len` `u16`s of an in-bounds, initialized directory record.
    unsafe fn name_from_maybe_unaligned<'a>(p: *const u16, len: usize) -> std::borrow::Cow<'a, [u16]> {
        if p.is_aligned() {
            std::borrow::Cow::Borrowed(unsafe { std::slice::from_raw_parts(p, len) })
        } else {
            std::borrow::Cow::Owned((0..len).map(|i| unsafe { p.add(i).read_unaligned() }).collect())
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

    /// Build the initial [`WorkItem`] for a walk of `worktree` seeded at the
    /// worktree-relative directory `start_dir` (empty: the worktree root).
    ///
    /// A non-UTF-8 `start_dir` can't be turned into the `String` rel-prefix the
    /// walker builds keys from (and `gix-pathspec` never produces one), so it
    /// falls back to walking the whole tree — more work, never wrong.
    pub(super) fn seed_work_item(worktree: &Path, start_dir: &bstr::BStr) -> WorkItem {
        let start_dir = start_dir.strip_suffix(b"/").unwrap_or(start_dir);
        match std::str::from_utf8(start_dir) {
            Ok(rel) if !rel.is_empty() => (utf16_null_terminated(&worktree.join(rel)), rel.to_string()),
            _ => (utf16_null_terminated(worktree), String::new()),
        }
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
    ///
    /// A `.git` entry in a non-root directory marks a nested repository
    /// (submodule worktree or embedded repo) — its contents are returned without
    /// subdirectories to recurse into. The direct children stay in the map: the
    /// nested root itself is what submodule status looks up, and in the odd case
    /// of files tracked by the *outer* index inside such a directory, anything
    /// deeper simply misses and falls back to a live `lstat`.
    fn walk_directory(
        dir_path: &[u16],
        rel_prefix: &str,
        is_root: bool,
        buffer: &mut [u64],
    ) -> std::io::Result<WalkResult> {
        let mut files = Vec::new();
        let mut subdirs = Vec::new();
        let mut has_dotgit = false;

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
                // The buffer is `Vec<u64>` (8-byte aligned), but only the *first* record is
                // guaranteed aligned: subsequent records sit at `offset += NextEntryOffset`,
                // and while the docs say those offsets are aligned, some filesystem drivers
                // return them unaligned in practice. Forming `&*info_ptr` over an unaligned
                // record is instant UB (and has crashed callers), so read the fixed header by
                // value with an unaligned read and copy the name out when it isn't aligned.
                // std hit and worked around the same `FileIdBothDirectoryInfo` issue:
                // rust-lang/rust#104530.
                //
                // The cast to a more-aligned pointer type is sound here precisely because we
                // never dereference it as aligned — only `read_unaligned` / `&raw const`.
                #[allow(clippy::cast_ptr_alignment)]
                let info_ptr = unsafe { buffer.as_ptr().cast::<u8>().add(offset).cast::<FILE_ID_BOTH_DIR_INFO>() };
                let info = unsafe { info_ptr.read_unaligned() };
                let info = &info;

                let name_len = (info.FileNameLength / 2) as usize;
                let name_ptr = unsafe { (&raw const (*info_ptr).FileName).cast::<u16>() };
                let name = unsafe { name_from_maybe_unaligned(name_ptr, name_len) };
                let name_slice: &[u16] = &name;

                let is_dot = name_len == 1 && name_slice[0] == b'.' as u16;
                let is_dotdot = name_len == 2 && name_slice[0] == b'.' as u16 && name_slice[1] == b'.' as u16;

                if name_is_dotgit(name_slice) {
                    has_dotgit = true;
                } else if !is_dot && !is_dotdot {
                    // Recurse only into what the live pipeline would consider a directory:
                    // name-surrogate reparse points (junctions, directory symlinks) are
                    // symlinks to it and must not be followed — that's also what protects
                    // the walk from filesystem cycles.
                    let (is_dir, _is_symlink) = type_flags_from_attributes(info.FileAttributes, info.EaSize);

                    // Build the worktree-relative path in a single allocation by decoding the
                    // UTF-16 name straight into a string already holding `rel_prefix/`, instead
                    // of materializing the name as its own `String` and then `format!`-ing a
                    // second copy. This runs once per worktree entry (~90k on the Linux kernel),
                    // so the extra allocation + copy per entry was a measurable slice of the walk.
                    //
                    // Decode fallibly: skip on ill-formed sequences rather than substituting
                    // U+FFFD. Lossy substitution can collapse two distinct invalid names onto the
                    // same key (one overwriting the other in the map) and never matches what
                    // `gix-index` stored anyway, so a miss + live `lstat` fallback is strictly
                    // cleaner. `char::decode_utf16` yields an error on an unpaired surrogate; we
                    // drop the whole entry in that case.
                    let prefix_len = if rel_prefix.is_empty() { 0 } else { rel_prefix.len() + 1 };
                    // `name_len` is UTF-16 code units; for the ASCII paths that dominate git
                    // worktrees that equals the UTF-8 byte count, so this reservation is exact and
                    // non-ASCII names cost at most a single realloc.
                    let mut rel_path = String::with_capacity(prefix_len + name_len);
                    if !rel_prefix.is_empty() {
                        rel_path.push_str(rel_prefix);
                        rel_path.push('/');
                    }
                    let mut valid = true;
                    for ch in char::decode_utf16(name_slice.iter().copied()) {
                        match ch {
                            Ok(ch) => rel_path.push(ch),
                            Err(_) => {
                                valid = false;
                                break;
                            }
                        }
                    }

                    if valid {
                        let stat = stat_from_info(info);
                        if is_dir {
                            let child = join_utf16(dir_path, name_slice);
                            subdirs.push((child, rel_path.clone()));
                        }
                        files.push((rel_path.into_bytes().into(), stat));
                    }
                }

                if info.NextEntryOffset == 0 {
                    break;
                }
                offset += info.NextEntryOffset as usize;
            }
        }

        unsafe { CloseHandle(handle) };

        if has_dotgit && !is_root {
            subdirs.clear();
        }
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
        start_dir: &bstr::BStr,
        thread_limit: Option<usize>,
        should_interrupt: &AtomicBool,
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

        let root = seed_work_item(worktree, start_dir);
        if num_threads == 1 {
            return walk_worktree_single_threaded(root, should_interrupt, make_excludes());
        }

        // Only the root's `rel_prefix` ever equals this — child prefixes are strictly longer.
        let root_rel = root.1.clone();
        let root_rel = root_rel.as_str();
        let queue_mutex = Mutex::new(WorkQueue {
            dirs: VecDeque::from([root]),
            active_workers: 0,
        });
        let cvar = Condvar::new();
        let shared = Mutex::new(WorktreeStats::default());

        thread::scope(|s| {
            for _ in 0..num_threads {
                let make_excludes = &make_excludes;
                s.spawn(|| {
                    worker(
                        &queue_mutex,
                        &cvar,
                        &shared,
                        root_rel,
                        should_interrupt,
                        make_excludes(),
                    )
                });
            }
        });

        Ok(shared.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner))
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
        root_rel: &str,
        should_interrupt: &AtomicBool,
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
                    if should_interrupt.load(Ordering::Relaxed) {
                        // Drop pending work — with the queue empty, every worker exits
                        // through the regular done-path once in-flight batches finish.
                        queue.dirs.clear();
                    }
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
                if should_interrupt.load(Ordering::Relaxed) {
                    local_stack.clear();
                    break;
                }
                if let Ok((files, subdirs)) = walk_directory(&dir, &rel_prefix, rel_prefix == root_rel, &mut buffer) {
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
    pub(super) fn walk_worktree_single_threaded<E: FnMut(&bstr::BStr) -> bool>(
        root: WorkItem,
        should_interrupt: &AtomicBool,
        mut is_excluded: E,
    ) -> std::io::Result<WorktreeStats> {
        let mut stats = WorktreeStats::default();
        let root_rel = root.1.clone();
        let mut dir_stack: Vec<WorkItem> = vec![root];
        let mut buffer = vec![0u64; BUFFER_U64S];

        while let Some((dir, rel_prefix)) = dir_stack.pop() {
            if should_interrupt.load(Ordering::Relaxed) {
                break;
            }
            if let Ok((files, subdirs)) = walk_directory(&dir, &rel_prefix, rel_prefix == root_rel, &mut buffer) {
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
    #[cfg(windows)]
    fn type_flags_match_std_symlink_metadata_semantics() {
        use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT};

        let flags = super::windows::type_flags_from_attributes;
        const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
        const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
        const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
        const IO_REPARSE_TAG_CLOUD: u32 = 0x9000_001A;
        const IO_REPARSE_TAG_WOF: u32 = 0x8000_0017;

        // plain file / plain dir
        assert_eq!(flags(FILE_ATTRIBUTE_NORMAL, 0), (false, false));
        assert_eq!(flags(FILE_ATTRIBUTE_DIRECTORY, 0), (true, false));
        // file and directory symlinks: symlink, never dir — like `std`
        assert_eq!(
            flags(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_SYMLINK),
            (false, true)
        );
        assert_eq!(
            flags(
                FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
                IO_REPARSE_TAG_SYMLINK
            ),
            (false, true)
        );
        // junction (mount point): treated as symlink by `std`
        assert_eq!(
            flags(
                FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
                IO_REPARSE_TAG_MOUNT_POINT
            ),
            (false, true)
        );
        // non-surrogate reparse points are regular files/dirs (cloud placeholders, WOF)
        assert_eq!(
            flags(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_CLOUD),
            (false, false)
        );
        assert_eq!(
            flags(
                FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
                IO_REPARSE_TAG_CLOUD
            ),
            (true, false)
        );
        assert_eq!(flags(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_WOF), (false, false));
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

    fn unique_temp_dir() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("gix_status_test_{timestamp}"));
        std::fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    fn prepare_simple(worktree: &Path, start_dir: &str) -> WorktreeStats {
        prepare(worktree, start_dir.into(), Some(1), &AtomicBool::new(false), || {
            |_: &bstr::BStr| false
        })
        .unwrap()
    }

    #[test]
    fn prepare_returns_stats() {
        // Use a unique temp directory to avoid walking other files.
        let temp_dir = unique_temp_dir();

        let test_file = temp_dir.join("test.txt");
        std::fs::write(&test_file, b"hello").unwrap();

        let subdir = temp_dir.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let nested_file = subdir.join("nested.txt");
        std::fs::write(&nested_file, b"world").unwrap();

        let stats = prepare_simple(&temp_dir, "");
        assert!(!stats.is_empty());
        assert!(stats.contains_key(&b"test.txt"[..]));
        assert!(stats.contains_key(&b"subdir/nested.txt"[..]));

        // Seeded at a subdirectory: keys stay worktree-relative, unrelated files are not walked.
        let stats = prepare_simple(&temp_dir, "subdir");
        assert!(stats.contains_key(&b"subdir/nested.txt"[..]));
        assert!(!stats.contains_key(&b"test.txt"[..]));
        // Trailing slash and missing directories degrade to (partial) maps, never errors.
        let stats = prepare_simple(&temp_dir, "subdir/");
        assert!(stats.contains_key(&b"subdir/nested.txt"[..]));
        assert!(prepare_simple(&temp_dir, "does-not-exist").is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn nested_repositories_are_not_descended_into() {
        let temp_dir = unique_temp_dir();

        let nested = temp_dir.join("nested");
        std::fs::create_dir_all(nested.join(".git")).unwrap();
        std::fs::create_dir_all(nested.join("sub")).unwrap();
        std::fs::write(nested.join("file.txt"), b"direct child").unwrap();
        std::fs::write(nested.join("sub").join("deep.txt"), b"below nested root").unwrap();

        let stats = prepare_simple(&temp_dir, "");
        // The nested root and its direct children are recorded — the former is what
        // submodule status looks up — but nothing below its subdirectories.
        assert!(stats.contains_key(&b"nested"[..]));
        assert!(stats.contains_key(&b"nested/file.txt"[..]));
        assert!(stats.contains_key(&b"nested/sub"[..]));
        assert!(!stats.contains_key(&b"nested/sub/deep.txt"[..]));
        assert!(!stats.contains_key(&b"nested/.git"[..]));

        // Seeded *at* the nested repo, it is the walk root and is walked normally,
        // matching a status run inside the nested repo's own worktree.
        let stats = prepare_simple(&temp_dir, "nested");
        assert!(stats.contains_key(&b"nested/sub/deep.txt"[..]));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn interrupt_stops_the_walk_early() {
        let temp_dir = unique_temp_dir();
        std::fs::write(temp_dir.join("file.txt"), b"content").unwrap();

        let interrupted = AtomicBool::new(true);
        let stats = prepare(&temp_dir, "".into(), Some(1), &interrupted, || |_: &bstr::BStr| false).unwrap();
        assert!(stats.is_empty(), "pre-set interrupt yields an empty (valid) map");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
