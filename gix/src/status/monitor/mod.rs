//! An owned status snapshot with detached repository and submodule monitoring.

use std::{
    collections::{BTreeMap, HashSet},
    ops::Bound,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, ResultExt, message};

use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
    notify::{Changes, RepositoryMonitor, Scope},
    status::{Item, Submodule, UntrackedFiles},
};

const CHILDREN_PER_SERVICE: usize = 8;
const MAX_ERRORS: usize = 8;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(250);

mod submodules;
#[cfg(test)]
mod tests;

/// A status collection or monitoring failure with its underlying cause.
pub type Error = gix_error::Exn<gix_error::Message>;

/// Control notification scheduling and the contents of the cached status snapshot.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Filesystem monitoring and safety-refresh options.
    pub repository: crate::notify::Options,
    /// How untracked paths are represented.
    pub untracked_files: UntrackedFiles,
    /// Which submodule changes contribute to status.
    pub submodules: Submodule,
    /// Optional worktree rename/copy detection. Incremental refreshes become full scans when enabled.
    pub worktree_rewrites: Option<gix_diff::Rewrites>,
}

/// Which parts of the snapshot were successfully recomputed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Update {
    /// Whether the owned status items changed.
    pub changed: bool,
    /// Whether staged changes were recomputed.
    pub staged: bool,
    /// The worktree paths recomputed, including any conservative scope expansion.
    ///
    /// Consumers with content-derived caches must refresh these paths even when `changed` is false.
    pub unstaged: Scope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Head {
    reference: Option<gix_ref::FullName>,
    commit_id: Option<gix_hash::ObjectId>,
}

impl Head {
    fn read(repo: &Repository) -> Result<Self, Error> {
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
}

/// An owned status snapshot and its filesystem subscriptions.
///
/// No repository, index, object database, or producer thread is retained between calls.
/// Call [`service()`](Self::service) from the event loop, then [`refresh()`](Self::refresh) when
/// [`refresh_due()`](Self::refresh_due) is true. A failed or interrupted refresh preserves the previous
/// snapshot and its pending invalidations. Initialized nested submodules are monitored recursively.
pub struct Monitor {
    repository: RepositoryMonitor,
    options: Options,
    snapshot: Option<Vec<Item>>,
    head: Option<Head>,
    head_dirty: bool,
    staged: bool,
    unstaged: Scope,
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
    pub fn new(repo: &Repository, options: Options) -> Result<Self, Error> {
        Ok(Self {
            repository: repo.monitor(options.repository.clone())?,
            options,
            snapshot: None,
            head: None,
            head_dirty: true,
            staged: true,
            unstaged: Scope::All,
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
    pub fn snapshot(&self) -> Option<&[Item]> {
        self.snapshot.as_deref()
    }

    /// Whether a refresh is needed, including checking if a reference event changed `HEAD`.
    pub fn is_dirty(&self) -> bool {
        self.snapshot.is_none() || self.head_dirty || self.staged || !self.unstaged.is_none() || self.submodules_dirty
    }

    /// Whether pending status can be retried now, honoring the retry interval after failure.
    pub fn refresh_due(&self, now: Instant) -> bool {
        self.is_dirty() && self.retry.is_none_or(|deadline| deadline <= now)
    }

    /// Inspect the root repository's detached watch layout.
    pub fn repository_monitor(&self) -> &RepositoryMonitor {
        &self.repository
    }

    /// Whether root and child subscriptions are verified and no submodule reconciliation is pending.
    ///
    /// This is a positive recovery signal for previously reported monitoring errors. A service
    /// call without new errors can still be waiting for a failed subscription's retry deadline.
    pub fn is_healthy(&self) -> bool {
        self.repository.is_healthy()
            && self.children.values().all(|child| child.monitor.is_healthy())
            && (!self.options.repository.worktree
                || self.repository.layout().worktree.is_none()
                || !self.submodules_dirty)
    }

    /// Enable or disable worktree subscriptions without releasing metadata subscriptions.
    pub fn set_worktree_enabled(&mut self, enabled: bool) {
        if self.options.repository.worktree == enabled {
            return;
        }
        self.options.repository.worktree = enabled;
        self.repository.set_worktree_enabled(enabled);
        self.children.clear();
        self.child_round_remaining = 0;
        self.child_poll = None;
        self.aliased_children = false;
        self.submodule_paths.clear();
        self.invalidate();
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
    pub fn reconfigure(&mut self, repo: &Repository) -> Result<(), Error> {
        *self = Self::new(repo, self.options.clone())?;
        Ok(())
    }

    /// Mark the complete snapshot and submodule inventory for refresh.
    pub fn invalidate(&mut self) {
        self.retry = None;
        self.staged = true;
        self.unstaged = Scope::All;
        self.head_dirty = true;
        self.submodules_dirty = true;
    }

    /// Apply invalidations from application-owned Git operations or other event sources.
    pub fn invalidate_changes(&mut self, changes: &Changes) {
        self.head_dirty |= changes.references;
        if changes.index || changes.configuration || changes.ignores || changes.attributes || changes.layout {
            self.staged = true;
            self.unstaged = Scope::All;
        } else {
            self.unstaged.merge(changes.worktree.clone());
        }
        self.submodules_dirty |= changes.index
            || changes.configuration
            || changes.attributes
            || changes.layout
            || scope_touches_modules(&changes.worktree, &self.submodule_paths);
    }

    /// Drain bounded notifications from the root and its initialized nested submodules.
    ///
    /// The root's `opener` is used only when its watch inventory needs maintenance. Child
    /// repositories reuse detached open options inherited through the submodule API.
    pub fn service(
        &mut self,
        now: Instant,
        opener: impl FnMut() -> Result<Repository, gix_error::Exn>,
    ) -> crate::notify::Outcome {
        let mut outcome = self.repository.service(now, opener);
        self.invalidate_changes(&outcome.changes);
        let mut children_scope = Scope::None;
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
                    children_scope = Scope::All;
                } else {
                    children_scope.insert(child.top_level.clone());
                }
            }
            self.submodules_dirty |= child_outcome.changes.index
                || child_outcome.changes.configuration
                || child_outcome.changes.attributes
                || child_outcome.changes.layout
                || scope_touches_modules_at(&child_outcome.changes.worktree, &child.mount, &self.submodule_paths);
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
        self.unstaged.merge(children_scope.clone());
        outcome.changes.worktree.merge(children_scope);
        outcome
    }

    /// The next service or enabled status-refresh deadline across the root and its submodules.
    ///
    /// Disabling worktree monitoring suspends status deadlines while preserving metadata service.
    pub fn next_timeout(&self, now: Instant) -> Option<Duration> {
        self.next_service_timeout(now)
            .into_iter()
            .chain(
                (self.options.repository.worktree && self.repository.layout().worktree.is_some() && self.is_dirty())
                    .then(|| {
                        self.retry
                            .map_or(Duration::ZERO, |deadline| deadline.saturating_duration_since(now))
                    }),
            )
            .min()
    }

    /// The next notification-service deadline, independent of pending status collection.
    ///
    /// Use this while the consumer cannot refresh status, such as when its status view is hidden.
    /// Metadata and child notifications continue to accumulate bounded invalidations.
    pub fn next_service_timeout(&self, now: Instant) -> Option<Duration> {
        if self.child_round_remaining != 0 {
            return Some(Duration::ZERO);
        }
        self.repository
            .next_timeout(now)
            .into_iter()
            .chain(self.child_poll.map(|deadline| deadline.saturating_duration_since(now)))
            .min()
    }

    /// Recompute pending status synchronously, borrowing repository resources only for this call.
    ///
    /// Refresh coverage is returned even when the status items compare equal: a modified file can
    /// change contents again without changing its status classification. Errors and interruption
    /// preserve the previous snapshot and leave the covered paths dirty for a later attempt.
    pub fn refresh(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<Update, Error> {
        let result = self.refresh_inner(repo, interrupt);
        self.retry = result
            .as_ref()
            .err()
            .map(|_| Instant::now() + self.options.repository.retry_interval);
        result
    }

    fn refresh_inner(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<Update, Error> {
        check_interrupted(interrupt)?;
        if !self.is_dirty() {
            return Ok(Update::default());
        }
        self.repository.prepare(repo, Instant::now())?;
        let head = if self.head_dirty || self.staged || self.snapshot.is_none() {
            Some(Head::read(repo)?)
        } else {
            None
        };
        let staged = self.staged || head.as_ref().is_some_and(|head| self.head.as_ref() != Some(head));
        let unstaged = self.effective_scope(repo)?;
        if self.submodules_dirty {
            self.reconcile_submodules(repo, interrupt)?;
        }
        let collect = |patterns, staged, unstaged| {
            repo.status(gix_features::progress::Discard)
                .or_raise(|| message("could not prepare cached status"))?
                .untracked_files(self.options.untracked_files)
                .index_worktree_submodules(self.options.submodules)
                .index_worktree_rewrites(self.options.worktree_rewrites)
                .index_worktree_options_mut(|options| {
                    options.sorting = Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .collect_internal(patterns, staged, unstaged, interrupt)
        };
        let mut replacement = Vec::new();
        if repo.workdir().is_some() {
            match &unstaged {
                Scope::All => replacement = collect(Vec::new(), staged, true)?,
                Scope::Paths(scopes) => {
                    if staged {
                        replacement = collect(Vec::new(), true, false)?;
                    }
                    let patterns = scopes
                        .iter()
                        .map(|scope| {
                            gix_pathspec::Pattern::from_literal(scope.as_slice(), gix_pathspec::MagicSignature::TOP)
                                .to_bstring()
                        })
                        .collect();
                    replacement.extend(collect(patterns, false, true)?);
                }
                Scope::None => {
                    if staged {
                        replacement = collect(Vec::new(), true, false)?;
                    }
                }
            }
        }
        check_interrupted(interrupt)?;
        let ignore_case = repo
            .filesystem_options()
            .or_raise(|| message("could not read status filesystem options"))?
            .ignore_case;
        let mut next = self.snapshot.clone().unwrap_or_default();
        next.retain(|item| match item {
            Item::TreeIndex(_) => !staged,
            Item::IndexWorktree(_) => !scope_contains(&unstaged, item.location(), ignore_case),
        });
        next.extend(replacement);
        next.sort_by(|a, b| {
            item_rank(a)
                .cmp(&item_rank(b))
                .then_with(|| a.location().cmp(b.location()))
        });
        let changed = self.snapshot.as_ref() != Some(&next);
        self.snapshot = Some(next);
        if let Some(head) = head {
            self.head = Some(head);
        }
        self.head_dirty = false;
        self.staged = false;
        self.unstaged = Scope::None;
        self.submodules_dirty = false;
        Ok(Update {
            changed,
            staged,
            unstaged,
        })
    }

    fn effective_scope(&self, repo: &Repository) -> Result<Scope, Error> {
        let Scope::Paths(scopes) = &self.unstaged else {
            return Ok(self.unstaged.clone());
        };
        if self.options.worktree_rewrites.is_some() {
            return Ok(Scope::All);
        }
        let defaults = repo
            .pathspec_defaults()
            .or_raise(|| message("could not read incremental status pathspec defaults"))?;
        if defaults.literal || defaults.signature.contains(gix_pathspec::MagicSignature::ICASE) {
            return Ok(Scope::All);
        }
        let index = repo
            .index_or_empty()
            .or_raise(|| message("could not read index for incremental status scopes"))?;
        let ignore_case = repo
            .filesystem_options()
            .or_raise(|| message("could not read status filesystem options"))?
            .ignore_case;
        let mut expanded = Vec::new();
        for scope in scopes {
            let path = gix_path::from_bstr(scope.as_bstr());
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Ok(Scope::All);
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
            Scope::None
        } else {
            Scope::Paths(expanded)
        })
    }

    fn reconcile_submodules(&mut self, repo: &Repository, interrupt: &AtomicBool) -> Result<(), Error> {
        if !self.options.repository.worktree || repo.workdir().is_none() {
            self.children.clear();
            self.submodule_paths.clear();
            self.child_round_remaining = 0;
            self.child_poll = None;
            self.aliased_children = false;
            return Ok(());
        }
        let discovered = submodules::discover(repo, &self.options, interrupt)?;
        let mut retain = HashSet::new();
        let mut first_error = None;
        for (key, prepared) in discovered.children {
            check_interrupted(interrupt)?;
            retain.insert(key.clone());
            let mut child = prepared.child;
            if let Some(old) = self.children.get_mut(&key)
                && old.monitor.layout() == child.monitor.layout()
            {
                old.monitor.set_worktree_enabled(child.worktree);
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

fn check_interrupted(interrupt: &AtomicBool) -> Result<(), Error> {
    if interrupt.load(Ordering::Relaxed) {
        Err(message("status refresh was interrupted").raise())
    } else {
        Ok(())
    }
}

fn item_rank(item: &Item) -> u8 {
    match item {
        Item::TreeIndex(_) => 0,
        Item::IndexWorktree(_) => 1,
    }
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

fn scope_contains(scope: &Scope, path: &BStr, ignore_case: bool) -> bool {
    match scope {
        Scope::None => false,
        Scope::All => true,
        Scope::Paths(scopes) => scopes
            .iter()
            .any(|scope| path_in_scope(path.as_bytes(), scope, ignore_case)),
    }
}

fn scope_touches_modules(scope: &Scope, modules: &[BString]) -> bool {
    match scope {
        Scope::None => false,
        Scope::All => true,
        Scope::Paths(paths) => paths.iter().any(|path| {
            modules
                .iter()
                .any(|module| path_in_scope(path, module, true) || path_in_scope(module, path, true))
        }),
    }
}

fn scope_touches_modules_at(scope: &Scope, mount: &BString, modules: &[BString]) -> bool {
    match scope {
        Scope::None => false,
        Scope::All => true,
        Scope::Paths(paths) => paths.iter().any(|path| {
            let full = submodules::join(mount, path);
            modules
                .iter()
                .filter(|module| module.len() > mount.len())
                .any(|module| path_in_scope(&full, module, true) || path_in_scope(module, &full, true))
        }),
    }
}
