use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Instant,
};

use gix_error::{ErrorExt, ResultExt, message};

use super::{
    Changes, Event, EventKind, Layout, Options, RepositoryMonitor, Scope, Statistics, WorktreeChange, WorktreeChanges,
    absolute_path, same_path, strip_prefix,
};
use crate::{Error, Repository, Result};

impl RepositoryMonitor {
    /// Layouts of additional checkouts discovered by the most recent successful maintenance.
    ///
    /// Empty until the first service call and when [`Options::all_worktrees`] is disabled.
    /// Temporarily inaccessible checkouts remain listed while their coverage is retried.
    pub fn other_worktrees(&self) -> impl Iterator<Item = &Layout> {
        self.other_worktrees.values().map(|other| &other.layout)
    }

    pub(super) fn all_worktrees_enabled(&self) -> bool {
        self.options.all_worktrees && self.options.worktree_coverage.is_some()
    }

    pub(super) fn metadata_is_shared(&self) -> bool {
        self.shared_metadata || self.all_worktrees_enabled()
    }

    pub(super) fn take_changes(&mut self, now: Instant) -> Changes {
        let mut changes = self.pending_metadata.take_due(now);
        changes.merge(std::mem::take(&mut self.pending_worktree));
        if !self.options.metadata {
            changes = Changes {
                worktree: changes.worktree,
                ..Changes::default()
            };
        }
        if let Some(Scope::Paths(paths)) = &mut changes.worktree {
            paths.sort_by(|a, b| a.path.cmp(&b.path));
        }
        changes
    }

    pub(super) fn worktree_change(&self, kind: WorktreeChange) -> Option<WorktreeChanges> {
        Some(WorktreeChanges {
            git_dir: self.layout.git_dir.clone(),
            worktree: self.layout.worktree.clone()?,
            kind,
        })
    }

    pub(super) fn observe_index(&mut self, event: &Event, now: Instant) {
        if !self.worktree_enabled() {
            return;
        }
        let mut index_lock = self.layout.index.as_os_str().to_owned();
        index_lock.push(".lock");
        for path in &event.paths {
            let path = if self.precompose_unicode {
                gix_utils::str::precompose_path(path.as_path().into())
            } else {
                path.as_path().into()
            };
            if same_path(&path, &self.layout.index, self.ignore_case)
                || event.kind == EventKind::Rename && same_path(&path, Path::new(&index_lock), self.ignore_case)
            {
                self.pending_worktree.index = true;
                self.index_dirty = true;
                self.maintenance = Some(now);
            }
        }
    }

    pub(super) fn maintain_other_worktrees(
        &mut self,
        repo: &Repository,
        now: Instant,
        statistics: &mut Statistics,
        errors: &mut Vec<Error>,
    ) -> Result<()> {
        // List before reconciling: a failed enumeration must not look like every checkout disappeared.
        let linked = repo
            .worktrees()
            .or_raise(|| message("could not enumerate worktrees for monitoring"))?;
        let mut seen = HashSet::new();
        let main_git_dir = absolute_path(repo.common_dir(), repo.current_dir(), self.precompose_unicode)?;
        if !same_path(&main_git_dir, &self.layout.git_dir, self.ignore_case) {
            seen.insert(main_git_dir.clone());
            if self.other_needs_maintenance(&main_git_dir, now) {
                self.maintain_other(main_git_dir, repo.main_repo(), now, statistics, errors);
            }
        }
        for proxy in linked {
            let git_dir = absolute_path(proxy.git_dir(), repo.current_dir(), self.precompose_unicode)?;
            if same_path(&git_dir, &self.layout.git_dir, self.ignore_case) {
                continue;
            }
            seen.insert(git_dir.clone());
            if self.other_needs_maintenance(&git_dir, now) {
                // Do not let the permissive opener silently suppress an unreadable registration.
                let other = proxy
                    .base()
                    .and_then(|_| proxy.into_repo_with_possibly_inaccessible_worktree());
                self.maintain_other(git_dir, other, now, statistics, errors);
            }
        }
        let missing: Vec<_> = self
            .other_worktrees
            .keys()
            .filter(|key| !seen.contains(*key))
            .cloned()
            .collect();
        self.undiscovered_worktrees.retain(|git_dir, _| seen.contains(git_dir));
        for git_dir in missing {
            let exists = git_dir.try_exists();
            if !matches!(exists, Ok(false)) && !self.other_needs_maintenance(&git_dir, now) {
                continue;
            }
            match exists {
                Ok(false) => {
                    if let Some(other) = self.other_worktrees.remove(&git_dir) {
                        statistics.removed += other.worktree.registered.len();
                        self.retired_worktrees
                            .extend(other.worktree_change(WorktreeChange::Removed));
                    }
                }
                Ok(true) => errors.push(
                    message!(
                        "worktree registration in {} is incomplete; retaining its coverage",
                        git_dir.display()
                    )
                    .raise(),
                ),
                Err(error) => errors.push(error.and_raise(message!(
                    "could not inspect worktree registration in {}",
                    git_dir.display()
                ))),
            }
            if let Some(other) = self.other_worktrees.get_mut(&git_dir) {
                other.degrade_metadata(now);
            }
        }
        // Child-private inventories are installed by the parent, including when only a child changed.
        self.metadata_dirty = true;
        Ok(())
    }

    fn other_needs_maintenance(&self, git_dir: &Path, now: Instant) -> bool {
        self.other_worktrees.get(git_dir).map_or_else(
            || {
                self.undiscovered_worktrees
                    .get(git_dir)
                    .is_none_or(|deadline| *deadline <= now)
            },
            |other| other.maintenance.is_some_and(|deadline| deadline <= now),
        )
    }

    fn degrade_metadata(&mut self, now: Instant) {
        self.maintenance_failed(now);
        self.metadata_dirty = true;
        self.inventory_dirty = true;
        self.index_dirty = true;
        // A vanished checkout must not poison the shared union with its old registration paths.
        self.metadata.registered = super::inventory::private_metadata_watches(&self.layout);
        if self.worktree_enabled() {
            super::inventory::watch_parent(&self.layout, &mut self.metadata.registered, &self.layout.index);
        }
        self.maintenance = Some(now + self.options.retry_interval);
    }

    fn maintain_other(
        &mut self,
        git_dir: PathBuf,
        repo: Result<Repository>,
        now: Instant,
        statistics: &mut Statistics,
        errors: &mut Vec<Error>,
    ) {
        let result = (|| -> Result<()> {
            let repo = repo?;
            if repo.workdir().is_none() {
                if self.other_worktrees.contains_key(&git_dir)
                    || !same_path(&git_dir, &self.layout.common_dir, self.ignore_case)
                {
                    return Err(message!("could not resolve the checkout for {}", git_dir.display()).raise());
                }
                self.undiscovered_worktrees.remove(&git_dir);
                return Ok(());
            }
            let other = match self.other_worktrees.entry(git_dir.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let mut other = repo.monitor(Options {
                        all_worktrees: false,
                        ..self.options.clone()
                    })?;
                    other.shared_metadata = true;
                    other.worktree_transition = Some(WorktreeChange::Added);
                    entry.insert(other)
                }
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            };
            let layout = Layout::for_options(&repo, &other.options)?;
            if other.layout.worktree != layout.worktree {
                if let Some(previous_worktree) = other.layout.worktree.clone() {
                    other.worktree_transition = Some(WorktreeChange::Moved { previous_worktree });
                }
                // Do not relabel pending old-root paths as paths in the new checkout.
                other.pending_metadata = super::Pending::default();
                other.pending_worktree = Changes::default();
            }
            other.maintenance = None;
            other.maintain(&repo, now, statistics, errors)?;
            self.undiscovered_worktrees.remove(&git_dir);
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(error.and_raise(message!(
                "could not maintain worktree monitoring for {}",
                git_dir.display()
            )));
            if let Some(other) = self.other_worktrees.get_mut(&git_dir) {
                other.degrade_metadata(now);
            } else {
                self.undiscovered_worktrees
                    .insert(git_dir, now + self.options.retry_interval);
            }
        }
    }
}

pub(super) fn is_private_metadata(path: &Path, git_dir: &Path, ignore_case: bool) -> bool {
    let Some(relative) = strip_prefix(path, git_dir, ignore_case) else {
        return false;
    };
    if relative.as_os_str().is_empty() {
        return true;
    }
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = Path::new(first.as_os_str());
    [
        "HEAD",
        "index",
        "index.lock",
        "config.worktree",
        "commondir",
        "gitdir",
        "ORIG_HEAD",
        "FETCH_HEAD",
        "MERGE_HEAD",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "AUTO_MERGE",
        "MERGE_MSG",
        "MERGE_MODE",
        "SQUASH_MSG",
        "BISECT_LOG",
        "BISECT_START",
        "rebase-apply",
        "rebase-merge",
        "sequencer",
    ]
    .iter()
    .any(|name| same_path(first, Path::new(name), ignore_case))
        || ["refs/bisect", "refs/rewritten", "refs/worktree"]
            .iter()
            .any(|prefix| strip_prefix(relative, Path::new(prefix), ignore_case).is_some())
}
