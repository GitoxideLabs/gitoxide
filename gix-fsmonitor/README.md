# gix-fsmonitor

Clients for Git's filesystem monitor hook protocols (versions 1 and 2) and native
Simple IPC protocol over Unix sockets or Windows named pipes. Missing native
daemons can be started through Git on macOS and Windows.

Callers supply endpoint paths, opaque token bytes, and command context. Repository
configuration, trust decisions, index checkpoints, and status invalidation remain
with the caller. Unix socket paths can exceed the platform address limit without
changing the process working directory. Retry counts, retry delays, Unix I/O
timeouts, and response limits are configurable through `client::daemon::Options`.
Defaults allow 100 retries at 50 ms intervals, a 30-second Unix timeout, and
64 MiB of response data. Windows response reads remain synchronous.

Enable the optional `server` feature to run the monitor inside an application.
`server::spawn(Context)` takes owned worktree and administrative paths, an IPC
endpoint, and filesystem settings. It returns a `Handle` after the endpoint is
bound and native watches are registered. Watches remain active while the server
runs, including between queries. Initial registration failures are returned
to the caller; failures after startup invalidate client baselines and trigger
recovery.

Use `server::spawn_with_options(Context, Options)` to configure `idle_timeout`.
It defaults to `None`, which keeps the server running until explicitly stopped.
A positive duration stops the server after that much time without a client
connection. Completed requests and status probes restart the idle period;
filesystem events do not. An active exchange finishes before idle shutdown.
Expiry releases watches and the endpoint, just like explicit shutdown.

Use `server::spawn_with_source(Context, Options, make_source)` to inject a
`server::EventSource`. The factory receives validated, normalized context paths
on the server worker thread; only the factory must be `Send`, not the source.
Roots and the endpoint's parent use their native filesystem spelling, and the
returned handle exposes the normalized endpoint. Readiness requires binding,
cookie directory preparation, and successful source registration. Factory or
registration failures release resources before returning.

An event source installs the entire supplied watch set on **one stream**,
including the worktree root, cookies, and parent sentinels. It may add its own
worktree root to that same stream; native watch deduplication handles overlap.
Repeated registration repairs coverage without discarding generation history.
Draining must preserve raw events for all watched paths, including administrative
paths and sentinels, along with loss, diagnostics, generation, and sequence.
Fences must certify that same stream: source failures, unknown event locations,
and lost coverage cannot certify incremental results. The server retains path
conversion, journal, recovery, and root-loss handling, and never creates a second
watcher alongside an injected source.

`gix_notify::Watcher` implements the trait and remains the default for `spawn`
and `spawn_with_options`. An application or daemon depending on `gix` can instead
adapt `gix::notify::WorktreeStream`, using the native types re-exported through
`gix::notify`, without adding a porcelain dependency to this library.

The server does not periodically force new client baselines. Reported event loss,
incomplete delivery fences, and tokens outside the retained journal history still
require full scans. Native root checks and delivery fences do not detect every
silent notification gap. Applications that need protection against those gaps
must arrange independent full verification.

Use `Handle::shutdown()` to stop and join the server, or `request_shutdown()`
followed by `wait()` to separate the request from waiting. Dropping the handle
also stops and joins it. Joining can wait for an active client exchange or
uncancellable native synchronization, so perform it away from UI threads.
Shutdown targets the owned instance, even if another server has subsequently
claimed its endpoint. Git's native IPC `quit` command can also stop a server.
No pause/resume API is provided.

There is normally one endpoint per **worktree**, at its private Git directory's
`fsmonitor--daemon.ipc`; Windows derives a named-pipe name from that path.
Linked worktrees therefore have separate endpoints and watch lifetimes.
Starting a second owner of an occupied endpoint fails, allowing applications to
use the existing service. Restarting establishes a new token epoch and requires
a fresh client baseline.

The standalone `gix-fsmonitor` executable is provided by the
[`gix-fsmonitor-daemon`](../gix-fsmonitor-daemon) package. It handles discovery,
configuration and process launching around this server runtime. The optional
server uses plumbing dependencies and `gix-notify`, keeping the library free of
a dependency on `gix`. Native macOS/Linux synchronization permits incremental
replies; Windows currently conservatively requests full scans.
