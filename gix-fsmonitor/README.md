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
