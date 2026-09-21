# gix-fsmonitor

A standalone filesystem monitor for Git's hook v2 and native Simple IPC
protocols. It uses `gix-notify` for filesystem events and keeps no repository
handle open while idle.

Build and install it from this workspace:

```sh
cargo install --path gix-fsmonitor
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

The macOS backend synchronizes each query through a marker under the worktree's
Git directory, including linked worktrees with an external administrative
directory. This provides useful incremental replies, including changes made
immediately before a query. The current Linux and Windows compatibility
backends cannot certify delivery; they return Git's full-invalidation marker.
Their protocol and lifecycle support is functional, but they do not accelerate
Git status until native synchronization is available.
Windows clients and the daemon should run at the same elevation; Git's custom
ACL for crossing elevation levels is not implemented. Windows builds and tests
are cross-compiled, with runtime validation still outstanding.

Coverage includes every worktree path regardless of ignore rules or the default
index. Git interprets the candidate paths against the client's own index and
configuration. Administrative files are excluded. Pathnames retain native Unix
bytes and follow the repository's Unicode composition configuration.

The journal is bounded to 16,384 changes, 16 MiB, and five minutes. Overflow,
expired or malformed tokens, daemon restarts, unknown paths, and incomplete
synchronization require full verification. A fixed 60-second safety deadline
also invalidates all client baselines, independently of event traffic. Native
synchronization has a one-second deadline; each IPC exchange has a five-second
deadline. Failed registration is retried while idle. Removing or replacing a
repository root or IPC endpoint closes the obsolete daemon so clients can start
a fresh instance.

Tests exercise Git status parity, hook/native interoperability in both
directions, linked worktrees, alternate indices, forced tracked ignored files,
Unicode composition, root loss, protocol limits, journal loss, and endpoint
ownership. Native integration tests need permission to bind a local socket and
receive filesystem notifications. They use disposable repositories and isolated
Git environments.
