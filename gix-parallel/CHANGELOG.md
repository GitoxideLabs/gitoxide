# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.50.2 (2026-11-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 9 commits contributed to the release.
- 45 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge pull request #3211 from GitoxideLabs/jiff-upgrade ([`12eff45`](https://github.com/GitoxideLabs/gitoxide/commit/12eff45f0e16fa7a9f75086034ccb0198f55b9f3))
  - Merge pull request #3168 from GitoxideLabs/fix-ident-filter ([`f06f106`](https://github.com/GitoxideLabs/gitoxide/commit/f06f106b1d5b158f90a41d1d64970be76c1cfd2b))
  - Merge pull request #3143 from GitoxideLabs/fast-ci ([`c4a28f8`](https://github.com/GitoxideLabs/gitoxide/commit/c4a28f8c9331603ee2e2928974ee587a23026c49))
  - Discover QA work from manifests and share it between local runs and CI ([`8bf25a2`](https://github.com/GitoxideLabs/gitoxide/commit/8bf25a232d00f71f73b964f39d509ac9ae976efe))
  - Merge pull request #3117 from GitoxideLabs/release-testtools ([`8929480`](https://github.com/GitoxideLabs/gitoxide/commit/89294801971c59a8379344ca4404aa36f0305761))
  - Release gix-utils v0.5.2, gix-parallel v0.50.1, gix-hashtable v0.18.1, gix-sec v0.16.1, gix-config v0.62.2, gix-ignore v0.24.1, gix-imara-diff v0.3.3, gix-testtools v0.22.0 ([`584a602`](https://github.com/GitoxideLabs/gitoxide/commit/584a602385dd3ad4dacfdd9fd77a13c193a87dfe))
  - Merge pull request #3083 from GitoxideLabs/report-october ([`e87b6ee`](https://github.com/GitoxideLabs/gitoxide/commit/e87b6eefbcaa77156ddee85576c847e395ec0586))
  - Release gix-error v0.5.2, gix-date v0.18.1, gix-actor v0.44.1, gix-trace v0.3.1, gix-validate v0.13.1, gix-path v0.14.1, gix-quote v0.10.1, gix-command v0.12.1, gix-hash v0.28.1, gix-fs v0.24.1, gix-tempfile v26.0.1, gix-object v0.66.1, gix-glob v0.29.1, gix-attributes v0.37.1, gix-packetline v0.24.1, gix-filter v0.36.1, gix-chunk v0.10.1, gix-commitgraph v0.42.0, gix-revwalk v0.38.0, gix-traverse v0.64.0, gix-worktree-stream v0.39.0, gix-archive v0.39.0, gix-bitmap v0.6.1, gix-lock v26.0.1, gix-index v0.58.0, gix-config-value v0.21.1, gix-pathspec v0.22.1, gix-worktree v0.59.0, gix-imara-diff v0.3.2, gix-diff v0.70.0, gix-blame v0.20.0, gix-ref v0.69.1, gix-config v0.62.1, gix-prompt v0.19.1, gix-url v0.40.1, gix-credentials v0.42.1, gix-discover v0.57.1, gix-dir v0.32.0, gix-mailmap v0.36.1, gix-revision v0.52.0, gix-merge v0.23.0, gix-negotiate v0.38.0, gix-note v0.3.1, gix-zlib v0.4.0, gix-pack v0.77.0, gix-odb v0.87.0, gix-refspec v0.47.1, gix-shallow v0.15.1, gix-transport v0.62.0, gix-protocol v0.68.0, gix-status v0.37.0, gix-submodule v0.36.1, gix-worktree-state v0.37.0, gix v0.90.0, gix-fsck v0.27.1, gitoxide-core v0.64.0, gix-tix v0.6.0, gitoxide v0.61.0, safety bump 21 crates ([`dd39fb6`](https://github.com/GitoxideLabs/gitoxide/commit/dd39fb66e3b1551c9fe849043e39b357a9799ca1))
  - Merge pull request #3048 from GitoxideLabs/new-release ([`5fccccf`](https://github.com/GitoxideLabs/gitoxide/commit/5fccccfdedd17f7d4b0cc57039d118a3496aa8aa))

</details>

## 0.50.1 (2026-10-24)

## 0.50.0 (2026-10-08)

### Changed (BREAKING)

- <csr-id-0dcb9c164d141825514f61b7faf021af55385bcc/> centralize plumbing thread-safety configuration
  <!-- agent -->
  Plumbing crates share the threading primitives and algorithms selected by
  `gix-parallel/parallel`. Their forwarding-only `parallel` features duplicate
  that switch and obscure Cargo's feature unification. Remove those features
  from `gix-attributes`, `gix-filter`, `gix-index`, `gix-odb`, `gix-pathspec`,
  `gix-ref`, `gix-status`, `gix-worktree`, and `gix-worktree-state`.
  
  **Breaking change:** callers using the removed features must enable
  `gix-parallel/parallel` directly. Keep `gix/parallel` as the porcelain switch,
  forwarding to `gix-parallel/parallel` and the distinct `gix-pack/parallel`.
  
  Tree streaming always spawns a producer thread, so `gix-worktree-stream`
  enables `gix-parallel/parallel` directly and documents its effect on other
  crates through feature unification. Make `gix/worktree-stream` also enable
  `gix/parallel`, including through `worktree-archive`, so locally gated code
  and shared thread-safe types agree. Likewise, select thread-safe types for
  `gix-testtools`' shared fixture-exclusion cache explicitly.
  
  Move thread-safety assertions and the object-database concurrency regression
  into test targets requiring `gix-parallel/parallel`. Collect worker handles
  before releasing the barrier and use at least two workers, ensuring the
  regression actually exercises concurrent access. Replace the reference-store
  fixture with a direct `Send + Sync` assertion.
  
  Avoid enabling parallelism indirectly through test-helper defaults where
  serial coverage is intended. Update `just unit-tests` to exercise serial
  plumbing and explicitly selected threaded targets, including SHA-256 object
  lookup coverage, and remove the unused worktree forwarding dependency.
  
  Also replace the disconnected-branch ASCII graph snapshot with a
  `git rev-list disjoint --not main` comparison. This retains Git reference
  coverage without depending on version-specific graph layout.

### Other (BREAKING)

- <csr-id-0c49d795fbe135ed4e36dadf222abfc451b14fa6/> specialize `gix-features` for parallel execution
  <!-- Byron -->
  
  The `gix-parallel` crate is mostly copy-paste from `gix-features`, but
  most tests are newly generated.
  
  <!-- agent -->
  `gix-features` once coordinated several alternative implementations.
  Serial versus threaded execution is its remaining shared implementation
  policy; tracing, hashing, compression, and filesystem operations already
  have their own owners.
  
  Move progress, interruption, byte pipes, decoding, iterator chunks, and
  cache diagnostics into `gix-utils`. Keep utilities that add dependencies
  behind feature toggles, including `interrupt` for typed cancellation via
  `gix-error`. Use `crc32fast` directly in `gix-pack` and `gix-odb`.
  
  Rename the remaining crate to `gix-parallel` and expose its computation
  helpers, reducers, and shared ownership primitives at the crate root.
  Replace `gix::features` and `gix::threading` with the existing owner exports,
  including `gix::parallel` and `gix::utils`, and adapt all workspace callers.
  
  Expose both progress formatting choices individually on `gix`, retaining
  `comfort` as their bundle. Forward tracing directly to `gix-trace` and
  cache diagnostics to `gix-utils`. Update documentation, feature checks,
  the pack fuzz harness, and package-size checks for the new ownership.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 5 commits contributed to the release.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.0, gix-date v0.18.0, gix-actor v0.44.0, gix-trace v0.3.0, gix-validate v0.13.0, gix-path v0.14.0, gix-utils v0.5.0, gix-quote v0.10.0, gix-command v0.12.0, gix-hash v0.28.0, gix-hashtable v0.18.0, gix-parallel v0.50.0, gix-fs v0.24.0, gix-tempfile v26.0.0, gix-object v0.66.0, gix-glob v0.29.0, gix-attributes v0.37.0, gix-packetline v0.24.0, gix-filter v0.36.0, gix-chunk v0.10.0, gix-commitgraph v0.41.0, gix-revwalk v0.37.0, gix-traverse v0.63.0, gix-worktree-stream v0.38.0, gix-archive v0.38.0, gix-bitmap v0.6.0, gix-lock v26.0.0, gix-index v0.57.0, gix-config-value v0.21.0, gix-pathspec v0.22.0, gix-ignore v0.24.0, gix-worktree v0.58.0, gix-imara-diff v0.3.1, gix-diff v0.69.0, gix-blame v0.19.0, gix-ref v0.69.0, gix-sec v0.16.0, gix-config v0.62.0, gix-prompt v0.19.0, gix-url v0.40.0, gix-credentials v0.42.0, gix-discover v0.57.0, gix-dir v0.31.0, gix-mailmap v0.36.0, gix-revision v0.51.0, gix-merge v0.22.0, gix-negotiate v0.37.0, gix-note v0.3.0, gix-zlib v0.3.0, gix-pack v0.76.0, gix-odb v0.86.0, gix-refspec v0.47.0, gix-shallow v0.15.0, gix-transport v0.61.0, gix-protocol v0.67.0, gix-status v0.36.0, gix-submodule v0.36.0, gix-worktree-state v0.36.0, gix v0.89.0, gix-fsck v0.27.0, gitoxide-core v0.63.0, gix-tix v0.5.0, gitoxide v0.60.0, safety bump 62 crates ([`0115e90`](https://github.com/GitoxideLabs/gitoxide/commit/0115e902fd0334b8f370766993eb17babed3157f))
  - Prepare changelogs prior to release ([`794eeef`](https://github.com/GitoxideLabs/gitoxide/commit/794eeef835f8dde0fc8f4fd1d215f304d28d7f77))
  - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
  - Centralize plumbing thread-safety configuration ([`0dcb9c1`](https://github.com/GitoxideLabs/gitoxide/commit/0dcb9c164d141825514f61b7faf021af55385bcc))
  - Specialize `gix-features` for parallel execution ([`0c49d79`](https://github.com/GitoxideLabs/gitoxide/commit/0c49d795fbe135ed4e36dadf222abfc451b14fa6))

</details>
