

## 0.4.1 (2026-11-22)

### New Features

- <csr-id-710cadd1feff02ad2df0d6ce0904e56122be6336/> compress with `sirocco` with the `sirocco` feature
  `sirocco` 0.3 adds a zlib-compatible compressor whose levels compress
  about as well as zlib's same level, in less time. With the feature,
  `stream::deflate::Write` compresses with it; the standalone `Compress`
  API stays on `zlib-rs`, as `Decompress` does.
  
  - Encoders are reused across writers on a thread, up to two, by level:
  their tables are about 340 KiB, and a writer is made per object.
  - Input is buffered up to 1 MiB, so objects up to that size are
  compressed in one call; larger ones stream through that buffer.
  - Writes after `flush()` fail until `reset()`, as with `zlib-rs`.
  
  On git.git's objects, with a writer per object as gitoxide makes them,
  level 1 (loose objects) is 22% faster than `zlib-rs` with 16% smaller
  output, and level 6 (packs) is 1.5x as fast with 0.9% smaller output.
  
  The `zlib-sirocco` feature of `gix` and the `fast` feature of `gitoxide`
  now compress with it too.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 7 commits contributed to the release.
- 31 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3143 from GitoxideLabs/fast-ci ([`c4a28f8`](https://github.com/GitoxideLabs/gitoxide/commit/c4a28f8c9331603ee2e2928974ee587a23026c49))
  - Discover QA work from manifests and share it between local runs and CI ([`8bf25a2`](https://github.com/GitoxideLabs/gitoxide/commit/8bf25a232d00f71f73b964f39d509ac9ae976efe))
  - Merge pull request #3127 from srijs/gix-zlib-sirocco-deflate ([`8644bf3`](https://github.com/GitoxideLabs/gitoxide/commit/8644bf3a05ec10a9fcf6bfa461b79d0378825aad))
  - Review ([`9b8c4e1`](https://github.com/GitoxideLabs/gitoxide/commit/9b8c4e141a5ad26793fcf31896051f258e15a68c))
  - Compress with `sirocco` with the `sirocco` feature ([`710cadd`](https://github.com/GitoxideLabs/gitoxide/commit/710cadd1feff02ad2df0d6ce0904e56122be6336))
  - Merge pull request #3083 from GitoxideLabs/report-october ([`e87b6ee`](https://github.com/GitoxideLabs/gitoxide/commit/e87b6eefbcaa77156ddee85576c847e395ec0586))

</details>

## 0.4.0 (2026-10-22)

### New Features

- <csr-id-2b4faf99e11d788ffd98cfa5bd62c3dda2650515/> move the `sirocco` feature to `sirocco` 0.2
  `sirocco` 0.2 decodes into a buffer the caller owns, in one call or many,
  so it now handles all zlib decompression with the feature, with no
  fallback to `zlib-rs`:
  
  - `Inflate::once()` and `Inflate::prefix()` decode with `sirocco`. A
  short output buffer gets `sirocco`'s partial output.
  - The new `Inflate::rest()` continues the stream `prefix()` started, which
  loose objects use for their bodies instead of `Inflate::state`.
  Without the feature it continues with `zlib-rs` as before.
  - `stream::inflate::Measure` uses `sirocco::inflate::Scanner`.
  
  `sirocco`'s errors map to the classes and messages `zlib-rs` produces,
  corruption as "Invalid input data" and a preset dictionary as a
  validation error.
- <csr-id-95a0b1427b59a6a0bbe14cf6fc025c936236199d/> add the `sirocco` feature to decompress with `sirocco`
  `sirocco` is a pure-Rust DEFLATE/zlib decompressor that is faster than `zlib-rs`
  on whole buffers, and can find where a stream ends without producing its output.
  With the feature enabled:
  
  - `Inflate::once()` decodes whole streams with `sirocco`, and falls back to `zlib-rs`
  on any error, so partial output, statuses and errors stay what they were.
  - The new `Inflate::prefix()` always uses `zlib-rs`, for callers that only want the
  start of a stream: the delta-size probe in `gix-pack` and the loose object header.
  - The new `stream::inflate::Measure` finds the compressed and decompressed size of a
  stream read from a `BufRead`. `BytesToEntriesIter` uses it instead of decompressing
  into `io::sink()`. With `sirocco`, the decompressed bytes aren't produced, so the
  Adler-32 checksum isn't checked in that pass; the delta-resolution pass of index
  creation still decodes and checks every object.
  
  `zlib-rs` is still used for everything else, including all compression.
  
  The `gix` crate exposes it as `zlib-sirocco`, and the `fast` feature of `gitoxide`
  enables it for the `gix` and `ein` binaries. `sirocco` requires Rust 1.89.

### Changed (BREAKING)

- <csr-id-0ed7c8cbc8b18462f3da57724dc16a83a4e738ba/> hide the `Inflate` backend and align stream lifecycles
  Spend a lot of time with this one and massaged it into place, overall simplifying
  a little and adding test-coverage (that I am sure is already in Sirocco but doppelt
  haelt besser).
  
  <!-- agent -->
  The public `Inflate::state` field exposed a `zlib-rs` decoder even when
  `sirocco` handled all decoding. Make backend state private and retain only
  the selected implementation, avoiding an unused `zlib-rs` allocation when
  `sirocco` is enabled.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 14 calendar days.
- 14 days passed between releases.
- 3 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.2, gix-date v0.18.1, gix-actor v0.44.1, gix-trace v0.3.1, gix-validate v0.13.1, gix-path v0.14.1, gix-quote v0.10.1, gix-command v0.12.1, gix-hash v0.28.1, gix-fs v0.24.1, gix-tempfile v26.0.1, gix-object v0.66.1, gix-glob v0.29.1, gix-attributes v0.37.1, gix-packetline v0.24.1, gix-filter v0.36.1, gix-chunk v0.10.1, gix-commitgraph v0.42.0, gix-revwalk v0.38.0, gix-traverse v0.64.0, gix-worktree-stream v0.39.0, gix-archive v0.39.0, gix-bitmap v0.6.1, gix-lock v26.0.1, gix-index v0.58.0, gix-config-value v0.21.1, gix-pathspec v0.22.1, gix-worktree v0.59.0, gix-imara-diff v0.3.2, gix-diff v0.70.0, gix-blame v0.20.0, gix-ref v0.69.1, gix-config v0.62.1, gix-prompt v0.19.1, gix-url v0.40.1, gix-credentials v0.42.1, gix-discover v0.57.1, gix-dir v0.32.0, gix-mailmap v0.36.1, gix-revision v0.52.0, gix-merge v0.23.0, gix-negotiate v0.38.0, gix-note v0.3.1, gix-zlib v0.4.0, gix-pack v0.77.0, gix-odb v0.87.0, gix-refspec v0.47.1, gix-shallow v0.15.1, gix-transport v0.62.0, gix-protocol v0.68.0, gix-status v0.37.0, gix-submodule v0.36.1, gix-worktree-state v0.37.0, gix v0.90.0, gix-fsck v0.27.1, gitoxide-core v0.64.0, gix-tix v0.6.0, gitoxide v0.61.0, safety bump 21 crates ([`dd39fb6`](https://github.com/GitoxideLabs/gitoxide/commit/dd39fb66e3b1551c9fe849043e39b357a9799ca1))
  - Merge pull request #3065 from srijs/claude/project-thread-kr3hdl ([`0a49d46`](https://github.com/GitoxideLabs/gitoxide/commit/0a49d46f6a0b08943930d5247693992b89e98c1b))
  - Hide the `Inflate` backend and align stream lifecycles ([`0ed7c8c`](https://github.com/GitoxideLabs/gitoxide/commit/0ed7c8cbc8b18462f3da57724dc16a83a4e738ba))
  - Move the `sirocco` feature to `sirocco` 0.2 ([`2b4faf9`](https://github.com/GitoxideLabs/gitoxide/commit/2b4faf99e11d788ffd98cfa5bd62c3dda2650515))
  - Add the `sirocco` feature to decompress with `sirocco` ([`95a0b14`](https://github.com/GitoxideLabs/gitoxide/commit/95a0b1427b59a6a0bbe14cf6fc025c936236199d))
  - Merge pull request #3053 from GitoxideLabs/various-improvements ([`2d8dcc4`](https://github.com/GitoxideLabs/gitoxide/commit/2d8dcc466750e474e6c51ac636c24259563a26cf))
  - Release gix-error v0.5.1, gix-utils v0.5.1 ([`6d1b686`](https://github.com/GitoxideLabs/gitoxide/commit/6d1b68632cec55fa5d3f752ca97daca4e5485b6c))
  - Merge pull request #3048 from GitoxideLabs/new-release ([`5fccccf`](https://github.com/GitoxideLabs/gitoxide/commit/5fccccfdedd17f7d4b0cc57039d118a3496aa8aa))

</details>

## 0.3.0 (2026-10-08)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 13 calendar days.
- 13 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.0, gix-date v0.18.0, gix-actor v0.44.0, gix-trace v0.3.0, gix-validate v0.13.0, gix-path v0.14.0, gix-utils v0.5.0, gix-quote v0.10.0, gix-command v0.12.0, gix-hash v0.28.0, gix-hashtable v0.18.0, gix-parallel v0.50.0, gix-fs v0.24.0, gix-tempfile v26.0.0, gix-object v0.66.0, gix-glob v0.29.0, gix-attributes v0.37.0, gix-packetline v0.24.0, gix-filter v0.36.0, gix-chunk v0.10.0, gix-commitgraph v0.41.0, gix-revwalk v0.37.0, gix-traverse v0.63.0, gix-worktree-stream v0.38.0, gix-archive v0.38.0, gix-bitmap v0.6.0, gix-lock v26.0.0, gix-index v0.57.0, gix-config-value v0.21.0, gix-pathspec v0.22.0, gix-ignore v0.24.0, gix-worktree v0.58.0, gix-imara-diff v0.3.1, gix-diff v0.69.0, gix-blame v0.19.0, gix-ref v0.69.0, gix-sec v0.16.0, gix-config v0.62.0, gix-prompt v0.19.0, gix-url v0.40.0, gix-credentials v0.42.0, gix-discover v0.57.0, gix-dir v0.31.0, gix-mailmap v0.36.0, gix-revision v0.51.0, gix-merge v0.22.0, gix-negotiate v0.37.0, gix-note v0.3.0, gix-zlib v0.3.0, gix-pack v0.76.0, gix-odb v0.86.0, gix-refspec v0.47.0, gix-shallow v0.15.0, gix-transport v0.61.0, gix-protocol v0.67.0, gix-status v0.36.0, gix-submodule v0.36.0, gix-worktree-state v0.36.0, gix v0.89.0, gix-fsck v0.27.0, gitoxide-core v0.63.0, gix-tix v0.5.0, gitoxide v0.60.0, safety bump 62 crates ([`0115e90`](https://github.com/GitoxideLabs/gitoxide/commit/0115e902fd0334b8f370766993eb17babed3157f))
  - Prepare changelogs prior to release ([`794eeef`](https://github.com/GitoxideLabs/gitoxide/commit/794eeef835f8dde0fc8f4fd1d215f304d28d7f77))
  - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
  - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
  - Merge pull request #3022 from GitoxideLabs/release-testtools ([`f819565`](https://github.com/GitoxideLabs/gitoxide/commit/f819565c2c4c56619c4888acef6cf3b8144cbccb))
  - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))

</details>

## 0.2.0 (2026-09-25)

### Changed (BREAKING)

- <csr-id-383779c7f3c64c862a1101c6e196aa462160635f/> migrate errors to gix-error
- <csr-id-4b42e0ce80ae934cae4f102f44c392581758608f/> raise MSRV to Rust 1.88
  <!-- agent -->
  The newly published `dua-core` 3.3 release used by linked-worktree removal
  requires Rust 1.88, so raise every workspace crate and the advertised badge
  together.
  
  Keep the MSRV checks buildable by selecting the latest `sysinfo` and `rusqlite`
  release lines that support Rust 1.88.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 63 calendar days.
- 64 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
  - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
  - Use borrowed error inspection throughout the workspace ([`daf73b5`](https://github.com/GitoxideLabs/gitoxide/commit/daf73b5fe5a21e3ddcc58f0882c2d880f48b7860))
  - Merge pull request #2989 from GitoxideLabs/error-conversion-review ([`4b9ff51`](https://github.com/GitoxideLabs/gitoxide/commit/4b9ff511a49f7963e97a669ca82c6f6e833d8ea2))
  - Migrate errors to gix-error ([`383779c`](https://github.com/GitoxideLabs/gitoxide/commit/383779c7f3c64c862a1101c6e196aa462160635f))
  - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
  - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
  - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))

</details>

## 0.1.0 (2026-07-23)

### New Features

- <csr-id-519e2d88a91d33b1576eabc5df0c2eccd7722fa0/> add `gix-zlib` crate
  This merely moves the `zlib` module into its own crate.
  Previously, there were multiple backends, but these times
  are over for a while and there is only one implementation: zlib-rs.

### New Features (BREAKING)

- <csr-id-6936e855adbce26c3acdc2ac2c3a6c6eef4e643e/> make zlib compression levels configurable
  Introduce a validated Compression type and require callers to choose a level when creating deflate streams. Optional serde support allows options in dependent crates to retain their serialization APIs.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 9 commits contributed to the release over the course of 13 calendar days.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Update changelogs prior to release ([`cb6ec7d`](https://github.com/GitoxideLabs/gitoxide/commit/cb6ec7dce283943d811b1600b577f586d7a13e1f))
  - Merge pull request #2722 from GitoxideLabs/reasons ([`c16b5a1`](https://github.com/GitoxideLabs/gitoxide/commit/c16b5a1892704b7c72a253bdd74a6848dd61032a))
  - Replace lint allowances with expectations ([`43ff87a`](https://github.com/GitoxideLabs/gitoxide/commit/43ff87a73897b70313e3a58e7de82231be5b59ad))
  - Merge pull request #2695 from ameyypawar/fix/2024-compression-level ([`6e1c4a2`](https://github.com/GitoxideLabs/gitoxide/commit/6e1c4a24813d99ad0bbfb231618210ffe6a5cd6a))
  - Review ([`f1ac335`](https://github.com/GitoxideLabs/gitoxide/commit/f1ac3359c3d88f550219116f1f3e8cb107f5f86f))
  - Make zlib compression levels configurable ([`6936e85`](https://github.com/GitoxideLabs/gitoxide/commit/6936e855adbce26c3acdc2ac2c3a6c6eef4e643e))
  - Merge pull request #2707 from ameyypawar/fix/2703-inflate-error ([`6d95da6`](https://github.com/GitoxideLabs/gitoxide/commit/6d95da6e7082e19a03123ad765b3d5f117731621))
  - Adapt to changes in `gix-features`, use `gix-zlib` accordingly. ([`9c2977a`](https://github.com/GitoxideLabs/gitoxide/commit/9c2977a3b6d540690a1a263a037c8d54c316a020))
  - Add `gix-zlib` crate ([`519e2d8`](https://github.com/GitoxideLabs/gitoxide/commit/519e2d88a91d33b1576eabc5df0c2eccd7722fa0))

</details>
