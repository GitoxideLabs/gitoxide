# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2026-11-22)

### New Features

- <csr-id-1ac8fe18d7499ecd1463517bad4cf4dca9901f0d/> provide native filesystem notifications
  Knowing how the venerable `notify` crate sometimes doesn't work as one would expect,
  particularly on Linux, it's probably worth it to give it another shot with assistance,
  thinking that todays models can probably do a better job, ultimately.
  
  Walked through the watcher tests (high-level API) and actually tried to understand and
  follow. Many changes were done based on my feedback.
  
  <!-- agent -->
  Long-running applications such as Tix need bounded event intake, explicit
  coverage loss, replaceable subscriptions, and predictable shutdown without
  owning platform notification machinery themselves. Provide these facilities in
  the Git-independent `gix-notify` crate so repository monitoring and other event
  loops can share the same contract.
  
  Bound queued events and diagnostic samples, wake consumers without holding the
  queue lock, and invalidate coverage on overflow, disconnection, or root
  replacement. Canonicalize registrations, preserve native path representations,
  and keep file, nonrecursive-directory, and recursive-directory interests
  separate from the native registrations used to implement them.
  
  Own notification delivery on macOS through FSEvents, on Linux through inotify,
  and on Windows through `ReadDirectoryChangesW` and a completion-port worker.
  macOS and Linux support administrative-marker delivery fences. Windows uses
  bounded asynchronous reads, validates native records, and drains cancelled
  operations before releasing their buffers; delivery synchronization remains
  explicitly unsupported there. Retain an upstream `notify` compatibility backend
  for other platforms.
  
  Include the workspace registration, dependencies, documentation, example, queue
  tests, and native backend tests with the complete crate. Repository-specific
  watch inventories and application integration can build on this independently.

### Bug Fixes

- <csr-id-ad30af9d45f66dac891835035fb0f312e6e758f8/> avoid retaining ancestor descriptors on macOS
  Added application-like test so there is more cross-platform coverage of what
  apps would be expected to do.
  Also made hardcoded 'check-root-interval' value configurable.
  Finally, make use of error classes more consistent.
  
  <!-- agent -->
  Long-running `tix` monitors exposed directory descriptors retained by FSEvents
  `WatchRoot` for every root and ancestor. Sibling roots duplicate their shared
  ancestors even in one stream.
  
  Remove `WatchRoot` and validate reduced native root identities once per second,
  including while idle, and around delivery fences. Drain and release a failed
  stream before reporting one backend error. Consumers can re-register identical
  paths without repeated errors or wakeups. Root checks do not scan directories
  and cannot detect every transient rename and restoration between checks.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 5 commits contributed to the release over the course of 6 calendar days.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3191 from GitoxideLabs/notify-and-more ([`ab7c5dc`](https://github.com/GitoxideLabs/gitoxide/commit/ab7c5dc0596943b42fe9294813a8cdc0383c403a))
  - Exercise `gix-notify` on FreeBSD through the QA scheduler ([`9022bec`](https://github.com/GitoxideLabs/gitoxide/commit/9022bec4f0d1e4ed60550533c332c8a38d8af1ca))
  - Avoid retaining ancestor descriptors on macOS ([`ad30af9`](https://github.com/GitoxideLabs/gitoxide/commit/ad30af9d45f66dac891835035fb0f312e6e758f8))
  - Provide native filesystem notifications ([`1ac8fe1`](https://github.com/GitoxideLabs/gitoxide/commit/1ac8fe18d7499ecd1463517bad4cf4dca9901f0d))

</details>
