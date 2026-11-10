//! Bounded filesystem notifications for long-running applications.
//!
//! Notifications identify paths to reconcile with the filesystem, rather than operations to replay.
//! They may be duplicated, coalesced, or refer to paths which no longer exist. No Git knowledge,
//! ignore matching, or repository dependencies are required.
//!
//! # Typical workflow
//!
//! 1. Create a [`Watcher`] with [`Watcher::new()`], using [`Options`] to bound the number and
//!    estimated memory size of events retained while the application is busy.
//! 2. Describe the desired coverage with [`Watch`] values: absolute file or directory paths and
//!    whether to watch descendants recursively. Install them with [`Watcher::replace()`] **before**
//!    scanning those paths to establish a baseline. The initial [`Loss::WatchSetChanged`] signals
//!    that this baseline is needed. Keep processing notifications received while scanning.
//! 3. Connect [`Watcher::set_waker()`] to the application's event loop, or poll periodically.
//!    The callback may run on a backend thread and must return promptly. Call [`Watcher::drain()`]
//!    with a [`Budget`] to obtain a [`Batch`]. Unlike the queue limits in `Options`, this budget
//!    limits the events and estimated bytes returned by one drain. Draining consumes the returned
//!    events, loss notification, and diagnostics; do not discard the batch.
//! 4. Inspect [`Batch::loss`] and [`Batch::error`]. A [`Loss`] means incremental observations are
//!    insufficient: scan every watched path again, retrying registration first if coverage failed.
//!    Changing the desired watches with `replace()` also requires a new baseline. Otherwise,
//!    reconcile the paths in each [`Event`] with the current filesystem. Its [`EventKind`] hints at
//!    what changed, and its [`PathKind`] hints at whether a path names a file or directory; neither
//!    is authoritative. Invalidate every supplied rename path, and consider descendants when a
//!    directory changes. Keep draining while [`Batch::more`] indicates queued work, allowing other
//!    event-loop work between batches as needed.
//!
//! # Knowing when delivery has caught up
//!
//! An empty batch does not prove that nothing changed: native notifications may still be in transit.
//! When a catch-up boundary is needed, call [`Watcher::synchronize_at()`] with a deadline and a safe,
//! caller-owned, writable administrative or temporary directory covered by the watcher. It creates
//! and deletes a temporary marker there. On supported platforms and filesystems, the returned
//! [`Fence`] identifies the delivery boundary for preceding covered writes. Drain and process batches
//! through [`Fence::sequence`] in the same [`Fence::generation`]; intervening loss requires a full scan.
//! A [`SynchronizeError`] means delivery was not certified, for example because synchronization is
//! unsupported, timed out, or lost coverage. Do not treat it as evidence that the cache is current.
//! Applications can offer an explicit full refresh for recovery when users notice stale state that
//! native notifications did not report.
//!
//! # Platform behavior
//!
//! macOS uses an owned FSEvents backend, Linux uses an owned inotify backend, and Windows uses an
//! owned `ReadDirectoryChangesW` backend; other platforms use `notify`. macOS and Linux support
//! marker-based delivery fences on the filesystems described by [`Watcher::synchronize_at()`];
//! other backends return [`SynchronizeError::Unsupported`].
//!
//! macOS checks the identities of its reduced native roots periodically while the control worker
//! is available, including while idle, and around delivery fences. [`Options::root_check_interval`]
//! configures or disables the periodic checks, defaulting to once per second. These checks do not
//! scan directory contents or retain ancestor-directory descriptors. A missing or replaced root stops
//! its stream and reports [`Loss::BackendError`]; retry [`Watcher::replace()`] even if the desired paths
//! are unchanged.
//! A transient rename and restoration between checks can go undetected. Root checks do not replace
//! application-level refreshes for silent notification blind spots.
//!
//! Linux installs one watch per directory without following descendant symlinks. Directory moves,
//! removals, and native queue overflow can require re-registering the desired watches before a new
//! baseline scan. Like inotify itself, notifications and fences cannot cover memory-mapped writes
//! or changes made through unobserved hard links or directly to an overlay's underlying layers.
//!
//! Windows retains a 64 KiB read buffer per native directory registration, in addition to the
//! bounded application queue. File watches share their parent's registration and filter sibling
//! events. Root identity checks detect moved or replaced directories before publishing their
//! notifications and periodically while idle, using [`Options::root_check_interval`]; losing a root
//! requires registration to be retried.
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use gix_error::{ClassificationMarker, Error, ErrorExt, Result, message, validation};
use gix_parallel::{Mutable, OwnShared, lock};

mod backend;
mod queue;

/// Notification queue limits and native coverage checks.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Maximum number of queued events. Must be nonzero.
    pub max_events: usize,
    /// Maximum estimated bytes owned by queued events and their paths. Must be nonzero.
    ///
    /// This excludes the native backend's buffers, an event currently being received, and the
    /// single retained error diagnostic. Crossing either limit drops pending events and latches loss.
    pub max_bytes: usize,
    /// Interval between periodic native-root identity checks on macOS and Windows.
    ///
    /// Defaults to one second. `Some` must contain a nonzero duration; `None` disables periodic
    /// checks. Other backends ignore this setting. Checks during registration, event delivery,
    /// or synchronization remain active where supported.
    ///
    /// Disabling periodic checks can leave moved or replaced roots undetected while idle.
    /// Applications must then verify coverage themselves or, where supported, synchronize before
    /// relying on it. These checks never replace application-level refreshes of directory contents.
    pub root_check_interval: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_events: 4096,
            max_bytes: 4 * 1024 * 1024,
            root_check_interval: Some(Duration::from_secs(1)),
        }
    }
}

/// A path to observe. Files are supported; missing paths must be observed through an existing parent.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Watch {
    /// Absolute path to observe. Events use the platform's absolute path spelling.
    pub path: PathBuf,
    /// Whether descendants of a watched directory are observed recursively.
    pub recursive: bool,
}

/// A hint about a path change; the current filesystem remains authoritative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    /// A change whose nature is unknown.
    Any,
    /// An object was created.
    Create,
    /// Data or metadata may have changed.
    Modify,
    /// An object was removed.
    Remove,
    /// An object was renamed. All supplied paths must be invalidated, even without a matching pair.
    Rename,
}

/// A hint about the affected object. Never use it to decide whether a removed path still exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathKind {
    /// The backend did not identify the object reliably; it may be a directory.
    Any,
    /// A file.
    File,
    /// A directory, whose descendants may need reconciliation as well.
    Directory,
}

/// One related group of paths to reconcile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Absolute paths affected by this event, including both rename endpoints when available.
    pub paths: Vec<PathBuf>,
    /// What may have happened at the paths.
    pub kind: EventKind,
    /// The affected object type, when known.
    pub path_kind: PathKind,
}

/// Why incremental observations are insufficient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Loss {
    /// The desired watch set changed, or coverage is being re-established.
    WatchSetChanged,
    /// The native backend requires a fresh recursive scan.
    Rescan,
    /// The bounded event queue overflowed.
    Overflow,
    /// The backend reported an error; retry registration to recover coverage.
    BackendError,
    /// The backend stopped delivering events; retry registration to recover coverage.
    BackendStopped,
}

/// Limits for one nonblocking [`Watcher::drain()`] call.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    /// Maximum events to return. Zero permits polling only loss and diagnostics.
    pub max_events: usize,
    /// Stop before exceeding this many estimated bytes, after returning at least one event.
    ///
    /// A single larger event is returned to guarantee progress. Its size is bounded by [`Options`].
    pub max_bytes: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_events: 256,
            max_bytes: 256 * 1024,
        }
    }
}

/// Notifications currently ready for the application.
#[derive(Debug)]
#[must_use = "process the batch to avoid silently losing notifications, coverage loss, or diagnostics"]
pub struct Batch {
    /// Paths to reconcile. No filesystem reads are performed by `drain()`.
    pub events: Vec<Event>,
    /// If present, perform a complete baseline scan. This cannot be hidden by queue overflow.
    pub loss: Option<Loss>,
    /// The most recent diagnostic and its cause, if any.
    pub error: Option<Error>,
    /// Changes whenever coverage is lost, including watch replacement and queue overflow.
    pub generation: u64,
    /// The publication sequence through which events have been drained or explicitly invalidated.
    pub sequence: u64,
    /// Whether further queued events remain after this drain.
    pub more: bool,
}

/// A native delivery boundary. Consume notifications through `sequence` in this generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fence {
    /// Coverage generation certified by the fence.
    pub generation: u64,
    /// Last publication preceding the fence.
    pub sequence: u64,
}

/// Why notification delivery could not be synchronized.
///
/// Classifications distinguish unsupported capabilities, retryable timeouts or busy workers,
/// and coverage conflicts requiring reconciliation or re-registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SynchronizeError {
    /// This backend provides no delivery barrier. Scanning is required for a complete answer.
    Unsupported,
    /// The deadline elapsed before native delivery was certified. Retrying requires a fresh deadline.
    TimedOut,
    /// Coverage changed or failed while synchronizing.
    CoverageLost,
    /// The native control worker stopped unexpectedly.
    Stopped,
    /// An earlier native request is still pending after its caller timed out.
    Busy,
}

impl std::fmt::Display for SynchronizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("the filesystem backend does not support synchronization"),
            Self::TimedOut => f.write_str("filesystem synchronization timed out"),
            Self::CoverageLost => f.write_str("filesystem coverage changed while synchronizing"),
            Self::Stopped => f.write_str("the filesystem backend stopped"),
            Self::Busy => f.write_str("a filesystem synchronization request is still pending"),
        }
    }
}

impl std::error::Error for SynchronizeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Unsupported => const { &ClassificationMarker::UNSUPPORTED },
            Self::TimedOut | Self::Busy => const { &ClassificationMarker::RETRYABLE },
            Self::CoverageLost | Self::Stopped => const { &ClassificationMarker::CONFLICT },
        })
    }
}

/// A bounded stream of filesystem invalidations, with replaceable desired coverage.
pub struct Watcher {
    state: OwnShared<Mutable<queue::State>>,
    options: Options,
    desired: Vec<Watch>,
    backend: Option<backend::Backend>,
}

impl Watcher {
    /// Create an empty watcher. Native resources are acquired by [`replace()`](Self::replace).
    /// Zero queue limits or a zero root-check interval return a [validation error](Error::is_validation).
    pub fn new(options: Options) -> Result<Self> {
        if options.max_events == 0 || options.max_bytes == 0 {
            return Err(validation("filesystem notification queue limits must be nonzero").raise());
        }
        if options.root_check_interval.is_some_and(|interval| interval.is_zero()) {
            return Err(
                validation("filesystem root check interval must be nonzero, or None to disable checks").raise(),
            );
        }
        Ok(Self {
            state: OwnShared::new(Mutable::new(queue::State::new(options))),
            options,
            desired: Vec::new(),
            backend: None,
        })
    }

    /// Set a lightweight callback to wake an application's event loop when notifications arrive.
    ///
    /// It may run on a native backend thread and must return promptly. It is called without the
    /// queue lock, and may be called spuriously. Panics disable it and become a queued diagnostic.
    pub fn set_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        lock(&self.state).waker = Some(OwnShared::new(wake));
        queue::wake(&self.state);
    }

    /// Replace the desired watch set, reporting any registration failure with its cause.
    ///
    /// Repeating a healthy set is a no-op; repeating a failed set retries it. Duplicates are
    /// combined, preferring recursive coverage; descendants of recursive roots are redundant.
    /// A change invalidates the previous generation
    /// before any native registration changes. Success requires a new baseline scan; it does not
    /// promise gap-free replacement. Failed registration leaves no partial set installed.
    /// Relative paths return a [validation error](Error::is_validation) without changing coverage.
    pub fn replace(&mut self, watches: impl IntoIterator<Item = Watch>) -> Result {
        let mut paths = std::collections::BTreeMap::new();
        for watch in watches {
            if !watch.path.is_absolute() {
                return Err(message!("watched path must be absolute: {}", watch.path.display()).validation_error());
            }
            paths
                .entry(watch.path)
                .and_modify(|recursive| *recursive |= watch.recursive)
                .or_insert(watch.recursive);
        }
        let desired: Vec<_> = paths
            .iter()
            .filter(|(path, _)| !path.ancestors().skip(1).any(|parent| paths.get(parent) == Some(&true)))
            .map(|(path, recursive)| Watch {
                path: path.clone(),
                recursive: *recursive,
            })
            .collect();
        let reuse_backend = {
            let state = lock(&self.state);
            if desired == self.desired && !state.needs_restart {
                return Ok(());
            }
            self.backend.is_some() && !state.needs_restart && !desired.is_empty()
        };
        let registration = {
            let mut state = lock(&self.state);
            if !reuse_backend {
                state.registration = state.registration.wrapping_add(1);
            }
            state.invalidate(Loss::WatchSetChanged);
            state.needs_restart = false;
            state.registration
        };
        self.desired = desired;
        if !reuse_backend {
            self.backend = None;
        }
        queue::wake(&self.state);
        if !self.desired.is_empty() {
            let result = match self.backend.as_mut() {
                Some(backend) => backend.replace(&self.desired),
                None => backend::Backend::new(
                    &self.desired,
                    queue::Sender::new(self.state.clone(), registration),
                    self.options,
                )
                .map(|backend| {
                    self.backend = Some(backend);
                }),
            };
            if let Err(error) = result {
                {
                    let mut state = lock(&self.state);
                    state.needs_restart = true;
                    state.registration = state.registration.wrapping_add(1);
                }
                self.backend = None;
                return Err(error);
            }
        }
        Ok(())
    }

    /// Drain ready notifications without waiting, respecting the supplied work budget.
    ///
    /// Returned events, loss notifications, and diagnostics are consumed, not merely inspected.
    /// Loss and diagnostics are taken even when the event budget is zero. Discarding the batch
    /// discards this information; it does not satisfy a required baseline scan or restore coverage.
    /// Check [`Batch::more`] for events left queued by the budget.
    ///
    /// This method does not synchronize native delivery. An empty batch does not prove that the
    /// filesystem is unchanged; notifications may still be in transit. Use
    /// [`synchronize_at()`](Self::synchronize_at) when a delivery fence is required and supported.
    pub fn drain(&mut self, budget: Budget) -> Batch {
        lock(&self.state).drain(budget)
    }

    /// Request a delivery barrier without writing a filesystem marker.
    ///
    /// Current backends cannot certify preceding filesystem writes this way, and return
    /// [`SynchronizeError::Unsupported`]. Use [`synchronize_at()`](Self::synchronize_at) with a safe
    /// administrative or temporary directory when a native marker barrier is available.
    pub fn synchronize(&mut self, _deadline: Instant) -> std::result::Result<Fence, SynchronizeError> {
        Err(SynchronizeError::Unsupported)
    }

    /// Fence preceding filesystem writes using a temporary marker in `directory` before `deadline`.
    ///
    /// The caller supplies an existing writable, caller-owned administrative or temporary directory
    /// covered by this same watcher's single native stream. This creates and deletes a uniquely named file there; it
    /// never adds watches or chooses a directory inside the application's data. macOS supports
    /// this on local filesystems. Linux supports known local filesystems, including ext4, XFS,
    /// Btrfs, tmpfs, and overlayfs, and rejects network, FUSE, and unknown filesystems. Linux also
    /// verifies that mounts intersecting coverage have not changed. Other backends return
    /// [`SynchronizeError::Unsupported`].
    ///
    /// A successful fence follows publication of the complete native batch containing the marker.
    /// Drain through its sequence in the same generation; any intervening loss requires a full
    /// scan. Marker cleanup can itself produce notifications. A deadline does not cancel an active
    /// native operation, and dropping the watcher waits for its control worker to finish.
    pub fn synchronize_at(
        &mut self,
        directory: &std::path::Path,
        deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        let generation = lock(&self.state).fence()?.generation;
        match self.backend.as_mut() {
            Some(backend) => backend.synchronize_at(directory, deadline, generation),
            None => Err(SynchronizeError::Unsupported),
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Ignore late callbacks from backends whose shutdown is asynchronous.
        {
            let mut state = lock(&self.state);
            state.registration = state.registration.wrapping_add(1);
        }
        self.backend = None;
    }
}
