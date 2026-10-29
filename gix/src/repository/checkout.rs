use crate::{Repository, Result};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::bstr::{BStr, BString, ByteSlice};
use crate::error::{ErrorExt, ResultExt, bail, ensure, message, unsupported, validation};
use gix_index::{
    State,
    entry::{Flags, Mode, Stage},
};
use gix_status::index_as_worktree::{Change as StatusChange, EntryStatus};
use gix_worktree_state::checkout::Change;

impl Repository {
    /// Return options that can be used to drive a low-level checkout operation.
    /// Use `attributes_source` to determine where `.gitattributes` files should be read from, which depends on
    /// the presence of a worktree to begin with.
    /// Here, typically this value would be [`gix_worktree::stack::state::attributes::Source::IdMapping`]
    pub fn checkout_options(
        &self,
        attributes_source: gix_worktree::stack::state::attributes::Source,
    ) -> Result<gix_worktree_state::checkout::Options> {
        self.config.checkout_options(self, attributes_source)
    }

    /// Materialize `target` relative to an expected worktree `baseline`, updating the caller's staging `index`.
    ///
    /// `baseline` describes expected worktree contents, which may differ from staged contents. `target` is
    /// authoritative at changed paths and receives refreshed stat data for written entries. `index` retains
    /// unrelated staged entries; entries at changed paths are replaced with the target, including deletions.
    /// This operation does not choose merge or branch-switch policy and never persists an index or changes refs.
    ///
    /// Before any worktree write, changed tracked files must match either the baseline or target. With
    /// `overwrite_tracked`, dirty or missing baseline-tracked files are restored as well. Existing untracked
    /// and ignored files, including obstructions inside directories, are preserved unless `overwrite_ignored`
    /// permits replacing ignored obstructions. Nested repositories remain protected. Submodules are not
    /// checked out recursively. Files already excluded by `SKIP_WORKTREE` remain excluded while their index
    /// content is updated; sparse patterns are not reevaluated. Compressed sparse indexes are not yet supported.
    /// Baseline and target must be full, stage-zero indexes without intent-to-add entries. Touched conflicts
    /// in the staging index are rejected; unrelated conflicts remain intact.
    ///
    /// Callers must prevent concurrent worktree/index changes. Preflight failures leave the worktree and
    /// staging index untouched. Later IO or filter failures can leave partial worktree changes, but the staging
    /// index is updated only after complete success. Cancellation is an error. `target` may retain stat updates
    /// from completed writes after a failure. Use the returned `index_changed` to decide whether to persist `index`.
    pub fn checkout_index_with_baseline<P>(
        &self,
        baseline: &State,
        target: &mut State,
        index: &mut State,
        mut progress: P,
        should_interrupt: &AtomicBool,
        options: crate::worktree::checkout::Options,
    ) -> Result<crate::worktree::checkout::Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: gix_utils::progress::NestedProgress + 'static,
    {
        interrupted(should_interrupt)?;
        let repo = self.with_absolute_paths()?;
        ensure!(
            !baseline.is_sparse() && !target.is_sparse() && !index.is_sparse(),
            unsupported("Checkout requires expanded sparse indexes")
        );
        ensure!(
            [baseline, target, index]
                .iter()
                .all(|state| state.object_hash() == repo.object_hash()),
            validation("Checkout indexes must use the repository's object hash")
        );
        let workdir = repo
            .workdir()
            .ok_or_else(|| validation("Checkout requires a worktree").raise())?;
        let checkout_options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        let ignore_case = checkout_options.fs.ignore_case;
        for state in [baseline, &*target] {
            state.verify_entries()?;
            let lookup = state.prepare_icase_backing();
            for entry in state.entries() {
                ensure!(
                    entry.stage() == Stage::Unconflicted && !entry.flags.contains(Flags::INTENT_TO_ADD),
                    validation("Checkout requires materialized stage-zero entries")
                );
                ensure!(
                    matches!(
                        entry.mode,
                        Mode::FILE | Mode::FILE_EXECUTABLE | Mode::SYMLINK | Mode::COMMIT
                    ),
                    validation("Checkout requires supported, non-sparse entry modes")
                );
                let path = entry.path(state);
                validate_path(path, entry.mode, checkout_options.validate)?;
                ensure!(
                    state
                        .entry_by_path_icase(path, ignore_case, &lookup)
                        .is_none_or(|other| other.path(state) == path),
                    conflict(path, "Index paths collide on this filesystem")
                );
                for parent in parents(path) {
                    ensure!(
                        state.entry_by_path_icase(parent, ignore_case, &lookup).is_none(),
                        conflict(path, "Index contains both a file and its descendant")
                    );
                }
            }
        }
        let mut pending = BTreeMap::<BString, Pending>::new();
        gix_diff::index::<gix_odb::Handle>(
            baseline,
            target,
            |change| {
                let item = match &change {
                    gix_diff::index::ChangeRef::Addition { index, .. } => Pending::new(None, Some(*index)),
                    gix_diff::index::ChangeRef::Deletion { index, .. } => Pending::new(Some(*index), None),
                    gix_diff::index::ChangeRef::Modification {
                        previous_index, index, ..
                    } => Pending::new(Some(*previous_index), Some(*index)),
                    gix_diff::index::ChangeRef::Rewrite { .. } => unreachable!("rewrite tracking is disabled"),
                };
                pending.insert(change.location().to_owned(), item);
                Ok(std::ops::ControlFlow::Continue(()))
            },
            None,
            &mut gix_pathspec::Search::from_specs([], None, workdir)?,
            &mut |_, _, _, _| false,
        )?;
        if options.overwrite_tracked {
            for (position, entry) in baseline.entries().iter().enumerate() {
                let path = entry.path(baseline);
                if let Some(target_position) = target.entry_index_by_path_and_stage(path, Stage::Unconflicted) {
                    pending.entry(path.to_owned()).or_insert(Pending {
                        from: Some(position),
                        to: Some(target_position),
                        changed: false,
                        skipped: false,
                    });
                }
            }
        }
        let mut check = baseline.clone();
        for entry in check.entries_mut() {
            entry
                .flags
                .remove(Flags::ASSUME_VALID | Flags::UPTODATE | Flags::FSMONITOR_VALID);
        }
        for (path, item) in &mut pending {
            let path = path.as_bstr();
            item.skipped = item
                .to
                .is_some_and(|position| target.entries()[position].flags.contains(Flags::SKIP_WORKTREE))
                || index
                    .entry_by_path(path)
                    .is_some_and(|entry| entry.flags.contains(Flags::SKIP_WORKTREE))
                || item
                    .from
                    .is_some_and(|position| baseline.entries()[position].flags.contains(Flags::SKIP_WORKTREE));
            if let Some(position) = item.from {
                let disk = inspect_path(workdir, path)?;
                let blocked = matches!(disk, Disk::Blocked(_));
                if item.skipped || blocked {
                    check.entries_mut()[position].flags |= Flags::UPTODATE;
                }
                if blocked && !item.skipped && options.overwrite_tracked {
                    item.changed = true;
                }
            }
        }
        let mut recorded = gix_status::index_as_worktree_with_renames::Recorder { records: Vec::new() };
        if !pending.is_empty() {
            repo.index_worktree_status(
                &check,
                pending.keys().map(|path| literal(path.as_bstr())),
                &mut recorded,
                gix_status::index_as_worktree::traits::HashEq,
                NoSubmodules,
                &mut progress.add_child("checking worktree"),
                should_interrupt,
                crate::status::index_worktree::Options {
                    dirwalk_options: None,
                    ..Default::default()
                },
            )?;
        }
        let mut check_against_target = Vec::new();
        for record in recorded.records {
            let gix_status::index_as_worktree_with_renames::Entry::Modification {
                entry,
                rela_path,
                status,
                ..
            } = record
            else {
                unreachable!("directory walking and rewrites are disabled")
            };
            let Some(item) = pending.get_mut(rela_path) else {
                continue;
            };
            if item.skipped {
                continue;
            }
            match status {
                EntryStatus::NeedsUpdate(_) => {}
                EntryStatus::Change(change) => {
                    if options.overwrite_tracked {
                        item.changed = true;
                        continue;
                    }
                    let safe = match change {
                        StatusChange::Removed => item.to.is_none(),
                        StatusChange::Modification {
                            executable_bit_changed,
                            content_change,
                            ..
                        } => item.to.is_some_and(|position| {
                            let desired = &target.entries()[position];
                            let mode = if executable_bit_changed {
                                if entry.mode == Mode::FILE {
                                    Mode::FILE_EXECUTABLE
                                } else {
                                    Mode::FILE
                                }
                            } else {
                                entry.mode
                            };
                            desired.mode == mode && desired.id == content_change.unwrap_or(entry.id)
                        }),
                        StatusChange::SubmoduleModification(()) => true,
                        StatusChange::Type { .. } => false,
                    };
                    if !safe && item.to.is_some() {
                        check_against_target.push(rela_path.to_owned());
                        continue;
                    }
                    ensure!(
                        safe,
                        conflict(rela_path, "Tracked worktree content would be overwritten")
                    );
                }
                EntryStatus::Conflict { .. } | EntryStatus::IntentToAdd => {
                    bail!(conflict(rela_path, "Checkout requires resolved baseline content"))
                }
            }
        }
        if !check_against_target.is_empty() {
            // Type changes, including emulated symlinks, can require different content filters on each side.
            check.clone_from(target);
            for path in &check_against_target {
                let entry = check
                    .entry_mut_by_path_and_stage(path.as_bstr(), Stage::Unconflicted)
                    .expect("fallback checks have a target entry");
                entry
                    .flags
                    .remove(Flags::ASSUME_VALID | Flags::UPTODATE | Flags::FSMONITOR_VALID);
                entry.stat = Default::default();
            }
            let mut recorded = gix_status::index_as_worktree_with_renames::Recorder { records: Vec::new() };
            repo.index_worktree_status(
                &check,
                check_against_target.iter().map(|path| literal(path.as_bstr())),
                &mut recorded,
                gix_status::index_as_worktree::traits::HashEq,
                NoSubmodules,
                &mut progress.add_child("checking target content"),
                should_interrupt,
                crate::status::index_worktree::Options {
                    dirwalk_options: None,
                    ..Default::default()
                },
            )?;
            for record in recorded.records {
                if let gix_status::index_as_worktree_with_renames::Entry::Modification { rela_path, status, .. } =
                    record
                {
                    ensure!(
                        matches!(status, EntryStatus::NeedsUpdate(_)),
                        conflict(rela_path, "Tracked worktree content would be overwritten")
                    );
                }
            }
        }
        pending.retain(|_, item| item.changed);
        let selected_paths = pending
            .iter()
            .filter(|(_, item)| item.to.is_some())
            .map(|(path, _)| (path_key(path.as_bstr(), ignore_case), path.as_bstr()))
            .collect::<BTreeMap<_, _>>();
        let mut staged = index.clone();
        let mut staging_conflict = None;
        staged.remove_entries(|_, path, entry| {
            if pending.contains_key(path) {
                if entry.stage() != Stage::Unconflicted {
                    staging_conflict = Some(path.to_owned());
                }
                return true;
            }
            if clashing_path(&selected_paths, path_key(path, ignore_case).as_bstr()).is_some() {
                staging_conflict = Some(path.to_owned());
            }
            false
        });
        if let Some(path) = staging_conflict {
            bail!(conflict(path.as_bstr(), "An unrelated staged entry would be replaced"));
        }
        let lookup = ignore_case.then(|| baseline.prepare_icase_backing());
        let owned_entry = |path: &BStr| {
            baseline.entry_by_path(path).or_else(|| {
                lookup
                    .as_ref()
                    .and_then(|lookup| baseline.entry_by_path_icase(path, true, lookup))
            })
        };
        let mut changes = Vec::new();
        let mut obstructions = Vec::new();
        for (path, item) in &pending {
            let path = path.as_bstr();
            if item.skipped {
                continue;
            }
            let disk = inspect_path(workdir, path)?;
            match item.to {
                None => {
                    if let Disk::Present(metadata) = disk
                        && (!metadata.is_dir()
                            || metadata.is_symlink()
                            || (item
                                .from
                                .is_some_and(|position| baseline.entries()[position].mode == Mode::COMMIT)
                                && std::fs::read_dir(workdir.join(gix_path::from_bstr(path)?))
                                    .or_raise(|| message!("Could not inspect removed gitlink {path:?}"))?
                                    .next()
                                    .transpose()
                                    .or_raise(|| message!("Could not read removed gitlink {path:?}"))?
                                    .is_none()))
                    {
                        changes.push(Change::Remove { path: path.to_owned() });
                    }
                }
                Some(position) => {
                    let desired = &target.entries()[position];
                    match disk {
                        Disk::Missing => {}
                        Disk::Blocked(parent) => {
                            if !owned_entry(parent.as_bstr()).is_some_and(|entry| {
                                pending
                                    .get(entry.path(baseline))
                                    .is_some_and(|item| item.to.is_none() && !item.skipped)
                            }) {
                                ensure!(
                                    options.overwrite_ignored,
                                    conflict(parent.as_bstr(), "Untracked parent would be overwritten")
                                );
                                obstructions.push(parent);
                            }
                        }
                        Disk::Present(metadata) => {
                            if metadata.is_dir() && !metadata.is_symlink() {
                                if desired.mode != Mode::COMMIT {
                                    obstructions.push(path.to_owned());
                                }
                            } else {
                                ensure!(
                                    metadata.is_file() || metadata.is_symlink(),
                                    conflict(path, "Unsupported worktree file type")
                                );
                                if let Some(old) = owned_entry(path) {
                                    if (metadata.is_symlink() || matches!(desired.mode, Mode::SYMLINK | Mode::COMMIT))
                                        && !pending
                                            .get(old.path(baseline))
                                            .is_some_and(|item| item.to.is_none() && !item.skipped)
                                    {
                                        changes.push(Change::Remove { path: path.to_owned() });
                                    }
                                } else {
                                    ensure!(
                                        options.overwrite_ignored,
                                        conflict(path, "Untracked worktree content would be overwritten")
                                    );
                                    obstructions.push(path.to_owned());
                                }
                            }
                        }
                    }
                    changes.push(Change::Write { entry_index: position });
                }
            }
        }
        if !obstructions.is_empty() {
            check.clone_from(baseline);
            // Removed gitlinks must be classified from disk instead of acting as repository boundaries.
            check.remove_entries(|_, path, entry| {
                entry.mode == Mode::COMMIT
                    && pending.get(path).is_some_and(|item| {
                        item.to
                            .is_none_or(|position| target.entries()[position].mode != Mode::COMMIT)
                    })
            });
            let mut collect = gix_dir::walk::delegate::Collect::default();
            repo.dirwalk(
                &check,
                obstructions.iter().map(|path| literal(path.as_bstr())),
                should_interrupt,
                repo.dirwalk_options()?
                    .for_deletion(
                        options
                            .overwrite_ignored
                            .then_some(gix_dir::walk::ForDeletionMode::FindRepositoriesInIgnoredDirectories),
                    )
                    .classify_untracked_bare_repositories(options.overwrite_ignored)
                    .emit_tracked(true)
                    .emit_ignored(Some(gix_dir::walk::EmissionMode::Matching))
                    .emit_empty_directories(true),
                &mut collect,
            )?;
            for (entry, _) in collect.into_entries_by_path() {
                if entry.disk_kind == Some(gix_dir::entry::Kind::Directory) {
                    if entry.property == Some(gix_dir::entry::Property::EmptyDirectory) {
                        changes.push(Change::Remove { path: entry.rela_path });
                    }
                    continue;
                }
                ensure!(
                    entry.disk_kind != Some(gix_dir::entry::Kind::Repository),
                    conflict(entry.rela_path.as_bstr(), "A nested repository would be overwritten")
                );
                let owned = owned_entry(entry.rela_path.as_bstr()).is_some_and(|old| {
                    pending
                        .get(old.path(baseline))
                        .is_some_and(|item| item.to.is_none() && !item.skipped)
                });
                if !owned
                    && options.overwrite_ignored
                    && entry.status == gix_dir::entry::Status::Ignored(gix_ignore::Kind::Expendable)
                {
                    changes.push(Change::Remove { path: entry.rela_path });
                    continue;
                }
                ensure!(
                    owned,
                    conflict(
                        entry.rela_path.as_bstr(),
                        "Untracked directory content would be overwritten"
                    )
                );
            }
        }
        interrupted(should_interrupt)?;
        let worktree = gix_worktree_state::checkout::apply_changes(
            target,
            &changes,
            workdir,
            repo.objects
                .clone()
                .into_arc()
                .or_raise(|| message("Could not share the checkout object database"))?,
            &progress.add_child("checkout"),
            &progress.add_child("writing"),
            should_interrupt,
            checkout_options,
        )?;
        interrupted(should_interrupt)?;
        ensure!(
            worktree.collisions.is_empty() && worktree.errors.is_empty(),
            validation(format!(
                "Checkout failed with collisions: {:?}, errors: {:?}",
                worktree.collisions, worktree.errors
            ))
        );
        let index_changed = !pending.is_empty();
        if index_changed {
            for (path, item) in &pending {
                if let Some(position) = item.to {
                    let entry = &target.entries()[position];
                    let mut flags = entry.flags;
                    if item.skipped {
                        flags |= Flags::EXTENDED | Flags::SKIP_WORKTREE;
                    }
                    staged.dangerously_push_entry(entry.stat, entry.id, flags, entry.mode, path.as_bstr());
                }
            }
            staged.sort_entries();
            staged.remove_tree();
            staged.remove_resolve_undo();
            *index = staged;
        }
        Ok(crate::worktree::checkout::Outcome {
            worktree,
            index_changed,
        })
    }
}

struct Pending {
    from: Option<usize>,
    to: Option<usize>,
    changed: bool,
    skipped: bool,
}
impl Pending {
    fn new(from: Option<usize>, to: Option<usize>) -> Self {
        Self {
            from,
            to,
            changed: true,
            skipped: false,
        }
    }
}

fn literal(path: &BStr) -> BString {
    let mut pattern = BString::from(":(top,literal)");
    pattern.extend_from_slice(path);
    pattern
}

fn path_key(path: &BStr, ignore_case: bool) -> BString {
    if ignore_case {
        path.to_ascii_lowercase().into()
    } else {
        path.into()
    }
}

fn parents(path: &BStr) -> impl Iterator<Item = &BStr> {
    path.iter()
        .enumerate()
        .filter_map(|(position, byte)| (*byte == b'/').then_some(path[..position].as_bstr()))
}

fn clashing_path<'a>(paths: &'a BTreeMap<BString, &BStr>, path: &BStr) -> Option<&'a BStr> {
    if let Some(found) = paths.get(path) {
        return Some(*found);
    }
    if let Some(found) = parents(path).find_map(|parent| paths.get(parent)) {
        return Some(*found);
    }
    let mut prefix = path.to_owned();
    prefix.push(b'/');
    paths
        .range(prefix.clone()..)
        .next()
        .filter(|(path, _)| path.starts_with(&prefix))
        .map(|(_, path)| *path)
}

fn conflict(path: &BStr, message: &str) -> gix_error::Message {
    validation(format!("{message}: {path:?}")).with_input(path)
}

fn interrupted(flag: &AtomicBool) -> Result {
    ensure!(
        !flag.load(Ordering::Relaxed),
        crate::error::cancelled("Checkout was interrupted")
    );
    Ok(())
}

fn validate_path(path: &BStr, mode: Mode, options: gix_worktree::validate::path::component::Options) -> Result {
    let mut parts = path.split(|byte| *byte == b'/').peekable();
    while let Some(part) = parts.next() {
        gix_worktree::validate::path::component(
            part.as_bstr(),
            (parts.peek().is_none() && mode == Mode::SYMLINK)
                .then_some(gix_worktree::validate::path::component::Mode::Symlink),
            options,
        )
        .or_error()?;
    }
    Ok(())
}

enum Disk {
    Missing,
    Blocked(BString),
    Present(gix_index::fs::Metadata),
}

fn inspect_path(root: &Path, path: &BStr) -> Result<Disk> {
    for end in path
        .iter()
        .enumerate()
        .filter_map(|(position, byte)| (*byte == b'/').then_some(position))
        .chain(std::iter::once(path.len()))
    {
        let prefix = path[..end].as_bstr();
        let absolute = root.join(gix_path::from_bstr(prefix)?);
        let metadata = match gix_index::fs::Metadata::from_path_no_follow(&absolute) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Disk::Missing),
            Err(err) => return Err(err.and_raise(message!("Could not inspect checkout path {prefix:?}"))),
        };
        if end == path.len() {
            return Ok(Disk::Present(metadata));
        }
        if !metadata.is_dir() || metadata.is_symlink() {
            return Ok(Disk::Blocked(prefix.into()));
        }
    }
    unreachable!("the terminal component is always inspected")
}

#[derive(Clone)]
struct NoSubmodules;
impl gix_status::index_as_worktree::traits::SubmoduleStatus for NoSubmodules {
    type Output = ();
    fn status(&mut self, _entry: &gix_index::Entry, _path: &BStr) -> Result<Option<()>> {
        Ok(None)
    }
}
