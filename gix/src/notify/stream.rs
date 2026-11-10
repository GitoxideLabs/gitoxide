use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use gix_error::{ErrorExt, validation};

use super::{Batch, Budget, Fence, RepositoryMonitor, SynchronizeError, Watch, WorktreeCoverage, inventory};
use crate::Result;

/// A detached, unfiltered native stream covering a worktree and optional auxiliary paths.
///
/// Create it with [`RepositoryMonitor::into_worktree_stream()`]. It owns one native watcher and
/// the canonical worktree path, not a repository or its caches. Native queue limits and root
/// identity checks use the defaults of the underlying watcher.
///
/// # Registration and baselines
///
/// Call [`register()`](Self::register) **before** scanning to establish a baseline. Construction
/// does not register any paths, and draining or synchronizing never implicitly registers them.
/// The first registration reports [`Loss::WatchSetChanged`](super::Loss::WatchSetChanged); retain
/// and process notifications received during the baseline scan rather than discarding them.
///
/// Every [`Batch::loss`] requires reconciliation of the entire watched set. Inspect [`Batch::error`]
/// too, and retry registration before establishing a new baseline if coverage failed. Repeating
/// registration keeps the same watcher and its generation history, including across recovery;
/// changing coverage or recovering from loss can advance the generation and invalidate old fences.
///
/// # Raw events and scheduling
///
/// Events include ignored files, nested repositories, administrative paths (including `.git`),
/// and auxiliary watches. There is no Git classification or ignore filtering. Paths use native
/// absolute spelling, not repository-relative paths or Git's Unicode normalization. Notifications
/// are hints to reconcile with the filesystem, not operations to replay; process both rename paths
/// and account for descendants when directories change.
///
/// This stream does not run [`RepositoryMonitor::service()`], reopen repositories, debounce events,
/// schedule retries or generate periodic safety invalidations. The caller owns scheduling,
/// recovery and any periodic verification needed for silent native-notification blind spots.
/// An empty batch alone does not certify that delivery has caught up.
pub struct WorktreeStream {
    watcher: gix_notify::Watcher,
    worktree: PathBuf,
}

impl RepositoryMonitor {
    /// Consume an unstarted monitor to obtain unfiltered, recursive worktree notifications.
    ///
    /// Requires `metadata = false`, `worktree = true` and
    /// `worktree_coverage = WorktreeCoverage::All`, and a non-bare repository. Invalid selections
    /// and monitors already serviced or with initialized native domains return a
    /// [validation error](crate::Error::is_validation). Consuming the monitor prevents two APIs
    /// from draining the same notifications; rejecting a started monitor prevents queued batches
    /// from being silently discarded during conversion.
    ///
    /// This resolves the native canonical root but installs no subscriptions. Call
    /// [`WorktreeStream::register()`] before establishing a baseline. Monitor debounce and retry
    /// settings do not apply to the raw stream.
    pub fn into_worktree_stream(self) -> Result<WorktreeStream> {
        if self.options.metadata || !self.options.worktree || self.options.worktree_coverage != WorktreeCoverage::All {
            return Err(validation(
                "a worktree stream requires metadata=false, worktree=true and worktree_coverage=All",
            )
            .raise());
        }
        let root = self
            .layout
            .worktree
            .as_deref()
            .ok_or_else(|| validation("a bare repository has no worktree stream").raise())?;
        if self.started || self.metadata.watcher.is_some() || self.worktree.watcher.is_some() {
            return Err(
                validation("cannot convert an already started repository monitor to a worktree stream").raise(),
            );
        }
        let watch = inventory::recursive_root_watch(root)?;
        Ok(WorktreeStream {
            watcher: gix_notify::Watcher::new(gix_notify::Options::default())?,
            worktree: watch.path,
        })
    }
}

impl WorktreeStream {
    /// Install the recursive worktree root plus the complete set of `additional_watches`.
    ///
    /// Auxiliary paths must be absolute. They share the worktree's single native stream and can
    /// cover, for example, a caller-owned writable cookie directory outside the worktree. Each call
    /// replaces the previous auxiliary set; pass it again on retries. Duplicate and redundant
    /// watches are combined by the native watcher, retaining recursive coverage for the root.
    ///
    /// Register before taking a baseline. Repeating a healthy set is a no-op; repeating a failed
    /// set retries recovery without recreating the watcher or resetting its generation history.
    /// Changed or recovered coverage requires a new baseline, not incremental replay. Registration
    /// errors retain their causes; do not treat a failed registration as proof of coverage.
    pub fn register(&mut self, additional_watches: &[Watch]) -> Result<()> {
        let root = inventory::recursive_root_watch(&self.worktree)?;
        self.watcher
            .replace(std::iter::once(root).chain(additional_watches.iter().cloned()))
    }

    /// Consume a bounded batch of ready native notifications, without waiting or registering.
    ///
    /// Loss and errors are consumed even with a zero event budget. Do not discard them. Continue
    /// draining while [`Batch::more`] is set, allowing other event-loop work between batches.
    /// An empty batch does not synchronize native delivery; use [`synchronize_at()`](Self::synchronize_at)
    /// for a delivery fence when supported.
    pub fn drain(&mut self, budget: Budget) -> Batch {
        self.watcher.drain(budget)
    }

    /// Fence preceding covered writes using a temporary cookie in `directory` before `deadline`.
    ///
    /// The directory must already exist, be writable and caller-owned, and be covered by this
    /// **same stream**, either beneath the worktree or through [`register()`](Self::register).
    /// A separate watcher for cookies cannot certify delivery on this stream. This creates and
    /// deletes a uniquely named marker; it never adds coverage or chooses a directory for you.
    ///
    /// Drain and process batches through [`Fence::sequence`] in the same [`Fence::generation`].
    /// Intervening loss invalidates incremental state and requires a new baseline. Marker cleanup
    /// can itself produce notifications. Unsupported platforms/filesystems, timeout and coverage
    /// errors do not certify freshness. macOS supports local filesystems; Linux supports known
    /// local filesystems (including ext4, XFS, Btrfs, tmpfs and overlayfs). Other backends currently
    /// return [`SynchronizeError::Unsupported`]. A deadline does not cancel an active native
    /// operation, and dropping the stream waits for its control worker to finish.
    pub fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        self.watcher.synchronize_at(directory, deadline)
    }

    /// Wake the caller's event loop when notifications arrive, without registering any paths.
    ///
    /// The callback may run spuriously or on a native thread and must return promptly. It runs
    /// outside the queue lock. A panic disables the callback and becomes a queued diagnostic.
    pub fn set_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        self.watcher.set_waker(wake);
    }
}
