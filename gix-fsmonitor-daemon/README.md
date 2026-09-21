# gix-fsmonitor-daemon

A standalone filesystem monitor for Git's hook v2 and native Simple IPC
protocols. This executable wraps the embeddable server in
[`gix-fsmonitor`](../gix-fsmonitor), supplying its filesystem events through
`gix::Repository::monitor()` and `RepositoryMonitor::into_worktree_stream()`.
The detached stream keeps no repository handle open while idle. Applications can
enable the server crate's `server` feature to own its lifetime without launching
an executable.

Build and install it from this workspace:

```sh
cargo install --path gix-fsmonitor-daemon
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
verification. Filesystems without synchronization support and the Windows
backend return Git's full-invalidation marker. Windows protocol
and lifecycle support is functional, but it does not accelerate Git status until
native synchronization is available.
Windows clients and the daemon should run at the same elevation; Git's custom
ACL for crossing elevation levels is not implemented. Windows builds and tests
are cross-compiled, with runtime validation still outstanding.

The daemon selects `gix::notify::Options` with `metadata: false` and
`worktree_coverage: WorktreeCoverage::All`. Coverage includes every worktree path
regardless of ignore rules or the default index. Git interprets the candidate
paths against the client's own index and configuration. Applications using
`repo.monitor()` directly can instead retain the default `IgnoreAware` coverage
and disable metadata to watch only worktree changes. That mode keeps the index,
configuration and ignore dependencies needed to maintain its watch inventory. Administrative files are excluded. Pathnames retain native Unix
bytes and follow the repository's Unicode composition configuration.

The journal is bounded to 16,384 changes, 16 MiB, and five minutes. Overflow,
expired or malformed tokens, daemon restarts, reported event loss, unknown paths,
and incomplete synchronization require full verification. The daemon does not
periodically force new client baselines. Native synchronization has a one-second
deadline; each IPC exchange has a five-second deadline. Failed registration is
retried while idle. Removing or replacing a repository root or IPC endpoint
closes the obsolete daemon so clients can start a fresh instance.

Native root checks and delivery fences do not detect every silent notification
gap. Callers that need protection against those gaps must arrange independent
full verification.

Tests exercise Git status parity, hook/native interoperability in both
directions, linked worktrees, alternate indices, forced tracked ignored files,
Unicode composition, root loss, protocol limits, journal loss, and endpoint
ownership. Native integration tests need permission to bind a local socket and
receive filesystem notifications. They use disposable repositories and isolated
Git environments.
