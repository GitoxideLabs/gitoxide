//! An owned status snapshot with detached repository and submodule monitoring.

use std::{
    collections::{BTreeMap, HashSet},
    ops::Bound,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, Result, ResultExt, message};

use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
    notify::{Changes, CoverageStatus, Layout, RepositoryMonitor, Scope, WorktreeChange},
    status::{Item, Submodule, UntrackedFiles},
};

const CHILDREN_PER_SERVICE: usize = 8;
const MAX_ERRORS: usize = 8;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[cfg(feature = "async-notify")]
mod asynchronous;
mod delta;
pub use delta::Change;
mod index;
mod rewrites;
mod submodules;
#[cfg(test)]
mod tests;

/// Control notification scheduling and the contents of the cached status snapshot.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Filesystem monitoring and recovery options. Metadata notifications must be enabled.
    pub repository: crate::notify::Options,
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

/// One selected checkout's owned status snapshot and project filesystem subscriptions.
///
/// The repository used at construction remains the notification coordinator. Select another
/// checkout with [`select_worktree()`](Self::select_worktree) without replacing shared metadata
/// subscriptions. Only one checkout's status is cached at a time.
///
/// No repository, object database, or producer thread is retained between calls. Index entries
/// needed for incremental comparisons are retained in an exact, prefix-compressed byte buffer,
/// without index extensions, per-entry allocations, or open files.
/// A detached index expanded from the captured HEAD tree is reused until the tree or path-protection
/// settings change. Full, configuration, layout, and target invalidations release it.
/// Call [`service()`](Self::service) from the event loop, then [`refresh()`](Self::refresh) when
/// [`refresh_due()`](Self::refresh_due) is true. A failed or interrupted refresh preserves the previous
/// snapshot and its pending invalidations. Initialized nested submodules are monitored recursively.
///
/// With `async-notify`, use `service_async_with()` or `process_async()` for worker-owned filesystem
/// processing, then `ready_async()` for notifications or due status. `ready_service_async()` suspends
/// refresh deadlines while the status view is hidden. `shutdown_async()` releases native resources
/// on a worker. Nested submodules retain bounded polling deadlines.
pub struct Monitor {
    repository: RepositoryMonitor,
    selected: Option<Layout>,
    selected_namespace: Option<gix_ref::Namespace>,
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
    children: BTreeMap<PathBuf, submodules::Child>,
    child_cursor: Option<PathBuf>,
    child_round_remaining: usize,
    child_round_deadline: Option<Instant>,
    child_poll: Option<Instant>,
    aliased_children: bool,
    retry: Option<Instant>,
    submodule_paths: Vec<BString>,
    submodules_dirty: bool,
}

impl Monitor {
    /// Prepare detached monitoring and mark the initial snapshot for a complete refresh.
    /// Metadata notifications are required to keep the index and HEAD comparisons current.
    pub fn new(repo: &Repository, mut options: Options) -> Result<Self> {
        if !options.repository.metadata {
            return Err(message("status monitoring requires repository metadata notifications").raise());
        }
        let repository = repo.monitor(options.repository.clone())?;
        options.repository = repository.options().clone();
        Ok(Self {
            selected: Some(repository.layout().clone()),
            selected_namespace: repo.namespace().cloned(),
            repository,
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
            children: BTreeMap::new(),
            child_cursor: None,
            child_round_remaining: 0,
            child_round_deadline: None,
            child_poll: None,
            aliased_children: false,
            retry: None,
            submodule_paths: Vec::new(),
            submodules_dirty: true,
        })
    }

    /// The most recent successful snapshot, including while a later refresh is pending or failed.
    /// Directory entries describe the complete snapshot, so their pathspec match is always `Always`.
    pub fn snapshot(&self) -> Option<&[Item]> {
        self.snapshot.as_deref()
    }

    /// The HEAD commit used by the most recent successful snapshot.
    ///
    /// Returns `None` before a snapshot exists or when its HEAD is unborn. Dirty or failed refreshes
    /// retain the previous committed baseline. A cached, unchanged refresh does not relabel status
    /// with the repository's current HEAD; compare this ID before using cached status with new reads.
    pub fn snapshot_head_id(&self) -> Option<gix_hash::ObjectId> {
        self.head.as_ref().and_then(|head| head.commit_id)
    }

    /// Whether a refresh is needed, including checking if a reference event changed `HEAD`.
    pub fn is_dirty(&self) -> bool {
        self.snapshot.is_none()
            || self.head_dirty
            || self.index_dirty
            || self.staged
            || self.unstaged.is_some()
            || self.submodules_dirty
    }

    /// Whether selected status can be retried now, honoring the retry interval after failure.
    /// Returns false while no checkout is selected.
    pub fn refresh_due(&self, now: Instant) -> bool {
        self.selected.is_some() && self.is_dirty() && self.retry.is_none_or(|deadline| deadline <= now)
    }

    /// Inspect the root repository's detached watch layout.
    pub fn repository_monitor(&self) -> &RepositoryMonitor {
        &self.repository
    }

    /// Set the quiet period for recoverable notification loss in this checkout and its submodules.
    /// See [`RepositoryMonitor::set_loss_debounce()`] for scheduling and validation.
    pub fn set_loss_debounce(&mut self, quiet: Option<Duration>) -> Result {
        self.repository.set_loss_debounce(quiet)?;
        for child in self.children.values_mut() {
            child.monitor.apply_loss_debounce(quiet);
        }
        Ok(())
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
        let mut active = false;
        let mut updating = self.status_enabled() && self.submodules_dirty;
        let mut recovering = false;
        let mut retry_at = None;
        for coverage in std::iter::once(self.repository.coverage_status())
            .chain(self.children.values().map(|child| child.monitor.coverage_status()))
        {
            match coverage {
                CoverageStatus::Inactive => {}
                CoverageStatus::Healthy => active = true,
                CoverageStatus::Updating => updating = true,
                CoverageStatus::Recovering { retry_at: deadline } => {
                    recovering = true;
                    retry_at = retry_at.into_iter().chain(deadline).min();
                }
            }
        }
        if recovering {
            CoverageStatus::Recovering { retry_at }
        } else if updating {
            CoverageStatus::Updating
        } else if active {
            CoverageStatus::Healthy
        } else {
            CoverageStatus::Inactive
        }
    }

    /// The current snapshot and notification options.
    ///
    /// Use [`update_subscriptions()`](Self::update_subscriptions) for project subscription selection,
    /// [`update_checkout_filter()`](Self::update_checkout_filter) for checkout coverage, and
    /// [`set_submodules()`](Self::set_submodules) for the target's submodule status policy.
    /// Clone these before dropping this instance to change other options in its replacement.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Replace linked-worktree selection and application dependencies without discarding status.
    ///
    /// Resolution and maintenance follow [`RepositoryMonitor::update_subscriptions()`]. Invalid
    /// paths leave all options unchanged. The next service applies the previous and new dependency
    /// invalidations; existing snapshot data and pending refresh retries remain available.
    /// Call this on a blocking worker when using an asynchronous executor.
    pub fn update_subscriptions(
        &mut self,
        worktree_filter: Option<Vec<PathBuf>>,
        additional_paths: Vec<crate::notify::InvalidationPath>,
    ) -> Result {
        self.repository
            .update_subscriptions(worktree_filter, additional_paths)?;
        self.options.repository = self.repository.options().clone();
        Ok(())
    }

    /// Replace checkout coverage without changing project metadata or membership subscriptions.
    ///
    /// See [`RepositoryMonitor::update_checkout_filter()`]. This does not change the status target;
    /// use [`select_worktree()`](Self::select_worktree) separately. Run on a blocking worker.
    pub fn update_checkout_filter(&mut self, checkout_filter: Option<Vec<PathBuf>>) -> Result {
        self.repository.update_checkout_filter(checkout_filter)?;
        self.options.repository = self.repository.options().clone();
        if !self.status_enabled() {
            self.clear_children();
        }
        Ok(())
    }

    /// The checkout whose owned status snapshot is selected, independently of the project coordinator.
    pub fn selected_worktree(&self) -> Option<&Layout> {
        self.selected.as_ref()
    }

    /// Select one checkout in the same project without replacing project subscriptions.
    ///
    /// The repository used at construction remains the notification coordinator. Selecting another
    /// checkout requires `all_worktrees` and inclusion in `worktree_filter`. Checkout coverage is
    /// independent: pair this with [`update_checkout_filter()`](Self::update_checkout_filter) as needed.
    /// A changed target discards its old snapshot, index, HEAD and submodule coverage. An identical
    /// target and reference namespace are a no-op; validation failures retain the previous target and cache. `None` releases
    /// status state and suspends its refresh and submodule deadlines while metadata remains active.
    /// Only one target is cached; switching back requires a complete refresh. Run on a blocking worker.
    pub fn select_worktree(&mut self, repo: Option<&Repository>) -> Result {
        let selected = repo
            .map(|repo| Layout::for_options(repo, &self.options.repository))
            .transpose()?;
        if let (Some(layout), Some(repo)) = (&selected, repo) {
            if !same_repository_path(&layout.common_dir, &self.repository.layout().common_dir, repo)? {
                return Err(
                    message("selected status worktree must share the coordinator's common directory")
                        .validation()
                        .raise(),
                );
            }
            if layout.git_dir != self.repository.layout().git_dir
                && (!self.options.repository.all_worktrees
                    || self
                        .options
                        .repository
                        .worktree_filter
                        .as_ref()
                        .is_some_and(|paths| !paths.contains(&layout.git_dir)))
            {
                return Err(
                    message("selected status worktree must be included in all-worktree monitoring")
                        .validation()
                        .raise(),
                );
            }
        }
        let namespace = repo.and_then(Repository::namespace).cloned();
        let changed = match (&self.selected, &selected) {
            (Some(previous), Some(next)) => !same_target(previous, next) || self.selected_namespace != namespace,
            (None, None) => false,
            _ => true,
        };
        self.selected = selected;
        self.selected_namespace = namespace;
        if changed {
            self.clear_snapshot();
        }
        Ok(())
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
        self.selected
            .as_ref()
            .is_some_and(|layout| layout.worktree.is_some() && self.repository.covers_worktree(&layout.git_dir))
    }

    fn clear_children(&mut self) {
        self.children.clear();
        self.submodule_paths.clear();
        self.child_cursor = None;
        self.child_round_remaining = 0;
        self.child_round_deadline = None;
        self.child_poll = None;
        self.aliased_children = false;
    }

    fn clear_snapshot(&mut self) {
        self.snapshot = None;
        self.unrewritten_snapshot = None;
        self.index = None;
        self.index_stamp = None;
        self.head = None;
        self.clear_children();
        self.invalidate();
    }

    fn invalidate_repository_outcome(&mut self, outcome: &crate::notify::Outcome) {
        let Some(selected) = &self.selected else {
            return;
        };
        let mut changes = outcome.changes.clone();
        if selected.git_dir != self.repository.layout().git_dir {
            // Parent scalar metadata may include shared refs/configuration. Its relative checkout
            // scopes and index belong to the coordinator and cannot be relabeled for this target.
            changes.worktree = None;
            changes.index = false;
            for other in &outcome.other_worktrees {
                if other.git_dir != selected.git_dir {
                    continue;
                }
                match &other.kind {
                    WorktreeChange::Changed(other) => changes.merge(other.clone()),
                    WorktreeChange::Added => {
                        self.invalidate();
                        break;
                    }
                    WorktreeChange::Moved { .. } => {
                        self.selected.as_mut().expect("the selected target exists").worktree =
                            Some(other.worktree.clone());
                        self.clear_snapshot();
                        return;
                    }
                    WorktreeChange::Removed => {
                        self.selected = None;
                        self.selected_namespace = None;
                        self.clear_snapshot();
                        return;
                    }
                }
            }
        }
        self.invalidate_changes(&changes);
    }

    /// Force a complete refresh and rebuild all native subscriptions.
    pub fn rescan(&mut self) {
        self.repository.rescan();
        for child in self.children.values_mut() {
            child.monitor.rescan();
        }
        self.invalidate();
    }

    /// Switch repository layout and discard the old repository's cached status.
    pub fn reconfigure(&mut self, repo: &Repository) -> Result {
        let mut replacement = Self::new(repo, self.options.clone())?;
        replacement
            .repository
            .apply_loss_debounce(self.repository.loss_debounce());
        *self = replacement;
        Ok(())
    }

    /// Mark the complete snapshot and submodule inventory for refresh.
    pub fn invalidate(&mut self) {
        self.head_tree_index = None;
        self.retry = None;
        self.staged = true;
        self.index_dirty = true;
        self.unstaged = Some(Scope::All);
        self.head_dirty = true;
        self.submodules_dirty = true;
    }

    /// Apply invalidations from application-owned Git operations or other event sources.
    /// Native Unicode aliases of configured submodule paths require a full worktree refresh.
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
                self.submodule_paths
                    .iter()
                    .any(|module| unicode_path_alias(path.path.as_bstr(), module.as_bstr()))
            })
        {
            // A native Unicode alias cannot select the gitlink's literal index spelling.
            self.unstaged = Some(Scope::All);
        }
        self.submodules_dirty |= changes.configuration
            || changes.attributes
            || changes.layout
            || scope_touches_modules(changes.worktree.as_ref(), &self.submodule_paths);
    }

    /// Drain bounded notifications from the root and its initialized nested submodules.
    ///
    /// The root's `opener` is used only when its watch inventory needs maintenance. Child
    /// repositories reuse detached open options inherited through the submodule API.
    /// Coverage and recovery include every child, including those outside the serviced batch.
    pub fn service(&mut self, now: Instant, opener: impl FnMut() -> Result<Repository>) -> crate::notify::Outcome {
        let mut outcome = self.repository.service(now, opener);
        self.invalidate_repository_outcome(&outcome);
        let mut children_changes = Changes::default();
        let mut omitted_errors = 0usize;
        if self.child_round_remaining == 0 && self.child_poll.is_some_and(|deadline| deadline <= now) {
            self.child_round_remaining = self.children.len();
            self.child_round_deadline = Some(now + CHILD_POLL_INTERVAL);
        }
        let budget = CHILDREN_PER_SERVICE
            .min(self.child_round_remaining)
            .min(self.children.len());
        let mut keys: Vec<_> = match &self.child_cursor {
            Some(cursor) => self
                .children
                .range::<PathBuf, _>((Bound::Excluded(cursor), Bound::Unbounded))
                .take(budget)
                .map(|(key, _)| key.clone())
                .collect(),
            None => self.children.keys().take(budget).cloned().collect(),
        };
        if keys.len() < budget {
            keys.extend(self.children.keys().take(budget - keys.len()).cloned());
        }
        for key in &keys {
            let child = self
                .children
                .get_mut(key)
                .expect("selected child keys exist for the entire service call");
            let child_outcome = child.service(now);
            if let Some(timeout) = child.monitor.next_timeout(now) {
                let deadline = now + timeout;
                self.child_round_deadline = Some(self.child_round_deadline.map_or(deadline, |old| old.min(deadline)));
            }
            if !child_outcome.changes.is_empty() || !child_outcome.errors.is_empty() {
                if self.aliased_children {
                    children_changes.worktree = Some(Scope::All);
                } else {
                    children_changes.insert_worktree_path(path_change(child.top_level.clone()));
                }
            }
            self.submodules_dirty |= child_outcome.changes.index
                || child_outcome.changes.configuration
                || child_outcome.changes.attributes
                || child_outcome.changes.layout
                || scope_touches_modules_at(
                    child_outcome.changes.worktree.as_ref(),
                    &child.mount,
                    &self.submodule_paths,
                );
            for error in child_outcome.errors {
                if outcome.errors.len() < MAX_ERRORS - 1 {
                    outcome.errors.push(error);
                } else {
                    omitted_errors = omitted_errors.saturating_add(1);
                }
            }
            outcome.statistics.merge(child_outcome.statistics);
        }
        self.child_round_remaining = self.child_round_remaining.saturating_sub(keys.len());
        if let Some(key) = keys.last() {
            self.child_cursor = Some(key.clone());
        }
        if self.children.is_empty() {
            self.child_poll = None;
        } else if !keys.is_empty() && self.child_round_remaining == 0 {
            self.child_poll = self.child_round_deadline.take().or(Some(now + CHILD_POLL_INTERVAL));
        }
        if omitted_errors != 0 {
            outcome
                .errors
                .push(message!("{omitted_errors} additional submodule monitoring errors were omitted").raise());
        }
        merge_scopes(&mut self.unstaged, children_changes.worktree.clone());
        if let Some(selected) = &self.selected {
            if selected.git_dir == self.repository.layout().git_dir {
                outcome.changes.merge(children_changes);
            } else if !children_changes.is_empty() {
                if let Some(other) = outcome
                    .other_worktrees
                    .iter_mut()
                    .find(|other| other.git_dir == selected.git_dir)
                {
                    if let WorktreeChange::Changed(changes) = &mut other.kind {
                        changes.merge(children_changes);
                    }
                } else if let Some(worktree) = &selected.worktree {
                    outcome.other_worktrees.push(crate::notify::WorktreeChanges {
                        git_dir: selected.git_dir.clone(),
                        worktree: worktree.clone(),
                        kind: WorktreeChange::Changed(children_changes),
                    });
                }
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
        self.repository
            .next_timeout(now)
            .into_iter()
            .chain(self.next_child_timeout(now))
            .min()
    }

    fn next_child_timeout(&self, now: Instant) -> Option<Duration> {
        if self.child_round_remaining != 0 {
            Some(Duration::ZERO)
        } else {
            self.child_poll.map(|deadline| deadline.saturating_duration_since(now))
        }
    }

    fn next_refresh_timeout(&self, now: Instant) -> Option<Duration> {
        (self.status_enabled() && self.is_dirty()).then(|| {
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
    /// Supply a freshly opened repository, or reload its configuration after configuration
    /// notifications. The repository must match [`selected_worktree()`](Self::selected_worktree),
    /// including on a cached no-op. Coverage is prepared through the stable coordinator before
    /// collection. Collection and index comparison use one shared index snapshot for this call.
    pub fn refresh(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<Update> {
        // Validate before changing retry state: a foreign request must not postpone the selected
        // checkout's pending status, even when the previous snapshot is currently clean.
        let layout = self.validate_refresh_target(repo)?;
        let result = self.refresh_inner(repo, layout, interrupt);
        self.retry = result
            .as_ref()
            .err()
            .map(|_| Instant::now() + self.options.repository.retry_interval);
        result
    }

    fn validate_refresh_target(&self, repo: &Repository) -> Result<Layout> {
        let layout = Layout::for_options(repo, &self.options.repository)?;
        if !self.selected.as_ref().is_some_and(|selected| {
            same_target(selected, &layout) && self.selected_namespace.as_ref() == repo.namespace()
        }) {
            return Err(
                message("status refresh repository does not match the selected worktree")
                    .validation()
                    .raise(),
            );
        }
        Ok(layout)
    }

    fn refresh_inner(&mut self, repo: &Repository, layout: Layout, interrupt: &AtomicBool) -> Result<Update> {
        check_interrupted(interrupt)?;
        if !self.is_dirty() {
            return Ok(Update::default());
        }
        self.repository.prepare_worktree(repo, Instant::now())?;
        self.selected = Some(layout);
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
            self.clear_children();
            return Ok(());
        }
        let discovered = submodules::discover(repo, &self.options, interrupt)?;
        let mut retain = HashSet::new();
        let mut first_error = None;
        for (key, prepared) in discovered.children {
            check_interrupted(interrupt)?;
            retain.insert(key.clone());
            let mut child = prepared.child;
            child.monitor.apply_loss_debounce(self.repository.loss_debounce());
            if let Some(old) = self.children.get_mut(&key)
                && old.monitor.layout() == child.monitor.layout()
                && old.worktree == child.worktree
            {
                old.worktree = child.worktree;
                old.open_options = child.open_options;
                old.mount = child.mount;
                old.top_level = child.top_level;
                if let Err(error) = old.monitor.prepare(&prepared.repository, Instant::now()) {
                    first_error.get_or_insert(error);
                }
            } else {
                match child.monitor.prepare(&prepared.repository, Instant::now()) {
                    Ok(()) => {
                        self.children.insert(key, child);
                    }
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        self.children.retain(|key, _| retain.contains(key));
        self.submodule_paths = discovered.paths;
        self.aliased_children = discovered.aliased;
        self.child_cursor = None;
        self.child_round_remaining = self.children.len();
        self.child_round_deadline = Some(Instant::now() + CHILD_POLL_INTERVAL);
        self.child_poll = (!self.children.is_empty()).then(Instant::now);
        Ok(())
    }
}

fn same_target(a: &Layout, b: &Layout) -> bool {
    a.git_dir == b.git_dir && a.common_dir == b.common_dir && a.worktree == b.worktree && a.index == b.index
}

fn same_repository_path(a: &std::path::Path, b: &std::path::Path, repo: &Repository) -> Result<bool> {
    let ignore_case = repo.config.fs_capabilities(None)?.ignore_case;
    Ok(crate::notify::same_path(a, b, ignore_case))
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
