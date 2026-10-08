# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
    - Centralize plumbing thread-safety configuration ([`0dcb9c1`](https://github.com/GitoxideLabs/gitoxide/commit/0dcb9c164d141825514f61b7faf021af55385bcc))
    - Specialize `gix-features` for parallel execution ([`0c49d79`](https://github.com/GitoxideLabs/gitoxide/commit/0c49d795fbe135ed4e36dadf222abfc451b14fa6))
</details>

