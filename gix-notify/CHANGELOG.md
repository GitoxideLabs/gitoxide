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

- 4 commits contributed to the release over the course of 6 calendar days.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge pull request #3191 from GitoxideLabs/notify-and-more ([`ab7c5dc`](https://github.com/GitoxideLabs/gitoxide/commit/ab7c5dc0596943b42fe9294813a8cdc0383c403a))
  - Exercise `gix-notify` on FreeBSD through the QA scheduler ([`9022bec`](https://github.com/GitoxideLabs/gitoxide/commit/9022bec4f0d1e4ed60550533c332c8a38d8af1ca))
  - Avoid retaining ancestor descriptors on macOS ([`ad30af9`](https://github.com/GitoxideLabs/gitoxide/commit/ad30af9d45f66dac891835035fb0f312e6e758f8))
  - Provide native filesystem notifications ([`1ac8fe1`](https://github.com/GitoxideLabs/gitoxide/commit/1ac8fe18d7499ecd1463517bad4cf4dca9901f0d))

</details>
