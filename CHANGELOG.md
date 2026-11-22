# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.62.0 (2026-11-22)

Headline benefits of this extra-curricular release are various security improvements,
and greatly improved error handling and consisstency. Plumbing crates for instance will now
use a standard `std::error::Error` compatible error type.

Besides that, the tracing of all commands was greatly improved.

### Changed

- <csr-id-c9441a7d7fff32f66bc715f34b5de98cd889b53a/> require Rust 1.91 for CLI tooling
  <!-- agent -->
  Keep `gix` and its plumbing on Rust 1.88. The supported library API does not
  require a broad move to Rust 1.99, as experimental module gates can use
  inline bodies with `include!` on the existing MSRV.
  
  Declare Rust 1.91 for `gitoxide`, `gix-tix`, and `internal-tools` to match
  the tooling dependencies, and make `gitoxide-core`'s Rust 1.88 requirement
  explicit. Document the per-crate MSRV policy and express Tix's existing
  seven-day log retention with `Duration::from_hours`.

### New Features

- <csr-id-dd5e45b5adbc951f2699d1ed1f59513320516a65/> set repository trust with `--repo-trust`
  <!-- agent -->
  Let CLI users compare how repository configuration behaves at different
  trust levels. Use `full` or `reduced` to supply initial trust, or
  `force-full` or `force-reduced` to set the final level.
  
  Print the effective trust level in `gix env` when the selected repository
  can be opened.
- <csr-id-5d2b98322c5da6a998ddb6702c33ccdcb52c7477/> add `gix notify`, `gix watch`, and self-spawning `gix fsmonitor`
  A very straightforward implementation of the subcommands above. By the book,
  nothing special, and all working nicely.
  
  <!-- agent -->
  Expose filesystem notifications, repository invalidations, and Git-compatible
  filesystem monitor control through the installed `gix` binary. These commands
  make native coverage and recovery observable and let applications exercise the
  public `gix` APIs against real worktrees.
  
  `gix notify` observes files and multiple directory roots without Git discovery.
  Print event and path kinds, loss, diagnostics, coverage generations, and
  publication sequences. Expose recursion, queue limits, drain budgets, and root
  check intervals. Bounded wakeups drive delivery, and interruption between
  batches permits graceful teardown under sustained filesystem activity.
  
  Move the repository notification example into `gix watch`. Report metadata and
  worktree invalidations, checkout inventory, linked-worktree lifecycle changes,
  watcher statistics, and recovery diagnostics. Expose coverage, debounce, burst
  latency, retry, and idle-wait controls. Stream output immediately, including
  with `-v`, and release watches on Ctrl-C.
  
  `gix fsmonitor` adapts the public daemon API for foreground service, detached
  startup, status, shutdown, flush, binary token queries, endpoint inspection,
  and Git hook-v2 autostart. Reexecute the running binary as `fsmonitor run` and
  forward configuration overrides. Shared daemon facilities preserve repository
  identity, wait for native child readiness, and clean up unsuccessful startup.
  The standalone executable supplies its own adapter and starts itself too.
  Resolve remote-worktree policy and startup timing from trusted repository
  configuration, and carry the resolved values into the detached child.
  
  Share packet framing between blocking clients and servers and their async
  adapters. Preserve binary payloads, cumulative limits, interrupted-read retries,
  and timeout or connection errors. Use one Windows pipe-name transformation for
  all transports, retaining native wide characters and rejecting NUL bytes.
  Use standard file opening for Windows client handles and `rustix` for Unix
  session creation and macOS filesystem classification, preserving validation
  errors, native I/O causes, and inspection context.
  
  Expose endpoint selection for already-canonical paths so repository daemon
  contexts avoid resolving them twice. Unresolved paths still use the original
  helper, which resolves the worktree only when a remote endpoint needs it.
  Tests cover equivalent endpoint selection and local Git-directory paths that
  do not require a worktree.
  
  Keep all CLI commands blocking when async library features are unified; normal
  default and `small` builds remain Tokio-free. Enable the workspace lint policy
  for `gix-imara-diff` with matching code, test, and provenance adaptations, and
  remove unused `fs-err` feature, dependency, and lockfile wiring.
  
  Document debugging controls, daemon lifecycle, hook setup, and platform
  requirements. Isolated integration tests cover live output, Ctrl-C, multiple
  roots, daemon control and tokens, Git hook status parity, linked worktrees,
  configuration forwarding, symlink-dependent includes, and idle shutdown.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 23 commits contributed to the release.
- 31 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3220 from GitoxideLabs/trust-in-CLI ([`689a724`](https://github.com/GitoxideLabs/gitoxide/commit/689a72495f618743599e5e2ba65f278b98079d33))
  - Set repository trust with `--repo-trust` ([`dd5e45b`](https://github.com/GitoxideLabs/gitoxide/commit/dd5e45b5adbc951f2699d1ed1f59513320516a65))
  - Merge pull request #3028 from GitoxideLabs/repo-sharing-permissions ([`a77e8d4`](https://github.com/GitoxideLabs/gitoxide/commit/a77e8d4170486d8c098faa58ab0774b1eb2a2e85))
  - Merge pull request #3191 from GitoxideLabs/notify-and-more ([`ab7c5dc`](https://github.com/GitoxideLabs/gitoxide/commit/ab7c5dc0596943b42fe9294813a8cdc0383c403a))
  - Add `gix notify`, `gix watch`, and self-spawning `gix fsmonitor` ([`5d2b983`](https://github.com/GitoxideLabs/gitoxide/commit/5d2b98322c5da6a998ddb6702c33ccdcb52c7477))
  - Merge pull request #3211 from GitoxideLabs/jiff-upgrade ([`12eff45`](https://github.com/GitoxideLabs/gitoxide/commit/12eff45f0e16fa7a9f75086034ccb0198f55b9f3))
  - Merge pull request #3187 from trevorlasn/status-macos-thread-cap ([`8118e5e`](https://github.com/GitoxideLabs/gitoxide/commit/8118e5e6867dcbe1194206e97c648b1939dccaf9))
  - Review ([`6f51b5c`](https://github.com/GitoxideLabs/gitoxide/commit/6f51b5c7ae7674039f8a21305b7905115ce11db0))
  - Merge pull request #3036 from ameyypawar/merge-base-is-ancestor ([`e2743fe`](https://github.com/GitoxideLabs/gitoxide/commit/e2743fe28ef61f06585f68ec65e0e9033931e729))
  - Merge pull request #3165 from GitoxideLabs/encoding ([`1b025f7`](https://github.com/GitoxideLabs/gitoxide/commit/1b025f7cda5411be7f77f01a911881431a6aa737))
  - Merge pull request #3168 from GitoxideLabs/fix-ident-filter ([`f06f106`](https://github.com/GitoxideLabs/gitoxide/commit/f06f106b1d5b158f90a41d1d64970be76c1cfd2b))
  - Merge pull request #3152 from GitoxideLabs/even-faster-qa ([`2baa819`](https://github.com/GitoxideLabs/gitoxide/commit/2baa819ba6d4b0c6177e00902928ce94ee7f537e))
  - Reduce development and CI artifacts while preserving runtime checks ([`d2e37e5`](https://github.com/GitoxideLabs/gitoxide/commit/d2e37e57cb33791113cf1328362504ad65497b2c))
  - Merge pull request #3143 from GitoxideLabs/fast-ci ([`c4a28f8`](https://github.com/GitoxideLabs/gitoxide/commit/c4a28f8c9331603ee2e2928974ee587a23026c49))
  - Parse workflow YAML in `internal-tools` without `yq` ([`59ff9df`](https://github.com/GitoxideLabs/gitoxide/commit/59ff9dfdebed68a8ee9d6f3a671891590016eda4))
  - Discover QA work from manifests and share it between local runs and CI ([`8bf25a2`](https://github.com/GitoxideLabs/gitoxide/commit/8bf25a232d00f71f73b964f39d509ac9ae976efe))
  - Merge pull request #3061 from GitoxideLabs/stable-gix ([`5c88568`](https://github.com/GitoxideLabs/gitoxide/commit/5c88568c5ad8ab85ef4d60d64b4ab48958dc3226))
  - Require Rust 1.91 for CLI tooling ([`c9441a7`](https://github.com/GitoxideLabs/gitoxide/commit/c9441a7d7fff32f66bc715f34b5de98cd889b53a))
  - Merge pull request #3127 from srijs/gix-zlib-sirocco-deflate ([`8644bf3`](https://github.com/GitoxideLabs/gitoxide/commit/8644bf3a05ec10a9fcf6bfa461b79d0378825aad))
  - Review ([`9b8c4e1`](https://github.com/GitoxideLabs/gitoxide/commit/9b8c4e141a5ad26793fcf31896051f258e15a68c))
  - Merge pull request #3092 from GitoxideLabs/brace-brace ([`15b71a3`](https://github.com/GitoxideLabs/gitoxide/commit/15b71a3a0bfbab5e1667d26f2259805f4fecb216))
  - Merge pull request #3083 from GitoxideLabs/report-october ([`e87b6ee`](https://github.com/GitoxideLabs/gitoxide/commit/e87b6eefbcaa77156ddee85576c847e395ec0586))

</details>

## 0.61.0 (2026-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 14 calendar days.
- 14 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.2, gix-date v0.18.1, gix-actor v0.44.1, gix-trace v0.3.1, gix-validate v0.13.1, gix-path v0.14.1, gix-quote v0.10.1, gix-command v0.12.1, gix-hash v0.28.1, gix-fs v0.24.1, gix-tempfile v26.0.1, gix-object v0.66.1, gix-glob v0.29.1, gix-attributes v0.37.1, gix-packetline v0.24.1, gix-filter v0.36.1, gix-chunk v0.10.1, gix-commitgraph v0.42.0, gix-revwalk v0.38.0, gix-traverse v0.64.0, gix-worktree-stream v0.39.0, gix-archive v0.39.0, gix-bitmap v0.6.1, gix-lock v26.0.1, gix-index v0.58.0, gix-config-value v0.21.1, gix-pathspec v0.22.1, gix-worktree v0.59.0, gix-imara-diff v0.3.2, gix-diff v0.70.0, gix-blame v0.20.0, gix-ref v0.69.1, gix-config v0.62.1, gix-prompt v0.19.1, gix-url v0.40.1, gix-credentials v0.42.1, gix-discover v0.57.1, gix-dir v0.32.0, gix-mailmap v0.36.1, gix-revision v0.52.0, gix-merge v0.23.0, gix-negotiate v0.38.0, gix-note v0.3.1, gix-zlib v0.4.0, gix-pack v0.77.0, gix-odb v0.87.0, gix-refspec v0.47.1, gix-shallow v0.15.1, gix-transport v0.62.0, gix-protocol v0.68.0, gix-status v0.37.0, gix-submodule v0.36.1, gix-worktree-state v0.37.0, gix v0.90.0, gix-fsck v0.27.1, gitoxide-core v0.64.0, gix-tix v0.6.0, gitoxide v0.61.0, safety bump 21 crates ([`dd39fb6`](https://github.com/GitoxideLabs/gitoxide/commit/dd39fb66e3b1551c9fe849043e39b357a9799ca1))
  - Merge pull request #3065 from srijs/claude/project-thread-kr3hdl ([`0a49d46`](https://github.com/GitoxideLabs/gitoxide/commit/0a49d46f6a0b08943930d5247693992b89e98c1b))
  - Merge pull request #3064 from GitoxideLabs/various-improvements ([`8c513c5`](https://github.com/GitoxideLabs/gitoxide/commit/8c513c59cc7f3892dfae2235045214ad5dae088b))
  - Merge pull request #3066 from GitoxideLabs/check-commit-graph ([`7540f69`](https://github.com/GitoxideLabs/gitoxide/commit/7540f69bf871c851b826437f7efc5d84b5d9962f))
  - Merge pull request #3037 from ameyypawar/gix-remote-default-branch ([`ad0202e`](https://github.com/GitoxideLabs/gitoxide/commit/ad0202e3ab396d752a56664001c134f48d834177))
  - Merge pull request #3048 from GitoxideLabs/new-release ([`5fccccf`](https://github.com/GitoxideLabs/gitoxide/commit/5fccccfdedd17f7d4b0cc57039d118a3496aa8aa))

</details>

## 0.60.0 (2026-10-08)

### New Features

- <csr-id-bc278ac4b79c31ff9e9cc2a3f0c3d435ca7c7f8d/> make trace output independently configurable
  <!-- Byron -->
  
  Quick review only, as it's mostly about tracing and `gix` which both aren't super
  important, and are seemingly working well. An extra agent-review couldn't
  find anything either.
  
  <!-- agent -->
  Replace the old boolean trace switch with a bounded `-t`/`--trace` counter:
  forest INFO and DEBUG at one or two occurrences, then flat DEBUG and TRACE at
  three or four. Keep `--threads` long-only and route tracing independently of
  verbose and progress rendering, including commands with bespoke execution paths
  and corpus runs.
  
  Initialize display tracing once before repository discovery, buffer it outside
  `prodash`, and emit it after command and progress teardown. Format forest and
  flat output with ANSI, then adapt the final stderr write so terminals get color
  while pipes and `NO_COLOR` remain plain.
  
  Corpus worker subscribers share that sink while retaining their complete
  JSON trace storage, and flat modes include completed spans. `gix tix` remains
  delegated to Tix so it installs only one subscriber.
- <csr-id-ca0473a7748e40b3feef4a077dca6b53f7c41f91/> add and remove linked worktrees from the CLI
  <!-- Byron -->
  
  rubber-stamp mostly for gitoxide-core changes, did look at it though
  just to be sure.
  
  Journey test was present but removed.
  
  <!-- agent -->
  The CLI previously exposed only worktree listing, despite `gix` supporting
  both checkout and safe removal. Add `worktree add` (alias `create`) and
  `worktree remove` using those APIs.
  
  Creation follows Git's local-branch and detached-revision selection, including
  destination-basename defaults and `-b`. It retains new branches after setup
  failures, uses the existing listing format on success, and reports incomplete
  checkouts. Removal maps repeated `-f` flags to dirty-worktree and lock overrides
  while retaining branches.
  
  Both commands use the existing repository, configuration, and progress options.
  Unsupported JSON output is rejected before mutation.

### Changed (BREAKING)

- <csr-id-624ddcda33c22a48c9641543041b050e9b0d8f87/> remove full-screen progress mode
  <!-- agent -->
  Full-screen progress required separate command and UI threads plus channel
  coordination, while line progress already covers long-running commands and works
  with independently buffered tracing.
  
  Remove `--progress`, `--progress-keep-open`, and the `prodash-render-tui` feature. Keep synchronous line progress as the only progress UI.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 17 commits contributed to the release over the course of 13 calendar days.
- 13 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.0, gix-date v0.18.0, gix-actor v0.44.0, gix-trace v0.3.0, gix-validate v0.13.0, gix-path v0.14.0, gix-utils v0.5.0, gix-quote v0.10.0, gix-command v0.12.0, gix-hash v0.28.0, gix-hashtable v0.18.0, gix-parallel v0.50.0, gix-fs v0.24.0, gix-tempfile v26.0.0, gix-object v0.66.0, gix-glob v0.29.0, gix-attributes v0.37.0, gix-packetline v0.24.0, gix-filter v0.36.0, gix-chunk v0.10.0, gix-commitgraph v0.41.0, gix-revwalk v0.37.0, gix-traverse v0.63.0, gix-worktree-stream v0.38.0, gix-archive v0.38.0, gix-bitmap v0.6.0, gix-lock v26.0.0, gix-index v0.57.0, gix-config-value v0.21.0, gix-pathspec v0.22.0, gix-ignore v0.24.0, gix-worktree v0.58.0, gix-imara-diff v0.3.1, gix-diff v0.69.0, gix-blame v0.19.0, gix-ref v0.69.0, gix-sec v0.16.0, gix-config v0.62.0, gix-prompt v0.19.0, gix-url v0.40.0, gix-credentials v0.42.0, gix-discover v0.57.0, gix-dir v0.31.0, gix-mailmap v0.36.0, gix-revision v0.51.0, gix-merge v0.22.0, gix-negotiate v0.37.0, gix-note v0.3.0, gix-zlib v0.3.0, gix-pack v0.76.0, gix-odb v0.86.0, gix-refspec v0.47.0, gix-shallow v0.15.0, gix-transport v0.61.0, gix-protocol v0.67.0, gix-status v0.36.0, gix-submodule v0.36.0, gix-worktree-state v0.36.0, gix v0.89.0, gix-fsck v0.27.0, gitoxide-core v0.63.0, gix-tix v0.5.0, gitoxide v0.60.0, safety bump 62 crates ([`0115e90`](https://github.com/GitoxideLabs/gitoxide/commit/0115e902fd0334b8f370766993eb17babed3157f))
  - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
  - Merge pull request #2938 from GitoxideLabs/tix-improvements ([`ac6e89f`](https://github.com/GitoxideLabs/gitoxide/commit/ac6e89ffb1eb7ab1b50aaf710107256f6fcd977b))
  - Address review feedback about gix tix dispatch ([`5295bfb`](https://github.com/GitoxideLabs/gitoxide/commit/5295bfb575f12ca99277e501bfcf4834a3450613))
  - Address review feedback about root Tix network feature wiring ([`56d47f3`](https://github.com/GitoxideLabs/gitoxide/commit/56d47f3157b05a407f29ce7cb286b6a1e35c163b))
  - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
  - Remove full-screen progress mode ([`624ddcd`](https://github.com/GitoxideLabs/gitoxide/commit/624ddcda33c22a48c9641543041b050e9b0d8f87))
  - Make trace output independently configurable ([`bc278ac`](https://github.com/GitoxideLabs/gitoxide/commit/bc278ac4b79c31ff9e9cc2a3f0c3d435ca7c7f8d))
  - Merge pull request #3035 from any-victor/fix/clone-ignores-caller-repository-env ([`d2d078f`](https://github.com/GitoxideLabs/gitoxide/commit/d2d078f25ec2abb18a28a0e635c21d082ef5a830))
  - Review ([`95043b9`](https://github.com/GitoxideLabs/gitoxide/commit/95043b9895c760fb2cf1f9d7bd8c0d7f84c3e619))
  - Merge pull request #3022 from GitoxideLabs/release-testtools ([`f819565`](https://github.com/GitoxideLabs/gitoxide/commit/f819565c2c4c56619c4888acef6cf3b8144cbccb))
  - Merge pull request #3029 from cruessler/fix-minor-sha-256-issues-in-cli ([`6c12f2d`](https://github.com/GitoxideLabs/gitoxide/commit/6c12f2dc5cb0a17c7c2050c03e7f1fde7f02b469))
  - Add SHA-256 to `AsHashKind` ([`106db87`](https://github.com/GitoxideLabs/gitoxide/commit/106db87d37e7293fdf0240cf80fb774538fe285e))
  - Don't hardcode hash in `gix free index` ([`a8e3143`](https://github.com/GitoxideLabs/gitoxide/commit/a8e314336ff29f4f8e38aa1b9688dabcf04d3dd5))
  - Merge pull request #3025 from GitoxideLabs/worktree-create ([`acf906f`](https://github.com/GitoxideLabs/gitoxide/commit/acf906f8fcdb8e8d143570737786543dc5a204a0))
  - Add and remove linked worktrees from the CLI ([`ca0473a`](https://github.com/GitoxideLabs/gitoxide/commit/ca0473a7748e40b3feef4a077dca6b53f7c41f91))
  - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))

</details>

## 0.59.0 (2026-09-25)

### New Features

- <csr-id-d9306539c890c8c57b65bbc33604c50d2bfc67cf/> add a `gix editor` command
  <!-- agent -->
  Provide a small plumbing command that opens zero or more paths with
  `Repository::editor()`'s Git-compatible selection. Prepare editor definitions with
  command-aware argument splitting so PATH-resolved programs run directly while
  actual shell syntax still uses the shell, reject explicitly empty settings, and
  honor ':' as the no-op editor.

### Changed (BREAKING)

- <csr-id-4b42e0ce80ae934cae4f102f44c392581758608f/> raise MSRV to Rust 1.88
  <!-- agent -->
  The newly published `dua-core` 3.3 release used by linked-worktree removal
  requires Rust 1.88, so raise every workspace crate and the advertised badge
  together.
  
  Keep the MSRV checks buildable by selecting the latest `sysinfo` and `rusqlite`
  release lines that support Rust 1.88.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 9 commits contributed to the release over the course of 31 calendar days.
- 32 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
  - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
  - Merge pull request #3008 from srijs/sha1dc-integration ([`d6181a2`](https://github.com/GitoxideLabs/gitoxide/commit/d6181a29f9156d637d82827618458d0413c1a8cb))
  - Merge pull request #2989 from GitoxideLabs/error-conversion-review ([`4b9ff51`](https://github.com/GitoxideLabs/gitoxide/commit/4b9ff511a49f7963e97a669ca82c6f6e833d8ea2))
  - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
  - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
  - Merge pull request #2942 from GitoxideLabs/error-conversion-review ([`a1d5a55`](https://github.com/GitoxideLabs/gitoxide/commit/a1d5a5520d597bdc33c1cf84c1d061b5bc1e382e))
  - Add a `gix editor` command ([`d930653`](https://github.com/GitoxideLabs/gitoxide/commit/d9306539c890c8c57b65bbc33604c50d2bfc67cf))
  - Merge pull request #2940 from GitoxideLabs/vendor-bisync ([`dda600d`](https://github.com/GitoxideLabs/gitoxide/commit/dda600d7ee29a6bda4cf1047d0d1782e76b16f98))

</details>

## 0.58.0 (2026-08-24)

### New Features

- <csr-id-608830f508fec4cf2a5f37133b4944c25ec1fad1/> delegate tix commands through gix-tix
  Embed the complete gix-tix Clap platform in the plumbing command instead of maintaining a second parser and dispatcher. This keeps standalone and plumbing invocations aligned, adds worktree-tip parity, and makes amend and spill available through gix tix without duplicating their execution logic.
- <csr-id-ff25141fa396d5179425cc6058e57e1be39ca780/> show other worktree checkouts in tix
  Decorate commits checked out by other main or linked worktrees with light-blue name@ labels, replacing the ordinary branch label. Keep the checkout from which tix was opened represented by the graph @ marker and its ordinary local reference, and use directory basenames for other detached worktrees. Keep other-worktree labels visible on the selected row when references are hidden.
  
  Add -w/--worktrees to include every successfully resolved worktree HEAD, including the current checkout, alongside implicit or explicit traversal tips without weakening hidden-revision exclusions. Discover worktrees from their private Git metadata so stale checkout directories remain useful, while malformed, unborn, or inaccessible entries are logged and skipped.
  
  Watch linked HEAD and worktree membership changes so decorations and optional tips stay current, while ignoring unrelated linked indexes, logs, and metadata. Document the behavior and cover current, attached, detached, stale, malformed, hidden-reference, CLI, and watcher cases.

### Changed (BREAKING)

- <csr-id-ef93be6d4e938b500cfb22cd468a19187ab9d8da/> remove gix tix screen selection
  Remove the plumbing CLI screen-mode option now that the interactive history always owns the alternate screen. The automatic and half-screen modes depended on inline rendering behavior that is being retired from gix-tix.
  
  Stop accepting --screen for gix tix and its aliases, and remove the mode validation and translation into gix-tix. Construct the reduced gix-tix options directly from the remaining quit and hidden-revision arguments. Keep a command-line regression assertion so the removed option cannot silently return as an ignored argument.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 7 commits contributed to the release over the course of 1 calendar day.
- 2 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-packetline v0.22.2, gix-worktree-stream v0.36.1, gix-archive v0.36.1, gix-diff v0.67.1, gix-blame v0.17.1, gix-dir v0.29.1, gix-mailmap v0.34.1, gix-revision v0.49.1, gix-merge v0.20.1, gix-negotiate v0.35.1, gix-note v0.1.1, gix-pack v0.74.2, gix-macros v0.1.6, gix-refspec v0.45.1, gix-transport v0.59.1, gix-protocol v0.65.1, gix-status v0.34.1, gix-worktree-state v0.34.1, gix v0.87.1, gix-fsck v0.25.1, gitoxide-core v0.61.1, gix-tix v0.3.0, gitoxide v0.58.0, safety bump gitoxide v0.58.0 ([`3ebca8b`](https://github.com/GitoxideLabs/gitoxide/commit/3ebca8b66017ab2dd02a38f75f78f485bee1ded8))
  - Merge pull request #2842 from GitoxideLabs/tix-improvements ([`fbebed7`](https://github.com/GitoxideLabs/gitoxide/commit/fbebed746e296be64d5607e0a75b5a4e08cf4413))
  - Adapt plumbing to standard tix help flags ([`1d4d997`](https://github.com/GitoxideLabs/gitoxide/commit/1d4d9975ec92b9a14fae6b811da2c02ea528db8e))
  - Delegate tix commands through gix-tix ([`608830f`](https://github.com/GitoxideLabs/gitoxide/commit/608830f508fec4cf2a5f37133b4944c25ec1fad1))
  - Show other worktree checkouts in tix ([`ff25141`](https://github.com/GitoxideLabs/gitoxide/commit/ff25141fa396d5179425cc6058e57e1be39ca780))
  - Remove gix tix screen selection ([`ef93be6`](https://github.com/GitoxideLabs/gitoxide/commit/ef93be6d4e938b500cfb22cd468a19187ab9d8da))
  - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))

</details>

## 0.57.0 (2026-08-22)

### New Features

- <csr-id-b1174b69b60b478be24f0b81787e95ac08564e7e/> support cloning a single revision
  <!-- agent -->
  A full object ID passed through with_ref_name() produced an object-ID refspec
  mapping and panicked while clone assumed every mapping had a name. Branch and
  tag checkout also retained ordinary clone tracking semantics instead of offering
  a single-revision mode.
  
  Add PrepareFetch::with_revision() and gix clone --revision for full refs, HEAD,
  and full object IDs. Revision clones use a one source-only implicit refspec,
  detach HEAD to the fetched commit, create no ordinary refs, persist no fetch
  refspec, and disable tag following. Existing with_ref_name() and --ref behavior
  stays unchanged.
  
  This follows Git commit 337855629f59 (builtin/clone: teach git-clone(1) the
  --revision= option) and its t/t5621-clone-revision.sh behavior.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 30 calendar days.
- 30 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#1930](https://github.com/GitoxideLabs/gitoxide/issues/1930)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#1930](https://github.com/GitoxideLabs/gitoxide/issues/1930)**
  - Support cloning a single revision ([`b1174b6`](https://github.com/GitoxideLabs/gitoxide/commit/b1174b69b60b478be24f0b81787e95ac08564e7e))
- **Uncategorized**
  - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
  - Merge pull request #2910 from codeAnqiang-ma/fix/relative-date-month-rollover ([`566fea1`](https://github.com/GitoxideLabs/gitoxide/commit/566fea12b59005673dd4beede48df12d8884264e))
  - Adapt to changes in `gix-date` ([`613ff86`](https://github.com/GitoxideLabs/gitoxide/commit/613ff86cf77922aaab1f7885e014792458b81606))
  - Merge pull request #2824 from GitoxideLabs/fetch-revision ([`7a34c17`](https://github.com/GitoxideLabs/gitoxide/commit/7a34c175866222ba6ebeb108d2594369a97d5526))
  - Merge pull request #2879 from GitoxideLabs/remove-jwalk ([`b6492d3`](https://github.com/GitoxideLabs/gitoxide/commit/b6492d3a4b10d1c4b58988fac62171b65731b42d))
  - Replace jwalk with dua-core ([`c49dc45`](https://github.com/GitoxideLabs/gitoxide/commit/c49dc457b879eb9f9d6c844878d82e11c185babd))
  - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))

</details>

## 0.56.0 (2026-07-23)

### New Features

- <csr-id-21401427be1873e72dce7a802587571543e529e7/> add configurable terminal screen modes.
- <csr-id-fed0051609450abf49c6aeebdfa36e191f4292c7/> hide revision ancestry in tix
- <csr-id-31a94aa8e268fb9e3442ce786788624938fce275/> add tix to the gix CLI
- <csr-id-06a927c422c73be8a433ef71ea9696770bffd31c/> add explicit gix config show subcommand.
- <csr-id-bd595de3f204fa431d681bde20774463cb72fbc5/> add `gix config list` for showing the git-configuration files.
- <csr-id-87a5e89ef78751282919c3882064b7d01c94170d/> add the `gix config fmt` subcommand
  Expose the gix-config whitespace formatter as
  `gix config fmt [--in-place] [in-file] [out-file]`: with no in-file it
  formats the repository-local configuration, and with no out-file it writes
  to stdout. The repository is only opened when the repository-local
  configuration is needed, so formatting an explicit file works outside a
  repository too.
- <csr-id-9316ff8168a2d147e5791663168ae3ee2a78d395/> add `gix status --untracked` flag
  This allows to control how untracked files are folded, but also
  can completely turn dirwalking off.
- <csr-id-6767b308dab7cb02b0946ce94bb1e05a1aecd370/> add `gix free remote refs` to list and write remote refs.
  Add `gix free remote refs` to perform an upload-pack handshake, discover
  the remote reference advertisement, and print the refs without negotiating
  or receiving a pack.
  
  The command can also write the advertised refs into a standalone ref store:
  
  ```sh
  gix free remote refs --refs-directory out-refs <url>
  ```

### Bug Fixes

- <csr-id-5f244b32c44795062b0aa9e352404cbd1412a844/> preserve multiple remote URLs
  Configured remotes can have multiple remote.<name>.url or remote.<name>.pushUrl
  values, but `gix::Remote` kept only one value because lookup used the singular
  config accessor. That meant the most recent config value won, while Git exposes
  every effective URL and uses the first one for the singular get-url form.
  
  Add ordered URL storage to `gix::Remote` and expose `Remote::urls(Direction)` for
  all effective URLs. Keep Remote::url(Direction) as the singular compatibility
  API, now returning the first effective URL. Fetch URLs used as push fallbacks
  try pushInsteadOf first and then insteadOf, matching Git behavior.
  
  The regression fixture records Git 2.50.1 behavior with git remote get-url:
  without --all it prints the first configured URL, and with --all it prints all
  configured URLs in order for both fetch and push.

### Bug Fixes (BREAKING)

- <csr-id-b1851470ef477a3edfa3a96fa07c4472c2ae7962/> make exclude queries index-aware
  <!-- agent -->
  
  `gix exclude query` reported ignore matches for tracked files and directories
  containing tracked entries, unlike `git check-ignore`. Positional arguments were
  also interpreted as pathspecs while stdin supplied paths.
  
  Treat positional and stdin input uniformly as paths, normalize them relative
  to the repository, and suppress ignore matches for indexed files or directories
  containing indexed entries. This changes positional arguments from pathspecs
  to paths.
  
  Add journey coverage for tracked and untracked paths below ignored directories,
  stdin from a nested working directory, ignore-pattern display, and positional
  output order.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 27 commits contributed to the release.
- 31 days passed between releases.
- 10 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#2562](https://github.com/GitoxideLabs/gitoxide/issues/2562), [#2696](https://github.com/GitoxideLabs/gitoxide/issues/2696)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#2562](https://github.com/GitoxideLabs/gitoxide/issues/2562)**
  - Make exclude queries index-aware ([`b185147`](https://github.com/GitoxideLabs/gitoxide/commit/b1851470ef477a3edfa3a96fa07c4472c2ae7962))
- **[#2696](https://github.com/GitoxideLabs/gitoxide/issues/2696)**
  - Preserve multiple remote URLs ([`5f244b3`](https://github.com/GitoxideLabs/gitoxide/commit/5f244b32c44795062b0aa9e352404cbd1412a844))
- **Uncategorized**
  - Release gix-actor v0.41.2, gix-features v0.49.0, gix-hash v0.26.0, gix-hashtable v0.16.0, gix-object v0.63.0, gix-glob v0.27.0, gix-attributes v0.34.0, gix-packetline v0.22.0, gix-filter v0.33.0, gix-fs v0.22.0, gix-chunk v0.7.3, gix-commitgraph v0.38.0, gix-revwalk v0.34.0, gix-traverse v0.60.0, gix-worktree-stream v0.35.0, gix-archive v0.35.0, gix-bitmap v0.3.3, gix-tempfile v24.0.0, gix-lock v24.0.0, gix-index v0.54.0, gix-pathspec v0.19.0, gix-ignore v0.22.0, gix-worktree v0.55.0, gix-imara-diff v0.2.4, gix-diff v0.66.0, gix-blame v0.16.0, gix-ref v0.66.0, gix-config v0.59.0, gix-discover v0.54.0, gix-dir v0.28.0, gix-mailmap v0.33.2, gix-revision v0.48.0, gix-merge v0.19.0, gix-negotiate v0.34.0, gix-zlib v0.1.0, gix-pack v0.73.0, gix-odb v0.83.0, gix-refspec v0.44.0, gix-shallow v0.13.0, gix-transport v0.58.0, gix-protocol v0.64.0, gix-status v0.33.0, gix-submodule v0.33.0, gix-worktree-state v0.33.0, gix v0.86.0, gix-fsck v0.24.0, gitoxide-core v0.60.0, gix-tix v0.1.0, gitoxide v0.56.0, safety bump 40 crates ([`842bc44`](https://github.com/GitoxideLabs/gitoxide/commit/842bc447e3aeacf5d9d36f7f8a01068eda4b7999))
  - Release gix-trace v0.1.21, gix-validate v0.11.3, gix-path v0.12.3, gix-utils v0.3.5, gix-config-value v0.19.0, gix-prompt v0.16.0, gix-sec v0.14.2, gix-url v0.37.0, gix-credentials v0.39.0, safety bump 18 crates ([`f0ec710`](https://github.com/GitoxideLabs/gitoxide/commit/f0ec71076aa1cef3181b77946ee556a89c651b8e))
  - Add configurable terminal screen modes. ([`2140142`](https://github.com/GitoxideLabs/gitoxide/commit/21401427be1873e72dce7a802587571543e529e7))
  - Merge pull request #2809 from GitoxideLabs/gix-tix-mvp ([`443b401`](https://github.com/GitoxideLabs/gitoxide/commit/443b401730e91503666192f502556f334049fbc0))
  - Hide revision ancestry in tix ([`fed0051`](https://github.com/GitoxideLabs/gitoxide/commit/fed0051609450abf49c6aeebdfa36e191f4292c7))
  - Add tix to the gix CLI ([`31a94aa`](https://github.com/GitoxideLabs/gitoxide/commit/31a94aa8e268fb9e3442ce786788624938fce275))
  - Merge pull request #2735 from GitoxideLabs/better-exclude-handling ([`02cb162`](https://github.com/GitoxideLabs/gitoxide/commit/02cb162e31fb7fed5f93d29c7447f68f690442df))
  - Merge pull request #2636 from ameyypawar/gix-config-formatter ([`cfd3899`](https://github.com/GitoxideLabs/gitoxide/commit/cfd389911b33a80576519a228e60d51a789df857))
  - Add explicit gix config show subcommand. ([`06a927c`](https://github.com/GitoxideLabs/gitoxide/commit/06a927c422c73be8a433ef71ea9696770bffd31c))
  - Add `gix config list` for showing the git-configuration files. ([`bd595de`](https://github.com/GitoxideLabs/gitoxide/commit/bd595de3f204fa431d681bde20774463cb72fbc5))
  - Add the `gix config fmt` subcommand ([`87a5e89`](https://github.com/GitoxideLabs/gitoxide/commit/87a5e89ef78751282919c3882064b7d01c94170d))
  - Merge pull request #2721 from GitoxideLabs/remove-kstring ([`e70732a`](https://github.com/GitoxideLabs/gitoxide/commit/e70732a7cad4b5dca4890d394908c858ab406906))
  - Adapt to changes in `gix-attributes` ([`e11d7a2`](https://github.com/GitoxideLabs/gitoxide/commit/e11d7a2c734882e4ddb74c82ce5fe50b46265467))
  - Merge pull request #2722 from GitoxideLabs/reasons ([`c16b5a1`](https://github.com/GitoxideLabs/gitoxide/commit/c16b5a1892704b7c72a253bdd74a6848dd61032a))
  - Replace lint allowances with expectations ([`43ff87a`](https://github.com/GitoxideLabs/gitoxide/commit/43ff87a73897b70313e3a58e7de82231be5b59ad))
  - Merge pull request #2667 from GitoxideLabs/lifetime-free-config-parser ([`55b5158`](https://github.com/GitoxideLabs/gitoxide/commit/55b51580c2b018f9f35b4b865fe86a28a5c0ff84))
  - Adapt to changes in `gix-config` ([`376e946`](https://github.com/GitoxideLabs/gitoxide/commit/376e94627bf694d78314e044ece058fe09a54dec))
  - Merge pull request #2698 from GitoxideLabs/multi-remote-url ([`7056d62`](https://github.com/GitoxideLabs/gitoxide/commit/7056d62dd3fda005f55a1f59ce36179000749013))
  - Merge pull request #2707 from ameyypawar/fix/2703-inflate-error ([`6d95da6`](https://github.com/GitoxideLabs/gitoxide/commit/6d95da6e7082e19a03123ad765b3d5f117731621))
  - Adapt to changes in `gix-features`, use `gix-zlib` accordingly. ([`9c2977a`](https://github.com/GitoxideLabs/gitoxide/commit/9c2977a3b6d540690a1a263a037c8d54c316a020))
  - Merge pull request #2547 from special-bread/windows-status-performance ([`3e43c42`](https://github.com/GitoxideLabs/gitoxide/commit/3e43c42e96502d7fb0998ba05efcdf67d07eb957))
  - Add `gix status --untracked` flag ([`9316ff8`](https://github.com/GitoxideLabs/gitoxide/commit/9316ff8168a2d147e5791663168ae3ee2a78d395))
  - Merge pull request #2659 from willstott101/fix-ref-transaction-toctou ([`43dd683`](https://github.com/GitoxideLabs/gitoxide/commit/43dd6839d7d63c396c7c296244a5d87a92f09804))
  - Add `gix free remote refs` to list and write remote refs. ([`6767b30`](https://github.com/GitoxideLabs/gitoxide/commit/6767b308dab7cb02b0946ce94bb1e05a1aecd370))
  - Merge pull request #2646 from GitoxideLabs/report ([`1b1541e`](https://github.com/GitoxideLabs/gitoxide/commit/1b1541ed7a457afd48385c1ee39113949a9f5263))

</details>

## 0.55.0 (2026-06-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 27 calendar days.
- 27 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.15.5, gix-hashtable v0.15.2, gix-object v0.62.0, gix-attributes v0.33.2, gix-filter v0.32.0, gix-revwalk v0.33.0, gix-traverse v0.59.0, gix-worktree-stream v0.34.0, gix-archive v0.34.0, gix-tempfile v23.0.2, gix-index v0.53.0, gix-worktree v0.54.0, gix-imara-diff v0.2.3, gix-diff v0.65.0, gix-blame v0.15.0, gix-ref v0.65.0, gix-config v0.58.0, gix-discover v0.53.0, gix-dir v0.27.0, gix-revision v0.47.0, gix-merge v0.18.0, gix-negotiate v0.33.0, gix-pack v0.72.0, gix-odb v0.82.0, gix-refspec v0.43.0, gix-transport v0.57.2, gix-protocol v0.63.0, gix-status v0.32.0, gix-submodule v0.32.0, gix-worktree-state v0.32.0, gix v0.85.0, gix-fsck v0.23.0, gitoxide-core v0.59.0, gitoxide v0.55.0, safety bump 28 crates ([`6428edc`](https://github.com/GitoxideLabs/gitoxide/commit/6428edc82fc8a16d5ef34ca2d49aa6fdff3645fe))
  - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))

</details>

## 0.54.0 (2026-05-26)

### New Features

- <csr-id-add4cf4c0d2ce0f331a619ffd31f709a3b9f0bff/> `gix dirwalk` as a way to run gix-dir directly
  This is mostly for testing walks specifically, without needing them as part
  of `gix clean` or `gix status`.
- <csr-id-6ab327e54a71ca5e4d555bfedfffd81359d8b529/> add `gix merge tree --message ''` to allow creating commits
  This is useful for controlling cherry-picks precisely.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 17 commits contributed to the release over the course of 28 calendar days.
- 28 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.4, gix-date v0.15.4, gix-actor v0.41.1, gix-trace v0.1.20, gix-validate v0.11.2, gix-path v0.12.1, gix-utils v0.3.3, gix-features v0.48.1, gix-hash v0.25.1, gix-hashtable v0.15.1, gix-object v0.61.0, gix-glob v0.26.1, gix-quote v0.7.2, gix-attributes v0.33.1, gix-command v0.9.1, gix-packetline v0.21.4, gix-filter v0.31.0, gix-fs v0.21.2, gix-chunk v0.7.2, gix-commitgraph v0.37.1, gix-revwalk v0.32.0, gix-traverse v0.58.0, gix-worktree-stream v0.33.0, gix-archive v0.33.0, gix-bitmap v0.3.2, gix-tempfile v23.0.1, gix-lock v23.0.1, gix-index v0.52.0, gix-config-value v0.18.1, gix-pathspec v0.18.1, gix-ignore v0.21.1, gix-worktree v0.53.0, gix-imara-diff v0.2.2, gix-diff v0.64.0, gix-blame v0.14.0, gix-ref v0.64.0, gix-sec v0.14.1, gix-config v0.57.0, gix-prompt v0.15.1, gix-url v0.36.1, gix-credentials v0.38.1, gix-discover v0.52.0, gix-dir v0.26.0, gix-mailmap v0.33.1, gix-revision v0.46.0, gix-merge v0.17.0, gix-negotiate v0.32.0, gix-pack v0.71.0, gix-odb v0.81.0, gix-refspec v0.42.0, gix-shallow v0.12.1, gix-transport v0.57.1, gix-protocol v0.62.0, gix-status v0.31.0, gix-submodule v0.31.0, gix-worktree-state v0.31.0, gix v0.84.0, gix-fsck v0.22.0, gitoxide-core v0.58.0, gitoxide v0.54.0, safety bump 27 crates ([`10c58bb`](https://github.com/GitoxideLabs/gitoxide/commit/10c58bb56597d9335611da121aac21f9b09b6e5b))
  - Merge pull request #2590 from GitoxideLabs/independent-testtools ([`575113d`](https://github.com/GitoxideLabs/gitoxide/commit/575113dfb10b3ba12eb57f57a81b241e773968bd))
  - Adapt to changes in `gix-testtools` ([`ce9e6bd`](https://github.com/GitoxideLabs/gitoxide/commit/ce9e6bded2cb47cc9b995f8882d98c09ba5c2c8b))
  - Merge pull request #2584 from GitoxideLabs/improvements ([`1322a36`](https://github.com/GitoxideLabs/gitoxide/commit/1322a36599d252a822dbc4548763a2db75816075))
  - `gix dirwalk` as a way to run gix-dir directly ([`add4cf4`](https://github.com/GitoxideLabs/gitoxide/commit/add4cf4c0d2ce0f331a619ffd31f709a3b9f0bff))
  - Merge pull request #2573 from cruessler/run-gix-traverse-tests-with-sha-256 ([`278d7ec`](https://github.com/GitoxideLabs/gitoxide/commit/278d7ec395124b3ce00db6f3e029265bfec8ccd1))
  - Cleanup `sha1` feature in `gix` to just set what's needed. ([`16a6fc4`](https://github.com/GitoxideLabs/gitoxide/commit/16a6fc44cf331464ee42c78a9c9ccb07aaf05aa1))
  - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
  - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
  - Remove rust_2018_idioms lint declarations ([`e10d5f6`](https://github.com/GitoxideLabs/gitoxide/commit/e10d5f662df2ee05f973a3167ad215a330ee74e1))
  - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
  - Merge pull request #2558 from GitoxideLabs/improvements ([`fd36eb8`](https://github.com/GitoxideLabs/gitoxide/commit/fd36eb810166ce60065909910d5db3f6ae379765))
  - Address auto-review ([`1b1d496`](https://github.com/GitoxideLabs/gitoxide/commit/1b1d496607ed94dd290cd10290098a326f78c0a6))
  - Add commented out notes on lints to activate ([`3bbae6f`](https://github.com/GitoxideLabs/gitoxide/commit/3bbae6f6515b45c73871ad0a5fa1ecfebeb80ad7))
  - Merge pull request #2557 from GitoxideLabs/cherry-pick ([`0771cb2`](https://github.com/GitoxideLabs/gitoxide/commit/0771cb2fc6e1b08a2d8e14b9f1226f555ed59b79))
  - Add `gix merge tree --message ''` to allow creating commits ([`6ab327e`](https://github.com/GitoxideLabs/gitoxide/commit/6ab327e54a71ca5e4d555bfedfffd81359d8b529))
  - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))

</details>

## 0.53.0 (2026-04-28)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 2 calendar days.
- 4 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
  - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))

</details>

## 0.52.1 (2026-04-24)

### New Features (BREAKING)

- <csr-id-8094f5dcd4f24f4d54f7fbe7f716f80f2974b586/> Use `imara-diff-v2` with git sliders processing
  The slider post-processing imrpoves the diff quality for about 8% slower diffs.
  Line-counts, however, will be 50% faster to compute.

### New Features

- <csr-id-ab2016f634d3138ec9312b541558bbb2da27f3c7/> add `gix free trust` to easily check the assigned trust level of any given path
  This is particularly useful on Windows, which makes it easy to probe existing paths
  with ownership that might be complex to reproduce otherwise.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 14 commits contributed to the release over the course of 32 calendar days.
- 33 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gitoxide v0.52.1 ([`a4bab3c`](https://github.com/GitoxideLabs/gitoxide/commit/a4bab3cc3275a8d645a906915ae59ede926e13b1))
  - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
  - Merge pull request #2510 from GitoxideLabs/folder-identity-on-windows ([`a96587c`](https://github.com/GitoxideLabs/gitoxide/commit/a96587c23b267f74065ac9bed2e50de69113e67f))
  - Add `gix free trust` to easily check the assigned trust level of any given path ([`ab2016f`](https://github.com/GitoxideLabs/gitoxide/commit/ab2016f634d3138ec9312b541558bbb2da27f3c7))
  - Merge pull request #2513 from GitoxideLabs/v2-diff ([`2a5db88`](https://github.com/GitoxideLabs/gitoxide/commit/2a5db88d0330b0d125de4b6f3819f17a7f76f4b8))
  - Thanks clippy ([`e4f380e`](https://github.com/GitoxideLabs/gitoxide/commit/e4f380eff3b0440002f7e9b64a14ddcfbe63192a))
  - Use `imara-diff-v2` with git sliders processing ([`8094f5d`](https://github.com/GitoxideLabs/gitoxide/commit/8094f5dcd4f24f4d54f7fbe7f716f80f2974b586))
  - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
  - Make `package.include` patterns more specific so they don't match ignored files ([`c2c917f`](https://github.com/GitoxideLabs/gitoxide/commit/c2c917fce56c40a9af0d06bd603b7d1d2e51474f))
  - Merge pull request #2506 from GitoxideLabs/vendor-imara-diff ([`8f091d1`](https://github.com/GitoxideLabs/gitoxide/commit/8f091d108cd75371be2ed9de6e81f785cda53d92))
  - Vendor `imara-diff` 0.1 and 0.2 ([`fd49295`](https://github.com/GitoxideLabs/gitoxide/commit/fd49295c5ed4a57bf5771e23c0f803435990ecfa))
  - Merge pull request #2494 from GitoxideLabs/improvements ([`50fb46f`](https://github.com/GitoxideLabs/gitoxide/commit/50fb46f17acba7d75d4ed6795b7b9cef484e34c6))
  - Adapt to changes in `gix-config`. ([`344218a`](https://github.com/GitoxideLabs/gitoxide/commit/344218a4946008f54bda2f1a6bbce15e84c1df3c))
  - Merge pull request #2480 from GitoxideLabs/report ([`98bae84`](https://github.com/GitoxideLabs/gitoxide/commit/98bae84fe534879899489c6f2c5e8cfcc863116d))

</details>

## 0.52.0 (2026-03-22)

### New Features

- <csr-id-383291689c659a2cc0bee7687f5a9b9f7a3659a4/> add `sha1` and `sha256` features to `gix`.
  This way one can control which hashes are compiled in exactly,
  while having reasonable defaults automatically.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release.
- 28 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.1, gix-date v0.15.1, gix-path v0.11.2, gix-features v0.46.2, gix-hash v0.23.0, gix-hashtable v0.13.0, gix-object v0.58.0, gix-packetline v0.21.2, gix-filter v0.28.0, gix-fs v0.19.2, gix-commitgraph v0.35.0, gix-revwalk v0.29.0, gix-traverse v0.55.0, gix-worktree-stream v0.30.0, gix-archive v0.30.0, gix-tempfile v21.0.2, gix-lock v21.0.2, gix-index v0.49.0, gix-pathspec v0.16.1, gix-ignore v0.19.1, gix-worktree v0.50.0, gix-diff v0.61.0, gix-blame v0.11.0, gix-ref v0.61.0, gix-sec v0.13.2, gix-config v0.54.0, gix-prompt v0.14.1, gix-credentials v0.37.1, gix-discover v0.49.0, gix-dir v0.23.0, gix-revision v0.43.0, gix-merge v0.14.0, gix-negotiate v0.29.0, gix-pack v0.68.0, gix-odb v0.78.0, gix-refspec v0.39.0, gix-shallow v0.10.0, gix-transport v0.55.1, gix-protocol v0.59.0, gix-status v0.28.0, gix-submodule v0.28.0, gix-worktree-state v0.28.0, gix v0.81.0, gix-fsck v0.19.0, gitoxide-core v0.55.0, gitoxide v0.52.0, safety bump 31 crates ([`c389a2c`](https://github.com/GitoxideLabs/gitoxide/commit/c389a2ccb32b36c1178a1352a2bb3229aef3b016))
  - Merge pull request #2441 from cruessler/remove-sha-1-from-default-features ([`e8bf096`](https://github.com/GitoxideLabs/gitoxide/commit/e8bf096c07205a41089a697a9726f075d3515643))
  - Add `sha1` and `sha256` features to `gix`. ([`3832916`](https://github.com/GitoxideLabs/gitoxide/commit/383291689c659a2cc0bee7687f5a9b9f7a3659a4))
  - Merge pull request #2445 from GitoxideLabs/improvements ([`6a7287c`](https://github.com/GitoxideLabs/gitoxide/commit/6a7287c9247120167e49154463f7e86c25100649))
  - Add `cargo machete` CI job including exclusions ([`abd0724`](https://github.com/GitoxideLabs/gitoxide/commit/abd072444ff076557aa7e4c5b76ad7c47d488a4a))
  - Merge pull request #2442 from GitoxideLabs/report ([`f7277f3`](https://github.com/GitoxideLabs/gitoxide/commit/f7277f3c9e3e5130edb714ff5bd3db06b7f589b3))

</details>

## 0.51.0 (2026-02-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 31 calendar days.
- 31 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.0, gix-date v0.15.0, gix-actor v0.40.0, gix-object v0.57.0, gix-quote v0.7.0, gix-attributes v0.31.0, gix-command v0.8.0, gix-filter v0.27.0, gix-chunk v0.7.0, gix-commitgraph v0.34.0, gix-revwalk v0.28.0, gix-traverse v0.54.0, gix-worktree-stream v0.29.0, gix-archive v0.29.0, gix-bitmap v0.3.0, gix-index v0.48.0, gix-pathspec v0.16.0, gix-worktree v0.49.0, gix-diff v0.60.0, gix-blame v0.10.0, gix-ref v0.60.0, gix-config v0.53.0, gix-prompt v0.14.0, gix-url v0.35.2, gix-credentials v0.37.0, gix-discover v0.48.0, gix-dir v0.22.0, gix-mailmap v0.32.0, gix-revision v0.42.0, gix-merge v0.13.0, gix-negotiate v0.28.0, gix-pack v0.67.0, gix-odb v0.77.0, gix-refspec v0.38.0, gix-shallow v0.9.0, gix-transport v0.55.0, gix-protocol v0.58.0, gix-status v0.27.0, gix-submodule v0.27.0, gix-worktree-state v0.27.0, gix v0.80.0, gix-fsck v0.18.0, gitoxide-core v0.54.0, gitoxide v0.51.0, safety bump 42 crates ([`ecf90fc`](https://github.com/GitoxideLabs/gitoxide/commit/ecf90fccb9d43bff320c17f46fdc3f5832533a52))
  - Merge branch 'release' ([`9327b73`](https://github.com/GitoxideLabs/gitoxide/commit/9327b73785227f1322a327cb48fbb0800e1286ae))
  - Release gix-error v0.1.0, gix-date v0.14.0, gix-actor v0.39.0, gix-trace v0.1.18, gix-path v0.11.1, gix-features v0.46.1, gix-hash v0.22.1, gix-object v0.56.0, gix-quote v0.6.2, gix-attributes v0.30.1, gix-command v0.7.1, gix-packetline v0.21.1, gix-filter v0.26.0, gix-fs v0.19.1, gix-chunk v0.6.0, gix-commitgraph v0.33.0, gix-revwalk v0.27.0, gix-traverse v0.53.0, gix-worktree-stream v0.28.0, gix-archive v0.28.0, gix-bitmap v0.2.16, gix-tempfile v21.0.1, gix-lock v21.0.1, gix-index v0.47.0, gix-config-value v0.17.1, gix-pathspec v0.15.1, gix-worktree v0.48.0, gix-diff v0.59.0, gix-blame v0.9.0, gix-ref v0.59.0, gix-sec v0.13.1, gix-config v0.52.0, gix-prompt v0.13.1, gix-url v0.35.1, gix-credentials v0.36.0, gix-discover v0.47.0, gix-dir v0.21.0, gix-mailmap v0.31.0, gix-revision v0.41.0, gix-merge v0.12.0, gix-negotiate v0.27.0, gix-pack v0.66.0, gix-odb v0.76.0, gix-refspec v0.37.0, gix-shallow v0.8.1, gix-transport v0.54.0, gix-protocol v0.57.0, gix-status v0.26.0, gix-submodule v0.26.0, gix-worktree-state v0.26.0, gix v0.79.0, safety bump 35 crates ([`d66ac10`](https://github.com/GitoxideLabs/gitoxide/commit/d66ac1057a5b7bfb608d4e6be585c69fb692bfee))
  - Merge pull request #2420 from cruessler/remove-imara-diff-0-1-in-gix-blame ([`28fbeb8`](https://github.com/GitoxideLabs/gitoxide/commit/28fbeb86a9b82b3eb69b6e3618e286b2507204eb))
  - Remove feature flag blame-experimental ([`44e447b`](https://github.com/GitoxideLabs/gitoxide/commit/44e447b963134a7f1aeecf9b18018ed2ea27df6d))
  - Merge pull request #2393 from GitoxideLabs/report ([`f7d0975`](https://github.com/GitoxideLabs/gitoxide/commit/f7d09758d245aaa89409e39bb6ba1ed6b7118ea5))

</details>

## 0.50.0 (2026-01-22)

### Chore (BREAKING)

- <csr-id-2358b1d250d3d2348210fb61dcb95ebe7aa6314b/> Upgrade `prodash` and `crosstermion` to the latest version.
  This will fix the `cargo deny` issue as it brings in a newer `lru` crate.

### New Features

- <csr-id-3774a03f35f9748605aa9f0ddaac5a57fd9850a2/> add `http-client-curl-rust-openssl` feature, and make it the default.
  Previously it was implicit, now it's explicit so it can be turned off.
  When that's the case, the `openssl` dependencies will be removed.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 12 commits contributed to the release over the course of 21 calendar days.
- 22 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.0.0, gix-date v0.13.0, gix-actor v0.38.0, gix-validate v0.11.0, gix-path v0.11.0, gix-features v0.46.0, gix-hash v0.22.0, gix-hashtable v0.12.0, gix-object v0.55.0, gix-glob v0.24.0, gix-attributes v0.30.0, gix-command v0.7.0, gix-packetline v0.21.0, gix-filter v0.25.0, gix-fs v0.19.0, gix-chunk v0.5.0, gix-commitgraph v0.32.0, gix-revwalk v0.26.0, gix-traverse v0.52.0, gix-worktree-stream v0.27.0, gix-archive v0.27.0, gix-tempfile v21.0.0, gix-lock v21.0.0, gix-index v0.46.0, gix-config-value v0.17.0, gix-pathspec v0.15.0, gix-ignore v0.19.0, gix-worktree v0.47.0, gix-diff v0.58.0, gix-blame v0.8.0, gix-ref v0.58.0, gix-sec v0.13.0, gix-config v0.51.0, gix-prompt v0.13.0, gix-url v0.35.0, gix-credentials v0.35.0, gix-discover v0.46.0, gix-dir v0.20.0, gix-mailmap v0.30.0, gix-revision v0.40.0, gix-merge v0.11.0, gix-negotiate v0.26.0, gix-pack v0.65.0, gix-odb v0.75.0, gix-refspec v0.36.0, gix-shallow v0.8.0, gix-transport v0.53.0, gix-protocol v0.56.0, gix-status v0.25.0, gix-submodule v0.25.0, gix-worktree-state v0.25.0, gix v0.78.0, gix-fsck v0.17.0, gitoxide-core v0.53.0, gitoxide v0.50.0, safety bump 50 crates ([`562e684`](https://github.com/GitoxideLabs/gitoxide/commit/562e684319fa649db6a96c0a22d64bbe3c11e9e6))
  - Merge pull request #2374 from GitoxideLabs/gix-error ([`25233ce`](https://github.com/GitoxideLabs/gitoxide/commit/25233ced7f17e14842aa400cf007a0feb6127d89))
  - Turn `Exn::into_box()` to `Exn::into_inner()`. ([`939b8fc`](https://github.com/GitoxideLabs/gitoxide/commit/939b8fcbb2115eba77aca1be8527ad0d7f644c56))
  - Merge pull request #2352 from GitoxideLabs/gix-error ([`ae23762`](https://github.com/GitoxideLabs/gitoxide/commit/ae23762932ea0d78e91463185a304d778746a167))
  - Add `http-client-curl-rust-openssl` feature, and make it the default. ([`3774a03`](https://github.com/GitoxideLabs/gitoxide/commit/3774a03f35f9748605aa9f0ddaac5a57fd9850a2))
  - Adadpt `exn` to most pressing needs of `gitoxide` ([`abedade`](https://github.com/GitoxideLabs/gitoxide/commit/abedadec5463b57e78aa53e62d8c511b989ae9ca))
  - Create a basic `gix-error` crate to forward `exn` ([`35cf1ff`](https://github.com/GitoxideLabs/gitoxide/commit/35cf1ff837ea30a1366b20bde0d59baf9ab699be))
  - Merge pull request #2353 from GitoxideLabs/improvements ([`ee0a1f6`](https://github.com/GitoxideLabs/gitoxide/commit/ee0a1f6cc99831d7c5f96b657df6ebca5ee5638c))
  - Upgrade `prodash` and `crosstermion` to the latest version. ([`2358b1d`](https://github.com/GitoxideLabs/gitoxide/commit/2358b1d250d3d2348210fb61dcb95ebe7aa6314b))
  - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
  - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
  - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))

</details>

## 0.49.0 (2025-12-31)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 9 calendar days.
- 9 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
  - Bump the main crate to the next minor level ([`f7c2248`](https://github.com/GitoxideLabs/gitoxide/commit/f7c224899ffdb277822564184f0c3eddc7124f26))
  - Merge pull request #2307 from cruessler/add-feature-flag-to-binary ([`d602d7e`](https://github.com/GitoxideLabs/gitoxide/commit/d602d7e836071f1e9e3af44c33f01522dde5c417))
  - Refactor ([`d532bbf`](https://github.com/GitoxideLabs/gitoxide/commit/d532bbffab880ef253207a5e68a2ad39e7d1c9ac))
  - Add feature flag `blame-experimental` to `gix` ([`826effb`](https://github.com/GitoxideLabs/gitoxide/commit/826effb563e369f57b22a8418be2e9ea58fc4844))
  - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))

</details>

## 0.48.0 (2025-12-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 1 commit contributed to the release.
- 30 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))

</details>

## 0.47.0 (2025-11-22)

<csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/>

### Bug Fixes (BREAKING)

- <csr-id-f2c6a19f676f1de9e0b4e0ff77f4e26d817fac7e/> make `gix-packeline` features additive.
  For good measure, everything that was touched is marked as breaking change.

### New Features

- <csr-id-924cd62f7ef74f479114c80cee0db0ba5f522579/> allow credential fill with `gix credential fill` to run without a repo

### Commit Statistics

<csr-read-only-do-not-edit/>

- 10 commits contributed to the release.
- 31 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#2198](https://github.com/GitoxideLabs/gitoxide/issues/2198)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#2198](https://github.com/GitoxideLabs/gitoxide/issues/2198)**
  - Allow credential fill with `gix credential fill` to run without a repo ([`924cd62`](https://github.com/GitoxideLabs/gitoxide/commit/924cd62f7ef74f479114c80cee0db0ba5f522579))
- **Uncategorized**
  - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
  - Merge pull request #2254 from ralphmodales/credential-fill-fix ([`dba337d`](https://github.com/GitoxideLabs/gitoxide/commit/dba337da2430190b2151a2814fcccc09fd9f1d50))
  - Merge pull request #2204 from cruessler/improve-blame-ranges ([`663b41e`](https://github.com/GitoxideLabs/gitoxide/commit/663b41eb65f0fffbc8397e91ce5107382b08b441))
  - Merge pull request #2220 from djc/dedup-packetline ([`cbf7445`](https://github.com/GitoxideLabs/gitoxide/commit/cbf7445055357c7bac8b5946fb6e55b04b1a862a))
  - Make `gix-packeline` features additive. ([`f2c6a19`](https://github.com/GitoxideLabs/gitoxide/commit/f2c6a19f676f1de9e0b4e0ff77f4e26d817fac7e))
  - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
  - Release gix-date v0.10.7, gix-actor v0.35.6, gix-trace v0.1.15, gix-features v0.44.1, gix-hash v0.20.1, gix-object v0.51.1, gix-glob v0.22.1, gix-attributes v0.28.1, gix-packetline-blocking v0.19.3, gix-commitgraph v0.30.1, gix-archive v0.23.1, gix-tempfile v19.0.1, gix-index v0.42.1, gix-config-value v0.15.3, gix-ignore v0.17.1, gix-worktree v0.43.1, gix-diff v0.54.1, gix-ref v0.54.1, gix-sec v0.12.2, gix-config v0.47.1, gix-url v0.33.1, gix-credentials v0.31.1, gix-mailmap v0.27.4, gix-revision v0.36.1, gix-pack v0.61.1, gix-odb v0.71.1, gix-packetline v0.19.3, gix-transport v0.49.1, gix-protocol v0.52.1, gix-status v0.21.1, gix v0.74.1 ([`bdcce5f`](https://github.com/GitoxideLabs/gitoxide/commit/bdcce5f2c6723ebe489dbe936a4656859ce1c2a5))
  - Remove `doc_auto_cfg` feature to fix docs.rs documentation. ([`6f469a6`](https://github.com/GitoxideLabs/gitoxide/commit/6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b))
  - Merge pull request #2224 from GitoxideLabs/report ([`3313233`](https://github.com/GitoxideLabs/gitoxide/commit/3313233aa4e7009aed0ddf644f4271fd2a98e8d4))

</details>

## 0.46.0 (2025-10-22)

### Documentation

- <csr-id-6ab07f77289487ad26c49b6fab26119395bf64d1/> Note that the `small` feature doesn't include `clone`
  The `small` feature of the `gitoxide` crate does not directly or
  indirectly include the `gitoxide-core-blocking-client` feature, and
  therefore does not provide `gix clone`. Although it was documented
  to provide only local operations, this was ambiguous: in the sense
  of local and remote repository operations, cloning is arguably a
  remote operation even when one clones from a filesystem path, or
  file URL. But in the broader meaning of "local," this could mean
  merely that network transport is omitted and that local cloning is
  included.
  
  This adds a short explicit note that `small` does not include
  `gix clone`. This is a minimal fix for #2185 and it may make sense
  to improve the description further (unless `small` is to change).

### New Features

- <csr-id-aacc326a198e4c78dd546018eebaff192ff5223d/> add a `gix commit sign` prototype
- <csr-id-58b5060958e0090773bb15ffb3679d99704b70a0/> add first debug version of `gix branch list`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 24 commits contributed to the release over the course of 99 calendar days.
- 99 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
  - Merge pull request #2208 from cruessler/add-commit-signing ([`8699d0f`](https://github.com/GitoxideLabs/gitoxide/commit/8699d0f010458155f7f5e19c8018ea5e58079963))
  - Refactor ([`82f788e`](https://github.com/GitoxideLabs/gitoxide/commit/82f788e17aa3a4ad4bf21cc4293ec4f621f227ce))
  - Add a `gix commit sign` prototype ([`aacc326`](https://github.com/GitoxideLabs/gitoxide/commit/aacc326a198e4c78dd546018eebaff192ff5223d))
  - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
  - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
  - Adapt to change in `gix-blame` ([`a721e74`](https://github.com/GitoxideLabs/gitoxide/commit/a721e7428ba4fd145e14c4576d49b9ff5a1a9816))
  - Adapt to change in `gix-blame` ([`268dac4`](https://github.com/GitoxideLabs/gitoxide/commit/268dac4a18c8be8bacd89e1890d870253b0e1400))
  - Merge pull request #2186 from EliahKagan/small ([`f8be65f`](https://github.com/GitoxideLabs/gitoxide/commit/f8be65fef6ed3cdc55de531da30d0b4ad0ce0f14))
  - Note that the `small` feature doesn't include `clone` ([`6ab07f7`](https://github.com/GitoxideLabs/gitoxide/commit/6ab07f77289487ad26c49b6fab26119395bf64d1))
  - Merge pull request #2142 from cruessler/add-branch-list ([`525873f`](https://github.com/GitoxideLabs/gitoxide/commit/525873fafacf339dd781df0d812df02942c220fd))
  - Fix copy-paste doc comment ([`04650a7`](https://github.com/GitoxideLabs/gitoxide/commit/04650a7786f5fb0ea4f4454d08ca16ab6cef3b74))
  - Move --all to 'branch list' ([`6651548`](https://github.com/GitoxideLabs/gitoxide/commit/6651548f6e426d79f834c5948e3d7877784dfc77))
  - Refactor ([`e537611`](https://github.com/GitoxideLabs/gitoxide/commit/e53761181b05f90f59b33906da033096b57a50d8))
  - Add first debug version of `gix branch list` ([`58b5060`](https://github.com/GitoxideLabs/gitoxide/commit/58b5060958e0090773bb15ffb3679d99704b70a0))
  - Merge pull request #2137 from tmm1/typo-fix ([`ad1e2ed`](https://github.com/GitoxideLabs/gitoxide/commit/ad1e2ed9ce32d1e21acda2d89d26b11895401877))
  - Fix typo ([`2abbaab`](https://github.com/GitoxideLabs/gitoxide/commit/2abbaab7247bc02f324f4a872ef27301be4cd7fb))
  - Merge pull request #2106 from cruessler/add-open-with-environment-overrides ([`c149116`](https://github.com/GitoxideLabs/gitoxide/commit/c1491167c2ea8f8a34bb2e30f9e632f790f0fcd8))
  - Fail `gix tag list` when JSON is requested ([`fb2766b`](https://github.com/GitoxideLabs/gitoxide/commit/fb2766bd006ba1a276dfd5554cf0cdc3b7043ee7))
  - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
  - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
  - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
  - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
  - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))

</details>

## 0.45.0 (2025-07-15)

### New Features

- <csr-id-37d3bf24ac1a79302f3e97b97372e4ad381c45e2/> add first debug version of `gix tag list`
- <csr-id-ab52a49a555ab25e6cf632cb0b080eab72958a7d/> `gix revision list --long-hashes` for faster iteration.
  The performance of the short-hash generation was improved as well.
- <csr-id-c5bc49f2a02e9b28c2466ea4c7ae711d091ffc96/> support for `commitgraph list from..to` to exercise the new 'hide' capability.
- <csr-id-1df1ebb34dd3e2101d8a112dda66f6bac5261ea7/> Enable precious file parsing in `gix` CLI by default, allow overrides.
  That's pretty neat as one can now set `GIX_PARSE_PRECIOUS=0` in the environment
  to disable precious file parsing, good to see what difference it makes.
  
  It's also possible to do this wiht `gix -c gitoxide.parsePrecious=0`.
- <csr-id-36a6ffeea7bbde7fb3689ddf2a107e09a50e602c/> add support for multiple blame ranges like `gix blame -L <range> -L <other-range> ...`
  Update the blame subcommand to handle multiple line ranges. This allows specifying multiple `-L` options similar to the usage of git.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 27 commits contributed to the release over the course of 79 calendar days.
- 80 days passed between releases.
- 5 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
  - Merge pull request #2073 from cruessler/add-tag-list ([`c7af04d`](https://github.com/GitoxideLabs/gitoxide/commit/c7af04db9b6bb1204e0f4c436d1db8f48a491e86))
  - Refactor ([`750ae9b`](https://github.com/GitoxideLabs/gitoxide/commit/750ae9bc3cf72c1d9a358307e423523324eb25fb))
  - Add first debug version of `gix tag list` ([`37d3bf2`](https://github.com/GitoxideLabs/gitoxide/commit/37d3bf24ac1a79302f3e97b97372e4ad381c45e2))
  - Merge pull request #2070 from GitoxideLabs/dependabot/cargo/cargo-827bceb7eb ([`dab97f7`](https://github.com/GitoxideLabs/gitoxide/commit/dab97f7618f160421b6e31de8f3e2f3d11dc2ef2))
  - Bump the cargo group across 1 directory with 68 updates ([`a9a8ea1`](https://github.com/GitoxideLabs/gitoxide/commit/a9a8ea1472532dde03bce4e0afdfa82924af1f96))
  - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
  - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
  - Merge pull request #2041 from cruessler/add-blame-extraction ([`dd5f0a4`](https://github.com/GitoxideLabs/gitoxide/commit/dd5f0a4811bc738051f7af164b8d2815aaa23220))
  - Merge pull request #2051 from GitoxideLabs/improvements ([`f933f80`](https://github.com/GitoxideLabs/gitoxide/commit/f933f8065c218ee1e9ae7158b15c8a0917140803))
  - `gix revision list --long-hashes` for faster iteration. ([`ab52a49`](https://github.com/GitoxideLabs/gitoxide/commit/ab52a49a555ab25e6cf632cb0b080eab72958a7d))
  - Adapt to changes in `gix-blame` ([`4afc51d`](https://github.com/GitoxideLabs/gitoxide/commit/4afc51d4ba669ad3c4b26f1c4222442d1dab1695))
  - Merge pull request #2022 from cruessler/add-rename-tracking-to-blame ([`76eddf8`](https://github.com/GitoxideLabs/gitoxide/commit/76eddf86b91afc3535f7eb0d9004652823ccda36))
  - Refactor ([`3e5365c`](https://github.com/GitoxideLabs/gitoxide/commit/3e5365cb066895c787a22422964a2b9459f37ec3))
  - Merge pull request #2037 from GitoxideLabs/hide ([`92febae`](https://github.com/GitoxideLabs/gitoxide/commit/92febae025165c55e596d58511b1634fb6580b9c))
  - Support for `commitgraph list from..to` to exercise the new 'hide' capability. ([`c5bc49f`](https://github.com/GitoxideLabs/gitoxide/commit/c5bc49f2a02e9b28c2466ea4c7ae711d091ffc96))
  - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
  - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
  - Merge pull request #2019 from GitoxideLabs/precious-opt-in ([`5f9de52`](https://github.com/GitoxideLabs/gitoxide/commit/5f9de52cf286163b503047b1ab3b51dfa093b4d4))
  - Enable precious file parsing in `gix` CLI by default, allow overrides. ([`1df1ebb`](https://github.com/GitoxideLabs/gitoxide/commit/1df1ebb34dd3e2101d8a112dda66f6bac5261ea7))
  - Merge pull request #1977 from GitoxideLabs/dependabot/cargo/cargo-811d7b929d ([`800738a`](https://github.com/GitoxideLabs/gitoxide/commit/800738a37f3d33926a427edfa294423bbe3f2b66))
  - Bump the cargo group with 12 updates ([`4408166`](https://github.com/GitoxideLabs/gitoxide/commit/4408166bf56197a67419277a4ef8feeba9060fee))
  - Merge pull request #1973 from holodorum/feature/blame-range-support ([`de13b16`](https://github.com/GitoxideLabs/gitoxide/commit/de13b16728f6d29452cb97b50281aa91d498eb49))
  - Refactor ([`d4461e7`](https://github.com/GitoxideLabs/gitoxide/commit/d4461e700657d049a8cbc1552f328e35b27c92c3))
  - Add support for multiple blame ranges like `gix blame -L <range> -L <other-range> ...` ([`36a6ffe`](https://github.com/GitoxideLabs/gitoxide/commit/36a6ffeea7bbde7fb3689ddf2a107e09a50e602c))
  - Adapt to changes in `gix-blame` ([`8143d69`](https://github.com/GitoxideLabs/gitoxide/commit/8143d692e51c8d1b3d8c2323e83676e1be122f13))
  - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))

</details>

## 0.44.0 (2025-04-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release.
- 1 day passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.1, gix-utils v0.3.0, gix-actor v0.35.1, gix-validate v0.10.0, gix-path v0.10.17, gix-features v0.42.1, gix-hash v0.18.0, gix-hashtable v0.8.1, gix-object v0.49.1, gix-glob v0.20.0, gix-quote v0.6.0, gix-attributes v0.26.0, gix-command v0.6.0, gix-packetline-blocking v0.19.0, gix-filter v0.19.1, gix-fs v0.15.0, gix-commitgraph v0.28.0, gix-revwalk v0.20.1, gix-traverse v0.46.1, gix-worktree-stream v0.21.1, gix-archive v0.21.1, gix-tempfile v17.1.0, gix-lock v17.1.0, gix-index v0.40.0, gix-config-value v0.15.0, gix-pathspec v0.11.0, gix-ignore v0.15.0, gix-worktree v0.41.0, gix-diff v0.52.1, gix-blame v0.2.1, gix-ref v0.52.1, gix-sec v0.11.0, gix-config v0.45.1, gix-prompt v0.11.0, gix-url v0.31.0, gix-credentials v0.29.0, gix-discover v0.40.1, gix-dir v0.14.1, gix-mailmap v0.27.1, gix-revision v0.34.1, gix-merge v0.5.1, gix-negotiate v0.20.1, gix-pack v0.59.1, gix-odb v0.69.1, gix-refspec v0.30.1, gix-shallow v0.4.0, gix-packetline v0.19.0, gix-transport v0.47.0, gix-protocol v0.50.1, gix-status v0.19.1, gix-submodule v0.19.1, gix-worktree-state v0.19.0, gix v0.72.1, gix-fsck v0.11.1, gitoxide-core v0.47.1, gitoxide v0.44.0 ([`e104545`](https://github.com/GitoxideLabs/gitoxide/commit/e104545b78951ca882481d4a58f4425a8bc81c87))
  - Bump all prior pratch levels to majors ([`5f7f805`](https://github.com/GitoxideLabs/gitoxide/commit/5f7f80570e1a5522e76ea58cccbb957249a0dffe))
  - Merge pull request #1969 from GitoxideLabs/new-release ([`631f07a`](https://github.com/GitoxideLabs/gitoxide/commit/631f07ad0c1cb93d9da42cf2c8499584fe91880a))

</details>

## 0.43.0 (2025-04-25)

<csr-id-f952c101cc8686e685074e8604e0f332c06d6767/>

### Bug Fixes

- <csr-id-b07f907ba2e01849744c72df35dac57b624f2f85/> Adapt to changes in gix-actor
  Use the committer date and author date that are now backed by bytes and
  interpret these bytes into a `gix_date::Time` on demand.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 22 commits contributed to the release.
- 21 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
  - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
  - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
  - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
  - Thanks clippy ([`6f009d7`](https://github.com/GitoxideLabs/gitoxide/commit/6f009d781da9e931d44b113a925a80e77e8788af))
  - Adapt to changes in `gix-date` and `gix-actor` ([`afdf1a5`](https://github.com/GitoxideLabs/gitoxide/commit/afdf1a5d5c9fb2645f481c17f580ad59d14d6095))
  - Adapt to changes in gix-actor ([`b07f907`](https://github.com/GitoxideLabs/gitoxide/commit/b07f907ba2e01849744c72df35dac57b624f2f85))
  - Merge pull request #1968 from GitoxideLabs/dependabot/cargo/cargo-bd18780e40 ([`46227e6`](https://github.com/GitoxideLabs/gitoxide/commit/46227e6d1ddc0879662730e5bb21a8597716b1ca))
  - Bump the cargo group with 40 updates ([`06bf1e1`](https://github.com/GitoxideLabs/gitoxide/commit/06bf1e1552de65ce692911bdc4c501d487bbc3d7))
  - Merge pull request #1963 from joshtriplett/zlib-rs-default ([`9e075b9`](https://github.com/GitoxideLabs/gitoxide/commit/9e075b99ffc79173d4052d7550fd1d2826c5ec71))
  - Switch to zlib-rs by default and drop other zlib backends ([`96164c5`](https://github.com/GitoxideLabs/gitoxide/commit/96164c5936032b4edb973828178cc55793dd57cc))
  - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
  - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
  - Merge pull request #1953 from GitoxideLabs/dependabot/cargo/cargo-4a3cda0de8 ([`3aec7fb`](https://github.com/GitoxideLabs/gitoxide/commit/3aec7fbac52377bdeebc49759d4e0420b18b4e81))
  - Bump the cargo group with 3 updates ([`9f1fbc7`](https://github.com/GitoxideLabs/gitoxide/commit/9f1fbc741e1b6c718c7787f2858e07f3bd5473e9))
  - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
  - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
  - Merge pull request #1937 from EliahKagan/document-commands ([`de2f97d`](https://github.com/GitoxideLabs/gitoxide/commit/de2f97d793d8675659020883a04f47fc9ff06471))
  - Add missing command docs + small style fixups ([`f952c10`](https://github.com/GitoxideLabs/gitoxide/commit/f952c101cc8686e685074e8604e0f332c06d6767))
  - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
  - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
  - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))

</details>

## 0.42.0 (2025-04-04)

<csr-id-dd366a52d46078729a6e36468211b97c92c2921d/>

### New Features (BREAKING)

- <csr-id-f253f02a6658b3b7612a50d56c71f5ae4da4ca21/> detect SHA‐1 collision attacks
  Fix [GHSA-2frx-2596-x5r6].

### Changed (BREAKING)

- <csr-id-fd12ef89af29bf0684fc1df3e7b76ff367dee994/> drop obsolete SHA‐1 features
  The hashing API has moved to `gix_hash::hasher`, and we now use
  `sha1-checked` unconditionally.

### New Features

- <csr-id-dbb15328d7b26aaa51ceadcd5a8cb021155023b3/> add first 'debug' version of `gix diff file`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 21 commits contributed to the release.
- 76 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
  - Merge pull request #1915 from emilazy/push-qvyqmopsoltr ([`4660f7a`](https://github.com/GitoxideLabs/gitoxide/commit/4660f7a6f71873311f68f170b0f1f6659a02829d))
  - Drop obsolete SHA‐1 features ([`fd12ef8`](https://github.com/GitoxideLabs/gitoxide/commit/fd12ef89af29bf0684fc1df3e7b76ff367dee994))
  - Detect SHA‐1 collision attacks ([`f253f02`](https://github.com/GitoxideLabs/gitoxide/commit/f253f02a6658b3b7612a50d56c71f5ae4da4ca21))
  - Merge pull request #1880 from cruessler/add-gix-diff-file ([`b10cc6f`](https://github.com/GitoxideLabs/gitoxide/commit/b10cc6f0f2c3757677d9fdb3c01196d371ac6631))
  - Apply suggestions from code review ([`1039ae9`](https://github.com/GitoxideLabs/gitoxide/commit/1039ae95981a21340c77739891baa742b801a604))
  - Use revspecs for revision and path ([`dd366a5`](https://github.com/GitoxideLabs/gitoxide/commit/dd366a52d46078729a6e36468211b97c92c2921d))
  - Add first 'debug' version of `gix diff file` ([`dbb1532`](https://github.com/GitoxideLabs/gitoxide/commit/dbb15328d7b26aaa51ceadcd5a8cb021155023b3))
  - Merge pull request #1888 from cruessler/respect-diff-algorithm-in-blame ([`dce127e`](https://github.com/GitoxideLabs/gitoxide/commit/dce127e63f7788c5424e2da2cf4e3112f9c3b159))
  - Adapt to changes in `gix-blame` ([`8a31d88`](https://github.com/GitoxideLabs/gitoxide/commit/8a31d88dab3f2a79d4a0343f62d63a8d074ed94d))
  - Merge pull request #1859 from cruessler/add-failing-test-for-as-time ([`36a846f`](https://github.com/GitoxideLabs/gitoxide/commit/36a846f23ae0a3dfe95648605f0f618ccb55a881))
  - Merge pull request #1858 from cruessler/add-git-blame-since ([`7059609`](https://github.com/GitoxideLabs/gitoxide/commit/70596096e35ff8a910dacd6fefdc31d162282b81))
  - Add test for `--since` ([`330711d`](https://github.com/GitoxideLabs/gitoxide/commit/330711d69b343d3f4d59b3c9d9b87f75408fdc25))
  - Adapt to changes in `gix-blame` ([`e7084d8`](https://github.com/GitoxideLabs/gitoxide/commit/e7084d8db829bbc33aea958a997deb1db4b5a453))
  - Merge pull request #1882 from emilazy/push-ylwwuwymlmwt ([`10e41ee`](https://github.com/GitoxideLabs/gitoxide/commit/10e41ee6d1d3607c3d26a66b488d7d1eabc45c6e))
  - Fix cargo-deny using a prodash-update and ignore directive ([`cf7f34d`](https://github.com/GitoxideLabs/gitoxide/commit/cf7f34dcd653ddafaaecb149d4b98efa97d5b871))
  - Merge pull request #1861 from GitoxideLabs/revert-lazylock ([`6acca19`](https://github.com/GitoxideLabs/gitoxide/commit/6acca19163196f8402603cb031cb1d37f7178a43))
  - Don't use LazyLock just yet. ([`147eb41`](https://github.com/GitoxideLabs/gitoxide/commit/147eb4196acea76110b4adb26a295089e2365df5))
  - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
  - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
  - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))

</details>

## 0.41.0 (2025-01-18)

### New Features

- <csr-id-4a783959c4862985cbffc4fe5cd2c1bed38383b1/> add `gix blame -L start,end`
- <csr-id-75d689f6a195f301e4a3000e79f58e5ec1c20557/> add `gix env` to print paths relevant to the Git installation.
- <csr-id-278125a845a22fef39b9bc0c69e22e21fb559104/> Document the remaining subcommands
- <csr-id-4ffe6eb8f7921c6a03db0aa6d796cc2e3cc328e0/> Add support for statistics and additional performance information.
- <csr-id-80e5804dea9c1090efdcddbfc97ed1d573c28091/> add `gix blame` to the CLI
  That way it's possible to see the `blame` result of any file in the
  repository.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 15 commits contributed to the release over the course of 27 calendar days.
- 27 days passed between releases.
- 5 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
  - Merge pull request #1766 from cruessler/add-range-to-gix-blame ([`90fef01`](https://github.com/GitoxideLabs/gitoxide/commit/90fef0148376167763a3ebeff91a1cf9c236cf8a))
  - Add `gix blame -L start,end` ([`4a78395`](https://github.com/GitoxideLabs/gitoxide/commit/4a783959c4862985cbffc4fe5cd2c1bed38383b1))
  - Merge pull request #1758 from GitoxideLabs/git-shell ([`851a7c4`](https://github.com/GitoxideLabs/gitoxide/commit/851a7c4d0677b15e2158dbddcdd01d1b99711ea4))
  - Merge pull request #1757 from lu-zero/document-subcommands ([`31d83a4`](https://github.com/GitoxideLabs/gitoxide/commit/31d83a49c6a3f9b061b5df3728ab242e651c2ea7))
  - Add tailing `.` for consistency ([`ddddf02`](https://github.com/GitoxideLabs/gitoxide/commit/ddddf02a74d4fb749bb625b70434b3100843a59c))
  - Add `gix env` to print paths relevant to the Git installation. ([`75d689f`](https://github.com/GitoxideLabs/gitoxide/commit/75d689f6a195f301e4a3000e79f58e5ec1c20557))
  - Document the remaining subcommands ([`278125a`](https://github.com/GitoxideLabs/gitoxide/commit/278125a845a22fef39b9bc0c69e22e21fb559104))
  - Merge pull request #1752 from GitoxideLabs/git-shell ([`1ca480a`](https://github.com/GitoxideLabs/gitoxide/commit/1ca480aa4093328a7e047e770fdffdb8cc6d8e8d))
  - Thanks clippy ([`9193b05`](https://github.com/GitoxideLabs/gitoxide/commit/9193b05b2528f62d829447ccc50314bd4cffc415))
  - Merge pull request #1453 from cruessler/gix-blame ([`6ed9976`](https://github.com/GitoxideLabs/gitoxide/commit/6ed9976abaa3915b50efa46c46b195f3a1fc4ff7))
  - Add support for statistics and additional performance information. ([`4ffe6eb`](https://github.com/GitoxideLabs/gitoxide/commit/4ffe6eb8f7921c6a03db0aa6d796cc2e3cc328e0))
  - Add `gix blame` to the CLI ([`80e5804`](https://github.com/GitoxideLabs/gitoxide/commit/80e5804dea9c1090efdcddbfc97ed1d573c28091))
  - Release gix v0.69.1 ([`7659a65`](https://github.com/GitoxideLabs/gitoxide/commit/7659a651205c08ea4ec0cbf0b441a3bd17ec49dd))
  - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))

</details>

## 0.40.0 (2024-12-22)

### New Features (BREAKING)

- <csr-id-6367c7d0a796aff8ee8778916c1a1ddae68b654d/> Add `gix-shallow` crate and use it from `gix` and `gix-protocol`
  That way it's easier to reuse shallow-handling code from plumbing crates.
  
  Note that this is a breaking change as `gix-protocol` now uses the `gix-shallow::Update`
  type, which doesn't implement a formerly public `from_line()` method anymore.
  Now it is available as `fetch::response::shallow_update_from_line()`.

### New Features

- <csr-id-c7e04e976989435ba752628522d53ac39348b49b/> add first 'debug' version of `gix log`
  It's primarily meant to better understand `gix blame`.
- <csr-id-471e046a6002a68209e215db7ee904a4149f9d51/> add `--tree-favor` to `gix merge tree|commit`.
  With it one can decide which side to favor in case of
  irreconcilable tree-conflicts.

### Bug Fixes

- <csr-id-0727b5679f9ddeb05a9a50c895b6d77ba61ed544/> `gix merge file` now uses `THEIRS` instead of `OURS` where needed

### Commit Statistics

<csr-read-only-do-not-edit/>

- 11 commits contributed to the release over the course of 28 calendar days.
- 28 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#1703](https://github.com/GitoxideLabs/gitoxide/issues/1703)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#1703](https://github.com/GitoxideLabs/gitoxide/issues/1703)**
  - `gix merge file` now uses `THEIRS` instead of `OURS` where needed ([`0727b56`](https://github.com/GitoxideLabs/gitoxide/commit/0727b5679f9ddeb05a9a50c895b6d77ba61ed544))
- **Uncategorized**
  - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
  - Merge pull request #1643 from cruessler/add-gix-log ([`29cb775`](https://github.com/GitoxideLabs/gitoxide/commit/29cb775e6b85265a4907e882a45621aeb0432564))
  - Add first 'debug' version of `gix log` ([`c7e04e9`](https://github.com/GitoxideLabs/gitoxide/commit/c7e04e976989435ba752628522d53ac39348b49b))
  - Merge pull request #1634 from GitoxideLabs/remove-delegates ([`ddeb97f`](https://github.com/GitoxideLabs/gitoxide/commit/ddeb97f550bb95835648841b476d7647dd7c1dc0))
  - Add `gix-shallow` crate and use it from `gix` and `gix-protocol` ([`6367c7d`](https://github.com/GitoxideLabs/gitoxide/commit/6367c7d0a796aff8ee8778916c1a1ddae68b654d))
  - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
  - Adapt to changes in `gix-diff` ([`960773e`](https://github.com/GitoxideLabs/gitoxide/commit/960773e5526d02e1f2294224859c821ed86a3463))
  - Add `--tree-favor` to `gix merge tree|commit`. ([`471e046`](https://github.com/GitoxideLabs/gitoxide/commit/471e046a6002a68209e215db7ee904a4149f9d51))
  - Merge pull request #1704 from GitoxideLabs/fix-1703 ([`e8ce25f`](https://github.com/GitoxideLabs/gitoxide/commit/e8ce25f8d752c383ad8aea45fc7710a6d4aeb2ef))
  - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))

</details>

## 0.39.0 (2024-11-24)

### Bug Fixes

- <csr-id-3506afbae149f2c8dabbd3b362604d3e9d9ff725/> no `ein t query` in `max-pure`
  That way, it won't require sqlite to be compiled, which was
  part of the original idea.
  
  It's worth noting that some C is still getting compiled right now
  as part of `ring`.

### New Features

- <csr-id-0a5993f4ed6e3815949b3cd398d38d69417d21c8/> add `gix merge commit --debug`
  This yields additional debugging information with details about detected conflicts.
- <csr-id-4a5afc7524fc96213385454079ebf9baf302ad4b/> add `gix merge commits`
- <csr-id-84707c2b7540f9a73cc3f0cde74dabd9822cd809/> add `gix merge tree` to merge trees similarly to `git merge-tree`.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 13 commits contributed to the release.
- 33 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#1681](https://github.com/GitoxideLabs/gitoxide/issues/1681)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#1681](https://github.com/GitoxideLabs/gitoxide/issues/1681)**
  - No `ein t query` in `max-pure` ([`3506afb`](https://github.com/GitoxideLabs/gitoxide/commit/3506afbae149f2c8dabbd3b362604d3e9d9ff725))
- **Uncategorized**
  - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
  - Merge pull request #1661 from GitoxideLabs/merge ([`0b7abfb`](https://github.com/GitoxideLabs/gitoxide/commit/0b7abfbdebe8c5ab30b89499a70dd7727de41184))
  - Add `gix merge commit --debug` ([`0a5993f`](https://github.com/GitoxideLabs/gitoxide/commit/0a5993f4ed6e3815949b3cd398d38d69417d21c8))
  - Merge pull request #1684 from GitoxideLabs/fixes ([`9ab86a2`](https://github.com/GitoxideLabs/gitoxide/commit/9ab86a23d45941c4f0a3239e0cb57d4161dd279c))
  - Merge pull request #1658 from GitoxideLabs/merge ([`905e5b4`](https://github.com/GitoxideLabs/gitoxide/commit/905e5b42a6163f92edef8fab82d97aeb6f17cf06))
  - Add `gix merge commits` ([`4a5afc7`](https://github.com/GitoxideLabs/gitoxide/commit/4a5afc7524fc96213385454079ebf9baf302ad4b))
  - Merge pull request #1651 from GitoxideLabs/merge ([`a876533`](https://github.com/GitoxideLabs/gitoxide/commit/a8765330fc16997dee275866b18a128dec1c5d55))
  - Adapt to changes in `gix` ([`8d590f3`](https://github.com/GitoxideLabs/gitoxide/commit/8d590f33f49b556de1748818e0bbec610566842f))
  - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
  - Add `gix merge tree` to merge trees similarly to `git merge-tree`. ([`84707c2`](https://github.com/GitoxideLabs/gitoxide/commit/84707c2b7540f9a73cc3f0cde74dabd9822cd809))
  - Support for merge related options in config tree ([`80e006b`](https://github.com/GitoxideLabs/gitoxide/commit/80e006b759d130f4f07a346b75cfc0b39986210c))
  - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))

</details>

## 0.38.0 (2024-10-22)

<csr-id-28ac6572722f7ea31795dc0417521c70bcb6ec8f/>
<csr-id-786bfec7c2cf25a68705b6ef09ba6390600742b8/>

### Bug Fixes (BREAKING)

- <csr-id-17573779688e755a786546d5e42ab533088cd726/> remove all workspace dependencies
  The problem is that with them, we don't notice anymore if the crate changes,
  because a dependency changes. That also means that older versions of the dependency
  may stay even though some other crates might pick up a newer version.
  
  Ultimately, this will lead to drift and subtle incompatibilities.
  
  We declare this breaking to enforce a proper re-release.

### Documentation

- <csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/> Update gitoxide repository URLs
  This updates `Byron/gitoxide` URLs to `GitoxideLabs/gitoxide` in:
  
  - Markdown documentation, except changelogs and other such files
  where such changes should not be made.
  
  - Documentation comments (in .rs files).
  
  - Manifest (.toml) files, for the value of the `repository` key.
  
  - The comments appearing at the top of a sample hook that contains
  a repository URL as an example.
  
  When making these changes, I also allowed my editor to remove
  trailing whitespace in any lines in files already being edited
  (since, in this case, there was no disadvantage to allowing this).
  
  The gitoxide repository URL changed when the repository was moved
  into the recently created GitHub organization `GitoxideLabs`, as
  detailed in #1406. Please note that, although I believe updating
  the URLs to their new canonical values is useful, this is not
  needed to fix any broken links, since `Byron/gitoxide` URLs
  redirect (and hopefully will always redirect) to the coresponding
  `GitoxideLabs/gitoxide` URLs.
  
  While this change should not break any URLs, some affected URLs
  were already broken. This updates them, but they are still broken.
  They will be fixed in a subsequent commit.
  
  This also does not update `Byron/gitoxide` URLs in test fixtures
  or test cases, nor in the `Makefile`. (It may make sense to change
  some of those too, but it is not really a documentation change.)

### New Features

- <csr-id-c7213bcf2cf6be4058a78dbd86bbab4c5ad4afa7/> support for listing worktrees with `gix worktree list`
- <csr-id-0c554e096cf2bf0a2bb91397143f524219693a3b/> add first 'debug' version of `gix diff tree`.
  It's primarily meant to better understand `gix blame`.
- <csr-id-9c8bc03de99e6494abd9755deef7e7be5577bce2/> add new `gix cat` command.
  It only prints things without fuzz.
  
  Inspired by https://youtu.be/JYH5ILv5g1g?si=bHLBPFJiZyRUTl6u&t=211.
- <csr-id-3da2da9d7993adc16b19fc63e7524c768a6e2e7f/> add `gix merge-file` with similar features as `git merge-file`
- <csr-id-7249291016253647c920852fb37eb9e29d615775/> `gix merge-base` for the CLI
  For now it only supports the standard merge-base, but more derivatives
  can be added easily on demand.
- <csr-id-f2e813d8712f245f7f93ecae2a3da768dd6dc2db/> provide facilities to perform rewrite-tracking for tree-diffs in plumbing.
  This effectively pulls down a higher-level implementation in `gix` to the plumbing
  level, to allow it to be used there as well.

### Bug Fixes

- <csr-id-6eb4ea6cf27b2feb3edb9dbf754393b0c7d272fd/> Adjust `gix clean` warning and help for worktree fix
- <csr-id-c241422dc64f97e9dc781465ae69130a423ee501/> Clarify -r/--repositories and --skip-hidden-repositories
  This adds information to the help text for the `git clean` options
  `-r`/`--repositories` and `--skip-hidden-repositories` to make it
  clearer what their relationship is, avoid creating the false
  impression that repositories are never deleted in the absence of
  `-r`/`--repositories`, and note that `--skip-hidden-repositories`
  is sometimes needed to preserve not only separate repositories but
  the (probably rarer) case of hidden nested worktrees of the current
  repository.
- <csr-id-a10e7c2b26d2151894a29f543e3cd9576bc92dc4/> assure build-script is added to the package
  Otherwise, the `GIX_VERSION` environment variable is not available at build time,
  which can lead to runtime errors.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 58 commits contributed to the release over the course of 89 calendar days.
- 91 days passed between releases.
- 13 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#1466](https://github.com/GitoxideLabs/gitoxide/issues/1466), [#1572](https://github.com/GitoxideLabs/gitoxide/issues/1572)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#1466](https://github.com/GitoxideLabs/gitoxide/issues/1466)**
  - Prepare manual release of `gitoxide-core` ([`8b09cf2`](https://github.com/GitoxideLabs/gitoxide/commit/8b09cf254e8e376907b9e26d172cb9adf98f4d6e))
- **[#1572](https://github.com/GitoxideLabs/gitoxide/issues/1572)**
  - Assure build-script is added to the package ([`a10e7c2`](https://github.com/GitoxideLabs/gitoxide/commit/a10e7c2b26d2151894a29f543e3cd9576bc92dc4))
  - Make clear what `gix commit-graph` is ([`49bca2a`](https://github.com/GitoxideLabs/gitoxide/commit/49bca2abe4512180f080f678e73c0fa40885379a))
- **Uncategorized**
  - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
  - Merge pull request #1639 from cruessler/respect-env-variables ([`48aa74b`](https://github.com/GitoxideLabs/gitoxide/commit/48aa74b911fb874986c244712b7fd5b5cc10070b))
  - Respect env variables when running gix ([`0bebe52`](https://github.com/GitoxideLabs/gitoxide/commit/0bebe524b75346edca219d13c10b52dee3273643))
  - Merge pull request #1626 from cruessler/add-gix-diff ([`f186c23`](https://github.com/GitoxideLabs/gitoxide/commit/f186c2381b91f350813076927bf988d253fe1ad0))
  - Some minor modifications ([`6777ecb`](https://github.com/GitoxideLabs/gitoxide/commit/6777ecb99306830a3353a0db24caaa69e348ca74))
  - Add first 'debug' version of `gix diff tree`. ([`0c554e0`](https://github.com/GitoxideLabs/gitoxide/commit/0c554e096cf2bf0a2bb91397143f524219693a3b))
  - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
  - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
  - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
  - Provide facilities to perform rewrite-tracking for tree-diffs in plumbing. ([`f2e813d`](https://github.com/GitoxideLabs/gitoxide/commit/f2e813d8712f245f7f93ecae2a3da768dd6dc2db))
  - Merge pull request #1616 from Byron/git-cat ([`31bdd2e`](https://github.com/GitoxideLabs/gitoxide/commit/31bdd2ecc6c800dc57faedc9250be6d5fbcc1133))
  - Add new `gix cat` command. ([`9c8bc03`](https://github.com/GitoxideLabs/gitoxide/commit/9c8bc03de99e6494abd9755deef7e7be5577bce2))
  - Merge pull request #1611 from Byron/merge ([`5ffccd2`](https://github.com/GitoxideLabs/gitoxide/commit/5ffccd2f08d70576347e3ae17a66ca5a60f1d81c))
  - Add `gix merge-file` with similar features as `git merge-file` ([`3da2da9`](https://github.com/GitoxideLabs/gitoxide/commit/3da2da9d7993adc16b19fc63e7524c768a6e2e7f))
  - Add all keys for merge-configuration ([`e0b09d2`](https://github.com/GitoxideLabs/gitoxide/commit/e0b09d2764fd02a2b69340d9b3aef9773ae899ce))
  - Merge pull request #1585 from Byron/merge ([`2261de4`](https://github.com/GitoxideLabs/gitoxide/commit/2261de470aeb77be080f9e423e1513bde85d9cc0))
  - Add the `gix-merge` crate for capturing merge algorithms ([`65efcb7`](https://github.com/GitoxideLabs/gitoxide/commit/65efcb7624031ae478056fbb49a07ef176f0c96b))
  - Merge pull request #1586 from Byron/fix-ci ([`22fbe70`](https://github.com/GitoxideLabs/gitoxide/commit/22fbe705968689acdc08e7472a1345cf690d1b19))
  - Update crate-status to inform about tree-editing capabilities ([`fe1eb97`](https://github.com/GitoxideLabs/gitoxide/commit/fe1eb9740c66ccb49d1c43a269f2970a721b1a34))
  - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
  - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
  - `gix merge-base` for the CLI ([`7249291`](https://github.com/GitoxideLabs/gitoxide/commit/7249291016253647c920852fb37eb9e29d615775))
  - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
  - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
  - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
  - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
  - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
  - Merge branch 'fixes' ([`46cd1ae`](https://github.com/GitoxideLabs/gitoxide/commit/46cd1aed7815d27cdc818edb87641b20b82ba048))
  - Merge pull request #1537 from EliahKagan/progress-style ([`472fe5f`](https://github.com/GitoxideLabs/gitoxide/commit/472fe5fa18d6d1b948e319919a48f965a1199929))
  - Unify style in config support info ([`786bfec`](https://github.com/GitoxideLabs/gitoxide/commit/786bfec7c2cf25a68705b6ef09ba6390600742b8))
  - Remove all workspace dependencies ([`1757377`](https://github.com/GitoxideLabs/gitoxide/commit/17573779688e755a786546d5e42ab533088cd726))
  - Release gix-actor v0.32.0, gix-object v0.44.0, gix-filter v0.13.0, gix-revwalk v0.15.0, gix-traverse v0.41.0, gix-worktree-stream v0.15.0, gix-archive v0.15.0, gix-ref v0.47.0, gix-config v0.40.0, gix-index v0.35.0, gix-worktree v0.36.0, gix-diff v0.46.0, gix-discover v0.35.0, gix-dir v0.8.0, gix-mailmap v0.24.0, gix-negotiate v0.15.0, gix-pack v0.53.0, gix-odb v0.63.0, gix-revision v0.29.0, gix-refspec v0.25.0, gix-status v0.13.0, gix-submodule v0.14.0, gix-worktree-state v0.13.0, gix v0.66.0, gix-fsck v0.6.0, gitoxide-core v0.41.0, gitoxide v0.38.0, safety bump 26 crates ([`b3ff033`](https://github.com/GitoxideLabs/gitoxide/commit/b3ff033b602f303433f0b2e4daa2dba90b619c9e))
  - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
  - Merge pull request #1531 from EliahKagan/progress-typos ([`25a3f1b`](https://github.com/GitoxideLabs/gitoxide/commit/25a3f1b0b07c01dd44df254f46caa6f78a4d3014))
  - Fix typos in config support info ([`b31d6b7`](https://github.com/GitoxideLabs/gitoxide/commit/b31d6b79fd9bc528deaf0e3027dcaae0229ff581))
  - Merge pull request #1529 from Byron/better-copy-detection ([`7b7902e`](https://github.com/GitoxideLabs/gitoxide/commit/7b7902eb8478607d611030d59fa87b390ca7ee76))
  - Add `it copy-royal` a degenerative copy tool ([`ba91274`](https://github.com/GitoxideLabs/gitoxide/commit/ba91274543a8f1c36bbb1ab096be24f6f7ca428a))
  - Remove superfluous `build-override` ([`b0668fb`](https://github.com/GitoxideLabs/gitoxide/commit/b0668fba23c714f7d49013bd2e3ff248341b5770))
  - Merge pull request #1495 from EliahKagan/profiles ([`96d3470`](https://github.com/GitoxideLabs/gitoxide/commit/96d3470031b43a948e2393ed1f757ecddbaf83a2))
  - Make use of profile clear by '-github' suffix ([`34b1060`](https://github.com/GitoxideLabs/gitoxide/commit/34b1060a63cc6d3461d045371ca2ae1c8341ce2c))
  - Reenable publishing; warn that `release-opt` may go away ([`871eec7`](https://github.com/GitoxideLabs/gitoxide/commit/871eec7ab34c0ffd1f38931e6622a12659d4b095))
  - Split `release` profile into `release` and `release-opt` ([`afd8d39`](https://github.com/GitoxideLabs/gitoxide/commit/afd8d395d0afda95cee5dbf20c143b5c2429989a))
  - Merge branch 'dependabot/github_actions/github-actions-c4bcf5a8e2' ([`2e00b5e`](https://github.com/GitoxideLabs/gitoxide/commit/2e00b5ef6e8a15e7f0a34d54739a5cd1c986b322))
  - Use workspace dependency for `unicode-bom` ([`89d77dc`](https://github.com/GitoxideLabs/gitoxide/commit/89d77dc262f3b576ab4f4939e65cac866da18927))
  - Make `winnow` a workspace dependency ([`78a7e32`](https://github.com/GitoxideLabs/gitoxide/commit/78a7e32c34150dece4065e513cd177356619419f))
  - Merge branch 'upgrades' ([`1d37bf6`](https://github.com/GitoxideLabs/gitoxide/commit/1d37bf6a773d56eea9003aa626ced413e8e0eaa3))
  - Upgrade `rusqlite` to the latest version ([`51c4706`](https://github.com/GitoxideLabs/gitoxide/commit/51c4706db61348531759686c3d6b805b72bcdb6b))
  - Upgrade `prodash` to latest version ([`067a822`](https://github.com/GitoxideLabs/gitoxide/commit/067a8225a8264832335ca3f40a44121bd5a64841))
  - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
  - Switch from `time` to `jiff` ([`28ac657`](https://github.com/GitoxideLabs/gitoxide/commit/28ac6572722f7ea31795dc0417521c70bcb6ec8f))
  - Merge pull request #1471 from EliahKagan/hidden ([`a9aac4f`](https://github.com/GitoxideLabs/gitoxide/commit/a9aac4f88d1d6245a3b03bd46e691697f661c114))
  - Adjust `gix clean` warning and help for worktree fix ([`6eb4ea6`](https://github.com/GitoxideLabs/gitoxide/commit/6eb4ea6cf27b2feb3edb9dbf754393b0c7d272fd))
  - Clarify -r/--repositories and --skip-hidden-repositories ([`c241422`](https://github.com/GitoxideLabs/gitoxide/commit/c241422dc64f97e9dc781465ae69130a423ee501))
  - Merge branch 'fix-clean' ([`33eacfb`](https://github.com/GitoxideLabs/gitoxide/commit/33eacfbaace2021043e2b5d72dcb9293af6c4020))
  - Support for listing worktrees with `gix worktree list` ([`c7213bc`](https://github.com/GitoxideLabs/gitoxide/commit/c7213bcf2cf6be4058a78dbd86bbab4c5ad4afa7))

</details>

## 0.37.0 (2024-07-23)

<csr-id-202f3e48a96b38d32b11d28449358c7d1f3546ff/>

### Chore

- <csr-id-7f0a08f31661b917859dd761bf6610a303c1d432/> version bump dependency zip-2.0.0
  upstream revoked version zip-1.3.1

### New Features

- <csr-id-2a563870fc4d2f65e34c4e4a448b446fc05a43b2/> add `mailmap check` similar to `git check-mailmap`.
  That way it becomes a little easier to maybe try out other special cases
  in the wild.
- <csr-id-9bf01e42b8d8964dfd1e099d645082c10bdabcdf/> `gix clone` with `--ref` support.
  `--ref` is similar to `--branch`, but was renamed as it also supports
  tags for example.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 18 commits contributed to the release.
- 62 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
  - Merge branch 'gix-blame' ([`e6fbea9`](https://github.com/GitoxideLabs/gitoxide/commit/e6fbea9be2ef7ab4064dc57c8233dfe81fac3bb4))
  - Add new `gix-blame` crate ([`f5f616d`](https://github.com/GitoxideLabs/gitoxide/commit/f5f616d8345898effc79d587c139e249f1c85ab6))
  - Merge branch 'fix-mailmap' ([`f107014`](https://github.com/GitoxideLabs/gitoxide/commit/f107014022a62271f02790a83336aed186ad38a3))
  - Add descriptive docs for `mailmap check` ([`3e08fa3`](https://github.com/GitoxideLabs/gitoxide/commit/3e08fa313c490f4ff299724a9363e0f3b370eea6))
  - Add `mailmap check` similar to `git check-mailmap`. ([`2a56387`](https://github.com/GitoxideLabs/gitoxide/commit/2a563870fc4d2f65e34c4e4a448b446fc05a43b2))
  - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
  - Merge branch 'feat/checkout-other-refs' ([`ecfde07`](https://github.com/GitoxideLabs/gitoxide/commit/ecfde07d0887322db34f5ea531891c92676e1ff4))
  - `gix clone` with `--ref` support. ([`9bf01e4`](https://github.com/GitoxideLabs/gitoxide/commit/9bf01e42b8d8964dfd1e099d645082c10bdabcdf))
  - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
  - Thanks clippy ([`acc1331`](https://github.com/GitoxideLabs/gitoxide/commit/acc13318731fabac8f65d604baf7e47814f92ad4))
  - Merge pull request #1384 from NobodyXu/feat/easier-to-compile-gix-as-dynlib ([`bb30e52`](https://github.com/GitoxideLabs/gitoxide/commit/bb30e52112a1b738afae557f6ba7bc23889cdcb7))
  - Fix clippy warning ([`bf5a111`](https://github.com/GitoxideLabs/gitoxide/commit/bf5a1112245b6d60ceaf5591acf15acd0c8c6363))
  - Fix compilation error in `src/plumbing/main.rs` ([`4596a39`](https://github.com/GitoxideLabs/gitoxide/commit/4596a3949cdeb7c6953c2a9b8ac1e51609e1b160))
  - Fix compilation errors ([`0b5dc74`](https://github.com/GitoxideLabs/gitoxide/commit/0b5dc744b951a2ff49ddc4d2c10aaeaf10da2c4b))
  - Make it easier to compile gitoxide as dynlib ([`202f3e4`](https://github.com/GitoxideLabs/gitoxide/commit/202f3e48a96b38d32b11d28449358c7d1f3546ff))
  - Merge pull request #1379 from belt/main ([`b4ab6f7`](https://github.com/GitoxideLabs/gitoxide/commit/b4ab6f77596ae54f68743bbd93228dca08cc0440))
  - Version bump dependency zip-2.0.0 ([`7f0a08f`](https://github.com/GitoxideLabs/gitoxide/commit/7f0a08f31661b917859dd761bf6610a303c1d432))

</details>

## 0.36.0 (2024-05-22)

### New Features

- <csr-id-886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e/> checkout respects options for `core.protectHFS` and `core.protectNTFS`.
  This also adds `gitoxide.core.protectWindows` as a way to enforce
  additional restrictions that are usually only available on Windows.
  
  Note that `core.protectNFS` is always enabled by default, just like
  [it is in Git](https://github.com/git/git/commit/9102f958ee5254b10c0be72672aa3305bf4f4704).

### Bug Fixes

- <csr-id-addf446f052ff74edcdb083f2b2968b313daa940/> more robustness in the face of a trampling-herd of threads loading a single index.
  The motivating example is here: https://github.com/praetorian-inc/noseyparker/issues/179
  
  Previously, it was possible for a trampling herd of threads to consolidate the
  disk state. Most of them would be 'needs-init' threads which could notice that
  the initialization already happened, and just use that.
  
  But a thread might be late for the party and somehow manages to not get any
  newly loaded index, and thus tries to consolidate with what's on disk again.
  Then it would again determine no change, and return nothing, causing the caller
  to abort and not find objects it should find because it wouldn't see the index
  that it should have seen.
  
  The reason the thread got into this mess is that the 'is-load-ongoing' flagging
  was racy itself, so it would not wait for ongoing loads and just conclude nothing
  happened. An extra delay (by yielding) now assures it either seees the loading state
  and waits for it, sees the newly loaded indices.
  
  Note that this issue can be reproduced with:
  
  ```
  './target/release/gix -r repo-with-one-pack -t10 --trace odb stats --extra-header-lookup'
  ```

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 10 calendar days.
- 39 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
  - Merge branch 'various-fixes' ([`d6cd449`](https://github.com/GitoxideLabs/gitoxide/commit/d6cd44930fb204b06e2b70fc6965e7705530c47a))
  - Update dependencies ([`cd4de83`](https://github.com/GitoxideLabs/gitoxide/commit/cd4de8327fc195eb862ab6e138f2315a87374f85))
  - Merge pull request from GHSA-7w47-3wg8-547c ([`79dce79`](https://github.com/GitoxideLabs/gitoxide/commit/79dce79c62f6072aa2653780d590dc3993dfa401))
  - Checkout respects options for `core.protectHFS` and `core.protectNTFS`. ([`886d6b5`](https://github.com/GitoxideLabs/gitoxide/commit/886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e))
  - Mark safety-related core-flags as planned ([`f3d5a69`](https://github.com/GitoxideLabs/gitoxide/commit/f3d5a69bbe0ad14502ce617dc580cc2aa481bb0a))
  - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
  - More robustness in the face of a trampling-herd of threads loading a single index. ([`addf446`](https://github.com/GitoxideLabs/gitoxide/commit/addf446f052ff74edcdb083f2b2968b313daa940))

</details>

## 0.35.0 (2024-04-13)

### New Features

- <csr-id-84c74ffa698d35f8395c63db6acd3d0e6700d07f/> add `gix status --ignored` support
- <csr-id-66e87cd31c060c3f97ac685ee0541c408f600362/> add `gix status --index-worktree-renames`
  This enables rename-tracking between worktree and index, something
  that Git also doesn't do or doesn't do by default.
  It is, however, available in `git2`.
- <csr-id-61c002bc4ca5b5345c411e561fdcb492e7ae1d97/> `gix status` with submodule and rewrite support.
  Submodule changes are now picked up as long as the submodule is
  in the index.
  Further, it's possible to enable rename-tracking between
  index and worktree separately.
- <csr-id-98b368095ec99d1bc287da7f9294a9fce424deed/> add `gix is-clean|is-changed`
  It's a good way to compare the time it takes to run a full status
  compared to a quick is-dirty check.
- <csr-id-afd20caadb40b6b793f2099b7232669f9a8f9086/> `gix submodules list --dirty-suffix` for dirty-information
  This is a submodule-centric and greatly simplified way of obtaining
  describe information with dirty-suffix.
  
  Note that `status` information is also possible, but it seems
  hard to display nicely, which this command isn't great at
  in the first place.
- <csr-id-58231b418fa39ea122ef41bb7691289f5b0be855/> add `gix commit describe --dirty-suffix`
  That way a suffix will be added depending on the dirty-state of the repository.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 18 commits contributed to the release.
- 48 days passed between releases.
- 6 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
  - Merge pull request #1341 from szepeviktor/typos ([`55f379b`](https://github.com/GitoxideLabs/gitoxide/commit/55f379bc47065822d078393d83d30c0835a89782))
  - Fix typos ([`f72ecce`](https://github.com/GitoxideLabs/gitoxide/commit/f72ecce45babcad2a0c9b73c79d01ff502907a57))
  - Release gix-packetline v0.17.5, gix-transport v0.41.3, gix v0.61.1 ([`57579f1`](https://github.com/GitoxideLabs/gitoxide/commit/57579f1ee4ef12c214db36325a2a0b2e8b2b14fd))
  - Merge branch 'patch-1' ([`8fde62b`](https://github.com/GitoxideLabs/gitoxide/commit/8fde62b2617985f835e2e2fa07c735a5158789cf))
  - Turn`curl` into a workspace package ([`adee500`](https://github.com/GitoxideLabs/gitoxide/commit/adee50016007619495c93580e845ae757377c4f0))
  - Make reqwest a workspace package ([`369cf1b`](https://github.com/GitoxideLabs/gitoxide/commit/369cf1b03735617debe1527b3f23247685181e7d))
  - Release gix-actor v0.31.1, gix-object v0.42.1, gix-index v0.31.1, gix-pathspec v0.7.2, gix-dir v0.3.0, gix-status v0.8.0, gix v0.61.0, safety bump 2 crates ([`155cc45`](https://github.com/GitoxideLabs/gitoxide/commit/155cc45730b259e662d7c4be42a469a3af3750e1))
  - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
  - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
  - Add `gix status --format` to communicate the current format is very simple. ([`23bea36`](https://github.com/GitoxideLabs/gitoxide/commit/23bea36f046a6f652cd1e06885ae132c85bb4f05))
  - Add `gix status --ignored` support ([`84c74ff`](https://github.com/GitoxideLabs/gitoxide/commit/84c74ffa698d35f8395c63db6acd3d0e6700d07f))
  - Add `gix status --index-worktree-renames` ([`66e87cd`](https://github.com/GitoxideLabs/gitoxide/commit/66e87cd31c060c3f97ac685ee0541c408f600362))
  - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
  - `gix status` with submodule and rewrite support. ([`61c002b`](https://github.com/GitoxideLabs/gitoxide/commit/61c002bc4ca5b5345c411e561fdcb492e7ae1d97))
  - Add `gix is-clean|is-changed` ([`98b3680`](https://github.com/GitoxideLabs/gitoxide/commit/98b368095ec99d1bc287da7f9294a9fce424deed))
  - `gix submodules list --dirty-suffix` for dirty-information ([`afd20ca`](https://github.com/GitoxideLabs/gitoxide/commit/afd20caadb40b6b793f2099b7232669f9a8f9086))
  - Add `gix commit describe --dirty-suffix` ([`58231b4`](https://github.com/GitoxideLabs/gitoxide/commit/58231b418fa39ea122ef41bb7691289f5b0be855))

</details>

## 0.34.0 (2024-02-25)

### New Features

- <csr-id-9863d75147445d3a598fe1339d88c353850a5984/> add `gix clean --patterns-for-entries|-m` to help with wildcards.
- <csr-id-e8597f3559187fc8add294e72eb33403cdff0e09/> basic `gix clean`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 31 commits contributed to the release over the course of 57 calendar days.
- 58 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge branch 'status' ([`d53504a`](https://github.com/GitoxideLabs/gitoxide/commit/d53504a1fad41cec7b6ca2a4abb7f185d8941e3f))
  - Allow the `-n` argument as no-op to make them more compatible. ([`8959b21`](https://github.com/GitoxideLabs/gitoxide/commit/8959b2153f32c2fba599e3dfa1720e155b462b94))
  - Add `gix clean --patterns-for-entries|-m` to help with wildcards. ([`9863d75`](https://github.com/GitoxideLabs/gitoxide/commit/9863d75147445d3a598fe1339d88c353850a5984))
  - Merge branch 'tabled' ([`ed79aa7`](https://github.com/GitoxideLabs/gitoxide/commit/ed79aa7b410b9e5a013c4f204b20abd51d270559))
  - Refactor ([`e25d7eb`](https://github.com/GitoxideLabs/gitoxide/commit/e25d7eb443d9e1fde8d16422251ee0d288ff4a51))
  - Merge branch 'ein-completions' ([`cebeedf`](https://github.com/GitoxideLabs/gitoxide/commit/cebeedf572d7086c9959754c873312a183d575e3))
  - Fix clippy lints ([`454e6b9`](https://github.com/GitoxideLabs/gitoxide/commit/454e6b9de86def27c811cc93fd80f4eb169dda52))
  - Add back line wrapping simplify Usage Variants ([`4ac597a`](https://github.com/GitoxideLabs/gitoxide/commit/4ac597a0e9ccb8ed1e1864afcffdc55abc37260c))
  - Tell clap that ein is named ein ([`4fd6ab8`](https://github.com/GitoxideLabs/gitoxide/commit/4fd6ab89753f9b947034806fecb60ad77380334c))
  - Copy over code from gix to generate completions for ein as well ([`abe0043`](https://github.com/GitoxideLabs/gitoxide/commit/abe004372c6a0027339563a88162ca1aca320a1d))
  - Fix indenting by removing formatting on the config str ([`c0c4d59`](https://github.com/GitoxideLabs/gitoxide/commit/c0c4d59bbae688074184b186e08c6bf8e23c5705))
  - Reserve 50 characters for the config name ([`cc8d008`](https://github.com/GitoxideLabs/gitoxide/commit/cc8d008843a4e4567301c14ce5f25855417cf898))
  - Bring back old implementation from https://github.com/Byron/gitoxide/commit/65e64964c7cd151e53e5a7d4b9ba8fabda1c0e16 this also removes terminal_size, although its not large ([`1ecc96d`](https://github.com/GitoxideLabs/gitoxide/commit/1ecc96d319bc60cb9dbabcf17211aa7057134fd6))
  - Remove tabled ([`fcd585b`](https://github.com/GitoxideLabs/gitoxide/commit/fcd585bb6dc27bbf5bf74b422b36614241b59eb4))
  - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
  - Basic `gix clean` ([`e8597f3`](https://github.com/GitoxideLabs/gitoxide/commit/e8597f3559187fc8add294e72eb33403cdff0e09))
  - Merge branch 'tempfile-permissions' ([`7b44c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7b44c7ff1dc0b8875214d2673c7f52948cf04ff0))
  - Release gix-tempfile v13.1.0, gix-lock v13.1.0, safety bump 12 crates ([`8430442`](https://github.com/GitoxideLabs/gitoxide/commit/84304427dfe4d170c7732161b126961719f70059))
  - Merge branch 'chunks_exact' ([`d4d478b`](https://github.com/GitoxideLabs/gitoxide/commit/d4d478ba6a70fce34717e2164f8b3aa7a1521a2d))
  - Use chunks_exact where possible ([`2482023`](https://github.com/GitoxideLabs/gitoxide/commit/24820232f07ae55c80d95470228cc9e874830487))
  - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
  - Merge pull request #1257 from joshtriplett/faster-hex-no-serde ([`3ae5baa`](https://github.com/GitoxideLabs/gitoxide/commit/3ae5baa2e214d5e2e048f9793e4408864e31dc21))
  - Disable faster-hex's default `serde` feature ([`140341c`](https://github.com/GitoxideLabs/gitoxide/commit/140341c68f61b70f1585523b50310547edd4f5cf))
  - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
  - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
  - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
  - Merge pull request #1239 from NobodyXu/binstall ([`71ab298`](https://github.com/GitoxideLabs/gitoxide/commit/71ab298d72d5cd4901bde7907f40adaaac36829d))
  - Add `cargo-binstall` support for gitoxide ([`d5976b3`](https://github.com/GitoxideLabs/gitoxide/commit/d5976b3c074b2606f3098beb735b40871761b734))
  - Merge branch 'gix-status' ([`c3983c6`](https://github.com/GitoxideLabs/gitoxide/commit/c3983c6b8d63d85ec713ae8d661723f9cf0bd55b))
  - Initial version of the `gix-dir` crate ([`22acf0d`](https://github.com/GitoxideLabs/gitoxide/commit/22acf0def5c62563300aa8eaef01cb94bcd15645))
  - Release gix-date v0.8.3, gix-hash v0.14.1, gix-trace v0.1.6, gix-features v0.37.1, gix-actor v0.29.1, gix-validate v0.8.3, gix-object v0.40.1, gix-path v0.10.3, gix-glob v0.15.1, gix-quote v0.4.10, gix-attributes v0.21.1, gix-command v0.3.2, gix-packetline-blocking v0.17.2, gix-utils v0.1.8, gix-filter v0.8.1, gix-fs v0.9.1, gix-chunk v0.4.7, gix-commitgraph v0.23.1, gix-hashtable v0.5.1, gix-revwalk v0.11.1, gix-traverse v0.36.1, gix-worktree-stream v0.8.1, gix-archive v0.8.1, gix-config-value v0.14.3, gix-tempfile v12.0.1, gix-lock v12.0.1, gix-ref v0.40.1, gix-sec v0.10.3, gix-config v0.33.1, gix-prompt v0.8.2, gix-url v0.26.1, gix-credentials v0.23.1, gix-ignore v0.10.1, gix-bitmap v0.2.10, gix-index v0.28.1, gix-worktree v0.29.1, gix-diff v0.39.1, gix-discover v0.28.1, gix-macros v0.1.3, gix-mailmap v0.21.1, gix-negotiate v0.11.1, gix-pack v0.46.1, gix-odb v0.56.1, gix-pathspec v0.5.1, gix-packetline v0.17.2, gix-transport v0.40.1, gix-protocol v0.43.1, gix-revision v0.25.1, gix-refspec v0.21.1, gix-status v0.4.1, gix-submodule v0.7.1, gix-worktree-state v0.6.1, gix v0.57.1 ([`972241f`](https://github.com/GitoxideLabs/gitoxide/commit/972241f1904944e8b6e84c6aa1649a49be7a85c3))

</details>

## 0.33.0 (2023-12-29)

### New Features

- <csr-id-193ffcd9945891867a14aa5b6f87e27d3880b280/> Add `rev parse --reference`.
  It's similar to `git rev-parse --symbolic-full-name`.

### Bug Fixes (BREAKING)

- <csr-id-59b8104a5320d946abc9f5736fa76696cef1459d/> mark `gix::interrupt::init_handler()` as unsafe
  The passed `interrupt()` argument will be called from a signal
  handler, so that needs to be documented and the call sites need to
  state that they fulfill the contract.
  
  Thanks to @Manishearth for pointing this out.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 12 commits contributed to the release over the course of 21 calendar days.
- 23 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
  - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
  - Upgrade tabled and setup wordwrapping ([`a78caba`](https://github.com/GitoxideLabs/gitoxide/commit/a78caba1e70adfae7beb2b4f6a18b8124efba46c))
  - Upgrade `crosstermion`, `owo-colors`, `prodash` and `owo-colors` ([`29c3f93`](https://github.com/GitoxideLabs/gitoxide/commit/29c3f938664a0d721873bfb345eeba01b3c7593d))
  - Merge branch 'tracking-branch' ([`0fe20e8`](https://github.com/GitoxideLabs/gitoxide/commit/0fe20e80145419e1662f869657dabf689786395f))
  - Add `rev parse --reference`. ([`193ffcd`](https://github.com/GitoxideLabs/gitoxide/commit/193ffcd9945891867a14aa5b6f87e27d3880b280))
  - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
  - Merge branch 'push-yvzxzqrkkvry' ([`4917beb`](https://github.com/GitoxideLabs/gitoxide/commit/4917beb5760a9bafb75b59331b282f4d6dbb64f5))
  - Mark `gix::interrupt::init_handler()` as unsafe ([`59b8104`](https://github.com/GitoxideLabs/gitoxide/commit/59b8104a5320d946abc9f5736fa76696cef1459d))
  - Merge branch '32bit' ([`ff1542c`](https://github.com/GitoxideLabs/gitoxide/commit/ff1542cedf3072a8c7c493d454aef5cc61de6d4c))
  - Update `faster-hex` crate to latest version ([`b0bfd01`](https://github.com/GitoxideLabs/gitoxide/commit/b0bfd01e6c65cd9f6458a97bfe1218a604cd6507))
  - Update `prodash` to latest version ([`97f62cb`](https://github.com/GitoxideLabs/gitoxide/commit/97f62cb183cc36d6045c9320697bb0b4755d6f6b))

</details>

## 0.32.0 (2023-12-06)

### New Features

<csr-id-4aea9b097fb08e504cdfc4a7c3b7511a308dc074/>
<csr-id-c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c/>
<csr-id-8f795e8abf706a24fe104500bf15efaa2bc07b15/>
<csr-id-8dfbb4ba531d86894a59e2b85ec29cd3133c8c4d/>
<csr-id-f9ae1bc6b514b6bbcda2c672f7b7ccbd999356a0/>
<csr-id-886289f98003f935d774a40f6db746ae70649936/>

- <csr-id-cf51a4de2d06eb28435ef8e386131710003b6928/> `gix rev parse --format` to provide different versions of the same content.
  This only applies to blobs, but allows to obtain different versions of the same blob
  like:
  
  * what's stored in Git
* what would be checked out to the worktree
* what would be diffed
- <csr-id-f34f46a3895a157036b099d6663d8953567119e7/> Add `http-client-curl-rustls` (CLI) and `blocking-http-transport-curl-rustls` (lib) features to avoid openssl.
  That way, we should be able to avoid crashes on certain CI configurations.

### Bug Fixes

- <csr-id-b2300782ad09f17730fe2a5ba4938517d4e7ec16/> in `--trace` mode, greatly increase message-buffer size.
  That way, it's much less likely that messages will get lost
  due to being overwritten before they can be displayed every
  100ms or so.
- <csr-id-eab22bd5d56bf91ee0edfc7156023c159c7f65f9/> set binary name for completions

### Bug Fixes (BREAKING)

- <csr-id-2189cee47f99350b368390eaa2a01961bb77c250/> rename `GITOXIDE_*` environment variables to `GIX_#`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 32 commits contributed to the release.
- 54 days passed between releases.
- 11 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
  - Merge branch 'adjustments-for-cargo' ([`8156340`](https://github.com/GitoxideLabs/gitoxide/commit/8156340724b1b7cb15824f88c75f6ddd7302cff5))
  - Rename `GITOXIDE_*` environment variables to `GIX_#` ([`2189cee`](https://github.com/GitoxideLabs/gitoxide/commit/2189cee47f99350b368390eaa2a01961bb77c250))
  - Merge branch 'gix-status' ([`5fdc9df`](https://github.com/GitoxideLabs/gitoxide/commit/5fdc9df069f3d9a4bd88e4e0ca5d67916e2908c9))
  - `gix rev parse --format` to provide different versions of the same content. ([`cf51a4d`](https://github.com/GitoxideLabs/gitoxide/commit/cf51a4de2d06eb28435ef8e386131710003b6928))
  - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
  - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
  - Adapt to changes in `gix-diff` ([`1706e23`](https://github.com/GitoxideLabs/gitoxide/commit/1706e2394380c35cd98d0e106eb0985ae1912da0))
  - Add the`diff::resource_cache()` low-level utility for rapid in-memory diffing of combinations of resources. ([`4aea9b0`](https://github.com/GitoxideLabs/gitoxide/commit/4aea9b097fb08e504cdfc4a7c3b7511a308dc074))
  - Merge branch 'support_ssl_verify' ([`5ce9784`](https://github.com/GitoxideLabs/gitoxide/commit/5ce978432231e257ef625fc401895b34f963bf6d))
  - In gix read http.sslVerify config value and pass it to gix-transport. ([`c6e83cf`](https://github.com/GitoxideLabs/gitoxide/commit/c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c))
  - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
  - Remove dead code from non-existent "lean-cli" feature ([`be1fa2f`](https://github.com/GitoxideLabs/gitoxide/commit/be1fa2f6370501c6b8bbe9bb89f6ef5507b5f425))
  - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
  - Merge branch 'feat_basic_connectivity_check' ([`1f9aca5`](https://github.com/GitoxideLabs/gitoxide/commit/1f9aca5de45c1f7c25606cd1ddc6b93a915dcd77))
  - Flatten `fsck connectivity` into just `fsck` much like `git fsck` ([`7ab5c76`](https://github.com/GitoxideLabs/gitoxide/commit/7ab5c7625fcbfade21d4f754be7afd2f73a828ba))
  - Refactor ([`7a88b42`](https://github.com/GitoxideLabs/gitoxide/commit/7a88b420f0a43fb1ce163698723b30566add97a9))
  - Add basic connectivity check ([`8f795e8`](https://github.com/GitoxideLabs/gitoxide/commit/8f795e8abf706a24fe104500bf15efaa2bc07b15))
  - Merge branch 'fix-1096' ([`48ef17e`](https://github.com/GitoxideLabs/gitoxide/commit/48ef17e4d169f2b08d119c2dba51fcfd9c06fe61))
  - Allow to print a tree without prettification, using `--tree-style raw`. ([`8dfbb4b`](https://github.com/GitoxideLabs/gitoxide/commit/8dfbb4ba531d86894a59e2b85ec29cd3133c8c4d))
  - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
  - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
  - Merge branch 'trace-packetlines' ([`e7de4c7`](https://github.com/GitoxideLabs/gitoxide/commit/e7de4c702a223ad9eb19b407391028dcb08d80c4))
  - Automatically enforce strict mode if `-c` options are given on the command-line. ([`f9ae1bc`](https://github.com/GitoxideLabs/gitoxide/commit/f9ae1bc6b514b6bbcda2c672f7b7ccbd999356a0))
  - In `--trace` mode, greatly increase message-buffer size. ([`b230078`](https://github.com/GitoxideLabs/gitoxide/commit/b2300782ad09f17730fe2a5ba4938517d4e7ec16))
  - Merge branch 'discover-split-worktree' ([`16170d9`](https://github.com/GitoxideLabs/gitoxide/commit/16170d9c2e4de6a2e639ff99b75e65bbd0e782d7))
  - Add `gix free discover` to inform about repository discovery. ([`886289f`](https://github.com/GitoxideLabs/gitoxide/commit/886289f98003f935d774a40f6db746ae70649936))
  - Merge branch 'fuzz' ([`c5a7e66`](https://github.com/GitoxideLabs/gitoxide/commit/c5a7e66d901868237ef5a4f86534b9878cc397ff))
  - Add `http-client-curl-rustls` (CLI) and `blocking-http-transport-curl-rustls` (lib) features to avoid openssl. ([`f34f46a`](https://github.com/GitoxideLabs/gitoxide/commit/f34f46a3895a157036b099d6663d8953567119e7))
  - Merge branch 'fix_completion-bin-name' ([`fb81093`](https://github.com/GitoxideLabs/gitoxide/commit/fb81093684a7ff3d9979ef21c2c8f771b6f99a50))
  - Refactor ([`7f2bc08`](https://github.com/GitoxideLabs/gitoxide/commit/7f2bc08e5713758c6188428221aac518cc6ecf2f))
  - Set binary name for completions ([`eab22bd`](https://github.com/GitoxideLabs/gitoxide/commit/eab22bd5d56bf91ee0edfc7156023c159c7f65f9))

</details>

## 0.31.1 (2023-10-13)

A re-release to deal with breakage in the dependency tree (self-caused).

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release.
- 1 day passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-transport v0.38.0, gix-protocol v0.41.1, gix v0.55.2, gitoxide-core v0.33.1, gitoxide v0.31.1 ([`1955a57`](https://github.com/GitoxideLabs/gitoxide/commit/1955a57f003f7d731d04e582e70ea86f15e8e7d9))
  - Improve documentation slightly ([`b7c9275`](https://github.com/GitoxideLabs/gitoxide/commit/b7c92753f52e71f118b6e21ab672ae56961306c5))
  - Release gix v0.55.1 ([`4642c0c`](https://github.com/GitoxideLabs/gitoxide/commit/4642c0c78f45b1956837bc874f6757fc302bee4a))

</details>

## 0.31.0 (2023-10-12)

This release includes an overhaul of the url parsing implementation, allowing more urls to be parsed correctly and refining the error messages for
invalid ones. Improvements include the added support for ssh aliases (`github:byron/gitoxide` has previously been parsed as local path), adjustments
around the interpretation of colons in file names (previously we disallowed colons that were not followed up with a slash character) and some smaller
changes that bring the interpretation of file urls more in line with Git's implementation.

There are still many (edge) cases in Git's url parsing implementation which are not handled correctly by Gitoxide. If you notice any such deviation please
open a new issue to help us making Gitoxide even more correct.

### New Features

- <csr-id-70923a0fad3d409069bd9d0b492e6a9b7b113261/> add shell completions for bash
- <csr-id-46e591914d548bacae2656ffe14a0ea7ca2eb7ae/> `gix status` auto-writes changed indices.
  This prevents expensive operations to re-occour.
- <csr-id-7ba2fa1c7781230913b0a04ad8684fa7d0143c18/> `gix status -s/--statistics` to obtain additional information on what happened.
  This is useful for understanding performance characteristics in detail.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 11 commits contributed to the release over the course of 17 calendar days.
- 18 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
  - Merge branch 'feat_completions' ([`681c607`](https://github.com/GitoxideLabs/gitoxide/commit/681c60774704a3c1d3ed8de916078833cf0398a7))
  - Refactor ([`fdd2df8`](https://github.com/GitoxideLabs/gitoxide/commit/fdd2df8e42f2a14a90e9a575e19811f25ab62237))
  - User decides completion output ([`215c3ac`](https://github.com/GitoxideLabs/gitoxide/commit/215c3acfd25554b9c2259d069f3916a9d6d911f5))
  - Undo formatting changes ([`6c913bf`](https://github.com/GitoxideLabs/gitoxide/commit/6c913bfa36a2b66dc305c8542fe7a5e072f1efdb))
  - Add shell completions for bash ([`70923a0`](https://github.com/GitoxideLabs/gitoxide/commit/70923a0fad3d409069bd9d0b492e6a9b7b113261))
  - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
  - Trust Ctime again ([`f929d42`](https://github.com/GitoxideLabs/gitoxide/commit/f929d420cb768f2df1d7886564ca03b3c3254a82))
  - `gix status` auto-writes changed indices. ([`46e5919`](https://github.com/GitoxideLabs/gitoxide/commit/46e591914d548bacae2656ffe14a0ea7ca2eb7ae))
  - `gix status -s/--statistics` to obtain additional information on what happened. ([`7ba2fa1`](https://github.com/GitoxideLabs/gitoxide/commit/7ba2fa1c7781230913b0a04ad8684fa7d0143c18))
  - Release gix v0.54.1 ([`f603fd7`](https://github.com/GitoxideLabs/gitoxide/commit/f603fd7a68206a6989a9f959216eba6cca0a6733))

</details>

## 0.30.0 (2023-09-24)

<csr-id-d3ac691446c9d029eb4f04d111887fa06720939d/>
<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

This release contains a security fix which assure URLs like `'ssh://-oProxyCommand=open$IFS-aCalculator/foo'` will not execute attacker-controlled
strings. The above can, on MacOS, launch the calculator app when using it with `gix clone` for example, up to v0.29.0.

### New Features (BREAKING)

- <csr-id-24dd870919ba444aa8099c63a78ea120d47ec28e/> use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress`

### New Features

- <csr-id-f094f71dc1a50955552509d108556c01517c6ed6/> `gix status` with basic index-worktree comparison
- <csr-id-3ff5ac0cda9e3e089dc79fdfbff5ff619ee60b1f/> `gix free index from-list` and `gix index from-tree` gain `--skip-hash`.
  This flag can be derived from options, but thus far we have no higher-level
  writing of the index so this has to do to see the difference in performance.

### Bug Fixes (BREAKING)

- <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
  This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 29 commits contributed to the release over the course of 33 calendar days.
- 33 days passed between releases.
- 6 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#987](https://github.com/GitoxideLabs/gitoxide/issues/987), [#992](https://github.com/GitoxideLabs/gitoxide/issues/992)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#987](https://github.com/GitoxideLabs/gitoxide/issues/987)**
  - Use a multi-call binary ('uni') to have only one build step ([`4ef9a32`](https://github.com/GitoxideLabs/gitoxide/commit/4ef9a32307a5e91868161a5f683b6b97ad9ebbdd))
- **[#992](https://github.com/GitoxideLabs/gitoxide/issues/992)**
  - Both `ein` and `gix` now share some code via the `gitoxide` library. ([`d3ac691`](https://github.com/GitoxideLabs/gitoxide/commit/d3ac691446c9d029eb4f04d111887fa06720939d))
- **Uncategorized**
  - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
  - Merge branch 'reset' ([`54a8495`](https://github.com/GitoxideLabs/gitoxide/commit/54a849545140f7f1c0c7564c418071c0a76a34e7))
  - `gix status` with basic index-worktree comparison ([`f094f71`](https://github.com/GitoxideLabs/gitoxide/commit/f094f71dc1a50955552509d108556c01517c6ed6))
  - Merge branch 'optimize/progress-use' ([`1f2ffb6`](https://github.com/GitoxideLabs/gitoxide/commit/1f2ffb6d86ef073caf43a2f7a77fe712a1aa495e))
  - Bump dep prodash to 26.2.2 for trait object bugfix ([`46d0890`](https://github.com/GitoxideLabs/gitoxide/commit/46d0890a9aee1c2ab126614a5054a5c66293a2cc))
  - Fix `prodash` dep on CI: Use `git` dep for now ([`59ccbe3`](https://github.com/GitoxideLabs/gitoxide/commit/59ccbe380f6f4851cd9f077c4dc7b3befeb619c5))
  - Use trait object for `progress` in `PrepareFetch::fetch_only` ([`70989b3`](https://github.com/GitoxideLabs/gitoxide/commit/70989b3965077ae00ec6cf344f31627a804a8225))
  - Release gix v0.53.1 ([`1b1fc25`](https://github.com/GitoxideLabs/gitoxide/commit/1b1fc257d5748c7c41e899bf2d1447ffd9f22d19))
  - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
  - Merge branch 'optimizations' ([`6135a5e`](https://github.com/GitoxideLabs/gitoxide/commit/6135a5ea8709646f01da62939a59dd3a9750e007))
  - Adapt to changes in `gix` ([`805b8aa`](https://github.com/GitoxideLabs/gitoxide/commit/805b8aa74b064b7aa08d87094a994bb8c7aae6ed))
  - Merge branch 'feat/gix-momo' ([`a1ed6a1`](https://github.com/GitoxideLabs/gitoxide/commit/a1ed6a1aacae02a167b7ec44e1a47411a2194ff7))
  - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
  - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
  - Use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress` ([`24dd870`](https://github.com/GitoxideLabs/gitoxide/commit/24dd870919ba444aa8099c63a78ea120d47ec28e))
  - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
  - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
  - Release gix-date v0.7.4, gix-index v0.23.0, safety bump 5 crates ([`3be2b1c`](https://github.com/GitoxideLabs/gitoxide/commit/3be2b1ccfe30eeae45711c64b88efc522a2b51b7))
  - Merge branch 'fixes' ([`4bfd1cc`](https://github.com/GitoxideLabs/gitoxide/commit/4bfd1cc8f7922a8c4de6b9d078d54b93e78f51ff))
  - Thanks clippy ([`0d6d4ec`](https://github.com/GitoxideLabs/gitoxide/commit/0d6d4ec8030d2e8f4c7a9d6f421d54776c4b67fb))
  - Adapt to changes in `gix-index` and pass skip-hash through for performance.. ([`713cd59`](https://github.com/GitoxideLabs/gitoxide/commit/713cd59f0b1eff6397b80f1e1fceec278532fd59))
  - `gix free index from-list` and `gix index from-tree` gain `--skip-hash`. ([`3ff5ac0`](https://github.com/GitoxideLabs/gitoxide/commit/3ff5ac0cda9e3e089dc79fdfbff5ff619ee60b1f))
  - Add more configuration variables prior to potentially using them; remove `index.skipHash` ([`773b6e3`](https://github.com/GitoxideLabs/gitoxide/commit/773b6e3d5b69f819805fc77e963ec2a059bf40be))
  - Add new crate `gix-macros` ([`6dae9e9`](https://github.com/GitoxideLabs/gitoxide/commit/6dae9e9d455ba5e3bf18c452789d07ff6cfaf392))
  - Merge branch 'remove-cargo-smart-release' ([`12699d7`](https://github.com/GitoxideLabs/gitoxide/commit/12699d77066963a8cba0184e107bcaa47187f657))
  - Remove remaining mentions of cargo-smart-release ([`fb8fa13`](https://github.com/GitoxideLabs/gitoxide/commit/fb8fa13931000480e19b1a3801ebcb95658a81f9))
  - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))

</details>

## 0.29.0 (2023-08-22)

### New Features (BREAKING)

- <csr-id-d9e5414c7864a3f5a9d108cc81caaa1998311a37/> use `crates-index` with https://github.com/frewsxcv/rust-crates-index/pull/129 applied.
  This means `git2` is now fully removed from the tools used by `gitoxide`.
  
  Note that this also means that the `vendored-ssl` feature has been removed as there is no equivalent.
  It might be worth to add feature toggles that change to another backend though.

### Chore

- <csr-id-01c6ef62e260246f1623bdf05f49c03eff14ac69/> reorganize tests to get rid of feature toggles that are for internal testing only
- <csr-id-93feea269eebd114e866e6f29f4a73c0096df9e0/> split tests off into their own crate to allow feature toggles.
  That way we can test with the `parallel` feature and won't have to
  create bogus feature toggles that are only used for testing, yet visbible
  to users.
- <csr-id-9fe7b6d8bec1a408bd73c75acccc4df753824382/> assure serde_derive still gets compiled from source

### New Features

- <csr-id-1ccbe16117d58b68b25a8e3e66676a61a07dd49c/> `gix submodule` subcommand for simple submodule listing and information retrieval
- <csr-id-6bc69e37deb3ebd88af8b0e8ffb9661562dcc679/> `gix index entries --recurse-subomdules` to also list submodules.
- <csr-id-c30ac0c9f89fb4480c8f8c8c0c06fa046dcd4314/> `gix index entries` with styles and pathspecs.
  This adds support for more simple git style, which is faster and thus
  allows for more direct comparisons to `git ls-files`.
- <csr-id-f194cfc920cc8622215bab5aa6b7b87e4fb9d7b2/> use real pathspecs where it was supported before.
- <csr-id-cd6cfe4049bacd8d8a691668b5501a673867f149/> add `gix commit verify-signature` to do something akin to `git ... --show-signature`.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 33 commits contributed to the release over the course of 30 calendar days.
- 31 days passed between releases.
- 9 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
  - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
  - Reorganize tests to get rid of feature toggles that are for internal testing only ([`01c6ef6`](https://github.com/GitoxideLabs/gitoxide/commit/01c6ef62e260246f1623bdf05f49c03eff14ac69))
  - Split tests off into their own crate to allow feature toggles. ([`93feea2`](https://github.com/GitoxideLabs/gitoxide/commit/93feea269eebd114e866e6f29f4a73c0096df9e0))
  - `gix submodule` subcommand for simple submodule listing and information retrieval ([`1ccbe16`](https://github.com/GitoxideLabs/gitoxide/commit/1ccbe16117d58b68b25a8e3e66676a61a07dd49c))
  - Just fmt ([`0d258f4`](https://github.com/GitoxideLabs/gitoxide/commit/0d258f40afcd848509e2b0c7c264e9f346ed1726))
  - Merge branch 'submodule-in-gix' ([`36f7b78`](https://github.com/GitoxideLabs/gitoxide/commit/36f7b783c67b8a087076a130f5ee9b90b23bc3cc))
  - `gix index entries --recurse-subomdules` to also list submodules. ([`6bc69e3`](https://github.com/GitoxideLabs/gitoxide/commit/6bc69e37deb3ebd88af8b0e8ffb9661562dcc679))
  - Bring back serde without precompiled binaries ([`8fd7949`](https://github.com/GitoxideLabs/gitoxide/commit/8fd79497bf52dd8cdf3b6f2c2c6b02a60571d191))
  - Assure serde_derive still gets compiled from source ([`9fe7b6d`](https://github.com/GitoxideLabs/gitoxide/commit/9fe7b6d8bec1a408bd73c75acccc4df753824382))
  - Merge branch 'worktree-organization' ([`8d0d8e0`](https://github.com/GitoxideLabs/gitoxide/commit/8d0d8e005d7f11924a6717954d892aae5cec45e7))
  - Move worktree- checkout functionality into its own crate ([`bd961b3`](https://github.com/GitoxideLabs/gitoxide/commit/bd961b3065ca71ac4fa59e9988a3b7e705cd4c67))
  - Create new `gix-status` crate to capture `git-status` like functionality ([`be9af32`](https://github.com/GitoxideLabs/gitoxide/commit/be9af327c75d693658a2427ee9a711e631a8da7d))
  - Merge branch 'submodule-active' ([`a3afaa4`](https://github.com/GitoxideLabs/gitoxide/commit/a3afaa42741616a0f1abeef9b54557e7c2b800cb))
  - Adapt to changes in `gix` ([`ca972a2`](https://github.com/GitoxideLabs/gitoxide/commit/ca972a244c9b074a2dc89ff3bbf83df0c3853904))
  - Use `Defaults::from_environment()` when parsing pathspecs. ([`2a99034`](https://github.com/GitoxideLabs/gitoxide/commit/2a99034bf451b1a801650b80705c96ce4c9e61c1))
  - Optimize startup times of `gix index entries` ([`22477bd`](https://github.com/GitoxideLabs/gitoxide/commit/22477bdcf37419627dbb7afe0df166935825ad2b))
  - Merge branch 'pathspec-matching' ([`9f4dfe0`](https://github.com/GitoxideLabs/gitoxide/commit/9f4dfe0f0b948280692916b596923959ea2fd9da))
  - `gix index entries` with styles and pathspecs. ([`c30ac0c`](https://github.com/GitoxideLabs/gitoxide/commit/c30ac0c9f89fb4480c8f8c8c0c06fa046dcd4314))
  - Use real pathspecs where it was supported before. ([`f194cfc`](https://github.com/GitoxideLabs/gitoxide/commit/f194cfc920cc8622215bab5aa6b7b87e4fb9d7b2))
  - Merge branch 'handlers-mt' ([`f584d76`](https://github.com/GitoxideLabs/gitoxide/commit/f584d7698d93836daef2000fd369034de46037f0))
  - Adapt to changes in `gix` ([`a201f0d`](https://github.com/GitoxideLabs/gitoxide/commit/a201f0d45936826ef26ac815acb9d2efbb3d5c68))
  - Merge branch 'extract-signatures' ([`b37affe`](https://github.com/GitoxideLabs/gitoxide/commit/b37affefecfb30a94431cd21dae6659004ca6244))
  - Add `gix commit verify-signature` to do something akin to `git ... --show-signature`. ([`cd6cfe4`](https://github.com/GitoxideLabs/gitoxide/commit/cd6cfe4049bacd8d8a691668b5501a673867f149))
  - Merge branch 'archive-gz' ([`c7d9129`](https://github.com/GitoxideLabs/gitoxide/commit/c7d912917a2dad5c076d0bd645cfda092c66ff79))
  - Adapt to changes in `gix-archive` ([`feba76d`](https://github.com/GitoxideLabs/gitoxide/commit/feba76d636e3d177c41f120639980f8b46edfcaa))
  - Merge branch 'submodules' ([`b629f8a`](https://github.com/GitoxideLabs/gitoxide/commit/b629f8a774931d58c0a9b124fa75f85807c6c5d1))
  - `.gitmodule` file abstraction ([`6a2e6a4`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e6a436f76c8bbf2487f9967413a51356667a0))
  - Update `time` crate explicitly in Cargo.toml to latest version ([`e145a74`](https://github.com/GitoxideLabs/gitoxide/commit/e145a7489dd5e1a7c3458428ecbd101e7b53536b))
  - Release gix-actor v0.24.2, gix-object v0.33.2, gix-ref v0.33.3, gix-config v0.26.2, gix-prompt v0.5.5, gix-odb v0.50.2, gix-transport v0.34.2, gix-protocol v0.37.0, gix-worktree v0.23.1, gix v0.51.0, safety bump 3 crates ([`231ac1c`](https://github.com/GitoxideLabs/gitoxide/commit/231ac1c6ad5ca9a84dbeb0dee14bfbf2fef1ae1e))
  - Release gix-archive v0.2.1, gix-ref v0.33.2, gix-pack v0.40.2, gix v0.50.1 ([`13883e5`](https://github.com/GitoxideLabs/gitoxide/commit/13883e5528385f892ee402e911298121e0c297c0))
  - Merge branch 'fetch-head-tests' ([`eb2b513`](https://github.com/GitoxideLabs/gitoxide/commit/eb2b513bd939f6b59891d0a4cf5465b1c1e458b3))
  - Use `crates-index` with https://github.com/frewsxcv/rust-crates-index/pull/129 applied. ([`d9e5414`](https://github.com/GitoxideLabs/gitoxide/commit/d9e5414c7864a3f5a9d108cc81caaa1998311a37))

</details>

## 0.28.0 (2023-07-22)

### New Features

- <csr-id-32bbb8b7b1f195adf7e5f06fd2ddc19153516a2f/> add simple CLI for `gix archive`
- <csr-id-4ee285741e6e1cde3a967980fbf48bab20ddbf68/> optionally make `gix-workspace-stream` available via `Repository::worktree_stream()`
  That way it's easy to obtain a representation of the worktree
  in a fully streaming fashion, which is also the basis for
  `archive`-like functionality.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 17 commits contributed to the release over the course of 29 calendar days.
- 30 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
  - Merge branch 'gix-archive' ([`1dda48b`](https://github.com/GitoxideLabs/gitoxide/commit/1dda48ba2fccb93ebac00fe3460e923af43c86ce))
  - Allow to create additional entries via the command-line ([`4a9d0f1`](https://github.com/GitoxideLabs/gitoxide/commit/4a9d0f1df6f7a20f7f1769fd9c7dae2e53f7e83f))
  - Add compression support to `gix archive`, which is where it should shine. ([`567b1a4`](https://github.com/GitoxideLabs/gitoxide/commit/567b1a4488c43c1f7099435d10cdddbc3a98a5cc))
  - Add simple CLI for `gix archive` ([`32bbb8b`](https://github.com/GitoxideLabs/gitoxide/commit/32bbb8b7b1f195adf7e5f06fd2ddc19153516a2f))
  - Optionally make `gix-workspace-stream` available via `Repository::worktree_stream()` ([`4ee2857`](https://github.com/GitoxideLabs/gitoxide/commit/4ee285741e6e1cde3a967980fbf48bab20ddbf68))
  - Create the new `gix-worktree-stream` crate from what was `gix-archive`. ([`9a157ae`](https://github.com/GitoxideLabs/gitoxide/commit/9a157ae0a4f649dacd911ccfb50facd942b992f0))
  - Release gix-prompt v0.5.3, gix v0.49.1, cargo-smart-release v0.20.0 ([`f069852`](https://github.com/GitoxideLabs/gitoxide/commit/f0698522940c9ba4d45db5a44dce9f21ca29cb4e))
  - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
  - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
  - Add keys required to deal with worktree conversions and filters. ([`3fbd7b0`](https://github.com/GitoxideLabs/gitoxide/commit/3fbd7b0c864cf2f1a38ae24e85d47b0b26b271a7))
  - Merge branch 'filter-programs' ([`97f8e96`](https://github.com/GitoxideLabs/gitoxide/commit/97f8e960ed52538bb55b72f9dfc5f9d144d72885))
  - A duplicate of the `gix-packetline` crate that mirrors it, but pre-selects the `blocking-io` feature. ([`2d83d88`](https://github.com/GitoxideLabs/gitoxide/commit/2d83d8826bf3ebc6095288bd9770e338099a5017))
  - Release gix-features v0.31.1, gix-path v0.8.3, gix v0.48.0 ([`9ca3464`](https://github.com/GitoxideLabs/gitoxide/commit/9ca346462806671fbc49643a87cea25ab0da3be8))
  - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
  - Merge branch 'basic-filtering' ([`3fd5e16`](https://github.com/GitoxideLabs/gitoxide/commit/3fd5e16e205db18edc21341fb4c2a75d0726f5a5))
  - Add some configuration variables as planned due to relevancy with filters ([`fa04378`](https://github.com/GitoxideLabs/gitoxide/commit/fa0437899292e9e23db932041220bbeda732c246))

</details>

## 0.27.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

This release fixes a possibility for V1 fetches to deadlock if negotiation would span multiple rounds 
(see issue [#882](https://github.com/Byron/gitoxide/issues/882)) for details.

### New Features (BREAKING)

- <csr-id-b82edc8b965344a866039f6c99295db4f8e776e2/> list commit-graph entries by graph traversal, move commit-graph up to `gix` level.
  This is merely a debug tool to learn about generation numbers.
  All commit-graph commands now operate on a repository.

### New Features

- <csr-id-c494cfd6f80cf273107c626d0f2cbe054db378ee/> `gix --trace` to also print tree-like instrumentation
- <csr-id-452ed6b79ed0e12ae214af3c8cd49a87dba91c73/> `gix fetch --open-negotiation-graph[=limit]`
  Open the negotiation graph as SVG, after optionally specifying a limit
  as rendering/layouting can be very slow.
  
  It's useful to see how the negotiation algorithm is reasoning about each commit.
- <csr-id-096838ffa529148b2da3a2382a2d1fdbfe5bee5b/> `gix fetch --negotiation-info` to provide additional information about the negotiation phase.
- <csr-id-bd32e393fedd01e27a4f6984281bcc3182c63b67/> `bit revision list --svg` to create a visual graph of commits.
  It's mainly a test of how well `layout-rs` performs.
- <csr-id-093efafa7c39aa03bfef4894779cca6e3716f471/> add `tracing` feature toggle to provide minimal tracing API
  This API is based on `tracing-core`, not on tracing, and provides a limited
  API that is always available, while being a no-op if `tracing` isn't enabled.
  
  That way, plumbing crates can add instrumentation at will.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 32 commits contributed to the release over the course of 12 calendar days.
- 15 days passed between releases.
- 7 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Make `gitoxide` publishable again ([`c16b1df`](https://github.com/GitoxideLabs/gitoxide/commit/c16b1dfda63f08c40ebab22b0a07e4617765acf5))
  - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
  - Merge branch 'gix-corpus' ([`5861afb`](https://github.com/GitoxideLabs/gitoxide/commit/5861afb45f32c16eefcd8e7b7480309bf44b6edc))
  - Add `corpus --dry-run` and `--task-sql-suffix` and `--repo-sql-suffix` ([`4cef57d`](https://github.com/GitoxideLabs/gitoxide/commit/4cef57db735c20da3fa1c56d9c1744e4f653bce0))
  - Gix-corpus now respects the --trace flag ([`0f973ac`](https://github.com/GitoxideLabs/gitoxide/commit/0f973ac53df631ad2abdf85dbe2453e528c7e6c3))
  - Adapt to changes in `gix-trace` ([`bd6e3d7`](https://github.com/GitoxideLabs/gitoxide/commit/bd6e3d7682d122997f8d3543eb87a62121ef0669))
  - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
  - Thanks clippy ([`361cf96`](https://github.com/GitoxideLabs/gitoxide/commit/361cf964f523fff5d92a8641bc3805b7e1082b18))
  - `gix --trace` to also print tree-like instrumentation ([`c494cfd`](https://github.com/GitoxideLabs/gitoxide/commit/c494cfd6f80cf273107c626d0f2cbe054db378ee))
  - Add `tracing` feature toggle to provide minimal tracing API ([`093efaf`](https://github.com/GitoxideLabs/gitoxide/commit/093efafa7c39aa03bfef4894779cca6e3716f471))
  - Run tasks in parallel ([`cfd8e88`](https://github.com/GitoxideLabs/gitoxide/commit/cfd8e88e428075701a162cf8da6986700fd8c5af))
  - Refresh a corpus repositories by updating all of them. ([`a0b4385`](https://github.com/GitoxideLabs/gitoxide/commit/a0b4385d33112fc3f950ac40d874883e7ad075c0))
  - Obtain a repository-list with classifiers ([`a4300c8`](https://github.com/GitoxideLabs/gitoxide/commit/a4300c8318ded98dc465405d39ed014e0534de66))
  - Provide all the meta-data that is needed to make a run (and associate it with that) ([`fcbda1d`](https://github.com/GitoxideLabs/gitoxide/commit/fcbda1d2fee735c8bf3e7357a8691af2f7637922))
  - A build script to set a `gitoxide` version according to what's in git ([`83f6466`](https://github.com/GitoxideLabs/gitoxide/commit/83f646674118ff6227a605dc17b0be7325481069))
  - A basic command to perform a corpus run ([`d9e74ff`](https://github.com/GitoxideLabs/gitoxide/commit/d9e74ffdedf585a68d09d04f493c98eda729ff3b))
  - Merge branch 'gix-revision-graph' ([`036e60a`](https://github.com/GitoxideLabs/gitoxide/commit/036e60a3ad39ba9b018c0b56454f12fad455c7bb))
  - `gix fetch --open-negotiation-graph[=limit]` ([`452ed6b`](https://github.com/GitoxideLabs/gitoxide/commit/452ed6b79ed0e12ae214af3c8cd49a87dba91c73))
  - `gix fetch --negotiation-info` to provide additional information about the negotiation phase. ([`096838f`](https://github.com/GitoxideLabs/gitoxide/commit/096838ffa529148b2da3a2382a2d1fdbfe5bee5b))
  - Make top-level `gix` docs available when using `cargo doc` ([`518159d`](https://github.com/GitoxideLabs/gitoxide/commit/518159da99cf07384e9cacdc1d62a5dc1e02f09c))
  - `bit revision list --svg` to create a visual graph of commits. ([`bd32e39`](https://github.com/GitoxideLabs/gitoxide/commit/bd32e393fedd01e27a4f6984281bcc3182c63b67))
  - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
  - Merge branch 'fix-commitgraph' ([`2213321`](https://github.com/GitoxideLabs/gitoxide/commit/22133210f2d073eb472c51c83f29b18cf8bdd7ca))
  - Bring back the no-repo commitgraph for stress-tests to work ([`ff8d42a`](https://github.com/GitoxideLabs/gitoxide/commit/ff8d42aef988b99153950a24efda80acebde5cac))
  - Move commit-graph implementation into its correct place ([`750b07a`](https://github.com/GitoxideLabs/gitoxide/commit/750b07a69c659cedd46f33396468d4faa7e73f10))
  - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
  - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
  - List commit-graph entries by graph traversal, move commit-graph up to `gix` level. ([`b82edc8`](https://github.com/GitoxideLabs/gitoxide/commit/b82edc8b965344a866039f6c99295db4f8e776e2))
  - Add options related to commitgraph to `gix config tree` ([`7aaaebf`](https://github.com/GitoxideLabs/gitoxide/commit/7aaaebfea9c1935f0a87ab81cef4261e00e4c159))
  - Release gix-attributes v0.13.1, gix-diff v0.30.1, gix-revwalk v0.1.0, gix-traverse v0.27.0, gix-index v0.18.0, gix-revision v0.15.2, gix-negotiate v0.2.1, gix-pack v0.37.0, gix-odb v0.47.0, gix-protocol v0.33.2, gix-worktree v0.19.0, gix v0.46.0, safety bump 7 crates ([`2560a2c`](https://github.com/GitoxideLabs/gitoxide/commit/2560a2cc3e1d8c60cd812e15696fa4761d036e19))
  - Merge branch 'walk-with-commitgraph' ([`fdee9a2`](https://github.com/GitoxideLabs/gitoxide/commit/fdee9a22873a13ae644d3dc92f8fe93f8f0266c0))
  - Add new `gix-revwalk` crate for support types related to revision walking. ([`13ce887`](https://github.com/GitoxideLabs/gitoxide/commit/13ce887682f5c31d1f78a63613ca97b811e4ffba))

</details>

## 0.26.0 (2023-06-07)

The main attraction of this release is support for multi-round negotiations thanks to support for the standard negotiations algorithms.
Further, `gix index entries` now lists attributes as well, and there is the new `gix attribute query` sub-command in the vain of `git check-attrs`.

### New Features

- <csr-id-e1fcc7f69e7b95aeac8cbbde3719a1e6b9dafeba/> `gix attributes validate` to validate attributes and ignore against `git` as baseline.
  Use this command to test the entire index for ignored paths and their attributes and use
  `git check-attr` and `git check-ignore` to validate that `git` agrees. Collect all
  mismatches and print them.
- <csr-id-450b2325909459c7061f3d02f461c3f0f20f1e86/> `--statistics` for `gix excludes query`
  With it one gets some insights into the work done to fulfil the query.
- <csr-id-bfdcb147cdc3d0338a4cd2bc915b0e0eeced1e99/> `gix attribute query` as something similar to `git check-attrs`.
- <csr-id-b8db2072bb6a5625f37debe9e58d08461ece67dd/> `no-repo index from-list` to create an index with empty files from the given list.
  The list could be created with `find . -type f` for example.
- <csr-id-13a0f1e568cf9ee462ecf01d416cfcd4949fd9c6/> add pro-forma library to allow feature documentation of `gitoxide` on docs.rs
- <csr-id-add3a0594de1f4fadcbdcee41926a8ffd32c0cb6/> support package builders to allow configuring certain C dependencies
  Namely, `zlib` and sha1 hashing.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 26 commits contributed to the release over the course of 34 calendar days.
- 41 days passed between releases.
- 6 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-revision v0.15.1, gix v0.45.1 ([`11766a0`](https://github.com/GitoxideLabs/gitoxide/commit/11766a0a82754fee9918ccdb8eaf92af6d2561ba))
  - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
  - `just fmt` ([`ffc1276`](https://github.com/GitoxideLabs/gitoxide/commit/ffc1276e0c991ac33ce842f5dca0b45ac69680c0))
  - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
  - Adapt to changes in `gix` ([`7983f6f`](https://github.com/GitoxideLabs/gitoxide/commit/7983f6fd4ef242d494962fe35b8b8f91a6135f32))
  - Keep track of core.alternateRefsCommand ([`d925896`](https://github.com/GitoxideLabs/gitoxide/commit/d9258969047a53bcb437b4ebd9467e6c95d715d7))
  - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
  - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
  - Merge branch 'consecutive-negotiation' ([`97b3f7e`](https://github.com/GitoxideLabs/gitoxide/commit/97b3f7e2eaddea20c98f2f7ab6a0d2e2117b0793))
  - Release gix-commitgraph v0.15.0, gix-revision v0.14.0, gix-negotiate v0.1.0, safety bump 7 crates ([`92832ca`](https://github.com/GitoxideLabs/gitoxide/commit/92832ca2899cd2f222f4c7b1cc9e766178f55806))
  - Add new crate for implementing and testing git negotiation logic. ([`372ba09`](https://github.com/GitoxideLabs/gitoxide/commit/372ba09bb00e3fab674f0251f697aab11c5559f8))
  - Merge branch 'consecutive-negotiation' ([`4507f94`](https://github.com/GitoxideLabs/gitoxide/commit/4507f94984c811ea098e43472e5f54ec4dbb90c1))
  - Make clear that we do handle shallow repos, refactor tests ([`fc423e4`](https://github.com/GitoxideLabs/gitoxide/commit/fc423e470de50491a725d4802066d26c05bd2b2a))
  - Merge branch 'fix-851' ([`2f275d5`](https://github.com/GitoxideLabs/gitoxide/commit/2f275d5d3cb49b3b8ba53b30e4b4386fac32662b))
  - Adjust to changes in `gix-pack` ([`215889c`](https://github.com/GitoxideLabs/gitoxide/commit/215889ceb976a59368c132aabfffb71a6a2ac9f8))
  - Support reading the fetch algorithm from configuration ([`33b7770`](https://github.com/GitoxideLabs/gitoxide/commit/33b777074db21db8cd060ecf8cfdac0409a7e10c))
  - Merge branch 'gix-attributes-validate' ([`a849da8`](https://github.com/GitoxideLabs/gitoxide/commit/a849da8e35ca14fef9a2431fe1bb1c05b249680e))
  - `gix attributes validate` to validate attributes and ignore against `git` as baseline. ([`e1fcc7f`](https://github.com/GitoxideLabs/gitoxide/commit/e1fcc7f69e7b95aeac8cbbde3719a1e6b9dafeba))
  - Merge branch 'gix-attribute-query' ([`1d70213`](https://github.com/GitoxideLabs/gitoxide/commit/1d70213b5c41611c9a6172901b78844a76dece0e))
  - `--statistics` for `gix excludes query` ([`450b232`](https://github.com/GitoxideLabs/gitoxide/commit/450b2325909459c7061f3d02f461c3f0f20f1e86))
  - `gix attribute query` as something similar to `git check-attrs`. ([`bfdcb14`](https://github.com/GitoxideLabs/gitoxide/commit/bfdcb147cdc3d0338a4cd2bc915b0e0eeced1e99))
  - `no-repo index from-list` to create an index with empty files from the given list. ([`b8db207`](https://github.com/GitoxideLabs/gitoxide/commit/b8db2072bb6a5625f37debe9e58d08461ece67dd))
  - Merge branch 'fix-841' ([`edeb4de`](https://github.com/GitoxideLabs/gitoxide/commit/edeb4de2fa0f4e8ecab0c7f4af169d4443cf0eaf))
  - Support package builders to allow configuring certain C dependencies ([`add3a05`](https://github.com/GitoxideLabs/gitoxide/commit/add3a0594de1f4fadcbdcee41926a8ffd32c0cb6))
  - Add pro-forma libaray to allow feature documentation of `gitoxide` on docs.rs ([`13a0f1e`](https://github.com/GitoxideLabs/gitoxide/commit/13a0f1e568cf9ee462ecf01d416cfcd4949fd9c6))
  - Use unlimited parallelism for `jwalk` to have a better comparison. ([`d67551d`](https://github.com/GitoxideLabs/gitoxide/commit/d67551d4e43359990b4391ea2902eaf13631dd99))

</details>

## 0.25.0 (2023-04-27)

A maintenance release without user-facing changes, just to fix installation

### Commit Statistics

<csr-read-only-do-not-edit/>

- 1 commit contributed to the release.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Bump gix-path v0.8.0, safety bump 20 crates (gix set to 0.44.1 manually) ([`43ebaf2`](https://github.com/GitoxideLabs/gitoxide/commit/43ebaf267557218865862538ffc7bdf00558492f))

</details>

## 0.24.0 (2023-04-27)

### New Features

- <csr-id-08e8fc2152794652ba1c986df493c2ac915af9e7/> `gix index entries` also prints attributes.
- <csr-id-9723e1addf52cc336d59322de039ea0537cdca36/> `gix clone` and `gix fetch` with controls for shallow repositories.
- <csr-id-f93e964b1f7ba95861966ce58ebef477013dd1b4/> `ein t trace-path` know accepts the `trace-file` alias.

### Bug Fixes

- <csr-id-dd14a80a78fea3f092b1d5300637c80cfd9e59a7/> printing to stdout for commands that don't show progress is greatly improved.
  Previously it would have to lock `stdout` on each write, now this is done only once.
- <csr-id-7bd8823ab4241d6d0401f03aec8c0d34f68c347c/> opening repositories without 'strict' mode also ignores IO errors.
  These will instead be logged, but won't make it impossible to open an
  otherwise fine repository.

### New Features (BREAKING)

- <csr-id-ca8ebdfb9647ff15b0293823bb3bb3c6779c8dd2/> turn `gix free index entries` into `gix index entries`.
- <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
  With it it's possible to not automatically declare all optional dependencies externally visible
  features, and thus re-use feature names that oterwise are also a crate name.
  
  Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
  at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
  `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 45 commits contributed to the release.
- 62 days passed between releases.
- 7 commits were understood as [conventional](https://www.conventionalcommits.org).
- 3 unique issues were worked on: [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#790](https://github.com/GitoxideLabs/gitoxide/issues/790), [#814](https://github.com/GitoxideLabs/gitoxide/issues/814)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
  - Update crate status ([`ed63ec0`](https://github.com/GitoxideLabs/gitoxide/commit/ed63ec08ac651d642123d5abd1d4589e67049ff2))
- **[#790](https://github.com/GitoxideLabs/gitoxide/issues/790)**
  - Opening repositories without 'strict' mode also ignores IO errors. ([`7bd8823`](https://github.com/GitoxideLabs/gitoxide/commit/7bd8823ab4241d6d0401f03aec8c0d34f68c347c))
- **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
  - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
- **Uncategorized**
  - Merge branch 'index-entries-attrs' ([`f37a930`](https://github.com/GitoxideLabs/gitoxide/commit/f37a930aefa27e67f0b693ba9669cc26d49044fa))
  - `gix index entries` also prints attributes. ([`08e8fc2`](https://github.com/GitoxideLabs/gitoxide/commit/08e8fc2152794652ba1c986df493c2ac915af9e7))
  - Printing to stdout for commands that don't show progress is greatly improved. ([`dd14a80`](https://github.com/GitoxideLabs/gitoxide/commit/dd14a80a78fea3f092b1d5300637c80cfd9e59a7))
  - Turn `gix free index entries` into `gix index entries`. ([`ca8ebdf`](https://github.com/GitoxideLabs/gitoxide/commit/ca8ebdfb9647ff15b0293823bb3bb3c6779c8dd2))
  - Merge branch 'attributes-cache' ([`3456c84`](https://github.com/GitoxideLabs/gitoxide/commit/3456c845dfeedd2fa96b4313b1a84c8cbe9433c5))
  - Adjust to changes in `gix-worktree` ([`13a070f`](https://github.com/GitoxideLabs/gitoxide/commit/13a070f405230d52e4377e18f6bdc5c673b718a0))
  - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
  - Add information about configuration flags related to fsync usage ([`8d5762c`](https://github.com/GitoxideLabs/gitoxide/commit/8d5762c2031cc24bcea7eaba2911b907ba443b72))
  - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
  - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
  - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
  - Create new `gix-fs` crate to consolidate all filesystem utilities ([`f8cc33c`](https://github.com/GitoxideLabs/gitoxide/commit/f8cc33cb372dd2b4bbe4a09cf4f64916681ab1dd))
  - Merge branch 'main' into dev ([`23ee47f`](https://github.com/GitoxideLabs/gitoxide/commit/23ee47fb24c197f8437bd426544b2aa74e005bdc))
  - Merge branch 'worktree-stack' ([`3d47919`](https://github.com/GitoxideLabs/gitoxide/commit/3d47919c1a2f83fc7c1fd7ae590d098057a22626))
  - Assure we load all gitattributes when needed. ([`9237121`](https://github.com/GitoxideLabs/gitoxide/commit/923712175fffaa6d0b71e109f243bdd6bea3ff36))
  - Add new `gix-ignore` crate with the contents moved and adapted from `gix-attributes`. ([`fc28701`](https://github.com/GitoxideLabs/gitoxide/commit/fc28701b52139d9422dd73687d776cf95de98b35))
  - A new create for filesystem specific utilities ([`ef8f405`](https://github.com/GitoxideLabs/gitoxide/commit/ef8f405d06adf100bbe5f0b1fccbe8bb2c6fd650))
  - Release gix-path v0.7.3, gix-config-value v0.10.2, gix-config v0.20.1, gix-discover v0.16.2, gix-index v0.15.1, gix-odb v0.43.1, gix-packetline v0.15.1, gix-protocol v0.30.2, gix-worktree v0.15.2, gix v0.43.1 ([`38eed1d`](https://github.com/GitoxideLabs/gitoxide/commit/38eed1d06e7cbb8fbcd54b2cad3163ca45e0baf1))
  - Release gix-tempfile v5.0.2, gix-validate v0.7.4, gix-config v0.20.0, gix-prompt v0.3.3, gix-diff v0.28.1, gix-discover v0.16.1, gix-pack v0.33.2, gix-transport v0.29.1, gix-protocol v0.30.1, gix-revision v0.12.1, gix-worktree v0.15.1, gix v0.43.0, safety bump gix v0.43.0 ([`5dc1f9f`](https://github.com/GitoxideLabs/gitoxide/commit/5dc1f9f2bcb8b3e147115fcb6f76558e8f48ffef))
  - Merge branch 'fix-790' ([`ee36e5b`](https://github.com/GitoxideLabs/gitoxide/commit/ee36e5bb985e9ad90bc382cdd051a8b5295ca18c))
  - Add group headings to all shallow-related arguments. ([`bbb4cb0`](https://github.com/GitoxideLabs/gitoxide/commit/bbb4cb03ee2bcd0c5dd0635da542a942179cf6fd))
  - Add `gix-archive` as crate idea. ([`dec8c7c`](https://github.com/GitoxideLabs/gitoxide/commit/dec8c7c68212c5963aafd68c7ae9e9b4b446e997))
  - Keep track of `index.skipHash` ([`f29f35c`](https://github.com/GitoxideLabs/gitoxide/commit/f29f35cfc43f435f15aebc4d678f01d7f7ab7fbf))
  - Release gix-features v0.28.1, gix-tempfile v5.0.1, gix-ref v0.27.1, gix-pack v0.33.1, gix-packetline v0.15.0, gix-transport v0.29.0, gix-protocol v0.30.0, gix v0.42.0, safety bump 3 crates ([`c1f1bfb`](https://github.com/GitoxideLabs/gitoxide/commit/c1f1bfb8dc0e73993678353e4492d0614b642ed1))
  - Make fmt ([`3836cc0`](https://github.com/GitoxideLabs/gitoxide/commit/3836cc0c9c3e1158b56142b924483c8a77217d53))
  - Merge branch 'various-fixes' ([`cc0f506`](https://github.com/GitoxideLabs/gitoxide/commit/cc0f5061fba27d57022dc616c941034b98fd4875))
  - Fix clap derive declaration of conflicts ([`c502fa8`](https://github.com/GitoxideLabs/gitoxide/commit/c502fa8362cb449dc1837a0b8febfad03d9f66ef))
  - Merge branch 'shallow-protocol' ([`531dd19`](https://github.com/GitoxideLabs/gitoxide/commit/531dd19502b8b635fb1a60f747eb381fd12e00ca))
  - `gix clone` and `gix fetch` with controls for shallow repositories. ([`9723e1a`](https://github.com/GitoxideLabs/gitoxide/commit/9723e1addf52cc336d59322de039ea0537cdca36))
  - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
  - Release gix-tempfile v5.0.0, gix-lock v5.0.0, gix-ref v0.27.0, gix-config v0.19.0, gix-url v0.16.0, gix-credentials v0.12.0, gix-discover v0.16.0, gix-index v0.15.0, gix-pack v0.33.0, gix-odb v0.43.0, gix-transport v0.28.0, gix-protocol v0.29.0, gix-worktree v0.15.0, gix v0.41.0, safety bump 12 crates ([`29a0870`](https://github.com/GitoxideLabs/gitoxide/commit/29a087043d1feb2f127b065341c8028d0bd0301e))
  - Release gix v0.40.0 ([`18e72c9`](https://github.com/GitoxideLabs/gitoxide/commit/18e72c988a58415080d4555bc869ae04df8d04fa))
  - Release gix-features v0.28.0, gix-actor v0.19.0, gix-object v0.28.0, gix-diff v0.28.0, gix-traverse v0.24.0, gix-pack v0.32.0, safety bump 20 crates ([`0f411e9`](https://github.com/GitoxideLabs/gitoxide/commit/0f411e93ec812592bb9d3a52b751399dd86f76f7))
  - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
  - Merge branch 'adjustments-for-cargo' ([`d686d94`](https://github.com/GitoxideLabs/gitoxide/commit/d686d94e1030a8591ba074757d56927a346c8351))
  - Depend on latest version of `prodash` for performance improvements. ([`5d00324`](https://github.com/GitoxideLabs/gitoxide/commit/5d003242abe82b1604e2188d49dec9690ebb2a6a))
  - Adjust to changes in `gix` ([`49f656f`](https://github.com/GitoxideLabs/gitoxide/commit/49f656fa454f74dfa1e835b7d1602224fda8d112))
  - Prepare for git-tempfile release ([`56c005b`](https://github.com/GitoxideLabs/gitoxide/commit/56c005b13c44376f71e61781e73c0bf93416d0e4))
  - Improve error handling and reporting of `ein t query` ([`d5616b6`](https://github.com/GitoxideLabs/gitoxide/commit/d5616b66052cd20cbfabbecc1f5f500f754e89a2))
  - Make fmt ([`8ef1cb2`](https://github.com/GitoxideLabs/gitoxide/commit/8ef1cb293434c7b9e1fda4a6963368e0435920a9))
  - Merge branch 'rename-tracking' ([`3827ac6`](https://github.com/GitoxideLabs/gitoxide/commit/3827ac6c6b002219a7fb0877ec01d8afc909ca4c))
  - `ein t trace-path` know accepts the `trace-file` alias. ([`f93e964`](https://github.com/GitoxideLabs/gitoxide/commit/f93e964b1f7ba95861966ce58ebef477013dd1b4))

</details>

## 0.23.0 (2023-02-24)

This release adds the new `ein tool query` analytics engine, which maintains a database of a git repository as acceleration data structure to run useful queries on.
It's nothing more than an MVP, and was inspired by a program to accelerate [Git-Heat-Map](https://github.com/jmforsythe/Git-Heat-Map/pull/6).

### New Features

- <csr-id-f8cc62376bd6e940be4c41e84ccbfce111e64e39/> `ein tool query` - a git analytics engine.
  A tool to build and efficiently maintain a database of information contained
  in a git repository, preferably the kind of information that is expensive to obtain,
  in order to facilitate queries that would be prohibitive without an accelerating
  data structure.
- <csr-id-49520d1f03ee1d011f6e9f2eb07084d327fe95ae/> `gix tree entries` with rev-spec support.
  Previously it wanted a tree-id, now it can derive it itself.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 7 calendar days.
- 7 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-object v0.26.4, gix-diff v0.26.3, gix v0.37.2, gix-commitgraph v0.13.1, gitoxide-core v0.25.0, gitoxide v0.23.0 ([`9982949`](https://github.com/GitoxideLabs/gitoxide/commit/9982949cab401501d5ce3cba4e2ba900bc249c53))
  - Merge branch 'rename-tracking' ([`550144a`](https://github.com/GitoxideLabs/gitoxide/commit/550144a5fd37d501d86f4b1c4db2948d951d1c93))
  - `ein tool query` - a git analytics engine. ([`f8cc623`](https://github.com/GitoxideLabs/gitoxide/commit/f8cc62376bd6e940be4c41e84ccbfce111e64e39))
  - `gix tree entries` with rev-spec support. ([`49520d1`](https://github.com/GitoxideLabs/gitoxide/commit/49520d1f03ee1d011f6e9f2eb07084d327fe95ae))
  - Release gix-config v0.16.3, gix v0.37.1 ([`a3c283f`](https://github.com/GitoxideLabs/gitoxide/commit/a3c283ff0e3f21cedb3ba7cd464fdfa0f5133af0))
  - Release gix-object v0.26.3, gix-diff v0.26.2, gix-traverse v0.22.2, gix v0.37.0, safety bump 3 crates ([`8b3e42f`](https://github.com/GitoxideLabs/gitoxide/commit/8b3e42f69fe97fe5083eb845c323f10d7ac087b2))
  - Release gix-date v0.4.3, gix-hash v0.10.3, gix-features v0.26.5, gix-actor v0.17.2, gix-glob v0.5.5, gix-path v0.7.2, gix-quote v0.4.2, gix-attributes v0.8.3, gix-validate v0.7.3, gix-object v0.26.2, gix-ref v0.24.1, gix-config v0.16.2, gix-command v0.2.4, gix-url v0.13.3, gix-credentials v0.9.2, gix-discover v0.13.1, gix-index v0.12.4, gix-mailmap v0.9.3, gix-pack v0.30.3, gix-packetline v0.14.3, gix-transport v0.25.6, gix-protocol v0.26.4, gix-revision v0.10.4, gix-refspec v0.7.3, gix-worktree v0.12.3, gix v0.36.1 ([`9604783`](https://github.com/GitoxideLabs/gitoxide/commit/96047839a20a657a559376b0b14c65aeab96acbd))
  - Fix journey tests; improve panic handling when --progress is used. ([`571121c`](https://github.com/GitoxideLabs/gitoxide/commit/571121ccceaab25323bc6cd1ed2e2a1560289a57))

</details>

## 0.22.1 (2023-02-17)

### Bug Fixes

- <csr-id-73372d0bcdca8cf201f057ebee8d67a8e4752625/> re-enable local-time support for all binaries
  The `time` improved the way one can opt-in to potential unsoundness
  in their local time support, forcing the application to make that
  decision at runtime.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gitoxide v0.22.1 ([`e0a54c7`](https://github.com/GitoxideLabs/gitoxide/commit/e0a54c735ddc796a6cb30e387f12e411dc1cf6fc))
  - Re-enable local-time support for all binaries ([`73372d0`](https://github.com/GitoxideLabs/gitoxide/commit/73372d0bcdca8cf201f057ebee8d67a8e4752625))

</details>

## 0.22.0 (2023-02-17)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 57 commits contributed to the release.
- 8 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
  - Release git-date v0.4.3, git-hash v0.10.3, git-features v0.26.5, git-actor v0.17.2, git-glob v0.5.4, git-path v0.7.2, git-quote v0.4.2, git-attributes v0.8.3, git-bitmap v0.2.2, git-chunk v0.4.2, git-command v0.2.4, git-commitgraph v0.13.1, git-config-value v0.10.2, git-tempfile v3.0.3, git-lock v3.0.3, git-validate v0.7.3, git-object v0.26.2, git-ref v0.24.1, git-sec v0.6.3, git-config v0.16.2, git-prompt v0.3.3, git-url v0.13.3, git-credentials v0.9.2, git-diff v0.26.2, git-discover v0.13.1, git-fetchhead v0.1.0, git-filter v0.1.0, git-hashtable v0.1.2, git-traverse v0.22.2, git-index v0.12.4, git-lfs v0.1.0, git-mailmap v0.9.3, git-note v0.1.0, git-pack v0.31.0, git-odb v0.41.0, git-packetline v0.14.3, git-pathspec v0.1.0, git-transport v0.25.5, git-protocol v0.26.4, git-rebase v0.1.0, git-revision v0.10.4, git-refspec v0.7.3, git-sequencer v0.1.0, git-submodule v0.1.0, git-tix v0.1.0, git-tui v0.1.0, git-worktree v0.12.3, safety bump 2 crates ([`90035a3`](https://github.com/GitoxideLabs/gitoxide/commit/90035a332d0ba67584558db3605500fbcb424ddd))
  - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
  - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
  - Adjust to renaming of `git-index` to `gix-index` ([`86db5e0`](https://github.com/GitoxideLabs/gitoxide/commit/86db5e09fc58ce66b252dc13b8d7e2c48e4d5062))
  - Adjust to renaming of `git-diff` to `gix-diff` ([`49a163e`](https://github.com/GitoxideLabs/gitoxide/commit/49a163ec8b18f0e5fcd05a315de16d5d8be7650e))
  - Adjust to renaming of `git-commitgraph` to `gix-commitgraph` ([`f1dd0a3`](https://github.com/GitoxideLabs/gitoxide/commit/f1dd0a3366e31259af029da73228e8af2f414244))
  - Adjust to renaming of `git-mailmap` to `gix-mailmap` ([`2e28c56`](https://github.com/GitoxideLabs/gitoxide/commit/2e28c56bb9f70de6f97439818118d3a25859698f))
  - Adjust to renaming of `git-discover` to `gix-discover` ([`53adfe1`](https://github.com/GitoxideLabs/gitoxide/commit/53adfe1c34e9ea3b27067a97b5e7ac80b351c441))
  - Adjust to renaming of `git-fetchhead` to `gix-fetchhead` ([`0f536fb`](https://github.com/GitoxideLabs/gitoxide/commit/0f536fbd6b7d8a13823a8430026c1c62ad42ad01))
  - Rename `git-filter` to `gix-filter` ([`0875ae6`](https://github.com/GitoxideLabs/gitoxide/commit/0875ae61d9e2e7553ea990ea1c391a26f0eb1a59))
  - Adjust to renaming of `git-lfs` to `gix-lfs` ([`b9225c8`](https://github.com/GitoxideLabs/gitoxide/commit/b9225c830daf1388484ee7e05f727990fdeff43c))
  - Adjust to renaming of `git-note` to `gix-note` ([`9cc9ae2`](https://github.com/GitoxideLabs/gitoxide/commit/9cc9ae28102bca9c5eac4c7699be7029e7f7697f))
  - Adjust to renaming of `git-chunk` to `gix-chunk` ([`59194e3`](https://github.com/GitoxideLabs/gitoxide/commit/59194e3a07853eae0624ebc4907478d1de4f7599))
  - Adjust to renaming of `git-bitmap` to `gix-bitmap` ([`75f2a07`](https://github.com/GitoxideLabs/gitoxide/commit/75f2a079b17489f62bc43e1f1d932307375c4f9d))
  - Adjust to renaming for `git-protocol` to `gix-protocol` ([`823795a`](https://github.com/GitoxideLabs/gitoxide/commit/823795addea3810243cab7936cd8ec0137cbc224))
  - Adjust to renaming of `git-rebase` to `gix-rebase` ([`f00e531`](https://github.com/GitoxideLabs/gitoxide/commit/f00e53108f8a2de01f84fef35f9278b9114cf9dc))
  - Adjust to renaming of `git-refspec` to `gix-refspec` ([`c958802`](https://github.com/GitoxideLabs/gitoxide/commit/c9588020561577736faa065e7e5b5bb486ca8fe1))
  - Adjust to renaming of `git-revision` to `gix-revision` ([`ee0ee84`](https://github.com/GitoxideLabs/gitoxide/commit/ee0ee84607c2ffe11ee75f27a31903db68afed02))
  - Adjust to renaming of `git-sequencer` to `gix-sequencer` ([`eb07bda`](https://github.com/GitoxideLabs/gitoxide/commit/eb07bdaa9aec4617777f2ecb2c48a0dc5925f4c4))
  - Adjust to renaming of `git-submodule` to `gix-submodule` ([`2c4a2d0`](https://github.com/GitoxideLabs/gitoxide/commit/2c4a2d0113d5e60c50d2d72a1ff1cdf3e8ae77af))
  - Adjust to renaming of `git-tix` to `gix-tix` ([`531003b`](https://github.com/GitoxideLabs/gitoxide/commit/531003bb03dd83e8870643bbf114008a367c6599))
  - Adjust to renaming of `git-transport` to `gix-transport` ([`b2ccf71`](https://github.com/GitoxideLabs/gitoxide/commit/b2ccf716dc4425bb96651d4d58806a3cc2da219e))
  - Adjust to renaming of `git-credentials` to `gix-credentials` ([`6b18abc`](https://github.com/GitoxideLabs/gitoxide/commit/6b18abcf2856f02ab938d535a65e51ac282bf94a))
  - Adjust to renaming of `git-prompt` to `gix-prompt` ([`6a4654e`](https://github.com/GitoxideLabs/gitoxide/commit/6a4654e0d10ab773dd219cb4b731c0fc1471c36d))
  - Adjust to renaming of `git-command` to `gix-command` ([`d26b8e0`](https://github.com/GitoxideLabs/gitoxide/commit/d26b8e046496894ae06b0bbfdba77196976cd975))
  - Adjust to renaming of `git-packetline` to `gix-packetline` ([`5cbd22c`](https://github.com/GitoxideLabs/gitoxide/commit/5cbd22cf42efb760058561c6c3bbcd4dab8c8be1))
  - Adjust to renaming of `git-worktree` to `gix-worktree` ([`73a1282`](https://github.com/GitoxideLabs/gitoxide/commit/73a12821b3d9b66ec1714d07dd27eb7a73e3a544))
  - Adjust to renamining of `git-hashtable` to `gix-hashtable` ([`26a0c98`](https://github.com/GitoxideLabs/gitoxide/commit/26a0c98d0a389b03e3dc7bfc758b37155e285244))
  - Adjust to renamining of `git-worktree` to `gix-worktree` ([`108bb1a`](https://github.com/GitoxideLabs/gitoxide/commit/108bb1a634f4828853fb590e9fc125f79441dd38))
  - Adjust to renamining of `git-tui` to `gix-tui` ([`8782262`](https://github.com/GitoxideLabs/gitoxide/commit/878226282500993e755cdf99bf0076f91130e0cd))
  - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
  - Adjust to renaming of `git-date` to `gix-date` ([`9a79ff2`](https://github.com/GitoxideLabs/gitoxide/commit/9a79ff2d5cc74c1efad9f41e21095ae498cce00b))
  - Adjust to renaming of `git-pathspec` to `gix-pathspec` ([`37f7c6b`](https://github.com/GitoxideLabs/gitoxide/commit/37f7c6b9070e118604aa3fc0b38530699dcfec6e))
  - Adjust to renamining of `git-attributes` to `gix-attributes` ([`4a8b3b8`](https://github.com/GitoxideLabs/gitoxide/commit/4a8b3b812ac26f2a2aee8ce8ca81591273383c84))
  - Adjust to renaminig of `git-quote` to `gix-quote` ([`648025b`](https://github.com/GitoxideLabs/gitoxide/commit/648025b7ca94411fdd0d90c53e5faede5fde6c8d))
  - Adjust to renaming of `git-config` to `gix-config` ([`3a861c8`](https://github.com/GitoxideLabs/gitoxide/commit/3a861c8f049f6502d3bcbdac752659aa1aeda46a))
  - Adjust to renaming of `git-ref` to `gix-ref` ([`1f5f695`](https://github.com/GitoxideLabs/gitoxide/commit/1f5f695407b034377d94b172465ff573562b3fc3))
  - Adjust to renaming of `git-lock` to `gix-lock` ([`2028e78`](https://github.com/GitoxideLabs/gitoxide/commit/2028e7884ae1821edeec81612f501e88e4722b17))
  - Adjust to renaming of `git-tempfile` to `gix-tempfile` ([`b6cc3eb`](https://github.com/GitoxideLabs/gitoxide/commit/b6cc3ebb5137084a6327af16a7d9364d8f092cc9))
  - Adjust to renaming of `git-object` to `gix-object` ([`fc86a1e`](https://github.com/GitoxideLabs/gitoxide/commit/fc86a1e710ad7bf076c25cc6f028ddcf1a5a4311))
  - Adjust to renaming of `git-actor` to `gix-actor` ([`4dc9b44`](https://github.com/GitoxideLabs/gitoxide/commit/4dc9b44dc52f2486ffa2040585c6897c1bf55df4))
  - Adjust to renaming of `git-validate` to `gix-validate` ([`5e40ad0`](https://github.com/GitoxideLabs/gitoxide/commit/5e40ad078af3d08cbc2ca81ce755c0ed8a065b4f))
  - Adjust to renaming of `git-hash` to `gix-hash` ([`4a9d025`](https://github.com/GitoxideLabs/gitoxide/commit/4a9d0257110c3efa61d08c8457c4545b200226d1))
  - Adjust to renaming of `git-features` to `gix-features` ([`e2dd68a`](https://github.com/GitoxideLabs/gitoxide/commit/e2dd68a417aad229e194ff20dbbfd77668096ec6))
  - Adjust to renaming of `git-glob` to `gix-glob` ([`35b2a3a`](https://github.com/GitoxideLabs/gitoxide/commit/35b2a3acbc8f2a03f151bc0a3863163844e0ca86))
  - Adjust to renaming of `git-sec` to `gix-sec` ([`eabbb92`](https://github.com/GitoxideLabs/gitoxide/commit/eabbb923bd5a32fc80fa80f96cfdc2ab7bb2ed17))
  - Adapt to renaming of `git-path` to `gix-path` ([`d3bbcfc`](https://github.com/GitoxideLabs/gitoxide/commit/d3bbcfccad80fc44ea8e7bf819f23adaca06ba2d))
  - Adjust to rename of `git-config-value` to `gix-config-value` ([`622b3e1`](https://github.com/GitoxideLabs/gitoxide/commit/622b3e1d0bffa0f8db73697960f9712024fac430))
  - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
  - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
  - Merge branch 'rename-crates' ([`6461c3d`](https://github.com/GitoxideLabs/gitoxide/commit/6461c3da4d6daee857606d94294c3f87fc36965a))
  - Rename `git-repository` to `gix` ([`7bed2a9`](https://github.com/GitoxideLabs/gitoxide/commit/7bed2a96604397fa990f427b1a970ddeb6f09f1c))
  - Release git-repository v0.35.0, safety bump 3 crates ([`abe1e91`](https://github.com/GitoxideLabs/gitoxide/commit/abe1e91ae6b1bac5bcf767f51a4f242ae2633ecc))
  - Merge branch 'rename-tracking' ([`9e7d792`](https://github.com/GitoxideLabs/gitoxide/commit/9e7d79273487abfcb99ed2d439c475a659cd25e6))
  - Add `gix` crate to reserve name ([`5104a78`](https://github.com/GitoxideLabs/gitoxide/commit/5104a787127bf0b1f9b65f371b7c5b79f491e396))
  - Keep track of all keys relevant renaming when merging and obtaining status ([`0f2ed59`](https://github.com/GitoxideLabs/gitoxide/commit/0f2ed59b928ef3f4d6cd5a0cd0380504891d92f2))

</details>

## 0.21.0 (2023-02-09)

<csr-id-1d9a5e925890883e9e712db14ac82411ad2fdfc3/>

### Bug Fixes

- <csr-id-85fe1f46e88c1b2784eb9223f1cc193554d1c2f1/> upgrade to prodash v23 for avoiding to draw line progress while there is a prompt.
  This works by only drawing if something changed, which only happens if there
  the system isn't waiting for user input (at least where it matters, when fetching).

### New Features

- <csr-id-b81f650b476354c5bd030aa03b9a723d9d7a0970/> `gix clone <url>` is now permitted without specifying a destination directory.
  Note that the implementation doesn't take into account potential redirects and renames
  as it's implemented only with the first URL it sees (not the redirected ones).
- <csr-id-38ae61a805bd8cca5df8d1c1dcf3a8a0f9c85f5a/> make more HTTP options available
  - `http.schannelCheckRevoke`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 22 commits contributed to the release over the course of 47 calendar days.
- 49 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#683](https://github.com/GitoxideLabs/gitoxide/issues/683)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#683](https://github.com/GitoxideLabs/gitoxide/issues/683)**
  - `gix clone <url>` is now permitted without specifying a destination directory. ([`b81f650`](https://github.com/GitoxideLabs/gitoxide/commit/b81f650b476354c5bd030aa03b9a723d9d7a0970))
- **Uncategorized**
  - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
  - Make fmt ([`e22080e`](https://github.com/GitoxideLabs/gitoxide/commit/e22080e4a29d0bad15a99d565a5e3e304a8743ec))
  - Merge branch 'adjustments-for-cargo' ([`7bba270`](https://github.com/GitoxideLabs/gitoxide/commit/7bba2709488b7eb999b8136dbab03af977241678))
  - `gix progress` now usees the configuration tree as basis for data. ([`ea76bf5`](https://github.com/GitoxideLabs/gitoxide/commit/ea76bf5ba1b06bcf94bcadf5129ce265c90144d3))
  - Break cyclical dev dependencies ([`1fea18f`](https://github.com/GitoxideLabs/gitoxide/commit/1fea18f5f8b4189a23dc4fa3f041a672f6fbcfb3))
  - Thanks clippy ([`f7b8794`](https://github.com/GitoxideLabs/gitoxide/commit/f7b879448258a058b8607916da0ddf13b83896d9))
  - Upgrade to clap 4.1 ([`1d9a5e9`](https://github.com/GitoxideLabs/gitoxide/commit/1d9a5e925890883e9e712db14ac82411ad2fdfc3))
  - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
  - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
  - Thanks clippy ([`9e04685`](https://github.com/GitoxideLabs/gitoxide/commit/9e04685dd3f109bfb27663f9dc7c04102e660bf2))
  - Upgrade `tabled` ([`7aed326`](https://github.com/GitoxideLabs/gitoxide/commit/7aed326e706b13ae88f98880de055201b2a869cc))
  - Release git-date v0.4.0, git-actor v0.17.0, git-object v0.26.0, git-traverse v0.22.0, git-index v0.12.0, safety bump 15 crates ([`0e3d0a5`](https://github.com/GitoxideLabs/gitoxide/commit/0e3d0a56d7e6a60c6578138f2690b4fa54a2072d))
  - Merge branch 'gix-clone-improvements' ([`76c99f3`](https://github.com/GitoxideLabs/gitoxide/commit/76c99f3005f1b0031921b536f5d268715e41f3c8))
  - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
  - Merge branch 'adjustments-for-cargo' ([`f8c562a`](https://github.com/GitoxideLabs/gitoxide/commit/f8c562a559e6dc3377583cc7200585dad7c3d481))
  - Upgrade to prodash v23 for avoiding to draw line progress while there is a prompt. ([`85fe1f4`](https://github.com/GitoxideLabs/gitoxide/commit/85fe1f46e88c1b2784eb9223f1cc193554d1c2f1))
  - Adapt to changes in git-repository and git-transport ([`d336368`](https://github.com/GitoxideLabs/gitoxide/commit/d336368ea2b6e5918b709220522cf1509ef30be2))
  - Update planned usage of `ssh` related configuration variables. ([`3f91088`](https://github.com/GitoxideLabs/gitoxide/commit/3f910880fdf691b59460eea272772c7aff284716))
  - Release git-features v0.25.1, git-url v0.12.2, git-odb v0.38.1, git-transport v0.24.2, git-repository v0.30.2 ([`bb0a07b`](https://github.com/GitoxideLabs/gitoxide/commit/bb0a07b5edd5f980989d1a92e74df7f183febe87))
  - Merge branch 'adjustments-for-cargo' ([`d821fc5`](https://github.com/GitoxideLabs/gitoxide/commit/d821fc5b4ef4ba606f2b6bb68b66f7260a0205dc))
  - Make more HTTP options available ([`38ae61a`](https://github.com/GitoxideLabs/gitoxide/commit/38ae61a805bd8cca5df8d1c1dcf3a8a0f9c85f5a))

</details>

## 0.20.0 (2022-12-22)

This release also fixes compatibility issues that formerly prevented to fetch or clone form `https://googlesource.com`.

### Changed

- <csr-id-a4ac9cf3e667a3059e33aac8188150529578622d/> represent `GIT_(COMMITTER|AUTHOR)_(NAME|EMAIL|DATE)` with git configuration.
  That way it becomes more obvious where values are coming from.

### New Features

- <csr-id-98143699bb9481b010e21647f64dcb8a74bd80ad/> auto-enabled verbosity for `gix fetch/clone` and add `--no-verbose`.
  I found myself always adding (and having to remember to add) the `-v` flag
  for long-running operations so these should be able to default to a
  higher verbosity level.
  
  To counter that, there is a new `--no-verbose` flag to turn that off.
- <csr-id-aeb4a1d5cb76316058c7d687e26f5c7db351c09c/> add `--strict` option to enforce strict checking of configuration.
- <csr-id-8c9c243fc574dc17b92a8e2025fbd8efdf2833da/> `gix odb stats` to calculate statistics on the object database.
  This includes the amount of objects along with a listing of where they are
  located.
- <csr-id-e2b8c5dce7185b5fa194b90f32e642e5c9d1227f/> `gix clone --no-tags` support.
  This is the same as `git clone --no-tags`.
- <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
  That way callers can identify progress they are interested in, say, for
  selective visualizations.

### Changed (BREAKING)

- <csr-id-49f39d6bb487c0254176a5082f2c7851b83952a1/> `open::ReplacementObjects` is removed in favor of two custom git-configuration flags.
  Now it's possible to map the environment variables `GIT_REPLACE_REF_BASE` and `GIT_NO_REPLACE_OBJECTS`
  to custom git configuration keys which can also be set, namely `gitoxide.odb.replaceObjectsRefBase`
  and `gitoxide.odb.noReplaceObjects`.
  
  Along with the possibility of disabling the usage of `GIT_` prefixed environment variables one
  reaches the previous level of control without making object replacement a special case.

### New Features (BREAKING)

- <csr-id-becbd8d896a1663f1607be4e86e632773e926f1f/> represent object cache configuration like `GITOXIDE_PACK_CACHE_MEMORY` in git-configuration.
  That way there is a unified system for how to set values, which may be overridable by configuration
  variables or not.
  
  With this changes, the explicit application of environment variables for setting the cache
  isn't required anymore as everything happens using git-configuration, and automatically,
  while providing full control like before.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 31 commits contributed to the release over the course of 30 calendar days.
- 31 days passed between releases.
- 8 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release git-url v0.12.1, git-transport v0.24.1, git-protocol v0.25.1, git-repository v0.30.1, git-commitgraph v0.12.0, gitoxide-core v0.22.0, gitoxide v0.20.0 ([`08ec3a9`](https://github.com/GitoxideLabs/gitoxide/commit/08ec3a93d77a1018439a5c41c23729ffed27c5a5))
  - Fix label for `ein t estimate-hours` command ([`4c93de1`](https://github.com/GitoxideLabs/gitoxide/commit/4c93de113898c6ff6ae1da3ea38d646626c86205))
  - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
  - Merge branch 'read-header' ([`3d01252`](https://github.com/GitoxideLabs/gitoxide/commit/3d0125271ec7bd606734bd74757a7e31a18c7ce5))
  - `gix odb stats` to calculate statistics on the object database. ([`8c9c243`](https://github.com/GitoxideLabs/gitoxide/commit/8c9c243fc574dc17b92a8e2025fbd8efdf2833da))
  - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
  - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
  - `gix clone --no-tags` support. ([`e2b8c5d`](https://github.com/GitoxideLabs/gitoxide/commit/e2b8c5dce7185b5fa194b90f32e642e5c9d1227f))
  - Update `gix progress` to inform about `remote.<name>.tagOpt` ([`3ebb340`](https://github.com/GitoxideLabs/gitoxide/commit/3ebb3405f265fbd78a89c01bd30c3ad34036ef84))
  - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
  - Remove experiemnts to get rid of `git2` for good. ([`13c4fef`](https://github.com/GitoxideLabs/gitoxide/commit/13c4fefec40678c13f55eebd595b0e755468865a))
  - Upgrade to prodash v22 for API improvements ([`77ab98d`](https://github.com/GitoxideLabs/gitoxide/commit/77ab98dd41c3849b674d8b3794ef29219ca1447d))
  - Auto-enabled verbosity for `gix fetch/clone` and add `--no-verbose`. ([`9814369`](https://github.com/GitoxideLabs/gitoxide/commit/98143699bb9481b010e21647f64dcb8a74bd80ad))
  - Switch from `atty` to `is-terminal` ([`7304bc1`](https://github.com/GitoxideLabs/gitoxide/commit/7304bc1c0efaad64a39520962072343ef02f6c25))
  - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
  - Adapt to changes in `git-repository` ([`c4f68bf`](https://github.com/GitoxideLabs/gitoxide/commit/c4f68bf775b854625d901fe0bfcbdd38f656d408))
  - Represent object cache configuration like `GITOXIDE_PACK_CACHE_MEMORY` in git-configuration. ([`becbd8d`](https://github.com/GitoxideLabs/gitoxide/commit/becbd8d896a1663f1607be4e86e632773e926f1f))
  - Represent `GIT_(COMMITTER|AUTHOR)_(NAME|EMAIL|DATE)` with git configuration. ([`a4ac9cf`](https://github.com/GitoxideLabs/gitoxide/commit/a4ac9cf3e667a3059e33aac8188150529578622d))
  - `open::ReplacementObjects` is removed in favor of two custom git-configuration flags. ([`49f39d6`](https://github.com/GitoxideLabs/gitoxide/commit/49f39d6bb487c0254176a5082f2c7851b83952a1))
  - Apply related environment variables as config overrides ([`9441c26`](https://github.com/GitoxideLabs/gitoxide/commit/9441c261bcae61d1d1e674b5e783f38b0471be29))
  - Adapt to changes in `git-repository` ([`f1a4c8b`](https://github.com/GitoxideLabs/gitoxide/commit/f1a4c8b42ed8c94e7fe3a61eb222cf6b0886f4ee))
  - Merge branch 'optimize_hashtables' ([`95ad56c`](https://github.com/GitoxideLabs/gitoxide/commit/95ad56c11489bc46d6eb2b2f48cf0bf01e954c58))
  - Rename `git-shamap` and bring `git-hashtable` into the family. ([`f7ad1b1`](https://github.com/GitoxideLabs/gitoxide/commit/f7ad1b1df3ff4692d009ba60571fcc9272f5ac31))
  - Switch to custom Hasher implementation ([`269d59e`](https://github.com/GitoxideLabs/gitoxide/commit/269d59e0bee1f072096667b143800a0d85b18403))
  - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
  - Update progress of http.proxyAuthMethod ([`872dc1a`](https://github.com/GitoxideLabs/gitoxide/commit/872dc1ab43ce626b4166dae3dc8bddf8e85c9409))
  - Add `--strict` option to enforce strict checking of configuration. ([`aeb4a1d`](https://github.com/GitoxideLabs/gitoxide/commit/aeb4a1d5cb76316058c7d687e26f5c7db351c09c))
  - Don't lock stdout/stderr as it will deadlock on dbg-printing ([`62cae0e`](https://github.com/GitoxideLabs/gitoxide/commit/62cae0e6bfe8113c0225152a896338017c8de474))
  - Adapt to changes in `git-config` ([`1c2e755`](https://github.com/GitoxideLabs/gitoxide/commit/1c2e755e517b0f9fe8671187f5c30076ce43a3c9))
  - Upgrade to `prodash 21.1` and add `Ids` to all progress instances. ([`c8835c6`](https://github.com/GitoxideLabs/gitoxide/commit/c8835c6edae784c9ffcb69a674c0a6545dbb2af3))
  - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))

</details>

## 0.19.0 (2022-11-21)

### Bug Fixes (BREAKING)

- <csr-id-4308a209dddcbb461c34d45fb9af8b4621d4600a/> `max-pure` now builds without any C build tooling due to lack of `openssl-sys`.
  To make this work, we leave the `reqwest` configuration to downstream crates.
  Note that this means downstream will have to select their TLS settings
  themselves, so builds may fail after upgrade until this is done.
  
  An example for a `reqwest` configuration can be found in the
  `gitoxide` Cargo.toml in the root of the `gitoxide` repository.

### New Features

- <csr-id-3ddbd2de369b521fa3f21935f10fe9c248840893/> Make `reqwest` TLS backend configuration easy.
  We provide the choice of `native-tls` or `rust-tls`. If none is
  provided, the user can configure on their on similar to how it's done
  in `gix`.
  
  Please note that a choice now has to be made or HTTPS will not be
  available, so use one of…
  
  * blocking-http-transport-reqwest-rust-tls
* blocking-http-transport-reqwest-native-tls

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release.
- 4 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#602](https://github.com/GitoxideLabs/gitoxide/issues/602)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#602](https://github.com/GitoxideLabs/gitoxide/issues/602)**
  - `max-pure` now builds without any C build tooling due to lack of `openssl-sys`. ([`4308a20`](https://github.com/GitoxideLabs/gitoxide/commit/4308a209dddcbb461c34d45fb9af8b4621d4600a))
- **Uncategorized**
  - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
  - Make `reqwest` TLS backend configuration easy. ([`3ddbd2d`](https://github.com/GitoxideLabs/gitoxide/commit/3ddbd2de369b521fa3f21935f10fe9c248840893))
  - Merge branch 'max-pure' ([`03ff188`](https://github.com/GitoxideLabs/gitoxide/commit/03ff1882f2982fba38fbbf245eea13ef9df50f33))
  - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
  - Upgrade to 2021 edition ([`f01a5d1`](https://github.com/GitoxideLabs/gitoxide/commit/f01a5d11680a191f7fd8293ad4979604c29f2ccb))

</details>

## 0.18.0 (2022-11-17)

This releases fixes `gix fetch` so that it is able to clone or fetch `pytorch` or other repositories on case-insensitive file systems. 

It's also an attempt to trigger CI to build binary releases.

### New Features

- <csr-id-223be81a94a4fcd21f672302bcc5814dc36e41ef/> Add `max-pure` build option to build without C dependencies
  As opposed to the `small` option, it comes with all the features but
  avoids anything written in C (as far as we know), which should make
  building easier as it should only invoke the Rust compiler.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 16 commits contributed to the release over the course of 9 calendar days.
- 11 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#592](https://github.com/GitoxideLabs/gitoxide/issues/592)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#592](https://github.com/GitoxideLabs/gitoxide/issues/592)**
  - Add `max-pure` build option to build without C dependencies ([`223be81`](https://github.com/GitoxideLabs/gitoxide/commit/223be81a94a4fcd21f672302bcc5814dc36e41ef))
- **Uncategorized**
  - Release git-glob v0.4.2, git-config-value v0.8.2, git-lock v2.2.0, git-ref v0.19.0, git-config v0.11.0, git-discover v0.8.0, git-index v0.8.0, git-transport v0.22.0, git-protocol v0.23.0, git-worktree v0.8.0, git-repository v0.28.0, gitoxide-core v0.20.0, gitoxide v0.18.0, safety bump 9 crates ([`0c253b1`](https://github.com/GitoxideLabs/gitoxide/commit/0c253b15143dcedfe4c66d64ab1ea6e097030651))
  - Merge branch 'http-config' ([`665b53e`](https://github.com/GitoxideLabs/gitoxide/commit/665b53e1c2e1de65fafa28b669f58977868bbc81))
  - Document that histogram is now the default diff algorithm ([`c76572b`](https://github.com/GitoxideLabs/gitoxide/commit/c76572b3662776b524a7e4a1fd96d2eaa22a560f))
  - Introduce new `gitoxide.http.connectTimeout` for more control for git clients ([`2ab80e4`](https://github.com/GitoxideLabs/gitoxide/commit/2ab80e4c95a7bf3c7e56bb5a95ac78ac930fc9ee))
  - Keep track of `no_proxy` environment variable support ([`f0625de`](https://github.com/GitoxideLabs/gitoxide/commit/f0625de13073de4767881ed0398d0cd2791b0ad2))
  - Merge branch 'main' into http-config ([`f4ff821`](https://github.com/GitoxideLabs/gitoxide/commit/f4ff821fd4233dd1dc1a449af4d4600becf3b4ac))
  - Merge branch 'patch-1' ([`f030ac2`](https://github.com/GitoxideLabs/gitoxide/commit/f030ac219317c7656cb5113fa9dc7ebf98748839))
  - Remove redundant configuration of `git-transport` via top-level Cargo.toml ([`84f12b4`](https://github.com/GitoxideLabs/gitoxide/commit/84f12b45750ef38449a0d2ce45d8e4ec6c838984))
  - Update progress ([`3d9fb6c`](https://github.com/GitoxideLabs/gitoxide/commit/3d9fb6c095a272d5ddf6c5b6ce96820bc9d59cbb))
  - Don't forget to update 'progress' ([`0ec5220`](https://github.com/GitoxideLabs/gitoxide/commit/0ec5220fea50f06eb61bafff525111ce2435c994))
  - Update progress with gitoxide.userAgent ([`1c012f4`](https://github.com/GitoxideLabs/gitoxide/commit/1c012f4c2e05e1f565fc51fffee2f7d278e5a7de))
  - Plan for user agent string configuration ([`f5499a5`](https://github.com/GitoxideLabs/gitoxide/commit/f5499a55ed0230e2852b41b54648003e3d6cb859))
  - Merge branch 'main' into http-config ([`7c5b37d`](https://github.com/GitoxideLabs/gitoxide/commit/7c5b37d28e98f59a6847368a0d0166d2dbb4acc1))
  - Release git-diff v0.22.0, git-index v0.7.1, git-pack v0.26.0, git-odb v0.36.0, git-transport v0.21.2, git-repository v0.27.0, safety bump 6 crates ([`f0cab31`](https://github.com/GitoxideLabs/gitoxide/commit/f0cab317bb0c2799fa80d16f3ae1b89d6aee4284))
  - Keep track of http related configuration keys. ([`1afaebd`](https://github.com/GitoxideLabs/gitoxide/commit/1afaebdcae977af8a9a0f0788ec754746d6d05bb))

</details>

## 0.17.0 (2022-11-06)

### New Features

- <csr-id-e973dfeaf17ca63385496202e9fdcdd912e20f42/> `gix remote ref-map --show-unmapped-remote-refs`.
  That way it's more obvious to see what was filtered out by ref-specs.
  
  It's also great to validate that server-side filtering via ref-prefix
  will not send refs that are referred to by symbolic refs that are
  not filtered out. That should be fine as it's all about objects,
  it's just something to deal with as we may have to deal with symbolic
  refs that aren't in the set of refs the server sent to us.
- <csr-id-b1edb9e3537df86669714f03666f4a88e0ac3709/> diff algorithm is controlled by git configuration `diff.algorithm`
- <csr-id-20259da4ddf9fabfb2d2bd4e2274c0ed42bdb0e5/> `ein t hours` allows to specify the amount of worker threads.
- <csr-id-945fef0781c3d12992016c883ccb4ee11e882a1a/> allow automatic resizing of line-based progress bars

### Bug Fixes

- <csr-id-3a053284cfefe27873dcc5b4f74d63a560bb5d41/> collect `stderr` and print it afterwards to avoid intersection with line progress.
  Previously it would happen that stderr would be printed directly and mix
  with the line progress (as in `-v`) which also prints to stderr.
  
  Now errors are collected and output at the end once the line renderer
  was already shutdown.

### Changed (BREAKING)

- <csr-id-3a0fb1b45c757add49677450836c0aaf6179a2b5/> remote `lock_mode` from all methods dealing with reference edits.
  It is now read from `core.filesRefLockTimeout` accordingly.

### New Features (BREAKING)

- <csr-id-92bbe335688e4c8e96061663e71a599022f7b96f/> remove `gix remote --url` in favor of determining the intention similar to `git fetch`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 71 commits contributed to the release.
- 47 days passed between releases.
- 7 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#536](https://github.com/GitoxideLabs/gitoxide/issues/536)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
  - Keep track of several branch related keys ([`443a75a`](https://github.com/GitoxideLabs/gitoxide/commit/443a75a6098968995bed644c84ed8ce9d8ed0c34))
  - `gix remote ref-map --show-unmapped-remote-refs`. ([`e973dfe`](https://github.com/GitoxideLabs/gitoxide/commit/e973dfeaf17ca63385496202e9fdcdd912e20f42))
  - Update progress as we now respect `diff.algorithm` ([`30d32e7`](https://github.com/GitoxideLabs/gitoxide/commit/30d32e7255986e6e7ec92d55c86b747d7486e183))
  - Apply configuration overrides to newborn repo during clone ([`c8ef759`](https://github.com/GitoxideLabs/gitoxide/commit/c8ef759923a3c980b5a23c868f38804eccbc0fbc))
  - Collect `stderr` and print it afterwards to avoid intersection with line progress. ([`3a05328`](https://github.com/GitoxideLabs/gitoxide/commit/3a053284cfefe27873dcc5b4f74d63a560bb5d41))
  - Reduce verbosity of `clone` and print once entire clone is done ([`9a476df`](https://github.com/GitoxideLabs/gitoxide/commit/9a476df519bccfc6bcda8bd02aa4c1573a2691e7))
  - Upgrade to `prodash` v21 ([`a0655dc`](https://github.com/GitoxideLabs/gitoxide/commit/a0655dc7bc5dff388bc69a648e7f16b44fd1abd9))
  - First rough sketch of `gix clone` ([`23a5e8b`](https://github.com/GitoxideLabs/gitoxide/commit/23a5e8b658c5642c3f3060e013fd0eab06cbf027))
  - Progress totals (planned, devitaion) ([`63947ae`](https://github.com/GitoxideLabs/gitoxide/commit/63947ae77ecb5952d7b5da5c6695f002bb7a7c4c))
  - Prepare attribute-group setup as far as possible. ([`f5e2eeb`](https://github.com/GitoxideLabs/gitoxide/commit/f5e2eebe9560f664f044b82ffa0cd19fd0df311f))
  - Update usage of `core.checkState` ([`8b2aba1`](https://github.com/GitoxideLabs/gitoxide/commit/8b2aba1d2baf42a3578ee8c9ce89dbe29d4d3f4c))
  - Update progress based on filesystem config usage ([`de13c66`](https://github.com/GitoxideLabs/gitoxide/commit/de13c66ade29635ff585241fec2783cf972dd5c3))
  - Inform about even more keys in `gix progress` ([`0e9bd41`](https://github.com/GitoxideLabs/gitoxide/commit/0e9bd4133fe33382d6d8c42ca1280601582a1a17))
  - Plan more core filesystem attributes ([`d10a82e`](https://github.com/GitoxideLabs/gitoxide/commit/d10a82e86a461e72b9e5339a180f8adb34721e3e))
  - Update `init.defaultBranch` - now used by custom initialization. ([`4d6d7bb`](https://github.com/GitoxideLabs/gitoxide/commit/4d6d7bbf5c3d8159535e5756da082ca493e9a66a))
  - Support for handshake information in `gix fetch` ([`c47dcc6`](https://github.com/GitoxideLabs/gitoxide/commit/c47dcc69e54c635650b540c590131dbe7d32d05b))
  - Fix build ([`d034882`](https://github.com/GitoxideLabs/gitoxide/commit/d03488211ec5bd186f6d274c55cd96cfd9d119d5))
  - Remove `gix remote --url` in favor of determining the intention similar to `git fetch` ([`92bbe33`](https://github.com/GitoxideLabs/gitoxide/commit/92bbe335688e4c8e96061663e71a599022f7b96f))
  - Support for `--url` for arbitrary urls when fetching ([`8c7351c`](https://github.com/GitoxideLabs/gitoxide/commit/8c7351c4c50517b3ccc3479f2a7a020bc607bf24))
  - Frame for `gix fetch` ([`5b72d27`](https://github.com/GitoxideLabs/gitoxide/commit/5b72d2708889dc388facd9cbc61e5bfa5403e003))
  - Update progress information to include packedRefsTimeout ([`fd18320`](https://github.com/GitoxideLabs/gitoxide/commit/fd18320561e05431796aa4044c0a2b0605c9ca9d))
  - Remote `lock_mode` from all methods dealing with reference edits. ([`3a0fb1b`](https://github.com/GitoxideLabs/gitoxide/commit/3a0fb1b45c757add49677450836c0aaf6179a2b5))
  - Add information about planned lock timeout support (from configuration) ([`7076891`](https://github.com/GitoxideLabs/gitoxide/commit/7076891fb1b44cd442928f7e56f53f4b085e7a11))
  - Add remotes.<group> as planned feature for remotes ([`3c188b2`](https://github.com/GitoxideLabs/gitoxide/commit/3c188b2253bfb6d47394718425eef2d1a0547949))
  - Slightly nicer styling of config keys ([`eade88f`](https://github.com/GitoxideLabs/gitoxide/commit/eade88f8ebc638f504881e8bbbd60d42a5a3d9be))
  - Complete listing of records based on current usage, probably ([`6abd5a4`](https://github.com/GitoxideLabs/gitoxide/commit/6abd5a4e1daaf91fd109acb714057a82f67fa076))
  - Add more records ([`5c0d0ab`](https://github.com/GitoxideLabs/gitoxide/commit/5c0d0ab66d46a8d093ca0b5451996099a27ef1dd))
  - Add tabled for nicer printing ([`65e6496`](https://github.com/GitoxideLabs/gitoxide/commit/65e64964c7cd151e53e5a7d4b9ba8fabda1c0e16))
  - Refactor ([`b42b08a`](https://github.com/GitoxideLabs/gitoxide/commit/b42b08afafd904ff2adb1f00688437357532193a))
  - Add support for more types of configurations ([`317e02a`](https://github.com/GitoxideLabs/gitoxide/commit/317e02a5900189ec7f9f3c2bb27d5696178d7869))
  - A very first version of `gix progress show` ([`92e082a`](https://github.com/GitoxideLabs/gitoxide/commit/92e082a288cf5e48caa205a4c7bd1ced025fea46))
- **[#536](https://github.com/GitoxideLabs/gitoxide/issues/536)**
  - `ein t hours` allows to specify the amount of worker threads. ([`20259da`](https://github.com/GitoxideLabs/gitoxide/commit/20259da4ddf9fabfb2d2bd4e2274c0ed42bdb0e5))
- **Uncategorized**
  - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
  - Merge branch 'write-sparse-index' ([`ba17db0`](https://github.com/GitoxideLabs/gitoxide/commit/ba17db03e4e832db724ab3e08e3df05eb61dd401))
  - Plan `index.version` for when we can write V4 indices. ([`da96d34`](https://github.com/GitoxideLabs/gitoxide/commit/da96d34bd609776f3c8590171ddbf894f2205ae5))
  - Notes about the split-index extension. ([`ad44982`](https://github.com/GitoxideLabs/gitoxide/commit/ad449822c952a7ed5e6124216e7d7668f0f5873f))
  - Take note of additional options for promisor packs and partial clone filters ([`1ec27f8`](https://github.com/GitoxideLabs/gitoxide/commit/1ec27f86113aa45a1540441a21152988b3911015))
  - Make note of `extension.worktreeConfig` ([`fe1e646`](https://github.com/GitoxideLabs/gitoxide/commit/fe1e6467296ff93c23fe191ed5a265348ce930dc))
  - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
  - Add `init.templateDir` to `gix progress` ([`9fab050`](https://github.com/GitoxideLabs/gitoxide/commit/9fab0501f5017e43132138579238a46726a7348f))
  - Make fmt ([`ea2136b`](https://github.com/GitoxideLabs/gitoxide/commit/ea2136b065979cecb3a1fdbf7b20ed7514128d9a))
  - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
  - Diff algorithm is controlled by git configuration `diff.algorithm` ([`b1edb9e`](https://github.com/GitoxideLabs/gitoxide/commit/b1edb9e3537df86669714f03666f4a88e0ac3709))
  - Merge branch 'main' into gix-clone ([`fa27570`](https://github.com/GitoxideLabs/gitoxide/commit/fa27570f491388cce6137af44330d76870d07202))
  - Merge branch 'imra-diff' ([`f53f942`](https://github.com/GitoxideLabs/gitoxide/commit/f53f9426f206686b30abd73a201a92b1405e782d))
  - Adapt to changes in `git-diff` for a 2x speedup when calculating line changes ([`296f3b6`](https://github.com/GitoxideLabs/gitoxide/commit/296f3b6ee29d5e628a19d56db80ba8736223e226))
  - Update `gix progress` records ([`b05a2e7`](https://github.com/GitoxideLabs/gitoxide/commit/b05a2e7939df0c25da8186395e7eceda9e1baa9b))
  - Merge branch 'main' into gix-clone ([`de4fe06`](https://github.com/GitoxideLabs/gitoxide/commit/de4fe06202906ea5c62e667826b42cf7b57b1ff0))
  - Mark the upcoming usage of init.defaultBranch. ([`6225f35`](https://github.com/GitoxideLabs/gitoxide/commit/6225f35398bc494ad74da342c4ebbe0487b106f8))
  - Merge branch 'fix-gix-index-from-tree' ([`da5f63c`](https://github.com/GitoxideLabs/gitoxide/commit/da5f63cbc7506990f46d310f8064678decb86928))
  - Write index without output path to memory only. ([`c8d0345`](https://github.com/GitoxideLabs/gitoxide/commit/c8d03454f1b2c18876cc8935e0afcea611cb8647))
  - Merge branch 'gix-index-from-tree' ([`8c24386`](https://github.com/GitoxideLabs/gitoxide/commit/8c24386f1874cd94f78fefbe434963f772878b1f))
  - Refactor ([`67f2247`](https://github.com/GitoxideLabs/gitoxide/commit/67f224785193a5269cf65963fd21b21b723d485e))
  - Refactor ([`01ab5ca`](https://github.com/GitoxideLabs/gitoxide/commit/01ab5cac23427a9c3b7f153201627eb8c8898e96))
  - Update with various configuration variables relevant to checking out worktrees ([`09d767a`](https://github.com/GitoxideLabs/gitoxide/commit/09d767abd371a268a9fa475956be09ddda6b42ea))
  - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
  - Merge branch 'clone' ([`507dc7e`](https://github.com/GitoxideLabs/gitoxide/commit/507dc7e706cb3c9d89d048b3aff5df239a9b6788))
  - Update usage of clone related configuration ([`1a1e862`](https://github.com/GitoxideLabs/gitoxide/commit/1a1e862b2e8cd88f5f6fbb9d86f618761bb71ef1))
  - Update progress with intended uses of `clone.` variables ([`8b804a3`](https://github.com/GitoxideLabs/gitoxide/commit/8b804a31cb20a5264311f0b6ba02f857bea933ad))
  - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
  - Make fmt ([`53acf25`](https://github.com/GitoxideLabs/gitoxide/commit/53acf2565743eff7cead7a42011107b2fc8d7e0e))
  - Merge branch 'fetch-pack' ([`f47c891`](https://github.com/GitoxideLabs/gitoxide/commit/f47c89129732bcb06fe76a4696fe38ab1151fb0c))
  - Merge branch 'fetch-pack' ([`3c49400`](https://github.com/GitoxideLabs/gitoxide/commit/3c49400809c7c2120f4ce704c19a0421545b5acd))
  - Update docs ([`c5c0ac5`](https://github.com/GitoxideLabs/gitoxide/commit/c5c0ac50616957b5b0dcaf530d294abe63e3b4c7))
  - Input id can now be a commit or tree as prefix or full object id ([`8ef3fcb`](https://github.com/GitoxideLabs/gitoxide/commit/8ef3fcbc416b2a53ead7d6ba36991a3d035f5f22))
  - Thanks clippy ([`8dadd70`](https://github.com/GitoxideLabs/gitoxide/commit/8dadd70f8b7db1794652805c6238763886a8570d))
  - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
  - Merge branch 'fix-odb-race' ([`b862fc5`](https://github.com/GitoxideLabs/gitoxide/commit/b862fc52dd2409e912c892c7f428a571f565b64a))
  - Merge branch 'main' into fix-odb-race ([`30712dc`](https://github.com/GitoxideLabs/gitoxide/commit/30712dc9ab952d29c528ec58bcc467b425489304))
  - Allow automatic resizing of line-based progress bars ([`945fef0`](https://github.com/GitoxideLabs/gitoxide/commit/945fef0781c3d12992016c883ccb4ee11e882a1a))
  - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))

</details>

## 0.16.0 (2022-09-20)

### Changed

- <csr-id-3c7c9a735f5771ef787cbc86b46cbafc9226f4d6/> `ein tool hours -s` was split into `-f|--file-stats` and `-l|line-stats`.
  That way more information is generated at increasingly high costs.

### New Features

- <csr-id-28c4cae70aab2bd5b479961fcc6ee91ff80f651b/> `ein tool hours --stat` to collect additional statistics per author.
  Note that these are expensive and unconditionally use threads to speed
  up these computations.
- <csr-id-5d0332f51c63c5456a28c8f3f466ad805b2e0626/> `ein tool hours -b` ignores bots.
  For now it only considers bots with names containing `[bot]`.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 43 commits contributed to the release over the course of 27 calendar days.
- 27 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
  - Refactor ([`11851f3`](https://github.com/GitoxideLabs/gitoxide/commit/11851f334f642e7bd69bcbfc7ad4f1990fc326ba))
  - Option to print server information about the connection ([`4720666`](https://github.com/GitoxideLabs/gitoxide/commit/4720666c8bfdaa3acc5c832b44755d4b4f86e16e))
  - Show fixes as well ([`2237495`](https://github.com/GitoxideLabs/gitoxide/commit/2237495d82624b39bf75c6430549424a5e36b8bb))
  - Correct printing of tag information (even though it doesn't look great) ([`f4d8198`](https://github.com/GitoxideLabs/gitoxide/commit/f4d8198992b4c45f64d81e20f40a1cad69883162))
  - Wire up the `ref-map` sub-command. ([`94c2b78`](https://github.com/GitoxideLabs/gitoxide/commit/94c2b785f892f85503b8927c7fa98ae99d677be7))
  - Frame for `git-fetchhead` crate ([`f9aeda8`](https://github.com/GitoxideLabs/gitoxide/commit/f9aeda88554fb852dd21ae8402000e0da742e2b2))
  - Select `gix` commands will now load the git installation configuration ([`23d2dec`](https://github.com/GitoxideLabs/gitoxide/commit/23d2dec375305c39d472c4f8ff764274dd033f6b))
  - Refactor ([`7abc0a3`](https://github.com/GitoxideLabs/gitoxide/commit/7abc0a39205b9f374c90c4750fe6cc9b3749d7b9))
  - Add sketch of `gix credential` ([`642e21f`](https://github.com/GitoxideLabs/gitoxide/commit/642e21fc58d8d4b68cba3067c88d44c019ec4ace))
  - Add new git-config-value crate ([`f87edf2`](https://github.com/GitoxideLabs/gitoxide/commit/f87edf26c1cb795142fbe95e12c0dfc1166e4233))
  - Add `git-prompt` crate for registration ([`b0140a6`](https://github.com/GitoxideLabs/gitoxide/commit/b0140a6819cdfce0a5d88149f3713295b3bb54b5))
  - First frame of `git-command` crate ([`436632a`](https://github.com/GitoxideLabs/gitoxide/commit/436632a3822d3671c073cdbbbaf8e569de62bb09))
- **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
  - `ein tool hours -s` was split into `-f|--file-stats` and `-l|line-stats`. ([`3c7c9a7`](https://github.com/GitoxideLabs/gitoxide/commit/3c7c9a735f5771ef787cbc86b46cbafc9226f4d6))
  - Working progress printing ([`67ec2c7`](https://github.com/GitoxideLabs/gitoxide/commit/67ec2c7f9a4a6cefdf7148f5c7e48a79f201c4d2))
  - First attempt to get progress information from stat worker. ([`0947c70`](https://github.com/GitoxideLabs/gitoxide/commit/0947c703f9cecc31ceba101565e6ecafb00adb08))
  - Upgrade to prodash 20.1 for `Progress::counter()` feature ([`0ac4a2c`](https://github.com/GitoxideLabs/gitoxide/commit/0ac4a2c514aeb94d8e90ce28ae7a0e0350c21ab2))
  - `ein tool hours --stat` to collect additional statistics per author. ([`28c4cae`](https://github.com/GitoxideLabs/gitoxide/commit/28c4cae70aab2bd5b479961fcc6ee91ff80f651b))
- **Uncategorized**
  - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
  - Merge branch 'hours-upgrade' ([`26489d1`](https://github.com/GitoxideLabs/gitoxide/commit/26489d14472b840b36696435c22d9077f7ab323d))
  - Use rev-specs instead of ref-names ([`cf7182e`](https://github.com/GitoxideLabs/gitoxide/commit/cf7182e3390c03df97c10cd101440f7aa8874904))
  - `ein tool hours -b` ignores bots. ([`5d0332f`](https://github.com/GitoxideLabs/gitoxide/commit/5d0332f51c63c5456a28c8f3f466ad805b2e0626))
  - Merge branch 'filter-refs' ([`3773b92`](https://github.com/GitoxideLabs/gitoxide/commit/3773b92b8372c9a40a74d281149ca65b057a7da9))
  - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
  - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
  - Implement `gix index from-tree` ([`2fbd3df`](https://github.com/GitoxideLabs/gitoxide/commit/2fbd3df89373eea5d6268fa47e046c8ebad8bba9))
  - Merge branch 'dep-upgrade' ([`59767b1`](https://github.com/GitoxideLabs/gitoxide/commit/59767b1fc1d07b8a7a9333a719e3716746611bc4))
  - Upgrade prodash and crosstermion to latest versions ([`ab7ee5b`](https://github.com/GitoxideLabs/gitoxide/commit/ab7ee5b5d5c15771f431ada9c3b4f53e4be2afdd))
  - Make fmt ([`535e967`](https://github.com/GitoxideLabs/gitoxide/commit/535e967666c6da657ff1b7eff7c64ab27cafb182))
  - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
  - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
  - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
  - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
  - Merge branch 'main' into filter-refs-by-spec ([`51dc828`](https://github.com/GitoxideLabs/gitoxide/commit/51dc8282fb77b519ff7d2c94c6bd73af306cfe8b))
  - Release git-diff v0.18.1, git-discover v0.4.2, git-traverse v0.16.4, git-repository v0.23.1 ([`2571831`](https://github.com/GitoxideLabs/gitoxide/commit/2571831e5939bf4ea6f19537b0c1ccd71dc99088))
  - Merge branch 'main' into filter-refs-by-spec ([`56ba481`](https://github.com/GitoxideLabs/gitoxide/commit/56ba481f4c48f74f10397feb1b6dc3d7dd3704fb))
  - A basic implementation of rev-list without anything fancy ([`791dd66`](https://github.com/GitoxideLabs/gitoxide/commit/791dd666430fe0586c7db75b352487a72d3789e7))
  - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
  - Release git-object v0.20.3, git-ref v0.15.4, git-config v0.7.1, git-diff v0.18.0, git-traverse v0.16.3, git-pack v0.22.0, git-odb v0.32.0, git-url v0.7.3, git-transport v0.19.3, git-protocol v0.19.1, git-refspec v0.1.1, git-repository v0.23.0, safety bump 6 crates ([`85a3bed`](https://github.com/GitoxideLabs/gitoxide/commit/85a3bedd68d2e5f36592a2f691c977dc55298279))
  - Release git-features v0.22.3, git-revision v0.4.4 ([`c2660e2`](https://github.com/GitoxideLabs/gitoxide/commit/c2660e2503323531ba02519eaa51124ee22fec51))
  - Merge branch 'main' into filter-refs-by-spec ([`dbfa328`](https://github.com/GitoxideLabs/gitoxide/commit/dbfa3282cf876596b250b2040c1ec0b761741796))
  - Restrict `sha1` `asm` to supported archs ([`b383fab`](https://github.com/GitoxideLabs/gitoxide/commit/b383fabbe10868317be51b99cfdd9b0981816042))
  - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
  - Merge branch 'fix-ci-installation' ([`9245083`](https://github.com/GitoxideLabs/gitoxide/commit/92450839621a4d99cb22d08cbf9f9a89ff6b9e3f))

</details>

## 0.15.0 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-fb382321186e758fcd5aba9ea52e5018327d170d/> reduce redunant feature specification for performance related flags
- <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### New Features

- <csr-id-45a30f0f31a99cda5cf105408e9c3905f43071f2/> Support for `-c/--config` in `gix`
- <csr-id-5d6d5ca305615568dfedbcc10ea86294c0a0472d/> `gix remote refs` to list all remote references of a suitable remote.
  This takes into account either a named remote, or the remote associated
  with the current branch, or the default remote it could deduce or obtain
  from the configuration.
- <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
- <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

### Changed (BREAKING)

- <csr-id-dda995790c260131048484a11e66185b9c841311/> remove `gix free remote ref-list` in favor of `gix remote refs`
  The functionality is the same, but the latter is built on top of a
  repository which is slightly less flexible, but preferable over
  maintaining a non-repo version.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 22 commits contributed to the release over the course of 5 calendar days.
- 7 days passed between releases.
- 8 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
  - Support for -c CLI config overrides in `gix config`. ([`19c1746`](https://github.com/GitoxideLabs/gitoxide/commit/19c1746cefca9bc76537637ec99634eb4cf66a92))
  - Remove `gix free remote ref-list` in favor of `gix remote refs` ([`dda9957`](https://github.com/GitoxideLabs/gitoxide/commit/dda995790c260131048484a11e66185b9c841311))
  - Support for `-c/--config` in `gix` ([`45a30f0`](https://github.com/GitoxideLabs/gitoxide/commit/45a30f0f31a99cda5cf105408e9c3905f43071f2))
  - Refactor ([`e0be6e9`](https://github.com/GitoxideLabs/gitoxide/commit/e0be6e9558add3255de63f3785306daace2707a6))
  - Add support for passing urls directly to bypass all remote repository logic. ([`df3cf18`](https://github.com/GitoxideLabs/gitoxide/commit/df3cf18a6ac1e4f35f6d11d62184a43722397bbe))
  - `gix remote refs` to list all remote references of a suitable remote. ([`5d6d5ca`](https://github.com/GitoxideLabs/gitoxide/commit/5d6d5ca305615568dfedbcc10ea86294c0a0472d))
  - Try to use maybe async for the simplest of possibly blocking remote interactions ([`db4df25`](https://github.com/GitoxideLabs/gitoxide/commit/db4df250d7e58518015bed0b9a1e3391b209cb29))
  - Basic parsing of `gix remote refs` without any implementation. ([`f8f1249`](https://github.com/GitoxideLabs/gitoxide/commit/f8f124943f73bacf816c6d0055f0b66659fd3906))
- **Uncategorized**
  - Release git-date v0.1.0, git-actor v0.11.4, git-revision v0.4.3, git-repository v0.22.1, cargo-smart-release v0.11.0, git-commitgraph v0.8.2, gitoxide-core v0.17.0, gitoxide v0.15.0 ([`1fb931a`](https://github.com/GitoxideLabs/gitoxide/commit/1fb931a7ea59f1cf895a6c1392fd8615b723c743))
  - Reduce redunant feature specification for performance related flags ([`fb38232`](https://github.com/GitoxideLabs/gitoxide/commit/fb382321186e758fcd5aba9ea52e5018327d170d))
  - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
  - Merge branch 'remote-ls-refs' ([`39d585d`](https://github.com/GitoxideLabs/gitoxide/commit/39d585d9f9ac6f3ecf51359c8e37f0a50e21ed45))
  - Thanks clippy ([`bb6813a`](https://github.com/GitoxideLabs/gitoxide/commit/bb6813abf365728d9851ee205b2c25b925a0f06a))
  - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
  - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
  - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
  - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
  - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
  - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
  - Merge branch 'main' into remote-ls-refs ([`c82bbfa`](https://github.com/GitoxideLabs/gitoxide/commit/c82bbfaddc45bf9b5b55f056613046d977d9ef09))
  - Prepare for release of git-repository ([`8aa5389`](https://github.com/GitoxideLabs/gitoxide/commit/8aa5389d5a1bdd3a07f1caa1c2f55c8af4f9844a))
  - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))

</details>

## 0.14.0 (2022-08-17)

### Changed

- <csr-id-0235111a4fcc40c7b57d973bfce27a66eddea901/> Invert behaviour to `open::Options::strict_config()`, with lenient being the default.
  This means API users will get libgit2 behaviour but commands like `gix` can
  change options to emulate `git` behaviour.

### New Features

- <csr-id-b83f6bdf7f8d8f5cef2a57fa3932b6a0e0988db1/> `--cat-file` flag for `gix rev parse` to cat instead of resolving.
- <csr-id-e972aad020d3653a53b20fa6e535d5759e239a45/> `gix rev previous-branches` subcommand

### Changed (BREAKING)

- <csr-id-edf73dd4db5b0f5d9309c95bf366e11ea6723885/> `ein tools` to `ein tool` for as it's more intuitive

### New Features (BREAKING)

- <csr-id-c5846e05aa54f0601ac7b8e2e59bcf1ffaa305f1/> `gix rev resolve --explain`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 26 commits contributed to the release over the course of 26 calendar days.
- 26 days passed between releases.
- 5 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
  - `--cat-file` flag for `gix rev parse` to cat instead of resolving. ([`b83f6bd`](https://github.com/GitoxideLabs/gitoxide/commit/b83f6bdf7f8d8f5cef2a57fa3932b6a0e0988db1))
  - `gix rev resolve --explain` ([`c5846e0`](https://github.com/GitoxideLabs/gitoxide/commit/c5846e05aa54f0601ac7b8e2e59bcf1ffaa305f1))
  - `gix rev previous-branches` subcommand ([`e972aad`](https://github.com/GitoxideLabs/gitoxide/commit/e972aad020d3653a53b20fa6e535d5759e239a45))
  - Support for parsing multiple specs in one invocation ([`84b5448`](https://github.com/GitoxideLabs/gitoxide/commit/84b5448deb7b87f67a1b7461f00793e7ae33ef31))
  - Support overriding cache settings with environment variables in `gix` ([`b838202`](https://github.com/GitoxideLabs/gitoxide/commit/b8382026cb5b979a5c563ea40d1d8e483c1ca23a))
  - Turn `regex` support on in `max` builds ([`c5df859`](https://github.com/GitoxideLabs/gitoxide/commit/c5df859e2a68265d308e12590c6ab16941e67c68))
- **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
  - Empty `git-refspec` crate for name reservation prior to implementation ([`871a3c0`](https://github.com/GitoxideLabs/gitoxide/commit/871a3c054d4fe6c1e92b6f2e260b19463404509f))
- **Uncategorized**
  - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
  - Merge branch 'core-abbrev-handling' ([`dbaff13`](https://github.com/GitoxideLabs/gitoxide/commit/dbaff13eaf7ba5f9c0ee2c90ac9f17e6078cad9e))
  - Control which command is lenient or not. That way `gix-config` can be lenient. ([`6a9c58f`](https://github.com/GitoxideLabs/gitoxide/commit/6a9c58fde7ca4a52fa1c3225974a2019e7d93168))
  - Invert behaviour to `open::Options::strict_config()`, with lenient being the default. ([`0235111`](https://github.com/GitoxideLabs/gitoxide/commit/0235111a4fcc40c7b57d973bfce27a66eddea901))
  - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
  - Merge branch 'main' into remote-ls-refs ([`c4bf958`](https://github.com/GitoxideLabs/gitoxide/commit/c4bf9585d815bc342e5fb383336cc654280dd34f))
  - Merge branch 'index-write-refactor' ([`805f432`](https://github.com/GitoxideLabs/gitoxide/commit/805f432bf8e9d2dd9ede56caf959de386d5d80c7))
  - `ein tools` to `ein tool` for as it's more intuitive ([`edf73dd`](https://github.com/GitoxideLabs/gitoxide/commit/edf73dd4db5b0f5d9309c95bf366e11ea6723885))
  - Merge branch 'main' into write-index-v2 ([`a938986`](https://github.com/GitoxideLabs/gitoxide/commit/a938986877302c197d1aed087594c5605416fe5f))
  - Merge branch 'parse-refspec' ([`2ba338e`](https://github.com/GitoxideLabs/gitoxide/commit/2ba338e28eb45d4d3215dd6ff9882611880d4cd9))
  - Add aliases to make revision sub-commands more accessible ([`a6d79e3`](https://github.com/GitoxideLabs/gitoxide/commit/a6d79e38cb0dd7e87d00a098030bbcaa614f259d))
  - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
  - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
  - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
  - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
  - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
  - Make fmt ([`47724c0`](https://github.com/GitoxideLabs/gitoxide/commit/47724c0edb382c036a3fc99884becfd2b0740d4b))
  - Merge branch 'kianmeng-fix-typos' ([`4e7b343`](https://github.com/GitoxideLabs/gitoxide/commit/4e7b34349c0a01ad8686bbb4eb987e9338259d9c))
  - Fix typos ([`e9fcb70`](https://github.com/GitoxideLabs/gitoxide/commit/e9fcb70e429edb2974afa3f58d181f3ef14c3da3))

</details>

## 0.13.0 (2022-07-22)

### Changed (BREAKING)

- <csr-id-89a41bf2b37db29b9983b4e5492cfd67ed490b23/> remove local-time-support feature toggle.
  We treat local time as default feature without a lot of fuzz, and
  will eventually document that definitive support needs a compile
  time switch in the compiler (`--cfg unsound_local_offset` or something).
  
  One day it will perish. Failure is possible anyway and we will write
  code to deal with it while minimizing the amount of system time
  fetches when asking for the current local time.

### New Features

- <csr-id-eda39ec7d736d49af1ad9e2ad775e4aa12b264b7/> `gix config` with section and sub-section filtering.
- <csr-id-d99453ebeb970ed493be236def299d1e82b01f83/> `gix config` lists all entries of all configuration files git considers.
  Filters allow to narrow down the output.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 80 commits contributed to the release over the course of 106 calendar days.
- 108 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 7 unique issues were worked on: [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#325](https://github.com/GitoxideLabs/gitoxide/issues/325), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#382](https://github.com/GitoxideLabs/gitoxide/issues/382), [#386](https://github.com/GitoxideLabs/gitoxide/issues/386), [#396](https://github.com/GitoxideLabs/gitoxide/issues/396), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 3 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
  - Add empty git-discover crate ([`6565f16`](https://github.com/GitoxideLabs/gitoxide/commit/6565f163ba464f5adf63d214143a4e12f48b00fc))
  - Allow reading patterns from stdin ([`0c597fe`](https://github.com/GitoxideLabs/gitoxide/commit/0c597fe78acdd5672b4535a7d82620c5f7f93649))
  - Add `--show-ignore-patterns` to `gix repo exclude query` ([`09f904b`](https://github.com/GitoxideLabs/gitoxide/commit/09f904b1f393f03176882d491d7fffcad4058b49))
  - Basic prefix support as well the first working version of `exclude query` ([`9cb8385`](https://github.com/GitoxideLabs/gitoxide/commit/9cb83859f9bb76f38ab5bbd0ae6d6f20a691e9e1))
  - Support for overrides on the command-line ([`7d98b21`](https://github.com/GitoxideLabs/gitoxide/commit/7d98b2196c130263ace4a948418affdd950302ed))
  - Fix build ([`cb56f12`](https://github.com/GitoxideLabs/gitoxide/commit/cb56f12ad83cf2932a068ef4fa0ca5ce4aa73e84))
  - Refactor ([`3ff991d`](https://github.com/GitoxideLabs/gitoxide/commit/3ff991d0ca0d63632fc5710680351840f51c14c3))
  - Frame for `gix repo exclude query` ([`a331314`](https://github.com/GitoxideLabs/gitoxide/commit/a331314758629a93ba036245a5dd03cf4109dc52))
  - Add empty git-path crate ([`8d13f81`](https://github.com/GitoxideLabs/gitoxide/commit/8d13f81068b4663d322002a9617d39b307b63469))
  - Add git-filter crate for name generation ([`5a3c628`](https://github.com/GitoxideLabs/gitoxide/commit/5a3c628c757a7eb4ecfc5fd7265fa36c8362797b))
  - Make fmt ([`50ff7aa`](https://github.com/GitoxideLabs/gitoxide/commit/50ff7aa7fa86e5e2a94fb15aab86470532ac3f51))
  - Add git-glob crate with pattern matching parsing from git-attributes::ignore ([`b3efc94`](https://github.com/GitoxideLabs/gitoxide/commit/b3efc94134a32018db1d6a2d7f8cc397c4371999))
- **[#325](https://github.com/GitoxideLabs/gitoxide/issues/325)**
  - Empty crate for 'tix' tool ([`2efed08`](https://github.com/GitoxideLabs/gitoxide/commit/2efed088c572380a152f75dc7200f13fe3b055ad))
- **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
  - Fix journey tests after `gix` restructuring ([`59b95c9`](https://github.com/GitoxideLabs/gitoxide/commit/59b95c94aacac174e374048b7d11d2c0984a19e0))
  - Remove local-time-support feature toggle. ([`89a41bf`](https://github.com/GitoxideLabs/gitoxide/commit/89a41bf2b37db29b9983b4e5492cfd67ed490b23))
  - `gix config` with section and sub-section filtering. ([`eda39ec`](https://github.com/GitoxideLabs/gitoxide/commit/eda39ec7d736d49af1ad9e2ad775e4aa12b264b7))
  - `gix config` lists all entries of all configuration files git considers. ([`d99453e`](https://github.com/GitoxideLabs/gitoxide/commit/d99453ebeb970ed493be236def299d1e82b01f83))
  - Refactor ([`a437abe`](https://github.com/GitoxideLabs/gitoxide/commit/a437abe8e77ad07bf25a16f19ca046ebdaef42d6))
  - Move 'exclude' up one level and dissolve 'repo' subcommand ([`8e5b796`](https://github.com/GitoxideLabs/gitoxide/commit/8e5b796ea3fd760839f3c29a4f65bb42b1f3e893))
  - Move 'mailmap' up one level ([`5cf08ce`](https://github.com/GitoxideLabs/gitoxide/commit/5cf08ce3d04d635bbfee169cb77ce259efbf6bc3))
  - Move 'odb' up one level ([`0ed65da`](https://github.com/GitoxideLabs/gitoxide/commit/0ed65da9b66d4cc3c85d3b70fa4bc383c7a0d1a3))
  - Move 'tree' up one level ([`38a8350`](https://github.com/GitoxideLabs/gitoxide/commit/38a8350d75720a8455e9c55d12f7cdf4b1742e56))
  - Move 'commit' up one level ([`72876f1`](https://github.com/GitoxideLabs/gitoxide/commit/72876f1fd65efc816b704db6880ab881c89cff01))
  - Move 'verify' up one level ([`ac7d99a`](https://github.com/GitoxideLabs/gitoxide/commit/ac7d99ac42ff8561e81f476856d0bbe86b5fa627))
  - Move 'revision' one level up ([`c9c78e8`](https://github.com/GitoxideLabs/gitoxide/commit/c9c78e86c387c09838404c90de420892f41f4356))
  - Move 'remote' to 'free' ([`8967fcd`](https://github.com/GitoxideLabs/gitoxide/commit/8967fcd009260c2d32881866244ba673894775f2))
  - Move commitgraph to 'free' ([`f99c3b2`](https://github.com/GitoxideLabs/gitoxide/commit/f99c3b29cea30f1cbbea7e5855abfec3de6ca630))
  - Move index to 'free' ([`83585bd`](https://github.com/GitoxideLabs/gitoxide/commit/83585bdfccdc42b5307255b2d56d8cb12d4136cb))
  - Move 'pack' to 'free' ([`1cdecbc`](https://github.com/GitoxideLabs/gitoxide/commit/1cdecbc583ae412e7f25cade73b46e00a182125f))
  - Migrate mailmap to the new 'free' section ([`141c5f1`](https://github.com/GitoxideLabs/gitoxide/commit/141c5f1145f9d3864e2d879089c66c62f38a2b5d))
  - First step towards moving all repository-commands one level up. ([`f4e1810`](https://github.com/GitoxideLabs/gitoxide/commit/f4e1810fb711d57778be79c88f49aa583821abab))
  - Make obvious what plumbing and porcelain really are ([`faaf791`](https://github.com/GitoxideLabs/gitoxide/commit/faaf791cc960c37b180ddef9792dfabc7d106138))
  - Adjustments due to breaking changes in `git_path` ([`4420ae9`](https://github.com/GitoxideLabs/gitoxide/commit/4420ae932d5b20a9662a6d36353a27111b5cd672))
  - Frame for git-date ([`37e8ef8`](https://github.com/GitoxideLabs/gitoxide/commit/37e8ef890305db0798059919290a0d27a9a39194))
- **[#382](https://github.com/GitoxideLabs/gitoxide/issues/382)**
  - Add dummy crates for git-rebase and git-lfs ([`94c8122`](https://github.com/GitoxideLabs/gitoxide/commit/94c81226e2713c869462241429df608e04c4418d))
- **[#386](https://github.com/GitoxideLabs/gitoxide/issues/386)**
  - Add frame for git-credentials crate ([`be7a9cf`](https://github.com/GitoxideLabs/gitoxide/commit/be7a9cf776f958ac7228457bb4e1415f86f8e575))
  - An empty crate for git-sec ([`96a922c`](https://github.com/GitoxideLabs/gitoxide/commit/96a922c4c9be194aaa4928fb21c9690a5c6e4445))
- **[#396](https://github.com/GitoxideLabs/gitoxide/issues/396)**
  - Add git-sequencer crate ([`f8160d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8160d0d0caa6e616a6d0efed346722d093ad60a))
- **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
  - Add rough but working version of `rev-parse` ([`f3f176d`](https://github.com/GitoxideLabs/gitoxide/commit/f3f176db42cef4036cc7c0ced1ee68f247424896))
  - Basic infrastructure for delegate implementation ([`d3c0bc6`](https://github.com/GitoxideLabs/gitoxide/commit/d3c0bc6e8d7764728f4e10500bb895152ccd0b0b))
  - Hookup explain command ([`1049b00`](https://github.com/GitoxideLabs/gitoxide/commit/1049b00eaa261a67f060eaca4eb50dcda831eafd))
  - Frame for `gix repo rev explain` ([`12e6277`](https://github.com/GitoxideLabs/gitoxide/commit/12e6277a65a6572a0e43e8324d2d1dfb23d0bb40))
- **Uncategorized**
  - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
  - Merge branch 'gix-repo-config' ([`afecb63`](https://github.com/GitoxideLabs/gitoxide/commit/afecb6337dcf0fc51d5c74747c3c60fa06ae6346))
  - Thanks clippy ([`48b3f4a`](https://github.com/GitoxideLabs/gitoxide/commit/48b3f4a5077ba66d47482a80e505feb69e9ac9fc))
  - Merge branch 'config-cascade' ([`f144eaf`](https://github.com/GitoxideLabs/gitoxide/commit/f144eaf5863ae5cac63103f0db51c35fcf03a948))
  - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
  - Make fmt ([`0700b09`](https://github.com/GitoxideLabs/gitoxide/commit/0700b09d6828849fa2470df89af1f75a67bfb27d))
  - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
  - Merge branch 'main' into cont_include_if ([`0e9df36`](https://github.com/GitoxideLabs/gitoxide/commit/0e9df364c4cddf006b1de18b8d167319b7cc1186))
  - Use git_path::realpath in all places that allow it right now ([`229dc91`](https://github.com/GitoxideLabs/gitoxide/commit/229dc917fc7d9241b85e5818260a6fbdd3a5daaa))
  - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
  - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
  - Use clap 3.2.5 to be able to opt-in to deprecations ([`aaf1cde`](https://github.com/GitoxideLabs/gitoxide/commit/aaf1cdedf7bd181977faa66ef21f7ee75627bf9f))
  - Pin clap to v3.1 to avoid surprising deprecation warnings ([`ac05284`](https://github.com/GitoxideLabs/gitoxide/commit/ac0528425e455dd2140623decbd946e66d202e8d))
  - Hopefully fix msrv check and bring tools back into workspace ([`ad2f134`](https://github.com/GitoxideLabs/gitoxide/commit/ad2f134e5dce94cb4d63ed00ea93d44a1436cc25))
  - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
  - Also remove cargo-smart-release from workspace ([`8ef5197`](https://github.com/GitoxideLabs/gitoxide/commit/8ef5197178f879bdd5d191dba331ac318626bfb5))
  - Remove experiemnts from workspace tree ([`0ab101d`](https://github.com/GitoxideLabs/gitoxide/commit/0ab101d685632a4cf27a59551b7e2c4e68683792))
  - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
  - Release git-path v0.1.3, git-discover v0.1.2, git-repository v0.18.1, cargo-smart-release v0.10.1 ([`b7399cc`](https://github.com/GitoxideLabs/gitoxide/commit/b7399cc44ee419355a649a7b0ba7b352cd48b400))
  - Release git-path v0.1.2, git-sec v0.1.1, git-config v0.4.0, git-discover v0.1.1, git-pack v0.19.1, git-repository v0.18.0, cargo-smart-release v0.10.0, safety bump 2 crates ([`ceb6dff`](https://github.com/GitoxideLabs/gitoxide/commit/ceb6dff13362a2b4318a551893217c1d11643b9f))
  - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
  - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
  - Merge branch 'main' into git_includeif ([`b1bfc8f`](https://github.com/GitoxideLabs/gitoxide/commit/b1bfc8fe8efb6d8941f54dddd0fcad99aa13ed6c))
  - Merge branch 'basic-worktree-support' ([`e058bda`](https://github.com/GitoxideLabs/gitoxide/commit/e058bdabf8449b6a6fdff851e3929137d9b71568))
  - Merge branch 'main' into git_includeif ([`05eb340`](https://github.com/GitoxideLabs/gitoxide/commit/05eb34023933918c51c03cf2afd774db89cc5a33))
  - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
  - Make fmt ([`251b6df`](https://github.com/GitoxideLabs/gitoxide/commit/251b6df5dbdda24b7bdc452085f808f3acef69d8))
  - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
  - Thanks clippy ([`056e8d2`](https://github.com/GitoxideLabs/gitoxide/commit/056e8d26dc511fe7939ec87c62ef16aafd34fa9c))
  - Thanks clippy ([`fdec111`](https://github.com/GitoxideLabs/gitoxide/commit/fdec11135692b3503087b0a3245c12cc87554d67))
  - Merge branch 'main' into repo-status ([`9679d6b`](https://github.com/GitoxideLabs/gitoxide/commit/9679d6b0e68c28438e22cb65c554d0b31dfaf159))
  - Merge branch 'git-sec' ([`cd723b5`](https://github.com/GitoxideLabs/gitoxide/commit/cd723b5ae11148e7e9fd07daf28bc04455d5c46f))
  - Merge branch 'main' into git-sec ([`2fe70f9`](https://github.com/GitoxideLabs/gitoxide/commit/2fe70f96cfb68e108637ce78f8edda2eb4e2e61a))
  - Merge branch 'main' into repo-status ([`8122c5c`](https://github.com/GitoxideLabs/gitoxide/commit/8122c5c8a122e5297832f96c07a6fb1a5df02620))
  - Merge branch 'main' into repo-status ([`0eb2372`](https://github.com/GitoxideLabs/gitoxide/commit/0eb23721dca78f6e6bf864c5c3a3e44df8b419f0))
  - Merge branch 'main' into repo-status ([`4086335`](https://github.com/GitoxideLabs/gitoxide/commit/40863353a739ec971b49410fbc2ba048b2762732))
  - Release git-glob v0.2.0, safety bump 3 crates ([`ab6bed7`](https://github.com/GitoxideLabs/gitoxide/commit/ab6bed7e2aa19eeb9990441741008c430f373708))
  - Merge branch 'worktree-stack' ([`e90d3fd`](https://github.com/GitoxideLabs/gitoxide/commit/e90d3fd0a9764511e6280596f21d3a0494ed7021))

</details>

## 0.12.0 (2022-04-05)

### New Features

- <csr-id-7e99e6aeee9bf200a561d215c586301f5e4a8cbc/> Add `gix repo commit describe`
  It supports typical but basic flags mostly similar to the ones in git.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 1 calendar day.
- 2 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#298](https://github.com/GitoxideLabs/gitoxide/issues/298)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
  - Use all tags by default, instead of requiring annotated tags ([`00c42ca`](https://github.com/GitoxideLabs/gitoxide/commit/00c42ca36e93a22f233fc1d3f9a1afc241fd4464))
  - Support for the --max-candidates flag ([`b9e6754`](https://github.com/GitoxideLabs/gitoxide/commit/b9e67540801f2630be8aa1acbfddfec4202360ac))
  - Reduce amount of max candidates, add --debug flag ([`c8c13e3`](https://github.com/GitoxideLabs/gitoxide/commit/c8c13e398671a21e96282547fc0e3bd445627e2f))
  - Add `gix repo commit describe` ([`7e99e6a`](https://github.com/GitoxideLabs/gitoxide/commit/7e99e6aeee9bf200a561d215c586301f5e4a8cbc))
  - A first sketch of the `gix repo describe` plumbing command ([`2d6ccef`](https://github.com/GitoxideLabs/gitoxide/commit/2d6ccefd5506d84ba14e3ff11c2af4cb107a386d))
- **Uncategorized**
  - Release git-config v0.2.1, git-diff v0.15.0, git-traverse v0.14.0, git-pack v0.18.0, git-odb v0.28.0, git-ref v0.12.1, git-revision v0.1.0, git-repository v0.16.0, gitoxide-core v0.14.0, gitoxide v0.12.0, safety bump 6 crates ([`b612021`](https://github.com/GitoxideLabs/gitoxide/commit/b612021683ba709b693bd48aef3e2e3c2f5b9ead))

</details>

## 0.11.0 (2022-04-03)

<csr-id-4d2d433e7e98ac42db858688edac06e68ee4d10d/>

Adapt to changes in `gix-features` which change `Send + Sync` to `Send + Clone`. This happens to allow non-sync implementations (i.e. thread-local), along with `Sync` ones
which usually are `Clone` too as they are passed by immutable reference (which is `Clone + Copy`).

### Changed (BREAKING)

- <csr-id-bf04644ab75ed1969507f957dc8d4868790d462d/> remove `Option<impl Progress>` in favor of `impl Progress`
- <csr-id-d851bede97801096d188ff6af06c98a79fe276db/> remove unnecessary `Arc` around `should_interrupt` flag
- <csr-id-c2679a03358b9c19d63ed1af1cd57324c6381447/> remove Sha1 mentions in `index::verify::Mode::*` variants
  The hash is repository defined and not hard-coded
- <csr-id-51bf03feaa94bebb26690dff92262b2134070a44/> Remove lean plumbing CLI
- <csr-id-e8b091943f0c9a26317da0003f7fcdf5a56ef21a/> Rename gix->ein and gixp->gix

### Bug Fixes

- <csr-id-57ca0456cf02073099bfd403f9155290af756ecd/> Collect all stdout messages in line renderer as well
  Otherwise the threaded line renderer will interfere with genuine
  program output.

### New Features

- <csr-id-384ed665c7423feca1b1ee1f81db10867fa813a8/> `gix mailmap verify` command
- <csr-id-70109bee679d33a5c5fb3a78a708b479684b03b1/> `ein find --debug` to learn why it is slow
- <csr-id-00909619ff04e247aabc9ffe3c025f0064c3092d/> --counting-threads flag to configure amount of threads when counting
  The efficiency of multi-threaded counting is low per core, and despite
  some speedups might be desirable, one might not want to commit all cores
  to this amount of waste.
- <csr-id-25da30f3652bd72c157e84439dd6e3957471fa08/> in-manifest and in-bin documentation of feature toggles
  Unfortunately, these don't show up on docs.rs due to it being a binary
  only crate. One could consider throwing in a lib just for good measure.
- <csr-id-cb83beedd1aa389f6774e2296f79273e8c8f14f4/> git-hash::Prefix::from_id()
  A way to obtain a prefix of an object id, with all non-prefix
  bytes set to zero.
- <csr-id-7e95d8ab29051ffc892f2dcbaf5369e8c7e7b294/> add threading primitives with feature toggle
  If the `threading` feature is set, the `threading` module will contain thread-safe primitives
  for shared ownership and mutation, otherwise these will be their single threaded counterparts.
  
  This way, single-threaded applications don't have to pay for threaded primitives.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 98 commits contributed to the release over the course of 156 calendar days.
- 165 days passed between releases.
- 13 commits were understood as [conventional](https://www.conventionalcommits.org).
- 15 unique issues were worked on: [#215](https://github.com/GitoxideLabs/gitoxide/issues/215), [#247](https://github.com/GitoxideLabs/gitoxide/issues/247), [#259](https://github.com/GitoxideLabs/gitoxide/issues/259), [#262](https://github.com/GitoxideLabs/gitoxide/issues/262), [#263](https://github.com/GitoxideLabs/gitoxide/issues/263), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#287](https://github.com/GitoxideLabs/gitoxide/issues/287), [#289](https://github.com/GitoxideLabs/gitoxide/issues/289), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#366](https://github.com/GitoxideLabs/gitoxide/issues/366), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#215](https://github.com/GitoxideLabs/gitoxide/issues/215)**
  - Collect all stdout messages in line renderer as well ([`57ca045`](https://github.com/GitoxideLabs/gitoxide/commit/57ca0456cf02073099bfd403f9155290af756ecd))
  - Fix compile warning ([`e4514a8`](https://github.com/GitoxideLabs/gitoxide/commit/e4514a85d406aaa0aa959a18e0e32d46f1994cc8))
  - Remove reference of pretty-cli in code tree ([`4bd2f29`](https://github.com/GitoxideLabs/gitoxide/commit/4bd2f29da7e37c5d6e920c97df82c7860dd9f22c))
  - Remove lean plumbing CLI ([`51bf03f`](https://github.com/GitoxideLabs/gitoxide/commit/51bf03feaa94bebb26690dff92262b2134070a44))
  - Remove light* features, add 'lean-async' in its place; remove termion support ([`4d2d433`](https://github.com/GitoxideLabs/gitoxide/commit/4d2d433e7e98ac42db858688edac06e68ee4d10d))
  - Remove all targets we don't want anymore ([`2abba9c`](https://github.com/GitoxideLabs/gitoxide/commit/2abba9cb5ab2424a2634dce2e283f6fb6aedfc5e))
- **[#247](https://github.com/GitoxideLabs/gitoxide/issues/247)**
  - Rename gix->ein and gixp->gix ([`e8b0919`](https://github.com/GitoxideLabs/gitoxide/commit/e8b091943f0c9a26317da0003f7fcdf5a56ef21a))
- **[#259](https://github.com/GitoxideLabs/gitoxide/issues/259)**
  - Add experiment for type system ([`761c63a`](https://github.com/GitoxideLabs/gitoxide/commit/761c63a3cd8303a1272da3f1f90735e2c14cd34a))
- **[#262](https://github.com/GitoxideLabs/gitoxide/issues/262)**
  - Unwind instead of aborting ([`4d7e8a4`](https://github.com/GitoxideLabs/gitoxide/commit/4d7e8a45bbe0d8c9253c46ea9d690ae620854dd7))
- **[#263](https://github.com/GitoxideLabs/gitoxide/issues/263)**
  - Fmt ([`fbeddeb`](https://github.com/GitoxideLabs/gitoxide/commit/fbeddebcab999f4898f768a3184906091f8ce0b8))
  - A mad attempt to use thread-local everywhere and avoid Sync… ([`0af5077`](https://github.com/GitoxideLabs/gitoxide/commit/0af5077e1f028c1c69bbdc098bb567e486282c37))
  - Add threading primitives with feature toggle ([`7e95d8a`](https://github.com/GitoxideLabs/gitoxide/commit/7e95d8ab29051ffc892f2dcbaf5369e8c7e7b294))
- **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
  - Delete now unused experiment ([`c44f72e`](https://github.com/GitoxideLabs/gitoxide/commit/c44f72e9590aad456a7a921127c973e60d26dfb6))
  - Provide handle with a snapshot of the store's state ([`6e0cd6d`](https://github.com/GitoxideLabs/gitoxide/commit/6e0cd6d38c5df874990ace6c2c3c0b39342c4d05))
- **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
  - Fast-path multi-pack index verification in the CLI ([`bcde935`](https://github.com/GitoxideLabs/gitoxide/commit/bcde935e7102ba5cd50c057a8323353247d3dd85))
  - Basic multi-pack index creation ([`89428b2`](https://github.com/GitoxideLabs/gitoxide/commit/89428b2936fb0169606a543cf531bddaacb8187c))
  - 'index' with its own sub-commands ([`c4c5678`](https://github.com/GitoxideLabs/gitoxide/commit/c4c56787b1f9165984a8bddf35cfee530554fa2f))
  - Even nicer printing ([`d2bea27`](https://github.com/GitoxideLabs/gitoxide/commit/d2bea270787597d6aef48ffe023ff49969c33bd9))
  - Remove `Option<impl Progress>` in favor of `impl Progress` ([`bf04644`](https://github.com/GitoxideLabs/gitoxide/commit/bf04644ab75ed1969507f957dc8d4868790d462d))
  - Remove unnecessary `Arc` around `should_interrupt` flag ([`d851bed`](https://github.com/GitoxideLabs/gitoxide/commit/d851bede97801096d188ff6af06c98a79fe276db))
  - Remove Sha1 mentions in `index::verify::Mode::*` variants ([`c2679a0`](https://github.com/GitoxideLabs/gitoxide/commit/c2679a03358b9c19d63ed1af1cd57324c6381447))
  - Frame for git-chunk crate to share among git-pack and git-commitgraph ([`b2d2ae2`](https://github.com/GitoxideLabs/gitoxide/commit/b2d2ae221d43cc14aa169ada3c471e2bd2adadf4))
- **[#287](https://github.com/GitoxideLabs/gitoxide/issues/287)**
  - Share and pass cli arguments for pack verification ([`db43e47`](https://github.com/GitoxideLabs/gitoxide/commit/db43e47fc0a43ef45824ac1c9426c1889bdb13a3))
  - Upgrade to prodash 17 ([`47860b7`](https://github.com/GitoxideLabs/gitoxide/commit/47860b7e2769260cfb8522ae455c491605093423))
  - Very rough version of repository verification ([`80a4a7a`](https://github.com/GitoxideLabs/gitoxide/commit/80a4a7add688d16376b9bf2ed7f1c7f655b7c912))
  - Adjustments to deal with changes to git-pack/git-odb ([`fcf8fde`](https://github.com/GitoxideLabs/gitoxide/commit/fcf8fde7272974a70df808bd7ac03e925b7e39a8))
- **[#289](https://github.com/GitoxideLabs/gitoxide/issues/289)**
  - 'pack' with its own sub-commands ([`fb64af4`](https://github.com/GitoxideLabs/gitoxide/commit/fb64af4d747960bfa40ec23051ecb03ea8ec5d83))
  - 'remote' with its own sub-commands ([`8677f7e`](https://github.com/GitoxideLabs/gitoxide/commit/8677f7edd516ea54ec652a4a59cb220422036b90))
  - 'commitgraph' with its own sub-commands ([`db0251e`](https://github.com/GitoxideLabs/gitoxide/commit/db0251e277ee9035bd3b44bf5ec9152fb64ac8c8))
- **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
  - Faster writing to stdout/stderr for plumbing commands ([`d04dc01`](https://github.com/GitoxideLabs/gitoxide/commit/d04dc01115efa6688e71a2a0ef4ffce45d3d0db6))
  - Add 'index verify' subcommand to 'gix' ([`1ac2c21`](https://github.com/GitoxideLabs/gitoxide/commit/1ac2c210c311c4b2ef835e04e2d7c477981b850f))
  - Flag to hide extension details ([`34ea001`](https://github.com/GitoxideLabs/gitoxide/commit/34ea001fafa93b6453513cf458fe24327a13ff28))
  - Print basic index information, including the tree extension ([`9277cf8`](https://github.com/GitoxideLabs/gitoxide/commit/9277cf877e1f2276dcad1efdeb97e0e3d96ed3f0))
  - Basic entry information ([`239e7b2`](https://github.com/GitoxideLabs/gitoxide/commit/239e7b291297d6d49ebdf3d4986fb9fb86480e9a))
  - Refactor ([`8bf585d`](https://github.com/GitoxideLabs/gitoxide/commit/8bf585d67cd67b168d819ba05858cef7d9b90894))
  - JSON output for index entries ([`3fc1622`](https://github.com/GitoxideLabs/gitoxide/commit/3fc1622488054c6ab655eb9d2f941b68cc3ccf18))
  - Add git-bitmap crate for use in git-index ([`a517f26`](https://github.com/GitoxideLabs/gitoxide/commit/a517f2697678f31e29ec9982d02ccfec6a777bbf))
  - Preempt the eventual need for a worktree implementation ([`bce67d8`](https://github.com/GitoxideLabs/gitoxide/commit/bce67d8ec58f78a1fce1c76f7b93d9650f9f550e))
- **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
  - Git-hash::Prefix::from_id() ([`cb83bee`](https://github.com/GitoxideLabs/gitoxide/commit/cb83beedd1aa389f6774e2296f79273e8c8f14f4))
  - Simplify command-line options declaration ([`f790a55`](https://github.com/GitoxideLabs/gitoxide/commit/f790a55ff4263bea9b9476137bac3824912044ac))
  - Frame for printing index information ([`9ea98fd`](https://github.com/GitoxideLabs/gitoxide/commit/9ea98fda75fbef339647a0ca03776060356d1206))
- **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
  - Add empty git-quote crate ([`0d1aaf0`](https://github.com/GitoxideLabs/gitoxide/commit/0d1aaf00160f98e40fb92fd401c67f59da2475fc))
  - Greatly simplify render-line logic ([`a8fa53a`](https://github.com/GitoxideLabs/gitoxide/commit/a8fa53a007780ec89f4768745b1549e8e73a8478))
  - Add future crate for good measure ([`625eb1d`](https://github.com/GitoxideLabs/gitoxide/commit/625eb1d7d266036c1f30caab7bd3897af9fdbef7))
  - Remove git-ignore as it's already taken ([`020ccc5`](https://github.com/GitoxideLabs/gitoxide/commit/020ccc576a3332c15014edf26b3ee51523265473))
  - Name-crates for git-ignore and git-attributes handling ([`2e04a49`](https://github.com/GitoxideLabs/gitoxide/commit/2e04a4934a42cc2bb90334cf75e4af2ab394cffa))
  - Pass thread-limit along to checkout ([`07e9081`](https://github.com/GitoxideLabs/gitoxide/commit/07e9081fb5628e4ddc8f87e2d4ba0c7b3247bb35))
  - Add thread-count and chunk-size computation; interrupt capability ([`8cbe85d`](https://github.com/GitoxideLabs/gitoxide/commit/8cbe85d135898826a91939726465a9e295c1e24b))
  - A first sketch of access odb information using a sub-command ([`89b628a`](https://github.com/GitoxideLabs/gitoxide/commit/89b628ab5b833a34f0b426b3a399bb182e63f3f4))
  - Sub-command to print multi-index entries ([`6c10e09`](https://github.com/GitoxideLabs/gitoxide/commit/6c10e097a432d81b930008abc00c6821ed7ac9be))
  - Pack multi-index info subcommand ([`21c2dd5`](https://github.com/GitoxideLabs/gitoxide/commit/21c2dd5da20a9e3cbae618b6311b6c9de12cf43c))
  - Refactor ([`e6a3d43`](https://github.com/GitoxideLabs/gitoxide/commit/e6a3d437e1a97c56fba18d80ac54928d953cb507))
  - Detailed report about issues after checkout ([`613483b`](https://github.com/GitoxideLabs/gitoxide/commit/613483b297b8a7e9a91cac3ef8205f2103ea946b))
  - Keep-going support on the command-line ([`73a7393`](https://github.com/GitoxideLabs/gitoxide/commit/73a73932f430fe991f26222ba2735332c03c0e77))
  - Add tree-info subcommand to more easily test actual tree-traversal performance ([`29fb0c8`](https://github.com/GitoxideLabs/gitoxide/commit/29fb0c8ff628716d33c9c41d3910e617791dcc77))
  - Frame for traversing tree entries ([`0e55fbb`](https://github.com/GitoxideLabs/gitoxide/commit/0e55fbb2fb0cec6f402b7a3aed7ee55078d233a1))
  - Add stub for git-submodule ([`3f9bcd8`](https://github.com/GitoxideLabs/gitoxide/commit/3f9bcd8a0608dbdc5673da055c65d9dc995f03f1))
  - Properly use 'max-performance' feature toggle to get pack caches :D ([`a39d476`](https://github.com/GitoxideLabs/gitoxide/commit/a39d4768e36f27aababefd5bd519e51f33ffa7b6))
  - Allow writing empty files during checkout but also query the odb ([`5388d80`](https://github.com/GitoxideLabs/gitoxide/commit/5388d8091ef02cf927478a1492847ae1666040d4))
  - Support for repo to write actual objects ([`5494fb3`](https://github.com/GitoxideLabs/gitoxide/commit/5494fb3e1de1234dde8c47336597283dbd8bcb29))
  - Basic version of index checkout via command-line ([`f23b8d2`](https://github.com/GitoxideLabs/gitoxide/commit/f23b8d2f1c4b767d337ec51888afaa8b3719798c))
  - Support for unicode-precomposition for gix apps ([`e90c123`](https://github.com/GitoxideLabs/gitoxide/commit/e90c123675a98ab62fc6bb22019f889cee8b7301))
- **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
  - In-manifest and in-bin documentation of feature toggles ([`25da30f`](https://github.com/GitoxideLabs/gitoxide/commit/25da30f3652bd72c157e84439dd6e3957471fa08))
- **[#366](https://github.com/GitoxideLabs/gitoxide/issues/366)**
  - Frame for printing mailmap entries using git-repository ([`2a01f47`](https://github.com/GitoxideLabs/gitoxide/commit/2a01f4728ae858b47280b587501d343fdb86655d))
  - Gix mailmap verify can now detect collisions ([`f89fe2f`](https://github.com/GitoxideLabs/gitoxide/commit/f89fe2f867fa792db5d9e003ce342a337a6ac973))
  - `gix mailmap verify` command ([`384ed66`](https://github.com/GitoxideLabs/gitoxide/commit/384ed665c7423feca1b1ee1f81db10867fa813a8))
  - Empty git-mailmap crate ([`a30a5da`](https://github.com/GitoxideLabs/gitoxide/commit/a30a5da60d67a4e52ba69727318edb9832b7cae2))
- **[#67](https://github.com/GitoxideLabs/gitoxide/issues/67)**
  - --counting-threads flag to configure amount of threads when counting ([`0090961`](https://github.com/GitoxideLabs/gitoxide/commit/00909619ff04e247aabc9ffe3c025f0064c3092d))
- **Uncategorized**
  - Release git-commitgraph v0.7.0, gitoxide-core v0.13.0, gitoxide v0.11.0 ([`ab08a7f`](https://github.com/GitoxideLabs/gitoxide/commit/ab08a7f066fb65671868424315d958ae985d76d8))
  - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
  - Empty crate for git-note ([`2fb8b46`](https://github.com/GitoxideLabs/gitoxide/commit/2fb8b46abd3905bdf654977e77ec2f36f09e754f))
  - Make fmt ([`7cf3545`](https://github.com/GitoxideLabs/gitoxide/commit/7cf354509b545f7e7c99e159b5989ddfbe86273d))
  - Small build now uses the line renderer as well ([`652a0ac`](https://github.com/GitoxideLabs/gitoxide/commit/652a0acdf9f06e35e65c1b66d264d5e8734ccc65))
  - Upgrade to prodash 19 ([`90c6c5a`](https://github.com/GitoxideLabs/gitoxide/commit/90c6c5aec4015ff969d6e2514fa4d49873ee80f5))
  - `ein find --debug` to learn why it is slow ([`70109be`](https://github.com/GitoxideLabs/gitoxide/commit/70109bee679d33a5c5fb3a78a708b479684b03b1))
  - Merge branch 'short-id' ([`5849d5b`](https://github.com/GitoxideLabs/gitoxide/commit/5849d5b326b83f98a16cf1d956c720c7f0fd4445))
  - Fix clap warnings ([`aa51e05`](https://github.com/GitoxideLabs/gitoxide/commit/aa51e05923695e20aecc16317331c7e26d49a2e7))
  - Release git-tempfile v2.0.0, safety bump 6 crates ([`90b1c42`](https://github.com/GitoxideLabs/gitoxide/commit/90b1c42d5487904a9f329362d185b035d0ddb975))
  - Upgrade document-features ([`c35e62e`](https://github.com/GitoxideLabs/gitoxide/commit/c35e62e0da9ac1f7dcb863f5f9c69108c728d32e))
  - Rename git-rev to git-revision ([`2e939c9`](https://github.com/GitoxideLabs/gitoxide/commit/2e939c973ab3635a946317af08f37c4e23450f18))
  - Add git-rev crate ([`9381c5d`](https://github.com/GitoxideLabs/gitoxide/commit/9381c5de5e68c599af3178440fd6af3e28e770c5))
  - Merge branch 'index-verification' ([`ad3c803`](https://github.com/GitoxideLabs/gitoxide/commit/ad3c8032cee02052ef3940d1d7c950270a0a299a))
  - Merge branch 'index-information' ([`025f157`](https://github.com/GitoxideLabs/gitoxide/commit/025f157de10a509a4b36a9aed41de80487e8c15c))
  - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
  - Upgrade to tui 0.17 and prodash 18 ([`eba101a`](https://github.com/GitoxideLabs/gitoxide/commit/eba101a576ecb7bc0f63357d0dd81eb817b94be4))
  - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
  - Improve CLI docs ([`866530a`](https://github.com/GitoxideLabs/gitoxide/commit/866530a154c3ef9383fae30c694991e31e97528c))
  - Rename 'gix commitgraph' back to 'gix commit-graph' ([`d6a72e7`](https://github.com/GitoxideLabs/gitoxide/commit/d6a72e70c9b4ee9b10a1172cce64ade5664599eb))
  - Upgrade to clap 3.0.0 ([`3325d50`](https://github.com/GitoxideLabs/gitoxide/commit/3325d5022a43085e93d4c7b65b01b65b36e2f77d))
  - Merge branch 'use-midx-in-store' ([`338521b`](https://github.com/GitoxideLabs/gitoxide/commit/338521b0443b9dc1007581de42ef6a950f6e0bbf))
  - Release git-chunk v0.2.0, safety bump 4 crates ([`b792fab`](https://github.com/GitoxideLabs/gitoxide/commit/b792fabf9f5f93ab906ac5a5bb3e4f01c179290a))
  - Upgrade to latest clap rc ([`1b76db0`](https://github.com/GitoxideLabs/gitoxide/commit/1b76db05e097c4106de3d326079cbff83629589b))
  - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
  - Thanks clippy ([`b0f7328`](https://github.com/GitoxideLabs/gitoxide/commit/b0f73280c0233e05b68a22b0b01f40d574786a03))
  - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
  - Optimize certain dependencies for speedier test runs ([`4a948d0`](https://github.com/GitoxideLabs/gitoxide/commit/4a948d0cd111fd94bdecfce228d4ee286503fea5))
  - Merge branch 'pack-consistency' ([`5982406`](https://github.com/GitoxideLabs/gitoxide/commit/5982406b4e1b26fd383d9ec21a3cf652ec8ab25f))
  - Adjusting changelogs prior to release of git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0, safety bump 5 crates ([`39b40c8`](https://github.com/GitoxideLabs/gitoxide/commit/39b40c8c3691029cc146b893fa0d8d25d56d0819))

</details>

## v0.10.0 (2021-10-20)

This release pins beta versions of `clap` to avoid it to automatically fetch the latest one
during installation.

This is made possible due to `clap` itself pinning its dependency
to the `clap-derive` crate.

### Bug Fixes

- <csr-id-dfc588b25ede3faa578eb8e131e73c857117a6df/> pin version of clap to beta 5
  This assures we don't get broken automatically in future.
  Previously that wasn't possible as the dependency of `clap`,
  `clap-derive` was also using a beta verion and wasn't constrained,
  hence it would be updated and cause breaking changes with pinned
  versions of consumers of `clap`.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release over the course of 1 calendar day.
- 5 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#224](https://github.com/GitoxideLabs/gitoxide/issues/224)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#222](https://github.com/GitoxideLabs/gitoxide/issues/222)**
  - Upgrade to clap 3 beta 5 ([`2ddc4ed`](https://github.com/GitoxideLabs/gitoxide/commit/2ddc4eddda23c77b5891a11a3e7215702c63882b))
- **[#224](https://github.com/GitoxideLabs/gitoxide/issues/224)**
  - Pin version of clap to beta 5 ([`dfc588b`](https://github.com/GitoxideLabs/gitoxide/commit/dfc588b25ede3faa578eb8e131e73c857117a6df))
- **Uncategorized**
  - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))

</details>

## v0.9.0 (2021-10-15)

A first usable version of `gix` to make using `gitoxide` from your applications so much easier. It serves as a one-stop shop for application developers without sacrificing performance by default while making common use-cases more convenient.

### Chore

- <csr-id-aba8e5603833c85302db0b610802286a03a084df/> remove old and unnecessary experiment

### Feature list

* `gix` as hub crate for application development with focus on usability without sacrificing any knob to tune performance.
* opt-in `async` for `gix-packetline`, `gix-transport` and `gix-protocol` for fully async git clients, along with the `light-async` feature toggle to build a `gix pack-receive` with an async client instead of a blocking one.
* Statistics for `gix pack-create` with the `-s/--statistics` flag to have data indicating the cost of the operation. Currently it's doing a lot of work that has to be avoided in order to be useable in production and the numbers underline that. Future iterations will cause key metrics to go down.
* Packs are now reproducible by default, which means that the same tip will always generate a pack with the same hash. This may be a desirable property for some kinds of packs, but not for others which is why it can be turned off for a considerable speed boost.
* `gix-tempfile` crate
* `gix-lock` crate
* `gix-ref` crate with complete loose-ref, packed-ref and transaction support.


### Performance

* On M1, thanks to [a new release](https://github.com/RustCrypto/hashes/pull/289#event-5035369215), Sha1 is now computed much faster which unlocks a massive performance boost. In my test, verifying/decoding the entire linux kernel pack now happens in 17s, as compared to 37s for canonical `git`.
* `gix-object` parsing is a few percent faster thanks a reworked error handling for objects. By default, error collection is disabled entirely making the error case zero-sized. If needed, verbose and stacked errors can be turned on using a feature toggle for applications who expect repositories with malformed objects and need detailed diagnostics.

### New Features

- <csr-id-60c9fad8002b4e3f6b9607bba6361871752f4d3d/> control pack and object cache size in megabytes in some sub-commands

### Commit Statistics

<csr-read-only-do-not-edit/>

- 10 commits contributed to the release over the course of 34 calendar days.
- 35 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 3 unique issues were worked on: [#164](https://github.com/GitoxideLabs/gitoxide/issues/164), [#200](https://github.com/GitoxideLabs/gitoxide/issues/200), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67)

## v0.8.4 (2021-09-10)

This is a maintenance release.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 4 commits contributed to the release over the course of 1 calendar day.
- 3 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gitoxide v0.8.4 ([`effb2a6`](https://github.com/GitoxideLabs/gitoxide/commit/effb2a612d5912ea7bd9e7c65465ca3da3797a7a))
  - Release gitoxide-core v0.10.5 ([`590e59b`](https://github.com/GitoxideLabs/gitoxide/commit/590e59b2b41a419574443e6b850bdb119a172279))
  - [smart-release #194] basic journey test setup ([`d5d90a6`](https://github.com/GitoxideLabs/gitoxide/commit/d5d90a654170c32750ef26872b72a6080184ac5d))
  - Bump git-repository v0.9.0 ([`b797fc1`](https://github.com/GitoxideLabs/gitoxide/commit/b797fc10f3f3d1fbc23916a4ff6e5e860e2dd4ed))

</details>

## v0.8.3 (2021-09-07)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 12 commits contributed to the release over the course of 15 calendar days.
- 21 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

## v0.8.2 (2021-08-17)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 1 calendar day.
- 2 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

## v0.8.1 (2021-08-15)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 74 commits contributed to the release over the course of 95 calendar days.
- 98 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#83](https://github.com/GitoxideLabs/gitoxide/issues/83)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

## v0.7.0 (2021-05-09)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 66 commits contributed to the release over the course of 128 calendar days.
- 144 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 2 unique issues were worked on: [#56](https://github.com/GitoxideLabs/gitoxide/issues/56), [#63](https://github.com/GitoxideLabs/gitoxide/issues/63)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 3 times to make code idiomatic. 

## v0.6.0 (2020-12-16)

Maintenance release without any new features.

These are created to account for breaking changes within the dependency graph of
`gitoxide` crates. Due to some blunders in the past the version on crates.io
could not be installed anymore.
This was eventually fixed with new minor releases across the ecosystem.

Finally, yet another breaking change due to the introduction of the `gix-hash`
crate to break a dependency cycle between `gix-object` and `gix-features` caused
yet another maintenance release.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 5 commits contributed to the release.
- 1 day passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - (cargo-release) version 0.6.0 ([`b5c270d`](https://github.com/GitoxideLabs/gitoxide/commit/b5c270dbe095753bd63e753fae3a6bfc8c5a3aac))
  - (cargo-release) version 0.5.0 ([`28df5e9`](https://github.com/GitoxideLabs/gitoxide/commit/28df5e9131aec3efb2b68db204662b92b232b33c))
  - All crates use git-hash::Kind and its types, sometimes through git-object ([`124c171`](https://github.com/GitoxideLabs/gitoxide/commit/124c171aaf546d8977e9913ff84e65383a80ee98))
  - Use git-hash in git-features ([`5b307e0`](https://github.com/GitoxideLabs/gitoxide/commit/5b307e076f6f5975592c8b177c122c91c1d809c6))
  - First incarnation of git-hash to separate concerns and resolve cycle ([`9803041`](https://github.com/GitoxideLabs/gitoxide/commit/9803041c29c18f2976531c9b487e63cd90fa3e72))

</details>

## v0.5.0 (2020-12-15)

Maintenance release without any new features.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 19 commits contributed to the release over the course of 78 calendar days.
- 84 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - (cargo-release) version 0.5.0 ([`28763ee`](https://github.com/GitoxideLabs/gitoxide/commit/28763eee35f8475ba9d7505a2176c9691b11afdb))
  - (cargo-release) version 0.6.0 ([`4df97ce`](https://github.com/GitoxideLabs/gitoxide/commit/4df97ce6869a53a688b9af18405b284d9ff27b24))
  - (cargo-release) version 0.4.0 ([`32aefc0`](https://github.com/GitoxideLabs/gitoxide/commit/32aefc051c7ad9d1a160f77db070df7fa4843dbc))
  - (cargo-release) version 0.9.0 ([`a89fdb9`](https://github.com/GitoxideLabs/gitoxide/commit/a89fdb98f64bb0ca070fa79a1f58f1232bb14090))
  - (cargo-release) version 0.5.0 ([`ae9c52b`](https://github.com/GitoxideLabs/gitoxide/commit/ae9c52bdbe43488bb9d5b5448bf07367a1d0a24a))
  - (cargo-release) version 0.3.0 ([`d19ee35`](https://github.com/GitoxideLabs/gitoxide/commit/d19ee35cc6683c63e0eabd717e4758075faeaa71))
  - Uograde everything else ([`0cd79d0`](https://github.com/GitoxideLabs/gitoxide/commit/0cd79d00bce3f042b5cc849cf48739e29f95fcb0))
  - Upgrade prodash and tui ([`b5eadca`](https://github.com/GitoxideLabs/gitoxide/commit/b5eadca343bbaa1af86722b5f1bcd33f4e3939a6))
  - (cargo-release) version 0.8.0 ([`47c00c2`](https://github.com/GitoxideLabs/gitoxide/commit/47c00c2228cf25c79e1fa3eb4229c7ab24de91e5))
  - (cargo-release) version 0.7.0 ([`7fa7bae`](https://github.com/GitoxideLabs/gitoxide/commit/7fa7baeb3e7d008a25e4d714eff908e2516c828b))
  - Merge branch 'commit-graph' into main ([`9cb09b2`](https://github.com/GitoxideLabs/gitoxide/commit/9cb09b248796f0ff5c9d3f3e857de4731324cfd5))
  - Add lean-plumbing docs for path of commit-graph-verify ([`5c7b52d`](https://github.com/GitoxideLabs/gitoxide/commit/5c7b52d658d5b86bf4cf05c724202e824016c0e2))
  - [commitgraph] Implement basic commit-graph file verification. ([`2571113`](https://github.com/GitoxideLabs/gitoxide/commit/2571113fea516737acedac08d66632ead499b474))
  - [commitgraph] Stub out commit-graph-verify plumbing command. ([`aacf0f0`](https://github.com/GitoxideLabs/gitoxide/commit/aacf0f05a909e5b7d9ffd5623ef9833e0465be93))
  - Revert "remove git-config from workspace while it's so fresh…" ([`99214f4`](https://github.com/GitoxideLabs/gitoxide/commit/99214f4c1097fa8da8f14f1279caf00db78fa822))
  - Remove git-config from workspace while it's so fresh… ([`84e0d19`](https://github.com/GitoxideLabs/gitoxide/commit/84e0d19ab2285916cb6a6b941ec2206aef485d56))
  - Stub for git-config crate ([`3539531`](https://github.com/GitoxideLabs/gitoxide/commit/3539531adb06e8f59609f0a83e8ed94d0864c0a1))
  - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
  - Merge branch 'main' into commit-graph ([`ca5b801`](https://github.com/GitoxideLabs/gitoxide/commit/ca5b80174b73cc9ac162b3f33b5d3721ef936cb1))

</details>

## v0.4.3 (2020-09-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release over the course of 1 calendar day.
- 4 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

## v0.4.1 (2020-09-18)

* fix installation via `cargo install`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 7 commits contributed to the release over the course of 6 calendar days.
- 6 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - (cargo-release) version 0.4.1 ([`64fff36`](https://github.com/GitoxideLabs/gitoxide/commit/64fff36dcfdade887b7f417605d81b9d5750f000))
  - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
  - Finish removal of rust 2018 idioms ([`0d1699e`](https://github.com/GitoxideLabs/gitoxide/commit/0d1699e0e0bc9052be0a74b1b3f3d3eeeec39e3e))
  - (cargo-release) version 0.6.0 ([`9ef184e`](https://github.com/GitoxideLabs/gitoxide/commit/9ef184e35712f938fb4f9f6da7390a8777a9284e))
  - Upgrade futures-lite ([`1d83033`](https://github.com/GitoxideLabs/gitoxide/commit/1d830330101b797f840f9f4a61fe4f28058fdb4c))
  - Switch to prodash 10 and safe a lot of trait bounds in the process ([`e2fb1d9`](https://github.com/GitoxideLabs/gitoxide/commit/e2fb1d944b4d803a11c91f868b831d406fb5e35f))
  - Provide terminal dimensions to better use horizontal space ([`11f6b84`](https://github.com/GitoxideLabs/gitoxide/commit/11f6b8497a5089377e605f4cbe1cd317ef677d59))

</details>

## v0.4.0 (2020-09-12)

* add `remote-ref-list` and `pack-receive` subcommands to **gix**

### Other

- <csr-id-b1118f2d2e9192b9f07937d8ce0c31f39994ce90/> Use SPDX license syntax ("OR") in license field

### CLI Breaking

 * rename plumbing sub-command from `index-from-pack` to `pack-index-from-data`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 31 commits contributed to the release over the course of 30 calendar days.
- 31 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - (cargo-release) version 0.4.0 ([`f667785`](https://github.com/GitoxideLabs/gitoxide/commit/f667785e9e9db16e1afb498c06ff89246e4f6aef))
  - (cargo-release) version 0.4.0 ([`92e8b27`](https://github.com/GitoxideLabs/gitoxide/commit/92e8b273654c3dedce60de244944683c7cf153e7))
  - (cargo-release) version 0.2.0 ([`779e9d0`](https://github.com/GitoxideLabs/gitoxide/commit/779e9d0ad67c20fa9cec14359e87774ca2d74ee4))
  - (cargo-release) version 0.5.0 ([`82b7313`](https://github.com/GitoxideLabs/gitoxide/commit/82b73131b79ec3c42a712dad1c0766a72209d737))
  - [clone] refs can now be written into a specified directory ([`fb1f048`](https://github.com/GitoxideLabs/gitoxide/commit/fb1f04837be994fa5bcb9aa24f25b5f4f72e4ce0))
  - [clone] First version of writing references, but… ([`445be27`](https://github.com/GitoxideLabs/gitoxide/commit/445be27cf81663ba4fe941c00262448444efbac2))
  - [clone] first journey test for pack-receive ([`46a3511`](https://github.com/GitoxideLabs/gitoxide/commit/46a3511aead043bc45256ce603285ff4d0fff60a))
  - [clone] This actually works: first MVP of retrieving packs via clone ([`c06d819`](https://github.com/GitoxideLabs/gitoxide/commit/c06d8194173f9ec468ddd0faf72dd6d8dbf7d35d))
  - [ref-ls] add pretty version for ls-refs ([`487d06d`](https://github.com/GitoxideLabs/gitoxide/commit/487d06d53b9cc201b5a009977e835b51f4b9f690))
  - [ref-ls] Fix progress display ([`2fcb557`](https://github.com/GitoxideLabs/gitoxide/commit/2fcb557dce941eb94ca60f46ecee86b94e029db7))
  - [ref-ls] And it even doesn't work if it is the very same transport ([`4ba50fe`](https://github.com/GitoxideLabs/gitoxide/commit/4ba50fe06f7423c31f4cd78079d51ef3ffd51920))
  - Refactor ([`b38290e`](https://github.com/GitoxideLabs/gitoxide/commit/b38290e4a8fcabd758f26a15407710ab2abcdc07))
  - [ref-ls] refactor ([`35e26fc`](https://github.com/GitoxideLabs/gitoxide/commit/35e26fc32978232aebda3468c9f172fb7b08b815))
  - Refactor ([`f90b92f`](https://github.com/GitoxideLabs/gitoxide/commit/f90b92ffc2994f594352abaf4bacd9767cbc2e6c))
  - [ref-ls] Frame for remote-ref-ls command in gitoxide-core ([`161e7df`](https://github.com/GitoxideLabs/gitoxide/commit/161e7df34a53db40551879c6d2319ee775dfd551))
  - Add placeholder for git-index crate ([`52ff13c`](https://github.com/GitoxideLabs/gitoxide/commit/52ff13cf260b53423faf59e5c666ff1565bde947))
  - [clone] sketch for http infrastructure to get going with curl ([`8351299`](https://github.com/GitoxideLabs/gitoxide/commit/835129968b5414c980f24dc73aa89b43ac39bcaa))
  - Reorganize git-commitgraph goals; add crate ([`21c9b75`](https://github.com/GitoxideLabs/gitoxide/commit/21c9b7500cb144b3169a6537961ec2b9e865be81))
  - Bump git-features to 0.4 to allow publishes after breaking changes ([`9d6b879`](https://github.com/GitoxideLabs/gitoxide/commit/9d6b8790e2edd7fa01b3239adff86a7cd2393f10))
  - [clone] move packet-line code into own crate ([`879af67`](https://github.com/GitoxideLabs/gitoxide/commit/879af671fcde405d3d08ddbc07ea70d0bee23ef1))
  - [clone] http protocol is now optional ([`06c0816`](https://github.com/GitoxideLabs/gitoxide/commit/06c0816bd0ed0ae0f125c1fa93ffe1b89e7e7eb1))
  - [clone] link up lean plumbing command with gitoxide-core: pack-receive ([`5ea49c8`](https://github.com/GitoxideLabs/gitoxide/commit/5ea49c8aa0d449bed98ce0147ad222ff25c27c32))
  - Refactor ([`40a6412`](https://github.com/GitoxideLabs/gitoxide/commit/40a64125dc5556630576ec2164b68838c76ccd79))
  - [protocol] properly implement remote progress reporting ([`a81954a`](https://github.com/GitoxideLabs/gitoxide/commit/a81954a6a37afacd51add6661a656b8fb663ca54))
  - Less ambiguous name for 'index-from-pack': 'pack-index-from-data' ([`386673c`](https://github.com/GitoxideLabs/gitoxide/commit/386673ccc99d18d023c7df3fcd40e86d71960b25))
  - Use SPDX license syntax ("OR") in license field ([`b1118f2`](https://github.com/GitoxideLabs/gitoxide/commit/b1118f2d2e9192b9f07937d8ce0c31f39994ce90))
  - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
  - Refactor ([`b4a6e16`](https://github.com/GitoxideLabs/gitoxide/commit/b4a6e16364822c0dccb56f98dbfb0ca4c8007069))
  - Add git-url crate ([`fd2e5ba`](https://github.com/GitoxideLabs/gitoxide/commit/fd2e5bab97f09666c983634fa89947a4bed1c92d))
  - First bunch of tasks I see after studying parts of the protocol docs ([`9bd97ba`](https://github.com/GitoxideLabs/gitoxide/commit/9bd97bafd299efefd063dde73cef53fde9d36670))
  - Actually removes 'demos' from workspace ([`2eac661`](https://github.com/GitoxideLabs/gitoxide/commit/2eac661382590da91031c2bc4e478c87766e85dc))

</details>

## v0.3.0 (2020-08-12)

* add `pack-explode` and `pack-index-from-data` sub-commands
* massive speed improvements for `pack-verify`

Many small and possibly breaking changes are not mentioned here.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 62 commits contributed to the release.
- 31 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Bump minor version to 0.3 ([`4351e28`](https://github.com/GitoxideLabs/gitoxide/commit/4351e2871c9dcf342b8471fffa74cae338a53269))
  - Make obvious that interrupt request was received ([`34b2373`](https://github.com/GitoxideLabs/gitoxide/commit/34b23737f560fe52d4f98fb886eba754652f9a5e))
  - Make interrupt handler work reliably ([`e71da0f`](https://github.com/GitoxideLabs/gitoxide/commit/e71da0fce6d6eab68f7b81b13cdc78ce8e9b7ee3))
  - Revert "A test to see how much time can be saved by not zeroing zlib buffers" ([`3d51d59`](https://github.com/GitoxideLabs/gitoxide/commit/3d51d595469d8451867331089e75b808f9361912))
  - A test to see how much time can be saved by not zeroing zlib buffers ([`fd41a51`](https://github.com/GitoxideLabs/gitoxide/commit/fd41a51de2261f425262ee7dee7a24fd87d87432))
  - Remove patch ([`41cd0fd`](https://github.com/GitoxideLabs/gitoxide/commit/41cd0fd56ea7aace5f02ee93b4684b9395e2c6ee))
  - Unify used ranges for line renderer amond pretty and lean interface ([`f59f66e`](https://github.com/GitoxideLabs/gitoxide/commit/f59f66e189732f567414f68c7463364e510f41c4))
  - Add percentage and throughput to tasks that matter ([`763d7ca`](https://github.com/GitoxideLabs/gitoxide/commit/763d7caa4c70111b7cb3ef5733d2c3c697758c28))
  - Fix 'release' make target ([`0a2ea35`](https://github.com/GitoxideLabs/gitoxide/commit/0a2ea354af0ae59cd0f40746bafba3c3ba68c567))
  - Upgrade to latest iteration of prodash ([`3a4faec`](https://github.com/GitoxideLabs/gitoxide/commit/3a4faecab56e37670c553e6563f11a46d740c333))
  - First part of migration to prodash 8.0, but… ([`6901a09`](https://github.com/GitoxideLabs/gitoxide/commit/6901a098641820c8d974ce56a24d6cdca779730d))
  - Support for JSON format output ([`1931575`](https://github.com/GitoxideLabs/gitoxide/commit/19315750f4f409e3f105c3c4054c4afbef91daad))
  - First pieces of the index-from-pack journey tests ([`181d69c`](https://github.com/GitoxideLabs/gitoxide/commit/181d69c1da46a931c513cbd7d8bca7b2fa53351c))
  - Add versions back to main command, remove from sub-commands ([`e509373`](https://github.com/GitoxideLabs/gitoxide/commit/e509373b26c9a7b120057fc6e75970568f328fc4))
  - Ditch structopt in favor of clap 3.0 beta1 ([`d7591e2`](https://github.com/GitoxideLabs/gitoxide/commit/d7591e24a5178732713286a5e28cbc90f5fe9ed9))
  - Move common flags to common plac ([`c0352c2`](https://github.com/GitoxideLabs/gitoxide/commit/c0352c2643e2badde79778d4a22d2e392a44f0a3))
  - Fix feature flags to actually work as advertised ([`d102701`](https://github.com/GitoxideLabs/gitoxide/commit/d1027012bfaa77ab81ba6d56e2ffe27afbaefd51))
  - Write about user interfaces and the use/non-use of async ([`91ba045`](https://github.com/GitoxideLabs/gitoxide/commit/91ba0457745f860b7a68cb38b13e69754747e8d9))
  - Interrupt support for pretty plumbing ([`bca7ce2`](https://github.com/GitoxideLabs/gitoxide/commit/bca7ce2e668a4be2600d2d04d00f46b21c82eee2))
  - Support for interruptible operations ([`a025593`](https://github.com/GitoxideLabs/gitoxide/commit/a02559378f9165df97a217f24834a851be719b08))
  - Revert "Less memory for look up mode, faster start" - too slow ([`584350a`](https://github.com/GitoxideLabs/gitoxide/commit/584350af91f533db4cf980327d530445384c6b5a))
  - Less memory for look up mode, faster start ([`395c7e7`](https://github.com/GitoxideLabs/gitoxide/commit/395c7e78ef344ee56cf3d4ef49828942a09094bc))
  - Remove memory mode entirely (and some complexity with it) ([`8812e91`](https://github.com/GitoxideLabs/gitoxide/commit/8812e916a21983868a37c4aade10f79a1dc9b926))
  - Turns out you never want to keep deltas in memory ([`657aa2c`](https://github.com/GitoxideLabs/gitoxide/commit/657aa2c38673cf10174f42bcb97039ac37b2926e))
  - Remove support for keeping compressed memory to reduce the index size ([`1e2ec7e`](https://github.com/GitoxideLabs/gitoxide/commit/1e2ec7e9d0ef2f2a4908860672080e411e945bff))
  - Document miniz-oxide performance opportunity instead of trying to fix it ([`31934f6`](https://github.com/GitoxideLabs/gitoxide/commit/31934f6e26359e2b160cc7505280ea1e1e0219a8))
  - Dependency upgrade + update ([`c6692c6`](https://github.com/GitoxideLabs/gitoxide/commit/c6692c6d494fe2bf1f9b924cf27da5908b74d62b))
  - …but there seem to be issues with the kernel pack… ([`cc147bc`](https://github.com/GitoxideLabs/gitoxide/commit/cc147bc60066c4ef31353a499958edadc960a9c4))
  - Minor fixes after first local tests - it's up to twice as fast!! ([`43c7fd1`](https://github.com/GitoxideLabs/gitoxide/commit/43c7fd1f81b9b4c938f99c0bf1deabdf121226b9))
  - Quick and dirty impl of lean command-line for index-from-pack ([`9660bbf`](https://github.com/GitoxideLabs/gitoxide/commit/9660bbffd8ace621178b067e22d227ef8c50ba84))
  - Upgrade dependencies ([`44b8221`](https://github.com/GitoxideLabs/gitoxide/commit/44b8221800454f9b651778a422186bd5061877f4))
  - Remove invalid clap configuration ([`665696f`](https://github.com/GitoxideLabs/gitoxide/commit/665696f636e152ad9969ea0ca004cb83f1641ae6))
  - Prepare full 'verify' implementation ([`ee45c7f`](https://github.com/GitoxideLabs/gitoxide/commit/ee45c7f47b95fc406cc5922a322c8fd6c0f52775))
  - Refactor ([`0a33b24`](https://github.com/GitoxideLabs/gitoxide/commit/0a33b24f5b61ccdf1358f1e9adcf0f6fd4099c1c))
  - Allow sink-compress configuration; choose best algorithm ([`29b9c23`](https://github.com/GitoxideLabs/gitoxide/commit/29b9c230e35ba9b4334797b63ab9fa88c2fe59d0))
  - Nice error message on failure ([`adbc82c`](https://github.com/GitoxideLabs/gitoxide/commit/adbc82c31450681fcb38233eeb8095efc5e52a18))
  - The first 'explode' implementation… ([`0d31ad1`](https://github.com/GitoxideLabs/gitoxide/commit/0d31ad1b61997fa0d0692c5919fb8032ffaaa35b))
  - Get all pieces ready for action ([`1805d64`](https://github.com/GitoxideLabs/gitoxide/commit/1805d64b9222d6a05a8718f04b29b789c1f42fea))
  - Pass option for safety checks down to explode(…) ([`0bcb790`](https://github.com/GitoxideLabs/gitoxide/commit/0bcb790dc8c35097916876afbb68bbfcc894c369))
  - Refactor ([`f66b116`](https://github.com/GitoxideLabs/gitoxide/commit/f66b116ddfbee62c3e20a4c5e7cd878fbf064195))
  - Basic tests and CLI args for explode pack ([`f932256`](https://github.com/GitoxideLabs/gitoxide/commit/f932256a62d6fc5d5558446de079fb666ddc27da))
  - Rename verify-pack to pack-verify (keeping it more formal) ([`ec8c48a`](https://github.com/GitoxideLabs/gitoxide/commit/ec8c48a8fcbcd748c9c764734d881b5f0217e1e4))
  - Refactor ([`d3c00c8`](https://github.com/GitoxideLabs/gitoxide/commit/d3c00c841ee1aeda6bb0534fe365db13c31f8d3c))
  - Commit to also giving a git-tui a go ([`a0b73af`](https://github.com/GitoxideLabs/gitoxide/commit/a0b73afdd1df9b1096f0c6fe388f795a6dfe7f33))
  - Change bin names from 'gio' to 'gix' and 'gixp' ([`5e23137`](https://github.com/GitoxideLabs/gitoxide/commit/5e231371432ad02c67b095448564b2aa6af76799))
  - (cargo-release) version 0.2.0 ([`0bb8314`](https://github.com/GitoxideLabs/gitoxide/commit/0bb831480d8657e1bb29ee7009aeac673471403e))
  - Revert "Invert --statitics switch to become --no-statistics" ([`93a9b30`](https://github.com/GitoxideLabs/gitoxide/commit/93a9b30069d9abc5742546ade90913026ac5774b))
  - Invert --statitics switch to become --no-statistics ([`aeb8778`](https://github.com/GitoxideLabs/gitoxide/commit/aeb87789ecc5cf3fd0ac69d67c7d0785e4eb329c))
  - Use faster algorithm by default ([`bb45c3d`](https://github.com/GitoxideLabs/gitoxide/commit/bb45c3d8a2aabf87231981000240f0444abf6fc4))
  - Fix clippy ([`ec40e09`](https://github.com/GitoxideLabs/gitoxide/commit/ec40e093d72f93d86168f39ebaca5b122ca0bec3))
  - Change course and do pack streaming first ([`bcb275e`](https://github.com/GitoxideLabs/gitoxide/commit/bcb275e91cfd6f0a71b3cb59a2b706b60608a594))
  - Upgrade crosstermion ([`109ef03`](https://github.com/GitoxideLabs/gitoxide/commit/109ef032697f5ade6365c47ad2097184485cbe23))
  - Get rid of annoying warnings - there is no better and easier way ([`41f38c4`](https://github.com/GitoxideLabs/gitoxide/commit/41f38c442e086b1f3fb48eea25839ef6207f0cbc))
  - Fully implement --encode and --re-encode flags ([`a7cfac8`](https://github.com/GitoxideLabs/gitoxide/commit/a7cfac83ddd859d9c2c25e457c0d7043738792dc))
  - Prepare for re-encoding each pack object ([`afae684`](https://github.com/GitoxideLabs/gitoxide/commit/afae684c72e5dc4b718976056dd5d34ed61de72a))
  - Add new 'git-ref' crate; place ref name validation code there ([`1a0e84e`](https://github.com/GitoxideLabs/gitoxide/commit/1a0e84e627b17be1b1fb53b4dc98ab78e9cfb9a7))
  - Fix naming change, which was introduced accidentally ([`fbb9f98`](https://github.com/GitoxideLabs/gitoxide/commit/fbb9f98508ec722e192466e28ded47aef2fb78b3))
  - Refactor ([`2888f1b`](https://github.com/GitoxideLabs/gitoxide/commit/2888f1b10a2baf40155544e667ddd461f3ddc938))
  - Pass threadlimit down from CLIs ([`f98c5b1`](https://github.com/GitoxideLabs/gitoxide/commit/f98c5b160db80a7cac530e18b9256562c25be47f))
  - Add new Context argument to support more configuration options ([`7c5d8b8`](https://github.com/GitoxideLabs/gitoxide/commit/7c5d8b8bb318e59a59ad74ad767a1525e2833632))
  - Merge pull request #2 from joshtriplett/faster ([`8e99f62`](https://github.com/GitoxideLabs/gitoxide/commit/8e99f621ad4f4f7ed79c2b49c777d2811e4557c1))
  - Don't spend build time optimizing build-time-only crates ([`add2e3e`](https://github.com/GitoxideLabs/gitoxide/commit/add2e3e8d155571154c8816cf57f473a6e4d8d31))

</details>

## v0.1.0 (2020-07-12)

* Initial release with `pack-verify`

### Commit Statistics

<csr-read-only-do-not-edit/>

- 83 commits contributed to the release.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Cargo-diet for the top-level crate ([`19e7fec`](https://github.com/GitoxideLabs/gitoxide/commit/19e7fec7deb5a6419f36a2732c90006377414181))
  - Make crates publishable ([`5688a34`](https://github.com/GitoxideLabs/gitoxide/commit/5688a3427ff3673e1422d43106f4d685fa837aed))
  - Bring color back to 'max' versions ([`c68d9ab`](https://github.com/GitoxideLabs/gitoxide/commit/c68d9ab6e9cccab3610a77a2c6839a26fb42de2d))
  - Fix tests ([`59ed51d`](https://github.com/GitoxideLabs/gitoxide/commit/59ed51d0c84bf067ef0a921730260f2c444e5409))
  - Support for disabling the cursor in the pretty line renderer ([`48c4bbd`](https://github.com/GitoxideLabs/gitoxide/commit/48c4bbd35f678de801fa2243f7a8705e825bdbf9))
  - Allow TUI to terminate action properly ([`1f1b725`](https://github.com/GitoxideLabs/gitoxide/commit/1f1b7257895a219b5623b1ba3beaee1282ff2b63))
  - Refactor ([`cce71aa`](https://github.com/GitoxideLabs/gitoxide/commit/cce71aadb53528cb2f1d173b81df939ad1df8083))
  - Refactor ([`f276a05`](https://github.com/GitoxideLabs/gitoxide/commit/f276a0561fcec78f802b166f09110becf22ea7ee))
  - Preliminary support for line renderer in max version ([`4aa8022`](https://github.com/GitoxideLabs/gitoxide/commit/4aa8022c248fc26292ea9f1c61e2e8dbadc56e7c))
  - Add line-renderer dependencies to max configuration ([`9bb8a4d`](https://github.com/GitoxideLabs/gitoxide/commit/9bb8a4d6c313c85bd52a0ce68c273c786015c67e))
  - Try release build split into windows and unix to support crossterm/termion separately ([`9098eb8`](https://github.com/GitoxideLabs/gitoxide/commit/9098eb8bda6a796c18a375cd8207f1a73e4e9543))
  - Unify frame rate across plumbing (and later potentially porcelain) ([`e2a7bdd`](https://github.com/GitoxideLabs/gitoxide/commit/e2a7bdd9a549400f0e8d31b590f469660e458c89))
  - Refactor ([`41e01a5`](https://github.com/GitoxideLabs/gitoxide/commit/41e01a53a7f48dd3341ee287a243a33190bccea0))
  - Proper implementation of line renderer into 'lean' CLI ([`e98e7c2`](https://github.com/GitoxideLabs/gitoxide/commit/e98e7c280d73e9d9ebd13202afb93a56cb2f7c9c))
  - Mild improvements to look of verbose log ([`5fff552`](https://github.com/GitoxideLabs/gitoxide/commit/5fff5524c4443d9c9ae26307c19745c722334d0c))
  - First very basic version of line renderer progress - works… ([`0cc1bf2`](https://github.com/GitoxideLabs/gitoxide/commit/0cc1bf25c69611f9512fec415ae8e09b608706fc))
  - Prepare for optional addition of line renderer for lean version ([`aac0d34`](https://github.com/GitoxideLabs/gitoxide/commit/aac0d341eb02f0dccdf740f7ef15e8f585907544))
  - Upgrade to prodash version 7 ([`af02b46`](https://github.com/GitoxideLabs/gitoxide/commit/af02b46cc1eff5ba1da7da20d3f524a79fad686f))
  - Update prodash to verion 6.0 ([`a4731a3`](https://github.com/GitoxideLabs/gitoxide/commit/a4731a3aca159f8916b29d9ce5a71856089c5a6b))
  - Make --version flags work as expected. ([`a4d978c`](https://github.com/GitoxideLabs/gitoxide/commit/a4d978ccc11e73fd752055c9a28b3b23dea145ea))
  - Switch to prodash 5.0 for windows support ([`88542e1`](https://github.com/GitoxideLabs/gitoxide/commit/88542e117dd1c2e7606fcbe88b30c51b4c115989))
  - Merge branch 'release' ([`a1a0b13`](https://github.com/GitoxideLabs/gitoxide/commit/a1a0b135c991edfe5cddb71c5fbfbed25b47e3b3))
  - Rename 'pretty' target into 'max', a better fit for what it is ([`5acecc5`](https://github.com/GitoxideLabs/gitoxide/commit/5acecc59d2d39141f2e98b6f8556c6d457ab0965))
  - Make features easier to use ([`e287c08`](https://github.com/GitoxideLabs/gitoxide/commit/e287c0877ddc55160d29ee72b51402d124bae6b1))
  - Make gio commands less cumbersome, self-document their build type (pretty, lean) ([`1f9bc03`](https://github.com/GitoxideLabs/gitoxide/commit/1f9bc03dd773d90960a6f6d4ee59af3f938ad80b))
  - Allow to limit the logging depth for less cluttered output ([`fce7035`](https://github.com/GitoxideLabs/gitoxide/commit/fce703531d7006f7d961d6ffa66f51f6c9bc0efc))
  - Support for json in pretty-plumbing and gitoxide (on demand) ([`b3780f8`](https://github.com/GitoxideLabs/gitoxide/commit/b3780f87438d34b372c48b7385199f7ea22b3965))
  - Simplify the 'keep open' logic of TUI progress window ([`13cd8ce`](https://github.com/GitoxideLabs/gitoxide/commit/13cd8ce372800eb0016190960834c759c9744b9c))
  - Attempt to implement progress with a mode enum ([`ac490c2`](https://github.com/GitoxideLabs/gitoxide/commit/ac490c21b8f369c45ee0d7688ddb381ce6f4af94))
  - Allow for more screen space when formatting ([`6794300`](https://github.com/GitoxideLabs/gitoxide/commit/67943002e7f4215b5383bd0538786ce2857f011e))
  - Assure pretty progress doesn't occlude the output ([`122d69f`](https://github.com/GitoxideLabs/gitoxide/commit/122d69fee217eb264a335f0a056d03eba066332e))
  - Pretty-printing of some statistics ([`125b565`](https://github.com/GitoxideLabs/gitoxide/commit/125b565f0fb4085c615fdf136f35a2285d69966a))
  - Fix pretty build ([`6adf615`](https://github.com/GitoxideLabs/gitoxide/commit/6adf615ed7d6c488c25589940fc0a55bf0fb3d5c))
  - Pass average stats through to the top level ([`5b4979c`](https://github.com/GitoxideLabs/gitoxide/commit/5b4979c1dfeb9a29974dd4e6529ae5da074d0b1a))
  - Refactor ([`7add82c`](https://github.com/GitoxideLabs/gitoxide/commit/7add82c39169e3c2fff76c48cdd318fe6040d7bc))
  - Now ACTUALLY stop TUI when there is no progress anymore :D ([`3bf3321`](https://github.com/GitoxideLabs/gitoxide/commit/3bf33210a96e1e3bc2a81782b339b5c67344ac34))
  - Automatically close the TUI when there is no progress anymore. ([`c416152`](https://github.com/GitoxideLabs/gitoxide/commit/c416152b04051958de7bd161a8a2ee42ca163275))
  - Assure we wait for GUI thread to finish ([`60eaea0`](https://github.com/GitoxideLabs/gitoxide/commit/60eaea0ee01214202ab9f23514dc45a9909d7888))
  - Pretty progress in a generalized form ([`caa883b`](https://github.com/GitoxideLabs/gitoxide/commit/caa883b96827deb63b5c8787ed820d22f2c85249))
  - Neater progress log messages: don't show the module it originates from ([`026a0dd`](https://github.com/GitoxideLabs/gitoxide/commit/026a0dd1faf28c4668f58bd1790c168a0134559f))
  - Refactor ([`30925e6`](https://github.com/GitoxideLabs/gitoxide/commit/30925e654144a05365908f7d2aa90deb7b2952d3))
  - Support for logging in pretty binaries ([`67026e4`](https://github.com/GitoxideLabs/gitoxide/commit/67026e479f0aa3e47ff3fd230c8741a7a5dbe99c))
  - --verbose flag for lean plumbing binary ([`aaf4825`](https://github.com/GitoxideLabs/gitoxide/commit/aaf482584d1ee080c0a6c091c4675736c4c8d6a7))
  - First very basic progress implementation ([`b820717`](https://github.com/GitoxideLabs/gitoxide/commit/b8207177daee8a9ffa23c7c052cf9ca651b15804))
  - Pass progress everywhere, for now just to discard it ([`da3ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/da3ae1c82cd726b8fae9b8d26069719930e9ba99))
  - Implement `Progress` trait for prodash::tree::Item ([`0eeb6d7`](https://github.com/GitoxideLabs/gitoxide/commit/0eeb6d770d58621427bc88107a20860b89b86a24))
  - Implement progress trait for logs with throttling ([`287eca9`](https://github.com/GitoxideLabs/gitoxide/commit/287eca91b244ccbc703cb275b1ae032bfeb02532))
  - Split plumbing into separate binary ([`b1e51d6`](https://github.com/GitoxideLabs/gitoxide/commit/b1e51d6a83ca7a00923b39209d0a2bfb3b78de0d))
  - Refactor ([`0fbba9f`](https://github.com/GitoxideLabs/gitoxide/commit/0fbba9fe7597af03912f956c251c88472b48c3eb))
  - Refactor ([`ba6a8ef`](https://github.com/GitoxideLabs/gitoxide/commit/ba6a8ef064a9884066414c82f4f7d1bb72ab524f))
  - Add initial version of 'lean-cli' feature toggle, but… ([`f01c298`](https://github.com/GitoxideLabs/gitoxide/commit/f01c2985732ac05b24a7fcbc3752ef52dd1bc438))
  - Prepare for allowing two CLI crates to be used ([`f558ce5`](https://github.com/GitoxideLabs/gitoxide/commit/f558ce5e1bf0e5acb6be8d12857ea350f6b733bc))
  - Add feature toggles to gitoxide (CLI) itself ([`4e50e5e`](https://github.com/GitoxideLabs/gitoxide/commit/4e50e5e8259faff52d9b9773e7bc8b22811b23dd))
  - Add 'fast-sha1' to git-features ([`b22541f`](https://github.com/GitoxideLabs/gitoxide/commit/b22541f0c39af470877119b136e4eb1b82dff2db))
  - Use git-features to toggle 'parallel' mode from the 'gitoxide' level ([`d944fbf`](https://github.com/GitoxideLabs/gitoxide/commit/d944fbf181acc5fb83a841613174702af1e074d6))
  - A new crate to represent features that can toggle from the top-level ([`23c420c`](https://github.com/GitoxideLabs/gitoxide/commit/23c420cc95219dc7c04d3905aaa03281cb51724e))
  - Support for verifying pack files and index files ([`b09b4e1`](https://github.com/GitoxideLabs/gitoxide/commit/b09b4e1f35c3802dfd3418bda42b96828acd9ec8))
  - Reorganize crates to make 'gitoxide' the CLI, and 'gitoxide-core' the library ([`0ac9c5a`](https://github.com/GitoxideLabs/gitoxide/commit/0ac9c5af0cbb562d3cb48a661736afd98dd1a940))
  - Use fast-sha1 in `gio` for a little more performance, yet… ([`d134ddf`](https://github.com/GitoxideLabs/gitoxide/commit/d134ddf533051bce2a6bf274aee33ed5a8284db5))
  - Add simple pack verification to gio ([`8c0e0b5`](https://github.com/GitoxideLabs/gitoxide/commit/8c0e0b5bb79c8c337eed03d37cbf818d8bb9c924))
  - Goodbye git-core, hello git-repository ([`7cec2b6`](https://github.com/GitoxideLabs/gitoxide/commit/7cec2b648f86fc665b4fc5bfe269e9ca16679a55))
  - Prepare git-transport just so that we don't forget to take the name ([`2c3ad7d`](https://github.com/GitoxideLabs/gitoxide/commit/2c3ad7d916ca513cc9dff26ff2150bae0dcb93e1))
  - Simplify folder names ([`36fde1f`](https://github.com/GitoxideLabs/gitoxide/commit/36fde1f90e9034060b5ede8a923365474659085e))
  - Add release configuration for maximum performance + native CPU features ([`07859cb`](https://github.com/GitoxideLabs/gitoxide/commit/07859cb6031ccc1938f66481ec1dd88cf05185b8))
  - Rename grit to 'gitoxide', CLI name is 'gio' ([`9d6007f`](https://github.com/GitoxideLabs/gitoxide/commit/9d6007f83b3b018d736d58aa0722b83b9cffb228))
  - Document existing use of unsafe, deny everywhere else ([`41f4bce`](https://github.com/GitoxideLabs/gitoxide/commit/41f4bce9d9a492f8e20a6eb5b3eaf5adc6d78329))
  - Cargo clippy ([`1179ac1`](https://github.com/GitoxideLabs/gitoxide/commit/1179ac16ea2bb84816f9b615d1191f8a2d4e775b))
  - Move all object related code into own crate… ([`605ef20`](https://github.com/GitoxideLabs/gitoxide/commit/605ef20ec5ccf66e4f42df6d0140e4087aa13053))
  - Move parsing tests close to actual parsing ([`3ca2c59`](https://github.com/GitoxideLabs/gitoxide/commit/3ca2c592d91c9aa8fab8ed749871d6d96f2ef4e2))
  - Color for all grit commands/subcommands ([`aa8efdd`](https://github.com/GitoxideLabs/gitoxide/commit/aa8efdd922d45bdab668dc71e8b30adf79930667))
  - Use structopt instead of clap ([`eb7388c`](https://github.com/GitoxideLabs/gitoxide/commit/eb7388c5d51e4ef3ea928d5f8f9e5b218cdbbd57))
  - Remove failure from grit binary, too ([`417c34b`](https://github.com/GitoxideLabs/gitoxide/commit/417c34b82469bcc3391706646dd39c7f6d1ad69c))
  - Move examples into demos, having their very own dependencies; optimize tests ([`b757712`](https://github.com/GitoxideLabs/gitoxide/commit/b757712f82de1d75ed813e744f979c1c652350e6))
  - Refactor ([`87c8a2e`](https://github.com/GitoxideLabs/gitoxide/commit/87c8a2e288140b04e163fe85266d040d039ec69c))
  - Cargo fmt ([`2aa0857`](https://github.com/GitoxideLabs/gitoxide/commit/2aa085752aa3e99b51034a3dec882aea8c27ad94))
  - Reorganize repository a bit; use different contact email address ([`cb9fa28`](https://github.com/GitoxideLabs/gitoxide/commit/cb9fa2848476e30767deb9d9807c649e0bc366da))
  - Make cli an edition 2018 crate ([`9ad3d1e`](https://github.com/GitoxideLabs/gitoxide/commit/9ad3d1edeb4532aad54e4f7d41da39e6d477f15a))
  - Put all crates in a workspac ([`c088873`](https://github.com/GitoxideLabs/gitoxide/commit/c088873997a932664fa1d63ec947d438eb1b3cf1))
  - Add Cargo.lock ([`ca54e93`](https://github.com/GitoxideLabs/gitoxide/commit/ca54e931564eb6f17349f4aba910c068b1d0b351))
  - Add MIT license ([`2bc6477`](https://github.com/GitoxideLabs/gitoxide/commit/2bc6477df5707d3fe4968bf1ddd4ca92a8b91b9a))
  - First unit-tests for git-odb ([`557ac49`](https://github.com/GitoxideLabs/gitoxide/commit/557ac49882a1955cf0f8756ea820d8f55e2e2d52))
  - Implement git-init ([`57737c2`](https://github.com/GitoxideLabs/gitoxide/commit/57737c2c48ff898a327ba57712fea21b5d83188e))
  - Initial commit - based on standard project template ([`c3d319f`](https://github.com/GitoxideLabs/gitoxide/commit/c3d319f2b3076a0bb169bcd8a7b6a011f6aba9a5))

</details>
