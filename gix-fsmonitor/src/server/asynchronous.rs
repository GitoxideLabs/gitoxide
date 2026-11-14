//! Tokio server orchestration. Blocking [`crate::server::spawn()`] remains available with both features.
//!
//! A live Tokio runtime with I/O and timers enabled is required through shutdown. Dropping a
//! handle requests shutdown and leaves cleanup owned by the task; await [`crate::server::asynchronous::Handle::shutdown()`]
//! to observe resource release. Cancelling startup or waiting also requests shutdown. Active
//! exchanges have one five-second deadline; native work abandoned at that deadline still owns
//! its resources until completion. Source notifications, clients and shutdown wake the task directly.

use super::{Context, Layout, Options, cookie, transport};
use gix_error::{Result, ResultExt, bail, message};
use gix_macros::keep as maybe_async;
use gix_notify::{Batch, Budget, Fence, SynchronizeError, Watch, asynchronous::Watcher as NativeWatcher};
use gix_parallel::OwnShared;
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, oneshot};

pub(super) trait EventSource {
    fn housekeeping(&self) -> &Housekeeping;
    async fn register(&mut self, watches: &[Watch]) -> Result;
    fn drain(&mut self, budget: Budget) -> Batch;
    async fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError>;
    fn supports_cookie_synchronization(&self) -> bool;
    async fn close(self) -> Result;
    async fn ready(&self);
}

struct Watcher {
    native: NativeWatcher,
    housekeeping: Housekeeping,
}

impl Watcher {
    fn new(options: gix_notify::Options) -> Result<Self> {
        Ok(Self {
            native: NativeWatcher::new(options)?,
            housekeeping: Housekeeping::default(),
        })
    }
    fn set_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        self.native.set_waker(wake);
    }
}

pub(super) struct Housekeeping {
    io: OwnShared<Semaphore>,
    cookies: OwnShared<Semaphore>,
}

impl Default for Housekeeping {
    fn default() -> Self {
        Self {
            io: OwnShared::new(Semaphore::new(1)),
            cookies: OwnShared::new(Semaphore::new(1)),
        }
    }
}

async fn run_io<T: Send + 'static>(
    budget: &OwnShared<Semaphore>,
    operation: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    let permit = budget.clone().acquire_owned().await.map_err(io::Error::other)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        operation()
    })
    .await
    .map_err(io::Error::other)?
}

impl EventSource for Watcher {
    fn housekeeping(&self) -> &Housekeeping {
        &self.housekeeping
    }
    async fn register(&mut self, watches: &[Watch]) -> Result {
        self.native.replace(watches.iter().cloned())?.await
    }
    fn drain(&mut self, budget: Budget) -> Batch {
        self.native.drain(budget)
    }
    async fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        self.native.synchronize_at(directory, deadline)?.await
    }
    fn supports_cookie_synchronization(&self) -> bool {
        cfg!(windows)
    }
    async fn ready(&self) {
        self.native.ready().await;
    }
    async fn close(self) -> Result {
        let result = self.native.shutdown().await;
        let _io = self
            .housekeeping
            .io
            .acquire()
            .await
            .or_raise(|| message("waiting for filesystem housekeeping"))?;
        let _cookies = self
            .housekeeping
            .cookies
            .acquire()
            .await
            .or_raise(|| message("waiting for cookie cleanup"))?;
        result
    }
}

async fn prepare_cookies<S: EventSource>(source: &S, layout: &Layout) -> io::Result<()> {
    let layout = layout.clone();
    run_io(&source.housekeeping().io, move || layout.prepare_cookies()).await
}
async fn roots_lost<S: EventSource>(source: &S, layout: &Layout) -> io::Result<bool> {
    let layout = layout.clone();
    run_io(&source.housekeeping().io, move || {
        Ok(layout.roots().any(|root| !root.exists()))
    })
    .await
}

struct AsyncCookie {
    path: PathBuf,
    native: Option<cookie::Cookie>,
    permit: Option<OwnedSemaphorePermit>,
    runtime: tokio::runtime::Handle,
}
impl Drop for AsyncCookie {
    fn drop(&mut self) {
        let native = self.native.take();
        let permit = self.permit.take();
        self.runtime.spawn_blocking(move || {
            drop(native);
            drop(permit);
        });
    }
}
async fn create_cookie<S: EventSource>(source: &S, directory: &Path) -> io::Result<AsyncCookie> {
    let directory = directory.to_owned();
    let permit = source
        .housekeeping()
        .cookies
        .clone()
        .acquire_owned()
        .await
        .map_err(io::Error::other)?;
    // Hold the permit through marker cleanup, even if creation or its caller is cancelled.
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let native = cookie::Cookie::create(&directory)?;
        Ok(AsyncCookie {
            path: native.path.clone(),
            native: Some(native),
            permit: Some(permit),
            runtime,
        })
    })
    .await
    .map_err(io::Error::other)?
}
async fn wait_for_cookie<S: EventSource>(source: &S, remaining: Duration, _interval: Duration) {
    let _ = tokio::time::timeout(remaining, source.ready()).await;
}
include!("monitor.rs");

/// Ownership of an asynchronously served endpoint and its cleanup task.
#[must_use = "dropping the handle requests shutdown"]
pub struct Handle {
    endpoint: PathBuf,
    stop: OwnShared<AtomicBool>,
    wake: OwnShared<Notify>,
    task: Option<tokio::task::JoinHandle<Result>>,
}

impl Handle {
    /// The normalized native IPC endpoint owned by this instance.
    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }
    /// Request shutdown without waiting. This targets only the owned server instance.
    pub fn request_shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.wake.notify_one();
    }
    /// Await server exit and safe release of native watches and the endpoint.
    pub async fn wait(mut self) -> Result {
        match self.task.take() {
            Some(task) => task.await.or_raise(|| message("filesystem monitor task failed"))?,
            None => Ok(()),
        }
    }
    /// Request shutdown and await safe resource release. Cancellation still requests cleanup.
    pub async fn shutdown(self) -> Result {
        self.request_shutdown();
        self.wait().await
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

/// Start a Tokio server, awaiting endpoint binding and complete native registration before returning.
/// Cancelling this future requests cleanup, including when a native registration is still active.
pub async fn spawn(context: Context, options: Options) -> Result<Handle> {
    if options.idle_timeout.is_some_and(|timeout| timeout.is_zero()) {
        bail!(message("filesystem monitor idle timeout must be positive"));
    }
    let endpoint = context.endpoint.clone();
    let stop = OwnShared::new(AtomicBool::new(false));
    let shutdown = stop.clone();
    let wake = OwnShared::new(Notify::new());
    let worker_wake = wake.clone();
    let (ready, receive) = oneshot::channel();
    let task = tokio::spawn(async move {
        let context = tokio::task::spawn_blocking(move || -> Result<Context> {
            let context = context.normalize()?;
            context.check_worktree(options.allow_remote)?;
            Ok(context)
        })
        .await
        .or_raise(|| message("validating filesystem monitor context"))??;
        let mut listener = Listener::bind(&context.endpoint)
            .await
            .or_raise(|| message("acquiring the native IPC endpoint"))?;
        let result = async {
            let mut watcher = Watcher::new(Default::default())?;
            let source_wake = worker_wake.clone();
            watcher.set_waker(move || source_wake.notify_one());
            let mut monitor = Monitor::new(Layout::from_context(context), watcher, options).await?;
            if ready.send(monitor.layout.endpoint.clone()).is_err() {
                monitor.close().await?;
                return Ok(());
            }
            let result = run(&mut listener, &mut monitor, &shutdown, &worker_wake, options).await;
            monitor.close().await?;
            if let Ok(Some((mut stream, deadline))) = result {
                let _ = tokio::time::timeout_at(deadline, crate::asynchronous::write_message(&mut stream, &[])).await;
                let _ = listener.finish(stream, deadline).await;
                Ok(())
            } else {
                result.map(|_| ())
            }
        }
        .await;
        listener
            .close()
            .await
            .or_raise(|| message("releasing the native IPC endpoint"))?;
        result
    });
    let mut handle = Handle {
        endpoint,
        stop,
        wake,
        task: Some(task),
    };
    match receive.await {
        Ok(endpoint) => {
            handle.endpoint = endpoint;
            Ok(handle)
        }
        Err(_) => {
            handle.wait().await?;
            bail!(message("filesystem monitor exited before reporting readiness"));
        }
    }
}

async fn run(
    listener: &mut Listener,
    monitor: &mut Monitor<Watcher>,
    stop: &AtomicBool,
    wake: &Notify,
    options: Options,
) -> Result<Option<(Stream, tokio::time::Instant)>> {
    let mut last_activity = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let more = monitor.maintenance(Instant::now()).await;
        if monitor.root_lost
            || !listener
                .owns_endpoint()
                .await
                .or_raise(|| message("checking native IPC ownership"))?
        {
            bail!(message(
                "repository root or filesystem monitor endpoint was removed or replaced"
            ));
        }
        let mut timeout = Duration::from_secs(60);
        if let Some(retry) = monitor.retry {
            timeout = timeout.min(retry.saturating_duration_since(Instant::now()));
        }
        if let Some(idle) = options.idle_timeout {
            timeout = timeout.min(idle.saturating_sub(last_activity.elapsed()));
        }
        if more {
            timeout = Duration::ZERO;
        }
        let stream = tokio::select! {
            biased;
            result = listener.accept() => Some(result.or_raise(|| message("accepting an IPC client"))?),
            _ = wake.notified() => None,
            _ = tokio::time::sleep(timeout) => None,
        };
        let Some(mut stream) = stream else {
            if options
                .idle_timeout
                .is_some_and(|timeout| last_activity.elapsed() >= timeout)
            {
                return Ok(None);
            }
            tokio::task::yield_now().await;
            continue;
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let request = tokio::time::timeout_at(deadline, crate::asynchronous::read_message(&mut stream, 16384)).await;
        last_activity = Instant::now();
        let request = match request {
            Ok(Ok(Some(request))) if !request.contains(&0) => request,
            _ => {
                let _ = listener.finish(stream, deadline).await;
                continue;
            }
        };
        if request == b"quit" {
            return Ok(Some((stream, deadline)));
        }
        if request == b"flush" {
            monitor.journal.reset();
        }
        let result = tokio::time::timeout_at(deadline, async {
            let response = monitor.response(&request, Instant::now()).await;
            crate::asynchronous::write_message(&mut stream, &response).await
        })
        .await;
        if result.is_err() {
            monitor.journal.reset();
        }
        let _ = listener.finish(stream, deadline).await;
        last_activity = Instant::now();
    }
    Ok(None)
}

#[cfg(unix)]
type Stream = tokio::net::UnixStream;
#[cfg(windows)]
type Stream = tokio::net::windows::named_pipe::NamedPipeServer;

struct Listener {
    #[cfg(unix)]
    owner: Option<transport::Listener>,
    #[cfg(unix)]
    runtime: tokio::runtime::Handle,
    #[cfg(unix)]
    ownership: OwnShared<Semaphore>,
    #[cfg(unix)]
    socket: tokio::net::UnixListener,
    #[cfg(windows)]
    pipe: Option<Stream>,
}

impl Listener {
    async fn bind(path: &Path) -> io::Result<Self> {
        #[cfg(unix)]
        {
            let path = path.to_owned();
            let owner = tokio::task::spawn_blocking(move || transport::Listener::bind(&path))
                .await
                .map_err(io::Error::other)??;
            let socket = match owner.async_socket() {
                Ok(socket) => socket,
                Err(error) => {
                    tokio::task::spawn_blocking(move || drop(owner))
                        .await
                        .map_err(io::Error::other)?;
                    return Err(error);
                }
            };
            Ok(Self {
                owner: Some(owner),
                runtime: tokio::runtime::Handle::current(),
                socket,
                ownership: OwnShared::new(Semaphore::new(1)),
            })
        }
        #[cfg(windows)]
        {
            let name = crate::pipe::name(path)?;
            let pipe = tokio::task::spawn_blocking(move || transport::create_async_pipe(&name))
                .await
                .map_err(io::Error::other)??;
            Ok(Self { pipe: Some(pipe) })
        }
    }
    async fn accept(&mut self) -> io::Result<Stream> {
        #[cfg(unix)]
        {
            self.socket.accept().await.map(|(stream, _)| stream)
        }
        #[cfg(windows)]
        {
            let pipe = self
                .pipe
                .as_ref()
                .ok_or_else(|| io::Error::other("IPC exchange already active"))?;
            pipe.connect().await?;
            self.pipe
                .take()
                .ok_or_else(|| io::Error::other("IPC listener was lost"))
        }
    }
    #[cfg(unix)]
    async fn owns_endpoint(&self) -> io::Result<bool> {
        let owner = self
            .owner
            .as_ref()
            .ok_or_else(|| io::Error::other("IPC endpoint was released"))?;
        run_io(&self.ownership, owner.ownership_check()).await
    }
    #[cfg(windows)]
    fn owns_endpoint(&self) -> std::future::Ready<io::Result<bool>> {
        std::future::ready(Ok(true))
    }
    #[cfg(unix)]
    async fn close(mut self) -> io::Result<()> {
        let owner = self.owner.take();
        run_io(&self.ownership, move || {
            drop(owner);
            Ok(())
        })
        .await
    }
    #[cfg(windows)]
    fn close(mut self) -> std::future::Ready<io::Result<()>> {
        self.pipe.take();
        std::future::ready(Ok(()))
    }
    #[cfg(unix)]
    fn finish(&mut self, stream: Stream, _deadline: tokio::time::Instant) -> std::future::Ready<io::Result<()>> {
        drop(stream);
        std::future::ready(Ok(()))
    }

    #[cfg(windows)]
    async fn finish(&mut self, mut stream: Stream, deadline: tokio::time::Instant) -> io::Result<()> {
        use tokio::io::AsyncReadExt;
        // Disconnect discards unread output; wait for the one-request client to close first.
        let finished = tokio::time::timeout_at(deadline, async {
            let mut byte = [0];
            loop {
                match stream.read(&mut byte).await {
                    Ok(0) => return Ok::<_, io::Error>(()),
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
                    Err(error) => return Err(error),
                }
            }
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "IPC client did not close"));
        let disconnected = stream.disconnect();
        self.pipe = Some(stream);
        finished??;
        disconnected
    }
}

#[cfg(unix)]
impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            self.runtime.spawn_blocking(move || drop(owner));
        }
    }
}

#[cfg(test)]
#[path = "async_tests.rs"]
mod tests;
