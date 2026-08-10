use std::{
    collections::BTreeSet,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use bstr::{BStr, BString, ByteSlice};
use gix_index::entry::{Flags, Mode, Stat};

use crate::checkout;

/// Remove all files from `dir` that are tracked in `previous_index` but not in `index`, along with
/// directories that become empty as a result, and return the amount of removed files.
///
/// Files that appear to have changed on disk since `previous_index` was written are not removed, but
/// either abort the operation or are recorded in `errors`, depending on
/// [`keep_going`](checkout::Options::keep_going). [`overwrite_existing`](checkout::Options::overwrite_existing)
/// removes such files nonetheless. Untracked files are never touched, which also keeps
/// the directories they reside in alive.
pub fn remove_stale_entries(
    dir: &Path,
    previous_index: &gix_index::State,
    index: &gix_index::State,
    paths: &gix_index::PathStorageRef,
    errors: &mut Vec<checkout::ErrorRecord>,
    should_interrupt: &AtomicBool,
    options: &checkout::Options,
) -> Result<usize, checkout::Error> {
    let mut files_removed = 0;
    let mut new_paths = index.entries().iter().map(|e| e.path_in(paths)).peekable();
    let mut leading = LeadingDirectories::default();
    let mut dirs_to_prune = BTreeSet::<BString>::new();
    let mut prev_rela_path = None;
    for entry in previous_index.entries() {
        if should_interrupt.load(Ordering::Relaxed) {
            break;
        }
        if entry.flags.contains(Flags::SKIP_WORKTREE) || matches!(entry.mode, Mode::DIR | Mode::COMMIT) {
            continue;
        }
        let rela_path = entry.path(previous_index);
        if prev_rela_path == Some(rela_path) {
            continue;
        }
        prev_rela_path = Some(rela_path);
        while new_paths.peek().is_some_and(|new_path| *new_path < rela_path) {
            new_paths.next();
        }
        if new_paths.peek() == Some(&rela_path) {
            continue;
        }
        match remove_entry(dir, entry, rela_path, &mut leading, &mut dirs_to_prune, options) {
            Ok(true) => files_removed += 1,
            Ok(false) => {}
            Err(err) => {
                if options.keep_going {
                    errors.push(checkout::ErrorRecord {
                        path: rela_path.into(),
                        error: Box::new(err),
                    });
                } else {
                    return Err(err);
                }
            }
        }
    }
    remove_empty_directories(dir, dirs_to_prune);
    Ok(files_removed)
}

/// Remove the file previously checked out as `entry` at `rela_path` within `dir`, and return `true` if it was removed.
fn remove_entry(
    dir: &Path,
    entry: &gix_index::Entry,
    rela_path: &BStr,
    leading: &mut LeadingDirectories,
    dirs_to_prune: &mut BTreeSet<BString>,
    options: &checkout::Options,
) -> Result<bool, checkout::Error> {
    for component in rela_path.split(|b| *b == b'/') {
        gix_worktree::validate::path::component(component.as_bstr(), None, options.validate)
            .map_err(std::io::Error::other)?;
    }
    let parent = rela_path.rfind_byte(b'/').map(|slash| rela_path[..slash].as_bstr());
    if let Some(parent) = parent {
        if !leading.is_intact(dir, parent)? {
            return Ok(false);
        }
    }
    let path = dir.join(
        gix_path::try_from_bstr(rela_path).map_err(|_| checkout::Error::IllformedUtf8 {
            path: rela_path.to_owned(),
        })?,
    );
    let md = match gix_index::fs::Metadata::from_path_no_follow(&path) {
        Ok(md) => md,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = parent {
                dirs_to_prune.insert(parent.to_owned());
            }
            return Ok(false);
        }
        Err(err) => return Err(err.into()),
    };
    if md.is_dir() {
        if !options.overwrite_existing {
            return Err(checkout::Error::RemovalPrevented {
                rela_path: rela_path.to_owned(),
            });
        }
        std::fs::remove_dir_all(&path)?;
    } else {
        if !options.overwrite_existing {
            let kind_matches = md.is_symlink() == (entry.mode == Mode::SYMLINK);
            let fs_stat = Stat::from_fs(&md)?;
            if !kind_matches || !entry.stat.matches(&fs_stat, options.stat_options) {
                return Err(checkout::Error::RemovalPrevented {
                    rela_path: rela_path.to_owned(),
                });
            }
        }
        if md.is_symlink() {
            gix_fs::symlink::remove(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    if let Some(parent) = parent {
        dirs_to_prune.insert(parent.to_owned());
    }
    Ok(true)
}

/// A cache for information about the leading directories of previously deleted files, all relative to the
/// worktree root. As entries are sorted by path, consecutive entries share most of their leading path.
#[derive(Default)]
struct LeadingDirectories {
    /// A path whose components are all known to be directories on disk.
    verified: BString,
    /// A path whose last component is known to not be a directory, so nothing beneath it can be removed.
    blocked: Option<BString>,
}

impl LeadingDirectories {
    /// Return `true` if all components of `parent` below `base` are directories on disk, without ever
    /// following symlinks, similar to how `git` verifies leading paths before unlinking an entry.
    fn is_intact(&mut self, base: &Path, parent: &BStr) -> std::io::Result<bool> {
        if let Some(blocked) = self.blocked.as_ref() {
            if is_component_prefix(blocked.as_bstr(), parent) {
                return Ok(false);
            }
        }
        let common = component_prefix_len(self.verified.as_bstr(), parent);
        let mut verified: BString = parent[..common].into();
        for component in parent[common..].split(|b| *b == b'/').filter(|c| !c.is_empty()) {
            if !verified.is_empty() {
                verified.push(b'/');
            }
            verified.extend_from_slice(component);
            let path = gix_path::try_from_bstr(verified.as_bstr())
                .map_err(|_| std::io::Error::other("path component contained illformed UTF-8"))?;
            match std::fs::symlink_metadata(base.join(path)) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => {
                    self.blocked = Some(verified);
                    return Ok(false);
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    self.blocked = Some(verified);
                    return Ok(false);
                }
                Err(err) => return Err(err),
            }
        }
        self.verified = verified;
        Ok(true)
    }
}

/// Return `true` if `path` is equal to `prefix` or resides beneath it.
fn is_component_prefix(prefix: &BStr, path: &BStr) -> bool {
    path == prefix || (path.starts_with(prefix) && path.get(prefix.len()) == Some(&b'/'))
}

/// Return the length in bytes of the longest shared directory prefix of `lhs` and `rhs`.
fn component_prefix_len(lhs: &BStr, rhs: &BStr) -> usize {
    let mut len = 0;
    let mut lhs = lhs.split(|b| *b == b'/');
    let mut rhs = rhs.split(|b| *b == b'/');
    loop {
        let (Some(a), Some(b)) = (lhs.next(), rhs.next()) else {
            return len;
        };
        if a != b || a.is_empty() {
            return len;
        }
        len = if len == 0 { a.len() } else { len + 1 + a.len() };
    }
}

/// Remove all directories in `dirs` that are empty, along with their parents that become empty in turn,
/// while leaving directories that still contain untracked files untouched.
fn remove_empty_directories(base: &Path, mut dirs: BTreeSet<BString>) {
    while let Some(dir) = dirs.pop_last() {
        let Ok(path) = gix_path::try_from_bstr(dir.as_bstr()) else {
            continue;
        };
        if std::fs::remove_dir(base.join(path)).is_err() {
            continue;
        }
        if let Some(slash) = dir.rfind_byte(b'/') {
            dirs.insert(dir[..slash].into());
        }
    }
}
