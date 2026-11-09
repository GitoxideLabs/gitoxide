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
bound and native watches are registered. Watches remain active while the handle
is alive, including between queries. Initial registration failures are returned
to the caller; failures after startup invalidate client baselines and trigger
recovery.

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

Applications using `gix` can enable its `fsmonitor-server` feature and use:

```rust,ignore
let repo = gix::open_opts(
    worktree,
    gix::open::Options::default().config_overrides(["core.fsmonitor=true"]),
)?;
let server = repo.fsmonitor_server()?;
// Ordinary status calls now use the running server. Persist their index updates
// when requested so subsequent calls can reuse the filesystem-monitor checkpoint.
server.shutdown()?;
```

The helper leaves repository configuration unchanged and retains no repository
handle. Its endpoint matches the current status client in the private Git
directory; alternate socket locations for remote Git directories remain outside
that client's support. Status queries after shutdown can start Git's own daemon
on macOS/Windows while `core.fsmonitor=true` remains enabled.

The standalone `gix-fsmonitor` executable is provided by the
[`gix-fsmonitor-daemon`](../gix-fsmonitor-daemon) package. It handles discovery,
configuration and process launching around this server runtime. The optional
server uses plumbing dependencies and `gix-notify`, keeping the library free of
a dependency on `gix`. Native macOS/Linux synchronization permits incremental
replies; Windows currently conservatively requests full scans.
