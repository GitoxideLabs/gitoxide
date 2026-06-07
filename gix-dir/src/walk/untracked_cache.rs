use std::path::Path;

use bstr::{BStr, ByteSlice};

use crate::walk::{Context, Options};

const DIR_SHOW_OTHER_DIRECTORIES: u32 = 1 << 1;
const DIR_HIDE_EMPTY_DIRECTORIES: u32 = 1 << 2;

pub(crate) struct Validated<'a> {
    cache: &'a gix_index::extension::UntrackedCache,
    object_hash: gix_index::hash::Kind,
}

impl<'a> Validated<'a> {
    pub(crate) fn root_dir(&self) -> usize {
        0
    }

    pub(crate) fn child_dir(&self, parent: usize, name: &BStr) -> Option<usize> {
        self.directory(parent)?
            .sub_directories()
            .iter()
            .copied()
            .find(|idx| self.directory(*idx).is_some_and(|dir| dir.name() == name))
    }

    pub(crate) fn directory(&self, idx: usize) -> Option<&'a gix_index::extension::untracked_cache::Directory> {
        self.cache.directories().get(idx)
    }

    pub(crate) fn is_dir_valid(&self, idx: usize, absolute_dir: &Path) -> bool {
        let Some(dir) = self.directory(idx) else {
            return false;
        };
        let Some(expected) = dir.stat() else {
            return false;
        };
        let actual = match gix_index::fs::Metadata::from_path_no_follow(absolute_dir)
            .and_then(|meta| gix_index::entry::Stat::from_fs(&meta).map_err(std::io::Error::other))
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        let use_nsec = expected.mtime.nsecs != 0;
        let opts = gix_index::entry::stat::Options {
            use_nsec,
            ..Default::default()
        };
        if !expected.matches(&actual, opts) {
            return false;
        }
        // If the IOUC recorded a .gitignore OID, verify the current file matches it.
        // If no OID was recorded, trust the directory stat — git skips the .gitignore
        // check entirely when exclude_oid is null and the directory stat is valid (see
        // valid_cached_dir() / prep_exclude() in git's dir.c). A newly-added .gitignore
        // would change the directory stat, which is already checked above.
        if let Some(expected_oid) = dir.exclude_file_oid() {
            let ignore_path = absolute_dir.join(gix_path::from_bstr(self.cache.exclude_filename_per_dir()));
            gitignore_matches(&expected_oid, &ignore_path, self.object_hash)
        } else {
            true
        }
    }
}

pub(crate) fn validate<'a>(
    worktree_root: &Path,
    index: &'a gix_index::State,
    ctx: &Context<'_>,
    opts: Options<'_>,
) -> Option<Validated<'a>> {
    let cache = index.untracked()?;
    if !cache_is_applicable(worktree_root, opts, ctx)? {
        return None;
    }

    let ident = cache.identifier().split_str("\0").next().unwrap_or(cache.identifier());
    if !ident.starts_with(expected_ident(worktree_root, ctx.current_dir).as_bytes()) {
        return None;
    }

    #[allow(unreachable_patterns)]
    let ignore = match ctx.excludes.as_deref()?.state() {
        gix_worktree::stack::State::IgnoreStack(ignore) => ignore,
        #[cfg(feature = "attributes")]
        gix_worktree::stack::State::AttributesAndIgnoreStack { ignore, .. } => ignore,
        _ => return None,
    };
    if !matches!(
        ignore.source(),
        gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped
    ) {
        return None;
    }
    if !ignore.overrides().patterns.is_empty() {
        return None;
    }
    if ignore.exclude_file_name_for_directories() != cache.exclude_filename_per_dir() {
        return None;
    }
    let info_exclude_path = ctx.git_dir_realpath.join("info").join("exclude");
    let excludes_file = ignore
        .globals()
        .patterns
        .iter()
        .filter_map(|list| list.source.as_deref())
        .find(|path| gix_path::realpath(*path).ok().as_deref() != Some(info_exclude_path.as_path()));
    let object_hash = index.object_hash();
    match (cache.excludes_file(), excludes_file) {
        (Some(expected), Some(path)) if validate_cached_stat(expected, path) => {}
        (None, None) => {}
        _ => return None,
    }
    // Also validate the cached .git/info/exclude stat and OID. If info/exclude changed since
    // the UNTR snapshot was written, cached ignore decisions for directories could be stale.
    // We verify the content hash in addition to the stat to catch same-second, same-size edits.
    match cache.info_exclude() {
        Some(expected)
            if !validate_cached_stat(expected, &info_exclude_path)
                || !gitignore_matches(&expected.id(), &info_exclude_path, object_hash) =>
        {
            return None;
        }
        _ => {}
    }

    Some(Validated {
        cache,
        object_hash: index.object_hash(),
    })
}

fn cache_is_applicable(worktree_root: &Path, opts: Options<'_>, ctx: &Context<'_>) -> Option<bool> {
    if opts.emit_ignored.is_some()
        || opts.for_deletion.is_some()
        || opts.emit_tracked
        || opts.recurse_repositories
        || opts.classify_untracked_bare_repositories
        || opts.symlinks_to_directories_are_ignored_like_directories
        || opts.worktree_relative_worktree_dirs.is_some()
        || ctx.explicit_traversal_root.is_some_and(|root| root != worktree_root)
        || ctx.pathspec.patterns().len() != 0
    {
        return Some(false);
    }
    let _dir_flags_hint = match (opts.emit_untracked, opts.emit_empty_directories) {
        (crate::walk::EmissionMode::CollapseDirectory, false) => {
            DIR_SHOW_OTHER_DIRECTORIES | DIR_HIDE_EMPTY_DIRECTORIES
        }
        (crate::walk::EmissionMode::Matching, _) => 0,
        (crate::walk::EmissionMode::CollapseDirectory, true) => return Some(false),
    };
    Some(true)
}

/// Check whether the `.gitignore` file at `path` matches the `expected_oid` stored in the IOUC.
///
/// Git's `add_patterns()` in `dir.c` always appends `'\n'` to the buffer before hashing
/// (to ensure the last pattern is terminated), but it uses the index entry OID directly
/// when the file is tracked and uptodate in the index. This means the IOUC may contain
/// either `hash(content)` or `hash(content + '\n')` depending on whether the file's
/// index stat was current at the time of the last `git status` run. We accept both.
fn gitignore_matches(
    expected_oid: &gix_index::hash::ObjectId,
    path: &Path,
    object_hash: gix_index::hash::Kind,
) -> bool {
    let Ok(data) = std::fs::read(path) else { return false };
    if let Ok(oid) = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, &data) {
        if oid == *expected_oid {
            return true;
        }
    }
    // Also try with appended '\n' (git's condition-3 hashing path)
    let mut data_plus_nl = data;
    data_plus_nl.push(b'\n');
    gix_object::compute_hash(object_hash, gix_object::Kind::Blob, &data_plus_nl).is_ok_and(|oid| oid == *expected_oid)
}

fn validate_cached_stat(expected: &gix_index::extension::untracked_cache::OidStat, path: &Path) -> bool {
    let Ok(actual) = gix_index::fs::Metadata::from_path_no_follow(path)
        .and_then(|meta| gix_index::entry::Stat::from_fs(&meta).map_err(std::io::Error::other))
    else {
        return false;
    };
    expected.stat().matches(
        &actual,
        gix_index::entry::stat::Options {
            trust_ctime: false,
            ..Default::default()
        },
    )
}

fn expected_ident(worktree_root: &Path, _current_dir: &Path) -> String {
    let path = normalize_ident_path(worktree_root);
    format!("Location {path}, system {}", system_name())
}

#[cfg(not(windows))]
fn normalize_ident_path(path: &Path) -> String {
    gix_path::realpath(path)
        .unwrap_or_else(|_| path.to_owned())
        .display()
        .to_string()
}

#[cfg(windows)]
fn normalize_ident_path(path: &Path) -> String {
    // Use canonicalize to resolve symlinks and expand 8.3 short names (via
    // GetFinalPathNameByHandleW), matching how git normalizes paths in the IOUC
    // identifier on Windows.
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    // canonicalize on Windows may return a verbatim path (\\?\C:\...), strip it.
    let s = canonical.to_string_lossy();
    let s = s.strip_prefix("\\\\?\\").unwrap_or(&*s);
    // git uses forward slashes in the ident.
    s.replace('\\', "/")
}

#[cfg(unix)]
fn system_name() -> String {
    rustix::system::uname().sysname().to_string_lossy().into_owned()
}

#[cfg(not(unix))]
fn system_name() -> String {
    // std::env::consts::OS returns "windows" (lowercase), but git writes "Windows".
    let os = std::env::consts::OS;
    let mut chars = os.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix_testtools::tempfile;
    use std::fs;

    #[test]
    #[cfg(unix)]
    fn expected_ident_resolves_symlinks() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let real_dir = tmp.path().join("real");
        fs::create_dir(&real_dir)?;
        let symlink_dir = tmp.path().join("symlink");
        std::os::unix::fs::symlink(&real_dir, &symlink_dir)?;

        let current_dir = std::env::current_dir()?;
        let ident_real = expected_ident(&real_dir, &current_dir);
        let ident_symlink = expected_ident(&symlink_dir, &current_dir);

        assert_eq!(
            ident_real, ident_symlink,
            "identifiers must be identical for the same physical location"
        );
        assert!(ident_real.contains("real"), "it must contain the resolved path");
        assert!(!ident_real.contains("symlink"), "it must not contain the symlink path");
        Ok(())
    }
}
