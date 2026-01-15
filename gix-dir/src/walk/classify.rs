use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString, ByteSlice};

use crate::{
    entry,
    entry::PathspecMatch,
    walk::{Context, Error, ForDeletionMode, Options},
    Entry, EntryRef,
};

/// Classify the `worktree_relative_root` path and return the first `PathKind` that indicates that
/// it isn't a directory, leaving `buf` with the path matching the returned `PathKind`,
/// which is at most equal to `worktree_relative_root`.
pub fn root(
    worktree_root: &Path,
    buf: &mut BString,
    worktree_relative_root: &Path,
    options: Options<'_>,
    ctx: &mut Context<'_>,
) -> Result<(Outcome, bool), Error> {
    buf.clear();
    let mut last_length = None;
    let mut path_buf = worktree_root.to_owned();
    // These initial values kick in if worktree_relative_root.is_empty();
    let file_kind = path_buf.symlink_metadata().ok().map(|m| m.file_type().into());
    let mut out = path(&mut path_buf, buf, 0, file_kind, || None, options, ctx)?;
    let worktree_root_is_repository = out
        .disk_kind
        .is_some_and(|kind| matches!(kind, entry::Kind::Repository));

    for component in worktree_relative_root.components() {
        if last_length.is_some() {
            buf.push(b'/');
        }
        path_buf.push(component);
        buf.extend_from_slice(gix_path::os_str_into_bstr(component.as_os_str()).expect("no illformed UTF8"));
        let file_kind = path_buf.symlink_metadata().ok().map(|m| m.file_type().into());

        out = path(
            &mut path_buf,
            buf,
            last_length.map(|l| l + 1 /* slash */).unwrap_or_default(),
            file_kind,
            || None,
            options,
            ctx,
        )?;
        if !out.status.can_recurse(
            out.disk_kind,
            out.pathspec_match,
            options.for_deletion,
            worktree_root_is_repository,
        ) {
            break;
        }
        last_length = Some(buf.len());
    }
    Ok((out, worktree_root_is_repository))
}
/// The product of [`path()`] calls.
#[derive(Debug, Copy, Clone)]
pub struct Outcome {
    /// The computed status of an entry. It can be seen as aggregate of things we know about an entry.
    pub status: entry::Status,
    /// An additional property.
    pub property: Option<entry::Property>,
    /// What the entry is on disk, or `None` if we aborted the classification early or an IO-error occurred
    /// when querying the disk.
    ///
    /// Note that the index is used to avoid disk access provided its entries are marked uptodate
    /// (possibly by a prior call to update the status).
    pub disk_kind: Option<entry::Kind>,
    /// What the entry looks like in the index, or `None` if we aborted early.
    pub index_kind: Option<entry::Kind>,
    /// If a pathspec matched, this is how it matched. Maybe `None` if computation didn't see the need to evaluate it.
    pub pathspec_match: Option<PathspecMatch>,
}

impl Outcome {
    fn with_status(mut self, status: entry::Status) -> Self {
        self.status = status;
        self
    }

    fn with_kind(mut self, disk_kind: Option<entry::Kind>, index_kind: Option<entry::Kind>) -> Self {
        self.disk_kind = disk_kind;
        self.index_kind = index_kind;
        self
    }
}

impl From<&Entry> for Outcome {
    fn from(e: &Entry) -> Self {
        Outcome {
            status: e.status,
            property: e.property,
            disk_kind: e.disk_kind,
            index_kind: e.index_kind,
            pathspec_match: e.pathspec_match,
        }
    }
}

impl<'a> EntryRef<'a> {
    pub(super) fn from_outcome(rela_path: Cow<'a, BStr>, info: crate::walk::classify::Outcome) -> Self {
        EntryRef {
            rela_path,
            property: info.property,
            status: info.status,
            disk_kind: info.disk_kind,
            index_kind: info.index_kind,
            pathspec_match: info.pathspec_match,
        }
    }
}

/// Figure out what to do with `rela_path`, provided as worktree-relative path, with `disk_file_type` if it is known already
/// as it helps to match pathspecs correctly, which can be different for directories.
/// `path` is a disk-accessible variant of `rela_path` which is within the `worktree_root`, and will be modified temporarily but remain unchanged.
///
/// Note that `rela_path` is used as buffer for convenience, but will be left as is when this function returns.
/// `filename_start_idx` is the index at which the filename begins, i.e. `a/b` has `2` as index.
/// It may resemble a directory on the way to a leaf (like a file)
///
/// Returns `(status, file_kind, pathspec_matches_how)` to identify the `status` on disk, along with a classification `file_kind`,
/// and if `file_kind` is not a directory, the way the pathspec matched with `pathspec_matches_how`.
///
/// Note that non-files are pruned by default.
pub fn path(
    path: &mut PathBuf,
    rela_path: &mut BString,
    filename_start_idx: usize,
    disk_kind: Option<entry::Kind>,
    on_demand_disk_kind: impl FnOnce() -> Option<entry::Kind>,
    Options {
        ignore_case,
        recurse_repositories,
        emit_ignored,
        for_deletion,
        classify_untracked_bare_repositories,
        symlinks_to_directories_are_ignored_like_directories,
        worktree_relative_worktree_dirs,
        ..
    }: Options<'_>,
    ctx: &mut Context<'_>,
) -> Result<Outcome, Error> {
    let mut out = Outcome {
        status: entry::Status::Pruned,
        property: None,
        disk_kind,
        index_kind: None,
        pathspec_match: None,
    };
    if is_eq(rela_path[filename_start_idx..].as_bstr(), ".git", ignore_case) {
        out.pathspec_match = ctx
            .pathspec
            .pattern_matching_relative_path(
                rela_path.as_bstr(),
                disk_kind.map(|ft| ft.is_dir()),
                ctx.pathspec_attributes,
            )
            .map(Into::into);
        if for_deletion.is_some() {
            if let Some(excluded) = ctx
                .excludes
                .as_mut()
                .map_or(Ok(None), |stack| {
                    stack
                        .at_entry(
                            rela_path.as_bstr(),
                            disk_kind.map(|ft| is_dir_to_mode(ft.is_dir())),
                            ctx.objects,
                        )
                        .map(|platform| platform.excluded_kind())
                })
                .map_err(Error::ExcludesAccess)?
                .filter(|_| filename_start_idx > 0)
            {
                out.status = entry::Status::Ignored(excluded);
            }
        }
        out.property = entry::Property::DotGit.into();
        return Ok(out);
    }
    let pathspec_could_match = ctx
        .pathspec
        .can_match_relative_path(rela_path.as_bstr(), disk_kind.map(|ft| ft.is_dir()));
    if !pathspec_could_match {
        return Ok(out.with_status(entry::Status::Pruned));
    }

    let (uptodate_index_kind, index_kind, property) = resolve_file_type_with_index(
        rela_path,
        ctx.index,
        ctx.ignore_case_index_lookup.filter(|_| ignore_case),
    );
    let mut kind = uptodate_index_kind.or(disk_kind).or_else(on_demand_disk_kind);

    // We always check the pathspec to have the value filled in reliably.
    out.pathspec_match = ctx
        .pathspec
        .pattern_matching_relative_path(rela_path.as_bstr(), kind.map(|ft| ft.is_dir()), ctx.pathspec_attributes)
        .map(Into::into);

    if worktree_relative_worktree_dirs.is_some_and(|worktrees| worktrees.contains(&*rela_path)) {
        return Ok(out
            .with_kind(Some(entry::Kind::Repository), None)
            .with_status(entry::Status::Tracked));
    }

    let maybe_status = if property.is_none() {
        (index_kind.map(|k| k.is_dir()) == kind.map(|k| k.is_dir())).then_some(entry::Status::Tracked)
    } else {
        out.property = property;
        Some(entry::Status::Pruned)
    };

    let is_dir = if symlinks_to_directories_are_ignored_like_directories
        && ctx.excludes.is_some()
        && kind == Some(entry::Kind::Symlink)
    {
        path.metadata().ok().map(|md| is_dir_to_mode(md.is_dir()))
    } else {
        kind.map(|ft| is_dir_to_mode(ft.is_dir()))
    };

    let mut maybe_upgrade_to_repository = |current_kind, find_harder: bool| {
        maybe_upgrade_to_repository(
            current_kind,
            find_harder,
            recurse_repositories,
            path,
            ctx.current_dir,
            ctx.git_dir_realpath,
        )
    };
    if let Some(status) = maybe_status {
        if kind == Some(entry::Kind::Directory) && index_kind == Some(entry::Kind::Repository) {
            kind = maybe_upgrade_to_repository(kind, false);
        }
        return Ok(out.with_status(status).with_kind(kind, index_kind));
    }

    debug_assert!(maybe_status.is_none(), "It only communicates a single state right now");
    if let Some(excluded) = ctx
        .excludes
        .as_mut()
        .filter(|_| !rela_path.is_empty())
        .map_or(Ok(None), |stack| {
            stack
                .at_entry(rela_path.as_bstr(), is_dir, ctx.objects)
                .map(|platform| platform.excluded_kind())
        })
        .map_err(Error::ExcludesAccess)?
    {
        if emit_ignored.is_some() {
            if matches!(
                for_deletion,
                Some(
                    ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories
                        | ForDeletionMode::FindRepositoriesInIgnoredDirectories
                )
            ) {
                kind = maybe_upgrade_to_repository(
                    kind,
                    matches!(
                        for_deletion,
                        Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories)
                    ),
                );
            }
            if kind.is_some_and(|d| d.is_recursable_dir())
                && (out.pathspec_match.is_none()
                    || worktree_relative_worktree_dirs.is_some_and(|worktrees| {
                        for_deletion.is_some()
                            && worktrees
                                .iter()
                                .any(|dir| dir.starts_with_str(&*rela_path) && dir.get(rela_path.len()) == Some(&b'/'))
                    }))
            {
                // We have patterns that didn't match at all, *yet*, or there are contained worktrees.
                // We want to look inside.
                out.pathspec_match = Some(PathspecMatch::Prefix);
            }
        }
        return Ok(out
            .with_status(entry::Status::Ignored(excluded))
            .with_kind(kind, index_kind));
    }

    debug_assert!(maybe_status.is_none());
    let mut status = entry::Status::Untracked;

    if kind.is_some_and(|ft| ft.is_dir()) {
        kind = maybe_upgrade_to_repository(kind, classify_untracked_bare_repositories);
    } else if out.pathspec_match.is_none() {
        status = entry::Status::Pruned;
    }
    Ok(out.with_status(status).with_kind(kind, index_kind))
}

pub fn maybe_upgrade_to_repository(
    current_kind: Option<entry::Kind>,
    find_harder: bool,
    recurse_repositories: bool,
    path: &mut PathBuf,
    current_dir: &Path,
    git_dir_realpath: &Path,
) -> Option<entry::Kind> {
    if recurse_repositories {
        return current_kind;
    }
    if find_harder {
        let mut is_nested_repo = gix_discover::is_git(path).is_ok();
        if is_nested_repo {
            let git_dir_is_our_own = gix_path::realpath_opts(path, current_dir, gix_path::realpath::MAX_SYMLINKS)
                .ok()
                .is_some_and(|realpath_candidate| realpath_candidate == git_dir_realpath);
            is_nested_repo = !git_dir_is_our_own;
        }
        if is_nested_repo {
            return Some(entry::Kind::Repository);
        }
    }
    path.push(gix_discover::DOT_GIT_DIR);
    let mut is_nested_nonbare_repo = gix_discover::is_git(path).is_ok();
    if is_nested_nonbare_repo {
        let git_dir_is_our_own = gix_path::realpath_opts(path, current_dir, gix_path::realpath::MAX_SYMLINKS)
            .ok()
            .is_some_and(|realpath_candidate| realpath_candidate == git_dir_realpath);
        is_nested_nonbare_repo = !git_dir_is_our_own;
    }
    path.pop();

    if is_nested_nonbare_repo {
        Some(entry::Kind::Repository)
    } else {
        current_kind
    }
}

/// Note that `rela_path` is used as buffer for convenience, but will be left as is when this function returns.
/// Also note `maybe_file_type` will be `None` for entries that aren't up-to-date and files, for directories at least one entry must be uptodate.
/// Returns `(maybe_file_type, Option<index_file_type>, flags)`, with the last option being a flag set only for sparse directories in the index.
/// `index_file_type` is the type of `rela_path` as available in the index.
///
/// ### Shortcoming
///
/// In case-insensitive mode, if there is an entry `d` and a `D/a` both in the index, we will always find the file `d` first, and always consider
/// the entry as not uptodate, while classifying it as file (the first one we found). As quite a huge exception, this isn't properly represented
/// in the data model, and we emit a trace to make it more obvious when it happens, in case this leads to less expected results.
fn resolve_file_type_with_index(
    rela_path: &mut BString,
    index: &gix_index::State,
    ignore_case: Option<&gix_index::AccelerateLookup<'_>>,
) -> (Option<entry::Kind>, Option<entry::Kind>, Option<entry::Property>) {
    // TODO: either get this to work for icase as well, or remove the need for it. Logic is different in both branches.
    let mut special_property = None;

    fn entry_to_kinds(entry: &gix_index::Entry) -> (Option<entry::Kind>, Option<entry::Kind>) {
        let kind = if entry.mode.is_submodule() {
            entry::Kind::Repository.into()
        } else if entry.mode.contains(gix_index::entry::Mode::FILE) {
            entry::Kind::File.into()
        } else if entry.mode.contains(gix_index::entry::Mode::SYMLINK) {
            entry::Kind::Symlink.into()
        } else {
            None
        };
        (
            kind.filter(|_| entry.flags.contains(gix_index::entry::Flags::UPTODATE)),
            kind,
        )
    }

    fn icase_directory_to_kinds(dir: Option<&gix_index::Entry>) -> (Option<entry::Kind>, Option<entry::Kind>) {
        let index_kind = dir.map(|_| entry::Kind::Directory);
        let uptodate_kind = dir
            .filter(|entry| entry.flags.contains(gix_index::entry::Flags::UPTODATE))
            .map(|_| entry::Kind::Directory);
        (uptodate_kind, index_kind)
    }

    // TODO(perf): multi-threaded hash-table so it's always used, even for case-sensitive lookups, just like Git does it.
    let (uptodate_kind, index_kind) = if let Some(accelerate) = ignore_case {
        match index.entry_by_path_icase(rela_path.as_bstr(), true, accelerate) {
            None => icase_directory_to_kinds(index.entry_closest_to_directory_or_directory_icase(
                rela_path.as_bstr(),
                true,
                accelerate,
            )),
            Some(entry) => {
                if entry.mode.is_submodule() || entry.mode.is_sparse() {
                    entry_to_kinds(entry)
                } else {
                    let icase_dir =
                        index.entry_closest_to_directory_or_directory_icase(rela_path.as_bstr(), true, accelerate);
                    let directory_matches_exactly = icase_dir.is_some_and(|dir| {
                        let path = dir.path(index);
                        let slash_idx = path.rfind_byte(b'/').expect("dir");
                        path[..slash_idx].as_bstr() == rela_path
                    });
                    if directory_matches_exactly {
                        icase_directory_to_kinds(icase_dir)
                    } else {
                        entry_to_kinds(entry)
                    }
                }
            }
        }
    } else {
        match index.entry_by_path(rela_path.as_bstr()) {
            None => {
                rela_path.push(b'/');
                let res = index.prefixed_entries_range(rela_path.as_bstr());
                rela_path.pop();

                let mut one_index_signalling_with_cone = None;
                let mut all_excluded_from_worktree_non_cone = false;
                let is_tracked = res.is_some();
                let kind = res
                    .filter(|range| {
                        if range.len() == 1 {
                            one_index_signalling_with_cone = range.start.into();
                        }
                        let entries = &index.entries()[range.clone()];
                        let any_up_to_date = entries
                            .iter()
                            .any(|e| e.flags.contains(gix_index::entry::Flags::UPTODATE));
                        if !any_up_to_date && one_index_signalling_with_cone.is_none() {
                            all_excluded_from_worktree_non_cone = entries
                                .iter()
                                .all(|e| e.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE));
                        }
                        any_up_to_date
                    })
                    .map(|_| entry::Kind::Directory);

                if all_excluded_from_worktree_non_cone
                    || one_index_signalling_with_cone
                        .filter(|_| kind.is_none())
                        .is_some_and(|idx| index.entries()[idx].mode.is_sparse())
                {
                    special_property = Some(entry::Property::TrackedExcluded);
                }
                (kind, is_tracked.then_some(entry::Kind::Directory))
            }
            Some(entry) => entry_to_kinds(entry),
        }
    };
    (uptodate_kind, index_kind, special_property)
}

fn is_eq(lhs: &BStr, rhs: impl AsRef<BStr>, ignore_case: bool) -> bool {
    if ignore_case {
        lhs.eq_ignore_ascii_case(rhs.as_ref().as_ref())
    } else {
        lhs == rhs.as_ref()
    }
}

fn is_dir_to_mode(is_dir: bool) -> gix_index::entry::Mode {
    if is_dir {
        gix_index::entry::Mode::DIR
    } else {
        gix_index::entry::Mode::FILE
    }
}
