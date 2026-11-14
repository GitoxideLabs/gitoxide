# gix-fsmonitor

Clients for Git's filesystem monitor hook protocols (versions 1 and 2) and native
Simple IPC protocol over Unix sockets or Windows named pipes. Missing native
daemons can be started through a caller-provided callback, or through Git by
default on macOS and Windows. The optional `server` feature provides an in-process
monitor server. The separate `daemon` feature adds shared blocking daemon control,
native endpoint selection, and detached self-spawn with startup readiness. It
includes `server`, while keeping daemon-only dependencies out of server-only builds.

## Daemon (`daemon` feature)

`daemon::endpoint()` selects the native endpoint from repository paths, requesting
an interpolated socket directory from the caller only for remote Git directories.
`daemon::execute()` handles foreground serving, detached startup, hook autostart,
and native control commands through `daemon::Command::{Run, Start, Stop, Status,
Flush, Query, Hook, Path}`. The caller supplies a `server::Context`, the logical
Git directory for reopening, `daemon::Options`, output/cancellation handlers, and
a `child_arguments` callback. Repository discovery and configuration resolution
remain the caller's responsibility.

`child_arguments` returns an **argument prefix**, not an executable path: startup
reexecutes the **current executable** in its foreground `run` mode. The prefix
must preserve application-specific options and configuration. Do not include
`--idle-timeout` or `--ready-file`; the launcher appends both. The child must pass
them to `daemon::Command::Run`, which reports its PID only after server startup
succeeds. Startup waits for that child's readiness notification and cleans up
unsuccessful children. The callback is invoked only when starting a child.
`Options::start_timeout` defaults to 60 seconds, matching `fsmonitor.startTimeout`;
zero checks readiness once without waiting. Foreground serving and idle shutdown
are unaffected by this timeout.

`Options::allow_remote` defaults to false, matching `fsmonitor.allowRemote`.
Daemon commands check the worktree's filesystem independently of Git-directory
endpoint placement. `Command::Path` only resolves the endpoint. Filesystem-probe
errors remain failures even when remote monitoring is allowed. Detached children
receive both resolved settings through appended `GIT_CONFIG_COUNT` entries, so
in-memory policy overrides survive reopening. The foreground entrypoint must
honor Git's configuration environment; other configuration overrides remain the
caller's responsibility.

## Client

Callers supply endpoint paths, opaque token bytes, and command context. Repository
configuration, trust decisions, index checkpoints, and status invalidation remain
with the caller. Unix socket paths can exceed the platform address limit without
changing the process working directory. Retry counts, retry delays, Unix I/O
timeouts, and response limits are configurable through `client::daemon::Options`.
Defaults allow 100 retries at 50 ms intervals, a 30-second Unix timeout, and
64 MiB of response data. Windows response reads remain synchronous.

## Server (`server` feature)

### Startup and idle shutdown

Enable the optional `server` feature to run the monitor inside an application.
`server::spawn(Context, Options)` takes owned worktree and administrative paths,
an IPC endpoint, filesystem settings, and server options. It returns a `Handle`
after the endpoint is bound and native watches are registered. Watches remain active while the server
runs, including between queries. Initial registration failures are returned
to the caller; failures after startup invalidate client baselines and trigger
recovery.

Remote worktrees require `Options::allow_remote=true`. Classification uses Git's
native macOS, Linux, and Windows rules; Linux includes all FUSE filesystems.
The permission does not guarantee reliable notifications or override full-scan
fallbacks. It does not change IPC endpoint placement.

The server waits for IPC connections, source notifications, or shutdown through
socket readiness on Unix and overlapped named-pipe connections and native events
on Windows. Recovery and idle shutdown use their own deadlines; an otherwise idle
server checks endpoint ownership once per minute. Synchronization cookies retain
the 20 ms maximum polling interval. Native watcher root checks remain active.

Configure idle shutdown with `Options::idle_timeout`. Pass `Options::default()`
to keep the default behavior: `idle_timeout` is `None`, so the server runs until
explicitly stopped. A positive duration stops the server after that much time without a client
connection. Completed requests and status probes restart the idle period;
filesystem events do not. An active exchange finishes before idle shutdown.
Expiry releases watches and the endpoint, just like explicit shutdown.

### Consistency guarantees

Native macOS/Linux synchronization and Windows cookie observation permit
incremental replies.

On Windows, the server uses Git-style cookie observation when native
synchronization is unsupported. The native watcher publishes ready worktree
completions before exposing cookies, including when a linked worktree's cookie
directory has a separate native watch. The server waits up to one second for its
unique cookie and consumes the complete queued batch; missing cookies,
notification loss, or changed watch coverage require a full scan.

Reported event loss, incomplete delivery fences, and tokens outside the retained
journal history require full scans. Native root checks and delivery fences do not
detect every silent notification gap. Applications that need protection against
those gaps must arrange independent full verification.

### Explicit shutdown

Use `Handle::shutdown()` to stop and join the server, or `request_shutdown()`
followed by `wait()` to separate the request from waiting. Dropping the handle
also stops and joins it. Joining can wait for an active client exchange or
uncancellable native synchronization, so perform it away from UI threads.
Shutdown targets the owned instance, even if another server has subsequently
claimed its endpoint. Use `server::ipc::query(endpoint, Request::Quit)` with
`server::ipc::Request` to send Git's native IPC `quit` command to the service
currently owning an endpoint, without starting a missing daemon. `Request::Flush`
invalidates token history; `Request::Message(&[u8])` sends arbitrary bytes,
typically interpreted as a checkpoint token. Empty or unknown tokens request a
full baseline; literal `quit` and `flush` payloads remain control commands.
No pause/resume API is provided.

### Endpoints and ownership

There is normally one endpoint per **worktree**, at its private Git directory's
`fsmonitor--daemon.ipc`; Windows derives a named-pipe name from that path.
Linked worktrees therefore have separate endpoints and watch lifetimes.
Starting a second owner of an occupied endpoint fails, allowing applications to
use the existing service. Restarting establishes a new token epoch and requires
a fresh client baseline.

On Windows, the named pipe uses Git's ACL, granting the `Everyone` group
read/write access so local clients can connect across elevation levels.
Remote named-pipe clients are rejected.

Applications using `gix` can enable its `fsmonitor-server` feature and use:

```rust,ignore
let repo = gix::open(worktree)?;
let server = repo.fsmonitor_server(gix::fsmonitor::server::Options {
    idle_timeout: Some(std::time::Duration::from_secs(300)),
    ..Default::default()
})?;
// External Git clients can now use this server with core.fsmonitor=true.
// Idle shutdown also releases the server without an explicit shutdown request.
server.shutdown()?;
```

The helper leaves repository configuration unchanged and retains no repository
handle. It uses the private Git directory's endpoint; alternate socket locations
for remote Git directories are not selected. This API serves external Git
clients and does not enable filesystem-monitor acceleration in `gix` status
operations. Git queries after shutdown can start Git's own daemon on macOS/Windows
while `core.fsmonitor=true` remains enabled.

## Async libraries

Async execution is opt-in and coexists with blocking execution. None of the new
features is enabled by normal `gix` CLI or daemon feature bundles. Both binaries
continue calling blocking APIs even when workspace feature unification exposes
async library APIs.

| Layer | Opt-in feature | API and runtime |
| --- | --- | --- |
| `gix-notify` | `async` | `asynchronous::Watcher`; runtime-independent operations, readiness, fences and shutdown |
| `gix-fsmonitor` | `async` | `client::daemon::query_async`, `client::hook::query_async`; Tokio |
| `gix-fsmonitor` | `server,async` | `server::asynchronous::spawn`, `server::ipc::{query_async,is_running_async}`; Tokio |
| `gix` | `async-notify` | `RepositoryMonitor::{service_async,ready_async,shutdown_async}` and raw watcher futures |
| `gix` | `async-fsmonitor` | Async clients via `gix::fsmonitor` |
| `gix` | `async-fsmonitor-server` | `Repository::fsmonitor_server_async` and async server IPC |

Tokio integrations require an entered runtime with I/O and timers enabled, kept
alive through shutdown. Hooks and default Git startup also require Tokio process
support. A current-thread runtime is supported. Applications supply their own
Tokio runtime; blocking-only library and binary builds do not depend on Tokio.
The existing networking feature selection is independent of these features.

### End-to-end examples through `gix`

Blocking use requires `fsmonitor-server` and no async runtime:

```rust,ignore
use gix::fsmonitor::server::{Options, ipc::{self, Request}};
let server = gix::open(worktree)?.fsmonitor_server(Options::default())?;
let response = ipc::query(server.endpoint(), Request::Message(b""))?;
server.shutdown()?;
```

Async use requires `async-fsmonitor-server`, within a Tokio runtime:

```rust,ignore
use gix::fsmonitor::server::{Options, ipc::{self, Request}};
let server = gix::open(worktree)?.fsmonitor_server_async(Options::default()).await?;
let response = ipc::query_async(server.endpoint(), Request::Message(b"")).await?;
server.shutdown().await?;
```

Both clients receive identical raw token/path responses and can query either
server frontend. Unknown tokens, loss, incompatible generations, or incomplete
fences require a full baseline. Async exchanges enforce response limits before
payload allocation and use one deadline across their request and response, on
Unix and Windows. Daemon client `Options::socket_timeout=None` explicitly disables
that exchange deadline. IPC control exchanges always have a five-second deadline.

Run the executable examples with:

```sh
cargo run -p gix --features fsmonitor-server --example fsmonitor-blocking -- /path/to/worktree
cargo run -p gix --features async-fsmonitor-server --example fsmonitor-async -- /path/to/worktree
cargo run -p gix --features notify --example notify -- /path/to/repository
cargo run -p gix --features async-notify --example notify-async -- /path/to/repository
```

For repository-aware notifications, construction remains synchronous: it prepares
the detached layout, without installing native watches. Async service moves the
monitor into an already-started worker operation and returns it together with the
complete outcome. Keep both values, process errors and invalidations, then await
readiness for native events or metadata-publication/recovery deadlines:

```rust,ignore
let mut monitor = repo.monitor(gix::notify::Options::default())?;
let git_dir = repo.git_dir().to_owned();
loop {
    let path = git_dir.clone();
    let (next, outcome) = monitor.service_async(std::time::Instant::now(), move || gix::open(&path)).await?;
    monitor = next;
    // Process the complete outcome, including loss-driven invalidations and errors.
    monitor.ready_async().await;
}
```

The raw watcher is available as `gix::notify::plumbing::asynchronous::Watcher`.
Its synchronous `replace()` and `synchronize_at()` initiate native work and
return operations supporting either `.await` or `.wait()`. `ready().await` only
observes readiness; `drain()` synchronously consumes the bounded batch.

### Cancellation and ownership

Dropping a raw watcher operation abandons its reply and does not cancel native
work. Watch invalidations stay in the shared queue; unobserved registration
errors become queue diagnostics, including replies completed before abandonment.
Only one control operation may be outstanding. `idle().await` waits for that
control operation to finish, but a timed-out native flush can still be active:
further operations may report `Busy`. Deadlines never imply native cancellation.
The control owner retains streams, callbacks and marker state until safe teardown.
Filesystem housekeeping also runs on bounded workers. Their permits remain held
through completion and cookie cleanup when waits are cancelled. Batches are
ingested before asynchronous root checks; interrupted checks retain conservative
recovery state. Server shutdown awaits housekeeping and endpoint cleanup.

Dropping raw async watchers or async server handles requests shutdown without
joining on the caller thread. Await `shutdown()` to observe safe resource release.
Cancelling server startup also requests cleanup. Cancelling repository async
service disposes that stream after active work finishes; it never returns a
monitor with an unobserved outcome. Start a fresh monitor and baseline to resume.
Cancelled readiness waits preserve notifications. Cancelling a client closes its
connection; a daemon startup or a transmitted control request may still execute.
Cancelling an async hook query kills its active child process.

The server's event interpretation, bounded journal, generation/token invalidation,
fence consumption and response construction are shared. Its blocking and Tokio
loops use their own transport and deadline adapters. Async cookie waits are
notification-driven, bounded by the fence deadline; they do not poll every 20 ms.
Native watcher root checks and the server's ownership check remain unchanged.
