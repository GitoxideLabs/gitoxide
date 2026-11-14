//! Repository-aware filesystem monitoring.
//!
//! Notifications invalidate cached information; they do not describe an atomic Git transaction.
//! [`RepositoryMonitor`](crate::notify::RepositoryMonitor) owns paths and watchers, and borrows a freshly
//! opened repository only while updating its watch inventory. Call
//! [`RepositoryMonitor::service()`](crate::notify::RepositoryMonitor::service) regularly and use
//! [`RepositoryMonitor::next_timeout()`](crate::notify::RepositoryMonitor::next_timeout) in an event loop.
//! Enable [`Options::all_worktrees`](crate::notify::Options::all_worktrees) to include other checkouts
//! without duplicating shared Git metadata watchers. Their private metadata and checkout changes
//! are identified separately in [`Outcome::other_worktrees`](crate::notify::Outcome::other_worktrees),
//! including checkout additions, moves and removals. [`Layout`](crate::notify::Layout) describes
//! only the selected checkout; use [`RepositoryMonitor::other_worktrees()`](crate::notify::RepositoryMonitor::other_worktrees)
//! after service for the additional discovered layouts, including temporarily inaccessible checkouts.
//!
//! Reftable notifications are conservative invalidations, not reference reads. Ignore-aware coverage
//! retains `gix`'s limitations for reftable branch-conditioned configuration.
//!
//! For raw native events, use [`plumbing::Watcher`](crate::notify::plumbing::Watcher) directly and
//! register the desired paths with [`replace()`](crate::notify::plumbing::Watcher::replace) before
//! establishing a baseline. The caller owns the
//! complete watch set, bounded draining, delivery fences and recovery after loss. Native watchers
//! do not interpret repository layout, apply ignore rules or maintain repository dependencies.
//!
//! [`WorktreeCoverage::IgnoreAware`](crate::notify::WorktreeCoverage::IgnoreAware) skips fully ignored
//! directories, preserving coverage for tracked
//! paths. Individual file invalidations are not filtered; callers check their current ignore rules.
//! Directory scopes and full invalidations still require reconciliation using current repository state.
//!
//! With `async-notify`, keep constructing monitors synchronously with [`Repository::monitor()`].
//! Use `RepositoryMonitor::service_async()` to move native registration and inventory work to a
//! Tokio blocking worker, and `RepositoryMonitor::ready_async()` to await notifications or shared
//! publication/recovery deadlines without polling. Service returns the owned monitor and the full
//! outcome; cancellation shuts down that stream after active work completes. Await
//! `RepositoryMonitor::shutdown_async()` for explicit cleanup. The raw
//! `plumbing::asynchronous::Watcher` remains runtime independent. Blocking APIs coexist.

use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use classify::{MetadataScope, classify_dependencies, classify_metadata, classify_metadata_scoped};
use gix_error::{ErrorExt, ResultExt, message};
use gix_hashtable::hash_table::HashTable;
/// Native notification events, budgets, fences and watcher primitives.
pub use gix_notify as plumbing;
use plumbing::{Event, EventKind, PathKind};

use crate::{Error, Repository, Result, bstr::BString};

#[cfg(feature = "async-notify")]
mod asynchronous;
mod classify;
mod inventory;
#[cfg(test)]
mod tests;
mod worktrees;

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_SCOPES: usize = 4096;
const MAX_SCOPE_BYTES: usize = 4 * 1024 * 1024;

/// The repository data affected by a filesystem dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DependencyKind {
    /// A configuration file, including an active include or a missing optional root.
    Configuration,
    /// An ignore file, including global excludes when ignore-aware coverage needs them.
    Ignore,
    /// A repository-local attributes file.
    Attributes,
}

/// A resolved filesystem dependency, including files that do not exist yet.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dependency {
    /// The dependency's filesystem path.
    pub path: PathBuf,
    /// What must be refreshed when the file changes.
    pub kind: DependencyKind,
}

/// Detached paths in native filesystem spelling describing one repository and its optional worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layout {
    /// The current worktree's administrative directory.
    pub git_dir: PathBuf,
    /// The directory shared by linked worktrees.
    pub common_dir: PathBuf,
    /// The working directory, absent for bare repositories.
    pub worktree: Option<PathBuf>,
    /// The selected index, watched when it is within the repository or worktree.
    /// Its absolute path is only resolved through symlinks when the selected monitoring needs it.
    pub index: PathBuf,
    /// Configuration, ignore and attribute dependencies used by the monitor.
    ///
    /// Includes configured pattern files even when missing, and the selected fallbacks for absent
    /// optional candidates. External ignore files receive shallow parent watches for ignore-aware
    /// worktree coverage; external configuration and attribute files are not watched.
    pub dependencies: Vec<Dependency>,
}

/// Which worktree directories receive native notification coverage.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorktreeCoverage {
    /// Use the index and ignore rules to select directories, stopping at nested repositories.
    ///
    /// Fully ignored directories without tracked paths receive no watches. Where the backend provides
    /// native recursive coverage, subtrees without exclusions or nested repositories use recursive
    /// watches. Otherwise, each included directory found by discovery receives a non-recursive watch.
    /// File invalidations are unfiltered, so callers must check individual exclusions.
    /// Initial discovery records directory names. Index/ignore changes reevaluate this inventory;
    /// ordinary staging only invalidates the index. Reenabled subtrees need local discovery,
    /// and directory changes reuse unaffected recursive coverage. The monitor retains directory names
    /// and a fingerprint of coverage-relevant index data, not individual file paths or ignore blobs.
    /// Global ignore dependencies are watched too. Loss requests a full baseline and discovery walk.
    #[default]
    IgnoreAware,
    /// Recursively observe the entire worktree, including ignored paths and nested repositories.
    ///
    /// No index or ignore evaluation is needed to select directories. Classified worktree scopes
    /// exclude administrative paths, including nested `.git` entries; [`plumbing::Watcher`] does not filter them.
    /// Use this when the consumer will filter invalidations using its own current repository state.
    All,
}

/// Scheduling and subscription options.
#[derive(Clone, Debug)]
pub struct Options {
    /// Publish repository metadata invalidations, including the index when worktree coverage is enabled.
    ///
    /// Defaults to `true`. If disabled, outcomes contain only worktree scopes, even after errors or
    /// coverage loss. Ignore-aware worktree coverage still watches configuration, `HEAD`, ignore,
    /// index and layout dependencies internally, without enumerating refs or linked worktrees.
    pub metadata: bool,

    /// Also observe the main and other linked worktrees, sharing one metadata subscription set.
    ///
    /// Defaults to `false` and requires `metadata = true`. Takes effect while worktree coverage is enabled.
    /// Each checkout uses its own index and ignore rules. Additional checkout changes are returned
    /// in [`Outcome::other_worktrees`]; the selected checkout retains [`Outcome::changes`].
    pub all_worktrees: bool,
    /// Observe the worktree when one exists, and its index if `metadata` is enabled.
    ///
    /// Defaults to `Some(WorktreeCoverage::IgnoreAware)`. `None` disables checkout coverage;
    /// if `metadata` is also disabled, servicing neither opens a repository nor installs watches.
    pub worktree_coverage: Option<WorktreeCoverage>,
    /// Wait this long after a metadata event before publishing its invalidation.
    pub metadata_debounce: Duration,
    /// Publish a continuous metadata burst within this duration of its first observed event.
    pub max_latency: Duration,
    /// Retry failed repository discovery and watcher registration after this interval.
    pub retry_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            metadata: true,
            all_worktrees: false,
            worktree_coverage: Some(WorktreeCoverage::default()),
            metadata_debounce: Duration::from_millis(100),
            max_latency: Duration::from_millis(250),
            retry_interval: Duration::from_secs(5),
        }
    }
}

/// A literal, byte-preserving repository-relative path to invalidate, with advisory change hints.
///
/// The filesystem remains authoritative. Hints do not describe an operation journal, and rename
/// endpoints are not paired. Coalescing preserves each hint only when all observations agree;
/// otherwise that hint becomes `Any` and stays unknown for the remainder of the batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathChange {
    /// The affected repository-relative path. Its descendants may need reconciliation as well.
    pub path: BString,
    /// The watcher's hint about what happened at this path.
    ///
    /// `Any` also represents mixed observations or an invalidation widened to an ancestor directory;
    /// it does not itself indicate native coverage loss.
    pub kind: plumbing::EventKind,
    /// The watcher's object-type hint, which must not be used to infer whether the path still exists.
    ///
    /// Widened ancestor-directory scopes use `Directory` rather than the triggering child's hint.
    pub path_kind: plumbing::PathKind,
}

/// Literal, byte-preserving repository-relative paths to invalidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scope {
    /// Paths and their descendants may have changed, with coalesced advisory hints.
    ///
    /// Monitor outcomes contain unique paths sorted by their bytes, not by event order.
    Paths(Vec<PathChange>),
    /// All worktree paths may have changed.
    All,
}

/// Conservative invalidations that may be delivered together.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Changes {
    /// References or `HEAD` may have changed.
    pub references: bool,
    /// The current worktree index may have changed (only with metadata and worktree enabled).
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
    /// Worktree paths to refresh, or `None` if no worktree invalidation is needed.
    pub worktree: Option<Scope>,
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
        match other.worktree {
            None => {}
            Some(Scope::All) => self.worktree = Some(Scope::All),
            Some(Scope::Paths(paths)) => paths.into_iter().for_each(|path| self.insert_worktree_path(path)),
        }
    }

    fn insert_worktree_path(&mut self, change: PathChange) {
        let Scope::Paths(paths) = self.worktree.get_or_insert_with(|| Scope::Paths(Vec::new())) else {
            return;
        };
        if let Some(previous) = paths.iter_mut().find(|previous| previous.path == change.path) {
            if previous.kind != change.kind {
                previous.kind = EventKind::Any;
            }
            if previous.path_kind != change.path_kind {
                previous.path_kind = PathKind::Any;
            }
            return;
        }
        if paths.len() >= MAX_SCOPES
            || paths
                .iter()
                .map(|p| p.path.len())
                .sum::<usize>()
                .saturating_add(change.path.len())
                > MAX_SCOPE_BYTES
        {
            self.worktree = Some(Scope::All);
        } else {
            paths.push(change);
        }
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
            worktree: Some(Scope::All),
            ..Self::default()
        }
    }
}

/// Event and registration counts accumulated until invalidations or failures are published.
#[derive(Clone, Debug, Default)]
pub struct Statistics {
    /// Number of native events received.
    pub received: usize,
    /// Number of native coverage losses observed.
    pub rescans: usize,
    /// Directory registrations added.
    pub added: usize,
    /// Directory registrations removed, including discarded failed coverage.
    pub removed: usize,
}

impl Statistics {
    fn merge(&mut self, other: Self) {
        self.received = self.received.saturating_add(other.received);
        self.rescans = self.rescans.saturating_add(other.rescans);
        self.added = self.added.saturating_add(other.added);
        self.removed = self.removed.saturating_add(other.removed);
    }
}

/// Changes in an additional checkout, identified independently of worktree-relative paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeChanges {
    /// The private Git directory identifying the checkout registration.
    pub git_dir: PathBuf,
    /// The checkout directory these changes refer to, in absolute native spelling.
    pub worktree: PathBuf,
    /// An observed registration transition or private metadata/checkout invalidation.
    pub kind: WorktreeChange,
}

/// What changed in a discovered additional checkout.
///
/// Lifecycle events compare successfully resolved registrations, not Git commands. Initial discovery
/// and re-enabling coverage both report `Added`; an add and remove between observations may be missed.
/// An inaccessible checkout is retained and retried, not reported as removed. Establish a complete
/// baseline after `Added` or `Moved`; only `Changed` carries incremental invalidation hints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeChange {
    /// This registration was discovered, including during the initial inventory.
    Added,
    /// The same private Git directory now resolves to a different checkout directory.
    Moved {
        /// The previously observed checkout directory in absolute native spelling.
        previous_worktree: PathBuf,
    },
    /// The registration disappeared. The checkout is no longer monitored.
    Removed,
    /// Private metadata and checkout invalidations; shared references are reported by the parent.
    /// A common reftable stack also contains main-private refs, so it can invalidate both.
    Changed(Changes),
}

/// Invalidations and failures from a service call. Failures never discard simultaneous changes.
#[derive(Default)]
pub struct Outcome {
    /// Cached information to refresh.
    pub changes: Changes,
    /// Changes in other checkouts when [`Options::all_worktrees`] is enabled.
    ///
    /// Paths within each entry are relative to that entry's `worktree`, not the selected checkout.
    /// Shared configuration can invalidate several checkouts, but is subscribed to only once.
    pub other_worktrees: Vec<WorktreeChanges>,
    /// Discovery or native watcher failures, retained with their error chains.
    pub errors: Vec<Error>,
    /// Native-event and registration counts.
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
    fn replace(&mut self, desired: BTreeMap<PathBuf, bool>, statistics: &mut Statistics) -> Result<bool> {
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

/// A detached repository monitor with one metadata stream and independent checkout streams.
///
/// Monitoring options are fixed for the lifetime of an instance. To change them or select another
/// repository, create a new instance with [`Repository::monitor()`]. Clone [`options()`](Self::options)
/// first if desired, then **drop existing instances before constructing their replacements** to avoid
/// temporary peaks in watcher, thread and memory usage. Dropping discards pending notifications;
/// replacements establish fresh baselines for enabled domains at their first [`service()`](Self::service) call.
/// If replacement construction fails, the dropped monitor is no longer available.
///
/// All-worktree monitoring unions private metadata and dependency watches into the same metadata
/// stream. Checkout streams retain independent ignore rules and recovery. Recursive `All` coverage
/// can still overlap metadata paths or another checkout nested inside a watched root.
///
/// Subscriptions stay within the selected checkouts and their private and shared Git directories, without
/// additional parent subscriptions to detect replacement of an entire root directory. External
/// configuration and attribute files are not watched. Ignore-aware worktree coverage also watches
/// the nearest existing parents of external ignore dependencies. Reopen the repository to read
/// current external configuration.
///
/// Reported event loss invalidates the affected caches at the next [`service()`](Self::service)
/// call and requests inventory repair. Lost native root identities also require re-registration,
/// even when the desired paths are unchanged. Use [`rescan()`](Self::rescan) for manual recovery
/// if cached state appears stale.
/// With metadata enabled, linked-worktree removal by Git is observed through the shared
/// administrative directories. Without it, only dependencies needed by ignore-aware coverage are
/// watched; [`WorktreeCoverage::All`] needs only the recursive worktree root.
pub struct RepositoryMonitor {
    #[cfg(feature = "async-notify")]
    wake: gix_parallel::OwnShared<tokio::sync::Notify>,
    layout: Layout,
    options: Options,
    metadata: Domain,
    worktree: Domain,
    directories: HashTable<PathBuf>,
    nested_repositories: HashSet<PathBuf>,
    index_fingerprint: Option<gix_hash::ObjectId>,
    directory_inventory: Option<inventory::Directories>,
    changed_directories: HashSet<PathBuf>,
    pending_metadata: Pending,
    pending_worktree: Changes,
    pending_statistics: Statistics,
    metadata_dirty: bool,
    inventory_dirty: bool,
    inventory_verify: bool,
    index_dirty: bool,
    maintenance: Option<Instant>,

    backlog: bool,
    started: bool,
    precompose_unicode: bool,
    ignore_case: bool,
    shared_metadata: bool,
    other_worktrees: BTreeMap<PathBuf, RepositoryMonitor>,
    retired_worktrees: Vec<WorktreeChanges>,
    worktree_transition: Option<WorktreeChange>,
    maintenance_failed: bool,
    undiscovered_worktrees: BTreeMap<PathBuf, Instant>,
}

impl Repository {
    /// Observe this repository using `options` for coverage and scheduling, without keeping its object
    /// database or configuration caches alive.
    ///
    /// To change monitoring options, drop the existing monitor before constructing its replacement
    /// to avoid resource usage peaks. See [`RepositoryMonitor`] for the replacement lifecycle.
    /// For raw native events with caller-owned watch registration and recovery, use [`plumbing::Watcher`].
    pub fn monitor(&self, options: Options) -> Result<RepositoryMonitor> {
        if options.all_worktrees && !options.metadata {
            return Err(gix_error::validation("all-worktree monitoring requires metadata monitoring").raise());
        }
        if options.retry_interval.is_zero() {
            return Err(message("monitor retry interval must be positive").raise());
        }
        let layout = Layout::for_options(self, &options)?;
        let filesystem = self
            .filesystem_options()
            .or_raise(|| message("could not read filesystem options"))?;
        Ok(RepositoryMonitor {
            #[cfg(feature = "async-notify")]
            wake: gix_parallel::OwnShared::new(tokio::sync::Notify::new()),
            layout,
            options,
            metadata: Domain::default(),
            worktree: Domain::default(),
            directories: HashTable::new(),
            nested_repositories: HashSet::new(),
            index_fingerprint: None,
            directory_inventory: None,
            changed_directories: HashSet::new(),
            pending_metadata: Pending::default(),
            pending_worktree: Changes::default(),
            pending_statistics: Statistics::default(),
            metadata_dirty: true,
            inventory_dirty: true,
            inventory_verify: false,
            index_dirty: true,
            maintenance: Some(Instant::now()),

            backlog: false,
            started: false,
            precompose_unicode: filesystem.precompose_unicode,
            ignore_case: filesystem.ignore_case,
            shared_metadata: false,
            other_worktrees: BTreeMap::new(),
            retired_worktrees: Vec::new(),
            worktree_transition: None,
            maintenance_failed: false,
            undiscovered_worktrees: BTreeMap::new(),
        })
    }
}

impl Layout {
    #[cfg(test)]
    fn from_repository(repo: &Repository) -> Result<Self> {
        Self::for_options(repo, &Options::default())
    }

    fn for_options(repo: &Repository, options: &Options) -> Result<Self> {
        let precompose = repo
            .filesystem_options()
            .or_raise(|| message("could not read filesystem options"))?
            .precompose_unicode;
        let absolute = |path: &Path| absolute_path(path, repo.current_dir(), precompose);
        let dependencies = options.metadata || options.worktree_coverage == Some(WorktreeCoverage::IgnoreAware);
        let index = if dependencies {
            absolute(&repo.index_path())?
        } else {
            // Unfiltered coverage must also work with an inaccessible or cyclic index path.
            repo.current_dir().join(repo.index_path())
        };
        let mut layout = Self {
            git_dir: absolute(repo.git_dir())?,
            common_dir: absolute(repo.common_dir())?,
            worktree: repo.workdir().map(absolute).transpose()?,
            index,
            dependencies: Vec::new(),
        };
        if !dependencies {
            return Ok(layout);
        }
        for dependency in repo.notification_dependencies(options.metadata)? {
            // Observe a symlink's target as well as replacement of the link itself.
            if let (Some(parent), Some(name)) = (dependency.path.parent(), dependency.path.file_name()) {
                let parent = if parent.as_os_str().is_empty() {
                    repo.current_dir()
                } else {
                    parent
                };
                let path = absolute(parent)?.join(name);
                if layout.contains(&path)
                    || options.worktree_coverage == Some(WorktreeCoverage::IgnoreAware)
                        && layout.worktree.is_some()
                        && dependency.kind == DependencyKind::Ignore
                {
                    layout.dependencies.push(Dependency {
                        path,
                        kind: dependency.kind,
                    });
                }
            }
            let path = absolute(&dependency.path)?;
            if layout.contains(&path)
                || options.worktree_coverage == Some(WorktreeCoverage::IgnoreAware)
                    && layout.worktree.is_some()
                    && dependency.kind == DependencyKind::Ignore
            {
                layout.dependencies.push(Dependency {
                    path,
                    kind: dependency.kind,
                });
            }
        }
        layout.dependencies.sort();
        layout.dependencies.dedup();
        Ok(layout)
    }

    fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.git_dir)
            || path.starts_with(&self.common_dir)
            || self.worktree.as_ref().is_some_and(|root| path.starts_with(root))
    }
}

/// Resolve `path` into an absolute path suitable for comparison with native notification paths.
///
/// This performs filesystem I/O, resolves symbolic links, and normalizes `.` and `..` components,
/// but preserves missing components so dependencies can be monitored before they exist. For relative
/// paths, `current_dir` must already be absolute with symlinks resolved: the base itself is not
/// traversed by `gix_path::realpath_opts()`. Windows drive-relative paths on another drive use that
/// drive's current directory supplied by the operating system instead.
///
/// On Windows, canonicalize the nearest existing ancestor to obtain native, verbatim spelling, then
/// append the unresolved suffix. If `precompose` is `true`, convert a valid-Unicode result to its
/// precomposed form; otherwise retain the spelling produced by resolution.
///
/// # Examples
///
/// These transformations assume the stated filesystem layout:
///
/// - With an absolute, physical `current_dir` of `/repo`, `missing/config` becomes
///   `/repo/missing/config`, even if `missing` does not exist.
/// - On macOS, if `/var` is a symlink to `/private/var`, `/var/tmp/repo` becomes
///   `/private/var/tmp/repo`, matching the physical spelling used by native events.
/// - On Windows, if `C:\repo` is an existing directory with that native spelling,
///   `C:\repo\missing\config` becomes `\\?\C:\repo\missing\config`, even if the suffix is missing.
/// - A valid-Unicode resolved path ending in `"a\u{308}"` (a decomposed `ä`) ends in `"ä"`
///   when `precompose` is `true`; with `false`, its decomposed spelling remains unchanged.
///
/// Errors include an empty input, excessive symlink traversal, or parent components escaping the
/// filesystem root. Windows native-path resolution also propagates errors other than missing-path
/// errors and fails if no ancestor can be canonicalized. Success does not guarantee existence or
/// provide race-free filesystem containment.
fn absolute_path(path: &Path, current_dir: &Path, precompose: bool) -> Result<PathBuf> {
    // Native event paths use the physical spelling (notably /private/var on macOS).
    // realpath also preserves missing suffixes for dependencies that can appear later.
    let path = gix_path::realpath_opts(path, current_dir, gix_path::realpath::MAX_SYMLINKS)
        .or_raise(|| message("could not resolve repository monitor path"))?;
    // Windows notifications use verbatim native roots; preserve missing dependency suffixes.
    #[cfg(windows)]
    let path = path
        .ancestors()
        .find_map(|ancestor| match ancestor.canonicalize() {
            Ok(native) => Some(Ok(native.join(
                path.strip_prefix(ancestor)
                    .expect("ancestors are prefixes of the resolved path"),
            ))),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                None
            }
            Err(error) => Some(Err(error).or_raise(|| message("could not resolve native repository monitor path"))),
        })
        .ok_or_else(|| message("repository monitor path has no existing ancestor").raise())??;
    let path = std::borrow::Cow::Owned(path);
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
    /// The selected repository's last resolved layout, not a guarantee of active native coverage.
    ///
    /// Additional layouts are discovered during service and available through [`other_worktrees()`](Self::other_worktrees).
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The current coverage and scheduling options.
    ///
    /// Clone these before dropping this instance to retain or change selected fields when creating
    /// its replacement with [`Repository::monitor()`].
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Force a full refresh at the next [`service()`](Self::service) call.
    ///
    /// Rebuild native watches, rediscover subscriptions, and invalidate every enabled domain,
    /// including all monitored worktrees, even without reported events. Connect this to an explicit
    /// user refresh for recovery when cached state appears stale.
    pub fn rescan(&mut self) {
        for other in self.other_worktrees.values_mut() {
            other.rescan();
        }
        self.metadata.rebuild = true;
        self.worktree.rebuild = true;
        self.coverage_lost(false, Instant::now());
    }

    fn worktree_enabled(&self) -> bool {
        self.options.worktree_coverage.is_some() && self.layout.worktree.is_some()
    }

    fn ignore_aware(&self) -> bool {
        self.options.worktree_coverage == Some(WorktreeCoverage::IgnoreAware)
    }

    fn active(&self) -> bool {
        self.options.metadata || self.worktree_enabled()
    }

    fn poll(&mut self, now: Instant, out: &mut Outcome) {
        if !self.active() {
            return;
        }
        self.started = true;
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
            out.statistics.received += batch.events.len();
            for event in batch.events {
                self.observe(&event, worktree, now);
                if !worktree && self.all_worktrees_enabled() {
                    self.observe_index(&event, now);
                    for other in self.other_worktrees.values_mut() {
                        other.observe(&event, false, now);
                        other.observe_index(&event, now);
                    }
                }
            }
            if let Some(error) = batch.error {
                self.coverage_lost(worktree, now);
                let domain = if worktree {
                    &mut self.worktree
                } else {
                    &mut self.metadata
                };
                domain.watcher = None;
                out.statistics.removed += domain.registered.len();
                domain.registered.clear();
                out.errors.push(error.and_raise(message!(
                    "filesystem monitoring failed for {}",
                    self.layout.git_dir.display()
                )));
                self.maintenance = Some(now + self.options.retry_interval);
            }
        }
    }

    /// Process bounded native batches, maintain watches when due, and publish invalidations.
    ///
    /// `now` is supplied explicitly mainly so tests can exercise scheduling without sleeping. In
    /// normal use, pass a fresh [`Instant::now()`] on each call. It provides the monotonic time for
    /// this service pass: it determines which maintenance, retry and metadata-publication deadlines
    /// are due, and anchors new debounce, maximum-latency and retry deadlines for events observed
    /// during this pass. It is not a native event timestamp or a limit on the call's running time.
    ///
    /// `opener` must return a freshly opened repository using the caller's intended open options.
    /// It is called at most once per service call. All-worktree mode requires the same private Git
    /// directory; drop this monitor and create a new instance to select another primary repository.
    /// Other repository handles are derived only during maintenance and are never retained.
    /// Worktree-only [`WorktreeCoverage::All`] does not call it. With metadata disabled, only
    /// worktree scopes are published; errors are still returned. Both domains disabled is a no-op.
    pub fn service(&mut self, now: Instant, mut opener: impl FnMut() -> Result<Repository>) -> Outcome {
        let mut out = Outcome::default();
        if !self.active() {
            return out;
        }
        self.poll(now, &mut out);
        for other in self.other_worktrees.values_mut() {
            other.poll(now, &mut out);
            if other.maintenance.is_some_and(|deadline| deadline <= now) {
                self.maintenance = Some(now);
            }
        }
        if self.maintenance.is_some_and(|deadline| deadline <= now) {
            self.maintenance = None;
            let result = if !self.options.metadata && !self.ignore_aware() {
                self.maintain_all(now, &mut out.statistics)
                    .map(|_| self.maintenance_recovered(now))
            } else {
                opener()
                    .or_raise(|| message("could not reopen repository for filesystem monitoring"))
                    .and_then(|repo| {
                        if self.all_worktrees_enabled() && !same_path(
                            &absolute_path(repo.git_dir(), repo.current_dir(), self.precompose_unicode)?,
                            &self.layout.git_dir,
                            self.ignore_case,
                        ) {
                            return Err(gix_error::validation("all-worktree monitoring requires the same primary repository; drop this monitor and create a new instance to switch").raise());
                        }
                        if self.all_worktrees_enabled() {
                            self.maintain_other_worktrees(&repo, now, &mut out.statistics, &mut out.errors)?;
                        }
                        self.maintain(&repo, now, &mut out.statistics, &mut out.errors)
                    })
            };
            if let Err(err) = result {
                out.errors.push(err);
                self.maintenance_failed(now);
                // Coordinator failure can leave child verification overdue; retain bounded retries.
                let retry = now + self.options.retry_interval;
                for other in self.other_worktrees.values_mut() {
                    if other.maintenance.is_some_and(|deadline| deadline <= now) {
                        other.maintenance = Some(retry);
                    }
                }
                for deadline in self.undiscovered_worktrees.values_mut() {
                    if *deadline <= now {
                        *deadline = retry;
                    }
                }
            }
        }
        out.changes = self.take_changes(now);
        out.other_worktrees.append(&mut self.retired_worktrees);
        for other in self.other_worktrees.values_mut() {
            let changes = other.take_changes(now);
            let kind = other
                .worktree_transition
                .take()
                .or_else(|| (!changes.is_empty()).then_some(WorktreeChange::Changed(changes)));
            if let Some(change) = kind.and_then(|kind| other.worktree_change(kind)) {
                out.other_worktrees.push(change);
            }
        }
        if let Some(deadline) = self.undiscovered_worktrees.values().copied().min() {
            self.maintenance = Some(self.maintenance.map_or(deadline, |current| current.min(deadline)));
        }
        self.pending_statistics.merge(std::mem::take(&mut out.statistics));
        if !out.changes.is_empty() || !out.other_worktrees.is_empty() || !out.errors.is_empty() {
            out.statistics = std::mem::take(&mut self.pending_statistics);
        }
        #[cfg(feature = "async-notify")]
        self.install_async_wakers();
        out
    }

    /// Return the maximum time to wait from `now` before the next service call, including the native polling ceiling.
    ///
    /// Explicit `now` mainly supports testing. In normal use, pass a fresh [`Instant::now()`] when
    /// deciding how long to wait, typically after calling [`service()`](Self::service). `now` is
    /// subtracted from pending maintenance and metadata-publication deadlines; overdue deadlines
    /// or queued native backlog yield `Some(Duration::ZERO)`.
    /// Return `None` when monitoring is inactive or no wakeup is scheduled. This does not service the monitor.
    pub fn next_timeout(&self, now: Instant) -> Option<Duration> {
        if !self.active() {
            return None;
        }
        if self.backlog {
            return Some(Duration::ZERO);
        }
        let poll = (self.metadata.watcher.is_some() || self.worktree.watcher.is_some()).then_some(POLL_INTERVAL);
        [self.maintenance, self.pending_metadata.deadline]
            .into_iter()
            .flatten()
            .map(|deadline| deadline.saturating_duration_since(now))
            .chain(poll)
            .chain(
                self.other_worktrees
                    .values()
                    .filter_map(|other| other.next_timeout(now)),
            )
            .min()
    }

    fn maintenance_failed(&mut self, now: Instant) {
        if !self.maintenance_failed {
            self.pending_metadata
                .add(Changes::metadata_all(), now, &self.options, true);
            if self.worktree_enabled() {
                self.pending_worktree.merge(Changes::worktree_all());
            }
            self.maintenance_failed = true;
        }
        self.maintenance = Some(now + self.options.retry_interval);
    }

    fn maintenance_recovered(&mut self, now: Instant) {
        if std::mem::take(&mut self.maintenance_failed) {
            self.pending_metadata
                .add(Changes::metadata_all(), now, &self.options, true);
            if self.worktree_enabled() {
                self.pending_worktree.merge(Changes::worktree_all());
            }
        }
    }

    fn coverage_lost(&mut self, worktree: bool, now: Instant) {
        self.directory_inventory = None;
        if worktree {
            self.pending_worktree.merge(Changes::worktree_all());
            self.inventory_dirty = true;
            self.index_dirty = true;
        } else {
            for other in self.other_worktrees.values_mut() {
                other.coverage_lost(false, now);
            }
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
        _errors: &mut Vec<Error>,
    ) -> Result {
        let layout = Layout::for_options(repo, &self.options)?;
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
                self.directory_inventory = None;
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
        if (self.options.metadata || self.worktree_enabled() && self.ignore_aware())
            && (self.metadata_dirty
                || !self.shared_metadata && self.metadata.watcher.is_none()
                || self.metadata.rebuild)
        {
            let mut watches = if self.shared_metadata {
                inventory::private_metadata_watches(&self.layout)
            } else if self.options.metadata {
                inventory::metadata_watches(&self.layout)?
            } else {
                inventory::dependency_watches(&self.layout)
            };
            if self.metadata_is_shared() && self.worktree_enabled() {
                inventory::watch_parent(&self.layout, &mut watches, &self.layout.index);
            }
            if self.shared_metadata {
                // The parent installs this inventory once in its union metadata domain.
                self.metadata.registered = watches;
                self.metadata_dirty = false;
                self.metadata.rebuild = false;
            } else {
                for other in self.other_worktrees.values() {
                    inventory::merge_watches(&mut watches, other.metadata.registered.clone());
                }
                match self.metadata.replace(watches, statistics) {
                    Ok(changed) => {
                        self.metadata_dirty = false;
                        if changed {
                            self.pending_metadata
                                .add(Changes::metadata_all(), now, &self.options, true);
                            if self.worktree_enabled() {
                                self.pending_worktree.merge(Changes::worktree_all());
                            }
                            for other in self.other_worktrees.values_mut() {
                                other.coverage_lost(false, now);
                            }
                            verify = true;
                        }
                    }
                    Err(err) => {
                        self.coverage_lost(false, now);
                        return Err(err);
                    }
                }
            }
        }
        if self.worktree_enabled() && !self.ignore_aware() {
            verify |= self.maintain_all(now, statistics)?;
        } else if self.worktree_enabled()
            && (self.inventory_dirty
                || self.inventory_verify
                || self.index_dirty
                || !self.changed_directories.is_empty()
                || self.worktree.watcher.is_none()
                || self.worktree.rebuild)
        {
            let index = repo
                .index_or_empty()
                .or_raise(|| message("could not load index for filesystem monitoring"))?;
            let fingerprint = self
                .directory_inventory
                .as_ref()
                .filter(|_| self.index_dirty)
                .map(|inventory| inventory.index_fingerprint(&index, self.ignore_case))
                .transpose()?;
            let changed_index = fingerprint.is_some() && fingerprint != self.index_fingerprint;
            if self.inventory_dirty
                || self.inventory_verify
                || changed_index
                || !self.changed_directories.is_empty()
                || self.worktree.watcher.is_none()
                || self.worktree.rebuild
            {
                let native_recursive = plumbing::Watcher::supports_native_recursive_watches();
                let mut directories = match self.directory_inventory.take() {
                    // Shallow registrations cannot cover children created before installation.
                    Some(_) if self.inventory_verify && !native_recursive => {
                        inventory::worktree_directories(repo, &index)?
                    }
                    Some(mut directories) => {
                        if self.inventory_dirty || changed_index {
                            self.changed_directories.extend(directories.recheck(repo, &index)?);
                        }
                        if self.changed_directories.is_empty() && !self.inventory_verify {
                            directories
                        } else {
                            inventory::worktree_directories_reusing(
                                repo,
                                &index,
                                Some(&directories),
                                &self.changed_directories,
                            )?
                        }
                    }
                    _ => inventory::worktree_directories(repo, &index)?,
                };
                let mut watches = directories.watches_for_backend(native_recursive);
                if !self.metadata_is_shared() {
                    inventory::watch_parent(&self.layout, &mut watches, &self.layout.index);
                }
                if self.replace_worktree_watches(watches, now, statistics)? {
                    self.pending_worktree.merge(Changes::worktree_all());
                    verify = true;
                }
                self.directories = inventory::directory_lookup(directories.paths.keys().cloned());
                self.nested_repositories = std::mem::take(&mut directories.repositories);
                self.index_fingerprint = Some(directories.index_fingerprint(&index, self.ignore_case)?);
                self.directory_inventory = Some(directories);
            }
            self.index_dirty = false;
            self.inventory_dirty = false;
            self.inventory_verify = false;
            self.changed_directories.clear();
        }
        self.maintenance_recovered(now);
        if verify {
            // Installation can race discovery. Verify on the next call, never loop under churn.
            self.metadata_dirty = true;
            self.inventory_verify = true;
            self.maintenance = Some(now);
        }
        Ok(())
    }

    fn replace_worktree_watches(
        &mut self,
        watches: BTreeMap<PathBuf, bool>,
        now: Instant,
        statistics: &mut Statistics,
    ) -> Result<bool> {
        self.worktree.replace(watches, statistics).inspect_err(|_| {
            // Registration failure loses native coverage even during an existing maintenance failure.
            self.coverage_lost(true, now);
        })
    }

    fn maintain_all(&mut self, now: Instant, statistics: &mut Statistics) -> Result<bool> {
        let Some(root) = self.layout.worktree.clone() else {
            return Ok(false);
        };
        let watch = inventory::recursive_root_watch(&root)?;
        let mut watches = BTreeMap::from([(watch.path.clone(), watch.recursive)]);
        if self.options.metadata && !self.metadata_is_shared() {
            inventory::watch_parent(&self.layout, &mut watches, &self.layout.index);
        }
        let changed = self.replace_worktree_watches(watches, now, statistics)?;
        self.directories = inventory::directory_lookup([root]);
        self.nested_repositories.clear();
        self.index_fingerprint = None;
        self.directory_inventory = None;
        self.changed_directories.clear();
        self.inventory_dirty = false;
        self.inventory_verify = false;
        self.index_dirty = false;
        if changed {
            self.pending_worktree.merge(Changes::worktree_all());
            // Verify registration without opening a repository or enumerating its inventory.
            self.maintenance = Some(now);
        }
        Ok(changed)
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
                let is_index = same_path(path, &self.layout.index, self.ignore_case);
                let is_index_lock = same_path(path, &index_lock(), self.ignore_case);
                if !self.metadata_is_shared()
                    && (self.options.metadata || self.ignore_aware())
                    && (is_index || is_index_lock && event.kind == EventKind::Rename)
                {
                    changes.index = true;
                    self.index_dirty = true;
                    self.maintenance = Some(now);
                    continue;
                }
                let Some(root) = self.layout.worktree.as_deref() else {
                    continue;
                };
                if strip_prefix(path, root, self.ignore_case).is_none()
                    || strip_prefix(path, &self.layout.git_dir, self.ignore_case).is_some()
                    || strip_prefix(root, &self.layout.common_dir, self.ignore_case).is_none()
                        && strip_prefix(path, &self.layout.common_dir, self.ignore_case).is_some()
                    || strip_prefix(path, &root.join(".git"), self.ignore_case).is_some()
                    || !self.ignore_aware()
                        && strip_prefix(path, root, self.ignore_case).is_some_and(|relative| {
                            relative.components().any(|component| {
                                same_path(Path::new(component.as_os_str()), Path::new(".git"), self.ignore_case)
                            })
                        })
                {
                    continue;
                }
                let known_directory = self
                    .directories
                    .find(inventory::path_hash(path), |directory| {
                        same_path(path, directory, self.ignore_case)
                    })
                    .is_some();
                let registered_directory = known_directory
                    && self
                        .worktree
                        .registered
                        .keys()
                        .any(|directory| same_path(path, directory, self.ignore_case));
                let is_directory = event.path_kind == PathKind::Directory
                    || known_directory
                    || std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
                if is_directory
                    && (self.ignore_aware() || registered_directory)
                    && matches!(
                        event.kind,
                        EventKind::Create | EventKind::Remove | EventKind::Rename | EventKind::Modify
                    )
                {
                    if !self.ignore_aware() {
                        self.inventory_dirty = true;
                    } else if event.kind != EventKind::Modify || self.worktree.registered.get(path) != Some(&true) {
                        if self.changed_directories.len() >= MAX_SCOPES {
                            self.inventory_dirty = true;
                            self.directory_inventory = None;
                            self.changed_directories.clear();
                        } else {
                            self.changed_directories.insert(path.to_owned());
                        }
                    }
                    if self.inventory_dirty || !self.changed_directories.is_empty() {
                        self.maintenance = Some(now);
                    }
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
                let scope = if !self.ignore_aware() {
                    if self.options.metadata {
                        match name {
                            Some(b".gitignore") => changes.ignores = true,
                            Some(b".gitattributes" | b".gitmodules") => changes.attributes = true,
                            _ => {}
                        }
                    }
                    path
                } else if let Some(repository) = self
                    .nested_repositories
                    .iter()
                    .find(|repository| strip_prefix(path, repository, self.ignore_case).is_some())
                {
                    if name == Some(b".git".as_slice()) {
                        self.inventory_dirty = true;
                        self.changed_directories.insert(repository.clone());
                        self.maintenance = Some(now);
                    }
                    repository.as_path()
                } else {
                    match name {
                        Some(b".git") => {
                            self.inventory_dirty = true;
                            self.changed_directories
                                .insert(path.parent().unwrap_or(root).to_owned());
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
                            changes.worktree = Some(Scope::All);
                            continue;
                        }
                        _ => path,
                    }
                };
                let (kind, path_kind) = if same_path(scope, path, self.ignore_case) {
                    (event.kind, event.path_kind)
                } else {
                    (EventKind::Any, PathKind::Directory)
                };
                match strip_prefix(scope, root, self.ignore_case) {
                    Some(relative)
                        if !relative.as_os_str().is_empty()
                            && relative
                                .components()
                                .all(|c| matches!(c, std::path::Component::Normal(_))) =>
                    {
                        match gix_path::into_bstr(relative) {
                            Ok(relative) => changes.insert_worktree_path(PathChange {
                                path: gix_path::to_unix_separators_on_windows(relative).into_owned(),
                                kind,
                                path_kind,
                            }),
                            Err(_) => changes.worktree = Some(Scope::All),
                        }
                    }
                    _ => changes.worktree = Some(Scope::All),
                }
            } else {
                if path
                    .file_name()
                    .is_some_and(|name| name.as_encoded_bytes().ends_with(b".lock"))
                    && !matches!(event.kind, EventKind::Rename)
                    && !self
                        .layout
                        .dependencies
                        .iter()
                        .any(|dependency| same_path(path, &dependency.path, self.ignore_case))
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
                let (classified, refresh) = if self.shared_metadata {
                    classify_metadata_scoped(path, &self.layout, self.ignore_case, MetadataScope::Other)
                } else if self.all_worktrees_enabled() {
                    classify_metadata_scoped(path, &self.layout, self.ignore_case, MetadataScope::Primary)
                } else if self.options.metadata {
                    classify_metadata(path, &self.layout, self.ignore_case)
                } else {
                    classify_dependencies(path, &self.layout, self.ignore_case)
                };
                if refresh || self.all_worktrees_enabled() && classified.layout {
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
                changes.worktree = Some(Scope::All);
                changes.index = true;
            }
            self.pending_metadata.add(changes, now, &self.options, false);
        }
    }
}
