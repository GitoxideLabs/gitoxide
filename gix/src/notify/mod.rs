//! Repository-aware filesystem monitoring.
//!
//! Notifications invalidate cached information; they do not describe an atomic Git transaction.
//! [`RepositoryMonitor`](crate::notify::RepositoryMonitor) owns paths and watchers, and borrows a freshly opened repository only
//! while updating its watch inventory. Call [`RepositoryMonitor::service()`](crate::notify::RepositoryMonitor::service) regularly and use
//! [`RepositoryMonitor::next_timeout()`](crate::notify::RepositoryMonitor::next_timeout) when integrating it into an event loop.

use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, ResultExt, message};
use gix_notify::{Event, EventKind, PathKind};

use crate::{Repository, bstr::BString};

mod inventory;
#[cfg(test)]
mod tests;

/// An error encountered while preparing or maintaining repository watches.
pub type Error = gix_error::Exn<gix_error::Message>;

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_SCOPES: usize = 4096;
const MAX_SCOPE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SAMPLE_PATHS: usize = 16;
const MAX_SAMPLE_BYTES: usize = 4096;

/// The repository data affected by a configuration dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceKind {
    /// A configuration file, including an active include or a missing optional root.
    Configuration,
    /// A global or repository-local ignore file.
    Ignore,
    /// A global or repository-local attributes file.
    Attributes,
}

/// A resolved filesystem dependency, including files that do not exist yet.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Source {
    /// The dependency's filesystem path.
    pub path: PathBuf,
    /// What must be refreshed when the file changes.
    pub kind: SourceKind,
}

/// Detached paths describing one repository and its optional worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layout {
    /// The current worktree's administrative directory.
    pub git_dir: PathBuf,
    /// The directory shared by linked worktrees.
    pub common_dir: PathBuf,
    /// The working directory, absent for bare repositories.
    pub worktree: Option<PathBuf>,
    /// The selected index, including an explicitly configured alternative.
    pub index: PathBuf,
    /// Configuration, ignore and attribute dependencies.
    pub sources: Vec<Source>,
}

/// Scheduling and worktree-subscription options.
#[derive(Clone, Debug)]
pub struct Options {
    /// Observe the worktree and index when a worktree exists.
    pub worktree: bool,
    /// Wait this long after a metadata event before publishing its invalidation.
    pub metadata_debounce: Duration,
    /// Publish a continuous metadata burst within this duration of its first observed event.
    pub max_latency: Duration,
    /// Retry failed repository discovery and watcher registration after this interval.
    pub retry_interval: Duration,
    /// Periodically rediscover watches and invalidate all cached data. `None` disables this.
    pub safety_interval: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            worktree: true,
            metadata_debounce: Duration::from_millis(100),
            max_latency: Duration::from_millis(250),
            retry_interval: Duration::from_secs(5),
            safety_interval: Some(Duration::from_secs(60)),
        }
    }
}

/// Literal, byte-preserving repository-relative paths to invalidate.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Scope {
    /// No worktree invalidation.
    #[default]
    None,
    /// Paths and their descendants may have changed.
    Paths(Vec<BString>),
    /// All worktree paths may have changed.
    All,
}

impl Scope {
    /// Return whether no worktree paths need refreshing.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub(crate) fn insert(&mut self, path: BString) {
        if matches!(self, Self::All) {
            return;
        }
        if let Self::None = self {
            *self = Self::Paths(Vec::new());
        }
        let Self::Paths(paths) = self else { return };
        if paths.contains(&path) {
            return;
        }
        if paths.len() >= MAX_SCOPES
            || paths.iter().map(|p| p.len()).sum::<usize>().saturating_add(path.len()) > MAX_SCOPE_BYTES
        {
            *self = Self::All;
        } else {
            paths.push(path);
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        match other {
            Self::None => {}
            Self::All => *self = Self::All,
            Self::Paths(paths) => paths.into_iter().for_each(|path| self.insert(path)),
        }
    }
}

/// Conservative invalidations that may be delivered together.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Changes {
    /// References or `HEAD` may have changed.
    pub references: bool,
    /// The current worktree index may have changed.
    pub index: bool,
    /// Configuration must be reloaded.
    pub configuration: bool,
    /// Ignore rules must be reloaded.
    pub ignores: bool,
    /// Attribute rules must be reloaded.
    pub attributes: bool,
    /// Operation state, such as a paused rebase, may have changed.
    pub operations: bool,
    /// Repository or linked-worktree membership/layout may have changed.
    pub layout: bool,
    /// Worktree paths to refresh.
    pub worktree: Scope,
}

impl Changes {
    /// Return whether no cached information was invalidated.
    pub fn is_empty(&self) -> bool {
        !self.references
            && !self.index
            && !self.configuration
            && !self.ignores
            && !self.attributes
            && !self.operations
            && !self.layout
            && self.worktree.is_none()
    }

    fn merge(&mut self, other: Self) {
        self.references |= other.references;
        self.index |= other.index;
        self.configuration |= other.configuration;
        self.ignores |= other.ignores;
        self.attributes |= other.attributes;
        self.operations |= other.operations;
        self.layout |= other.layout;
        self.worktree.merge(other.worktree);
    }

    fn metadata_all() -> Self {
        Self {
            references: true,
            configuration: true,
            ignores: true,
            attributes: true,
            operations: true,
            layout: true,
            ..Self::default()
        }
    }

    fn worktree_all() -> Self {
        Self {
            index: true,
            worktree: Scope::All,
            ..Self::default()
        }
    }
}

/// Bounded diagnostics accumulated until invalidations or failures are published.
#[derive(Clone, Debug, Default)]
pub struct Statistics {
    /// Number of native events received.
    pub received: usize,
    /// Number of native coverage losses observed.
    pub rescans: usize,
    /// Up to sixteen representative paths, using at most four KiB of path bytes.
    pub paths: Vec<PathBuf>,
    /// Path samples omitted because the sample budget was exhausted.
    pub omitted_paths: usize,
    /// Directory registrations added.
    pub added: usize,
    /// Directory registrations removed.
    pub removed: usize,
}

impl Statistics {
    pub(crate) fn merge(&mut self, other: Self) {
        self.received = self.received.saturating_add(other.received);
        self.rescans = self.rescans.saturating_add(other.rescans);
        self.omitted_paths = self.omitted_paths.saturating_add(other.omitted_paths);
        self.added = self.added.saturating_add(other.added);
        self.removed = self.removed.saturating_add(other.removed);
        for path in other.paths {
            self.sample(&path);
        }
    }

    fn observe(&mut self, event: &Event) {
        self.received += 1;
        for path in &event.paths {
            self.sample(path);
        }
    }

    fn sample(&mut self, path: &Path) {
        if self.paths.iter().any(|sample| sample == path) {
            return;
        }
        let bytes = self
            .paths
            .iter()
            .map(|p| p.as_os_str().as_encoded_bytes().len())
            .sum::<usize>();
        if self.paths.len() < MAX_SAMPLE_PATHS
            && bytes.saturating_add(path.as_os_str().as_encoded_bytes().len()) <= MAX_SAMPLE_BYTES
        {
            self.paths.push(path.to_owned());
        } else {
            self.omitted_paths = self.omitted_paths.saturating_add(1);
        }
    }
}

/// Invalidations and failures from a service call. Failures never discard simultaneous changes.
#[derive(Default)]
pub struct Outcome {
    /// Cached information to refresh.
    pub changes: Changes,
    /// Discovery or native watcher failures, retained with their error chains.
    pub errors: Vec<Error>,
    /// Bounded native-event diagnostics.
    pub statistics: Statistics,
}

#[derive(Default)]
struct Pending {
    changes: Changes,
    first: Option<Instant>,
    deadline: Option<Instant>,
}

impl Pending {
    fn add(&mut self, changes: Changes, now: Instant, options: &Options, immediate: bool) {
        if changes.is_empty() {
            return;
        }
        self.changes.merge(changes);
        let first = *self.first.get_or_insert(now);
        let deadline = if immediate {
            now
        } else {
            (now + options.metadata_debounce).min(first + options.max_latency)
        };
        self.deadline = Some(
            self.deadline
                .filter(|old| *old <= now)
                .map_or(deadline, |old| old.min(deadline)),
        );
    }

    fn take_due(&mut self, now: Instant) -> Changes {
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            return std::mem::take(self).changes;
        }
        Changes::default()
    }
}

#[derive(Default)]
struct Domain {
    watcher: Option<gix_notify::Watcher>,
    registered: BTreeMap<PathBuf, bool>,
    rebuild: bool,
}

impl Domain {
    fn replace(&mut self, desired: BTreeMap<PathBuf, bool>, statistics: &mut Statistics) -> Result<bool, Error> {
        if self.rebuild {
            self.watcher = None;
        }
        let changed = self.watcher.is_none() || desired != self.registered;
        if !changed {
            return Ok(false);
        }
        let watcher = match self.watcher.as_mut() {
            Some(watcher) => watcher,
            None => self
                .watcher
                .insert(gix_notify::Watcher::new(gix_notify::Options::default())?),
        };
        if let Err(err) = watcher.replace(desired.iter().map(|(path, recursive)| gix_notify::Watch {
            path: path.clone(),
            recursive: *recursive,
        })) {
            // A partial native update cannot certify the previous coverage either.
            self.rebuild = true;
            return Err(err);
        }
        statistics.added += desired
            .iter()
            .filter(|(path, mode)| self.registered.get(*path) != Some(*mode))
            .count();
        statistics.removed += self
            .registered
            .iter()
            .filter(|(path, mode)| desired.get(*path) != Some(*mode))
            .count();
        self.registered = desired;
        self.rebuild = false;
        Ok(true)
    }
}

/// A detached repository monitor with independent metadata and worktree native streams.
pub struct RepositoryMonitor {
    layout: Layout,
    options: Options,
    metadata: Domain,
    worktree: Domain,
    directories: HashSet<PathBuf>,
    nested_repositories: HashSet<PathBuf>,
    projection: Vec<inventory::IndexEntry>,
    pending_metadata: Pending,
    pending_worktree: Changes,
    pending_statistics: Statistics,
    metadata_dirty: bool,
    inventory_dirty: bool,
    index_dirty: bool,
    maintenance: Option<Instant>,
    safety: Option<Instant>,
    backlog: bool,
    precompose_unicode: bool,
    ignore_case: bool,
}

impl Repository {
    /// Observe this repository without keeping its object database or configuration caches alive.
    pub fn monitor(&self, options: Options) -> Result<RepositoryMonitor, Error> {
        if options.retry_interval.is_zero() || options.safety_interval.is_some_and(|interval| interval.is_zero()) {
            return Err(message("monitor retry and safety intervals must be positive").raise());
        }
        let layout = Layout::from_repository(self)?;
        let filesystem = self
            .filesystem_options()
            .or_raise(|| message("could not read filesystem options"))?;
        Ok(RepositoryMonitor {
            layout,
            options,
            metadata: Domain::default(),
            worktree: Domain::default(),
            directories: HashSet::new(),
            nested_repositories: HashSet::new(),
            projection: Vec::new(),
            pending_metadata: Pending::default(),
            pending_worktree: Changes::default(),
            pending_statistics: Statistics::default(),
            metadata_dirty: true,
            inventory_dirty: true,
            index_dirty: true,
            maintenance: Some(Instant::now()),
            safety: None,
            backlog: false,
            precompose_unicode: filesystem.precompose_unicode,
            ignore_case: filesystem.ignore_case,
        })
    }
}

impl Layout {
    fn from_repository(repo: &Repository) -> Result<Self, Error> {
        let precompose = repo
            .filesystem_options()
            .or_raise(|| message("could not read filesystem options"))?
            .precompose_unicode;
        let absolute = |path: &Path| absolute_path(path, repo.current_dir(), precompose);
        let mut sources = Vec::new();
        for source in repo.notification_sources()? {
            // Observe a symlink's target as well as replacement of the link itself.
            if let (Some(parent), Some(name)) = (source.path.parent(), source.path.file_name()) {
                let parent = if parent.as_os_str().is_empty() {
                    repo.current_dir()
                } else {
                    parent
                };
                sources.push(Source {
                    path: absolute(parent)?.join(name),
                    kind: source.kind,
                });
            }
            sources.push(Source {
                path: absolute(&source.path)?,
                kind: source.kind,
            });
        }
        sources.sort();
        sources.dedup();
        Ok(Self {
            git_dir: absolute(repo.git_dir())?,
            common_dir: absolute(repo.common_dir())?,
            worktree: repo.workdir().map(absolute).transpose()?,
            index: absolute(&repo.index_path())?,
            sources,
        })
    }
}

pub(crate) fn absolute_path(path: &Path, current_dir: &Path, precompose: bool) -> Result<PathBuf, Error> {
    // Resolve symlinks while preserving missing dependency suffixes, then obtain the OS spelling
    // of the existing ancestor. On macOS, aliases can differ in case and Unicode normalization.
    let path = gix_path::realpath_opts(path, current_dir, gix_path::realpath::MAX_SYMLINKS)
        .or_raise(|| message("could not resolve repository monitor path"))?;
    let mut ancestor = path.as_path();
    let physical = loop {
        match ancestor.canonicalize() {
            Ok(physical) => {
                break physical.join(
                    path.strip_prefix(ancestor)
                        .expect("ancestor belongs to the resolved path"),
                );
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) && let Some(parent) = ancestor.parent() =>
            {
                ancestor = parent;
            }
            Err(error) => return Err(error.and_raise(message("could not canonicalize repository monitor path"))),
        }
    };
    let path = std::borrow::Cow::Owned(physical);
    Ok(if precompose {
        gix_utils::str::precompose_path(path).into_owned()
    } else {
        path.into_owned()
    })
}

fn strip_prefix<'a>(path: &'a Path, prefix: &Path, ignore_case: bool) -> Option<&'a Path> {
    if !ignore_case {
        return path.strip_prefix(prefix).ok();
    }
    let mut components = path.components();
    for expected in prefix.components() {
        if !components
            .next()?
            .as_os_str()
            .as_encoded_bytes()
            .eq_ignore_ascii_case(expected.as_os_str().as_encoded_bytes())
        {
            return None;
        }
    }
    Some(components.as_path())
}

fn same_path(a: &Path, b: &Path, ignore_case: bool) -> bool {
    strip_prefix(a, b, ignore_case).is_some_and(|relative| relative.as_os_str().is_empty())
}

impl RepositoryMonitor {
    /// The paths currently being monitored.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Whether all enabled subscriptions are installed and their inventory is verified.
    ///
    /// Pending maintenance, retries, and registration verification keep this false, even when
    /// the latest [`service()`](Self::service) call has no new errors to report.
    pub fn is_healthy(&self) -> bool {
        self.maintenance.is_none()
            && self.metadata.watcher.is_some()
            && !self.metadata.rebuild
            && (!self.worktree_enabled() || self.worktree.watcher.is_some() && !self.worktree.rebuild)
    }

    /// Enable or disable worktree and index monitoring while retaining metadata monitoring.
    pub fn set_worktree_enabled(&mut self, enabled: bool) {
        if self.options.worktree == enabled {
            return;
        }
        self.options.worktree = enabled;
        self.worktree = Domain::default();
        self.directories.clear();
        self.nested_repositories.clear();
        self.projection.clear();
        self.pending_worktree = Changes::default();
        if !enabled {
            self.pending_metadata.changes.index = false;
            self.pending_metadata.changes.worktree = Scope::None;
        }
        self.inventory_dirty = true;
        self.index_dirty = true;
        self.maintenance = Some(Instant::now());
    }

    /// Rebuild native watches and invalidate caches at the next service call.
    pub fn rescan(&mut self) {
        self.metadata.rebuild = true;
        self.worktree.rebuild = true;
        self.request_inventory();
    }

    /// Switch to a new repository layout, discarding notifications from the previous one.
    pub fn reconfigure(&mut self, repo: &Repository) -> Result<(), Error> {
        let replacement = repo.monitor(self.options.clone())?;
        *self = replacement;
        Ok(())
    }

    fn worktree_enabled(&self) -> bool {
        self.options.worktree && self.layout.worktree.is_some()
    }

    fn request_inventory(&mut self) {
        self.metadata_dirty = true;
        self.inventory_dirty = true;
        self.index_dirty = true;
        self.maintenance = Some(Instant::now());
    }

    /// Process a bounded event batch, maintain watches when due, and publish invalidations.
    ///
    /// `opener` must return a freshly opened repository using the caller's intended open options.
    /// It is called at most once, only when the watch inventory needs fresh repository data.
    pub fn service(&mut self, now: Instant, mut opener: impl FnMut() -> Result<Repository, gix_error::Exn>) -> Outcome {
        let mut out = Outcome::default();
        self.backlog = false;
        for worktree in [false, true] {
            let domain = if worktree {
                &mut self.worktree
            } else {
                &mut self.metadata
            };
            let Some(watcher) = domain.watcher.as_mut() else {
                continue;
            };
            let batch = watcher.drain(gix_notify::Budget {
                max_events: 256,
                max_bytes: 1024 * 1024,
            });
            self.backlog |= batch.more;
            if let Some(loss) = batch.loss {
                out.statistics.rescans += 1;
                if matches!(loss, gix_notify::Loss::BackendError | gix_notify::Loss::BackendStopped) {
                    domain.rebuild = true;
                }
                // Successful replacement already published invalidation and queued verification.
                if loss != gix_notify::Loss::WatchSetChanged {
                    self.coverage_lost(worktree, now);
                }
            }
            for event in batch.events {
                out.statistics.observe(&event);
                self.observe(&event, worktree, now);
            }
            if !batch.errors.is_empty() {
                self.coverage_lost(worktree, now);
                let domain = if worktree {
                    &mut self.worktree
                } else {
                    &mut self.metadata
                };
                domain.watcher = None;
                domain.registered.clear();
                out.errors.extend(batch.errors);
                self.maintenance = Some(now + self.options.retry_interval);
            }
        }
        if let Some(interval) = self.options.safety_interval {
            match self.safety {
                None => self.safety = Some(now + interval),
                Some(deadline) if deadline <= now => {
                    self.request_inventory();
                    self.maintenance = Some(now);
                    self.safety = Some(now + interval);
                    self.pending_metadata
                        .add(Changes::metadata_all(), now, &self.options, true);
                    if self.worktree_enabled() {
                        self.pending_worktree.merge(Changes::worktree_all());
                    }
                }
                Some(_) => {}
            }
        }
        if self.maintenance.is_some_and(|deadline| deadline <= now) {
            self.maintenance = None;
            match opener()
                .or_raise(|| message("could not reopen repository for filesystem monitoring"))
                .and_then(|repo| self.maintain(&repo, now, &mut out.statistics, &mut out.errors))
            {
                Ok(()) => {}
                Err(err) => {
                    out.errors.push(err);
                }
            }
            if !out.errors.is_empty() {
                self.pending_metadata
                    .add(Changes::metadata_all(), now, &self.options, true);
                if self.worktree_enabled() {
                    self.pending_worktree.merge(Changes::worktree_all());
                }
                self.maintenance = Some(now + self.options.retry_interval);
            }
        }
        out.changes.merge(self.pending_metadata.take_due(now));
        out.changes.merge(std::mem::take(&mut self.pending_worktree));
        if let Scope::Paths(paths) = &mut out.changes.worktree {
            paths.sort();
        }
        self.pending_statistics.merge(std::mem::take(&mut out.statistics));
        if !out.changes.is_empty() || !out.errors.is_empty() {
            out.statistics = std::mem::take(&mut self.pending_statistics);
        }
        out
    }

    /// The maximum interval before the next service call, including the native polling ceiling.
    pub fn next_timeout(&self, now: Instant) -> Option<Duration> {
        if self.backlog {
            return Some(Duration::ZERO);
        }
        let poll = (self.metadata.watcher.is_some() || self.worktree.watcher.is_some()).then_some(POLL_INTERVAL);
        [self.maintenance, self.safety, self.pending_metadata.deadline]
            .into_iter()
            .flatten()
            .map(|deadline| deadline.saturating_duration_since(now))
            .chain(poll)
            .min()
    }

    fn coverage_lost(&mut self, worktree: bool, now: Instant) {
        if worktree {
            self.pending_worktree.merge(Changes::worktree_all());
            self.inventory_dirty = true;
            self.index_dirty = true;
        } else {
            self.pending_metadata
                .add(Changes::metadata_all(), now, &self.options, true);
            self.metadata_dirty = true;
            self.inventory_dirty = true;
            self.index_dirty = true;
            if self.worktree_enabled() {
                self.pending_worktree.merge(Changes::worktree_all());
            }
        }
        self.maintenance = Some(now);
    }

    fn maintain(
        &mut self,
        repo: &Repository,
        now: Instant,
        statistics: &mut Statistics,
        errors: &mut Vec<Error>,
    ) -> Result<(), Error> {
        let layout = Layout::from_repository(repo)?;
        if self.layout != layout {
            self.metadata_dirty = true;
            self.inventory_dirty = true;
            self.index_dirty = true;
            if self.layout.git_dir != layout.git_dir
                || self.layout.common_dir != layout.common_dir
                || self.layout.worktree != layout.worktree
                || self.layout.index != layout.index
            {
                self.metadata.rebuild = true;
                self.worktree.rebuild = true;
            }
            self.layout = layout;
            self.pending_metadata
                .add(Changes::metadata_all(), now, &self.options, true);
            if self.worktree_enabled() {
                self.pending_worktree.merge(Changes::worktree_all());
            }
        }
        let filesystem = repo
            .filesystem_options()
            .or_raise(|| message("could not read filesystem options"))?;
        self.precompose_unicode = filesystem.precompose_unicode;
        self.ignore_case = filesystem.ignore_case;
        let mut verify = false;
        if self.metadata_dirty || self.metadata.watcher.is_none() || self.metadata.rebuild {
            match self
                .metadata
                .replace(inventory::metadata_watches(&self.layout), statistics)
            {
                Ok(changed) => {
                    self.metadata_dirty = false;
                    if changed {
                        self.pending_metadata
                            .add(Changes::metadata_all(), now, &self.options, true);
                        if self.worktree_enabled() {
                            self.pending_worktree.merge(Changes::worktree_all());
                        }
                        verify = true;
                    }
                }
                Err(err) => errors.push(err),
            }
        }
        if self.worktree_enabled()
            && (self.inventory_dirty || self.index_dirty || self.worktree.watcher.is_none() || self.worktree.rebuild)
        {
            let index = repo
                .index_or_empty()
                .or_raise(|| message("could not load index for filesystem monitoring"))?;
            let projection = inventory::index_projection(&index);
            let changed_index = self.index_dirty && inventory::index_changes_directories(&self.projection, &projection);
            if self.inventory_dirty || changed_index || self.worktree.watcher.is_none() || self.worktree.rebuild {
                let directories = inventory::worktree_directories(repo, &index)?;
                let mut watches: BTreeMap<_, _> = directories.paths.iter().cloned().map(|path| (path, false)).collect();
                inventory::watch_parent(&mut watches, &self.layout.index);
                if self.worktree.replace(watches, statistics)? {
                    self.pending_worktree.merge(Changes::worktree_all());
                    verify = true;
                }
                self.directories = directories.paths;
                self.nested_repositories = directories.repositories;
            }
            if self.index_dirty {
                self.projection = projection;
            }
            self.index_dirty = false;
            self.inventory_dirty = false;
        }
        if verify {
            // Installation can race discovery. Verify on the next call, never loop under churn.
            self.metadata_dirty = true;
            self.inventory_dirty = true;
            self.maintenance = Some(now);
        }
        Ok(())
    }

    fn observe(&mut self, event: &Event, worktree: bool, now: Instant) {
        if event.paths.is_empty() || matches!(event.kind, EventKind::Any) {
            self.coverage_lost(worktree, now);
            return;
        }
        let mut changes = Changes::default();
        for original in &event.paths {
            let path = if self.precompose_unicode {
                gix_utils::str::precompose_path(original.as_path().into())
            } else {
                original.as_path().into()
            };
            let path = path.as_ref();
            if worktree {
                let index_lock = || {
                    let mut name = self.layout.index.as_os_str().to_os_string();
                    name.push(".lock");
                    PathBuf::from(name)
                };
                if same_path(path, &self.layout.index, self.ignore_case)
                    || event.kind == EventKind::Rename && same_path(path, &index_lock(), self.ignore_case)
                {
                    changes.merge(Changes::worktree_all());
                    self.index_dirty = true;
                    self.maintenance = Some(now);
                    continue;
                }
                let Some(root) = self.layout.worktree.as_deref() else {
                    continue;
                };
                if strip_prefix(path, root, self.ignore_case).is_none()
                    || strip_prefix(path, &self.layout.git_dir, self.ignore_case).is_some()
                    || strip_prefix(path, &root.join(".git"), self.ignore_case).is_some()
                {
                    continue;
                }
                let registered_directory = self
                    .directories
                    .iter()
                    .any(|directory| same_path(path, directory, self.ignore_case));
                let is_directory = event.path_kind == PathKind::Directory
                    || registered_directory
                    || std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
                if is_directory
                    && matches!(
                        event.kind,
                        EventKind::Create | EventKind::Remove | EventKind::Rename | EventKind::Modify
                    )
                {
                    self.inventory_dirty = true;
                    self.maintenance = Some(now);
                    if registered_directory && matches!(event.kind, EventKind::Remove | EventKind::Rename) {
                        self.worktree.rebuild = true;
                    }
                }
                let name = path.file_name().map(std::ffi::OsStr::as_encoded_bytes);
                let name = name.map(|name| {
                    if self.ignore_case {
                        std::borrow::Cow::Owned(name.to_ascii_lowercase())
                    } else {
                        std::borrow::Cow::Borrowed(name)
                    }
                });
                let name = name.as_deref();
                let scope = if let Some(repository) = self
                    .nested_repositories
                    .iter()
                    .find(|repository| strip_prefix(path, repository, self.ignore_case).is_some())
                {
                    if name == Some(b".git".as_slice()) {
                        self.inventory_dirty = true;
                        self.maintenance = Some(now);
                    }
                    repository.as_path()
                } else {
                    match name {
                        Some(b".git") => {
                            self.inventory_dirty = true;
                            self.maintenance = Some(now);
                            path.parent().unwrap_or(root)
                        }
                        Some(b".gitignore") => {
                            changes.ignores = true;
                            self.inventory_dirty = true;
                            self.maintenance = Some(now);
                            path.parent().unwrap_or(root)
                        }
                        Some(b".gitattributes" | b".gitmodules") => {
                            changes.attributes = true;
                            changes.worktree = Scope::All;
                            continue;
                        }
                        _ => path,
                    }
                };
                match strip_prefix(scope, root, self.ignore_case) {
                    Some(relative)
                        if !relative.as_os_str().is_empty()
                            && relative
                                .components()
                                .all(|c| matches!(c, std::path::Component::Normal(_))) =>
                    {
                        match gix_path::try_into_bstr(relative) {
                            Ok(relative) => changes
                                .worktree
                                .insert(gix_path::to_unix_separators_on_windows(relative).into_owned()),
                            Err(_) => changes.worktree = Scope::All,
                        }
                    }
                    _ => changes.worktree = Scope::All,
                }
            } else {
                if path
                    .file_name()
                    .is_some_and(|name| name.as_encoded_bytes().ends_with(b".lock"))
                    && !matches!(event.kind, EventKind::Rename)
                    && !self
                        .layout
                        .sources
                        .iter()
                        .any(|source| same_path(path, &source.path, self.ignore_case))
                {
                    continue;
                }
                if matches!(event.kind, EventKind::Remove | EventKind::Rename)
                    && self
                        .metadata
                        .registered
                        .keys()
                        .any(|registered| strip_prefix(registered, path, self.ignore_case).is_some())
                {
                    self.metadata.rebuild = true;
                    self.metadata_dirty = true;
                    self.maintenance = Some(now);
                }
                let (classified, refresh) = classify_metadata(path, &self.layout, self.ignore_case);
                if refresh {
                    self.metadata_dirty = true;
                    if classified.configuration || classified.ignores || classified.attributes || classified.layout {
                        self.inventory_dirty = true;
                        self.index_dirty = true;
                    }
                    self.maintenance = Some(now);
                }
                changes.merge(classified);
            }
        }
        if worktree {
            self.pending_worktree.merge(changes);
        } else {
            if self.worktree_enabled() && (changes.configuration || changes.ignores || changes.attributes) {
                changes.worktree = Scope::All;
                changes.index = true;
            }
            self.pending_metadata.add(changes, now, &self.options, false);
        }
    }
}

fn classify_metadata(path: &Path, layout: &Layout, ignore_case: bool) -> (Changes, bool) {
    let equal = |a: &Path, b: &Path| same_path(a, b, ignore_case);
    let beneath = |a: &Path, b: &Path| strip_prefix(a, b, ignore_case).is_some();
    let mut changes = Changes::default();
    let mut refresh = false;
    for source in &layout.sources {
        if beneath(&source.path, path) {
            match source.kind {
                SourceKind::Configuration => changes.configuration = true,
                SourceKind::Ignore => changes.ignores = true,
                SourceKind::Attributes => changes.attributes = true,
            }
            refresh = true;
        }
    }
    if equal(path, &layout.index)
        || equal(path, &layout.git_dir.join("index.lock"))
        || equal(path, &layout.common_dir.join("index"))
        || equal(path, &layout.common_dir.join("index.lock"))
    {
        return (changes, refresh);
    }
    let worktrees = layout.common_dir.join("worktrees");
    if let Some(relative) = strip_prefix(path, &worktrees, ignore_case) {
        if beneath(path, &layout.git_dir) && layout.git_dir != layout.common_dir {
            // The current worktree's metadata is classified below.
        } else {
            let components: Vec<_> = relative.components().collect();
            if components.len() <= 1 {
                changes.layout = true;
                changes.references = true;
                refresh = true;
            } else if components.len() == 2
                && (equal(Path::new(components[1].as_os_str()), Path::new("HEAD"))
                    || equal(Path::new(components[1].as_os_str()), Path::new("gitdir")))
            {
                changes.references = true;
                changes.layout = true;
            }
            return (changes, refresh);
        }
    }
    for directory in [&layout.git_dir, &layout.common_dir] {
        if let Some(relative) = strip_prefix(path, directory, ignore_case) {
            let count = relative.components().count();
            if count == 0 {
                changes.merge(Changes::metadata_all());
                refresh = true;
            } else if beneath(relative, Path::new("refs")) {
                changes.references = true;
                refresh |= count == 1;
            } else if beneath(relative, Path::new("rebase-apply"))
                || beneath(relative, Path::new("rebase-merge"))
                || beneath(relative, Path::new("sequencer"))
            {
                changes.operations = true;
                refresh |= count == 1;
            } else if count == 1 {
                let name = relative.as_os_str().as_encoded_bytes();
                let known: &[&[u8]] = &[
                    b"config",
                    b"config.worktree",
                    b"commondir",
                    b"gitdir",
                    b"HEAD",
                    b"ORIG_HEAD",
                    b"FETCH_HEAD",
                    b"packed-refs",
                    b"shallow",
                    b"MERGE_HEAD",
                    b"REBASE_HEAD",
                    b"CHERRY_PICK_HEAD",
                    b"REVERT_HEAD",
                    b"AUTO_MERGE",
                    b"MERGE_MSG",
                    b"MERGE_MODE",
                    b"SQUASH_MSG",
                    b"BISECT_LOG",
                    b"BISECT_START",
                ];
                let name = if ignore_case {
                    known
                        .iter()
                        .copied()
                        .find(|candidate| name.eq_ignore_ascii_case(candidate))
                        .unwrap_or(name)
                } else {
                    name
                };
                match name {
                    b"config" | b"config.worktree" => {
                        changes.configuration = true;
                        refresh = true;
                    }
                    b"commondir" | b"gitdir" => {
                        changes.layout = true;
                        refresh = true;
                    }
                    b"HEAD" => {
                        changes.references = true;
                        refresh = true;
                    }
                    b"ORIG_HEAD" | b"FETCH_HEAD" | b"packed-refs" | b"shallow" => changes.references = true,
                    b"MERGE_HEAD" | b"REBASE_HEAD" | b"CHERRY_PICK_HEAD" | b"REVERT_HEAD" | b"AUTO_MERGE" => {
                        changes.references = true;
                        changes.operations = true;
                    }
                    b"MERGE_MSG" | b"MERGE_MODE" | b"SQUASH_MSG" | b"BISECT_LOG" | b"BISECT_START" => {
                        changes.operations = true;
                    }
                    _ => {}
                }
            }
        } else if beneath(directory, path) {
            changes.merge(Changes::metadata_all());
            refresh = true;
        }
    }
    if layout
        .worktree
        .as_ref()
        .is_some_and(|root| equal(path, &root.join(".git")) || equal(path, root))
    {
        changes.layout = true;
        refresh = true;
    }
    (changes, refresh)
}

#[cfg(feature = "status-monitor")]
impl RepositoryMonitor {
    /// Install or maintain native coverage without consuming pending notifications.
    /// This lets status establish its baseline after watches are installed while leaving
    /// semantic metadata events available to the application's next service call.
    pub(crate) fn prepare(&mut self, repo: &Repository, now: Instant) -> Result<(), Error> {
        let mut statistics = Statistics::default();
        let mut errors = Vec::new();
        self.maintenance = None;
        if let Err(error) = self.maintain(repo, now, &mut statistics, &mut errors) {
            errors.push(error);
        }
        self.pending_statistics.merge(statistics);
        if errors.is_empty() {
            return Ok(());
        }
        self.pending_metadata
            .add(Changes::metadata_all(), now, &self.options, true);
        if self.worktree_enabled() {
            self.pending_worktree.merge(Changes::worktree_all());
        }
        self.maintenance = Some(now + self.options.retry_interval);
        Err(errors.remove(0))
    }
}
