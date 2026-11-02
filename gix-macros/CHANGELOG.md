# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.1 (2026-11-02)

### New Features

 - <csr-id-56c12b6fd979d1814ef1b55f93d0135bcbc4a24d/> gate experimental APIs with introduction dates
   <!-- agent -->
   A stable API needs to retain internal access to experimental items while making
   those items an explicit opt-in for downstream consumers. Cargo feature unification
   also means API checks must distinguish experimental members even when compiled
   with the feature enabled.
   
   Add `unstable` for items, inherent methods and trait implementations, plus
   `unstable_fields` for selected public fields. Disabled gates retain crate-private
   access where Rust permits it; trait implementations are omitted instead. Support
   file-backed modules on Rust 1.88 through inline bodies with `include!`;
   applying the attribute directly to `pub mod name;` requires Rust 1.99.
   
   Require `since = "YYYY-MM-DD"` with the item's introduction date from Git history.
   Validate calendar dates and include them in the generated instability marker.
   Keep `reason` optional and nonempty when supplied, for explanations beyond the
   signature or the fact that an item is experimental.
   
   Cover visibility, fields, trait implementations, documentation, missing and
   malformed dates, leap years and centuries, and invalid or duplicate arguments.
   The fixtures use their introduction dates from Git history.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #3061 from GitoxideLabs/stable-gix ([`5c88568`](https://github.com/GitoxideLabs/gitoxide/commit/5c88568c5ad8ab85ef4d60d64b4ab48958dc3226))
    - Gate experimental APIs with introduction dates ([`56c12b6`](https://github.com/GitoxideLabs/gitoxide/commit/56c12b6fd979d1814ef1b55f93d0135bcbc4a24d))
    - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
</details>

## 0.2.0 (2026-09-25)

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

 - 4 commits contributed to the release over the course of 31 calendar days.
 - 32 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
    - Merge pull request #2940 from GitoxideLabs/vendor-bisync ([`dda600d`](https://github.com/GitoxideLabs/gitoxide/commit/dda600d7ee29a6bda4cf1047d0d1782e76b16f98))
</details>

## 0.1.6 (2026-08-24)

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

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

 - <csr-id-209fb85c1129d7881a743cd71483700b0c6089f8/> replace the withdrawn bisync dependency
   <!-- agent -->
   The removal of bisync from crates.io made network-enabled builds impossible to
   resolve from a fresh dependency graph.
   
   Port its keep, discard, and async-to-sync attribute behavior into gix-macros,
   then preserve the existing mode-specific bisync aliases at each integration
   point. This removes both external bisync packages while keeping blocking and
   async implementations generated from the same source.
   
   A regression test first demonstrated the missing macros and blocking
   transformation, and now covers nested async move blocks, await removal, and
   mode-specific item selection.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release over the course of 794 calendar days.
 - 824 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2939](https://github.com/GitoxideLabs/gitoxide/issues/2939)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2939](https://github.com/GitoxideLabs/gitoxide/issues/2939)**
    - Replace the withdrawn bisync dependency ([`209fb85`](https://github.com/GitoxideLabs/gitoxide/commit/209fb85c1129d7881a743cd71483700b0c6089f8))
 * **Uncategorized**
    - Release gix-packetline v0.22.2, gix-worktree-stream v0.36.1, gix-archive v0.36.1, gix-diff v0.67.1, gix-blame v0.17.1, gix-dir v0.29.1, gix-mailmap v0.34.1, gix-revision v0.49.1, gix-merge v0.20.1, gix-negotiate v0.35.1, gix-note v0.1.1, gix-pack v0.74.2, gix-macros v0.1.6, gix-refspec v0.45.1, gix-transport v0.59.1, gix-protocol v0.65.1, gix-status v0.34.1, gix-worktree-state v0.34.1, gix v0.87.1, gix-fsck v0.25.1, gitoxide-core v0.61.1, gix-tix v0.3.0, gitoxide v0.58.0, safety bump gitoxide v0.58.0 ([`3ebca8b`](https://github.com/GitoxideLabs/gitoxide/commit/3ebca8b66017ab2dd02a38f75f78f485bee1ded8))
    - Merge pull request #2722 from GitoxideLabs/reasons ([`c16b5a1`](https://github.com/GitoxideLabs/gitoxide/commit/c16b5a1892704b7c72a253bdd74a6848dd61032a))
    - Replace lint allowances with expectations ([`43ff87a`](https://github.com/GitoxideLabs/gitoxide/commit/43ff87a73897b70313e3a58e7de82231be5b59ad))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
    - Make `package.include` patterns more specific so they don't match ignored files ([`c2c917f`](https://github.com/GitoxideLabs/gitoxide/commit/c2c917fce56c40a9af0d06bd603b7d1d2e51474f))
    - Merge pull request #2252 from GitoxideLabs/improvements ([`2f14246`](https://github.com/GitoxideLabs/gitoxide/commit/2f14246f01029ceee4c39fd9aedffe1fb938ba42))
    - Thanks clippy ([`70fcdb7`](https://github.com/GitoxideLabs/gitoxide/commit/70fcdb7ac53070c6f8577092dbe593f6acaf05b0))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
</details>

## 0.1.5 (2024-05-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 8 calendar days.
 - 69 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
    - Adapt to changes in `gix-pack` ([`bad5b48`](https://github.com/GitoxideLabs/gitoxide/commit/bad5b48e4f0d865b0b0937f136d9a0041aa88046))
</details>

## 0.1.4 (2024-03-14)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 4 calendar days.
 - 75 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
    - Prepare changelogs prior to release ([`52c3bbd`](https://github.com/GitoxideLabs/gitoxide/commit/52c3bbd36b9e94a0f3a78b4ada84d0c08eba27f6))
    - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
    - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
</details>

## 0.1.3 (2023-12-30)

<csr-id-3bd09ef120945a9669321ea856db4079a5dab930/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.3, gix-hash v0.14.1, gix-trace v0.1.6, gix-features v0.37.1, gix-actor v0.29.1, gix-validate v0.8.3, gix-object v0.40.1, gix-path v0.10.3, gix-glob v0.15.1, gix-quote v0.4.10, gix-attributes v0.21.1, gix-command v0.3.2, gix-packetline-blocking v0.17.2, gix-utils v0.1.8, gix-filter v0.8.1, gix-fs v0.9.1, gix-chunk v0.4.7, gix-commitgraph v0.23.1, gix-hashtable v0.5.1, gix-revwalk v0.11.1, gix-traverse v0.36.1, gix-worktree-stream v0.8.1, gix-archive v0.8.1, gix-config-value v0.14.3, gix-tempfile v12.0.1, gix-lock v12.0.1, gix-ref v0.40.1, gix-sec v0.10.3, gix-config v0.33.1, gix-prompt v0.8.2, gix-url v0.26.1, gix-credentials v0.23.1, gix-ignore v0.10.1, gix-bitmap v0.2.10, gix-index v0.28.1, gix-worktree v0.29.1, gix-diff v0.39.1, gix-discover v0.28.1, gix-macros v0.1.3, gix-mailmap v0.21.1, gix-negotiate v0.11.1, gix-pack v0.46.1, gix-odb v0.56.1, gix-pathspec v0.5.1, gix-packetline v0.17.2, gix-transport v0.40.1, gix-protocol v0.43.1, gix-revision v0.25.1, gix-refspec v0.21.1, gix-status v0.4.1, gix-submodule v0.7.1, gix-worktree-state v0.6.1, gix v0.57.1 ([`972241f`](https://github.com/GitoxideLabs/gitoxide/commit/972241f1904944e8b6e84c6aa1649a49be7a85c3))
    - Merge branch 'msrv' ([`8c492d7`](https://github.com/GitoxideLabs/gitoxide/commit/8c492d7b7e6e5d520b1e3ffeb489eeb88266aa75))
    - Change `rust-version` manifest field back to 1.65. ([`3bd09ef`](https://github.com/GitoxideLabs/gitoxide/commit/3bd09ef120945a9669321ea856db4079a5dab930))
</details>

## 0.1.2 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 14 calendar days.
 - 23 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
</details>

## 0.1.1 (2023-12-06)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 89 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`d3fd11e`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd11ec3783843d4e49081e1d14359ed9714b5f))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## 0.1.0 (2023-09-08)

The initial release with support for the `momo` proc-macro.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch 'feat/gix-momo' ([`a1ed6a1`](https://github.com/GitoxideLabs/gitoxide/commit/a1ed6a1aacae02a167b7ec44e1a47411a2194ff7))
    - Consolidate compile tests ([`d76efdd`](https://github.com/GitoxideLabs/gitoxide/commit/d76efddf5afb73563ce7e837cf975cedd01e979c))
    - Revert to use `item_fn.span()` ([`72545e9`](https://github.com/GitoxideLabs/gitoxide/commit/72545e971c894de511a723f6d5515f637a84f28f))
    - Fix ui test `error_if_ineffective` ([`9be2622`](https://github.com/GitoxideLabs/gitoxide/commit/9be26220876498df4d1add77da45c415268a77dc))
    - Add test-suite for failure modes ([`c8e7324`](https://github.com/GitoxideLabs/gitoxide/commit/c8e732430f3740348ccedd0dc1a9a28b06a0adee))
    - Remove `TODO` in `gix-macros/src/lib.rs` ([`c4ed7c1`](https://github.com/GitoxideLabs/gitoxide/commit/c4ed7c180e3ec1ff75cb10d78d4b8eed3b75be2f))
    - Improve docs ([`705f2f3`](https://github.com/GitoxideLabs/gitoxide/commit/705f2f34f1fa95d767646b154f41d2a6ce65ad10))
    - Refactor ([`48a2088`](https://github.com/GitoxideLabs/gitoxide/commit/48a20888d158b94811074a09a8c57ff5c8410769))
    - Feat `momo`: Rm unnecessary `#[allow(unused_mut)]` on generated inner fn ([`b619456`](https://github.com/GitoxideLabs/gitoxide/commit/b6194568e1d3042305f472103e1c00549cc4ccb9))
    - Feat `momo`: Support parsing pattern in params ([`b5f78be`](https://github.com/GitoxideLabs/gitoxide/commit/b5f78be06792153cd981c316a486974c000f1fd8))
    - Dramatically simplify `gix_macros::momo` ([`c72eaa0`](https://github.com/GitoxideLabs/gitoxide/commit/c72eaa05697a3e34adaa3ee90584dce4b5c00120))
    - Remove `TryInto` support from `gix_macros::momo` ([`95a1626`](https://github.com/GitoxideLabs/gitoxide/commit/95a16264b0a6f8c7d8e2acded3a4c9c170c2729b))
    - Add new crate `gix-macros` ([`6dae9e9`](https://github.com/GitoxideLabs/gitoxide/commit/6dae9e9d455ba5e3bf18c452789d07ff6cfaf392))
</details>

