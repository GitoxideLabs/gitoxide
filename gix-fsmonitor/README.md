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
and native control commands. The caller supplies a server context, the logical
Git directory for reopening, output/cancellation handlers, and arguments for its
own executable's foreground `run` command. Startup appends idle-timeout/readiness
options, waits for the child's PID notification, and cleans up unsuccessful children.

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

On Windows, the server uses Git-style cookie observation when native
synchronization is unsupported. The native watcher publishes ready worktree
completions before exposing cookies, including when a linked worktree's cookie
directory has a separate native watch. The server waits up to one second for its
unique cookie and consumes the complete queued batch; missing cookies,
notification loss, or changed watch coverage require a full scan.

Reported event loss, incomplete delivery fences, and tokens outside the retained journal history 
require full scans. Native root checks and delivery fences do not detect every
silent notification gap. Applications that need protection against those gaps
must arrange independent full verification.

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
