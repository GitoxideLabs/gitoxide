# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2026-11-22)

### New Features

- <csr-id-9a84a31eb6265c80257e9f60a01972a6f0274b49/> add clients and an embeddable filesystem monitor server
  All docs changed to mention their parameters in the doc string.
  Client and hook APIs were altered a bit to make it more flexible. It's interesting
  that Git doesn't allow overriding the daemon, as it's only executing hooks, i.e.
  the user cannot control the server start command which is a bit of a miss. Our client-API
  allows overriding it though.
  
  The server is pretty straightforward, and I mildly improved the IPC function to make clear
  what it can actually do.
  
  Rubber-stamped all tests after perusing them.
  
  <!-- agent -->
  Git clients and long-running applications need filesystem-monitor protocol
  support independently of repository discovery and process management. Add
  `gix-fsmonitor` with hook v1/v2 clients, native daemon clients, and an optional
  application-owned server behind the `server` feature.
  
  Serve bounded token history over Git's Simple IPC protocol using Unix sockets
  or Windows named pipes. Register complete raw worktree coverage before
  readiness, synchronize queries with native fences or ordered Windows cookies,
  and require full verification after loss or expired tokens. Each worktree
  owns its endpoint and watch lifetime; owned shutdown and optional idle expiry
  release the service. Keep coverage independent of a client's index and ignores.
  
  The server accepts injected event sources and defaults to `gix-notify`.
  Drain ready Windows completions atomically before exposing synchronization
  cookies, bounding completion count, bytes, and elapsed time so incomplete
  bursts cannot certify incremental results. Cover native path aliases and
  fences alongside protocol, client, journal, recovery, and ownership tests.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release over the course of 6 calendar days.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge pull request #3028 from GitoxideLabs/repo-sharing-permissions ([`a77e8d4`](https://github.com/GitoxideLabs/gitoxide/commit/a77e8d4170486d8c098faa58ab0774b1eb2a2e85))
  - Merge pull request #3191 from GitoxideLabs/notify-and-more ([`ab7c5dc`](https://github.com/GitoxideLabs/gitoxide/commit/ab7c5dc0596943b42fe9294813a8cdc0383c403a))
  - Add clients and an embeddable filesystem monitor server ([`9a84a31`](https://github.com/GitoxideLabs/gitoxide/commit/9a84a31eb6265c80257e9f60a01972a6f0274b49))

</details>
