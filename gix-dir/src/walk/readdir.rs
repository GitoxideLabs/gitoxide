use std::{
    borrow::Cow,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{ErrorExt, Result, bail, cancelled, message, validation};

use crate::{
    Entry, EntryRef, entry,
    entry::{PathspecMatch, Status},
    walk,
    walk::{
        Action, CollapsedEntriesEmissionMode, Context, Delegate,
        EmissionMode::CollapseDirectory,
        ForDeletionMode, Options, Outcome, classify,
        function::{can_recurse, emit_entry},
    },
};

/// Recursively walk `current`, returning `(action, prevent_collapse)` on success.
///
/// `action` indicates whether traversal should continue or stop. `prevent_collapse` is `true` if
/// this directory or a descendant prevents collapsing this directory and its ancestors into single
/// directory entries. Callers must propagate this flag to their parents.
///
/// ### Deviation
///
/// Like Git, ordinary walks skip directory-read errors, but deletion walks abort on every such error.
#[expect(clippy::too_many_arguments)]
pub(super) fn recursive(
    may_collapse: bool,
    current: &mut PathBuf,
    current_bstr: &mut BString,
    current_info: classify::Outcome,
    ctx: &mut Context<'_>,
    opts: Options<'_>,
    delegate: &mut dyn Delegate,
    out: &mut Outcome,
    state: &mut State,
) -> Result<(Action, bool)> {
    if ctx.should_interrupt.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        bail!(cancelled("Interrupted"));
    }
    let cached = ctx
        .cached_listing
        .filter(|listing| {
            listing.includes_ignored
                || (delegate.can_omit_ignored_directories()
                    && opts.emit_ignored.is_none()
                    && !opts.emit_pruned
                    && !opts.emit_empty_directories
                    && opts.emit_collapsed.is_none()
                    && opts.for_deletion.is_none()
                    && ctx.pathspec.patterns().len() == 0
                    && ctx.excludes.is_some())
        })
        .and_then(|listing| {
            listing.directories.get(current_bstr.as_bstr()).filter(|entries| {
                listing.includes_ignored
                    || (!entries.is_empty()
                        && (!may_collapse
                            || opts.emit_untracked != CollapseDirectory
                            || entries.iter().all(|(_, kind)| !kind.is_dir())))
            })
        });
    let disk = if cached.is_some() {
        out.cached_dir_listings += 1;
        None
    } else {
        out.read_dir_calls += 1;
        let Some(entries) = read_dir_result(
            gix_fs::read_dir(current, opts.precompose_unicode),
            current,
            opts.for_deletion,
            out,
        )?
        else {
            return Ok((std::ops::ControlFlow::Continue(()), true));
        };
        Some(entries)
    };
    let entries = cached
        .into_iter()
        .flatten()
        .map(|entry| Ok(DirectoryEntry::Cached(entry)))
        .chain(disk.into_iter().flatten().map(|entry| entry.map(DirectoryEntry::Disk)));

    let mut num_entries = 0;
    let mut mark = state.mark(may_collapse);
    let mut prevent_collapse = false;
    for entry in entries {
        let Some(entry) = read_dir_result(entry, current, opts.for_deletion, out)? else {
            // Preserve earlier entries without treating the unreadable directory as empty or collapsible.
            return Ok((mark.emit_all_held(state, opts, out, delegate), true));
        };
        // Important to count right away, otherwise the directory could be seen as empty even though it's not.
        // That is, this should be independent of the kind.
        num_entries += 1;

        let prev_len = current_bstr.len();
        if prev_len != 0 {
            current_bstr.push(b'/');
        }
        let file_name = entry.file_name()?;
        current_bstr.extend_from_slice(gix_path::try_os_str_into_bstr(Cow::Borrowed(file_name.as_ref()))?.as_ref());
        current.push(file_name);

        let mut info = classify::path(
            current,
            current_bstr,
            if prev_len == 0 { 0 } else { prev_len + 1 },
            None,
            || entry.kind(),
            opts,
            ctx,
        )?;

        if can_recurse(
            current_bstr.as_bstr(),
            info,
            opts.for_deletion,
            false, /* is root */
            delegate,
        ) {
            let subdir_may_collapse = state.may_collapse(current);
            let (action, subdir_prevent_collapse) = recursive(
                subdir_may_collapse,
                current,
                current_bstr,
                info,
                ctx,
                opts,
                delegate,
                out,
                state,
            )?;
            prevent_collapse |= subdir_prevent_collapse;
            if action.is_break() {
                return Ok((action, prevent_collapse));
            }
        } else {
            if opts.for_deletion == Some(ForDeletionMode::IgnoredDirectoriesCanHideNestedRepositories)
                && info.disk_kind == Some(entry::Kind::Directory)
                && matches!(info.status, Status::Ignored(_))
            {
                info.disk_kind = classify::maybe_upgrade_to_repository(
                    info.disk_kind,
                    true,
                    false,
                    current,
                    ctx.current_dir,
                    ctx.git_dir_realpath,
                );
            }
            if !state.held_for_directory_collapse(current_bstr.as_bstr(), info, &opts) {
                let action = emit_entry(Cow::Borrowed(current_bstr.as_bstr()), info, None, opts, out, delegate);
                if action.is_break() {
                    return Ok((action, prevent_collapse));
                }
            }
        }
        current_bstr.truncate(prev_len);
        current.pop();
    }

    let res = mark.reduce_held_entries(
        num_entries,
        state,
        &mut prevent_collapse,
        current,
        current_bstr.as_bstr(),
        current_info,
        opts,
        out,
        ctx,
        delegate,
    );
    Ok((res, prevent_collapse))
}

fn read_dir_result<T>(
    result: io::Result<T>,
    directory: &Path,
    for_deletion: Option<ForDeletionMode>,
    out: &mut Outcome,
) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            if for_deletion.is_some() {
                return Err(err.and_raise(message!("Failed to read the directory at \"{}\"", directory.display())));
            }
            gix_trace::warn!(?directory, ?err, "Skipping unreadable directory");
            out.read_dir_errors += 1;
            Ok(None)
        }
    }
}

enum DirectoryEntry<'a> {
    Cached(&'a (BString, entry::Kind)),
    Disk(gix_fs::read_dir::DirEntry),
}

impl DirectoryEntry<'_> {
    fn file_name(&self) -> Result<Cow<'_, OsStr>> {
        match self {
            Self::Cached((name, _)) => {
                let path = gix_path::from_byte_slice(name)?;
                if name.contains(&0) || path.file_name() != Some(path.as_os_str()) {
                    bail!(validation(format!(
                        "Cached directory entry '{name}' is not a single file name"
                    )));
                }
                Ok(Cow::Borrowed(path.as_os_str()))
            }
            Self::Disk(entry) => Ok(entry.file_name()),
        }
    }

    fn kind(&self) -> Option<entry::Kind> {
        match self {
            Self::Cached((_, kind)) => Some(*kind),
            Self::Disk(entry) => entry.file_type().ok().map(Into::into),
        }
    }
}

pub(super) struct State {
    /// The entries to hold back until it's clear what to do with them.
    pub on_hold: Vec<Entry>,
    /// The path the user is currently in, as seen from the workdir root.
    worktree_relative_current_dir: Option<PathBuf>,
}

impl State {
    /// Hold the entry with the given `status` if it's a candidate for collapsing the containing directory.
    fn held_for_directory_collapse(&mut self, rela_path: &BStr, info: classify::Outcome, opts: &Options<'_>) -> bool {
        if opts.should_hold(info.status) {
            self.on_hold
                .push(EntryRef::from_outcome(Cow::Borrowed(rela_path), info).into_owned());
            true
        } else {
            false
        }
    }

    /// Keep track of state we need to later resolve the state.
    /// Top-level directories are special, as they don't fold.
    fn mark(&self, may_collapse: bool) -> Mark {
        Mark {
            start_index: self.on_hold.len(),
            may_collapse,
        }
    }

    pub(super) fn new(worktree_root: &Path, current_dir: &Path, is_delete_mode: bool) -> Self {
        let worktree_relative_current_dir = if is_delete_mode {
            gix_path::realpath_opts(worktree_root, current_dir, gix_path::realpath::MAX_SYMLINKS)
                .ok()
                .and_then(|real_worktree_root| current_dir.strip_prefix(real_worktree_root).ok().map(ToOwned::to_owned))
                .map(|relative_cwd| worktree_root.join(relative_cwd))
        } else {
            None
        };
        Self {
            on_hold: Vec::new(),
            worktree_relative_current_dir,
        }
    }

    /// Returns `true` if the worktree-relative `directory_to_traverse` is not the current working directory.
    /// This is only the case when
    pub(super) fn may_collapse(&self, directory_to_traverse: &Path) -> bool {
        self.worktree_relative_current_dir
            .as_ref()
            .is_none_or(|cwd| cwd != directory_to_traverse)
    }

    pub(super) fn emit_remaining(
        &mut self,
        may_collapse: bool,
        opts: Options<'_>,
        out: &mut walk::Outcome,
        delegate: &mut dyn walk::Delegate,
    ) {
        if self.on_hold.is_empty() {
            return;
        }

        _ = Mark {
            start_index: 0,
            may_collapse,
        }
        .emit_all_held(self, opts, out, delegate);
    }
}

struct Mark {
    start_index: usize,
    may_collapse: bool,
}

impl Mark {
    #[expect(clippy::too_many_arguments)]
    fn reduce_held_entries(
        mut self,
        num_entries: usize,
        state: &mut State,
        prevent_collapse: &mut bool,
        dir_path: &Path,
        dir_rela_path: &BStr,
        dir_info: classify::Outcome,
        opts: Options<'_>,
        out: &mut walk::Outcome,
        ctx: &mut Context<'_>,
        delegate: &mut dyn walk::Delegate,
    ) -> walk::Action {
        if num_entries == 0 {
            let empty_info = classify::Outcome {
                property: {
                    assert_ne!(
                        dir_info.disk_kind,
                        Some(entry::Kind::Repository),
                        "BUG: it shouldn't be possible to classify an empty dir as repository"
                    );
                    if !state.may_collapse(dir_path) {
                        *prevent_collapse = true;
                        entry::Property::EmptyDirectoryAndCWD.into()
                    } else if dir_info.property.is_none() {
                        entry::Property::EmptyDirectory.into()
                    } else {
                        dir_info.property
                    }
                },
                pathspec_match: ctx
                    .pathspec
                    .pattern_matching_relative_path(dir_rela_path, Some(true), ctx.pathspec_attributes)
                    .map(|m| m.kind.into()),
                ..dir_info
            };
            if opts.should_hold(empty_info.status) {
                state
                    .on_hold
                    .push(EntryRef::from_outcome(Cow::Borrowed(dir_rela_path), empty_info).into_owned());
                std::ops::ControlFlow::Continue(())
            } else {
                emit_entry(Cow::Borrowed(dir_rela_path), empty_info, None, opts, out, delegate)
            }
        } else if *prevent_collapse {
            self.emit_all_held(state, opts, out, delegate)
        } else if let Some(action) = self.try_collapse(dir_rela_path, dir_info, state, out, opts, ctx, delegate) {
            action
        } else {
            *prevent_collapse = true;
            self.emit_all_held(state, opts, out, delegate)
        }
    }

    fn emit_all_held(
        &mut self,
        state: &mut State,
        opts: Options<'_>,
        out: &mut walk::Outcome,
        delegate: &mut dyn walk::Delegate,
    ) -> Action {
        for entry in state.on_hold.drain(self.start_index..) {
            let info = classify::Outcome::from(&entry);
            let action = emit_entry(Cow::Owned(entry.rela_path), info, None, opts, out, delegate);
            if action.is_break() {
                return action;
            }
        }
        std::ops::ControlFlow::Continue(())
    }

    #[expect(clippy::too_many_arguments)]
    fn try_collapse(
        &self,
        dir_rela_path: &BStr,
        dir_info: classify::Outcome,
        state: &mut State,
        out: &mut walk::Outcome,
        opts: Options<'_>,
        ctx: &mut Context<'_>,
        delegate: &mut dyn walk::Delegate,
    ) -> Option<Action> {
        if !self.may_collapse {
            return None;
        }
        let (mut expendable, mut precious, mut untracked, mut entries, mut matching_entries) = (0, 0, 0, 0, 0);
        for (kind, status, pathspec_match) in state.on_hold[self.start_index..]
            .iter()
            .map(|e| (e.disk_kind, e.status, e.pathspec_match))
        {
            entries += 1;
            if kind == Some(entry::Kind::Repository) {
                return None;
            }
            if pathspec_match.is_some_and(|m| matches!(m, PathspecMatch::Verbatim | PathspecMatch::Excluded)) {
                return None;
            }
            matching_entries += usize::from(pathspec_match.is_some_and(|m| !m.should_ignore()));
            match status {
                Status::Pruned => {
                    unreachable!("BUG: pruned aren't ever held, check `should_hold()`")
                }
                Status::Tracked => { /* causes the folder not to be collapsed */ }
                Status::Ignored(gix_ignore::Kind::Expendable) => expendable += 1,
                Status::Ignored(gix_ignore::Kind::Precious) => precious += 1,
                Status::Untracked => untracked += 1,
            }
        }

        if matching_entries != 0 && matching_entries != entries {
            return None;
        }

        let dir_status = if opts.emit_untracked == CollapseDirectory
            && untracked != 0
            && untracked + expendable + precious == entries
            && (opts.for_deletion.is_none()
                || (precious == 0 && expendable == 0)
                || (precious == 0 && opts.emit_ignored.is_some()))
        {
            entry::Status::Untracked
        } else if opts.emit_ignored == Some(CollapseDirectory) {
            if expendable != 0 && expendable == entries {
                entry::Status::Ignored(gix_ignore::Kind::Expendable)
            } else if precious != 0 && precious == entries {
                entry::Status::Ignored(gix_ignore::Kind::Precious)
            } else {
                return None;
            }
        } else {
            return None;
        };

        if !matches!(dir_status, entry::Status::Untracked | entry::Status::Ignored(_)) {
            return None;
        }

        if !ctx.pathspec.directory_matches_prefix(dir_rela_path, false) {
            return None;
        }

        // Pathspecs affect the collapse of the next level, hence find the highest-value one.
        let dir_pathspec_match = state.on_hold[self.start_index..]
            .iter()
            .filter_map(|e| e.pathspec_match)
            .max()
            .or_else(|| {
                // Only take directory matches as value if they are above the 'guessed' ones.
                // Otherwise we end up with seemingly matched entries in the parent directory which
                // affects proper folding.
                filter_dir_pathspec(dir_info.pathspec_match)
            });

        // If every collapsed entry is itself an empty directory, the collapsed directory has no files
        // to track. Keep the `EmptyDirectory` property so callers that skip empty directories by
        // default (such as `gix status`) behave like Git, which treats a tree of only empty
        // directories as clean. See https://github.com/GitoxideLabs/gitoxide/issues/2490.
        //
        // Restricted to untracked collapses in non-deletion walks: deletion has its own handling of
        // empty directories, and an explicitly requested *ignored* directory must stay observable even
        // when empty (it would otherwise be filtered out where empty directories aren't emitted).
        let collapsed_property = (opts.for_deletion.is_none()
            && matches!(dir_status, entry::Status::Untracked)
            && state.on_hold[self.start_index..]
                .iter()
                .all(|entry| entry.property == Some(entry::Property::EmptyDirectory)))
        .then_some(entry::Property::EmptyDirectory);

        let mut removed_without_emitting = 0;
        let mut action = std::ops::ControlFlow::Continue(());
        for entry in state.on_hold.drain(self.start_index..) {
            if action.is_break() {
                removed_without_emitting += 1;
                continue;
            }
            match opts.emit_collapsed {
                Some(mode) => {
                    if mode == CollapsedEntriesEmissionMode::All || entry.status != dir_status {
                        let info = classify::Outcome::from(&entry);
                        action = emit_entry(Cow::Owned(entry.rela_path), info, Some(dir_status), opts, out, delegate);
                    } else {
                        removed_without_emitting += 1;
                    }
                }
                None => removed_without_emitting += 1,
            }
        }
        out.seen_entries += removed_without_emitting as u32;

        state.on_hold.push(
            EntryRef::from_outcome(
                Cow::Borrowed(dir_rela_path),
                classify::Outcome {
                    status: dir_status,
                    property: dir_info.property.or(collapsed_property),
                    pathspec_match: dir_pathspec_match,
                    ..dir_info
                },
            )
            .into_owned(),
        );
        Some(action)
    }
}

fn filter_dir_pathspec(current: Option<PathspecMatch>) -> Option<PathspecMatch> {
    current.filter(|m| {
        matches!(
            m,
            PathspecMatch::Always | PathspecMatch::WildcardMatch | PathspecMatch::Verbatim
        )
    })
}

impl Options<'_> {
    fn should_hold(&self, status: entry::Status) -> bool {
        if status.is_pruned() {
            return false;
        }
        self.emit_ignored == Some(CollapseDirectory) || self.emit_untracked == CollapseDirectory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_read_failures_follow_deletion_policy() -> gix_testtools::TestResult {
        for for_deletion in [None, Some(ForDeletionMode::default())] {
            let mut out = Outcome::default();
            assert_eq!(
                read_dir_result(Ok(42), Path::new("directory"), for_deletion, &mut out)?,
                Some(42),
                "successful reads retain their value in either mode"
            );
            assert_eq!(out.read_dir_errors, 0, "successful reads do not count as errors");
            for kind in [
                io::ErrorKind::NotFound,
                io::ErrorKind::NotADirectory,
                io::ErrorKind::PermissionDenied,
                io::ErrorKind::Interrupted,
                io::ErrorKind::Other,
            ] {
                let mut out = Outcome::default();
                let result = read_dir_result::<()>(
                    Err(io::Error::from(kind)),
                    Path::new("directory"),
                    for_deletion,
                    &mut out,
                );
                if for_deletion.is_some() {
                    let err = result.expect_err("deletion must reject every directory-read failure");
                    assert_eq!(
                        err.downcast_any_ref::<io::Error>().map(io::Error::kind),
                        Some(kind),
                        "the original I/O error stays available for callers"
                    );
                } else {
                    assert_eq!(result?, None, "ordinary walks skip failed reads");
                    assert_eq!(out.read_dir_errors, 1, "each failed read marks the listing incomplete");
                }
            }
        }
        Ok(())
    }
}
