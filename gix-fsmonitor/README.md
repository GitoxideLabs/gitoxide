# gix-fsmonitor

Clients for Git's filesystem monitor hook protocols (versions 1 and 2) and native
Simple IPC protocol over Unix sockets or Windows named pipes. Missing native
daemons can be started through a caller-provided callback, or through Git by
default on macOS and Windows. The optional `server` feature provides native endpoint
selection and an in-process monitor server. The separate `daemon` feature adds shared
blocking daemon control and detached self-spawn with startup readiness. It includes
`server`, while keeping daemon-only dependencies out of server-only builds.

## Daemon (`daemon` feature)

`daemon::endpoint()` remains a compatibility re-export of `server::endpoint()`.
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
changing the process working directory. `client::daemon::Options` configures
connection retries, retry delays, I/O deadlines, and response limits for both
blocking and async clients on Unix and Windows. Defaults allow 100 retries at
50 ms intervals, 30 seconds for each connection attempt and, separately, the
complete request/response exchange, and 64 MiB of response data. Partial I/O does
not reset the exchange deadline. `socket_timeout: None` disables deadlines;
zero expires immediately. Daemon startup and retry delays are not included in
these I/O budgets; blocking filesystem preparation is not cancellable.

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

`server::endpoint()` selects the native endpoint without creating directories or
binding it, and is available without the `daemon` feature. It canonicalizes the
Git directory and, on Unix, classifies that directory's filesystem. Local Git
directories use `<git_dir>/fsmonitor--daemon.ipc` without consulting the socket
directory callback. Remote Git directories use the canonical worktree pathname
bytes to construct `.git-fsmonitor-<SHA1>` in the caller's trusted, already-interpolated
socket directory, requesting it exactly once. This SHA-1 is independent of Git
object formats. Windows always uses the canonical Git directory's
`fsmonitor--daemon.ipc` path and derives a named-pipe name from it.
Linked worktrees therefore have separate endpoints and watch lifetimes.
Starting a second owner of an occupied endpoint fails, allowing applications to
use the existing service. Restarting establishes a new token epoch and requires
a fresh client baseline.

On Windows, the named pipe uses Git's ACL, granting the `Everyone` group
read/write access so local clients can connect across elevation levels.
Remote named-pipe clients are rejected.

## Async execution (`async-tokio` feature)

Enable `async-tokio` for Tokio-based clients and server orchestration. Blocking
APIs remain available alongside their async counterparts.

| Required features       | APIs                                                                      |
| ----------------------- | ------------------------------------------------------------------------- |
| `async-tokio`           | `client::daemon::query_async`, `client::hook::query_async`                  |
| `server`, `async-tokio` | `server::asynchronous::spawn`, `server::ipc::{query_async,is_running_async}` |

Applications supply an entered Tokio runtime with I/O and timers enabled and
keep it alive through shutdown. Hooks and default Git startup also require Tokio
process support. A current-thread runtime is supported. Tokio is an optional
dependency enabled by `async-tokio`; blocking execution does not require a runtime.

### Server and IPC examples

Both frontends take a caller-prepared `server::Context` containing the paths and
filesystem settings described above. They return a handle only after binding the
endpoint and registering native watches.

Blocking use requires `server` and no async runtime:

```rust,ignore
use gix_fsmonitor::server::{self, Options, ipc::{self, Request}};
let server = server::spawn(context, Options::default())?;
let response = ipc::query(server.endpoint(), Request::Message(b""))?;
server.shutdown()?;
```

Async use requires both `server` and `async-tokio`, within a Tokio runtime:

```rust,ignore
use gix_fsmonitor::server::{self, Options, ipc::{self, Request}};
let server = server::asynchronous::spawn(context, Options::default()).await?;
let response = ipc::query_async(server.endpoint(), Request::Message(b"")).await?;
server.shutdown().await?;
```

Blocking and async clients receive identical raw token/path responses and can
query either server frontend. `server::ipc::query` and `query_async` both use one
five-second I/O deadline covering connection, request, and response, and accept
at most 32 MiB of response data. Unlike `client::daemon` queries, these helpers
never start a missing daemon.

### Cancellation and ownership

Dropping a `server::asynchronous::Handle` requests shutdown without joining on
the caller thread. Await `shutdown()` to observe safe resource release, or call
`request_shutdown()` and then await `wait()`. Cancelling server startup or a wait
also requests cleanup.

Deadlines do not imply native cancellation: synchronization work may continue
until it can safely release its resources. Filesystem housekeeping runs on
bounded workers whose permits remain held through completion and cookie cleanup,
even when waits are cancelled. Batches are ingested before asynchronous root
checks; interrupted checks retain conservative recovery state. Server shutdown
awaits housekeeping and endpoint cleanup.

Cancelling an async client query closes its connection; a daemon startup or a
transmitted control request may still execute. Cancelling an async hook query
kills its active child process.

The server's event interpretation, bounded journal, generation/token invalidation,
fence consumption and response construction are shared. Its blocking and Tokio
loops use their own transport and deadline adapters. Async cookie waits are
notification-driven, bounded by the fence deadline; they do not poll every 20 ms.
Native watcher root checks and the server's ownership check remain unchanged.
