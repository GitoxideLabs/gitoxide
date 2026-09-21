use std::{future::Future, time::Instant};

use super::WorktreeStatusHandle;
use crate::{
    Result,
    error::{ResultExt, message},
    notify::{Outcome, WorkerOwned},
};

impl WorktreeStatusHandle {
    /// Process this monitor on a Tokio blocking worker and return it with the owned result.
    ///
    /// Use this for demand-driven [`refresh()`](Self::refresh) or submodule service.
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

    /// Service this checkout and its submodule notifications on a Tokio blocking worker and return the complete outcome.
    ///
    /// Status remains pending until explicitly refreshed. Use
    /// [`service_async_with()`](Self::service_async_with) to refresh or process on the same worker.
    pub fn service_async(self, now: Instant) -> impl Future<Output = Result<(Self, Outcome)>> {
        self.service_async_with(now, |_, outcome| outcome)
    }

    /// Service and process the full outcome on one Tokio blocking worker.
    ///
    /// The callback can refresh pending status. Handle all simultaneous
    /// invalidations and errors. Query [`coverage_status()`](Self::coverage_status) after processing
    /// to observe changes since the service outcome. Cancellation follows [`process_async()`](Self::process_async).
    pub fn service_async_with<T: Send + 'static>(
        self,
        now: Instant,
        process: impl FnOnce(&mut Self, Outcome) -> T + Send + 'static,
    ) -> impl Future<Output = Result<(Self, T)>> {
        self.process_async(move |monitor| {
            let outcome = monitor.service(now);
            process(monitor, outcome)
        })
    }

    /// Await root or submodule notifications, maintenance deadlines, or a due status refresh.
    ///
    /// Refresh due status before waiting again. Both root and nested native watchers wake this wait.
    /// Use [`ready_service_async()`](Self::ready_service_async) while the status view is hidden.
    /// This does not consume notifications, and cancelling the wait leaves pending data intact.
    pub async fn ready_async(&self) {
        self.wait(true).await;
    }

    /// Await project invalidations or a submodule service deadline without waking for a pending refresh.
    pub async fn ready_service_async(&self) {
        self.wait(false).await;
    }

    async fn wait(&self, include_refresh: bool) {
        loop {
            let notified = self.subscription.wake.notified();
            let now = Instant::now();
            let refresh = if include_refresh {
                self.next_refresh_timeout(now).map(|timeout| now + timeout)
            } else {
                None
            };
            let deadline = self.service_deadline(now).into_iter().chain(refresh).min();
            if self.is_closed() || deadline.is_some_and(|deadline| deadline <= now) {
                return;
            }
            match deadline {
                Some(deadline) => tokio::select! {
                    _ = notified => {},
                    _ = tokio::time::sleep_until(deadline.into()) => return,
                },
                None => notified.await,
            }
        }
    }

    /// Release the status cache and return its submodule watches to the project on a Tokio blocking worker.
    /// Cancelling this operation still leaves native teardown owned by the worker.
    pub fn shutdown_async(self) -> impl Future<Output = Result<()>> {
        let task = tokio::task::spawn_blocking(move || drop(self));
        async move { task.await.or_raise(|| message("shutting down monitored status")) }
    }
}
