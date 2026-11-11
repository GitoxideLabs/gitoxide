# gix-fsmonitor-daemon

A standalone filesystem monitor for Git's hook v2 and native Simple IPC
protocols. This executable-only package provides the `gix-fsmonitor` binary.
The daemon retains no repository handle or caches.

Detached startup executes the running `gix-fsmonitor` binary in foreground
`run` mode. The executable and shared implementation remain blocking and
Tokio-free in normal builds.

To embed a monitor in an application instead of using this executable, see the
[`gix` repository API](../gix/src/repository/fsmonitor.rs).

## Repository configuration

Remote worktrees are rejected unless `fsmonitor.allowRemote=true`, independently
of the Git directory's filesystem and endpoint placement. This permission follows
Git's macOS, Linux, and Windows classification rules, including all Linux FUSE
filesystems. Probe errors still fail, and notification gaps still require full
verification. Like Git's explicit native daemon commands, control commands also
check worktree eligibility; `path` only reports the endpoint.

`fsmonitor.startTimeout` controls how long detached startup and hook autostart wait
for readiness. It defaults to 60 seconds; zero checks readiness once without
waiting. Negative or malformed values are rejected. This does not affect
foreground serving or idle shutdown. These settings respect the repository's
configuration trust filter and are forwarded to detached children through Git's
configuration environment, including values resolved from in-memory overrides.
Failed startup still kills and reaps the child, rather than leaving an unconfirmed
daemon behind as Git does on a readiness timeout.

`fsmonitor.ipcThreads` and `fsmonitor.announceStartup` are not consumed by this
implementation. IPC handling is single-threaded, and startup announcements are
not emitted.

## Standalone usage

Build and install it from this workspace:

```sh
cargo install --path gix-fsmonitor-daemon --features sha1,sha256
```

Use hook v2 when Git should automatically start this daemon:

```sh
git config core.fsmonitor 'gix-fsmonitor hook'
git config core.fsmonitorHookVersion 2
```

Or start the daemon explicitly and let Git use native IPC on platforms where
Git supports it:

```sh
gix-fsmonitor start
git config core.fsmonitor true
```

With `core.fsmonitor=true`, Git starts its own daemon if the endpoint disappears.
The hook configuration selects `gix-fsmonitor` for automatic startup. Both
implementations can serve the same native clients, and neither can take over an
occupied endpoint. Unix startup uses Git's temporary exclusive lockfile protocol;
Windows uses the named pipe's exclusive first-instance flag. An abandoned Unix
startup lock is reported rather than removed automatically. `git fsmonitor--daemon status` and `stop` also work with this
daemon.

`run` stays in the foreground. `start`, `status`, `stop`, and `flush` control the
daemon. `query [TOKEN]` prints the binary hook response, and `path` prints the
native endpoint. Every command accepts `--repo PATH`; the default is the current
directory. Hook v1 is intentionally unsupported.

`run`, `start`, and `hook` accept `--idle-timeout SECONDS` to exit successfully
after that many seconds without a client connection. The default, `0`, disables
idle shutdown. Queries and status probes reset the timeout after their exchanges
finish; filesystem activity alone does not keep the daemon alive. An active
exchange finishes before idle shutdown. For example:

```sh
gix-fsmonitor start --idle-timeout 300
git config core.fsmonitor 'gix-fsmonitor hook --idle-timeout 300'
```

`start` and hook autostart pass the timeout to their detached child. It applies
when starting a new instance; it does not reconfigure an already running daemon.
After idle shutdown releases the watches and endpoint, the next hook invocation
starts a fresh daemon and asks Git for a full baseline.

Watches are registered during startup and remain active while the daemon runs.
An initial registration failure fails startup; later coverage failures are
retried. `stop` releases watches and the IPC endpoint, while `flush` only resets
the token history. Detached daemons outlive the process that launched them.

The macOS and Linux backends synchronize each query through a marker under the
worktree's Git directory, including linked worktrees with an external administrative
directory. This provides useful incremental replies, including changes made
immediately before a query, on supported local filesystems. Linux uses inotify;
directory moves or lost native coverage require registration recovery and full
verification. Windows uses Git's cookie-observation approach: it creates a unique
marker in the same administrative directory and waits for its notification before
replying. Ready worktree notifications are published before external cookies, so
linked worktrees also receive incremental results. Missing cookies, notification
loss, and filesystems without supported synchronization require full verification.
Windows clients and the daemon should run at the same elevation; Git's custom
ACL for crossing elevation levels is not implemented.

The native watcher covers all worktree paths, deliberately independent of ignore
rules and the default index. A client may use an alternate index that tracks paths
ignored or absent from the default index, so filtering daemon coverage against
that default would miss relevant changes. Git interprets the candidate paths
against each client's own index and configuration. Administrative files are
excluded from replies. Pathnames retain native Unix bytes and follow the
repository's Unicode composition configuration.

The journal is bounded to 16,384 changes, 16 MiB, and five minutes. Overflow,
expired or malformed tokens, daemon restarts, reported event loss, unknown paths,
and incomplete synchronization require full verification. Native synchronization
has a one-second deadline; each IPC exchange has a five-second deadline. Failed
registration is retried while idle. Removing or replacing a repository root or
IPC endpoint closes the obsolete daemon so clients can start a fresh instance.

Native root checks and delivery fences do not detect every silent notification
gap. Callers that need protection against those gaps must arrange independent
full verification.
