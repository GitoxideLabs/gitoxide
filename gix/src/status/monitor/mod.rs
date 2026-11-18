//! Per-worktree status subscriptions for a [`RepositoryMonitor`](crate::notify::RepositoryMonitor).
//!
//! Service the project monitor to discover worktrees and deliver changes to their handles.
//! The initial refresh collects a full snapshot. Keep handles for dirty checkouts and drop
//! them when the snapshot becomes empty. The project keeps observing discovered submodules,
//! so a commit inside a clean checkout's submodule can trigger a fresh handle.
//!
//! ```no_run
//! # fn example(repo: &gix::Repository) -> gix::Result<()> {
//! use std::{sync::atomic::AtomicBool, time::Instant};
//! let mut project = repo.monitor(gix::notify::Options { all_worktrees: true, ..Default::default() })?;
//! let outcome = project.service(Instant::now(), || Ok(repo.clone()));
//! if let Some(error) = outcome.errors.into_iter().next() { return Err(error); }
//! let mut dirty = Vec::new();
//! for layout in std::iter::once(project.layout()).chain(project.other_worktrees()) {
//!     let checkout = project.open_worktree(&layout.git_dir)?;
//!     let mut status = project.worktree_status(&checkout, Default::default())?;
//!     status.refresh(&checkout, &AtomicBool::new(false))?;
//!     if status.snapshot().is_some_and(|items| !items.is_empty()) {
//!         dirty.push(status);
//!     }
//! }
//! # Ok(()) }
//! ```
//!
//! Use [`RepositoryMonitor::open_worktree()`](crate::notify::RepositoryMonitor::open_worktree) with a Git directory from
//! [`Outcome::other_worktrees`](crate::notify::Outcome::other_worktrees).
//! To drive all retained handles in one event loop, pass `dirty.iter_mut()` to
//! [`RepositoryMonitor::service_with_status()`](crate::notify::RepositoryMonitor::service_with_status).
//! Use [`RepositoryMonitor::next_timeout_with_status()`](crate::notify::RepositoryMonitor::next_timeout_with_status)
//! or, on Tokio, `ready_with_status_async(dirty.iter())` for its next wakeup.

use std::{
    collections::{BTreeMap, HashSet},
    ops::Bound,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, Result, ResultExt, bail, message};
use gix_parallel::{OwnShared, lock};

use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
    notify::{Changes, CoverageStatus, Layout, RepositoryMonitor, Scope, status::Subscription},
    status::{Item, Submodule, UntrackedFiles},
};

const CHILDREN_PER_SERVICE: usize = 8;
const MAX_ERRORS: usize = 8;

#[cfg(feature = "async-notify")]
mod asynchronous;
mod delta;
pub use delta::Change;
mod index;
mod rewrites;
pub(crate) mod submodules;
#[cfg(test)]
mod tests;

/// Control the contents of one worktree's cached status snapshot.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// How untracked paths are represented.
    pub untracked_files: UntrackedFiles,
    /// Which submodule changes contribute to status.
    pub submodules: Submodule,
    /// How staged rename/copy detection is configured. Defaults to repository configuration.
    pub tree_index_track_renames: crate::status::tree_index::TrackRenames,
    /// Optional worktree rename/copy detection, recomputed over retained status candidates.
    /// Scoped refreshes check changed paths and rehash untracked candidates without walking clean tracked files.
    pub worktree_rewrites: Option<gix_diff::Rewrites>,
    /// Limit the number of threads used for tracked worktree status.
    ///
    /// `None` or `Some(0)` uses all available parallelism.
    pub thread_limit: Option<usize>,
}

/// Changes since the previous successful snapshot and the coverage recomputed this round.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Update {
    /// Whether the owned status items changed.
    pub changed: bool,
    /// Exact additions, deletions and modifications, ordered by staged/worktree side, path and origin.
    ///
    /// The initial snapshot reports all items as additions. The initial empty snapshot has
    /// `changed = true` but no item changes. Content-derived caches must also use refresh coverage.
    pub changes: Vec<Change>,
    /// Whether staged changes were recomputed.
    pub staged: bool,
    /// The worktree paths recomputed, including any conservative scope expansion.
    ///
    /// Consumers with content-derived caches must refresh these paths even when `changed` is false.
    pub unstaged: Option<Scope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Head {
    reference: Option<gix_ref::FullName>,
    commit_id: Option<gix_hash::ObjectId>,
}

impl Head {
    fn read(repo: &Repository) -> Result<Self> {
        let mut head = repo
            .head()
            .or_raise(|| message("could not read HEAD for cached status"))?;
        let reference = head.referent_name().map(ToOwned::to_owned);
        let commit_id = head
            .try_peel_to_id()
            .or_raise(|| message("could not resolve HEAD for cached status"))?
            .map(crate::Id::detach);
        Ok(Self { reference, commit_id })
    }

    fn tree_id(&self, repo: &Repository) -> Result<gix_hash::ObjectId> {
        match self.commit_id {
            Some(commit_id) => Ok(repo
                .find_commit(commit_id)
                .or_raise(|| message("could not read captured HEAD commit for cached status"))?
                .tree_id()
                .or_raise(|| message("could not read captured HEAD tree for cached status"))?
                .into()),
            None => Ok(gix_hash::ObjectId::empty_tree(repo.object_hash())),
        }
    }
}

struct HeadTreeIndex {
    tree_id: gix_hash::ObjectId,
    protect: gix_validate::path::component::Options,
    index: gix_index::State,
}

/// An owned status cache and subscription for one checkout and reference namespace.
/// Create it with [`RepositoryMonitor::worktree_status()`]. Dropping it releases its cached status
/// and index comparisons. The project continues observing the checkout and its discovered submodules.
///
/// Call [`refresh()`](Self::refresh) for the first snapshot. Service the project monitor to deliver
/// subsequent invalidations directly to this handle. Use [`service()`](Self::service) to drain
/// nested submodule notifications and [`refresh_due()`](Self::refresh_due) to honor retry backoff.
/// With `async-notify`, `ready_async()` awaits this checkout's invalidations and service deadlines.
///
/// The handle retains detached status data. Failed or interrupted refreshes
/// preserve the last successful snapshot and pending work.
pub struct WorktreeStatusHandle {
    pub(crate) subscription: OwnShared<Subscription>,
    notifications: crate::notify::Options,
    options: Options,
    snapshot: Option<Vec<Item>>,
    unrewritten_snapshot: Option<Vec<Item>>,
    index: Option<index::Snapshot>,
    index_stamp: Option<(Option<gix_hash::ObjectId>, i64, u32)>,
    index_dirty: bool,
    head: Option<Head>,
    head_tree_index: Option<HeadTreeIndex>,
    head_dirty: bool,
    staged: bool,
    unstaged: Option<Scope>,
    pub(crate) watches: submodules::Watches,
    retry: Option<Instant>,
    submodules_dirty: bool,
}

impl Drop for WorktreeStatusHandle {
    fn drop(&mut self) {
        Subscription::retain_watches(
            &self.subscription,
            &mut self.watches,
            self.options.submodules,
            self.submodules_dirty.then(|| self.retry.unwrap_or_else(Instant::now)),
        );
    }
}

impl WorktreeStatusHandle {
    pub(crate) fn new(
        options: Options,
        notifications: &crate::notify::Options,
        subscription: OwnShared<Subscription>,
    ) -> Self {
        Self {
            subscription,
            notifications: crate::notify::Options {
                all_worktrees: false,
                worktree_filter: None,
                checkout_filter: None,
                additional_paths: Vec::new(),
                ..*notifications
            },
            options,
            snapshot: None,
            unrewritten_snapshot: None,
            index: None,
            index_stamp: None,
            index_dirty: true,
            head: None,
            head_tree_index: None,
            head_dirty: true,
            staged: true,
            unstaged: Some(Scope::All),
            watches: submodules::Watches::default(),
            retry: None,
            submodules_dirty: true,
        }
    }

    /// The most recent successful snapshot, while this subscription is open, including during a retry.
    /// Directory entries describe the complete snapshot, so their pathspec match is always `Always`.
    pub fn snapshot(&self) -> Option<&[Item]> {
        self.snapshot.as_deref().filter(|_| !self.is_closed())
    }

    /// The HEAD commit used by the most recent successful snapshot.
    ///
    /// Returns `None` before a snapshot exists or when its HEAD is unborn. Dirty or failed refreshes
    /// retain the previous committed baseline. A cached, unchanged refresh does not relabel status
    /// with the repository's current HEAD; compare this ID before using cached status with new reads.
    pub fn snapshot_head_id(&self) -> Option<gix_hash::ObjectId> {
        self.head
            .as_ref()
            .filter(|_| !self.is_closed())
            .and_then(|head| head.commit_id)
    }

    /// Whether the snapshot needs a refresh. To check for worktree changes, inspect the
    /// entries in the successful [`snapshot()`](Self::snapshot).
    pub fn needs_refresh(&self) -> bool {
        let pending = lock(&self.subscription.pending);
        !pending.closed
            && (!pending.changes.is_empty()
                || self.snapshot.is_none()
                || self.head_dirty
                || self.index_dirty
                || self.staged
                || self.unstaged.is_some()
                || self.submodules_dirty)
    }

    /// Whether the project monitor was dropped or ceased monitoring this checkout at this location.
    /// Drop a closed handle. A moved or rediscovered checkout needs a new baseline.
    pub fn is_closed(&self) -> bool {
        lock(&self.subscription.pending).closed
    }

    /// Whether this handle needs a refresh and its retry interval has elapsed.
    pub fn refresh_due(&self, now: Instant) -> bool {
        self.needs_refresh() && self.retry.is_none_or(|deadline| deadline <= now)
    }

    /// The checkout and index this handle was created for.
    pub fn layout(&self) -> &Layout {
        &self.subscription.layout
    }

    /// Whether root and child subscriptions are verified and no submodule reconciliation is pending.
    ///
    /// This is a positive recovery signal for previously reported monitoring errors. A service
    /// call without new errors can still be waiting for a failed subscription's retry deadline.
    pub fn is_healthy(&self) -> bool {
        matches!(
            self.coverage_status(),
            CoverageStatus::Healthy | CoverageStatus::Inactive
        )
    }

    /// Current coverage across the root and its submodules, including pending reconciliation.
    ///
    /// Inspect this after processing subscriptions or refreshing status: those operations can
    /// change coverage after the last service outcome was produced.
    pub fn coverage_status(&self) -> CoverageStatus {
        if self.is_closed() {
            return CoverageStatus::Inactive;
        }
        let root = lock(&self.subscription.pending).coverage;
        let coverage = if self.status_enabled() && self.submodules_dirty {
            root.combine(CoverageStatus::Updating)
        } else {
            root
        };
        self.watches.coverage(coverage)
    }

    /// The options used to collect this worktree's status.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Change the selected checkout's submodule status policy without replacing project monitoring.
    ///
    /// Changed policies invalidate all worktree status and reconcile submodule coverage on refresh.
    /// Equal policies leave the cache and pending retries unchanged.
    pub fn set_submodules(&mut self, submodules: Submodule) {
        if self.options.submodules != submodules {
            self.options.submodules = submodules;
            self.unstaged = Some(Scope::All);
            self.submodules_dirty = true;
            self.retry = None;
        }
    }

    fn status_enabled(&self) -> bool {
        let pending = lock(&self.subscription.pending);
        pending.covered && !pending.closed
    }

    fn take_changes(&mut self) -> Changes {
        let changes = {
            let mut pending = lock(&self.subscription.pending);
            pending.coverage_changed = false;
            std::mem::take(&mut pending.changes)
        };
        self.invalidate_changes(&changes);
        changes
    }

    /// Invalidate this snapshot and rebuild its submodule watches. Use [`RepositoryMonitor::rescan()`]
    /// to also rebuild the project's shared watches.
    pub fn rescan(&mut self) {
        self.watches.rescan();
        self.invalidate();
    }

    /// Mark the complete snapshot and submodule inventory for the next [`refresh()`](Self::refresh).
    ///
    /// Use this to request a full status refresh when you cannot reliably describe what changed,
    /// for example after an external Git operation that doesn't report which paths or metadata it affected.
    /// If you know what changed, prefer [`invalidate_changes()`](Self::invalidate_changes) to limit the work.
    /// Use [`RepositoryMonitor::rescan()`] to rebuild the project's native subscriptions.
    ///
    /// The last successful [`snapshot()`](Self::snapshot) remains available until a refresh succeeds.
    pub fn invalidate(&mut self) {
        self.head_tree_index = None;
        self.retry = None;
        self.staged = true;
        self.index_dirty = true;
        self.unstaged = Some(Scope::All);
        self.head_dirty = true;
        self.submodules_dirty = true;
    }

    /// Mark known changes for the next [`refresh()`](Self::refresh).
    ///
    /// Use this after an application-owned operation if you want to refresh before
    /// [`service()`](Self::service) processes its filesystem notifications. For example, after
    /// staging a file, pass `Changes { index: true, ..Default::default() }` before calling
    /// [`refresh()`](Self::refresh). Otherwise, if the monitor was clean, `refresh()` can leave
    /// the previous snapshot unchanged until the notification is processed.
    ///
    /// You do not need to call this if you can wait for `service()` to report the change.
    /// You can also use it to forward changes from sources the monitor does not watch.
    ///
    /// Use [`invalidate()`](Self::invalidate) if you cannot reliably describe what changed.
    /// The monitor may broaden the reported scope: for example, native Unicode aliases of
    /// configured submodule paths require a full worktree refresh.
    pub fn invalidate_changes(&mut self, changes: &Changes) {
        if changes.configuration || changes.layout {
            self.head_tree_index = None;
        }
        self.head_dirty |= changes.references;
        self.index_dirty |= changes.index || changes.configuration || changes.layout;
        if changes.configuration || changes.layout {
            self.staged = true;
            self.unstaged = Some(Scope::All);
        } else {
            self.staged |= changes.attributes;
            let mut scope = changes.worktree.clone();
            if changes.ignores || changes.attributes {
                match &mut scope {
                    Some(Scope::Paths(paths))
                        if paths.iter().any(|change| is_submodule_file(change.path.as_bstr())) =>
                    {
                        scope = Some(Scope::All);
                    }
                    Some(Scope::Paths(paths)) => {
                        for change in paths {
                            if is_rule_file(change.path.as_bstr()) {
                                change.path = parent_scope(change.path.as_bstr());
                            }
                        }
                    }
                    None => scope = Some(Scope::All),
                    Some(Scope::All) => {}
                }
            }
            merge_scopes(&mut self.unstaged, scope);
        }
        if let Some(Scope::Paths(paths)) = &changes.worktree
            && paths.iter().any(|path| {
                self.watches
                    .paths
                    .iter()
                    .any(|module| unicode_path_alias(path.path.as_bstr(), module.as_bstr()))
            })
        {
            // A native Unicode alias cannot select the gitlink's literal index spelling.
            self.unstaged = Some(Scope::All);
        }
        self.submodules_dirty |= self.watches.topology_changed(changes);
    }

    /// Consume this checkout's invalidations and drain a bounded batch of nested submodule notifications.
    /// Service the project monitor separately to publish its changes. The returned changes are relative
    /// to this checkout. Project-level diagnostics are reported by [`RepositoryMonitor::service()`].
    pub fn service(&mut self, now: Instant) -> crate::notify::Outcome {
        self.service_inner(now, true)
    }

    pub(crate) fn service_inner(&mut self, now: Instant, include_project_changes: bool) -> crate::notify::Outcome {
        let changes = self.take_changes();
        if !self.status_enabled() {
            self.watches = Default::default();
        }
        let quiet = lock(&self.subscription.pending).loss_debounce;
        let (mut outcome, topology_changed) = self.watches.service(now, quiet);
        self.submodules_dirty |= topology_changed;
        merge_scopes(&mut self.unstaged, outcome.changes.worktree.clone());
        if include_project_changes {
            outcome.changes.merge(changes);
            if let Some(Scope::Paths(paths)) = &mut outcome.changes.worktree {
                paths.sort_by(|a, b| a.path.cmp(&b.path));
            }
        }
        outcome.coverage = self.coverage_status();
        outcome
    }

    /// The next service or enabled status-refresh deadline across the root and its submodules.
    ///
    /// Disabling worktree monitoring suspends status deadlines while preserving metadata service.
    pub fn next_timeout(&self, now: Instant) -> Option<Duration> {
        self.next_service_timeout(now)
            .into_iter()
            .chain(self.next_refresh_timeout(now))
            .min()
    }

    /// The next notification-service deadline, independent of pending status collection.
    ///
    /// Use this while the consumer cannot refresh status, such as when its status view is hidden.
    /// Metadata and child notifications continue to accumulate bounded invalidations.
    pub fn next_service_timeout(&self, now: Instant) -> Option<Duration> {
        self.service_deadline(now)
            .map(|deadline| deadline.saturating_duration_since(now))
            .into_iter()
            .chain((!self.is_closed() && self.watches.has_children()).then_some(crate::notify::POLL_INTERVAL))
            .min()
    }

    pub(crate) fn service_deadline(&self, now: Instant) -> Option<Instant> {
        let pending = lock(&self.subscription.pending);
        if pending.closed {
            None
        } else if pending.coverage_changed || !pending.changes.is_empty() {
            Some(now)
        } else {
            self.watches.deadline(now)
        }
    }

    fn next_refresh_timeout(&self, now: Instant) -> Option<Duration> {
        (self.status_enabled() && self.needs_refresh()).then(|| {
            self.retry
                .map_or(Duration::ZERO, |deadline| deadline.saturating_duration_since(now))
        })
    }

    /// Recompute pending status synchronously, borrowing repository resources only for this call.
    ///
    /// Refresh coverage is returned even when the status items compare equal: a modified file can
    /// change contents again without changing its status classification. Errors and interruption
    /// preserve the previous snapshot and leave the covered paths dirty for a later attempt.
    ///
    /// Supply a freshly opened repository via `repo`, or reload its configuration after configuration
    /// notifications. The repository must match [`layout()`](Self::layout) and the original
    /// reference namespace, including on a cached no-op. This consumes pending project invalidations.
    /// Collection and index comparison use one shared index snapshot for this call.
    pub fn refresh(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<Update> {
        // Validate before changing retry state: a foreign request must not postpone the selected
        // checkout's pending status, even when the previous snapshot is currently clean.
        if self.is_closed() {
            bail!(gix_error::validation("the worktree status subscription is closed"));
        }
        self.validate_refresh_target(repo)?;
        self.take_changes();
        let result = self.refresh_inner(repo, interrupt);
        self.retry = result
            .as_ref()
            .err()
            .map(|_| Instant::now() + self.notifications.retry_interval);
        result
    }

    fn validate_refresh_target(&self, repo: &Repository) -> Result {
        let layout = Layout::for_options(repo, &self.notifications)?;
        if !self.layout().same_target(&layout) || self.subscription.namespace.as_ref() != repo.namespace() {
            return Err(
                message("status refresh repository does not match the selected worktree")
                    .validation()
                    .raise(),
            );
        }
        Ok(())
    }

    fn refresh_inner(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<Update> {
        check_interrupted(interrupt)?;
        if !self.needs_refresh() {
            return Ok(Update::default());
        }
        let index = repo
            .index_or_empty()
            .or_raise(|| message("could not read index for cached status"))?;
        let stamp = (
            index.checksum(),
            index.timestamp().unix_seconds(),
            index.timestamp().nanoseconds(),
        );
        let index_changed = self.index_dirty || self.index_stamp.as_ref() != Some(&stamp) || stamp.0.is_none();
        let mut staged = self.staged;
        let mut pending = self.unstaged.clone();
        let mut submodules_dirty = self.submodules_dirty;
        let next_index = if index_changed {
            let stat_options = repo.stat_options()?;
            if let Some(previous) = &self.index {
                let mut changes = Changes::default();
                let mut rules = Vec::new();
                previous.changes(&index, stat_options, interrupt, |change| {
                    staged |= change.staged;
                    let boundary = [change.previous_mode, change.current_mode]
                        .into_iter()
                        .flatten()
                        .any(|mode| matches!(mode, gix_index::entry::Mode::COMMIT | gix_index::entry::Mode::DIR));
                    if change.staged && (is_submodule_file(change.path) || boundary) {
                        changes.worktree = Some(Scope::All);
                        submodules_dirty = true;
                    } else {
                        if change.staged && is_rule_file(change.path) {
                            rules.push(change.path.to_owned());
                        }
                        let mut path = change.path.to_owned();
                        if self.options.untracked_files == UntrackedFiles::Collapsed
                            && (change.previous_mode.is_none() || change.current_mode.is_none())
                            && let Some(slash) = path.find_byte(b'/')
                        {
                            path.truncate(slash);
                        }
                        changes.insert_worktree_path(path_change(path));
                    }
                })?;
                for path in rules {
                    check_interrupted(interrupt)?;
                    if let Some(root) = repo.workdir() {
                        let file = root.join(gix_path::from_bstr(path.as_bstr())?);
                        // Windows rule loading can fall back to the index on read-denied files despite valid metadata.
                        let from_index = cfg!(windows)
                            || match std::fs::symlink_metadata(&file) {
                                Ok(metadata) => !metadata.is_file(),
                                Err(error)
                                    if matches!(
                                        error.kind(),
                                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                                    ) =>
                                {
                                    true
                                }
                                Err(error) => {
                                    return Err(error
                                        .and_raise(message!("could not inspect status rule file {}", file.display())));
                                }
                            };
                        if from_index {
                            changes.insert_worktree_path(path_change(parent_scope(path.as_bstr())));
                        }
                    }
                }
                merge_scopes(&mut pending, changes.worktree);
            } else {
                staged = true;
                pending = Some(Scope::All);
            }
            Some(index::Snapshot::new(&index, stat_options, interrupt)?)
        } else {
            None
        };
        let head = if self.head_dirty || staged || self.snapshot.is_none() {
            Some(Head::read(repo)?)
        } else {
            None
        };
        staged |= head.as_ref().is_some_and(|head| self.head.as_ref() != Some(head));
        let head_tree_id = head
            .as_ref()
            .filter(|_| staged)
            .map(|head| head.tree_id(repo))
            .transpose()?;
        let mut unstaged = if repo.workdir().is_some() {
            self.effective_scope(repo, &index, pending)?
        } else {
            None
        };
        if submodules_dirty {
            self.reconcile_submodules(repo, interrupt)?;
        }
        let mut next_tree_index = None;
        let tree_index = if let Some(tree_id) = head_tree_id.filter(|_| repo.workdir().is_some()) {
            let protect = repo.config.protect_options()?;
            if self.head_tree_index.as_ref().is_none_or(|cached| {
                cached.tree_id != tree_id
                    || cached.protect.protect_windows != protect.protect_windows
                    || cached.protect.protect_hfs != protect.protect_hfs
                    || cached.protect.protect_ntfs != protect.protect_ntfs
            }) {
                next_tree_index = Some(HeadTreeIndex {
                    tree_id,
                    protect,
                    index: repo.index_from_tree(&tree_id)?.into(),
                });
            }
            next_tree_index
                .as_ref()
                .or(self.head_tree_index.as_ref())
                .map(|cached| &cached.index)
        } else {
            None
        };
        let collect = |patterns, staged, unstaged| {
            let mut status = repo
                .status(gix_utils::progress::Discard)
                .or_raise(|| message("could not prepare cached status"))?;
            if let Some(tree_id) = head_tree_id {
                status = status.head_tree(tree_id);
            }
            status
                .index(index.clone().into())
                .untracked_files(self.options.untracked_files)
                .tree_index_track_renames(self.options.tree_index_track_renames)
                .index_worktree_submodules(self.options.submodules)
                .index_worktree_rewrites(None)
                .index_worktree_options_mut(|options| {
                    options.thread_limit = self.options.thread_limit;
                    options.sorting = Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .collect_internal(
                    patterns,
                    staged,
                    unstaged,
                    interrupt,
                    tree_index,
                    self.options
                        .worktree_rewrites
                        .is_some_and(|options| options.copies.is_some()),
                )
        };
        let replacement = match &unstaged {
            Some(Scope::All) => collect(Vec::new(), staged, true)?,
            Some(Scope::Paths(scopes)) => {
                let mut replacement = if staged {
                    collect(Vec::new(), true, false)?
                } else {
                    Vec::new()
                };
                let patterns = scopes
                    .iter()
                    .map(|scope| {
                        gix_pathspec::Pattern::from_literal(scope.path.as_slice(), gix_pathspec::MagicSignature::TOP)
                            .to_bstring()
                    })
                    .collect();
                replacement.extend(collect(patterns, false, true)?);
                replacement
            }
            None if staged => collect(Vec::new(), true, false)?,
            None => Vec::new(),
        };
        check_interrupted(interrupt)?;
        let ignore_case = repo
            .config
            .fs_capabilities(None)
            .or_raise(|| message("could not read status filesystem options"))?
            .ignore_case;
        let mut next = self
            .unrewritten_snapshot
            .as_ref()
            .or(self.snapshot.as_ref())
            .cloned()
            .unwrap_or_default();
        next.retain(|item| match item {
            Item::TreeIndex(_) => !staged,
            Item::IndexWorktree(_) => !scope_contains(unstaged.as_ref(), item.location(), ignore_case),
        });
        next.extend(replacement);
        for item in &mut next {
            check_interrupted(interrupt)?;
            refresh_item_metadata(item, &index)?;
        }
        // Without deleted sources or copy tracking, raw entries already are the final status.
        let rewrite_options = self.options.worktree_rewrites.filter(|options| {
            repo.workdir().is_some()
                && (options.copies.is_some()
                    || next.iter().any(|item| {
                        matches!(
                            item,
                            Item::IndexWorktree(item)
                                if item.summary() == Some(gix_status::index_as_worktree_with_renames::Summary::Removed)
                        )
                    }))
        });
        let unrewritten_snapshot = if let Some(options) = rewrite_options {
            let rewritten = rewrites::apply(repo, &index, &next, options, interrupt)?;
            // Global pairing rehashes retained candidates, so content caches need their coverage too.
            let candidates: Vec<_> = next
                .iter()
                .filter(|item| matches!(item, Item::IndexWorktree(_)))
                .map(|item| path_change(item.location().to_owned()))
                .collect();
            if !candidates.is_empty() {
                merge_scopes(&mut unstaged, Some(Scope::Paths(candidates)));
            }
            Some(std::mem::replace(&mut next, rewritten))
        } else {
            None
        };
        next.sort_by(|a, b| item_key(a).cmp(&item_key(b)));
        let changes = delta::changes(self.snapshot.as_deref().unwrap_or_default(), &next, interrupt)?;
        let changed = self.snapshot.is_none() || !changes.is_empty();
        check_interrupted(interrupt)?;
        self.snapshot = Some(next);
        self.unrewritten_snapshot = unrewritten_snapshot;
        if let Some(index) = next_index {
            self.index = Some(index);
        }
        self.index_stamp = Some(stamp);
        self.index_dirty = false;
        if next_tree_index.is_some() {
            self.head_tree_index = next_tree_index;
        }
        if let Some(head) = head {
            self.head = Some(head);
        }
        self.head_dirty = false;
        self.staged = false;
        self.unstaged = None;
        self.submodules_dirty = false;
        Ok(Update {
            changed,
            changes,
            staged,
            unstaged,
        })
    }

    fn effective_scope(
        &self,
        repo: &Repository,
        index: &gix_index::State,
        pending: Option<Scope>,
    ) -> Result<Option<Scope>> {
        let Some(Scope::Paths(scopes)) = &pending else {
            return Ok(pending);
        };
        // macOS Unicode aliases need not match the index's byte spelling; avoid retaining another path map.
        if cfg!(target_os = "macos") && scopes.iter().any(|change| !change.path.is_ascii()) {
            return Ok(Some(Scope::All));
        }
        let defaults = repo
            .pathspec_defaults()
            .or_raise(|| message("could not read incremental status pathspec defaults"))?;
        if defaults.literal || defaults.signature.contains(gix_pathspec::MagicSignature::ICASE) {
            return Ok(Some(Scope::All));
        }
        let ignore_case = repo
            .config
            .fs_capabilities(None)
            .or_raise(|| message("could not read status filesystem options"))?
            .ignore_case;
        let mut expanded = Vec::new();
        for change in scopes {
            let scope = &change.path;
            let path = gix_path::from_bstr(scope.as_bstr())?;
            if !scope.is_ascii()
                && index
                    .entries()
                    .iter()
                    .any(|entry| unicode_path_alias(scope.as_bstr(), entry.path(index)))
            {
                // Native paths may use a different Unicode normalization from literal index
                // paths. A literal pathspec and retained-row removal would otherwise miss them.
                return Ok(Some(Scope::All));
            }
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Ok(Some(Scope::All));
            }
            let mut scope = scope.clone();
            // An untracked descendant can change whether any ancestor collapses in the snapshot.
            if self.options.untracked_files == UntrackedFiles::Collapsed
                && index.entry_by_path(scope.as_bstr()).is_none()
                && let Some(slash) = scope.find_byte(b'/')
            {
                scope.truncate(slash);
            }
            if !expanded
                .iter()
                .any(|parent: &BString| path_in_scope(&scope, parent, ignore_case))
            {
                expanded.retain(|child| !path_in_scope(child, &scope, ignore_case));
                expanded.push(scope);
            }
        }
        expanded.sort();
        Ok(if expanded.is_empty() {
            None
        } else {
            Some(Scope::Paths(expanded.into_iter().map(path_change).collect()))
        })
    }

    fn reconcile_submodules(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result {
        if !self.status_enabled() || repo.workdir().is_none() {
            self.watches = Default::default();
            return Ok(());
        }
        self.watches.reconcile(
            repo,
            self.options.submodules,
            &self.notifications,
            &self.subscription,
            interrupt,
        )
    }
}

fn is_rule_file(path: &BStr) -> bool {
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or_default();
    name.eq_ignore_ascii_case(b".gitignore") || name.eq_ignore_ascii_case(b".gitattributes")
}

fn is_submodule_file(path: &BStr) -> bool {
    path.rsplit(|byte| *byte == b'/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(b".gitmodules"))
}

fn parent_scope(path: &BStr) -> BString {
    path[..path.rfind_byte(b'/').unwrap_or(0)].into()
}

fn refresh_item_metadata(item: &mut Item, index: &gix_index::State) -> Result {
    use crate::status::index_worktree::{Item as WorktreeItem, RewriteSource};

    let (entry, position, path, status) = match item {
        Item::IndexWorktree(WorktreeItem::Modification {
            entry,
            entry_index,
            rela_path,
            status,
        }) => (entry, entry_index, rela_path, status),
        Item::IndexWorktree(WorktreeItem::DirectoryContents { entry, .. }) => {
            entry.pathspec_match = Some(gix_dir::entry::PathspecMatch::Always);
            return Ok(());
        }
        Item::IndexWorktree(WorktreeItem::Rewrite {
            source, dirwalk_entry, ..
        }) => {
            dirwalk_entry.pathspec_match = Some(gix_dir::entry::PathspecMatch::Always);
            match source {
                RewriteSource::RewriteFromIndex {
                    source_entry,
                    source_entry_index,
                    source_rela_path,
                    source_status,
                    ..
                } => (source_entry, source_entry_index, source_rela_path, source_status),
                RewriteSource::CopyFromDirectoryEntry {
                    source_dirwalk_entry, ..
                } => {
                    source_dirwalk_entry.pathspec_match = Some(gix_dir::entry::PathspecMatch::Always);
                    return Ok(());
                }
            }
        }
        _ => return Ok(()),
    };
    *position = index
        .entry_index_by_path_and_stage(path.as_bstr(), entry.stage())
        .ok_or_else(|| message!("cached status entry {path:?} is absent from the refreshed index").raise())?;
    entry.clone_from(&index.entries()[*position]);
    if let gix_status::index_as_worktree::EntryStatus::Conflict { entries, .. } = status {
        for cached in entries.iter_mut().flatten() {
            let current = index
                .entry_by_path_and_stage(path.as_bstr(), cached.flags.stage())
                .ok_or_else(|| {
                    message!("cached conflict stage at {path:?} is absent from the refreshed index").raise()
                })?;
            *cached = current.into();
        }
    }
    Ok(())
}

fn check_interrupted(interrupt: &AtomicBool) -> Result {
    if interrupt.load(Ordering::Relaxed) {
        Err(message("status refresh was interrupted").raise())
    } else {
        Ok(())
    }
}

fn item_key(item: &Item) -> (u8, &BStr, u8) {
    use crate::status::index_worktree::Item as WorktreeItem;

    let (side, origin) = match item {
        Item::TreeIndex(_) => (0, 0),
        Item::IndexWorktree(WorktreeItem::Modification { .. }) => (1, 0),
        Item::IndexWorktree(WorktreeItem::DirectoryContents { .. } | WorktreeItem::Rewrite { .. }) => (1, 1),
    };
    (side, item.location(), origin)
}

fn path_in_scope(path: &[u8], scope: &[u8], ignore_case: bool) -> bool {
    let Some(prefix) = path.get(..scope.len()) else {
        return false;
    };
    (if ignore_case {
        prefix.eq_ignore_ascii_case(scope)
    } else {
        prefix == scope
    }) && (path.len() == scope.len() || path.get(scope.len()) == Some(&b'/') || scope.last() == Some(&b'/'))
}

fn scope_contains(scope: Option<&Scope>, path: &BStr, ignore_case: bool) -> bool {
    match scope {
        None => false,
        Some(Scope::All) => true,
        Some(Scope::Paths(scopes)) => scopes
            .iter()
            .any(|scope| path_in_scope(path.as_bytes(), &scope.path, ignore_case)),
    }
}

fn scope_touches_modules(scope: Option<&Scope>, modules: &[BString]) -> bool {
    match scope {
        None => false,
        Some(Scope::All) => true,
        Some(Scope::Paths(paths)) => paths.iter().any(|path| {
            modules.iter().any(|module| {
                path_in_scope(&path.path, module, true)
                    || path_in_scope(module, &path.path, true)
                    || unicode_path_alias(path.path.as_bstr(), module.as_bstr())
            })
        }),
    }
}

fn scope_touches_modules_at(scope: Option<&Scope>, mount: &BString, modules: &[BString]) -> bool {
    match scope {
        None => false,
        Some(Scope::All) => true,
        Some(Scope::Paths(paths)) => paths.iter().any(|path| {
            let full = submodules::join(mount, &path.path);
            modules
                .iter()
                .filter(|module| module.len() > mount.len())
                .any(|module| {
                    path_in_scope(&full, module, true)
                        || path_in_scope(module, &full, true)
                        || unicode_path_alias(full.as_bstr(), module.as_bstr())
                })
        }),
    }
}

fn unicode_path_alias(path: &BStr, module: &BStr) -> bool {
    if path.is_ascii() && module.is_ascii() || path_in_scope(path, module, true) || path_in_scope(module, path, true) {
        return false;
    }
    let path = gix_utils::str::precompose_bstr(path.as_bstr().into());
    let module = gix_utils::str::precompose_bstr(module.as_bstr().into());
    path_in_scope(path.as_ref(), module.as_ref(), true) || path_in_scope(module.as_ref(), path.as_ref(), true)
}

fn merge_scopes(scope: &mut Option<Scope>, other: Option<Scope>) {
    let mut changes = Changes {
        worktree: scope.take(),
        ..Default::default()
    };
    changes.merge(Changes {
        worktree: other,
        ..Default::default()
    });
    *scope = changes.worktree;
}

fn path_change(path: BString) -> crate::notify::PathChange {
    crate::notify::PathChange {
        path,
        kind: crate::notify::plumbing::EventKind::Any,
        path_kind: crate::notify::plumbing::PathKind::Any,
    }
}
