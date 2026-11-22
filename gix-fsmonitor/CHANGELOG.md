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

- 4 commits contributed to the release over the course of 6 calendar days.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3028 from GitoxideLabs/repo-sharing-permissions ([`a77e8d4`](https://github.com/GitoxideLabs/gitoxide/commit/a77e8d4170486d8c098faa58ab0774b1eb2a2e85))
  - Merge pull request #3191 from GitoxideLabs/notify-and-more ([`ab7c5dc`](https://github.com/GitoxideLabs/gitoxide/commit/ab7c5dc0596943b42fe9294813a8cdc0383c403a))
  - Add clients and an embeddable filesystem monitor server ([`9a84a31`](https://github.com/GitoxideLabs/gitoxide/commit/9a84a31eb6265c80257e9f60a01972a6f0274b49))

</details>
