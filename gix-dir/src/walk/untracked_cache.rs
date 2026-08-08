use std::path::Path;

use bstr::{BStr, BString, ByteSlice};

use crate::walk::{Context, EmissionMode, Options, classify};

const DIR_SHOW_OTHER_DIRECTORIES: u32 = 1 << 1;
const DIR_HIDE_EMPTY_DIRECTORIES: u32 = 1 << 2;

pub(super) struct State<'a> {
    cache: &'a gix_index::extension::UntrackedCache,
}

impl<'a> State<'a> {
    pub(super) fn new(
        worktree_root: &Path,
        index: &'a gix_index::State,
        pathspec: &gix_pathspec::Search,
        explicit_traversal_root: Option<&Path>,
        opts: Options<'_>,
    ) -> Option<Self> {
        let cache = index.untracked()?;
        (opts.use_untracked_cache
            && opts.emit_untracked == EmissionMode::CollapseDirectory
            && opts.emit_ignored.is_none()
            && opts.for_deletion.is_none()
            && !opts.recurse_repositories
            && !opts.classify_untracked_bare_repositories
            && !opts.emit_tracked
            && !opts.emit_empty_directories
            && opts.emit_collapsed.is_none()
            && pathspec.patterns().len() == 0
            && explicit_traversal_root.is_none_or(|root| root == worktree_root)
            && cache.dir_flags() == DIR_SHOW_OTHER_DIRECTORIES | DIR_HIDE_EMPTY_DIRECTORIES
            && cache.exclude_filename_per_dir() == ".gitignore"
            && opts.untracked_cache_excludes_file.map_or_else(
                || cache.excludes_file().is_none().then_some(true),
                |path| {
                    ignore_oid_at_path_matches(
                        path,
                        index.object_hash(),
                        cache
                            .excludes_file()
                            .map(gix_index::extension::untracked_cache::OidStat::id),
                    )
                },
            )?
            && identifier_matches(cache.identifier().as_bstr(), worktree_root)?
            && ignore_oid_at_path_matches(
                &worktree_root.join(".git/info/exclude"),
                index.object_hash(),
                cache
                    .info_exclude()
                    .map(gix_index::extension::untracked_cache::OidStat::id),
            )?)
        .then_some(State { cache })
    }

    pub(super) fn directory(
        &self,
        index: usize,
        current: &Path,
        current_rela_path: &BStr,
        current_info: classify::Outcome,
        ctx: &Context<'_>,
    ) -> Option<&'a gix_index::extension::untracked_cache::Directory> {
        let directory = self.cache.directories().get(index)?;
        let metadata = gix_index::fs::Metadata::from_path_no_follow(current).ok()?;
        let stat = gix_index::entry::Stat::from_fs(&metadata).ok()?;
        let expected_check_only =
            !current_rela_path.is_empty() && current_info.status == crate::entry::Status::Untracked;
        let cached_stat = directory.stat()?;
        let stat_options = gix_index::entry::stat::Options {
            use_nsec: cached_stat.mtime.nsecs != 0,
            ..Default::default()
        };
        (cached_stat.matches(&stat, stat_options)
            && directory.check_only() == expected_check_only
            && ignore_oid_matches(current, current_rela_path, ctx, directory.exclude_file_oid())?)
        .then_some(directory)
    }

    pub(super) fn child_index(&self, directory_index: usize, name: &BStr) -> Option<usize> {
        self.cache
            .directories()
            .get(directory_index)?
            .sub_directories()
            .iter()
            .copied()
            .find(|index| {
                self.cache
                    .directories()
                    .get(*index)
                    .is_some_and(|directory| directory.name() == name)
            })
    }

    pub(super) fn child_name(&self, index: usize) -> Option<&'a BStr> {
        self.cache
            .directories()
            .get(index)
            .map(gix_index::extension::untracked_cache::Directory::name)
    }
}

#[cfg(windows)]
fn identifier_matches(identifier: &BStr, worktree_root: &Path) -> Option<bool> {
    let location = identifier
        .strip_prefix(b"Location ")?
        .strip_suffix(b", system Windows\0")?;
    Some(
        std::fs::canonicalize(gix_path::from_bstr(location.as_bstr())).ok()?
            == std::fs::canonicalize(worktree_root).ok()?,
    )
}

#[cfg(not(windows))]
fn identifier_matches(identifier: &BStr, worktree_root: &Path) -> Option<bool> {
    let worktree_location = gix_path::into_bstr(gix_path::realpath(worktree_root).ok()?);
    Some(identifier == format!("Location {}, system {}\0", worktree_location, system_name()?))
}

#[cfg(unix)]
fn system_name() -> Option<String> {
    rustix::system::uname().sysname().to_str().ok().map(ToOwned::to_owned)
}

#[cfg(not(any(unix, windows)))]
fn system_name() -> Option<String> {
    None
}

pub(super) fn has_tracked_descendant(directory: &BStr, ignore_case: bool, ctx: &Context<'_>) -> bool {
    ctx.ignore_case_index_lookup
        .map_or_else(
            || ctx.index.entry_closest_to_directory_or_directory(directory),
            |lookup| {
                ctx.index
                    .entry_closest_to_directory_or_directory_icase(directory, ignore_case, lookup)
            },
        )
        .is_some()
}

fn ignore_oid_matches(
    current: &Path,
    current_rela_path: &BStr,
    ctx: &Context<'_>,
    expected: Option<gix_index::hash::ObjectId>,
) -> Option<bool> {
    let ignore_path = current.join(".gitignore");
    match std::fs::read(&ignore_path) {
        Ok(mut data) => {
            let raw = gix_object::compute_hash(ctx.index.object_hash(), gix_object::Kind::Blob, &data).ok()?;
            if Some(raw) == expected {
                return Some(true);
            }
            data.push(b'\n');
            gix_object::compute_hash(ctx.index.object_hash(), gix_object::Kind::Blob, &data)
                .ok()
                .map(|id| Some(id) == expected)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut rela_path = BString::from(current_rela_path);
            if !rela_path.is_empty() {
                rela_path.push(b'/');
            }
            rela_path.extend_from_slice(b".gitignore");
            Some(ctx.index.entry_by_path(rela_path.as_bstr()).map(|entry| entry.id) == expected)
        }
        Err(_) => None,
    }
}

fn ignore_oid_at_path_matches(
    path: &Path,
    object_hash: gix_index::hash::Kind,
    expected: Option<gix_index::hash::ObjectId>,
) -> Option<bool> {
    match std::fs::read(path) {
        Ok(mut data) => {
            let raw = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, &data).ok()?;
            if Some(raw) == expected {
                return Some(true);
            }
            data.push(b'\n');
            gix_object::compute_hash(object_hash, gix_object::Kind::Blob, &data)
                .ok()
                .map(|id| Some(id) == expected)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Some(expected.is_none()),
        Err(_) => None,
    }
}
