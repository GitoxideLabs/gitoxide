use std::{future::Future, time::Instant};

use super::Monitor;
use crate::{
    Repository, Result,
    error::{ResultExt, message},
    notify::{Outcome, WorkerOwned},
};

impl Monitor {
    /// Process this monitor on a Tokio blocking worker and return it with the owned result.
    ///
    /// Use this for filesystem work before servicing, such as resolving subscription changes before
    /// a rescan can open newly excluded worktrees, or for demand-driven [`refresh()`](Self::refresh).
    /// Returning a [`Result`] from `process` retains the monitor on a recoverable processing failure.
    /// Cancellation or a panic leaves native work and teardown on workers; an unclaimed result is
    /// also dropped there. Requires Tokio context and `async-notify`.
    pub fn process_async<T: Send + 'static>(
        mut self,
        process: impl FnOnce(&mut Self) -> T + Send + 'static,
    ) -> impl Future<Output = Result<(Self, T)>> {
        let runtime = tokio::runtime::Handle::current();
        let task = tokio::task::spawn_blocking(move || {
            let processed = process(&mut self);
            WorkerOwned {
                value: Some((self, processed)),
                runtime,
            }
        });
        async move {
            let mut reply = task.await.or_raise(|| message("processing monitored status"))?;
            Ok(reply
                .value
                .take()
                .expect("the status worker returns one complete reply"))
        }
    }

    /// Service root and child notifications on a Tokio blocking worker and return the complete outcome.
    ///
    /// The opener runs on the worker. Status remains pending until explicitly refreshed; use
    /// [`service_async_with()`](Self::service_async_with) to refresh or process on the same worker.
    pub fn service_async(
        self,
        now: Instant,
        opener: impl FnMut() -> Result<Repository> + Send + 'static,
    ) -> impl Future<Output = Result<(Self, Outcome)>> {
        self.service_async_with(now, opener, |_, outcome| outcome)
    }

    /// Service and process the full outcome on one Tokio blocking worker.
    ///
    /// The callback can update subscriptions and refresh pending status. Handle all simultaneous
    /// invalidations and errors. Query [`coverage_status()`](Self::coverage_status) after processing
    /// to observe changes since the service outcome. Cancellation follows [`process_async()`](Self::process_async).
    pub fn service_async_with<T: Send + 'static>(
        self,
        now: Instant,
        opener: impl FnMut() -> Result<Repository> + Send + 'static,
        process: impl FnOnce(&mut Self, Outcome) -> T + Send + 'static,
    ) -> impl Future<Output = Result<(Self, T)>> {
        self.process_async(move |monitor| {
            let outcome = monitor.service(now, opener);
            process(monitor, outcome)
        })
    }

    /// Await root notifications, child-service deadlines, or a due status refresh.
    ///
    /// Refresh due status before waiting again. Root notifications use asynchronous wakeups without
    /// the synchronous polling ceiling; nested submodules retain their bounded polling deadlines.
    /// Use [`ready_service_async()`](Self::ready_service_async) while the status view is hidden.
    /// This does not consume notifications, and cancelling the wait leaves pending data intact.
    pub async fn ready_async(&self) {
        let now = Instant::now();
        let timeout = self
            .next_child_timeout(now)
            .into_iter()
            .chain(self.next_refresh_timeout(now))
            .min();
        self.ready_with_timeout(timeout).await;
    }

    /// Await notification service without waking for an unconsumed status snapshot or refresh retry.
    /// Root notifications are asynchronous; child subscriptions retain bounded polling deadlines.
    pub async fn ready_service_async(&self) {
        self.ready_with_timeout(self.next_child_timeout(Instant::now())).await;
    }

    async fn ready_with_timeout(&self, timeout: Option<std::time::Duration>) {
        match timeout {
            Some(timeout) => tokio::select! {
                _ = self.repository.ready_async() => {},
                _ = tokio::time::sleep(timeout) => {},
            },
            None => self.repository.ready_async().await,
        }
    }

    /// Release the root and every child monitor on a Tokio blocking worker.
    /// Cancelling this operation still leaves native teardown owned by the worker.
    pub fn shutdown_async(self) -> impl Future<Output = Result<()>> {
        let task = tokio::task::spawn_blocking(move || drop(self));
        async move { task.await.or_raise(|| message("shutting down monitored status")) }
    }
}
