//! Bounded filesystem notifications for long-running applications.
//!
//! Notifications identify paths to reconcile with the filesystem, rather than operations to replay.
//! They may be duplicated, coalesced, or refer to paths which no longer exist. A [`Batch::loss`]
//! requires a fresh scan of every watched path. Install watches **before** taking that baseline,
//! and keep processing notifications received while scanning.
//!
//! This crate currently uses platform backends from `notify`. These cannot certify that every
//! earlier filesystem change has been delivered: [`Watcher::synchronize()`] explicitly reports
//! [`SynchronizeError::Unsupported`]. Applications should also periodically verify their state.
//! No Git knowledge, ignore matching, or repository dependencies are required.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{path::PathBuf, time::Instant};

use gix_error::{ErrorExt, message};
use gix_features::threading::{Mutable, OwnShared, lock};

mod backend;
mod queue;

/// An error with context about the filesystem operation that failed.
pub type Error = gix_error::Exn<gix_error::Message>;

/// Limits on notifications retained while the application is busy.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Maximum number of queued events. Must be nonzero.
    pub max_events: usize,
    /// Maximum estimated bytes owned by queued events and their paths. Must be nonzero.
    ///
    /// This excludes the native backend's buffers, an event currently being received, and the
    /// single retained error diagnostic. Crossing either limit drops pending events and latches loss.
    pub max_bytes: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_events: 4096,
            max_bytes: 4 * 1024 * 1024,
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
pub struct Batch {
    /// Paths to reconcile. No filesystem reads are performed by `drain()`.
    pub events: Vec<Event>,
    /// If present, perform a complete baseline scan. This cannot be hidden by queue overflow.
    pub loss: Option<Loss>,
    /// At most one diagnostic, retaining the most recent error and its cause.
    pub errors: Vec<Error>,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SynchronizeError {
    /// This backend provides no delivery barrier. Scanning is required for a complete answer.
    Unsupported,
}

impl std::fmt::Display for SynchronizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("the filesystem backend does not support synchronization"),
        }
    }
}

impl std::error::Error for SynchronizeError {}

/// A bounded stream of filesystem invalidations, with replaceable desired coverage.
pub struct Watcher {
    state: OwnShared<Mutable<queue::State>>,
    desired: Vec<Watch>,
    backend: Option<backend::Backend>,
}

impl Watcher {
    /// Create an empty watcher. Native resources are acquired by [`replace()`](Self::replace).
    pub fn new(options: Options) -> Result<Self, Error> {
        if options.max_events == 0 || options.max_bytes == 0 {
            return Err(message("filesystem notification queue limits must be nonzero").raise());
        }
        Ok(Self {
            state: OwnShared::new(Mutable::new(queue::State::new(options))),
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
    pub fn replace(&mut self, watches: impl IntoIterator<Item = Watch>) -> Result<(), Error> {
        let mut paths = std::collections::BTreeMap::new();
        for watch in watches {
            if !watch.path.is_absolute() {
                return Err(message!("watched path must be absolute: {}", watch.path.display()).raise());
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
                None => backend::Backend::new(&self.desired, queue::Sender::new(self.state.clone(), registration)).map(
                    |backend| {
                        self.backend = Some(backend);
                    },
                ),
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
    pub fn drain(&mut self, budget: Budget) -> Batch {
        lock(&self.state).drain(budget)
    }

    /// Request a delivery barrier before `deadline`.
    ///
    /// The compatibility backends cannot make this guarantee and always return
    /// [`SynchronizeError::Unsupported`], including when no notifications are pending.
    pub fn synchronize(&mut self, _deadline: Instant) -> Result<Fence, SynchronizeError> {
        Err(SynchronizeError::Unsupported)
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
