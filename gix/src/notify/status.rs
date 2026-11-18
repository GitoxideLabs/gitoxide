use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

use gix_error::{Result, bail, validation};
use gix_parallel::{Mutable, OwnShared, WeakShared, lock};

use super::{
    Changes, CoverageStatus, Layout, Outcome, RepositoryMonitor, Scope, WorktreeChange, WorktreeChanges, same_path,
};
use crate::{
    Repository,
    status::{WorktreeStatusHandle, monitor},
};

#[cfg_attr(test, derive(Default))]
pub(super) struct Subscriptions {
    open_options: crate::open::Options,
    namespace: Option<gix_ref::Namespace>,
    handles: Mutable<Vec<WeakShared<Subscription>>>,
    retained: OwnShared<Mutable<BTreeMap<PathBuf, Retained>>>,
}

impl Subscriptions {
    pub(super) fn new(repo: &Repository) -> Self {
        Self {
            open_options: repo.open_options().clone_with_trust(),
            namespace: repo.namespace().cloned(),
            handles: Default::default(),
            retained: Default::default(),
        }
    }
    pub(super) fn set_loss_debounce(&mut self, quiet: Option<Duration>) {
        for handle in self.handles.get_mut().iter().filter_map(WeakShared::upgrade) {
            lock(&handle.pending).loss_debounce = quiet;
        }
    }
}

impl Drop for Subscriptions {
    fn drop(&mut self) {
        for handle in self.handles.get_mut().iter().filter_map(WeakShared::upgrade) {
            lock(&handle.pending).closed = true;
            handle.wake();
        }
    }
}

pub(crate) struct Subscription {
    retained: WeakShared<Mutable<BTreeMap<PathBuf, Retained>>>,
    open_options: crate::open::Options,
    pub(crate) namespace: Option<gix_ref::Namespace>,
    pub(crate) layout: Layout,
    pub(crate) pending: Mutable<Pending>,
    #[cfg(feature = "async-notify")]
    pub(crate) wake: OwnShared<tokio::sync::Notify>,
}

struct Retained {
    subscription: OwnShared<Subscription>,
    watches: monitor::submodules::Watches,
    mode: crate::status::Submodule,
    rediscover: Option<Instant>,
    failed: bool,
}

pub(crate) struct Pending {
    pub(crate) changes: Changes,
    pub(crate) coverage: CoverageStatus,
    pub(crate) loss_debounce: Option<Duration>,
    pub(crate) coverage_changed: bool,
    pub(crate) covered: bool,
    pub(crate) closed: bool,
}

impl Subscription {
    fn wake(&self) {
        #[cfg(feature = "async-notify")]
        self.wake.notify_waiters();
    }

    pub(crate) fn retain_watches(
        subscription: &OwnShared<Self>,
        watches: &mut monitor::submodules::Watches,
        mode: crate::status::Submodule,
        rediscover: Option<Instant>,
    ) {
        if !watches.has_gitlinks() {
            return;
        }
        if let Some(retained) = subscription.retained.upgrade() {
            let mut retained = lock(&retained);
            let pending = lock(&subscription.pending);
            if pending.covered && !pending.closed {
                retained.insert(
                    subscription.layout.git_dir.clone(),
                    Retained {
                        subscription: subscription.clone(),
                        watches: std::mem::take(watches),
                        mode,
                        rediscover,
                        failed: false,
                    },
                );
                subscription.wake();
            }
        }
    }
}

impl RepositoryMonitor {
    /// Subscribe to status invalidations for `repo` and start with an empty status cache.
    ///
    /// A handle owns one worktree's status baseline. Any number can coexist,
    /// including handles with different status options or reference namespaces for the same checkout.
    /// Service this monitor to deliver repository notifications to every live handle. Pass the
    /// handles to [`Self::service_with_status()`] to include nested submodule notifications in
    /// the project outcome. Dropping a handle releases its cache. For a checkout with gitlinks
    /// the project continues servicing the last released handle's submodule watches until
    /// another handle takes them over or the checkout leaves coverage.
    ///
    /// Metadata must be enabled. Additional checkouts must be included in all-worktree monitoring.
    /// Use checkout coverage to receive changes to files in the working tree.
    pub fn worktree_status(
        &self,
        repo: &Repository,
        options: monitor::Options,
    ) -> Result<monitor::WorktreeStatusHandle> {
        if !self.options.metadata {
            bail!(validation(
                "status monitoring requires repository metadata notifications"
            ));
        }
        let layout = Layout::for_options(repo, &self.options)?;
        if !same_path(&layout.common_dir, &self.layout.common_dir, self.ignore_case) {
            bail!(validation("status worktree must share the monitor's common directory"));
        }
        let primary = same_path(&layout.git_dir, &self.layout.git_dir, self.ignore_case);
        if !(primary || self.all_worktrees_enabled() && self.includes_worktree(&layout.git_dir)) {
            bail!(validation(
                "status worktree must be included in all-worktree monitoring"
            ));
        }
        if self
            .monitored_worktree(&layout.git_dir)
            .is_some_and(|known| !known.same_target(&layout))
        {
            bail!(validation(
                "status repository must use the monitored checkout and index"
            ));
        }
        let covered = layout.worktree.is_some() && self.covers_worktree(&layout.git_dir);
        let subscription = OwnShared::new(Subscription {
            retained: OwnShared::downgrade(&self.status.retained),
            open_options: repo.open_options().clone_with_trust(),
            namespace: repo.namespace().cloned(),
            layout,
            pending: Mutable::new(Pending {
                changes: Changes::default(),
                coverage: self.root_coverage_status(),
                loss_debounce: self.loss_debounce(),
                coverage_changed: false,
                covered,
                closed: false,
            }),
            #[cfg(feature = "async-notify")]
            wake: self.wake.clone(),
        });
        let retained = lock(&self.status.retained).remove(&subscription.layout.git_dir);
        let mut handles = lock(&self.status.handles);
        handles.retain(|handle| handle.strong_count() != 0);
        handles.push(OwnShared::downgrade(&subscription));
        let mut handle = monitor::WorktreeStatusHandle::new(options, self.options(), subscription);
        if covered
            && let Some(retained) = retained
            && !lock(&retained.subscription.pending).closed
            && retained.subscription.layout.same_target(handle.layout())
        {
            handle.watches = retained.watches;
        }
        Ok(handle)
    }

    /// Service project notifications and the supplied handles' submodule watches in one pass.
    /// The outcome includes errors, statistics and coverage from both. Submodule changes are
    /// reported at the enclosing gitlink in the owning checkout. Status snapshots are refreshed
    /// only on request. Closed handles and handles from another monitor are skipped.
    pub fn service_with_status<'a>(
        &mut self,
        now: Instant,
        opener: impl FnMut() -> Result<Repository>,
        handles: impl IntoIterator<Item = &'a mut WorktreeStatusHandle>,
    ) -> Outcome {
        let mut outcome = self.service(now, opener);
        self.service_status_handles(now, &mut outcome, handles);
        outcome
    }

    /// Complete a project service outcome with the supplied handles on the same worker.
    /// Use this in the callback to `service_async_with()` when the worker also owns status handles.
    /// Pass the outcome from this monitor's current service call.
    pub fn service_status_handles<'a>(
        &self,
        now: Instant,
        outcome: &mut Outcome,
        handles: impl IntoIterator<Item = &'a mut WorktreeStatusHandle>,
    ) {
        for handle in handles {
            if !self.owns_status(handle) {
                continue;
            }
            let coverage = if handle.next_service_timeout(now) == Some(Duration::ZERO) {
                let local = handle.service_inner(now, false);
                outcome.errors.extend(local.errors);
                outcome.statistics.merge(local.statistics);
                if !local.changes.is_empty() {
                    outcome.merge_checkout(self, handle.layout(), local.changes);
                }
                local.coverage
            } else {
                handle.coverage_status()
            };
            outcome.coverage = outcome.coverage.combine(coverage);
        }
    }

    /// Current project and submodule coverage, including after a status refresh changes the watchers.
    pub fn coverage_with_status<'a>(
        &self,
        handles: impl IntoIterator<Item = &'a WorktreeStatusHandle>,
    ) -> CoverageStatus {
        handles
            .into_iter()
            .filter(|handle| self.owns_status(handle))
            .fold(self.coverage_status(), |coverage, handle| {
                coverage.combine(handle.coverage_status())
            })
    }

    /// Maximum blocking wait until this project or one of the handles needs notification service.
    /// Pending snapshot refreshes can wait until the application requests status.
    pub fn next_timeout_with_status<'a>(
        &self,
        now: Instant,
        handles: impl IntoIterator<Item = &'a WorktreeStatusHandle>,
    ) -> Option<Duration> {
        self.next_timeout(now)
            .into_iter()
            .chain(self.status_timeout(now, handles))
            .min()
    }

    /// Await project notifications or the earliest submodule service deadline, leaving snapshots demand-driven.
    #[cfg(feature = "async-notify")]
    pub async fn ready_with_status_async<'a>(&self, handles: impl IntoIterator<Item = &'a WorktreeStatusHandle>) {
        let handles: Vec<_> = handles.into_iter().filter(|handle| self.owns_status(handle)).collect();
        self.ready_with_deadline(|now| handles.iter().filter_map(|handle| handle.service_deadline(now)).min())
            .await;
    }

    fn status_timeout<'a>(
        &self,
        now: Instant,
        handles: impl IntoIterator<Item = &'a WorktreeStatusHandle>,
    ) -> Option<Duration> {
        handles
            .into_iter()
            .filter(|handle| self.owns_status(handle))
            .filter_map(|handle| handle.next_service_timeout(now))
            .min()
    }

    fn owns_status(&self, handle: &WorktreeStatusHandle) -> bool {
        WeakShared::ptr_eq(
            &handle.subscription.retained,
            &OwnShared::downgrade(&self.status.retained),
        ) && !handle.is_closed()
    }

    pub(super) fn retained_deadline(&self, now: Instant) -> Option<Instant> {
        lock(&self.status.retained)
            .values()
            .filter_map(|entry| {
                let pending = lock(&entry.subscription.pending);
                if pending.closed || !pending.covered || pending.coverage_changed || !pending.changes.is_empty() {
                    Some(now)
                } else {
                    entry.rediscover.into_iter().chain(entry.watches.deadline(now)).min()
                }
            })
            .min()
    }

    pub(super) fn rescan_retained(&self, now: Instant) {
        for entry in lock(&self.status.retained).values_mut() {
            entry.watches.rescan();
            entry.rediscover = Some(now);
        }
    }

    pub(super) fn retained_coverage(&self, root: CoverageStatus) -> CoverageStatus {
        lock(&self.status.retained).values().fold(root, |coverage, entry| {
            let discovery = match entry.rediscover {
                Some(retry) if entry.failed => CoverageStatus::Recovering { retry_at: Some(retry) },
                Some(_) => CoverageStatus::Updating,
                None => CoverageStatus::Inactive,
            };
            entry.watches.coverage(coverage.combine(discovery))
        })
    }

    pub(super) fn service_retained(&self, now: Instant, outcome: &mut Outcome) {
        // Reopening can invoke user callbacks. Other workers must be free to return their watches.
        let mut retained = std::mem::take(&mut *lock(&self.status.retained));
        if retained.is_empty() {
            return;
        }
        retained.retain(|_, entry| {
            let (changes, quiet) = {
                let mut pending = lock(&entry.subscription.pending);
                if pending.closed || !pending.covered {
                    return false;
                }
                pending.coverage_changed = false;
                (std::mem::take(&mut pending.changes), pending.loss_debounce)
            };
            if changes.index || entry.watches.topology_changed(&changes) {
                entry.rediscover.get_or_insert(now);
            }
            let (mut local, topology_changed) = entry.watches.service(now, quiet);
            if topology_changed {
                entry.rediscover.get_or_insert(now);
            }
            if entry.rediscover.is_some_and(|deadline| deadline <= now) {
                let subscription = &entry.subscription;
                match subscription
                    .layout
                    .reopen(&subscription.open_options, subscription.namespace.as_ref())
                    .and_then(|repo| {
                        entry
                            .watches
                            .reconcile(&repo, entry.mode, &self.options, subscription, &AtomicBool::new(false))
                    }) {
                    Ok(()) => {
                        if entry.failed {
                            local.changes.worktree = Some(Scope::All);
                        }
                        entry.rediscover = None;
                        entry.failed = false;
                    }
                    Err(err) => {
                        local.changes.worktree = Some(Scope::All);
                        local.errors.push(err);
                        entry.rediscover = Some(now + self.options.retry_interval);
                        entry.failed = true;
                    }
                }
            }
            if !local.changes.is_empty() {
                outcome.merge_checkout(self, &entry.subscription.layout, local.changes);
            }
            outcome.errors.extend(local.errors);
            outcome.statistics.merge(local.statistics);
            entry.watches.has_gitlinks() || entry.rediscover.is_some()
        });
        let mut returned = lock(&self.status.retained);
        retained.append(&mut returned);
        *returned = retained;
    }

    /// Open the checkout identified by a Git directory from [`Outcome::other_worktrees`],
    /// [`Self::other_worktrees()`] or [`Self::layout()`], using this project's opening policy and namespace.
    /// An unknown or removed registration, or an inaccessible linked checkout, returns an error.
    pub fn open_worktree(&self, git_dir: &Path) -> Result<Repository> {
        let Some(layout) = self.monitored_worktree(git_dir) else {
            bail!(validation(format!(
                "unknown monitored Git directory {}",
                git_dir.display()
            )));
        };
        let primary = self
            .layout
            .reopen(&self.status.open_options, self.status.namespace.as_ref())?;
        if same_path(git_dir, &self.layout.git_dir, self.ignore_case) {
            return Ok(primary);
        }
        let mut repo = if same_path(&layout.git_dir, &self.layout.common_dir, self.ignore_case) {
            primary.main_repo()?
        } else {
            crate::worktree::Proxy::new(&primary, &layout.git_dir).into_repo()?
        };
        repo.refs.namespace.clone_from(&self.status.namespace);
        Ok(repo)
    }

    fn monitored_worktree(&self, git_dir: &Path) -> Option<&Layout> {
        if same_path(git_dir, &self.layout.git_dir, self.ignore_case) {
            Some(&self.layout)
        } else {
            self.other_worktrees.get(git_dir).map(|other| &other.layout)
        }
    }

    pub(super) fn publish_status(&self, out: &Outcome) {
        lock(&self.status.handles).retain(|weak| {
            let Some(handle) = weak.upgrade() else { return false };
            let mut pending = lock(&handle.pending);
            let git_dir = &handle.layout.git_dir;
            let primary = same_path(git_dir, &self.layout.git_dir, self.ignore_case);
            let mut changes = status_changes(if primary {
                out.changes.clone()
            } else {
                // Shared metadata can affect every checkout. The primary's index and paths are private.
                Changes {
                    index: false,
                    worktree: None,
                    ..out.changes
                }
            });
            if !primary {
                pending.closed |= !self.all_worktrees_enabled() || !self.includes_worktree(git_dir);
                for other in out
                    .other_worktrees
                    .iter()
                    .filter(|other| same_path(&other.git_dir, git_dir, self.ignore_case))
                {
                    match &other.kind {
                        WorktreeChange::Changed(other) => changes.merge(status_changes(other.clone())),
                        WorktreeChange::Added => changes.layout = true,
                        WorktreeChange::Moved { .. } | WorktreeChange::Removed => pending.closed = true,
                    }
                }
            }
            if self
                .monitored_worktree(git_dir)
                .is_some_and(|known| !known.same_target(&handle.layout))
            {
                pending.closed = true;
            }
            let covered = handle.layout.worktree.is_some() && self.covers_worktree(git_dir);
            changes.layout |= pending.covered != covered;
            pending.covered = covered;
            if !changes.is_empty() || pending.coverage != out.coverage || pending.closed {
                pending.changes.merge(changes);
                pending.coverage_changed |= pending.coverage != out.coverage;
                pending.coverage = out.coverage;
                handle.wake();
            }
            !pending.closed
        });
    }
}

fn status_changes(mut changes: Changes) -> Changes {
    changes.operations = false;
    // A later path in the queue must not narrow an unscoped rule change.
    if changes.worktree.is_none() && (changes.ignores || changes.attributes) {
        changes.worktree = Some(Scope::All);
    }
    changes
}

impl Outcome {
    fn merge_checkout(&mut self, project: &RepositoryMonitor, layout: &Layout, changes: Changes) {
        let matches = |git_dir: &Path| same_path(git_dir, &layout.git_dir, project.ignore_case);
        let target = if matches(&project.layout.git_dir) {
            &mut self.changes
        } else if let Some(other) = self.other_worktrees.iter_mut().find(|other| matches(&other.git_dir)) {
            let WorktreeChange::Changed(existing) = &mut other.kind else {
                return;
            };
            existing
        } else if let Some(worktree) = &layout.worktree {
            self.other_worktrees.push(WorktreeChanges {
                git_dir: layout.git_dir.clone(),
                worktree: worktree.clone(),
                kind: WorktreeChange::Changed(changes),
            });
            return;
        } else {
            return;
        };
        target.merge(changes);
        if let Some(Scope::Paths(paths)) = &mut target.worktree {
            paths.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }
}

impl Layout {
    fn reopen(&self, options: &crate::open::Options, namespace: Option<&gix_ref::Namespace>) -> Result<Repository> {
        let mut repo = crate::open_opts(&self.git_dir, options.clone_with_trust().open_path_as_is(true))?;
        if repo.workdir() != self.worktree.as_deref() {
            repo.set_workdir(self.worktree.clone())?;
        }
        repo.refs.namespace = namespace.cloned();
        Ok(repo)
    }

    pub(crate) fn same_target(&self, other: &Self) -> bool {
        self.git_dir == other.git_dir
            && self.common_dir == other.common_dir
            && self.worktree == other.worktree
            && self.index == other.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bstr::ByteSlice,
        notify::{PathChange, plumbing},
    };
    use std::{sync::atomic::AtomicBool, time::Instant};

    fn open(path: &Path) -> Result<Repository> {
        crate::open_opts(path, crate::open::Options::isolated())
    }

    fn paths(path: &str) -> Option<Scope> {
        Some(Scope::Paths(vec![PathChange {
            path: path.into(),
            kind: plumbing::EventKind::Any,
            path_kind: plumbing::PathKind::Any,
        }]))
    }

    #[test]
    fn returned_watches_keep_the_checkouts_opening_policy_until_coverage_ends() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_submodules.sh")?;
        let root = fixture.join("root");
        let repo = open(&root)?;
        let mut project = repo.monitor(Default::default())?;
        let ignored = crate::open_opts(
            &root,
            crate::open::Options::isolated().config_overrides(["diff.ignoreSubmodules=all"]),
        )?;
        let mut handle = project.worktree_status(&ignored, Default::default())?;
        let interrupt = AtomicBool::new(false);
        handle.refresh(&ignored, &interrupt)?;
        project.publish_status(&Outcome {
            changes: Changes {
                configuration: true,
                ..Default::default()
            },
            ..Default::default()
        });
        drop(handle);
        let now = Instant::now();
        assert_eq!(
            project.retained_deadline(now),
            Some(now),
            "queued configuration survives handoff"
        );
        let mut out = Outcome::default();
        project.service_retained(now, &mut out);
        assert!(out.errors.is_empty(), "rediscovery succeeds: {:?}", out.errors);
        assert!(
            !lock(&project.status.retained)[&project.layout.git_dir]
                .watches
                .has_children(),
            "rediscovery honors the caller's configuration override"
        );
        let mut handle = project.worktree_status(&repo, Default::default())?;
        assert!(
            lock(&project.status.retained).is_empty(),
            "the handle takes over the watches"
        );
        handle.refresh(&repo, &interrupt)?;
        drop(handle);
        assert!(
            lock(&project.status.retained)[&project.layout.git_dir]
                .watches
                .has_children(),
            "the ordinary checkout observes initialized submodules"
        );
        project.update_checkout_filter(Some(Vec::new()))?;
        project.service(Instant::now(), || open(&root));
        assert!(
            lock(&project.status.retained).is_empty(),
            "uncovered checkouts release their watches"
        );
        Ok(())
    }

    #[test]
    fn selected_status_routes_only_its_checkout_scopes_and_releases_removed_targets() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
        let main = open(&fixture.join("main"))?;
        let selected = open(&fixture.join("linked-a"))?;
        let foreign = open(&fixture.join("linked-b"))?;
        let project = main.monitor(super::super::Options {
            all_worktrees: true,
            ..Default::default()
        })?;
        let mut monitor = project.worktree_status(&selected, Default::default())?;
        let sibling = project.worktree_status(&foreign, Default::default())?;
        monitor.refresh(&selected, &AtomicBool::new(false))?;
        let make_other = |repo: &Repository, kind| crate::notify::WorktreeChanges {
            git_dir: repo.git_dir().canonicalize().expect("fixture Git directories exist"),
            worktree: repo
                .workdir()
                .expect("linked fixtures have checkouts")
                .canonicalize()
                .expect("fixture worktrees exist"),
            kind,
        };
        let changed_path = |repo, path| {
            make_other(
                repo,
                WorktreeChange::Changed(Changes {
                    worktree: paths(path),
                    ..Default::default()
                }),
            )
        };
        project.publish_status(&Outcome {
            changes: Changes {
                worktree: paths("old-primary-path"),
                index: true,
                ..Default::default()
            },
            other_worktrees: vec![changed_path(&foreign, "foreign-path")],
            ..Default::default()
        });
        assert!(
            !monitor.needs_refresh(),
            "coordinator and foreign relative paths cannot be relabeled for selected status"
        );
        for path in ["selected-path", "another-path"] {
            project.publish_status(&Outcome {
                other_worktrees: vec![changed_path(&selected, path)],
                ..Default::default()
            });
        }
        let Some(Scope::Paths(delivered)) = monitor.service(Instant::now()).changes.worktree else {
            panic!("matching paths retain their checkout-relative scope");
        };
        assert_eq!(
            delivered.iter().map(|event| event.path.as_bstr()).collect::<Vec<_>>(),
            ["another-path", "selected-path"],
            "only matching checkout paths are delivered, sorted across project outcomes"
        );
        let mut primary = project.worktree_status(&main, Default::default())?;
        project.publish_status(&Outcome {
            changes: Changes {
                ignores: true,
                ..Default::default()
            },
            other_worktrees: vec![changed_path(&selected, "selected-path")],
            ..Default::default()
        });
        project.publish_status(&Outcome {
            changes: Changes {
                worktree: paths("primary-path"),
                ..Default::default()
            },
            ..Default::default()
        });
        for handle in [&mut primary, &mut monitor] {
            assert_eq!(
                handle.service(Instant::now()).changes.worktree,
                Some(Scope::All),
                "checkout paths in the same or a later outcome preserve a shared rule change's full scope"
            );
        }
        project.publish_status(&Outcome {
            other_worktrees: vec![changed_path(&selected, "queued")],
            ..Default::default()
        });
        let now = Instant::now();
        let foreign_project = main.monitor(Default::default())?;
        let mut current = Outcome::default();
        foreign_project.service_status_handles(now, &mut current, [&mut monitor]);
        assert_eq!(
            monitor.next_service_timeout(now),
            Some(Duration::ZERO),
            "another project cannot drain this inbox"
        );
        project.service_status_handles(now, &mut current, [&mut monitor]);
        assert!(
            current.changes.is_empty() && current.other_worktrees.is_empty(),
            "an earlier project outcome is not republished"
        );
        assert!(
            monitor.needs_refresh(),
            "the checkout still owes its consumer a snapshot"
        );
        monitor.refresh(&selected, &AtomicBool::new(false))?;
        project.publish_status(&crate::notify::Outcome {
            other_worktrees: vec![make_other(&selected, WorktreeChange::Removed)],
            ..Default::default()
        });
        assert!(
            monitor.is_closed() && monitor.snapshot().is_none(),
            "removing the selected checkout releases status state"
        );
        assert_eq!(
            monitor.next_timeout(Instant::now()),
            monitor.next_service_timeout(Instant::now()),
            "a removed target has no pending status deadline"
        );
        assert_eq!(
            project.layout().git_dir,
            main.git_dir().canonicalize()?,
            "target lifecycle leaves the project coordinator intact"
        );
        assert!(
            !sibling.is_closed(),
            "retiring one checkout leaves its sibling subscribed"
        );
        let weak = lock(&project.status.handles)[0].clone();
        drop(sibling);
        assert!(
            weak.upgrade().is_none(),
            "the project retains no status state after the handle is dropped"
        );
        Ok(())
    }
}
