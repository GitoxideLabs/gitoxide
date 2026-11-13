use std::{future::Future, time::Instant};

use super::{Outcome, RepositoryMonitor};
use crate::{
    Repository, Result,
    error::{ResultExt, message},
};

struct WorkerOwned<T: Send + 'static> {
    value: Option<T>,
    runtime: tokio::runtime::Handle,
}

impl<T: Send + 'static> Drop for WorkerOwned<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            // A completed but unclaimed reply can otherwise join native watchers on the executor.
            self.runtime.spawn_blocking(move || drop(value));
        }
    }
}

impl RepositoryMonitor {
    /// Service this monitor on a Tokio blocking worker and return it with the complete outcome.
    ///
    /// Requires `async-notify`. Registration and inventory filesystem work stay off the executor.
    /// Ownership moves into the started operation: cancelling it shuts this stream down after
    /// active native work finishes, rather than returning a monitor with an unobserved outcome.
    /// To keep monitoring, retain both returned values and process the entire outcome.
    /// The `opener` runs on the worker and need only produce repositories there; no repository
    /// handle is retained between calls. Blocking [`Self::service()`] remains available.
    pub fn service_async(
        mut self,
        now: Instant,
        opener: impl FnMut() -> Result<Repository> + Send + 'static,
    ) -> impl Future<Output = Result<(Self, Outcome)>> {
        let runtime = tokio::runtime::Handle::current();
        let task = tokio::task::spawn_blocking(move || {
            let outcome = self.service(now, opener);
            WorkerOwned {
                value: Some((self, outcome)),
                runtime,
            }
        });
        async move {
            let mut reply = task.await.or_raise(|| message("servicing repository notifications"))?;
            Ok(reply
                .value
                .take()
                .expect("the service worker returns one complete outcome"))
        }
    }

    pub(super) fn install_async_wakers(&mut self) {
        for domain in [&mut self.metadata, &mut self.worktree] {
            if let Some(watcher) = &mut domain.watcher {
                let wake = self.wake.clone();
                watcher.set_waker(move || wake.notify_one());
            }
        }
        for other in self.other_worktrees.values_mut() {
            other.wake = self.wake.clone();
            other.install_async_wakers();
        }
    }

    fn async_ready(&self, now: Instant) -> bool {
        self.backlog
            || [&self.metadata, &self.worktree]
                .into_iter()
                .any(|domain| domain.watcher.as_ref().is_some_and(gix_notify::Watcher::is_ready))
            || self.async_deadline().is_some_and(|deadline| deadline <= now)
            || self.other_worktrees.values().any(|other| other.async_ready(now))
    }

    fn async_deadline(&self) -> Option<Instant> {
        [self.maintenance, self.pending_metadata.deadline]
            .into_iter()
            .flatten()
            .chain(self.other_worktrees.values().filter_map(Self::async_deadline))
            .min()
    }

    /// Await native notifications or a publication/recovery deadline after servicing this monitor.
    /// This does not consume notifications, and cancellation leaves all queued data intact.
    /// Inactive monitors return immediately. No periodic event polling is performed.
    pub async fn ready_async(&self) {
        if !self.active() {
            return;
        }
        loop {
            let notified = self.wake.notified();
            if self.async_ready(Instant::now()) {
                return;
            }
            match self.async_deadline() {
                Some(deadline) => tokio::select! {
                    _ = notified => {},
                    _ = tokio::time::sleep_until(deadline.into()) => return,
                },
                None => notified.await,
            }
        }
    }

    /// Start shutdown on a Tokio blocking worker and await safe release of all native resources.
    /// Cancelling this operation still leaves teardown owned by the worker.
    pub fn shutdown_async(self) -> impl Future<Output = Result<()>> {
        let task = tokio::task::spawn_blocking(move || drop(self));
        async move {
            task.await
                .or_raise(|| message("shutting down repository notifications"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn abandoning_completed_service_resources_keeps_teardown_off_the_executor() -> gix_error::TestResult {
        struct RecordDrop(Option<tokio::sync::oneshot::Sender<std::thread::ThreadId>>);
        impl Drop for RecordDrop {
            fn drop(&mut self) {
                if let Some(record) = self.0.take() {
                    let _ = record.send(std::thread::current().id());
                }
            }
        }
        let executor = std::thread::current().id();
        let runtime = tokio::runtime::Handle::current();
        let (record, dropped) = tokio::sync::oneshot::channel();
        let completed = tokio::task::spawn_blocking(move || WorkerOwned {
            value: Some(RecordDrop(Some(record))),
            runtime,
        })
        .await?;
        drop(completed);
        assert_ne!(
            tokio::time::timeout(std::time::Duration::from_secs(2), dropped).await??,
            executor,
            "unclaimed native resources are released by a worker even after their reply completed"
        );
        Ok(())
    }
}
