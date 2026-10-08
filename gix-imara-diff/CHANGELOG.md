

## 0.3.1 (2026-10-08)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 13 calendar days.
 - 13 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Prepare changelogs prior to release ([`794eeef`](https://github.com/GitoxideLabs/gitoxide/commit/794eeef835f8dde0fc8f4fd1d215f304d28d7f77))
    - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
    - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
</details>

## 0.3.0 (2026-09-25)

### Bug Fixes

 - <csr-id-f48f94d38fe0e24ac9e1d6ea8469b2713b197927/> track end of file and trailing blanks the way git does
   Two bugs that were propping each other up.
   
   git tracks end of file positionally. measure_split sets end_of_file when
   `split >= nrec`, meaning the split sits past the last line, and that flag is
   the only thing END_OF_FILE_PENALTY keys off. Indents had no such flag, so
   score() inferred it from content instead: `next_indent == BLANK &&
   trailing_blanks == 0`.
 - <csr-id-4d137c35c8f5bca445f191eda0db04ca5600f66e/> round sqrt approximation the same way as xdl_bogosqrt
   sqrt() rounds the halved bit count down, where git's xdl_bogosqrt rounds it up:
   
   for (i = 1; n > 0; n >>= 2) i <<= 1;
   
   For every odd bit length it halves the result. A 450-line file gets a limit
   of 16 instead of git's 32, so a line occurring 27 times is treated as too
   frequent to be worth matching and becomes a candidate for discarding, while git
   treats it as ordinary and matches it. The same value is the cost ceiling the
   Myers search gives up at, which was likewise half of git's.
 - <csr-id-b813b0024e3b37a125932ad0591efc665170d261/> count candidate lines the way xdl_clean_mmatch does
   Before running Myers, a frequent line is discarded only when it sits inside a
   run of lines that match nothing. Git decides that in xdl_clean_mmatch
   (xdiff/xprepare.c), where the backward and forward scans each start their
   frequent-line counter at 1, so the line under test contributes 2 to the total:
   
   for (r = 1, rdis0 = 0, rpdis0 = 1; (i - r) >= s; r++) { ... }
   for (r = 1, rdis1 = 0, rpdis1 = 1; (i + r) <= e; r++) { ... }
   rdis1 += rdis0; rpdis1 += rpdis0;
   return rpdis1 * XDL_KPDIS_RUN < (rpdis1 + rdis1);
   
   should_prune_common_line started both counters at 0 and started its forward
   loop beginning at the line itself, so it contributed 1. The threshold for
   discarding was therefore an unmatched run longer than 3 rather than longer
   than 6, and a discarded line can never be matched, costing one removal and
   one insertion against git's answer each time.

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

 - 9 commits contributed to the release over the course of 33 calendar days.
 - 34 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
    - Merge pull request #2987 from tcrypt25519/tcrypt/diff-parity-fixes ([`7665437`](https://github.com/GitoxideLabs/gitoxide/commit/766543702cbf39d1c466de58fcefaed78c91af75))
    - Review ([`929df61`](https://github.com/GitoxideLabs/gitoxide/commit/929df61975bcebaa0377a551317302d2cc4b39c0))
    - Track end of file and trailing blanks the way git does ([`f48f94d`](https://github.com/GitoxideLabs/gitoxide/commit/f48f94d38fe0e24ac9e1d6ea8469b2713b197927))
    - Round sqrt approximation the same way as xdl_bogosqrt ([`4d137c3`](https://github.com/GitoxideLabs/gitoxide/commit/4d137c35c8f5bca445f191eda0db04ca5600f66e))
    - Count candidate lines the way xdl_clean_mmatch does ([`b813b00`](https://github.com/GitoxideLabs/gitoxide/commit/b813b0024e3b37a125932ad0591efc665170d261))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
</details>

## 0.2.5 (2026-08-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 30 calendar days.
 - 30 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.3.0, gix-date v0.16.0, gix-actor v0.42.0, gix-validate v0.11.4, gix-path v0.12.5, gix-utils v0.3.6, gix-quote v0.8.0, gix-command v0.10.0, gix-features v0.49.1, gix-hash v0.26.1, gix-fs v0.22.1, gix-object v0.64.0, gix-glob v0.27.1, gix-attributes v0.35.0, gix-filter v0.34.0, gix-chunk v0.8.0, gix-commitgraph v0.39.0, gix-revwalk v0.35.0, gix-traverse v0.61.0, gix-worktree-stream v0.36.0, gix-archive v0.36.0, gix-bitmap v0.4.0, gix-index v0.55.0, gix-pathspec v0.20.0, gix-ignore v0.22.1, gix-worktree v0.56.0, gix-imara-diff v0.2.5, gix-diff v0.67.0, gix-blame v0.17.0, gix-ref v0.67.0, gix-config v0.60.0, gix-prompt v0.17.0, gix-url v0.38.0, gix-credentials v0.40.0, gix-discover v0.55.0, gix-dir v0.29.0, gix-mailmap v0.34.0, gix-revision v0.49.0, gix-merge v0.20.0, gix-negotiate v0.35.0, gix-note v0.1.0, gix-pack v0.74.0, gix-odb v0.84.0, gix-refspec v0.45.0, gix-transport v0.59.0, gix-protocol v0.65.0, gix-status v0.34.0, gix-submodule v0.34.0, gix-worktree-state v0.34.0, gix v0.87.0, gix-fsck v0.25.0, gitoxide-core v0.61.0, gix-tix v0.2.0, gitoxide v0.57.0 ([`d2af4ed`](https://github.com/GitoxideLabs/gitoxide/commit/d2af4ed5532ea660fbd643e48d8925cd88de5ee0))
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))
</details>

## 0.2.4 (2026-07-23)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 31 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.41.2, gix-features v0.49.0, gix-hash v0.26.0, gix-hashtable v0.16.0, gix-object v0.63.0, gix-glob v0.27.0, gix-attributes v0.34.0, gix-packetline v0.22.0, gix-filter v0.33.0, gix-fs v0.22.0, gix-chunk v0.7.3, gix-commitgraph v0.38.0, gix-revwalk v0.34.0, gix-traverse v0.60.0, gix-worktree-stream v0.35.0, gix-archive v0.35.0, gix-bitmap v0.3.3, gix-tempfile v24.0.0, gix-lock v24.0.0, gix-index v0.54.0, gix-pathspec v0.19.0, gix-ignore v0.22.0, gix-worktree v0.55.0, gix-imara-diff v0.2.4, gix-diff v0.66.0, gix-blame v0.16.0, gix-ref v0.66.0, gix-config v0.59.0, gix-discover v0.54.0, gix-dir v0.28.0, gix-mailmap v0.33.2, gix-revision v0.48.0, gix-merge v0.19.0, gix-negotiate v0.34.0, gix-zlib v0.1.0, gix-pack v0.73.0, gix-odb v0.83.0, gix-refspec v0.44.0, gix-shallow v0.13.0, gix-transport v0.58.0, gix-protocol v0.64.0, gix-status v0.33.0, gix-submodule v0.33.0, gix-worktree-state v0.33.0, gix v0.86.0, gix-fsck v0.24.0, gitoxide-core v0.60.0, gix-tix v0.1.0, gitoxide v0.56.0, safety bump 40 crates ([`842bc44`](https://github.com/GitoxideLabs/gitoxide/commit/842bc447e3aeacf5d9d36f7f8a01068eda4b7999))
    - Update changelogs prior to release ([`cb6ec7d`](https://github.com/GitoxideLabs/gitoxide/commit/cb6ec7dce283943d811b1600b577f586d7a13e1f))
    - Merge pull request #2722 from GitoxideLabs/reasons ([`c16b5a1`](https://github.com/GitoxideLabs/gitoxide/commit/c16b5a1892704b7c72a253bdd74a6848dd61032a))
    - Replace lint allowances with expectations ([`43ff87a`](https://github.com/GitoxideLabs/gitoxide/commit/43ff87a73897b70313e3a58e7de82231be5b59ad))
    - Merge pull request #2702 from ameyypawar/fix/2694-exn-source-chain ([`e9c973d`](https://github.com/GitoxideLabs/gitoxide/commit/e9c973d9476bef293bec89cd683cb60b02a85e52))
    - Thanks clippy ([`d533f0c`](https://github.com/GitoxideLabs/gitoxide/commit/d533f0c7a6a6b20cbcfe5a755d472b921e38b4a1))
    - Merge pull request #2646 from GitoxideLabs/report ([`1b1541e`](https://github.com/GitoxideLabs/gitoxide/commit/1b1541ed7a457afd48385c1ee39113949a9f5263))
</details>

## 0.2.3 (2026-06-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.15.5, gix-hashtable v0.15.2, gix-object v0.62.0, gix-attributes v0.33.2, gix-filter v0.32.0, gix-revwalk v0.33.0, gix-traverse v0.59.0, gix-worktree-stream v0.34.0, gix-archive v0.34.0, gix-tempfile v23.0.2, gix-index v0.53.0, gix-worktree v0.54.0, gix-imara-diff v0.2.3, gix-diff v0.65.0, gix-blame v0.15.0, gix-ref v0.65.0, gix-config v0.58.0, gix-discover v0.53.0, gix-dir v0.27.0, gix-revision v0.47.0, gix-merge v0.18.0, gix-negotiate v0.33.0, gix-pack v0.72.0, gix-odb v0.82.0, gix-refspec v0.43.0, gix-transport v0.57.2, gix-protocol v0.63.0, gix-status v0.32.0, gix-submodule v0.32.0, gix-worktree-state v0.32.0, gix v0.85.0, gix-fsck v0.23.0, gitoxide-core v0.59.0, gitoxide v0.55.0, safety bump 28 crates ([`6428edc`](https://github.com/GitoxideLabs/gitoxide/commit/6428edc82fc8a16d5ef34ca2d49aa6fdff3645fe))
    - Merge pull request #2650 from GitoxideLabs/fix-fuzz-comprehensive-diff ([`f1fc9ea`](https://github.com/GitoxideLabs/gitoxide/commit/f1fc9ea44730fd096f22d05ed925ebc304675e8b))
    - Bound comprehensive diff fuzz inputs to avoid Myers timeout ([`58e7b75`](https://github.com/GitoxideLabs/gitoxide/commit/58e7b75a8d6337e8106e4691c5b76df754b4a56e))
    - Merge pull request #2644 from GitoxideLabs/fix-postprocess-timeout ([`8f55e18`](https://github.com/GitoxideLabs/gitoxide/commit/8f55e18b8b77f6ac76ea11ed1c0adc8b0fbfc40b))
    - Address auto-review ([`40c8690`](https://github.com/GitoxideLabs/gitoxide/commit/40c8690af54ec160ced15e7c9293583118019631))
    - Limit postprocess heuristics fuzz input size ([`20649b1`](https://github.com/GitoxideLabs/gitoxide/commit/20649b1076b84ed8cc6aa17490647220af19c382))
    - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))
</details>

## 0.2.2 (2026-05-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.4, gix-date v0.15.4, gix-actor v0.41.1, gix-trace v0.1.20, gix-validate v0.11.2, gix-path v0.12.1, gix-utils v0.3.3, gix-features v0.48.1, gix-hash v0.25.1, gix-hashtable v0.15.1, gix-object v0.61.0, gix-glob v0.26.1, gix-quote v0.7.2, gix-attributes v0.33.1, gix-command v0.9.1, gix-packetline v0.21.4, gix-filter v0.31.0, gix-fs v0.21.2, gix-chunk v0.7.2, gix-commitgraph v0.37.1, gix-revwalk v0.32.0, gix-traverse v0.58.0, gix-worktree-stream v0.33.0, gix-archive v0.33.0, gix-bitmap v0.3.2, gix-tempfile v23.0.1, gix-lock v23.0.1, gix-index v0.52.0, gix-config-value v0.18.1, gix-pathspec v0.18.1, gix-ignore v0.21.1, gix-worktree v0.53.0, gix-imara-diff v0.2.2, gix-diff v0.64.0, gix-blame v0.14.0, gix-ref v0.64.0, gix-sec v0.14.1, gix-config v0.57.0, gix-prompt v0.15.1, gix-url v0.36.1, gix-credentials v0.38.1, gix-discover v0.52.0, gix-dir v0.26.0, gix-mailmap v0.33.1, gix-revision v0.46.0, gix-merge v0.17.0, gix-negotiate v0.32.0, gix-pack v0.71.0, gix-odb v0.81.0, gix-refspec v0.42.0, gix-shallow v0.12.1, gix-transport v0.57.1, gix-protocol v0.62.0, gix-status v0.31.0, gix-submodule v0.31.0, gix-worktree-state v0.31.0, gix v0.84.0, gix-fsck v0.22.0, gitoxide-core v0.58.0, gitoxide v0.54.0, safety bump 27 crates ([`10c58bb`](https://github.com/GitoxideLabs/gitoxide/commit/10c58bb56597d9335611da121aac21f9b09b6e5b))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Bump the cargo group across 1 directory with 10 updates ([`4c77f81`](https://github.com/GitoxideLabs/gitoxide/commit/4c77f81e19b86979495abcf46401a4f226163177))
    - Merge pull request #2532 from cruessler/run-gix-diff-tests-with-sha-256 ([`7fbb9be`](https://github.com/GitoxideLabs/gitoxide/commit/7fbb9be28e1cf3dc3af874a02dc688374e109cf8))
    - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))
</details>

## 0.2.1 (2026-04-28)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 2 calendar days.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
    - Remove `memchr` dependency from `gix-imara-diff` ([`f267626`](https://github.com/GitoxideLabs/gitoxide/commit/f26762623b22314c7571ccf6668c12fb70e4941b))
    - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))
</details>

## 0.2.0 (2026-04-24)

### Bug Fixes

 - <csr-id-7a1b9cd0224956e86f9db4cf5098f879eea195a3/> non-terminating MyersMinimal split loop`
   The clusterfuzz testcase
   `clusterfuzz-testcase-minimized-gix-imara-diff-comprehensive_diff-6497314075377664`
   was timing out in the Myers implementation while running the new
   `comprehensive_diff` fuzz target.
   
   Root cause

### New Features (BREAKING)

 - <csr-id-8094f5dcd4f24f4d54f7fbe7f716f80f2974b586/> Use `imara-diff-v2` with git sliders processing
   The slider post-processing imrpoves the diff quality for about 8% slower diffs.
   Line-counts, however, will be 50% faster to compute.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 11 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
    - Merge pull request #2530 from GitoxideLabs/advisories ([`63b8419`](https://github.com/GitoxideLabs/gitoxide/commit/63b841907ce30b36bb50da5aae3a9e1a06eadf64))
    - Add fuzz tests for 10 more crates, and related fixes ([`0396152`](https://github.com/GitoxideLabs/gitoxide/commit/03961523d0208a12b7b480b14d57793049600283))
    - Remove profile.release section in gix-imara-diff ([`6945969`](https://github.com/GitoxideLabs/gitoxide/commit/6945969015741a54b324056e769501fd3f42d6c6))
    - Merge pull request #2524 from GitoxideLabs/reproduce-fuzz-diff-timeout ([`353940d`](https://github.com/GitoxideLabs/gitoxide/commit/353940dee9fdabe3301d3fb8132c84228b9e8d95))
    - Non-terminating MyersMinimal split loop` ([`7a1b9cd`](https://github.com/GitoxideLabs/gitoxide/commit/7a1b9cd0224956e86f9db4cf5098f879eea195a3))
    - Merge pull request #2513 from GitoxideLabs/v2-diff ([`2a5db88`](https://github.com/GitoxideLabs/gitoxide/commit/2a5db88d0330b0d125de4b6f3819f17a7f76f4b8))
    - Thanks clippy ([`e4f380e`](https://github.com/GitoxideLabs/gitoxide/commit/e4f380eff3b0440002f7e9b64a14ddcfbe63192a))
    - Last stretch to fix CI ([`1be2d4d`](https://github.com/GitoxideLabs/gitoxide/commit/1be2d4dff8a5000a147f4e36861a8d929f07cd91))
    - Optimise gix-imara-diff manifest. ([`3ec346b`](https://github.com/GitoxideLabs/gitoxide/commit/3ec346b41febc0b931c449b2e8703a8654b808cb))
    - Add license attributions to `gix-imara-diff` properly ([`e2d767d`](https://github.com/GitoxideLabs/gitoxide/commit/e2d767df8fa01d9977289fa009d7fced4e6df666))
    - Use `imara-diff-v2` with git sliders processing ([`8094f5d`](https://github.com/GitoxideLabs/gitoxide/commit/8094f5dcd4f24f4d54f7fbe7f716f80f2974b586))
    - Merge pull request #2506 from GitoxideLabs/vendor-imara-diff ([`8f091d1`](https://github.com/GitoxideLabs/gitoxide/commit/8f091d108cd75371be2ed9de6e81f785cda53d92))
    - Vendor `imara-diff` 0.1 and 0.2 ([`fd49295`](https://github.com/GitoxideLabs/gitoxide/commit/fd49295c5ed4a57bf5771e23c0f803435990ecfa))
</details>

