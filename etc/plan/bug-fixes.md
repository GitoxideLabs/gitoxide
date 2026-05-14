# GitHub Bug Fix Backlog

Generated on 2026-04-30 after downloading all GitHub issues in
`GitoxideLabs/gitoxide`. The actionable backlog below is filtered to open
GitHub issues, and intentionally excludes feature requests, tracking issues,
integration requests, and documentation-only requests unless the issue
describes incorrect behavior, a regression, a failing test, or a
soundness/security concern.

Source set: 712 GitHub issues total, excluding pull requests; 164 are open.

Priority key:

- P0: security, data corruption, hangs, or likely soundness problems.
- P1: user-visible operation failures in clone/fetch/status/config/worktree
  flows.
- P2: correctness and Git-parity defects with narrower impact.
- P3: performance regressions or severe inefficiencies.
- P4: test, CI, and maintenance defects that block confidence but are less
  directly user-facing.
- Parked: bug-shaped issues that are currently marked `wontfix`, ambiguous, or
  mostly require a product decision before implementation.

## P0 - Fix First

1. [#1534](https://github.com/GitoxideLabs/gitoxide/issues/1534) -
   `gitoxide-core` does not neutralize terminal control characters.
   Security-sensitive terminal output can render untrusted object data without
   sufficient escaping. Start by auditing every command path that prints names,
   object data, refs, config values, or remote-provided strings, then add a
   shared terminal-safe rendering helper and regression tests.

2. [#2421](https://github.com/GitoxideLabs/gitoxide/issues/2421) -
   `index.write()` can corrupt the effective index when the tree extension is
   stale.
   Updating index entries without updating or removing the tree cache can make
   `git status` and later commits disagree about what is staged. First safe
   fix: remove the tree extension before writing unless it can be updated
   correctly, then add a mixed Git/gix regression that demonstrates stale tree
   cache invisibility.

3. [#1231](https://github.com/GitoxideLabs/gitoxide/issues/1231) -
   `ItemSliceSync::get_mut()` may be unsafely callable for the same index.
   The report suggests a bad packfile might violate aliasing assumptions.
   Treat this as an audit before assuming it is benign: prove callers partition
   indexes, or replace the abstraction with checked interior mutability.

4. [#1784](https://github.com/GitoxideLabs/gitoxide/issues/1784) -
   Nonexclusive checkout never removes executable permissions.
   Checkout can leave executable bits behind when the target index says the
   file is non-executable. Reproduce with two checkouts over the same worktree,
   then make mode reconciliation explicit for reused files.

5. [#1783](https://github.com/GitoxideLabs/gitoxide/issues/1783) -
   Delayed process filters suppress executable bits in `gix clone`.
   Files that should be checked out as executable are left non-executable when
   a long-running smudge filter supports delayed output. Add a fixture with a
   delayed process filter and assert the final mode after delayed checkout.

6. [#135](https://github.com/GitoxideLabs/gitoxide/issues/135) -
   Fetching from misbehaving servers can block forever.
   Network operations need bounded progress or interruptibility. Start with a
   small fake server fixture that stops responding, then add timeout/cancel
   handling in the affected transport path.

7. [#24](https://github.com/GitoxideLabs/gitoxide/issues/24) -
   Robustness when killed in the middle.
   Interrupted operations should not leave repositories in misleading or
   corrupt states. Identify write paths that lack temp-file-plus-rename or
   rollback behavior and add crash/interruption-oriented tests around them.

## P1 - User-Visible Breakage

1. [#2554](https://github.com/GitoxideLabs/gitoxide/issues/2554) -
   Shallow clone with a tag refspec fails.
   `PrepareFetch::with_shallow()` plus `.with_ref_name()` treats the ref as a
   local branch, producing `refs/heads/<tag>` and failing required refspec
   validation. Fix ref-name classification for tags and add a shallow tag clone
   regression.

2. [#1025](https://github.com/GitoxideLabs/gitoxide/issues/1025) -
   Azure DevOps clone failure with `REF_DELTA` objects.
   Pack resolution needs Git-style delta handling for Azure-generated packs.
   Reproduce against the reported Azure fixture or archived pack, then align
   delta resolution with Git behavior.

3. [#1096](https://github.com/GitoxideLabs/gitoxide/issues/1096) -
   `gix` fails to decode a tree object that Git accepts.
   This is a compatibility failure in object decoding. Preserve the reported
   tree object as a fixture and decide whether the decoder needs to accept a
   Git-compatible edge case or improve diagnostics.

4. [#2140](https://github.com/GitoxideLabs/gitoxide/issues/2140) -
   Clone fails with `blocking-http-transport-reqwest`.
   The same clone works with curl and reqwest-rustls, so this is likely a
   transport configuration or error-surface bug. Add a focused clone example
   test for the failing feature set and expose the underlying source error.

5. [#2313](https://github.com/GitoxideLabs/gitoxide/issues/2313) -
   Local clones fail when `git-upload-pack` is available through `git` but not
   directly in `PATH`.
   Mainly affects Windows installations such as Scoop. For local clones, fall
   back to `git upload-pack` or derive the helper path from the discovered Git
   executable.

6. [#1055](https://github.com/GitoxideLabs/gitoxide/issues/1055) -
   Fetching repositories with special branch names breaks on Windows.
   Branch names that are acceptable to Git can collide with Windows path or ref
   handling. Capture the reported names in tests and fix ref storage/path
   translation.

7. [#2052](https://github.com/GitoxideLabs/gitoxide/issues/2052) -
   Worktree path resolves incorrectly for a bare repository with submodules.
   `gix status` resolves a submodule under `/home/...` without the username and
   fails. Reproduce with the dotfiles-style bare setup and trace worktree root
   derivation for submodules.

8. [#2210](https://github.com/GitoxideLabs/gitoxide/issues/2210) -
   Valid negative fetch refspecs are rejected as `NegativeGlobPattern`.
   Git accepts negative glob refspecs such as `^refs/heads/*-deploy`. Narrow
   the invalid-pattern check so valid negative fetch exclusions parse and round
   trip correctly.

9. [#1912](https://github.com/GitoxideLabs/gitoxide/issues/1912) -
   `safe.directory` and trust handling hide remotes unexpectedly.
   A repository listed in `safe.directory` can still yield no remotes. Add a
   trust test with owned, unowned, and safe-directory cases, then make remote
   lookup either honor the trust exception or fail explicitly.

10. [#2067](https://github.com/GitoxideLabs/gitoxide/issues/2067) -
    Trust checks do not work for UNC paths on Windows.
    `GetNamedSecurityInfoW` or path conversion appears to fail for WSL/network
    paths and long UNC paths. Add Windows-specific coverage for UNC inputs and
    support Git's `safe.directory` escape hatch behavior.

11. [#1951](https://github.com/GitoxideLabs/gitoxide/issues/1951) -
    New remotes can be saved into global-config metadata instead of repo config.
    Creating a local remote named like a global remote can mutate/select the
    wrong config section metadata. Make remote creation target the repository
    config file explicitly.

12. [#1819](https://github.com/GitoxideLabs/gitoxide/issues/1819) -
    `ein tool organize` cannot organize reftable repositories.
    Reftable repositories are not recognized by the organizer. Add a minimal
    reftable fixture and route detection through repository format support
    rather than loose-ref assumptions.

13. [#1621](https://github.com/GitoxideLabs/gitoxide/issues/1621) -
    Unable to extract a pack entry from `irs-manual-demo.git`.
    Preserve the problematic pack or a minimized reproduction and fix the pack
    access path that fails on the entry.

14. [#1391](https://github.com/GitoxideLabs/gitoxide/issues/1391) -
    libcurl reports unknown option 48 on CentOS 7.
    Older libcurl builds reject an option currently passed by the transport
    layer. Gate the option by libcurl version/capability or make it optional.

15. [#1353](https://github.com/GitoxideLabs/gitoxide/issues/1353) -
    `gix clone` ignores global `core.symlinks` on Windows.
    Windows checkout should respect global symlink capability/configuration.
    Add a Windows regression with `core.symlinks=false` and compare Git's
    checkout result.

16. [#1119](https://github.com/GitoxideLabs/gitoxide/issues/1119) -
    `gix-url` does not compile with the `url` version it specifies.
    Minimal-version builds fail because the crate uses APIs newer than its
    declared lower bound. Raise the dependency floor or avoid the newer API.

## P2 - Correctness and Git Parity

1. [#2490](https://github.com/GitoxideLabs/gitoxide/issues/2490) -
   `gix status --submodules all` falsely reports submodules dirty when they
   only contain empty untracked directories.
   Git ignores empty directories. Add the reported submodule fixture and make
   untracked-directory detection require files.

2. [#2161](https://github.com/GitoxideLabs/gitoxide/issues/2161) -
   Breadth-first simple commit traversal does not hide commits correctly.
   Add a graph where a hidden branch contains all traversed commits and assert
   breadth-first traversal matches the non-breadth-first hiding behavior.

3. [#2159](https://github.com/GitoxideLabs/gitoxide/issues/2159) -
   Hidden tips do not work with single-parent traversal.
   Single-parent traversal still needs to inspect all parents for hide
   propagation while only yielding first-parent results. Add explicit tests for
   that bitflag propagation.

4. [#1832](https://github.com/GitoxideLabs/gitoxide/issues/1832) -
   Rename tracking depends on order.
   Rename detection should be stable regardless of input ordering. Build a
   minimized fixture that shuffles candidate order and assert identical output.

5. [#1816](https://github.com/GitoxideLabs/gitoxide/issues/1816) -
   Pseudorandom numbers are sometimes identical across processes.
   Review seed generation for process-local uniqueness, especially in tests or
   temp fixture naming, and add a repeated-process smoke test.

6. [#1798](https://github.com/GitoxideLabs/gitoxide/issues/1798) -
   UTF-32 encoding is missing for `ansible.git` from Salsa.
   Add the reported repository or minimized object/config data as a fixture and
   extend encoding detection/decoding to cover the missing UTF-32 case.

7. [#2467](https://github.com/GitoxideLabs/gitoxide/issues/2467) -
   SCP-like URL to SSH URL conversion is lossy.
   The current conversion changes home-relative SCP-like paths into absolute
   SSH paths. Decide the canonical representation, then add round-trip tests
   for `host:path`, `host:/path`, `host:~/path`, and `host:~user/path`.

8. [#1868](https://github.com/GitoxideLabs/gitoxide/issues/1868) -
   `gix-command` on Windows can run shell commands outside POSIX mode.
   Git for Windows shims may invoke `bash.exe` instead of `sh.exe` semantics.
   Detect or invoke the POSIX-mode shell path deliberately.

9. [#1869](https://github.com/GitoxideLabs/gitoxide/issues/1869) -
   `gix-command` on Unix can choose non-POSIX `/bin/sh`.
   POSIX-compatible `sh` is not guaranteed to be `/bin/sh`. Consider resolving
   `sh` through the standard path or `getconf PATH` on affected platforms.

10. [#1842](https://github.com/GitoxideLabs/gitoxide/issues/1842) -
    `gix-command` passes `--` as `$0` to `sh -c`.
    This produces confusing shell diagnostics. Pass an informative shell name
    or command label as `$0` and add a test for error output.

11. [#1615](https://github.com/GitoxideLabs/gitoxide/issues/1615) -
    `gix-trace` tracing level set by subscriber is not always respected.
    Audit where trace level filtering is overridden or cached and add a
    subscriber-driven filtering test.

12. [#1649](https://github.com/GitoxideLabs/gitoxide/issues/1649) -
    `gix worktree list` mixes branch names and worktree names in similar rows.
    The output is easy to misread and differs from Git semantics. Clarify row
    labels or align with Git's worktree-list output.

13. [#1269](https://github.com/GitoxideLabs/gitoxide/issues/1269) -
    Replace an mtime workaround with the fixed `rustix` API.
    Upgrade to a `rustix` version containing the mtime fix and remove the local
    workaround once tests confirm equivalent behavior.

14. [#235](https://github.com/GitoxideLabs/gitoxide/issues/235) -
    `make tests` performs duplicate checks.
    Remove duplicated test invocations or split the target so repeated checks
    are intentional and visible.

## P3 - Performance Regressions and Scalability

1. [#2424](https://github.com/GitoxideLabs/gitoxide/issues/2424) -
   `phpstan/phpstan` pack resolves with a single thread only.
   Clone spends roughly 20 minutes resolving a large pack. Profile pack
   resolution on the reported repository and identify why parallelism is not
   engaged.

2. [#2024](https://github.com/GitoxideLabs/gitoxide/issues/2024) -
   Pack size regressed by about 50 percent from `gix-pack` 0.58 to 0.59.
   Use the provided wide-tree reproduction to bisect pack generation changes
   and add a size regression test with a stable threshold.

3. [#2296](https://github.com/GitoxideLabs/gitoxide/issues/2296) -
   `gix status` is slow on Windows.
   Direct `lstat` checks scale poorly. Investigate a Windows directory-listing
   stat cache shared across status traversal workers.

4. [#1771](https://github.com/GitoxideLabs/gitoxide/issues/1771) -
   `gix status` performance is inconsistent on clean trees.
   A Linux tree can be much slower with `gix status` than with the previous
   tool. Reproduce on the reported large repositories and profile unchanged and
   untracked paths separately.

5. [#2450](https://github.com/GitoxideLabs/gitoxide/issues/2450) -
   Dependabot cargo dependency runs time out.
   This blocks dependency maintenance. Compare Dependabot resolution behavior
   with `cargo update`, then reduce update scope, split ecosystems, or switch
   tooling if no Dependabot-specific mitigation is practical.

6. [#1788](https://github.com/GitoxideLabs/gitoxide/issues/1788) -
   The slotmap turned out to be too small.
   Confirm where capacity assumptions fail, then replace the fixed-size
   structure or make growth explicit and tested.

## P4 - Test, CI, and Reproducibility Defects

1. [#2548](https://github.com/GitoxideLabs/gitoxide/issues/2548) -
   `gix-status-tests` fixture creation fails with `PermissionDenied`.
   Regenerating fixtures can fail for `unreadable_untracked.tar`. Make fixture
   generation robust to unreadable files and ignored generated archives.

2. [#2259](https://github.com/GitoxideLabs/gitoxide/issues/2259) -
   `gix-worktree-tests` baseline ignore test fails because
   `user_exclude_path.is_file()` is false.
   Find why the expected user exclude file is absent in the reported setup and
   make the fixture setup explicit.

3. [#2006](https://github.com/GitoxideLabs/gitoxide/issues/2006) -
   Rare symlink overwrite protection test failure.
   The case sensitivity of `FAKE-FILE` changes in a macOS CI run. Add logging
   around the checkout path and try to reproduce under repeated nextest runs.

4. [#1894](https://github.com/GitoxideLabs/gitoxide/issues/1894) -
   `armv7-linux-androideabi` cross tests panic in `miniz_oxide`.
   Determine whether this is a dependency, target, or gitoxide pack handling
   issue. Keep the cross target as a focused reproducer.

5. [#1890](https://github.com/GitoxideLabs/gitoxide/issues/1890) -
   Four pack-related tests fail on s390x.
   Preserve failing outputs and compare endian-sensitive pack and multi-index
   code paths. Add s390x coverage if a hosted runner or emulator is reliable.

6. [#1722](https://github.com/GitoxideLabs/gitoxide/issues/1722) -
   Some tests terminate unusually when canceled on Windows.
   Ctrl-C during nextest can produce unusual termination. Investigate signal
   handling and child process cleanup in test helpers and spawned commands.

7. [#1622](https://github.com/GitoxideLabs/gitoxide/issues/1622) -
   `regex_matches` baseline fails after Git 2.47 `rev-parse` behavior changes.
   Update fixture generation or expected baseline behavior so tests compare
   against the intended Git version semantics.

8. [#1575](https://github.com/GitoxideLabs/gitoxide/issues/1575) -
   `jj_realistic_needs_to_be_more_clever` failure is not limited to CI.
   Reproduce locally and decide whether the test expectation, fixture, or jj
   integration behavior needs to change.

9. [#1358](https://github.com/GitoxideLabs/gitoxide/issues/1358) -
   Twelve Windows tests fail with `GIX_TEST_IGNORE_ARCHIVES=1`.
   Regenerated fixtures differ from committed archives on Windows. Work through
   the failures in small groups and commit platform-stable fixture generation.

10. [#2363](https://github.com/GitoxideLabs/gitoxide/issues/2363) -
    `CHANGELOG.md` files are no longer kept up to date.
    This is a release-process defect rather than a code bug. Add a release
    checklist or CI check that catches stale changelog state before publishing.

## Parked or Needs Product Decision

1. [#1930](https://github.com/GitoxideLabs/gitoxide/issues/1930) -
   Checkout of a specific branch, tag, or revision does not work in the
   expected cargo-generate migration flow.
   This is labeled `wontfix`, so do not schedule implementation without first
   deciding whether the current clone/checkout API intentionally omits this
   behavior or should grow a supported path.

2. [#2429](https://github.com/GitoxideLabs/gitoxide/issues/2429) -
   `LsRefsCommand`, `RefSpec`, and `RefSpecRef` construction is confusing.
   Primarily documentation/API usability, not a runtime bug. It may become a
   bug-fix task only if the API is proven to make valid protocol usage
   impossible or misleading.

3. [#1787](https://github.com/GitoxideLabs/gitoxide/issues/1787) -
   `overwrite_existing: false` is not documented to let executable bits be set.
   Mostly documentation around checkout semantics. Link this to #1783 and
   #1784 while fixing mode behavior.

## Suggested Work Order

1. Fix #2421 first because it can create misleading commits and has a plausible
   low-risk mitigation: remove the tree extension on writes until full tree
   cache updates are implemented.
2. Fix the recent clone regression #2554 next; the report includes root cause
   and a compact reproduction.
3. Handle the checkout mode cluster #1783 and #1784 together because both
   concern final file mode reconciliation.
4. Resolve high-impact clone/fetch compatibility issues: #1025, #2140, #2313,
   #2210, and #1055.
5. Work through status/worktree correctness: #2490, #2052, #1912, #2067.
6. Batch shell-command issues #1868, #1869, and #1842 since they share the
   `gix-command` execution model.
7. Dedicate a separate pass to platform test failures: #1358, #1890, #1894,
   #1622, #2548, and #2259.
