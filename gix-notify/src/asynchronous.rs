//! Futures backed by one control owner and the existing native watcher implementation.
//!
//! Initiating an operation is synchronous and nonblocking. Its [`crate::asynchronous::Operation`] supports both
//! `.await` and [`crate::asynchronous::Operation::wait()`]; no runtime or timer is required. Native work runs on an
//! owned control thread, including registration, native waits and teardown. The bounded event
//! queue, publication generations and fences are the same as those of [`crate::Watcher`].
//!
//! Dropping an operation abandons its reply, **not** its native work. Only one operation may
//! be outstanding; use [`crate::asynchronous::Watcher::idle()`] before retrying after abandoning a reply. Fence
//! deadlines bound the reply wait but do not cancel an uncancellable native flush. Unobserved
//! registration failures become queue diagnostics. Dropping a watcher requests shutdown without
//! joining on the caller's thread; the owner retains native resources until safe teardown finishes.
//! Await [`crate::asynchronous::Watcher::shutdown()`] to observe completion and release of all resources.

use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
    time::Instant,
};

use async_channel::{Receiver, Sender};
use event_listener::Event;
use gix_error::{ErrorExt, Result, ResultExt, conflict, message};
use gix_parallel::{Mutable, OwnShared, lock};

use crate::{Batch, Budget, Fence, Options, SynchronizeError, Watch, queue};

/// A started native operation. Dropping it does not cancel native work.
#[must_use = "await or wait for the result; dropping it only abandons the reply"]
pub struct Operation<T> {
    receive: Receiver<T>,
    future: Pin<Box<dyn Future<Output = T> + Send>>,
    stopped: fn() -> T,
    abandoned: Option<Box<dyn Fn(T) + Send>>,
}

impl<T> Unpin for Operation<T> {}

impl<T: Send + 'static> Operation<T> {
    fn new(receive: Receiver<T>, stopped: fn() -> T) -> Self {
        let reply = receive.clone();
        Self {
            receive,
            future: Box::pin(async move { reply.recv().await.unwrap_or_else(|_| stopped()) }),
            stopped,
            abandoned: None,
        }
    }

    /// Wait on the calling thread. Async callers should await this operation instead.
    pub fn wait(self) -> T {
        self.receive.recv_blocking().unwrap_or_else(|_| (self.stopped)())
    }
}

impl<T> Future for Operation<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        self.future.as_mut().poll(context)
    }
}

impl<T> Drop for Operation<T> {
    fn drop(&mut self) {
        // Close before inspecting the reply, so completion cannot race abandonment and lose a diagnostic.
        self.receive.close();
        if let Some(abandoned) = &self.abandoned
            && let Ok(reply) = self.receive.try_recv()
        {
            abandoned(reply);
        }
    }
}

type Completion = Box<dyn FnOnce() + Send>;
type Command = Box<dyn FnOnce(&mut crate::Watcher) -> Completion + Send>;

struct Activity {
    busy: AtomicBool,
    changed: Event,
}

/// An asynchronous frontend owning one native stream and its bounded notification queue.
pub struct Watcher {
    state: OwnShared<Mutable<queue::State>>,
    commands: Sender<Command>,
    activity: OwnShared<Activity>,
    finished: Receiver<Result>,
}

impl Watcher {
    /// Create an empty watcher and its control owner. Native coverage starts with [`Self::replace()`].
    pub fn new(options: Options) -> Result<Self> {
        // Validate with the blocking constructor, without acquiring native resources.
        drop(crate::Watcher::new(options)?);
        let state = OwnShared::new(Mutable::new(queue::State::new(options)));
        let native_state = state.clone();
        let (commands, receive) = async_channel::bounded::<Command>(1);
        let (done, finished) = async_channel::bounded(1);
        let activity = OwnShared::new(Activity {
            busy: AtomicBool::new(false),
            changed: Event::new(),
        });
        let active = activity.clone();
        std::thread::Builder::new()
            .name("gix-notify control".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut watcher = crate::Watcher {
                        state: native_state.clone(),
                        options,
                        desired: Vec::new(),
                        backend: None,
                    };
                    while let Ok(command) = receive.recv_blocking() {
                        let complete = command(&mut watcher);
                        active.busy.store(false, Ordering::Release);
                        active.changed.notify(usize::MAX);
                        complete();
                    }
                    drop(watcher);
                }))
                .map_err(|_| message("filesystem notification control thread panicked").raise());
                if result.is_err() {
                    {
                        let mut state = lock(&native_state);
                        state.invalidate(crate::Loss::BackendStopped);
                        state.needs_restart = true;
                        state.error =
                            Some(message("filesystem notification control owner stopped unexpectedly").raise());
                    }
                    queue::wake(&native_state);
                }
                active.busy.store(false, Ordering::Release);
                active.changed.notify(usize::MAX);
                let _ = done.try_send(result);
            })
            .or_raise(|| message("starting the filesystem notification control owner"))?;
        Ok(Self {
            state,
            commands,
            activity,
            finished,
        })
    }

    /// Whether native recursive coverage is supported; identical to [`crate::Watcher`].
    pub const fn supports_native_recursive_watches() -> bool {
        crate::Watcher::supports_native_recursive_watches()
    }

    /// Install a lightweight event-loop callback, without replacing future readiness notifications.
    pub fn set_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        lock(&self.state).waker = Some(OwnShared::new(wake));
        queue::wake(&self.state);
    }

    fn start(&self, command: Command) -> std::result::Result<(), SynchronizeError> {
        self.activity
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| SynchronizeError::Busy)?;
        if self.commands.try_send(command).is_err() {
            self.activity.busy.store(false, Ordering::Release);
            self.activity.changed.notify(usize::MAX);
            return Err(SynchronizeError::Stopped);
        }
        Ok(())
    }

    /// Start watch replacement. Only one native operation may be outstanding.
    /// Validation, normalization and coverage changes follow [`crate::Watcher::replace()`].
    pub fn replace(&mut self, watches: impl IntoIterator<Item = Watch>) -> Result<Operation<Result>> {
        let watches: Vec<_> = watches.into_iter().collect();
        let (reply, receive) = async_channel::bounded(1);
        self.start(Box::new(move |watcher| {
            let result = watcher.replace(watches);
            let state = watcher.state.clone();
            Box::new(move || {
                if let Err(undelivered) = reply.try_send(result)
                    && let Err(error) = undelivered.into_inner()
                {
                    lock(&state).error = Some(error);
                    queue::wake(&state);
                }
            })
        }))
        .or_raise(|| conflict("filesystem notification operation could not start"))?;
        let mut operation = Operation::new(receive, || {
            Err(conflict("filesystem notification owner stopped").raise())
        });
        let state = self.state.clone();
        operation.abandoned = Some(Box::new(move |result| {
            if let Err(error) = result {
                lock(&state).error = Some(error);
                queue::wake(&state);
            }
        }));
        Ok(operation)
    }

    /// Await completion of an outstanding operation without consuming notifications or its reply.
    pub async fn idle(&self) {
        loop {
            let changed = self.activity.changed.listen();
            if !self.activity.busy.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    /// Wait for events, loss or diagnostics, without consuming them. Cancellation leaves the queue intact.
    pub async fn ready(&self) {
        loop {
            let ready = lock(&self.state).ready.clone();
            let notified = ready.listen();
            if lock(&self.state).is_ready() {
                return;
            }
            notified.await;
        }
    }

    /// Consume ready notifications synchronously, with the same budget and guarantees as [`crate::Watcher::drain()`].
    pub fn drain(&mut self, budget: Budget) -> Batch {
        lock(&self.state).drain(budget)
    }

    /// Initiate a same-stream marker fence. Awaiting never waits for native work on the executor thread.
    /// The deadline and unsupported-platform behavior follow [`crate::Watcher::synchronize_at()`].
    pub fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
    ) -> std::result::Result<Operation<std::result::Result<Fence, SynchronizeError>>, SynchronizeError> {
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        let generation = lock(&self.state).fence()?.generation;
        let directory = directory.to_owned();
        let (reply, receive) = async_channel::bounded(1);
        self.start(Box::new(move |watcher| {
            let result = watcher.synchronize_at_generation(&directory, deadline, generation);
            Box::new(move || {
                let _ = reply.try_send(result);
            })
        }))?;
        Ok(Operation::new(receive, || Err(SynchronizeError::Stopped)))
    }

    /// Request shutdown now and return an operation completing after safe native teardown.
    /// Dropping the returned operation still allows the owner to finish cleanup.
    pub fn shutdown(self) -> Operation<Result> {
        self.commands.close();
        Operation::new(self.finished.clone(), || {
            Err(conflict("filesystem notification owner stopped").raise())
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.commands.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completing_an_operation_releases_the_control_slot_before_delivering_its_reply() -> gix_error::TestResult {
        futures_lite::future::block_on(async {
            let mut watcher = Watcher::new(Default::default())?;
            for _ in 0..100 {
                watcher.replace([])?.await?;
                watcher.replace([])?.wait()?;
            }
            watcher.shutdown().await?;
            Ok(())
        })
    }

    #[test]
    fn future_wakers_run_outside_the_queue_lock() -> gix_error::TestResult {
        struct ReentrantWake {
            state: OwnShared<Mutable<queue::State>>,
            called: OwnShared<AtomicBool>,
        }
        impl std::task::Wake for ReentrantWake {
            fn wake(self: std::sync::Arc<Self>) {
                let _state = lock(&self.state);
                self.called.store(true, Ordering::Relaxed);
            }
        }
        let mut watcher = Watcher::new(Default::default())?;
        let called = OwnShared::new(AtomicBool::new(false));
        let waker = std::task::Waker::from(OwnShared::new(ReentrantWake {
            state: watcher.state.clone(),
            called: called.clone(),
        }));
        let mut context = Context::from_waker(&waker);
        {
            let mut ready = std::pin::pin!(watcher.ready());
            assert!(
                ready.as_mut().poll(&mut context).is_pending(),
                "an empty queue registers its future waker"
            );
            let sender = queue::Sender::new(watcher.state.clone(), 0);
            sender.publish(Vec::new(), Some(crate::Loss::Rescan), None);
        }
        assert!(
            called.load(Ordering::Relaxed),
            "executor wakeups may inspect the shared queue without deadlocking"
        );
        let _batch = watcher.drain(Budget::default());
        watcher.shutdown().wait()?;
        Ok(())
    }

    #[test]
    fn outstanding_work_survives_abandonment_and_shutdown_waits_for_it() -> gix_error::TestResult {
        futures_lite::future::block_on(async {
            let mut watcher = Watcher::new(Default::default())?;
            let (entered, receive_entered) = std::sync::mpsc::sync_channel(1);
            let (release, receive_release) = std::sync::mpsc::sync_channel(1);
            watcher.start(Box::new(move |_| {
                entered.send(()).expect("the test observes native work starting");
                receive_release
                    .recv()
                    .expect("the test eventually releases native work");
                Box::new(|| {})
            }))?;
            receive_entered.recv_timeout(std::time::Duration::from_secs(1))?;
            let busy = watcher
                .replace([])
                .err()
                .expect("an active native operation prevents replacement");
            assert_eq!(
                busy.downcast_any_ref::<SynchronizeError>(),
                Some(&SynchronizeError::Busy)
            );
            let mut cleanup = Box::pin(watcher.shutdown());
            assert!(
                futures_lite::future::poll_once(&mut cleanup).await.is_none(),
                "shutdown does not complete or block the executor while native work is outstanding"
            );
            release.send(())?;
            cleanup.await?;
            Ok(())
        })
    }

    #[test]
    fn cancelling_readiness_does_not_consume_loss_or_diagnostics() -> gix_error::TestResult {
        futures_lite::future::block_on(async {
            let mut watcher = Watcher::new(Default::default())?;
            assert!(
                futures_lite::future::poll_once(watcher.ready()).await.is_none(),
                "an empty queue is not ready"
            );
            let sender = queue::Sender::new(watcher.state.clone(), 0);
            sender.publish(
                Vec::new(),
                Some(crate::Loss::Overflow),
                Some(message("diagnostic").raise()),
            );
            watcher.ready().await;
            let batch = watcher.drain(Budget::default());
            assert_eq!(
                batch.loss,
                Some(crate::Loss::Overflow),
                "readiness cancellation preserves coverage loss"
            );
            assert!(batch.error.is_some(), "readiness cancellation preserves diagnostics");
            drop(sender);
            watcher.shutdown().await?;
            Ok(())
        })
    }
}
