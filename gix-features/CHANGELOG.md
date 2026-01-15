# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Chore (BREAKING)

 - <csr-id-2358b1d250d3d2348210fb61dcb95ebe7aa6314b/> Upgrade `prodash` and `crosstermion` to the latest version.
   This will fix the `cargo deny` issue as it brings in a newer `lru` crate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 4 calendar days.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2353 from GitoxideLabs/improvements ([`ee0a1f6`](https://github.com/GitoxideLabs/gitoxide/commit/ee0a1f6cc99831d7c5f96b657df6ebca5ee5638c))
    - Upgrade `prodash` and `crosstermion` to the latest version. ([`2358b1d`](https://github.com/GitoxideLabs/gitoxide/commit/2358b1d250d3d2348210fb61dcb95ebe7aa6314b))
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
</details>

## 0.45.2 (2026-01-06)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 5 calendar days.
 - 5 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2341 from GitoxideLabs/dependabot/cargo/cargo-cf4a2135ae ([`d914d95`](https://github.com/GitoxideLabs/gitoxide/commit/d914d9533ed2243658d51ba05e68dd444b75a748))
    - Bump the cargo group across 1 directory with 51 updates ([`4edc5dd`](https://github.com/GitoxideLabs/gitoxide/commit/4edc5dda7ca39cc8249cb98dc39ca46c7d00eb44))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.45.1 (2025-12-31)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
    - Merge pull request #2301 from folkertdev/zlib-rs-0.5.5 ([`a000a73`](https://github.com/GitoxideLabs/gitoxide/commit/a000a7337d7e481ed2e576f66316cbcf0b01e19d))
    - Update to zlib-rs 0.5.5 ([`d77e15f`](https://github.com/GitoxideLabs/gitoxide/commit/d77e15f4707a612158c0ba30955583f88ddffc7e))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
</details>

## 0.45.0 (2025-12-22)

### New Features (BREAKING)

 - <csr-id-a75c852e93a026a6d07ffd966296ce1e42fa8aa5/> Replace libz-rs-sys with zlib-rs for safe zlib usage

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2298 from GitoxideLabs/copilot/replace-libz-rs-sys-with-zlib-rs ([`b77744f`](https://github.com/GitoxideLabs/gitoxide/commit/b77744f599d57d2fc9a710daf06c74387f6a60f9))
    - Address copilot review ([`2384d0c`](https://github.com/GitoxideLabs/gitoxide/commit/2384d0cae299c64577b10d61cdab63e4526c9082))
    - Refactor ([`e08b9b5`](https://github.com/GitoxideLabs/gitoxide/commit/e08b9b5ca957e6ca39baefb17ee2d144b022359d))
    - Replace libz-rs-sys with zlib-rs for safe zlib usage ([`a75c852`](https://github.com/GitoxideLabs/gitoxide/commit/a75c852e93a026a6d07ffd966296ce1e42fa8aa5))
    - Merge pull request #2275 from GitoxideLabs/dependabot/cargo/cargo-92eaa62a2e ([`93dd630`](https://github.com/GitoxideLabs/gitoxide/commit/93dd630ca6a2a4622ca74d7eaff42ece2750b6c5))
    - Bump the cargo group across 1 directory with 14 updates ([`703644c`](https://github.com/GitoxideLabs/gitoxide/commit/703644c8821aae161592d19495d3b3162133324f))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
</details>

## 0.44.1 (2025-10-23)

<csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/>

### Other

 - <csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/> Remove `doc_auto_cfg` feature to fix docs.rs documentation.
   It is part of `doc_cfg` feature since https://github.com/rust-lang/rust/pull/138907
   
   This fixes the docs.rs build

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.7, gix-actor v0.35.6, gix-trace v0.1.15, gix-features v0.44.1, gix-hash v0.20.1, gix-object v0.51.1, gix-glob v0.22.1, gix-attributes v0.28.1, gix-packetline-blocking v0.19.3, gix-commitgraph v0.30.1, gix-archive v0.23.1, gix-tempfile v19.0.1, gix-index v0.42.1, gix-config-value v0.15.3, gix-ignore v0.17.1, gix-worktree v0.43.1, gix-diff v0.54.1, gix-ref v0.54.1, gix-sec v0.12.2, gix-config v0.47.1, gix-url v0.33.1, gix-credentials v0.31.1, gix-mailmap v0.27.4, gix-revision v0.36.1, gix-pack v0.61.1, gix-odb v0.71.1, gix-packetline v0.19.3, gix-transport v0.49.1, gix-protocol v0.52.1, gix-status v0.21.1, gix v0.74.1 ([`bdcce5f`](https://github.com/GitoxideLabs/gitoxide/commit/bdcce5f2c6723ebe489dbe936a4656859ce1c2a5))
    - Remove `doc_auto_cfg` feature to fix docs.rs documentation. ([`6f469a6`](https://github.com/GitoxideLabs/gitoxide/commit/6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b))
    - Merge pull request #2224 from GitoxideLabs/report ([`3313233`](https://github.com/GitoxideLabs/gitoxide/commit/3313233aa4e7009aed0ddf644f4271fd2a98e8d4))
</details>

## 0.44.0 (2025-10-22)

### Bug Fixes (BREAKING)

 - <csr-id-f47f3e42b9d64274d210953c6fbd903b2d300f1f/> remove unused and previously deprecated `zlib` related features.
   We now use `zlib-rs` directly.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
    - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
    - Merge pull request #2155 from folkertdev/skip-flate2 ([`752d6dc`](https://github.com/GitoxideLabs/gitoxide/commit/752d6dc830f98980c96e9a1dc6ec8bf4432fb7f1))
    - Don't box compression/decompression type and avoid indirections and allocs ([`e5a7487`](https://github.com/GitoxideLabs/gitoxide/commit/e5a74876fd01ce89071701a7bb682642f7932418))
    - Remove unused and previously deprecated `zlib` related features. ([`f47f3e4`](https://github.com/GitoxideLabs/gitoxide/commit/f47f3e42b9d64274d210953c6fbd903b2d300f1f))
    - Refactor ([`0e7aa81`](https://github.com/GitoxideLabs/gitoxide/commit/0e7aa815dbe10e9d19334a3f0b881c6d608a3b8e))
    - In `gix-features`, use `libz-rs-sys` directly, skipping `flate2` ([`5a2361b`](https://github.com/GitoxideLabs/gitoxide/commit/5a2361b42b01d77a98c7f16c23783f69a59740f7))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
</details>

## 0.43.1 (2025-08-03)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 19 calendar days.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Prepare changelogs prior to release. ([`5e0122d`](https://github.com/GitoxideLabs/gitoxide/commit/5e0122df48392fb0ea6e16eb7b70d320b03244ca))
    - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
    - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.43.0 (2025-07-15)

### New Features (BREAKING)

 - <csr-id-6c77b541b476656827ee0542a650b9731ba549cf/> walkdir_sorted_new adds max_depth parameter
   max_depth parameter determines the maximum depth the WalkDir will
   recurse into.
   
   Example values:
   * 0    -> Returns only the root path with no children.
* 1    -> Returns the root path, with children.
* 2..n -> Returns the root path, children and {n}-grandchildren

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2061 from orthros/pseudo-refs ([`60c29a5`](https://github.com/GitoxideLabs/gitoxide/commit/60c29a59302bfc9d0be7aab5dd3ef05e4ee8e3fa))
    - Refactor ([`43f92b5`](https://github.com/GitoxideLabs/gitoxide/commit/43f92b5285af6696cd21f0e94f3bec568aef8468))
    - Walkdir_sorted_new adds max_depth parameter ([`6c77b54`](https://github.com/GitoxideLabs/gitoxide/commit/6c77b541b476656827ee0542a650b9731ba549cf))
    - Merge pull request #2070 from GitoxideLabs/dependabot/cargo/cargo-827bceb7eb ([`dab97f7`](https://github.com/GitoxideLabs/gitoxide/commit/dab97f7618f160421b6e31de8f3e2f3d11dc2ef2))
    - Bump the cargo group across 1 directory with 68 updates ([`a9a8ea1`](https://github.com/GitoxideLabs/gitoxide/commit/a9a8ea1472532dde03bce4e0afdfa82924af1f96))
    - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
    - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.42.1 (2025-04-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.1, gix-utils v0.3.0, gix-actor v0.35.1, gix-validate v0.10.0, gix-path v0.10.17, gix-features v0.42.1, gix-hash v0.18.0, gix-hashtable v0.8.1, gix-object v0.49.1, gix-glob v0.20.0, gix-quote v0.6.0, gix-attributes v0.26.0, gix-command v0.6.0, gix-packetline-blocking v0.19.0, gix-filter v0.19.1, gix-fs v0.15.0, gix-commitgraph v0.28.0, gix-revwalk v0.20.1, gix-traverse v0.46.1, gix-worktree-stream v0.21.1, gix-archive v0.21.1, gix-tempfile v17.1.0, gix-lock v17.1.0, gix-index v0.40.0, gix-config-value v0.15.0, gix-pathspec v0.11.0, gix-ignore v0.15.0, gix-worktree v0.41.0, gix-diff v0.52.1, gix-blame v0.2.1, gix-ref v0.52.1, gix-sec v0.11.0, gix-config v0.45.1, gix-prompt v0.11.0, gix-url v0.31.0, gix-credentials v0.29.0, gix-discover v0.40.1, gix-dir v0.14.1, gix-mailmap v0.27.1, gix-revision v0.34.1, gix-merge v0.5.1, gix-negotiate v0.20.1, gix-pack v0.59.1, gix-odb v0.69.1, gix-refspec v0.30.1, gix-shallow v0.4.0, gix-packetline v0.19.0, gix-transport v0.47.0, gix-protocol v0.50.1, gix-status v0.19.1, gix-submodule v0.19.1, gix-worktree-state v0.19.0, gix v0.72.1, gix-fsck v0.11.1, gitoxide-core v0.47.1, gitoxide v0.44.0 ([`e104545`](https://github.com/GitoxideLabs/gitoxide/commit/e104545b78951ca882481d4a58f4425a8bc81c87))
    - Bump all prior pratch levels to majors ([`5f7f805`](https://github.com/GitoxideLabs/gitoxide/commit/5f7f80570e1a5522e76ea58cccbb957249a0dffe))
    - Merge pull request #1969 from GitoxideLabs/new-release ([`631f07a`](https://github.com/GitoxideLabs/gitoxide/commit/631f07ad0c1cb93d9da42cf2c8499584fe91880a))
</details>

## 0.42.0 (2025-04-25)

### New Features (BREAKING)

 - <csr-id-788290717da8ec396635aec386af99e1917bf3a0/> `gix-features` uses and republishes `bytesize` version 2
   `gix-features` republishes `bytesize`, which has been bumped from
   major version 1 to major version 2. Because the interface of
   `bytesize` is effectively part of that of `gix-features` due to
   the explicit republication of the `bytesize` module in full (with
   no documented extra limitations related to interface stability),
   this is effectively a breaking change in `gix-features` as well,
   though many callers may not be substantially affected.
   
   Major changes that may affect some callers that use `bytesize`
   through `gix-features` include the removal of the top-level
   `bytesize::to_string()` function, the change in default behavior
   from displaying decimal SI units to displaying binary IEC units
   (though all or most gitoxide-related creates, in adapting to the
   changes, have thus far opted to continue using decimal SI units),
   and the small but UI-facing change that decimal SI kilobytes
   (units of 1000 bytes) are given with the symbol "kB" rather than
   the more ambiguous (and arguably less accurate) symbol "KB".
   
   In addition to republishing `bytesize`, `gix-features` also
   republishes `prodash`. Futhermore, some uses of `bytesize` are
   transitively through `prodash`, which itself has recently received
   an update to use `bytesize` major version 2. (Since `prodash` does
   not republish `bytesize`, that is not considered to be a breaking
   change in `prodash` itself.) To get the benefits of the newer
   version of `bytesize` while avoiding new inconsistencies, and also
   to avoid depending on multiple versions of `bytesize`, the
   `prodash` dependency version has also been upgraded.
   
   For more information, see:
   
   - https://github.com/GitoxideLabs/gitoxide/discussions/1952

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
    - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
    - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
    - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Thanks clippy ([`6f009d7`](https://github.com/GitoxideLabs/gitoxide/commit/6f009d781da9e931d44b113a925a80e77e8788af))
    - Merge pull request #1968 from GitoxideLabs/dependabot/cargo/cargo-bd18780e40 ([`46227e6`](https://github.com/GitoxideLabs/gitoxide/commit/46227e6d1ddc0879662730e5bb21a8597716b1ca))
    - Bump the cargo group with 40 updates ([`06bf1e1`](https://github.com/GitoxideLabs/gitoxide/commit/06bf1e1552de65ce692911bdc4c501d487bbc3d7))
    - Merge pull request #1963 from joshtriplett/zlib-rs-default ([`9e075b9`](https://github.com/GitoxideLabs/gitoxide/commit/9e075b99ffc79173d4052d7550fd1d2826c5ec71))
    - Switch to zlib-rs by default and drop other zlib backends ([`96164c5`](https://github.com/GitoxideLabs/gitoxide/commit/96164c5936032b4edb973828178cc55793dd57cc))
    - Merge pull request #1960 from joshtriplett/zlib-rs ([`f3684a4`](https://github.com/GitoxideLabs/gitoxide/commit/f3684a4f67f98c0e3884e2d348cb092f48ca443a))
    - Fix documentation of zlib-rs feature: no longer conflicts with other zlibs ([`8ece02a`](https://github.com/GitoxideLabs/gitoxide/commit/8ece02aeda4b3044dd68dd8a6c20ea4ee4ed1bda))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - `gix-features` uses and republishes `bytesize` version 2 ([`7882907`](https://github.com/GitoxideLabs/gitoxide/commit/788290717da8ec396635aec386af99e1917bf3a0))
    - Merge pull request #1953 from GitoxideLabs/dependabot/cargo/cargo-4a3cda0de8 ([`3aec7fb`](https://github.com/GitoxideLabs/gitoxide/commit/3aec7fbac52377bdeebc49759d4e0420b18b4e81))
    - Bump the cargo group with 3 updates ([`9f1fbc7`](https://github.com/GitoxideLabs/gitoxide/commit/9f1fbc741e1b6c718c7787f2858e07f3bd5473e9))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1944 from GitoxideLabs/dependabot/cargo/cargo-ea4584273b ([`79dabb0`](https://github.com/GitoxideLabs/gitoxide/commit/79dabb0a062a795b1dc5c9c840d67474dc6b343b))
    - Bump crossbeam-channel from 0.5.14 to 0.5.15 in the cargo group ([`1792178`](https://github.com/GitoxideLabs/gitoxide/commit/1792178cbed5abf4782370bc46207a8c5d113615))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
</details>

## 0.41.1 (2025-04-06)

### Bug Fixes

 - <csr-id-38b63c2fc9d407b3c634d8b0c72d4d0c104aa5ad/> make `fs::walkdir_sorted_new()` sort entries by paths literally
   This follows up 7b1b5bf864e74706aefeb1213e8bdb0545d5464a. Since packed-refs
   appears to be sorted by full ref name, loose-refs should also be emitted in
   that order.
   
   The comparison function is copied from gix::diff::object::tree::EntryRef.
   Non-utf8 file names are simply mapped to "" on Windows. We could add some
   fallback, but callers can't handle such file names anyway.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1928](https://github.com/GitoxideLabs/gitoxide/issues/1928)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1928](https://github.com/GitoxideLabs/gitoxide/issues/1928)**
    - Make `fs::walkdir_sorted_new()` sort entries by paths literally ([`38b63c2`](https://github.com/GitoxideLabs/gitoxide/commit/38b63c2fc9d407b3c634d8b0c72d4d0c104aa5ad))
 * **Uncategorized**
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1931 from yuja/push-klrqpplwxrkx ([`7502b4a`](https://github.com/GitoxideLabs/gitoxide/commit/7502b4abde6196b982cf66344c0df992e99493cb))
    - Refactor ([`7e6e751`](https://github.com/GitoxideLabs/gitoxide/commit/7e6e7518c78ba925382c0d4654b9e11b4cd641e9))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.41.0 (2025-04-04)

### Changed (BREAKING)

 - <csr-id-fd12ef89af29bf0684fc1df3e7b76ff367dee994/> drop obsolete SHA‐1 features
   The hashing API has moved to `gix_hash::hasher`, and we now use
   `sha1-checked` unconditionally.
 - <csr-id-e4439aa9a6969e14b7a03bea6b0b771534510edd/> move hashing API to `gix_hash`

### New Features (BREAKING)

 - <csr-id-f253f02a6658b3b7612a50d56c71f5ae4da4ca21/> detect SHA‐1 collision attacks
   Fix [GHSA-2frx-2596-x5r6].

### Bug Fixes (BREAKING)

 - <csr-id-7b1b5bf864e74706aefeb1213e8bdb0545d5464a/> `fs::walkdir_sorted_new` now returns files first.
   It's only used by `gix_ref` for reference traversal, which now needs this
   to be sorted differently to work correctly.
   
   As this isn't easily possible in the parallel walk implementation, it's hereby removed,
   along with the `fs-walkdir-parallel` feature toggle.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
    - Update changelogs prior to release ([`38dff41`](https://github.com/GitoxideLabs/gitoxide/commit/38dff41d09b6841ff52435464e77cd012dce7645))
    - Merge pull request #1915 from emilazy/push-qvyqmopsoltr ([`4660f7a`](https://github.com/GitoxideLabs/gitoxide/commit/4660f7a6f71873311f68f170b0f1f6659a02829d))
    - Refactor ([`4501086`](https://github.com/GitoxideLabs/gitoxide/commit/4501086adc544e675b3043c4c23b78a6c6711d8b))
    - Drop obsolete SHA‐1 features ([`fd12ef8`](https://github.com/GitoxideLabs/gitoxide/commit/fd12ef89af29bf0684fc1df3e7b76ff367dee994))
    - Move hashing API to `gix_hash` ([`e4439aa`](https://github.com/GitoxideLabs/gitoxide/commit/e4439aa9a6969e14b7a03bea6b0b771534510edd))
    - Detect SHA‐1 collision attacks ([`f253f02`](https://github.com/GitoxideLabs/gitoxide/commit/f253f02a6658b3b7612a50d56c71f5ae4da4ca21))
    - Merge pull request #1851 from GitoxideLabs/fix-1850 ([`cd96b64`](https://github.com/GitoxideLabs/gitoxide/commit/cd96b6439d119c5189a8e7349d2e7e2533db41b5))
    - `fs::walkdir_sorted_new` now returns files first. ([`7b1b5bf`](https://github.com/GitoxideLabs/gitoxide/commit/7b1b5bf864e74706aefeb1213e8bdb0545d5464a))
    - Merge pull request #1873 from NobodyXu/zlib-rs ([`316f113`](https://github.com/GitoxideLabs/gitoxide/commit/316f11322f156760a0e344a3bda33e11ca4e8862))
    - Review adjustments for zlib-rs support. ([`5e618b6`](https://github.com/GitoxideLabs/gitoxide/commit/5e618b6e7632a037326d759678bef452b32a3b30))
    - Add new feature zlib-rs ([`8b1b55c`](https://github.com/GitoxideLabs/gitoxide/commit/8b1b55c337e65071156856771daee3cbcead1e24))
    - Merge pull request #1882 from emilazy/push-ylwwuwymlmwt ([`10e41ee`](https://github.com/GitoxideLabs/gitoxide/commit/10e41ee6d1d3607c3d26a66b488d7d1eabc45c6e))
    - Fix cargo-deny using a prodash-update and ignore directive ([`cf7f34d`](https://github.com/GitoxideLabs/gitoxide/commit/cf7f34dcd653ddafaaecb149d4b98efa97d5b871))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.40.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 55 calendar days.
 - 55 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.39.1 (2024-11-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.39.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>

### Other

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

### Bug Fixes (BREAKING)

 - <csr-id-cdf2a6320e208bac69b9b1152c10f0b81b238c3e/> rename `hash::Sha1` to `hash::Hasher` and `Sha1Digest` to `Digest`.
   That way, one day we can turn this type into a compatible one which produce
   different kinds of hashes as well.
 - <csr-id-17573779688e755a786546d5e42ab533088cd726/> remove all workspace dependencies
   The problem is that with them, we don't notice anymore if the crate changes,
   because a dependency changes. That also means that older versions of the dependency
   may stay even though some other crates might pick up a newer version.
   
   Ultimately, this will lead to drift and subtle incompatibilities.
   
   We declare this breaking to enforce a proper re-release.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 24 commits contributed to the release over the course of 123 calendar days.
 - 153 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - Rename `hash::Sha1` to `hash::Hasher` and `Sha1Digest` to `Digest`. ([`cdf2a63`](https://github.com/GitoxideLabs/gitoxide/commit/cdf2a6320e208bac69b9b1152c10f0b81b238c3e))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - A few more missing semicolons ([`fc45c93`](https://github.com/GitoxideLabs/gitoxide/commit/fc45c931c132ac8b6ea4f2e4c3d5f0d19727f46f))
    - Lint fix, just file adjustment ([`7b6e176`](https://github.com/GitoxideLabs/gitoxide/commit/7b6e1764dc3b1dad30738fcb16b7428d880d4203))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge branch 'patch-2' ([`a64d94e`](https://github.com/GitoxideLabs/gitoxide/commit/a64d94e9b944f16b87d572e887bde5dec091d17d))
    - Update `flate2` to a version which hopefully runs on Windows by default. ([`77e808b`](https://github.com/GitoxideLabs/gitoxide/commit/77e808bbce4177bf78df68aec0a8e906d8c7d1cf))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Merge branch 'fixes' ([`46cd1ae`](https://github.com/GitoxideLabs/gitoxide/commit/46cd1aed7815d27cdc818edb87641b20b82ba048))
    - Remove all workspace dependencies ([`1757377`](https://github.com/GitoxideLabs/gitoxide/commit/17573779688e755a786546d5e42ab533088cd726))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
</details>

## 0.38.2 (2024-05-22)

### Bug Fixes

 - <csr-id-268323587faeada1abdd7f933d616af3165a37cf/> assure high-speed SHA1 assembly is only used in not on Windows

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 1 calendar day.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#917](https://github.com/GitoxideLabs/gitoxide/issues/917)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#917](https://github.com/GitoxideLabs/gitoxide/issues/917)**
    - Assure high-speed SHA1 assembly is only used in not on Windows ([`2683235`](https://github.com/GitoxideLabs/gitoxide/commit/268323587faeada1abdd7f933d616af3165a37cf))
 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Merge branch 'various-fixes' ([`d6cd449`](https://github.com/GitoxideLabs/gitoxide/commit/d6cd44930fb204b06e2b70fc6965e7705530c47a))
    - Merge pull request from GHSA-7w47-3wg8-547c ([`79dce79`](https://github.com/GitoxideLabs/gitoxide/commit/79dce79c62f6072aa2653780d590dc3993dfa401))
</details>

## 0.38.1 (2024-03-14)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 4 calendar days.
 - 54 days passed between releases.
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

## 0.38.0 (2024-01-20)

### New Features

 - <csr-id-24d081a0861cca1f3fa657297ad58a937b2d903d/> add `fs::read_dir::DirEntry` with precompose_unicode support.
   It's available with the `fs-read-dir` feature toggle.

### New Features (BREAKING)

 - <csr-id-47e2fec0db6c55d673c953d26f01fd88ac0f118e/> Support `precompose_unicode` flag in `fs::walkdir` types.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 2 calendar days.
 - 4 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Merge branch 'dirwalk' ([`5d176fc`](https://github.com/GitoxideLabs/gitoxide/commit/5d176fc5ab82bfc7c194b4d929e73da9659ae8b8))
    - Add `fs::read_dir::DirEntry` with precompose_unicode support. ([`24d081a`](https://github.com/GitoxideLabs/gitoxide/commit/24d081a0861cca1f3fa657297ad58a937b2d903d))
    - Support `precompose_unicode` flag in `fs::walkdir` types. ([`47e2fec`](https://github.com/GitoxideLabs/gitoxide/commit/47e2fec0db6c55d673c953d26f01fd88ac0f118e))
</details>

## 0.37.2 (2024-01-15)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 16 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
    - Prepare changelogs prior to `gix-index` release ([`17d1aac`](https://github.com/GitoxideLabs/gitoxide/commit/17d1aac91ad22291ad6d72f6e8798ebb741a8d7d))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
</details>

## 0.37.1 (2023-12-30)

<csr-id-3bd09ef120945a9669321ea856db4079a5dab930/>

### Chore

- <csr-id-3bd09ef120945a9669321ea856db4079a5dab930/> change `rust-version` manifest field back to 1.65.
  They didn't actually need to be higher to work, and changing them
  unecessarily can break downstream CI.

  Let's keep this value as low as possible, and only increase it when
  more recent features are actually used.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
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

## 0.37.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 9 calendar days.
 - 22 days passed between releases.
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
    - Merge branch 'fuzz-gix-ref' ([`8ee1194`](https://github.com/GitoxideLabs/gitoxide/commit/8ee119442b2ff4b846a34b57a222a6abdf6c95d8))
    - Fix ununsed imports warning on nightly ([`f130f63`](https://github.com/GitoxideLabs/gitoxide/commit/f130f631593f73eb00cf6dc132ee147ad8b5e970))
</details>

## 0.36.1 (2023-12-06)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - Fix formatting ([`5d8b5f4`](https://github.com/GitoxideLabs/gitoxide/commit/5d8b5f4e5b3792b310f7a23907a205b5db427420))
    - Merge pull request #1134 from alexanderkjall/gix-features-i386-test-failure ([`a12682d`](https://github.com/GitoxideLabs/gitoxide/commit/a12682d7a8bd263a18471f4a26d8af67b1cd4c1e))
    - Review feedback, use an 'if cfg' construct instead ([`1d45f05`](https://github.com/GitoxideLabs/gitoxide/commit/1d45f056263ba9c69612abc81fdd3783f95de4ca))
    - Test size_of_sha1 reports wrong size on i386 ([`5c2d615`](https://github.com/GitoxideLabs/gitoxide/commit/5c2d615548dd27f23586e4fe5cfa8c6507635694))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge pull request #1115 from martinvonz/push-vuzxvuvlztxy ([`c65b80b`](https://github.com/GitoxideLabs/gitoxide/commit/c65b80b048769bc999be5f3002c4d708da550d3c))
    - Fix pointer aliasing in in_parallel.rs ([`1383b0d`](https://github.com/GitoxideLabs/gitoxide/commit/1383b0df14134b0e0f1a2fbfccb4526130c84a03))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
    - Assure all crates have includes configured ([`065ab57`](https://github.com/GitoxideLabs/gitoxide/commit/065ab57d890f4b98cca7a7f81d68876fa84f49e0))
</details>

## 0.36.0 (2023-10-12)

### New Features (BREAKING)

 - <csr-id-4c03fdbc831349ea7f45b68331f554ade859abf5/> add `hash::bytes_with_header()`, also make it 32bit compatible.
   That way it's possible to hash entire files as object.
   Previously it wasn't possible to read more than u32::MAX bytes even
   on 32 bit system even though we are streaming the data.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 13 calendar days.
 - 17 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Add `hash::bytes_with_header()`, also make it 32bit compatible. ([`4c03fdb`](https://github.com/GitoxideLabs/gitoxide/commit/4c03fdbc831349ea7f45b68331f554ade859abf5))
</details>

## 0.35.0 (2023-09-24)

### New Features

 - <csr-id-51971969d2cf13587d4bfbd4cb047f2377b8bc0f/> Add `threading::make_mut()` to allow obtaining a mutable reference to shared data.
   This is particularly useful when handling an index file, which may be shared across clones of
   a repository.

### Bug Fixes

 - <csr-id-7a8f79357ce99c4b86e22be166b54f7376c71469/> cargo-auditable build error
   Use `prodash` instead of `dep:prodash` in gix-features and `tracing`
   instead of `dep:tracing` in gitoxide-core.
   
   The `dep:mydep` syntax removes the implicit `mydep` feature for optional
   dependencies, this triggers a bug in cargo that affects
   `cargo-auditable`. See https://github.com/rust-lang/cargo/issues/12336
   
   This affects some Linux distributions like NixOS which use
   `cargo-auditable` by default. Related issues:
   
   - https://github.com/NixOS/nixpkgs/issues/253911

### Bug Fixes (BREAKING)

 - <csr-id-70c7c29266bc4396c968c7aa311c9721929a7cab/> parallel utilities that create thread-state now use `FnOnce`.
   This way, all unnecessary cloning is avoided.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 16 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
    - Merge branch 'reset' ([`54a8495`](https://github.com/GitoxideLabs/gitoxide/commit/54a849545140f7f1c0c7564c418071c0a76a34e7))
    - Add `threading::make_mut()` to allow obtaining a mutable reference to shared data. ([`5197196`](https://github.com/GitoxideLabs/gitoxide/commit/51971969d2cf13587d4bfbd4cb047f2377b8bc0f))
    - Parallel utilities that create thread-state now use `FnOnce`. ([`70c7c29`](https://github.com/GitoxideLabs/gitoxide/commit/70c7c29266bc4396c968c7aa311c9721929a7cab))
    - Merge pull request #1024 from Byron/nix-adjustments ([`14e0763`](https://github.com/GitoxideLabs/gitoxide/commit/14e0763cbf368bda476046a0fd28be230d67b1bd))
    - Cargo-auditable build error ([`7a8f793`](https://github.com/GitoxideLabs/gitoxide/commit/7a8f79357ce99c4b86e22be166b54f7376c71469))
    - Merge branch 'optimize/progress-use' ([`1f2ffb6`](https://github.com/GitoxideLabs/gitoxide/commit/1f2ffb6d86ef073caf43a2f7a77fe712a1aa495e))
    - Use trait object for `progress` in `PrepareFetch::fetch_only` ([`70989b3`](https://github.com/GitoxideLabs/gitoxide/commit/70989b3965077ae00ec6cf344f31627a804a8225))
</details>

## 0.34.0 (2023-09-08)

<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

### Chore (BREAKING)

- <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
  It makes proper usage of `Progress` types easier and allows them to be used
  as `dyn` traits as well.

### New Features (BREAKING)

 - <csr-id-24dd870919ba444aa8099c63a78ea120d47ec28e/> use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress`

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress` ([`24dd870`](https://github.com/GitoxideLabs/gitoxide/commit/24dd870919ba444aa8099c63a78ea120d47ec28e))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.33.0 (2023-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 30 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
</details>

## 0.32.1 (2023-07-22)

### New Features

 - <csr-id-717950977fa758812bc4dd5713f96995bddc491a/> add `interrupt::Write` to auto-fail writes on interrupt.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Prepare more changelogs ([`c4cc5f2`](https://github.com/GitoxideLabs/gitoxide/commit/c4cc5f261d29f712a101033a18293a97a9d4ae85))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Merge branch 'gix-archive' ([`1dda48b`](https://github.com/GitoxideLabs/gitoxide/commit/1dda48ba2fccb93ebac00fe3460e923af43c86ce))
    - Add `interrupt::Write` to auto-fail writes on interrupt. ([`7179509`](https://github.com/GitoxideLabs/gitoxide/commit/717950977fa758812bc4dd5713f96995bddc491a))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.32.0 (2023-07-19)

### New Features

 - <csr-id-2a76929ece48c3b0fbdba7e08ae5d7e647db4d36/> add `in_parallel_with_finalize` to support a usecase for `gix-worktree`
 - <csr-id-717950977fa758812bc4dd5713f96995bddc491a/> add `interrupt::Write` to auto-fail writes on interrupt.

### Changed (BREAKING)

 - <csr-id-032cea3d6cfde80fdfc7bfe8722e2514d2bcb5cf/> remove `zlib::stream::inflate::ReadBoxed`.
   It was a special-case type that was only used in one spot, and thus
   not really that interesting to have in a shared place.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 10 calendar days.
 - 19 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
    - Merge branch 'refactor-pack-streaming' ([`8a46a7e`](https://github.com/GitoxideLabs/gitoxide/commit/8a46a7ef3efc22241b67f3447223b2505e205442))
    - Remove `zlib::stream::inflate::ReadBoxed`. ([`032cea3`](https://github.com/GitoxideLabs/gitoxide/commit/032cea3d6cfde80fdfc7bfe8722e2514d2bcb5cf))
    - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
    - Add `in_parallel_with_finalize` to support a usecase for `gix-worktree` ([`2a76929`](https://github.com/GitoxideLabs/gitoxide/commit/2a76929ece48c3b0fbdba7e08ae5d7e647db4d36))
</details>

## 0.31.1 (2023-06-29)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 6 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.31.1, gix-path v0.8.3, gix v0.48.0 ([`9ca3464`](https://github.com/GitoxideLabs/gitoxide/commit/9ca346462806671fbc49643a87cea25ab0da3be8))
    - Prepare changelogs once more ([`4bf355a`](https://github.com/GitoxideLabs/gitoxide/commit/4bf355a8c6a7dbcdb49105af3208d56a0ed8628d))
    - Adjust `gix-trace` to the latest version. ([`353df4b`](https://github.com/GitoxideLabs/gitoxide/commit/353df4bf59c7aa98da48bcdcc299f947d9449f55))
</details>

## 0.31.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### New Features

 - <csr-id-093efafa7c39aa03bfef4894779cca6e3716f471/> add `tracing` feature toggle to provide minimal tracing API
   This API is based on `tracing-core`, not on tracing, and provides a limited
   API that is always available, while being a no-op if `tracing` isn't enabled.
   
   That way, plumbing crates can add instrumentation at will.

### Bug Fixes (BREAKING)

 - <csr-id-bc69804d5f6cba56349b5b15a2f6d1741849a5eb/> `new_thread_state()` functions are now `FnOnce` across the board.
   This properly communicates that they will only be called once per thread,
   while providing more options to the implementor.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 10 calendar days.
 - 15 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Add `tracing` feature toggle to provide minimal tracing API ([`093efaf`](https://github.com/GitoxideLabs/gitoxide/commit/093efafa7c39aa03bfef4894779cca6e3716f471))
    - `new_thread_state()` functions are now `FnOnce` across the board. ([`bc69804`](https://github.com/GitoxideLabs/gitoxide/commit/bc69804d5f6cba56349b5b15a2f6d1741849a5eb))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
</details>

## 0.30.0 (2023-06-06)

### New Features

 - <csr-id-d81b7e3036371d0a2a22f02a92df3c2b2950ff22/> add `zlib::Inflate::reset()`.
   That way, each instance can be reused.
 - <csr-id-add5ea8b83d00972b560536da82f9914ef6080d3/> make `prodash::RawProgress` available.
   It's an object-safe version of the `Progress` trait.

### New Features (BREAKING)

 - <csr-id-0fa04bcbdf3102c5435e64cfef894a1bfc8d6e7b/> make current thread-count accessible in slice-workers.
   Threads started for working on an entry in a slice can now see the amount
   of threads left for use (and manipulate that variable) which effectively
   allows them to implement their own parallelization on top of the current one.
   
   This is useful if there is there is very imbalanced work within the slice itself.
   
   While at it, we also make consumer functions mutable as they exsit per thread.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 22 calendar days.
 - 48 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#851](https://github.com/GitoxideLabs/gitoxide/issues/851)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#851](https://github.com/GitoxideLabs/gitoxide/issues/851)**
    - Make current thread-count accessible in slice-workers. ([`0fa04bc`](https://github.com/GitoxideLabs/gitoxide/commit/0fa04bcbdf3102c5435e64cfef894a1bfc8d6e7b))
 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Thanks clippy ([`9525ac8`](https://github.com/GitoxideLabs/gitoxide/commit/9525ac822aa902f5325f17e7b08ffb60b683e0e7))
    - Merge branch 'fix-alloc' ([`d9d9bc0`](https://github.com/GitoxideLabs/gitoxide/commit/d9d9bc01b34ac75b28a5f1b75f40123aa6d83c60))
    - Add `zlib::Inflate::reset()`. ([`d81b7e3`](https://github.com/GitoxideLabs/gitoxide/commit/d81b7e3036371d0a2a22f02a92df3c2b2950ff22))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Auto-fix clippy to remove explicit iter looping ([`3eff567`](https://github.com/GitoxideLabs/gitoxide/commit/3eff567c683b5c650c14792b68968cbdbc90ec5c))
    - Merge pull request #865 from nyurik/fix-tests ([`3a45973`](https://github.com/GitoxideLabs/gitoxide/commit/3a45973c61c4816940b1c9bae4fe60af47a3f6c6))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
    - Make cargo check to pass all tests ([`f4a44f7`](https://github.com/GitoxideLabs/gitoxide/commit/f4a44f71e894dd8a08dd898a099b743a3ccff557))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Merge branch 'fix-851' ([`2f275d5`](https://github.com/GitoxideLabs/gitoxide/commit/2f275d5d3cb49b3b8ba53b30e4b4386fac32662b))
    - Make `prodash::RawProgress` available. ([`add5ea8`](https://github.com/GitoxideLabs/gitoxide/commit/add5ea8b83d00972b560536da82f9914ef6080d3))
</details>

## 0.29.0 (2023-04-19)

### New Features (BREAKING)

 - <csr-id-b645d28f9641c6b4022e1e37ad9fe528922ec747/> remove types that are now available in `gix-os`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 6 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Prepare changelog prior to release ([`7f06458`](https://github.com/GitoxideLabs/gitoxide/commit/7f064583bd0e1b078df89a7750f5a25deb70f516))
    - Support native zlib-ng via flate2's zlib-ng feature ([`9a6e0d7`](https://github.com/GitoxideLabs/gitoxide/commit/9a6e0d7b418ea721da6a7e4bc48c47b47d4dfa79))
    - Merge branch 'main' into dev ([`23ee47f`](https://github.com/GitoxideLabs/gitoxide/commit/23ee47fb24c197f8437bd426544b2aa74e005bdc))
    - Merge branch 'worktree-stack' ([`3d47919`](https://github.com/GitoxideLabs/gitoxide/commit/3d47919c1a2f83fc7c1fd7ae590d098057a22626))
    - Remove types that are now available in `gix-os` ([`b645d28`](https://github.com/GitoxideLabs/gitoxide/commit/b645d28f9641c6b4022e1e37ad9fe528922ec747))
    - Use existing concurrency primitive in_parallel ([`c5f3fc8`](https://github.com/GitoxideLabs/gitoxide/commit/c5f3fc8b5875745eb50bd80005b43a66cf255acb))
    - Parallel status check ([`d7f250d`](https://github.com/GitoxideLabs/gitoxide/commit/d7f250ddbd53a994a17db41f86cc780b45e9ee5a))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
</details>

## 0.28.1 (2023-03-14)

A maintenance release without any user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 1 calendar day.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.28.1, gix-tempfile v5.0.1, gix-ref v0.27.1, gix-pack v0.33.1, gix-packetline v0.15.0, gix-transport v0.29.0, gix-protocol v0.30.0, gix v0.42.0, safety bump 3 crates ([`c1f1bfb`](https://github.com/GitoxideLabs/gitoxide/commit/c1f1bfb8dc0e73993678353e4492d0614b642ed1))
    - Prepare changelogs prior to release ([`c66e298`](https://github.com/GitoxideLabs/gitoxide/commit/c66e2982577e4cd9faef63798986b8cf8ece93a2))
    - Make fmt ([`3836cc0`](https://github.com/GitoxideLabs/gitoxide/commit/3836cc0c9c3e1158b56142b924483c8a77217d53))
</details>

## 0.28.0 (2023-03-04)

### New Features (BREAKING)

 - <csr-id-571ec0d7c3e1eb167d55daa6551bd2b27d3c5b25/> use `std::thread::scope()` instead of `crossbeam-utils::thread::scope()`.
   This cuts a direct dependency.
   We can't removed `crossbeam-channel` yet due to the need for single-produce-multiple-consumer
   channels.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.28.0, gix-actor v0.19.0, gix-object v0.28.0, gix-diff v0.28.0, gix-traverse v0.24.0, gix-pack v0.32.0, safety bump 20 crates ([`0f411e9`](https://github.com/GitoxideLabs/gitoxide/commit/0f411e93ec812592bb9d3a52b751399dd86f76f7))
    - Prepare changelogs prior to release of `gix-pack` ([`6db30ef`](https://github.com/GitoxideLabs/gitoxide/commit/6db30ef6b5e931bbf12135507a3d922051de4d4b))
    - Merge branch 'adjustments-for-cargo' ([`04ab852`](https://github.com/GitoxideLabs/gitoxide/commit/04ab852f3be76bdf151affa25cf4b999b127bdfe))
    - Use `std::thread::scope()` instead of `crossbeam-utils::thread::scope()`. ([`571ec0d`](https://github.com/GitoxideLabs/gitoxide/commit/571ec0d7c3e1eb167d55daa6551bd2b27d3c5b25))
</details>

## 0.27.0 (2023-03-01)

<csr-id-cce96ee1382d3d56d77820a2aba6e2d17b52f91c/>

### Chore

- <csr-id-cce96ee1382d3d56d77820a2aba6e2d17b52f91c/> replace `quick-error` with `thiserror`
  This increases the compile time of the crate alone if there is no proc-macro
  in the dependency tree, but will ever so slightly improve compile times for `gix`
  as a whole.

### New Features

 - <csr-id-7a442313c57f58fec5217484f268516711c3d52b/> make `bytesize` available in `progress`.
   Note that it is stubbed out unless the `progress-unit-bytes` feature is set.
 - <csr-id-7f6a807ea506358b6dc4fd0c7a648770f1dc91e9/> add `progress::count_with_decimals()` and `progress-unit-*` feature toggles.
   The new feature toggles allow controlling the inclusion of the `bytesize` and `human_format`
   crates, which can be toggled with the `progress-unit-bytes` and `progress-unit-human-numbers` respectively.
   
   Without these features, the respective functions exist but don't provide special formatting, making bytes and
   larger numbers harder to read.

### Bug Fixes

 - <csr-id-264f78a508d3030cb346623f57ac75add7682169/> remove `num_cpus` in favor of `std::thread::available_parallelism()`.
   `num_cpus` was needed back in the days when `std` didn't support such
   functionality yet.

### Changed (BREAKING)

 - <csr-id-0cc548041a861be21001462cbe0ef29b1d61d1c4/> use new `dep:` syntax to hide optional features from feature-set.
   That way, the set of features is clearly defined and controlled.
   This migth break applications who relied on direect access to features named
   after optional dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 3 calendar days.
 - 8 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
    - Merge branch 'adjustments-for-cargo' ([`d686d94`](https://github.com/GitoxideLabs/gitoxide/commit/d686d94e1030a8591ba074757d56927a346c8351))
    - Depend on latest version of `prodash` for performance improvements. ([`5d00324`](https://github.com/GitoxideLabs/gitoxide/commit/5d003242abe82b1604e2188d49dec9690ebb2a6a))
    - Make `bytesize` available in `progress`. ([`7a44231`](https://github.com/GitoxideLabs/gitoxide/commit/7a442313c57f58fec5217484f268516711c3d52b))
    - Add `progress::count_with_decimals()` and `progress-unit-*` feature toggles. ([`7f6a807`](https://github.com/GitoxideLabs/gitoxide/commit/7f6a807ea506358b6dc4fd0c7a648770f1dc91e9))
    - Use new `dep:` syntax to hide optional features from feature-set. ([`0cc5480`](https://github.com/GitoxideLabs/gitoxide/commit/0cc548041a861be21001462cbe0ef29b1d61d1c4))
    - Remove `num_cpus` in favor of `std::thread::available_parallelism()`. ([`264f78a`](https://github.com/GitoxideLabs/gitoxide/commit/264f78a508d3030cb346623f57ac75add7682169))
    - Replace `quick-error` with `thiserror` ([`cce96ee`](https://github.com/GitoxideLabs/gitoxide/commit/cce96ee1382d3d56d77820a2aba6e2d17b52f91c))
    - Make fmt ([`8ef1cb2`](https://github.com/GitoxideLabs/gitoxide/commit/8ef1cb293434c7b9e1fda4a6963368e0435920a9))
</details>

## 0.26.5 (2023-02-20)

### Bug Fixes

 - <csr-id-e14dc7d475373d2c266e84ff8f1826c68a34ab92/> note that crates have been renamed from `git-*` to `gix-*`.
   This also means that the `git-*` prefixed crates of the `gitoxide` project
   are effectively unmaintained.
   Use the crates with the `gix-*` prefix instead.
   
   If you were using `git-repository`, then `gix` is its substitute.
 - <csr-id-135d317065aae87af302beb6c26bb6ca8e30b6aa/> compatibility with `bstr` v1.3, use `*.as_bytes()` instead of `.as_ref()`.
   `as_ref()` relies on a known target type which isn't always present. However, once
   there is only one implementation, that's no problem, but when that changes compilation
   fails due to ambiguity.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.4.3, gix-hash v0.10.3, gix-features v0.26.5, gix-actor v0.17.2, gix-glob v0.5.5, gix-path v0.7.2, gix-quote v0.4.2, gix-attributes v0.8.3, gix-validate v0.7.3, gix-object v0.26.2, gix-ref v0.24.1, gix-config v0.16.2, gix-command v0.2.4, gix-url v0.13.3, gix-credentials v0.9.2, gix-discover v0.13.1, gix-index v0.12.4, gix-mailmap v0.9.3, gix-pack v0.30.3, gix-packetline v0.14.3, gix-transport v0.25.6, gix-protocol v0.26.4, gix-revision v0.10.4, gix-refspec v0.7.3, gix-worktree v0.12.3, gix v0.36.1 ([`9604783`](https://github.com/GitoxideLabs/gitoxide/commit/96047839a20a657a559376b0b14c65aeab96acbd))
    - Compatibility with `bstr` v1.3, use `*.as_bytes()` instead of `.as_ref()`. ([`135d317`](https://github.com/GitoxideLabs/gitoxide/commit/135d317065aae87af302beb6c26bb6ca8e30b6aa))
</details>

## 0.26.4 (2023-02-17)

<csr-id-5bf0034fb3918e57562b7089ceba83d63a1854bf/>
<csr-id-ab6f98b905f13ed2a7c0c483f34fab63141fbc5b/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/>

### New Features (BREAKING)

 - <csr-id-0f27c67c92fc0bc23a6712b5c4c730ad6a0156bf/> add support for explicit non-parallel iteration.
   That way we can allow the implementation to choose whether they
   need greatest speed at some cost or not.
   
   This also allows us to create a new thread-pool on each iteration
   as those who expect high cost or many files will likely chose to do
   that instead of single-threaded iteration, which nicely contains the
   threads needed and avoids keeping them alive as part of some global pool.
 - <csr-id-3b29fc18672c0176684c797a0f16f85d09369bf8/> make jwalk fully optional
 - <csr-id-d078d6ee76a80d1dfaf71608c12d8a402bd670d4/> mild refactor of paths module to waste less on unix
   Previously it might have performed find-and-replace on unix paths even
   though they wouldn't have changed afterwards, yet costing an allocation.
   
   There is also the realization that it should go into its own crate to have
   neater import paths and more convenience.
 - <csr-id-8945d95f7fa88562d37ff67ac6e38bead73dd2df/> `interrupt::Iter`, rename `interrupt::Iter` -> `interrupt::IterWithError`

### Chore (BREAKING)

- <csr-id-5bf0034fb3918e57562b7089ceba83d63a1854bf/> upgrade to prodash v23

### Changed (BREAKING)

 - <csr-id-38446dc8824afef30ef121598de3451d13b9262c/> remove `fs-jwalk-single-threaded` in favor of `fs-walkdir-parallel`.
   This way, `jwalk` and the dependencies (and troubles) it brings have to
   be opted in, but also allow other users to actually opt out while
   allowing the `parallel` feature to be in effect.
   
   In other words, previously the `parallel` feature conflated `jwalk`
   dependencies into the tree, which isn't the case anymore.
 - <csr-id-89a41bf2b37db29b9983b4e5492cfd67ed490b23/> remove local-time-support feature toggle.
   We treat local time as default feature without a lot of fuzz, and
   will eventually document that definitive support needs a compile
   time switch in the compiler (`--cfg unsound_local_offset` or something).
   
   One day it will perish. Failure is possible anyway and we will write
   code to deal with it while minimizing the amount of system time
   fetches when asking for the current local time.
 - <csr-id-90611ce1527618bcc738440bfc1ccc7a45319974/> remove `path` module in favor of `git-path` crate
 - <csr-id-61e5cfece4d8f405e35fc1957b00ce1da7526c26/> renamed `progress::Read::reader` -> `progress::Read::inner`
 - <csr-id-e7526b2a7b51cbac4018e1ab3b623a85987fadc2/> parallel utilities now use `Send + Clone` insted of `Send + Sync`
   This helps to assure that thread-local computations always work with the
   kind of types we provide. The ones that are carrying out actions are
   notably not `Sync` anymore.
   
   We cater to that by defining our bounds accordingly, but for those
   who want to use other utilities that need Sync, using types like
   `Repository` and `thread_local!()` is the only way to make this
   work.

### Other

- <csr-id-ab6f98b905f13ed2a7c0c483f34fab63141fbc5b/> try-join with static typing works, but…
  …seems like a lot of effort. Probably not worth continuing here

### Bug Fixes

 - <csr-id-234cd10ca55482ce1a840ce3244308d249895bcc/> Assure std::io::copy() doesn't hang when we cause an interrupt
   The user can ask for interruptions which previously used the
   error kind Interrupted. This however has special meaning and
   usually means to retry.

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/> update sha-1 dependency to 0.10

### New Features

 - <csr-id-a3bd14a7753716e7ad67cd56d24eee1fa099a6fd/> re-export `prodash::progress::(Task|Value)` directly under `progress`
   This is in addition to the `progress` re-export which allows everything
   but yields less optimal import paths.
 - <csr-id-426057247a80821b3da22b4ae5d67bda89ce0631/> re-export `prodash` in `progress` module.
   That way one can access all types even if they are not re-exported.
 - <csr-id-c4a7634b0b29c74625e183953e59c65987e9d66c/> export `prodash::progress::Id` in the `progress` module for convenience.
 - <csr-id-25ad372bf500b851105f53b10369b5a689ba167e/> zlib::inflate::Error can now represent zlib status codes that represent failure.
 - <csr-id-6d530a1dc77f0f4ac00622a2fd47c7bdb731a77a/> name spawned threads
   That way it's a bit more obvious what's happening when the CPU goes
   up in flames.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.
 - <csr-id-9076ce33ec167e425a0163d3e40a81a3fd0db6cd/> `fs::Snapshot` can `Clone` if `T` can `Clone`.
 - <csr-id-a7c11d2cb5f88a4ff322d9a9848459062790d8b3/> perfect granularity for threads processing with `in_parallel_with_slice()`
 - <csr-id-ff1db66f2dad3afc8bc77610006bca9fea5947d2/> add `progress::Step|StepShared` as types of `prodash`
   This may help to use the `Progress::counter()` method as part of method
   signatures, being an `Option<progress::StepShared>`.
 - <csr-id-cfe46b502afc3ecb312849ddbd7748007d432cd1/> add zlib-ng feature to allow linking against system libz-ng
   Allow to use zlib-ng (zlib-ng-sys) with native API (no compat mode)
   that can co-exist with system libz (loaded by e.g. libcurl).
   This is used in gitoxide package on Alpine Linux.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-f498d35baba52e40ecd47381e87c1ce49cf13285/> add `fs-jwalk-single-threaded` feature to specifically decouple `jwalk` from rayon
   It has been an issue in https://github.com/starship/starship/issues/4251
   apparently and rayon interactions can be difficult.
 - <csr-id-7f199f0e5246809efde9880110093fbd11a4f8fe/> `fs::Snapshot` to on-demand reload shared resources.
 - <csr-id-c76fde7de278b49ded13b655d5345e4eb8c1b134/> initialize `Time` from `now_utc` and `now_local`
   Localtime support depends on some other factors now, but that
   will only get better over time.
   
   We might have to document `unsound_local_time` at some point.
 - <csr-id-e4d6685064ad2b433f8acd3a74b320bf0169a994/> Add `git_config::values::Path` for a typesafe git path
   Add a `Path` type to the `git_config::values` which
   can be interpolated according to gits own path interpolation
   rules.
 - <csr-id-3c8581fc294c65c9eb42698969fe3263135a864e/> add new 'path' module for all path-related conversions
   It's meant to unify all path and byte related handling to help assuring
   encoding is handled correctly or at least similarly everywhere.
 - <csr-id-15ff212b17087de93f259e366f4e4b821cfbc28e/> in-manifest and in-lib documentation of feature toggles
 - <csr-id-cb7e4e784d615f9fa3d6fb9c36240f0592403358/> Add InOrderIter to 'parallel' module
   This iterator makes possible identifies results using a sequence id
   and returns only consecutive items.
   
   Use it to collect unordered results produced by threads.
   It's advantage to collecting yourself and sorting is the potential
   for a smaller memory footprint of in-flight results, one doesn't
   have to collect them all for ordering, necessarily.
 - <csr-id-ca095ed881db2a8f06a6b067dbaac17e923b0945/> Make a scope-like abstraction available
   This allows more delicate threading control like is required for the
   index.
 - <csr-id-b8400ed80543d67a5895c975ba9b1fc28427411c/> decoding of variable int numbers.
   It's here only so that we can share the code across crates, for now
   without any feature toggles.
 - <csr-id-0a749a22057b5513a8cefa0e26b0a9a268c769d3/> Add `progress::Write` to automatically pass bytes written to a progress instance
 - <csr-id-7e95d8ab29051ffc892f2dcbaf5369e8c7e7b294/> add threading primitives with feature toggle
   If the `threading` feature is set, the `threading` module will contain thread-safe primitives
   for shared ownership and mutation, otherwise these will be their single threaded counterparts.
   
   This way, single-threaded applications don't have to pay for threaded primitives.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 509 commits contributed to the release.
 - 38 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 21 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#259](https://github.com/GitoxideLabs/gitoxide/issues/259), [#263](https://github.com/GitoxideLabs/gitoxide/issues/263), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#287](https://github.com/GitoxideLabs/gitoxide/issues/287), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#333](https://github.com/GitoxideLabs/gitoxide/issues/333), [#366](https://github.com/GitoxideLabs/gitoxide/issues/366), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#503](https://github.com/GitoxideLabs/gitoxide/issues/503), [#524](https://github.com/GitoxideLabs/gitoxide/issues/524), [#63](https://github.com/GitoxideLabs/gitoxide/issues/63), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Handle changelogs with upcoming version section if they were left for editing ([`0f5f47d`](https://github.com/GitoxideLabs/gitoxide/commit/0f5f47da4662b596cbbbd9c0d83e135e2cc52c11))
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Allow 'refactor' and 'other' in conventional messages if they have breaking changes ([`4eebaac`](https://github.com/GitoxideLabs/gitoxide/commit/4eebaac669e590beed112b622752997c64772ef1))
    - New changelogs for actor and features crates ([`e0d437c`](https://github.com/GitoxideLabs/gitoxide/commit/e0d437c4cfa06e0792609f41ed5876c390634921))
 * **[#222](https://github.com/GitoxideLabs/gitoxide/issues/222)**
    - Update changelogs prior to release ([`9a493d0`](https://github.com/GitoxideLabs/gitoxide/commit/9a493d0651b0b6d71cf230dc510a658be7f8cb19))
    - Stabilize changelogs ([`920e832`](https://github.com/GitoxideLabs/gitoxide/commit/920e83219911df1c440d3fe42fd5ec3a295b0bb8))
    - Update changelogs prior to release ([`b3e2252`](https://github.com/GitoxideLabs/gitoxide/commit/b3e2252f7461a003d9a4612da60ba931dd8c0bef))
 * **[#259](https://github.com/GitoxideLabs/gitoxide/issues/259)**
    - Unify trait bounds for parallel code: prefer Clone over Sync ([`c805d0b`](https://github.com/GitoxideLabs/gitoxide/commit/c805d0b231cf4d2f51dae7705bfbbc6562f86c32))
    - Remove trait bounds to allow single-threaded applications to exist ([`3c790e0`](https://github.com/GitoxideLabs/gitoxide/commit/3c790e01de0dbd3ffa2683d5cf060723d11d64a5))
 * **[#263](https://github.com/GitoxideLabs/gitoxide/issues/263)**
    - Make it possible to return read guards with packed buffers ([`f5c3c8f`](https://github.com/GitoxideLabs/gitoxide/commit/f5c3c8f7309bf53b9e53f786e75931d701a8585c))
    - Parallel utilities now use `Send + Clone` insted of `Send + Sync` ([`e7526b2`](https://github.com/GitoxideLabs/gitoxide/commit/e7526b2a7b51cbac4018e1ab3b623a85987fadc2))
    - A mad attempt to use thread-local everywhere and avoid Sync… ([`0af5077`](https://github.com/GitoxideLabs/gitoxide/commit/0af5077e1f028c1c69bbdc098bb567e486282c37))
    - Add threading primitives with feature toggle ([`7e95d8a`](https://github.com/GitoxideLabs/gitoxide/commit/7e95d8ab29051ffc892f2dcbaf5369e8c7e7b294))
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - Update sha-1 dependency to 0.10 ([`361892c`](https://github.com/GitoxideLabs/gitoxide/commit/361892ca15aa648802f6701ab6a5a30aedde3449))
    - Remove slow/unnecessary threading utilities ([`269b7ef`](https://github.com/GitoxideLabs/gitoxide/commit/269b7efc47bb1d6380b2059f63bd0c53fcd285de))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Add `progress::Write` to automatically pass bytes written to a progress instance ([`0a749a2`](https://github.com/GitoxideLabs/gitoxide/commit/0a749a22057b5513a8cefa0e26b0a9a268c769d3))
    - Renamed `progress::Read::reader` -> `progress::Read::inner` ([`61e5cfe`](https://github.com/GitoxideLabs/gitoxide/commit/61e5cfece4d8f405e35fc1957b00ce1da7526c26))
    - Adjust to changes in git-hash and git-pack ([`0cae25b`](https://github.com/GitoxideLabs/gitoxide/commit/0cae25b1bb3c902ec323f17a1d9743e42fe213d0))
 * **[#287](https://github.com/GitoxideLabs/gitoxide/issues/287)**
    - Upgrade to prodash 17 ([`47860b7`](https://github.com/GitoxideLabs/gitoxide/commit/47860b7e2769260cfb8522ae455c491605093423))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - Prepare changelogs for git-index and dependencies ([`f54bf4b`](https://github.com/GitoxideLabs/gitoxide/commit/f54bf4bde92b892b6d425987a6a37e10319c4635))
    - Fix docs ([`3f89b63`](https://github.com/GitoxideLabs/gitoxide/commit/3f89b6336e79bc12bc31d40b74221e79a72d2b36))
    - Fix build ([`e3977fe`](https://github.com/GitoxideLabs/gitoxide/commit/e3977fe033550bfd3297cdd674934e40476aa38b))
    - Use InOrderIter from git-features ([`7721b5f`](https://github.com/GitoxideLabs/gitoxide/commit/7721b5fc7cba86d785e0936fdfab2ea41163219f))
    - Add InOrderIter to 'parallel' module ([`cb7e4e7`](https://github.com/GitoxideLabs/gitoxide/commit/cb7e4e784d615f9fa3d6fb9c36240f0592403358))
    - Make a scope-like abstraction available ([`ca095ed`](https://github.com/GitoxideLabs/gitoxide/commit/ca095ed881db2a8f06a6b067dbaac17e923b0945))
    - Single and multi-threaded index tests ([`a22cb0f`](https://github.com/GitoxideLabs/gitoxide/commit/a22cb0f1ead9a2f32e43eb2fb378281e592a4ed3))
    - Decoding of variable int numbers. ([`b8400ed`](https://github.com/GitoxideLabs/gitoxide/commit/b8400ed80543d67a5895c975ba9b1fc28427411c))
 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Enforce path conversion on windows gnu, it doesn't seem to like slashes ([`4d55a8f`](https://github.com/GitoxideLabs/gitoxide/commit/4d55a8f99f2a0b7c0c4ed70a615b7e58b5bee04b))
    - Use hash_hasher based hash state for better keys/less collisions ([`814de07`](https://github.com/GitoxideLabs/gitoxide/commit/814de079f4226f42efa49ad334a348bce67184e4))
    - Upgrade parking_lot and cargo_toml ([`f95c1a0`](https://github.com/GitoxideLabs/gitoxide/commit/f95c1a0d9c19bcc6feb9b8739a09d86f9970a0e0))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
    - Remove `path` module in favor of `git-path` crate ([`90611ce`](https://github.com/GitoxideLabs/gitoxide/commit/90611ce1527618bcc738440bfc1ccc7a45319974))
    - Mild refactor of paths module to waste less on unix ([`d078d6e`](https://github.com/GitoxideLabs/gitoxide/commit/d078d6ee76a80d1dfaf71608c12d8a402bd670d4))
    - Refactor ([`8345b7c`](https://github.com/GitoxideLabs/gitoxide/commit/8345b7caa0cc1cd8489e41822eea89da4c539e6d))
    - More stable testing of perviously racy test for new parallelization mechanism ([`0b4b90f`](https://github.com/GitoxideLabs/gitoxide/commit/0b4b90fa498d9e07a55b72af2f799da4cd2da81f))
    - Salvage an alternative parallelization approach which might be good for index-creation ([`7e76796`](https://github.com/GitoxideLabs/gitoxide/commit/7e76796d5c2956961bd998286bec05fca1ba8fc4))
    - Refactor ([`f86eacc`](https://github.com/GitoxideLabs/gitoxide/commit/f86eacc5cfaf6d88ead4f8dbd65989d32674c213))
    - Use hopefully faster crossbeam channel to receive parallelized results ([`3b324b8`](https://github.com/GitoxideLabs/gitoxide/commit/3b324b868d9d172038797f911eeebfcba8107865))
    - Switch index checkout to chunk-based operation ([`e5f6943`](https://github.com/GitoxideLabs/gitoxide/commit/e5f69433e4a6cc7866b666e0baccfa32efb92a7f))
    - Add thread-count and chunk-size computation; interrupt capability ([`8cbe85d`](https://github.com/GitoxideLabs/gitoxide/commit/8cbe85d135898826a91939726465a9e295c1e24b))
    - `interrupt::Iter`, rename `interrupt::Iter` -> `interrupt::IterWithError` ([`8945d95`](https://github.com/GitoxideLabs/gitoxide/commit/8945d95f7fa88562d37ff67ac6e38bead73dd2df))
    - Fix `interrupt::Iter` ([`0f0d390`](https://github.com/GitoxideLabs/gitoxide/commit/0f0d390c475044a75e5db4dcd831d755e74aa3e9))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - In-manifest and in-lib documentation of feature toggles ([`15ff212`](https://github.com/GitoxideLabs/gitoxide/commit/15ff212b17087de93f259e366f4e4b821cfbc28e))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Remove local-time-support feature toggle. ([`89a41bf`](https://github.com/GitoxideLabs/gitoxide/commit/89a41bf2b37db29b9983b4e5492cfd67ed490b23))
    - Initialize `Time` from `now_utc` and `now_local` ([`c76fde7`](https://github.com/GitoxideLabs/gitoxide/commit/c76fde7de278b49ded13b655d5345e4eb8c1b134))
    - A first sketch on how identity management could look like. ([`780f14f`](https://github.com/GitoxideLabs/gitoxide/commit/780f14f5c270802e51cf039639c2fbdb5ac5a85e))
    - Update changelog prior to release ([`1d07934`](https://github.com/GitoxideLabs/gitoxide/commit/1d079346e789b0acc9a4bdf7577b21c1c37b6106))
    - Remove Option return values in favor of Result ([`493dbae`](https://github.com/GitoxideLabs/gitoxide/commit/493dbae434e8e4a939e90d03ec3f500744c0725a))
    - Add `git_config::values::Path` for a typesafe git path ([`e4d6685`](https://github.com/GitoxideLabs/gitoxide/commit/e4d6685064ad2b433f8acd3a74b320bf0169a994))
 * **[#333](https://github.com/GitoxideLabs/gitoxide/issues/333)**
    - Use git_features::path everywhere where there is a path conversion ([`2e1437c`](https://github.com/GitoxideLabs/gitoxide/commit/2e1437cb0b5dc77f2317881767f71eaf9b009ebf))
    - Make real clear panics are only possible on windows ([`6b283dc`](https://github.com/GitoxideLabs/gitoxide/commit/6b283dc7b9339fd65ea35f56eb29f121f571caf7))
    - One usage of os_str_bytes down, along with some custom conversion code ([`1cc95ce`](https://github.com/GitoxideLabs/gitoxide/commit/1cc95cefbd132a4277ec52c2147f7c81fea92d48))
    - Gitoxide-core without os-str-bytes ([`909aa14`](https://github.com/GitoxideLabs/gitoxide/commit/909aa1402c82c3128052023613a297b213716e3d))
    - Remove os_str_bytes from git-pack ([`86f6e50`](https://github.com/GitoxideLabs/gitoxide/commit/86f6e5054ea11b7aeb9c85321913de090f71e3a1))
    - Don't use os_str_ext in git-features; adapt git-ref ([`9258b7b`](https://github.com/GitoxideLabs/gitoxide/commit/9258b7baf0895593c13a152ff9e6f52e036cebe1))
    - Add new 'path' module for all path-related conversions ([`3c8581f`](https://github.com/GitoxideLabs/gitoxide/commit/3c8581fc294c65c9eb42698969fe3263135a864e))
 * **[#366](https://github.com/GitoxideLabs/gitoxide/issues/366)**
    - The first possibly working version of loading a mailmap with multiple sources ([`98d745e`](https://github.com/GitoxideLabs/gitoxide/commit/98d745e8080975a91cff1ce75e187258c851d3f4))
    - Cleanup bstr usage to not accidentally pull in unicode ([`8ff53af`](https://github.com/GitoxideLabs/gitoxide/commit/8ff53af9876a5e35bcfd076124ad776e1b6ff331))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - Make fmt ([`4b320e7`](https://github.com/GitoxideLabs/gitoxide/commit/4b320e773368ac5e8c38dd8a779ef3d6d2d024ec))
    - Optimize some portions of the Snapshot code for speed. ([`711fd5c`](https://github.com/GitoxideLabs/gitoxide/commit/711fd5c6c221440917fa68248e45d5278c780a9e))
    - More convenient API for fs::Snapshots ([`561d2e7`](https://github.com/GitoxideLabs/gitoxide/commit/561d2e746b1b82ac20f6f14b9c4e3910240075b4))
    - `fs::Snapshot` to on-demand reload shared resources. ([`7f199f0`](https://github.com/GitoxideLabs/gitoxide/commit/7f199f0e5246809efde9880110093fbd11a4f8fe))
    - Use generalized reload-on-demand in `git-ref` ([`8d0cce7`](https://github.com/GitoxideLabs/gitoxide/commit/8d0cce7d1521374d5199552fc69a417a957519bc))
    - Now it's possible to update packed refs using the shared code ([`78222c2`](https://github.com/GitoxideLabs/gitoxide/commit/78222c2e39aa24c84852e999448c042f2fd37db4))
    - The first step towards using the generalized `ReloadIfChanged` in git-ref ([`e8de0ef`](https://github.com/GitoxideLabs/gitoxide/commit/e8de0ef38db2f2d83cb277ed101464f23c0e98e4))
    - Generalized port of packed-refs update logic for use in index ([`e3aff0c`](https://github.com/GitoxideLabs/gitoxide/commit/e3aff0c2b83720e5745f3d7a8d0f571421a26d99))
    - Turn on performance mode for sha-1 computation ([`44371a1`](https://github.com/GitoxideLabs/gitoxide/commit/44371a10f464f32db346aa6b8309e983cfa20933))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Upgrade to `prodash` v21 ([`a0655dc`](https://github.com/GitoxideLabs/gitoxide/commit/a0655dc7bc5dff388bc69a648e7f16b44fd1abd9))
    - `fs::Snapshot` can `Clone` if `T` can `Clone`. ([`9076ce3`](https://github.com/GitoxideLabs/gitoxide/commit/9076ce33ec167e425a0163d3e40a81a3fd0db6cd))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
    - Working progress printing ([`67ec2c7`](https://github.com/GitoxideLabs/gitoxide/commit/67ec2c7f9a4a6cefdf7148f5c7e48a79f201c4d2))
    - First attempt to get progress information from stat worker. ([`0947c70`](https://github.com/GitoxideLabs/gitoxide/commit/0947c703f9cecc31ceba101565e6ecafb00adb08))
    - Upgrade to prodash 20.1 for `Progress::counter()` feature ([`0ac4a2c`](https://github.com/GitoxideLabs/gitoxide/commit/0ac4a2c514aeb94d8e90ce28ae7a0e0350c21ab2))
 * **[#503](https://github.com/GitoxideLabs/gitoxide/issues/503)**
    - Prepare changelog ([`3c99e7f`](https://github.com/GitoxideLabs/gitoxide/commit/3c99e7f02ada72a171856ffc5b870da83fffc703))
 * **[#524](https://github.com/GitoxideLabs/gitoxide/issues/524)**
    - Prepare changelogs prior to release ([`6446b39`](https://github.com/GitoxideLabs/gitoxide/commit/6446b395d5926565ef899b0c923f35468ccf1921))
 * **[#63](https://github.com/GitoxideLabs/gitoxide/issues/63)**
    - Git-protocol uses `oid` type ([`3930a6f`](https://github.com/GitoxideLabs/gitoxide/commit/3930a6ff508f5bb2249fb2c2f21e00b74fecda22))
    - Make ObjectId/oid happen! ([`ca78d15`](https://github.com/GitoxideLabs/gitoxide/commit/ca78d15373ec988d909be8f240baefe75555e077))
    - Move git-hash::owned::Id into git-hash::Id ([`fdbe704`](https://github.com/GitoxideLabs/gitoxide/commit/fdbe704b6c9ace2b8f629f681a0580b24749a238))
    - Rename `git_hash::*::Digest` to `Id` ([`188d90a`](https://github.com/GitoxideLabs/gitoxide/commit/188d90ad463d342d715af701b03f0ed392c977fc))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **Uncategorized**
    - Release gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6efd0d3`](https://github.com/GitoxideLabs/gitoxide/commit/6efd0d31fbeca31ab7319aa2ac97bb31dc4ce055))
    - Release gix-date v0.4.2, gix-hash v0.10.2, gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6ccc88a`](https://github.com/GitoxideLabs/gitoxide/commit/6ccc88a8e4a56973b1a358cf72dc012ee3c75d56))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
    - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
    - Adjust to renaming of `git-index` to `gix-index` ([`86db5e0`](https://github.com/GitoxideLabs/gitoxide/commit/86db5e09fc58ce66b252dc13b8d7e2c48e4d5062))
    - Adjust to renaming of `git-diff` to `gix-diff` ([`49a163e`](https://github.com/GitoxideLabs/gitoxide/commit/49a163ec8b18f0e5fcd05a315de16d5d8be7650e))
    - Adjust to renaming of `git-commitgraph` to `gix-commitgraph` ([`f1dd0a3`](https://github.com/GitoxideLabs/gitoxide/commit/f1dd0a3366e31259af029da73228e8af2f414244))
    - Adjust to renaming of `git-mailmap` to `gix-mailmap` ([`2e28c56`](https://github.com/GitoxideLabs/gitoxide/commit/2e28c56bb9f70de6f97439818118d3a25859698f))
    - Adjust to renaming of `git-discover` to `gix-discover` ([`53adfe1`](https://github.com/GitoxideLabs/gitoxide/commit/53adfe1c34e9ea3b27067a97b5e7ac80b351c441))
    - Adjust to renaming of `git-chunk` to `gix-chunk` ([`59194e3`](https://github.com/GitoxideLabs/gitoxide/commit/59194e3a07853eae0624ebc4907478d1de4f7599))
    - Adjust to renaming of `git-bitmap` to `gix-bitmap` ([`75f2a07`](https://github.com/GitoxideLabs/gitoxide/commit/75f2a079b17489f62bc43e1f1d932307375c4f9d))
    - Adjust to renaming for `git-protocol` to `gix-protocol` ([`823795a`](https://github.com/GitoxideLabs/gitoxide/commit/823795addea3810243cab7936cd8ec0137cbc224))
    - Adjust to renaming of `git-refspec` to `gix-refspec` ([`c958802`](https://github.com/GitoxideLabs/gitoxide/commit/c9588020561577736faa065e7e5b5bb486ca8fe1))
    - Adjust to renaming of `git-revision` to `gix-revision` ([`ee0ee84`](https://github.com/GitoxideLabs/gitoxide/commit/ee0ee84607c2ffe11ee75f27a31903db68afed02))
    - Adjust to renaming of `git-transport` to `gix-transport` ([`b2ccf71`](https://github.com/GitoxideLabs/gitoxide/commit/b2ccf716dc4425bb96651d4d58806a3cc2da219e))
    - Adjust to renaming of `git-credentials` to `gix-credentials` ([`6b18abc`](https://github.com/GitoxideLabs/gitoxide/commit/6b18abcf2856f02ab938d535a65e51ac282bf94a))
    - Adjust to renaming of `git-prompt` to `gix-prompt` ([`6a4654e`](https://github.com/GitoxideLabs/gitoxide/commit/6a4654e0d10ab773dd219cb4b731c0fc1471c36d))
    - Adjust to renaming of `git-command` to `gix-command` ([`d26b8e0`](https://github.com/GitoxideLabs/gitoxide/commit/d26b8e046496894ae06b0bbfdba77196976cd975))
    - Adjust to renaming of `git-packetline` to `gix-packetline` ([`5cbd22c`](https://github.com/GitoxideLabs/gitoxide/commit/5cbd22cf42efb760058561c6c3bbcd4dab8c8be1))
    - Adjust to renaming of `git-worktree` to `gix-worktree` ([`73a1282`](https://github.com/GitoxideLabs/gitoxide/commit/73a12821b3d9b66ec1714d07dd27eb7a73e3a544))
    - Adjust to renamining of `git-hashtable` to `gix-hashtable` ([`26a0c98`](https://github.com/GitoxideLabs/gitoxide/commit/26a0c98d0a389b03e3dc7bfc758b37155e285244))
    - Adjust to renamining of `git-worktree` to `gix-worktree` ([`108bb1a`](https://github.com/GitoxideLabs/gitoxide/commit/108bb1a634f4828853fb590e9fc125f79441dd38))
    - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
    - Adjust to renaming of `git-date` to `gix-date` ([`9a79ff2`](https://github.com/GitoxideLabs/gitoxide/commit/9a79ff2d5cc74c1efad9f41e21095ae498cce00b))
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
    - Rename `git-features` to `gix-features` ([`85f7e1a`](https://github.com/GitoxideLabs/gitoxide/commit/85f7e1a97c936f7cbebfbb3b2952f965601a7cbb))
    - Adjust to renaming of `git-glob` to `gix-glob` ([`35b2a3a`](https://github.com/GitoxideLabs/gitoxide/commit/35b2a3acbc8f2a03f151bc0a3863163844e0ca86))
    - Adjust to renaming of `git-sec` to `gix-sec` ([`eabbb92`](https://github.com/GitoxideLabs/gitoxide/commit/eabbb923bd5a32fc80fa80f96cfdc2ab7bb2ed17))
    - Adapt to renaming of `git-path` to `gix-path` ([`d3bbcfc`](https://github.com/GitoxideLabs/gitoxide/commit/d3bbcfccad80fc44ea8e7bf819f23adaca06ba2d))
    - Adjust to rename of `git-config-value` to `gix-config-value` ([`622b3e1`](https://github.com/GitoxideLabs/gitoxide/commit/622b3e1d0bffa0f8db73697960f9712024fac430))
    - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
    - Re-export `prodash::progress::(Task|Value)` directly under `progress` ([`a3bd14a`](https://github.com/GitoxideLabs/gitoxide/commit/a3bd14a7753716e7ad67cd56d24eee1fa099a6fd))
    - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
    - Re-export `prodash` in `progress` module. ([`4260572`](https://github.com/GitoxideLabs/gitoxide/commit/426057247a80821b3da22b4ae5d67bda89ce0631))
    - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
    - Prepare changelogs prior to release ([`7c846d2`](https://github.com/GitoxideLabs/gitoxide/commit/7c846d2102dc767366771925212712ef8cc9bf07))
    - Merge branch 'Lioness100/main' ([`1e544e8`](https://github.com/GitoxideLabs/gitoxide/commit/1e544e82455bf9ecb5e3c2146280eaf7ecd81f16))
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Make fmt ([`e22080e`](https://github.com/GitoxideLabs/gitoxide/commit/e22080e4a29d0bad15a99d565a5e3e304a8743ec))
    - Merge branch 'adjustments-for-cargo' ([`7bba270`](https://github.com/GitoxideLabs/gitoxide/commit/7bba2709488b7eb999b8136dbab03af977241678))
    - Export `prodash::progress::Id` in the `progress` module for convenience. ([`c4a7634`](https://github.com/GitoxideLabs/gitoxide/commit/c4a7634b0b29c74625e183953e59c65987e9d66c))
    - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
    - Prepare changelogs prior to release ([`93bef97`](https://github.com/GitoxideLabs/gitoxide/commit/93bef97b3c0c75d4bf7119fdd787516e1efc77bf))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`9e04685`](https://github.com/GitoxideLabs/gitoxide/commit/9e04685dd3f109bfb27663f9dc7c04102e660bf2))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Prepare changelogs prior to release ([`30d8ca1`](https://github.com/GitoxideLabs/gitoxide/commit/30d8ca19284049dcfbb0de2698cafae1d1a16b0c))
    - Make fmt ([`511ed00`](https://github.com/GitoxideLabs/gitoxide/commit/511ed0000397a5b268530c8f5362e7d25b7c1594))
    - Merge branch 'adjustments-for-cargo' ([`f8c562a`](https://github.com/GitoxideLabs/gitoxide/commit/f8c562a559e6dc3377583cc7200585dad7c3d481))
    - Upgrade to prodash v23 ([`5bf0034`](https://github.com/GitoxideLabs/gitoxide/commit/5bf0034fb3918e57562b7089ceba83d63a1854bf))
    - Release git-features v0.25.1, git-url v0.12.2, git-odb v0.38.1, git-transport v0.24.2, git-repository v0.30.2 ([`bb0a07b`](https://github.com/GitoxideLabs/gitoxide/commit/bb0a07b5edd5f980989d1a92e74df7f183febe87))
    - Merge branch 'loose-find-panic' ([`95cccdd`](https://github.com/GitoxideLabs/gitoxide/commit/95cccddd3c181eb2a85b12823c27beb054adf5d8))
    - Zlib::inflate::Error can now represent zlib status codes that represent failure. ([`25ad372`](https://github.com/GitoxideLabs/gitoxide/commit/25ad372bf500b851105f53b10369b5a689ba167e))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - Thanks clippy ([`f1160fb`](https://github.com/GitoxideLabs/gitoxide/commit/f1160fb42acf59b37cbeda546a7079af3c9bc050))
    - Add support for explicit non-parallel iteration. ([`0f27c67`](https://github.com/GitoxideLabs/gitoxide/commit/0f27c67c92fc0bc23a6712b5c4c730ad6a0156bf))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Upgrade to prodash v22 for API improvements ([`77ab98d`](https://github.com/GitoxideLabs/gitoxide/commit/77ab98dd41c3849b674d8b3794ef29219ca1447d))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
    - Merge branch 'named-threads' ([`726dd87`](https://github.com/GitoxideLabs/gitoxide/commit/726dd87b5db45c333ccad898338a1cacea9e3269))
    - Name spawned threads ([`6d530a1`](https://github.com/GitoxideLabs/gitoxide/commit/6d530a1dc77f0f4ac00622a2fd47c7bdb731a77a))
    - Upgrade to `prodash 21.1` and add `Ids` to all progress instances. ([`c8835c6`](https://github.com/GitoxideLabs/gitoxide/commit/c8835c6edae784c9ffcb69a674c0a6545dbb2af3))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Thanks clippy ([`ad96233`](https://github.com/GitoxideLabs/gitoxide/commit/ad96233e1aa77fb7d9185f653f3e9519128cf20f))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Remove `fs-jwalk-single-threaded` in favor of `fs-walkdir-parallel`. ([`38446dc`](https://github.com/GitoxideLabs/gitoxide/commit/38446dc8824afef30ef121598de3451d13b9262c))
    - Make jwalk fully optional ([`3b29fc1`](https://github.com/GitoxideLabs/gitoxide/commit/3b29fc18672c0176684c797a0f16f85d09369bf8))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Merge branch 'main' into clone ([`acb0738`](https://github.com/GitoxideLabs/gitoxide/commit/acb07382a9306d6962bea60e8977d83d021743f4))
    - Merge branch 'delta-tree-parallelization' ([`cca2ad5`](https://github.com/GitoxideLabs/gitoxide/commit/cca2ad5ee9483d7da968658e0a4d610dbc4ad4d6))
    - Don't enforce Send bounds in serial version of `in_parallel_with_slice()` ([`dda661e`](https://github.com/GitoxideLabs/gitoxide/commit/dda661e1b7cc0ace6cd9504233f20980e1e52387))
    - Allow discarding the state which could otherwise be used for aggregation. ([`56792fb`](https://github.com/GitoxideLabs/gitoxide/commit/56792fb53299d52073555c8646360ec0ae88c86d))
    - Allow input for `in_parallel_with_slice` to be mutable. ([`e928bf7`](https://github.com/GitoxideLabs/gitoxide/commit/e928bf7c699a7e48ad283c2cf7fd7479c37c70fc))
    - Perfect granularity for threads processing with `in_parallel_with_slice()` ([`a7c11d2`](https://github.com/GitoxideLabs/gitoxide/commit/a7c11d2cb5f88a4ff322d9a9848459062790d8b3))
    - Add `progress::Step|StepShared` as types of `prodash` ([`ff1db66`](https://github.com/GitoxideLabs/gitoxide/commit/ff1db66f2dad3afc8bc77610006bca9fea5947d2))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
    - Update changelog prior to release ([`ff80042`](https://github.com/GitoxideLabs/gitoxide/commit/ff80042c9691e5dba5834c674174fdf6d3bdfe7d))
    - Fix git-features docs build ([`e5963fe`](https://github.com/GitoxideLabs/gitoxide/commit/e5963fea183d81db1fe502121b494146a58bd86e))
    - Upgrade all dependencies, except for `windows` ([`2968181`](https://github.com/GitoxideLabs/gitoxide/commit/29681819ffe53d3926d631dc482f71d6200cb549))
    - Merge branch 'dep-upgrade' ([`59767b1`](https://github.com/GitoxideLabs/gitoxide/commit/59767b1fc1d07b8a7a9333a719e3716746611bc4))
    - Upgrade prodash and crosstermion to latest versions ([`ab7ee5b`](https://github.com/GitoxideLabs/gitoxide/commit/ab7ee5b5d5c15771f431ada9c3b4f53e4be2afdd))
    - Add zlib-ng feature to allow linking against system libz-ng ([`cfe46b5`](https://github.com/GitoxideLabs/gitoxide/commit/cfe46b502afc3ecb312849ddbd7748007d432cd1))
    - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
    - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
    - Merge branch 'main' into filter-refs-by-spec ([`1f6e5ab`](https://github.com/GitoxideLabs/gitoxide/commit/1f6e5ab15f5fd8d23719b13e6aea59cd231ac0fe))
    - Merge branch 'fix-522' ([`5869e9f`](https://github.com/GitoxideLabs/gitoxide/commit/5869e9ff2508d5a93c07635277af8764fcb57713))
    - Release git-hash v0.9.9 ([`da0716f`](https://github.com/GitoxideLabs/gitoxide/commit/da0716f8c27b4f29cfff0e5ce7fcb3d7240f4aeb))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-features v0.22.3, git-revision v0.4.4 ([`c2660e2`](https://github.com/GitoxideLabs/gitoxide/commit/c2660e2503323531ba02519eaa51124ee22fec51))
    - Merge branch 'main' into filter-refs-by-spec ([`dbfa328`](https://github.com/GitoxideLabs/gitoxide/commit/dbfa3282cf876596b250b2040c1ec0b761741796))
    - Merge branch 'zlib-sys' ([`7b48297`](https://github.com/GitoxideLabs/gitoxide/commit/7b482977a556d28f5d9759e1be33cdf3d85c8665))
    - Restrict `sha1` `asm` to supported archs ([`b383fab`](https://github.com/GitoxideLabs/gitoxide/commit/b383fabbe10868317be51b99cfdd9b0981816042))
    - Add feature to link to traditional zlib for dynamic linking support ([`c954bbf`](https://github.com/GitoxideLabs/gitoxide/commit/c954bbff60ba70a0f1680e3978dd1f8fc1e3a0e7))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
    - Add `fs-jwalk-single-threaded` feature to specifically decouple `jwalk` from rayon ([`f498d35`](https://github.com/GitoxideLabs/gitoxide/commit/f498d35baba52e40ecd47381e87c1ce49cf13285))
    - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
    - Thanks clippy! ([`c072dbb`](https://github.com/GitoxideLabs/gitoxide/commit/c072dbb3e203e4a42843895b7d99404d900fdccd))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Merge branch 'kianmeng-fix-typos' ([`4e7b343`](https://github.com/GitoxideLabs/gitoxide/commit/4e7b34349c0a01ad8686bbb4eb987e9338259d9c))
    - Fix typos ([`e9fcb70`](https://github.com/GitoxideLabs/gitoxide/commit/e9fcb70e429edb2974afa3f58d181f3ef14c3da3))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Prepare changelog prior to release ([`3c50625`](https://github.com/GitoxideLabs/gitoxide/commit/3c50625fa51350ec885b0f38ec9e92f9444df0f9))
    - Merge branch 'config-cascade' ([`f144eaf`](https://github.com/GitoxideLabs/gitoxide/commit/f144eaf5863ae5cac63103f0db51c35fcf03a948))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Git-features' walkdir: 2.3.1 -> 2.3.2 ([`41dd754`](https://github.com/GitoxideLabs/gitoxide/commit/41dd7545234e6d2637d2bca5bb6d4f6d8bfc8f57))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Update changelogs prior to release ([`bb424f5`](https://github.com/GitoxideLabs/gitoxide/commit/bb424f51068b8a8e762696890a55ab48900ab980))
    - Assure we used most recent version of crossbeam-utils ([`033f0d3`](https://github.com/GitoxideLabs/gitoxide/commit/033f0d3e0015b7eead6408c775d2101eb413ffbf))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Thanks clippy ([`380174f`](https://github.com/GitoxideLabs/gitoxide/commit/380174f0ad9e60ccafcd4cfb24e244f106137964))
    - Release git-features v0.20.0, git-config v0.2.0 ([`a6460db`](https://github.com/GitoxideLabs/gitoxide/commit/a6460db80ba3c49ea37c712465c7cbdefa5c32b6))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Remove 'unused_mut' warning on windows ([`4733e6c`](https://github.com/GitoxideLabs/gitoxide/commit/4733e6c6f5ea7d5afa4dd171bbba066cf5120ddc))
    - Make fmt ([`7cf3545`](https://github.com/GitoxideLabs/gitoxide/commit/7cf354509b545f7e7c99e159b5989ddfbe86273d))
    - Commit to using 'unicode' feature of bstr as git-object wants it too ([`471fa62`](https://github.com/GitoxideLabs/gitoxide/commit/471fa62b142ba744541d7472464d62826f5c6b93))
    - Assure std::io::copy() doesn't hang when we cause an interrupt ([`234cd10`](https://github.com/GitoxideLabs/gitoxide/commit/234cd10ca55482ce1a840ce3244308d249895bcc))
    - Upgrade to prodash 19 ([`90c6c5a`](https://github.com/GitoxideLabs/gitoxide/commit/90c6c5aec4015ff969d6e2514fa4d49873ee80f5))
    - Thanks clippy ([`07a4094`](https://github.com/GitoxideLabs/gitoxide/commit/07a4094965ac1b4eb223da8e5ca5cc4a86c5f596))
    - Properly document optional features ([`572e57d`](https://github.com/GitoxideLabs/gitoxide/commit/572e57d5796692764c5c9633969aad25a6f9221a))
    - Merge branch 'svetli-n-path_value' ([`e8383ca`](https://github.com/GitoxideLabs/gitoxide/commit/e8383caf6db211beb57d70019fe4ad13ce9066ee))
    - Small refactoring and documentation. ([`fefb01b`](https://github.com/GitoxideLabs/gitoxide/commit/fefb01b84f954700b19e010282c4b413de8e03d2))
    - Upgrade document-features ([`c35e62e`](https://github.com/GitoxideLabs/gitoxide/commit/c35e62e0da9ac1f7dcb863f5f9c69108c728d32e))
    - Merge branch 'unify-path-encoding' ([`566ff8a`](https://github.com/GitoxideLabs/gitoxide/commit/566ff8a3597b889899d41ca15e5b9af7e05f1a4b))
    - Thanks clippy ([`a8e9497`](https://github.com/GitoxideLabs/gitoxide/commit/a8e9497caebf1c0e9faac537717cd86378f1acf6))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Prepare changelogs for release ([`674ec73`](https://github.com/GitoxideLabs/gitoxide/commit/674ec73b0816baa2c63b4ef1b40b7a41849c5e95))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Upgrade to tui 0.17 and prodash 18 ([`eba101a`](https://github.com/GitoxideLabs/gitoxide/commit/eba101a576ecb7bc0f63357d0dd81eb817b94be4))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Upgrade dependencies ([`968df47`](https://github.com/GitoxideLabs/gitoxide/commit/968df4746729556dcf4f5039b1d1ed1a1da2705a))
    - Refactor ([`e7fbd9f`](https://github.com/GitoxideLabs/gitoxide/commit/e7fbd9f3700496ad7bb7e71226c4d25429f0ccd5))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`7dd2313`](https://github.com/GitoxideLabs/gitoxide/commit/7dd2313d980fe7c058319ae66d313b3097e3ae5f))
    - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
    - Adjust changelogs prior to release ([`ec38950`](https://github.com/GitoxideLabs/gitoxide/commit/ec3895005d141abe79764eaff7c0f04153e38d73))
    - Thanks clippy ([`db1bb99`](https://github.com/GitoxideLabs/gitoxide/commit/db1bb99101a9248b464b0df9f526067b8f2a184e))
    - Merge branch 'pack-consistency' ([`5982406`](https://github.com/GitoxideLabs/gitoxide/commit/5982406b4e1b26fd383d9ec21a3cf652ec8ab25f))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Make fmt, but now it picked up some parts that usually don't get altered… ([`01f7b72`](https://github.com/GitoxideLabs/gitoxide/commit/01f7b729337bd2c99498321c479a9a13b1858e3e))
    - Release git-features v0.16.4 ([`fd189c7`](https://github.com/GitoxideLabs/gitoxide/commit/fd189c7d973ad2a639d319f3761f37aa90712ef6))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - [features #190] be more explicit about why sha1-asm is disabled ([`507d710`](https://github.com/GitoxideLabs/gitoxide/commit/507d710d837c3911a9329c1c132eee912a37e1a8))
    - [various #190] rename 'local-offset' to 'local-time-support' ([`3a7d379`](https://github.com/GitoxideLabs/gitoxide/commit/3a7d3793a235ac872437f3bfedb9dd8fde9b31b1))
    - [actor #190] methods to get an actor signature at the current time ([`6d0bedd`](https://github.com/GitoxideLabs/gitoxide/commit/6d0beddb20092a80b113a39c862d6b680d79deb6))
    - [features #189] simple UTC-offset support for git-features ([`b58134b`](https://github.com/GitoxideLabs/gitoxide/commit/b58134bbd132f9e685d1adf7859ec5219c16dd25))
    - [features #???] WIP local time ([`1388ebf`](https://github.com/GitoxideLabs/gitoxide/commit/1388ebf0925eb326ec3045d7f83bd5beda22a6fe))
    - [#189] Upgrade to prodash 16… ([`8e98418`](https://github.com/GitoxideLabs/gitoxide/commit/8e98418652926860f58906a6f21a3210e2f0183f))
    - [pack #67] Optimize caches based on cache debugging ([`1271c01`](https://github.com/GitoxideLabs/gitoxide/commit/1271c01d2635ab49474add61a9feb78b98bd6180))
    - [pack #67] Add cache debugging capabilities to git-features ([`8776c98`](https://github.com/GitoxideLabs/gitoxide/commit/8776c9834ac4622b3057f5db464a9817ed9acdb0))
    - Merge branch 'main' into 162-repo-design-sketch ([`e63b634`](https://github.com/GitoxideLabs/gitoxide/commit/e63b63412c02db469fbdb17da82cd1e9fda1ef0f))
    - Thanks clippy ([`d689599`](https://github.com/GitoxideLabs/gitoxide/commit/d689599d1b819c18a3be60075170dbe00462e216))
    - [features] refactor ([`0958fc8`](https://github.com/GitoxideLabs/gitoxide/commit/0958fc8dbaa72dda0c1e2d40a88d74b4e18bfe39))
    - [features] refactor ([`d4605cd`](https://github.com/GitoxideLabs/gitoxide/commit/d4605cde6d825c0bfaf4282c4cbd85d9f07dc43f))
    - Release git-features v0.16.2 ([`42861ca`](https://github.com/GitoxideLabs/gitoxide/commit/42861ca4f0cc9b741d033d4ffa50147b08513b58))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - (cargo-release) version 0.16.1 ([`e10e55c`](https://github.com/GitoxideLabs/gitoxide/commit/e10e55c1bf1b40965da9b8b6c87953e6eafda09a))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - (cargo-release) version 0.16.0 ([`1231dbd`](https://github.com/GitoxideLabs/gitoxide/commit/1231dbd16dacefb39adec8e067c312d313a82e3c))
    - Upgrade prodash/crosstermion ([`f109409`](https://github.com/GitoxideLabs/gitoxide/commit/f1094099de028deabbee3587a70291a7e625e328))
    - Clippy on tests and thanks clippy ([`a77a71c`](https://github.com/GitoxideLabs/gitoxide/commit/a77a71cf02d328a2a964388928d6b2a235a0aa85))
    - [pack] fix build ([`98dd557`](https://github.com/GitoxideLabs/gitoxide/commit/98dd557b963acfe1c4e717451d222c187c46a5da))
    - [pack] all tests running for now, but… ([`aec8439`](https://github.com/GitoxideLabs/gitoxide/commit/aec8439683c639f7b6e344cb76bf1dd9fc769d17))
    - Refactor sha-1 specification to avoid duplication ([`e23d19c`](https://github.com/GitoxideLabs/gitoxide/commit/e23d19cd339f0ca5420c82e8125d2c9c7dfcb0da))
    - Resolver = 2: works! ([`6dc8779`](https://github.com/GitoxideLabs/gitoxide/commit/6dc877993135ce86649b239821e5b374251743d0))
    - Try windows one more time: resolver = "2" ([`69d52b8`](https://github.com/GitoxideLabs/gitoxide/commit/69d52b8ed7a733fe7f31826e576ba8b19619b148))
    - Fix windows, leave todo, move on ([`2de9e78`](https://github.com/GitoxideLabs/gitoxide/commit/2de9e78dba35de31456eb553ae703120de23cba6))
    - See if turning off "asm" support entirely fixes windows ([`b804ef2`](https://github.com/GitoxideLabs/gitoxide/commit/b804ef2ea6da1ebffaab4d09d0b91eae98ff70c9))
    - Try to fix build, again ([`c616627`](https://github.com/GitoxideLabs/gitoxide/commit/c616627cc9984e40798120a801387fc179d6640b))
    - Don't use ASM on windows for Sha1 as it fails to build there. ([`ba1fb7a`](https://github.com/GitoxideLabs/gitoxide/commit/ba1fb7ab5bc03f5a23ece32ff1e144544e1eaeae))
    - [features] enable ASM for everyone… ([`7a1128f`](https://github.com/GitoxideLabs/gitoxide/commit/7a1128f594c5395a22e5e2b23772bce1d4de7a37))
    - [ref] reproducible loose ref iteration with built-in sorting ([`e138748`](https://github.com/GitoxideLabs/gitoxide/commit/e13874807ccc3cbc2b4aacccf63ac5c3dd21c445))
    - [features] fix docs in the absence of sha1 related features ([`6ca02ac`](https://github.com/GitoxideLabs/gitoxide/commit/6ca02ace7552c1ffaead81929bc751d96afa713a))
    - Remove unnecessary pub(crate) exports ([`3d2456e`](https://github.com/GitoxideLabs/gitoxide/commit/3d2456e11709f0461b37c6df55ecc3861ca4cab5))
    - [ref] first rough implementation of loose ref iteration ([`918af42`](https://github.com/GitoxideLabs/gitoxide/commit/918af425298a1fdbb8e7dd6328daefe9eaa10cef))
    - Refactor ([`2174513`](https://github.com/GitoxideLabs/gitoxide/commit/21745135ced62411be535ebbc827b3638726318b))
    - Fix docs ([`e68d460`](https://github.com/GitoxideLabs/gitoxide/commit/e68d460716dc51c7f4757c11f3c8af6c3881e2cf))
    - Remove mentions of interrupt handling feature toggles ([`833ac04`](https://github.com/GitoxideLabs/gitoxide/commit/833ac0464b42bd3ecc76c6263b4b06e8ab4ff182))
    - Fix everything up so that… ([`5930563`](https://github.com/GitoxideLabs/gitoxide/commit/5930563601d6c2148cf39e109f69f8b7c7dfcb36))
    - A first attempt to make intrerupt tools work, but… ([`8fb8d37`](https://github.com/GitoxideLabs/gitoxide/commit/8fb8d374ecfeffa3ae1bd07bf9bc5014351730f5))
    - First step towards moving git-features::interrupt… ([`8a741d0`](https://github.com/GitoxideLabs/gitoxide/commit/8a741d0c5423ed7c35d9382307c760a6b9460ccd))
    - Fix build ([`ea2bfac`](https://github.com/GitoxideLabs/gitoxide/commit/ea2bfac65f742ca7617bc77a50376c29156c4ea5))
    - Refactor ([`7f9be36`](https://github.com/GitoxideLabs/gitoxide/commit/7f9be36ea909ee67555591287bcb140fdc54c801))
    - And one less usage of the global interrupt handler… ([`5da57a3`](https://github.com/GitoxideLabs/gitoxide/commit/5da57a3b0efef75ad82cb4d1cd496fc7fc0f1c23))
    - Make most interrupts local to the method or function ([`4588993`](https://github.com/GitoxideLabs/gitoxide/commit/458899306a3f3c8578f185d7ecbf1ade2a7142dd))
    - Fix build ([`04d919f`](https://github.com/GitoxideLabs/gitoxide/commit/04d919f9228d287912554275194487870500d18c))
    - Refactor ([`e0b7f69`](https://github.com/GitoxideLabs/gitoxide/commit/e0b7f695ee6bd1032544a29d91906f9b75e12d46))
    - [features] sketch of iterator to auto-check for interruptions ([`61d3a15`](https://github.com/GitoxideLabs/gitoxide/commit/61d3a15c66b4c1be1d98715b8a60705a3a314455))
    - [tempfile] integrate with git-features to have a single top-level interrupt… ([`6e9400d`](https://github.com/GitoxideLabs/gitoxide/commit/6e9400d9cb8e370d870c3aa635718b134c82268f))
    - [features] protect interrupt handler from multi-initialization ([`592404c`](https://github.com/GitoxideLabs/gitoxide/commit/592404c2b24dc9d24465ff5f73216213436a277a))
    - [interrupt] remove any user mesasages as it can't be done in a handler. ([`8a10af7`](https://github.com/GitoxideLabs/gitoxide/commit/8a10af77db654ebce940bb05f8eefd171036ef40))
    - [tempfile] a first somewhat working version of signal-hooks for interrupt handling ([`07b3242`](https://github.com/GitoxideLabs/gitoxide/commit/07b3242e446cb4520dbc54308632ab6221fc19c8))
    - Update to latest prodash to get rid of ctrlc ([`c070d6f`](https://github.com/GitoxideLabs/gitoxide/commit/c070d6f5273d7ef9049ddd02fd26332623dc0ae6))
    - Refactor ([`2e86723`](https://github.com/GitoxideLabs/gitoxide/commit/2e8672312a4b1e2638e3ffe82a97cc2f87b496cf))
    - Bump crossbeam-utils from 0.8.4 to 0.8.5 ([`fce4d10`](https://github.com/GitoxideLabs/gitoxide/commit/fce4d107c7abc778bbdfcd37349c3075e54fd756))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - [git-pack] fix docs ([`efd20d4`](https://github.com/GitoxideLabs/gitoxide/commit/efd20d4e1afbfbe573d620dea4761c06f948a296))
    - [git-features] fix compilation ([`38c7961`](https://github.com/GitoxideLabs/gitoxide/commit/38c796142dc5823e1cb14906d9cd4040a8c3be3a))
    - [git-pack] move hash-writer to git-features as it's quite general purpose ([`80e5640`](https://github.com/GitoxideLabs/gitoxide/commit/80e5640169363910b4189fda58bb495c6677eaaa))
    - [git-features] Remove feature that would break licensing agreements ([`cd6ce67`](https://github.com/GitoxideLabs/gitoxide/commit/cd6ce673308e7e5b1e86fb682ee3ace2ca9ae18c))
    - [git-features] fix typo ([`c6f342f`](https://github.com/GitoxideLabs/gitoxide/commit/c6f342f3d29a969a08d037f01eb24555bc03e85e))
    - [git-features] Finally zlib with feature toggles is working… ([`057016e`](https://github.com/GitoxideLabs/gitoxide/commit/057016e2df3138992c4857f9b65bf19dc2c9a097))
    - [git-features] And now zlib finally works! ([`6d887d5`](https://github.com/GitoxideLabs/gitoxide/commit/6d887d589a57e159986f049c8a9e19c52ce7b85b))
    - [git-features] simplify even more ([`ca54d97`](https://github.com/GitoxideLabs/gitoxide/commit/ca54d97d579dd4f16025a2325d5e39431f6e8a36))
    - [git-features] refactor to help understand a zlib-related logic bug ([`ae826e8`](https://github.com/GitoxideLabs/gitoxide/commit/ae826e8c3240efd14939beedd33a06695a6c112b))
    - [git-features] a first step towards supporting a pure rust zlib backend ([`040cab7`](https://github.com/GitoxideLabs/gitoxide/commit/040cab7f27de83b283957189244d523d71ca1457))
    - [git-features] Add zlib module to allow changing implementation on the fly ([`4bdf783`](https://github.com/GitoxideLabs/gitoxide/commit/4bdf7833d99d3c2884b9747614f9c14a06c1e945))
    - (cargo-release) version 0.15.0 ([`d69d9fb`](https://github.com/GitoxideLabs/gitoxide/commit/d69d9fb0931f8257cef96ef14a89da9340ad9738))
    - Put prodash behind a feature toggle, too ([`966058d`](https://github.com/GitoxideLabs/gitoxide/commit/966058d611c548e90c050462de52e36f1925e775))
    - Put 'walkdir' behind a feature flag/make it optional. ([`1a3cc5b`](https://github.com/GitoxideLabs/gitoxide/commit/1a3cc5bea1868ed3ae015403fbe0cdec788be749))
    - Put 'sha1' behind a feature toggle ([`4f326bc`](https://github.com/GitoxideLabs/gitoxide/commit/4f326bc261c4e7f0d5510df74ad4215da3580696))
    - Use crc32fast instead of `crc` ([`11955f9`](https://github.com/GitoxideLabs/gitoxide/commit/11955f95e200ef75e752a833952d288fbd0fc389))
    - Put crc functionality behind a feature toggle ([`458fa6e`](https://github.com/GitoxideLabs/gitoxide/commit/458fa6ec726ec7901c1f6d970cbb1c1ea975dded))
    - (cargo-release) version 0.14.0 ([`a760f8c`](https://github.com/GitoxideLabs/gitoxide/commit/a760f8c013e13ba82daa1acf1a4a57e0818a008d))
    - Upgrade to prodash 13/tui 0.15 ([`1c99f51`](https://github.com/GitoxideLabs/gitoxide/commit/1c99f51b35b4ba85792a3b32dbb7e48052facc5e))
    - (cargo-release) version 0.3.0 ([`e9665c7`](https://github.com/GitoxideLabs/gitoxide/commit/e9665c784ae7e5cdaf662151395ee2355e9b57b6))
    - Allow calling 'finalize()' on the entries iterator ([`3c617bc`](https://github.com/GitoxideLabs/gitoxide/commit/3c617bc2ae59adbb12c254308269e745149d462b))
    - Git-odb without cargo warnings due to using the same test twice ([`8945f95`](https://github.com/GitoxideLabs/gitoxide/commit/8945f95364b489e7a639d74dd0f28b17e82e70f3))
    - Fix compile warning for git-features ([`d457faa`](https://github.com/GitoxideLabs/gitoxide/commit/d457faac6bb56a229b74147c8a4cf2484026bb1a))
    - Fix doc links ([`870af2a`](https://github.com/GitoxideLabs/gitoxide/commit/870af2a6949bcb1f7f45bc0ff98d9e9a07014b22))
    - Run git-odb tests in parallel, too; improved threaded error handling ([`40802fd`](https://github.com/GitoxideLabs/gitoxide/commit/40802fd8bbb15b8a61249522d67f3a5b28da64b3))
    - Refactor ([`82c2f42`](https://github.com/GitoxideLabs/gitoxide/commit/82c2f428e22c3cda79913c9ca2f092c377d692aa))
    - Refactor ([`7a6b514`](https://github.com/GitoxideLabs/gitoxide/commit/7a6b514a5b9b93bf574cd3a114f27ad5967e89ac))
    - Refactor ([`5ef1f22`](https://github.com/GitoxideLabs/gitoxide/commit/5ef1f22c1e12ff8d607663d4dfbbbfe426a29e0f))
    - Fix docs #(67) ([`01db10a`](https://github.com/GitoxideLabs/gitoxide/commit/01db10a27431ad89a68ed3e4eabae810748a6f29))
    - Refactor ([`3e908bd`](https://github.com/GitoxideLabs/gitoxide/commit/3e908bd4b4077c4a5d113cefc113f9d71f249133))
    - Refactor ([`409d763`](https://github.com/GitoxideLabs/gitoxide/commit/409d763d2fca974a647487c72d15f568a9b62ccb))
    - Refactor ([`896ab94`](https://github.com/GitoxideLabs/gitoxide/commit/896ab940bcd475d026e4009b3aa2fa6a025c14bc))
    - Remove unused dependency ([`26beb2a`](https://github.com/GitoxideLabs/gitoxide/commit/26beb2a5ad87e173fd3d13d17b0e9676a650cac9))
    - Don't finish the computation on drop of SteppedReduce ([`6453633`](https://github.com/GitoxideLabs/gitoxide/commit/6453633f1420327aee07dca2ad27abd8f96108c0))
    - Thanks clippy ([`c320761`](https://github.com/GitoxideLabs/gitoxide/commit/c320761821b08946a2b37e219400ded853a86408))
    - Remove unsafe interface for stepped computation #(67) ([`c856613`](https://github.com/GitoxideLabs/gitoxide/commit/c856613a35aea7dea1d093bfcfe1ddbde93fdf26))
    - A first working version of a static parallel iterator #(67) ([`d7d5c68`](https://github.com/GitoxideLabs/gitoxide/commit/d7d5c6855a038a8b01571a6a16a61fe0d8036d30))
    - A way iteration won't work with 'static #(67) ([`6fda1f2`](https://github.com/GitoxideLabs/gitoxide/commit/6fda1f20a57b9dcc1a5818d8d0b656218b383230))
    - Sketch of machinery for producing pack entries #(67) ([`ac8e7fb`](https://github.com/GitoxideLabs/gitoxide/commit/ac8e7fb6c8ae4ac42f56482d9d7744aa66132702))
    - Less restrictive requirements: Clone instead of Copy #(67) ([`410e7d6`](https://github.com/GitoxideLabs/gitoxide/commit/410e7d64049b5a749113126f5412a61ae4b79887))
    - Improve Safety docs #(67) ([`15e4748`](https://github.com/GitoxideLabs/gitoxide/commit/15e47480054d9a517c28f47db3b5fa87968a307e))
    - A test to assure referenced input and references in 'consume' work #(67) ([`4526d82`](https://github.com/GitoxideLabs/gitoxide/commit/4526d82fab4d6e8f2ab05497aa5893d5a8f8b253))
    - Make iterator creation unsafe and document why #(67) ([`593d5df`](https://github.com/GitoxideLabs/gitoxide/commit/593d5df478e67e28f9b3d48b201ff6830208726f))
    - First seemingly working version of an iterator which allows controlling threaded work #(67) ([`4a7ef7d`](https://github.com/GitoxideLabs/gitoxide/commit/4a7ef7d6398c2ff5dd6aac41f8224cd2d61ee189))
    - Make the parallel SteppedReduce compile #(67) ([`017fdf4`](https://github.com/GitoxideLabs/gitoxide/commit/017fdf48972a6a09e5155bd76bd437d8e195dae3))
    - More docs to differentiate SteppedReduce from in_parallel() #(67) ([`153c083`](https://github.com/GitoxideLabs/gitoxide/commit/153c0837bbf1df3b5cb386e08265f9b06eaee2a9))
    - Serial version of SteppedReduce seems to be working #(67) ([`779542e`](https://github.com/GitoxideLabs/gitoxide/commit/779542e4f4c951e9b16d2310146020da9ce36859))
    - Only store thread state #(67) ([`0bf8a9b`](https://github.com/GitoxideLabs/gitoxide/commit/0bf8a9b3c4a086732ee04f81c6a214296d49eab9))
    - Sketch instantiation of iterator adapter #(67) ([`a3083ad`](https://github.com/GitoxideLabs/gitoxide/commit/a3083ad3aad7984afc6b6d343ca7453f79897062))
    - A reducer test in preparation for allow it to be used as iterator #(67) ([`1c2adf4`](https://github.com/GitoxideLabs/gitoxide/commit/1c2adf4a546273489bf8224eb7982dbdf3fb6aca))
    - (cargo-release) version 0.13.0 ([`ac2eddb`](https://github.com/GitoxideLabs/gitoxide/commit/ac2eddb06eb3d8a9a3dcdcd796eb54a7e45ab935))
    - Allow parallel reducers to produce something during 'feed()' #(67) ([`6c04fcd`](https://github.com/GitoxideLabs/gitoxide/commit/6c04fcd643083d9db633edd3bb838b4f5de8f0db))
    - (cargo-release) version 0.12.0 ([`3b71e7e`](https://github.com/GitoxideLabs/gitoxide/commit/3b71e7e8416e550b47e5aed2259c1181497ac9e8))
    - (cargo-release) version 0.2.0 ([`4ec09f4`](https://github.com/GitoxideLabs/gitoxide/commit/4ec09f4d2239ea1d44f7145027e64191bf2c158c))
    - Refactor ([`dee8c66`](https://github.com/GitoxideLabs/gitoxide/commit/dee8c66e300dc2a2b6e1a6d6c3674a7ce6aac687))
    - (cargo-release) version 0.11.0 ([`1aa1f5e`](https://github.com/GitoxideLabs/gitoxide/commit/1aa1f5e84a07427d5d7f3231735fe9c1923f506f))
    - (cargo-release) version 0.10.1 ([`0dcdfd7`](https://github.com/GitoxideLabs/gitoxide/commit/0dcdfd754649240f43fe0f4b6e1245e8c7b89635))
    - Remove usage of gitfeatures::fs in organize subcommand ([`b567d37`](https://github.com/GitoxideLabs/gitoxide/commit/b567d3709a74e9fdafef54b0fe58ca82721cd773))
    - Assure basic 'organize' operation is working as expected ([`deb6073`](https://github.com/GitoxideLabs/gitoxide/commit/deb6073671ae95de674aaef7ca01e03f95b41ca8))
    - A first stab at finding git repositories ([`e4dc964`](https://github.com/GitoxideLabs/gitoxide/commit/e4dc96403894f1fe509335905679347ecdf535c7))
    - Upgrade 'jwalk' ([`cba048f`](https://github.com/GitoxideLabs/gitoxide/commit/cba048f094858388f4242e37a2409fe0822f8c07))
    - Upgrade 'bytes' ([`3934392`](https://github.com/GitoxideLabs/gitoxide/commit/39343922b4a1129394aa788a9591920aee077569))
    - Upgrade prodash and friends ([`50755bc`](https://github.com/GitoxideLabs/gitoxide/commit/50755bc83f73072dc629301bf69c5c065d5c2aa4))
    - Add missing '.' at end of doc comments ([`7136854`](https://github.com/GitoxideLabs/gitoxide/commit/71368544f97369a4d371d43513607c4805bd0fd0))
    - Use git-hash in git-features ([`5b307e0`](https://github.com/GitoxideLabs/gitoxide/commit/5b307e076f6f5975592c8b177c122c91c1d809c6))
    - (cargo-release) version 0.9.0 ([`a89fdb9`](https://github.com/GitoxideLabs/gitoxide/commit/a89fdb98f64bb0ca070fa79a1f58f1232bb14090))
    - (cargo-release) version 0.5.0 ([`fc7d600`](https://github.com/GitoxideLabs/gitoxide/commit/fc7d600ac2c438c8b6b91f67cb69b0ac5ec37675))
    - More docs for owned git-object ([`b79101d`](https://github.com/GitoxideLabs/gitoxide/commit/b79101d714f59a42a30eb47776486a212ec0f738))
    - Fix io::pipe tests ([`9604154`](https://github.com/GitoxideLabs/gitoxide/commit/9604154e687813a11f0eee469e408561a6a74a4e))
    - Uograde everything else ([`0cd79d0`](https://github.com/GitoxideLabs/gitoxide/commit/0cd79d00bce3f042b5cc849cf48739e29f95fcb0))
    - Upgrade prodash and tui ([`b5eadca`](https://github.com/GitoxideLabs/gitoxide/commit/b5eadca343bbaa1af86722b5f1bcd33f4e3939a6))
    - Add remaining docs to git-features using the missing_docs directive ([`f8aafd6`](https://github.com/GitoxideLabs/gitoxide/commit/f8aafd6c78687899a2ca3a3e6147d93fc45b8cb9))
    - (cargo-release) version 0.8.0 ([`47c00c2`](https://github.com/GitoxideLabs/gitoxide/commit/47c00c2228cf25c79e1fa3eb4229c7ab24de91e5))
    - Finish git-features documentation ([`934a26c`](https://github.com/GitoxideLabs/gitoxide/commit/934a26c5e254baf2be9178096b6dead0e4c1ed1d))
    - Refactor ([`b3a8bb5`](https://github.com/GitoxideLabs/gitoxide/commit/b3a8bb5f7f0c6e80259922546928c2739c24f7b5))
    - Refactor ([`f9e8d29`](https://github.com/GitoxideLabs/gitoxide/commit/f9e8d2932c02c22bf57acd39fb0a9e6d521070bd))
    - Docs for the git-features::pipe module ([`67a950a`](https://github.com/GitoxideLabs/gitoxide/commit/67a950a2e0fd56b29565668ed0a0f399d5aa989d))
    - Document git-features::parallel ([`b899227`](https://github.com/GitoxideLabs/gitoxide/commit/b8992275cd4310b05494be41c059e9b6049d06b1))
    - Dependency update ([`fb077f9`](https://github.com/GitoxideLabs/gitoxide/commit/fb077f9fecb89ed8a60d57b45726401883e838bf))
    - Finish git_features::interrupt docs ([`471a1bf`](https://github.com/GitoxideLabs/gitoxide/commit/471a1bf24efee70f21b15839cdc9f8ebe319f917))
    - Dependency update ([`b3b4aba`](https://github.com/GitoxideLabs/gitoxide/commit/b3b4aba5e05596befecd17e225067be9315b74fd))
    - Docs for git-features::hash ([`a3fdecc`](https://github.com/GitoxideLabs/gitoxide/commit/a3fdecc9a3587b20c01e3b3a2d51190138131c3d))
    - First sketch of filesystem docs for git-features ([`1a8141c`](https://github.com/GitoxideLabs/gitoxide/commit/1a8141c2c4a8bcc79d68049a35bd8aba5ab822a3))
    - (cargo-release) version 0.7.0 ([`7fa7bae`](https://github.com/GitoxideLabs/gitoxide/commit/7fa7baeb3e7d008a25e4d714eff908e2516c828b))
    - Merge branch 'commit-graph' into main ([`9cb09b2`](https://github.com/GitoxideLabs/gitoxide/commit/9cb09b248796f0ff5c9d3f3e857de4731324cfd5))
    - Specify the hash to create with 'hash::bytes_of_file' ([`c000294`](https://github.com/GitoxideLabs/gitoxide/commit/c000294423ae0759b978399db3b69ac07c20578d))
    - Move 'git_odb::hash::bytes_of_file' into git_features::hash ([`c5f6b45`](https://github.com/GitoxideLabs/gitoxide/commit/c5f6b4587ee4042a080c0505613b0c72fdfe5273))
    - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
    - Merge branch 'main' into commit-graph ([`ca5b801`](https://github.com/GitoxideLabs/gitoxide/commit/ca5b80174b73cc9ac162b3f33b5d3721ef936cb1))
    - Use parallel walkdir (via jwalk) when parallel feature is enabled ([`f444c85`](https://github.com/GitoxideLabs/gitoxide/commit/f444c859f5b215ea70a46d5493a2babbf7a98235))
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Refactor ([`e4bcfe6`](https://github.com/GitoxideLabs/gitoxide/commit/e4bcfe6406b14feffa63598c7cdcc8ecc73222bd))
    - (cargo-release) version 0.6.0 ([`9ef184e`](https://github.com/GitoxideLabs/gitoxide/commit/9ef184e35712f938fb4f9f6da7390a8777a9284e))
    - Switch to prodash 10 and safe a lot of trait bounds in the process ([`e2fb1d9`](https://github.com/GitoxideLabs/gitoxide/commit/e2fb1d944b4d803a11c91f868b831d406fb5e35f))
    - (cargo-release) version 0.5.0 ([`82b7313`](https://github.com/GitoxideLabs/gitoxide/commit/82b73131b79ec3c42a712dad1c0766a72209d737))
    - [clone] This actually works: first MVP of retrieving packs via clone ([`c06d819`](https://github.com/GitoxideLabs/gitoxide/commit/c06d8194173f9ec468ddd0faf72dd6d8dbf7d35d))
    - [clone] test (and fix) for piped line reading ([`afe2996`](https://github.com/GitoxideLabs/gitoxide/commit/afe2996689b5bea915ac5f142d320056faf49899))
    - [clone] Send headers with BufReaders ([`6a95aaa`](https://github.com/GitoxideLabs/gitoxide/commit/6a95aaab582941c6d1697dde6982c0aa8896c73d))
    - [clone] pipe allows to send errors as well ([`69286ec`](https://github.com/GitoxideLabs/gitoxide/commit/69286ecb3680b5071693ef0d9fb2e9345b2722d4))
    - [clone] BufRead for Reader… ([`bf1d40f`](https://github.com/GitoxideLabs/gitoxide/commit/bf1d40f2d44a9b04ffe2134ddcd3779985cdafc4))
    - [clone] a piped iterator ([`5148c85`](https://github.com/GitoxideLabs/gitoxide/commit/5148c85efc70c0ec06be3ebce267ce727c8ee4e1))
    - [clone] pipe probably shouldn't abort on empty writes ([`9cfa9b7`](https://github.com/GitoxideLabs/gitoxide/commit/9cfa9b79841187167f0f96abfd1c17a37b4c365d))
    - Thanks clippy ([`c4f570f`](https://github.com/GitoxideLabs/gitoxide/commit/c4f570fcae7e21745a37a4265b05d21e6149157b))
    - [clone] more pipe tests ([`1652a74`](https://github.com/GitoxideLabs/gitoxide/commit/1652a74761631cadfc6feab366adc0808d83063d))
    - [clone] first working pipe implementation ([`490a9b9`](https://github.com/GitoxideLabs/gitoxide/commit/490a9b96915a760e339e576d9f49737b43a8739f))
    - [clone] frame for implementing 'pipe' support ([`c555681`](https://github.com/GitoxideLabs/gitoxide/commit/c55568127ff943cc6749dba5054d7b3e93c049eb))
    - Fix git-features hash tests ([`35e8809`](https://github.com/GitoxideLabs/gitoxide/commit/35e8809f6bc7d19ed9e0bac8e3af85f433978901))
    - Bump git-features to 0.4 to allow publishes after breaking changes ([`9d6b879`](https://github.com/GitoxideLabs/gitoxide/commit/9d6b8790e2edd7fa01b3239adff86a7cd2393f10))
    - [protocol] properly implement remote progress reporting ([`a81954a`](https://github.com/GitoxideLabs/gitoxide/commit/a81954a6a37afacd51add6661a656b8fb663ca54))
    - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
    - Add 'disable-interrupts' feature flag ([`ccd9c3e`](https://github.com/GitoxideLabs/gitoxide/commit/ccd9c3e2d37aa6898dc17f47a82c187baa810b03))
    - Refactor ([`b4a6e16`](https://github.com/GitoxideLabs/gitoxide/commit/b4a6e16364822c0dccb56f98dbfb0ca4c8007069))
    - Bump minor version to 0.3 ([`4351e28`](https://github.com/GitoxideLabs/gitoxide/commit/4351e2871c9dcf342b8471fffa74cae338a53269))
    - Thanks clippy ([`6725104`](https://github.com/GitoxideLabs/gitoxide/commit/6725104d2841e6518db641d06e3e107cf4f40f96))
    - First step towards parallelizing file hashes and traversal! ([`9573836`](https://github.com/GitoxideLabs/gitoxide/commit/95738369e0d3accf7f6239c8cd966a7f5c36825a))
    - Better usability for units ([`b226253`](https://github.com/GitoxideLabs/gitoxide/commit/b226253636d8146a084a7bcd7c0c320e37f9d2fb))
    - Update dependencie ([`ade06b4`](https://github.com/GitoxideLabs/gitoxide/commit/ade06b46bb3c16ac1e26dbbb4a7045f0c09f2d8e))
    - Make obvious that interrupt request was received ([`34b2373`](https://github.com/GitoxideLabs/gitoxide/commit/34b23737f560fe52d4f98fb886eba754652f9a5e))
    - Remove once_cell dependency as it is really not required anymore ([`5ac9538`](https://github.com/GitoxideLabs/gitoxide/commit/5ac95385cc8d1c50c16da6e5fb0c66ac138f9966))
    - Make interrupt handler work reliably ([`e71da0f`](https://github.com/GitoxideLabs/gitoxide/commit/e71da0fce6d6eab68f7b81b13cdc78ce8e9b7ee3))
    - Conditionally use an eager iterator… ([`e9b5511`](https://github.com/GitoxideLabs/gitoxide/commit/e9b5511568f4e64968596994855783f19672d678))
    - Refactor ([`d14f0f6`](https://github.com/GitoxideLabs/gitoxide/commit/d14f0f6c3b5f303df75b33aadbf16653075d2272))
    - Allow eager iterator to behave properly when used with index writing ([`66ebc5f`](https://github.com/GitoxideLabs/gitoxide/commit/66ebc5f1ad5f262eb464dc7ca0892ec952d34382))
    - First successful test of moving the streaming iterator into its own thread ([`c9fcb68`](https://github.com/GitoxideLabs/gitoxide/commit/c9fcb68c644c96a15cb9956a754bec7b65bb5fbd))
    - Now it's order preserving ([`4c8711e`](https://github.com/GitoxideLabs/gitoxide/commit/4c8711e51efd88e0f159ad02de2692c4cb72ce27))
    - First sketch of order-destroying eager iterator ([`20fca45`](https://github.com/GitoxideLabs/gitoxide/commit/20fca4515f6e9ea320d0bf21c15cd6d2c3cff742))
    - Print read throughput automatically ([`0a71b48`](https://github.com/GitoxideLabs/gitoxide/commit/0a71b482310a129aa8757475290b3b24a200b702))
    - Make sure interrupt logic works even without an interrupt handler… ([`66b1644`](https://github.com/GitoxideLabs/gitoxide/commit/66b164472f5893f9e634ac1f9147a41dc742296d))
    - Add percentage and throughput to tasks that matter ([`763d7ca`](https://github.com/GitoxideLabs/gitoxide/commit/763d7caa4c70111b7cb3ef5733d2c3c697758c28))
    - Upgrade to latest iteration of prodash ([`3a4faec`](https://github.com/GitoxideLabs/gitoxide/commit/3a4faecab56e37670c553e6563f11a46d740c333))
    - First part of migration to prodash 8.0, but… ([`6901a09`](https://github.com/GitoxideLabs/gitoxide/commit/6901a098641820c8d974ce56a24d6cdca779730d))
    - Thanks clippy ([`ed5882d`](https://github.com/GitoxideLabs/gitoxide/commit/ed5882d75e0a9fceb0628e84302eb49a66277fa6))
    - Write about user interfaces and the use/non-use of async ([`91ba045`](https://github.com/GitoxideLabs/gitoxide/commit/91ba0457745f860b7a68cb38b13e69754747e8d9))
    - Interrupt support for pretty plumbing ([`bca7ce2`](https://github.com/GitoxideLabs/gitoxide/commit/bca7ce2e668a4be2600d2d04d00f46b21c82eee2))
    - Support for interruptible operations ([`a025593`](https://github.com/GitoxideLabs/gitoxide/commit/a02559378f9165df97a217f24834a851be719b08))
    - Refactor ([`413968d`](https://github.com/GitoxideLabs/gitoxide/commit/413968dfee5e5a66ed9e63823f6bda5a5a22753e))
    - Receive progress information when reading packs in bundle ([`759091d`](https://github.com/GitoxideLabs/gitoxide/commit/759091d3c6696b427d7b5aab1b6da05a0d268c04))
    - Initial batch of progress usage for index creation… ([`b10e5c6`](https://github.com/GitoxideLabs/gitoxide/commit/b10e5c664be9bd1bdb2b72b858ebaf35c1ed4cb4))
    - First stab at streaming pack header encoding ([`3c6e78b`](https://github.com/GitoxideLabs/gitoxide/commit/3c6e78bec9cbd4df842919cc8dc3c575414ed002))
    - We can now restore (possibly half-written) packs ([`b1daa46`](https://github.com/GitoxideLabs/gitoxide/commit/b1daa465c40ea8c7c9de69a18e467d69459d911e))
    - See how big a Sha1 hasher really is ([`26b271d`](https://github.com/GitoxideLabs/gitoxide/commit/26b271d44863fb184b0a947c3a9da2b3252f9a78))
    - First sketch of new verify expressed in terms of traversal ([`4cb570f`](https://github.com/GitoxideLabs/gitoxide/commit/4cb570f96ddd7ee2faa62e54927afd78ba7822af))
    - (cargo-release) version 0.2.0 ([`0bb8314`](https://github.com/GitoxideLabs/gitoxide/commit/0bb831480d8657e1bb29ee7009aeac673471403e))
    - Incorporate dynamic chunking into 'less-time' algorithm ([`295aa2f`](https://github.com/GitoxideLabs/gitoxide/commit/295aa2f01dc596a8880cd2f68a8d83bc6913ce48))
    - Integrate new chunk size code into lookup code ([`a8422cf`](https://github.com/GitoxideLabs/gitoxide/commit/a8422cf0b0c9ff4d3275cc17a68a74811b5bd01f))
    - First round of number tuning done ([`a647b2d`](https://github.com/GitoxideLabs/gitoxide/commit/a647b2da2905c4079e646ea44cbec778f3f7c71f))
    - Somehow handle chunk size in absence of known chunk amount ([`acfccad`](https://github.com/GitoxideLabs/gitoxide/commit/acfccadef40ebcc67f8dea4e58c02392b7e2e7de))
    - Chunk computation seems alright, what about realistic values ([`973e6bb`](https://github.com/GitoxideLabs/gitoxide/commit/973e6bb3d67d89eec2faf2467a129d992b90ed72))
    - Getting there… ([`a1b5d56`](https://github.com/GitoxideLabs/gitoxide/commit/a1b5d565f305f0f2666fd59272d9bf9c62ae2962))
    - First step towards computing better chunk sizes and thread limits ([`1cdde7d`](https://github.com/GitoxideLabs/gitoxide/commit/1cdde7d339a6ed3650c54f9b48154089d7da9919))
    - Add 'inc()' convenience methods to progress ([`2e46c9b`](https://github.com/GitoxideLabs/gitoxide/commit/2e46c9b72a2a5b90bcdac249de07ffbc124cfb04))
    - (more) graceful shutdown of failing parallel tasks ([`163f50f`](https://github.com/GitoxideLabs/gitoxide/commit/163f50fab81b425e6e306ec54fb1eb60a7c02cf8))
    - Respect thread limit in 'in_parallel' ([`babfd84`](https://github.com/GitoxideLabs/gitoxide/commit/babfd84cba77ef7a0f541ba921b31ebd3f3c50e3))
    - Make crates publishable ([`5688a34`](https://github.com/GitoxideLabs/gitoxide/commit/5688a3427ff3673e1422d43106f4d685fa837aed))
    - Flume isn't actually needed for that… ([`c750022`](https://github.com/GitoxideLabs/gitoxide/commit/c750022394928aa37a8400611f6fdf4ee77c0f69))
    - Don't just ignore send errors - we should panic for now ([`f128117`](https://github.com/GitoxideLabs/gitoxide/commit/f128117138b24de780a00bb96e7c1c9f987e8aa0))
    - Proper implementation of line renderer into 'lean' CLI ([`e98e7c2`](https://github.com/GitoxideLabs/gitoxide/commit/e98e7c280d73e9d9ebd13202afb93a56cb2f7c9c))
    - Upgrade to prodash version 7 ([`af02b46`](https://github.com/GitoxideLabs/gitoxide/commit/af02b46cc1eff5ba1da7da20d3f524a79fad686f))
    - Update prodash to verion 6.0 ([`a4731a3`](https://github.com/GitoxideLabs/gitoxide/commit/a4731a3aca159f8916b29d9ce5a71856089c5a6b))
    - Add metadata to allow docs.rs build all featueres ([`10f9386`](https://github.com/GitoxideLabs/gitoxide/commit/10f9386a12decc1f13999aee72be484c8f6d48ce))
    - Switch to prodash 5.0 for windows support ([`88542e1`](https://github.com/GitoxideLabs/gitoxide/commit/88542e117dd1c2e7606fcbe88b30c51b4c115989))
    - Allow to limit the logging depth for less cluttered output ([`fce7035`](https://github.com/GitoxideLabs/gitoxide/commit/fce703531d7006f7d961d6ffa66f51f6c9bc0efc))
    - Finally speed up logging progress properly - needs input throttling ([`1a550c6`](https://github.com/GitoxideLabs/gitoxide/commit/1a550c6458b10fad2e42b641899216c5517c6e26))
    - Avoid calling system time too often in logs, it reduced performance ([`b17bd76`](https://github.com/GitoxideLabs/gitoxide/commit/b17bd76d35822b3af174c74af3d6fac887889fe2))
    - Revert "ABORT: try-join with static typing works, but…" ([`b8b979b`](https://github.com/GitoxideLabs/gitoxide/commit/b8b979b99b5f3848e0a6884c58594ba2b481a147))
    - Try-join with static typing works, but… ([`ab6f98b`](https://github.com/GitoxideLabs/gitoxide/commit/ab6f98b905f13ed2a7c0c483f34fab63141fbc5b))
    - Remove dependency to git-object from git-features - it better remains free ([`67c3a6a`](https://github.com/GitoxideLabs/gitoxide/commit/67c3a6ab4cc32358a1406c2f863e26a4c2929867))
    - \#[forbid(unsafe)] for all crates ([`afda803`](https://github.com/GitoxideLabs/gitoxide/commit/afda8039259b7a30cfed5dbcdd9caf4773b4c234))
    - Allow for more screen space when formatting ([`6794300`](https://github.com/GitoxideLabs/gitoxide/commit/67943002e7f4215b5383bd0538786ce2857f011e))
    - Refactor ([`7add82c`](https://github.com/GitoxideLabs/gitoxide/commit/7add82c39169e3c2fff76c48cdd318fe6040d7bc))
    - Automatically close the TUI when there is no progress anymore. ([`c416152`](https://github.com/GitoxideLabs/gitoxide/commit/c416152b04051958de7bd161a8a2ee42ca163275))
    - Pretty progress in a generalized form ([`caa883b`](https://github.com/GitoxideLabs/gitoxide/commit/caa883b96827deb63b5c8787ed820d22f2c85249))
    - Express DoOrDiscard in terms of Either (progress) ([`cb29a45`](https://github.com/GitoxideLabs/gitoxide/commit/cb29a45f4e73bfaa25cbf623b1cda2435673028b))
    - Provide 'either' type with implementation for Progress ([`237bb5e`](https://github.com/GitoxideLabs/gitoxide/commit/237bb5ee1c2b677f5bfd9ca7fdea9d9d2db865b3))
    - Better trait bounds of `in_parallel_if`… ([`6264f2f`](https://github.com/GitoxideLabs/gitoxide/commit/6264f2f99929ffaa4d50cdcae7bc296e1b4762f4))
    - First implementation of logging per thread ([`477dd90`](https://github.com/GitoxideLabs/gitoxide/commit/477dd90ce5e102875b19489bf8ae9877522ef9c8))
    - Support for providing progress to threads ([`2815858`](https://github.com/GitoxideLabs/gitoxide/commit/2815858adf7ac0f7b4cbc88cf05df0ea6aef4116))
    - First very basic progress implementation ([`b820717`](https://github.com/GitoxideLabs/gitoxide/commit/b8207177daee8a9ffa23c7c052cf9ca651b15804))
    - Pass progress everywhere, for now just to discard it ([`da3ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/da3ae1c82cd726b8fae9b8d26069719930e9ba99))
    - Implement `Progress` trait for prodash::tree::Item ([`0eeb6d7`](https://github.com/GitoxideLabs/gitoxide/commit/0eeb6d770d58621427bc88107a20860b89b86a24))
    - Implement progress trait for logs with throttling ([`287eca9`](https://github.com/GitoxideLabs/gitoxide/commit/287eca91b244ccbc703cb275b1ae032bfeb02532))
    - Add 'fast-sha1' to git-features ([`b22541f`](https://github.com/GitoxideLabs/gitoxide/commit/b22541f0c39af470877119b136e4eb1b82dff2db))
    - A new crate to represent features that can toggle from the top-level ([`23c420c`](https://github.com/GitoxideLabs/gitoxide/commit/23c420cc95219dc7c04d3905aaa03281cb51724e))
</details>

## 0.26.3 (2023-02-14)

### New Features

 - <csr-id-426057247a80821b3da22b4ae5d67bda89ce0631/> re-export `prodash` in `progress` module.
   That way one can access all types even if they are not re-exported.

## 0.26.2 (2023-02-09)

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### New Features

 - <csr-id-c4a7634b0b29c74625e183953e59c65987e9d66c/> export `prodash::progress::Id` in the `progress` module for convenience.

## 0.26.1 (2023-01-10)

A maintenance release without user-facing changes.

## 0.26.0 (2022-12-30)

<csr-id-5bf0034fb3918e57562b7089ceba83d63a1854bf/>

### Chore (BREAKING)

- <csr-id-5bf0034fb3918e57562b7089ceba83d63a1854bf/> upgrade to prodash v23

## 0.25.1 (2022-12-26)

### New Features

 - <csr-id-25ad372bf500b851105f53b10369b5a689ba167e/> zlib::inflate::Error can now represent zlib status codes that represent failure.

## 0.25.0 (2022-12-19)

### New Features (BREAKING)

 - <csr-id-0f27c67c92fc0bc23a6712b5c4c730ad6a0156bf/> add support for explicit non-parallel iteration.
   That way we can allow the implementation to choose whether they
   need greatest speed at some cost or not.
   
   This also allows us to create a new thread-pool on each iteration
   as those who expect high cost or many files will likely chose to do
   that instead of single-threaded iteration, which nicely contains the
   threads needed and avoids keeping them alive as part of some global pool.

## 0.24.1 (2022-11-27)

### New Features

 - <csr-id-6d530a1dc77f0f4ac00622a2fd47c7bdb731a77a/> name spawned threads
   That way it's a bit more obvious what's happening when the CPU goes
   up in flames.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.

## 0.24.0 (2022-11-21)

A maintenance release without user facing changes.

## 0.23.1 (2022-11-06)

### New Features

 - <csr-id-9076ce33ec167e425a0163d3e40a81a3fd0db6cd/> `fs::Snapshot` can `Clone` if `T` can `Clone`.

## 0.23.0 (2022-10-10)

### New Features

 - <csr-id-a7c11d2cb5f88a4ff322d9a9848459062790d8b3/> perfect granularity for threads processing with `in_parallel_with_slice()`
 - <csr-id-ff1db66f2dad3afc8bc77610006bca9fea5947d2/> add `progress::Step|StepShared` as types of `prodash`
   This may help to use the `Progress::counter()` method as part of method
   signatures, being an `Option<progress::StepShared>`.

### Changed (BREAKING)

 - <csr-id-38446dc8824afef30ef121598de3451d13b9262c/> remove `fs-jwalk-single-threaded` in favor of `fs-walkdir-parallel`.
   This way, `jwalk` and the dependencies (and troubles) it brings have to
   be opted in, but also allow other users to actually opt out while
   allowing the `parallel` feature to be in effect.
   
   In other words, previously the `parallel` feature conflated `jwalk`
   dependencies into the tree, which isn't the case anymore.

### New Features (BREAKING)

 - <csr-id-3b29fc18672c0176684c797a0f16f85d09369bf8/> make jwalk fully optional

## 0.22.5 (2022-09-20)

### New Features

 - <csr-id-cfe46b502afc3ecb312849ddbd7748007d432cd1/> add zlib-ng feature to allow linking against system libz-ng
   Allow to use zlib-ng (zlib-ng-sys) with native API (no compat mode)
   that can co-exist with system libz (loaded by e.g. libcurl).
   This is used in gitoxide package on Alpine Linux.

## 0.22.6 (2022-09-16)

Fix docs.rs rendering.

### New Features

 - <csr-id-cfe46b502afc3ecb312849ddbd7748007d432cd1/> add zlib-ng feature to allow linking against system libz-ng
   Allow to use zlib-ng (zlib-ng-sys) with native API (no compat mode)
   that can co-exist with system libz (loaded by e.g. libcurl).
   This is used in gitoxide package on Alpine Linux.

## 0.22.4 (2022-09-04)

A maintenance release without breaking changes.

## 0.22.3 (2022-08-27)

### Fix

- restrict `sha1` `asm` to supported archs ([`b383fab`](https://github.com/Byron/gitoxide/commit/b383fabbe10868317be51b99cfdd9b0981816042))

## 0.22.2 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

## 0.22.1 (2022-08-15)

### New Features

 - <csr-id-f498d35baba52e40ecd47381e87c1ce49cf13285/> add `fs-jwalk-single-threaded` feature to specifically decouple `jwalk` from rayon
   It has been an issue in https://github.com/starship/starship/issues/4251
   apparently and rayon interactions can be difficult.
 - <csr-id-7f199f0e5246809efde9880110093fbd11a4f8fe/> `fs::Snapshot` to on-demand reload shared resources.

## 0.22.0 (2022-07-22)

### New Features

 - <csr-id-c76fde7de278b49ded13b655d5345e4eb8c1b134/> initialize `Time` from `now_utc` and `now_local`
   Localtime support depends on some other factors now, but that
   will only get better over time.
   
   We might have to document `unsound_local_time` at some point.

### Changed (BREAKING)

 - <csr-id-89a41bf2b37db29b9983b4e5492cfd67ed490b23/> remove local-time-support feature toggle.
   We treat local time as default feature without a lot of fuzz, and
   will eventually document that definitive support needs a compile
   time switch in the compiler (`--cfg unsound_local_offset` or something).
   
   One day it will perish. Failure is possible anyway and we will write
   code to deal with it while minimizing the amount of system time
   fetches when asking for the current local time.

## 0.21.1 (2022-06-13)

A maintenance release without user-facing changes.

## 0.21.0 (2022-05-18)

### Changed (BREAKING)

 - <csr-id-90611ce1527618bcc738440bfc1ccc7a45319974/> remove `path` module in favor of `gix-path` crate

### New Features (BREAKING)

 - <csr-id-d078d6ee76a80d1dfaf71608c12d8a402bd670d4/> mild refactor of paths module to waste less on unix
   Previously it might have performed find-and-replace on unix paths even
   though they wouldn't have changed afterwards, yet costing an allocation.
   
   There is also the realization that it should go into its own crate to have
   neater import paths and more convenience.

## 0.20.0 (2022-04-02)

### New Features

 - <csr-id-e4d6685064ad2b433f8acd3a74b320bf0169a994/> Add `gix_config::values::Path` for a typesafe git path
   Add a `Path` type to the `gix_config::values` which
   can be interpolated according to gits own path interpolation
   rules.
 - <csr-id-3c8581fc294c65c9eb42698969fe3263135a864e/> add new 'path' module for all path-related conversions
   It's meant to unify all path and byte related handling to help assuring
   encoding is handled correctly or at least similarly everywhere.
 - <csr-id-15ff212b17087de93f259e366f4e4b821cfbc28e/> in-manifest and in-lib documentation of feature toggles

### Bug Fixes

 - <csr-id-234cd10ca55482ce1a840ce3244308d249895bcc/> Assure std::io::copy() doesn't hang when we cause an interrupt
   The user can ask for interruptions which previously used the
   error kind Interrupted. This however has special meaning and
   usually means to retry.

### New Features (BREAKING)

 - <csr-id-8945d95f7fa88562d37ff67ac6e38bead73dd2df/> `interrupt::Iter`, rename `interrupt::Iter` -> `interrupt::IterWithError`

## 0.19.1 (2022-01-23)

<csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/>

A maintenance release thanks to upgrade to `prodash` 18.

### Changed (BREAKING)

 - <csr-id-61e5cfece4d8f405e35fc1957b00ce1da7526c26/> renamed `progress::Read::reader` -> `progress::Read::inner`

### New Features

 - <csr-id-cb7e4e784d615f9fa3d6fb9c36240f0592403358/> Add InOrderIter to 'parallel' module
   This iterator makes possible identifies results using a sequence id
   and returns only consecutive items.
   
   Use it to collect unordered results produced by threads.
   It's advantage to collecting yourself and sorting is the potential
   for a smaller memory footprint of in-flight results, one doesn't
   have to collect them all for ordering, necessarily.
 - <csr-id-ca095ed881db2a8f06a6b067dbaac17e923b0945/> Make a scope-like abstraction available
   This allows more delicate threading control like is required for the
   index.
 - <csr-id-b8400ed80543d67a5895c975ba9b1fc28427411c/> decoding of variable int numbers.
   It's here only so that we can share the code across crates, for now
   without any feature toggles.
 - <csr-id-0a749a22057b5513a8cefa0e26b0a9a268c769d3/> Add `progress::Write` to automatically pass bytes written to a progress instance

### Chore

- <csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/> update sha-1 dependency to 0.10

## 0.19.0 (2022-01-19)

<csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/>

### Chore

- <csr-id-361892ca15aa648802f6701ab6a5a30aedde3449/> update sha-1 dependency to 0.10

### New Features

 - <csr-id-cb7e4e784d615f9fa3d6fb9c36240f0592403358/> Add InOrderIter to 'parallel' module
   This iterator makes possible identifies results using a sequence id
   and returns only consecutive items.
   
   Use it to collect unordered results produced by threads.
   It's advantage to collecting yourself and sorting is the potential
   for a smaller memory footprint of in-flight results, one doesn't
   have to collect them all for ordering, necessarily.
 - <csr-id-ca095ed881db2a8f06a6b067dbaac17e923b0945/> Make a scope-like abstraction available
   This allows more delicate threading control like is required for the
   index.
 - <csr-id-b8400ed80543d67a5895c975ba9b1fc28427411c/> decoding of variable int numbers.
   It's here only so that we can share the code across crates, for now
   without any feature toggles.
 - <csr-id-0a749a22057b5513a8cefa0e26b0a9a268c769d3/> Add `progress::Write` to automatically pass bytes written to a progress instance

### Changed (BREAKING)

 - <csr-id-61e5cfece4d8f405e35fc1957b00ce1da7526c26/> renamed `progress::Read::reader` -> `progress::Read::inner`

## 0.18.0 (2021-11-29)

### New Features

 - <csr-id-7e95d8ab29051ffc892f2dcbaf5369e8c7e7b294/> add threading primitives with feature toggle
   If the `threading` feature is set, the `threading` module will contain thread-safe primitives
   for shared ownership and mutation, otherwise these will be their single threaded counterparts.
   
   This way, single-threaded applications don't have to pay for threaded primitives.

### Changed (BREAKING)

 - <csr-id-e7526b2a7b51cbac4018e1ab3b623a85987fadc2/> parallel utilities now use `Send + Clone` instead of `Send + Sync`
   This helps to assure that thread-local computations always work with the
   kind of types we provide. The ones that are carrying out actions are
   notably not `Sync` anymore.
   
   We cater to that by defining our bounds accordingly, but for those
   who want to use other utilities that need Sync, using types like
   `Repository` and `thread_local!()` is the only way to make this
   work.

## v0.17.0 (2021-10-19)

A maintenance release due to properly dealing with previously breaking changes in `gix-hash`.

## v0.16.5 (2021-10-15)

This release contains no functional changes.

## v0.16.4 (2021-09-07)

## v0.16.3 (2021-08-27)

## v0.16.2 (2021-08-17)

## v0.16.1 (2021-08-10)

## v0.16.0 (2021-08-10)

## v0.14.0 (2021-05-08)

## v0.13.0 (2021-04-30)

## v0.12.0 (2021-04-08)

## v0.11.0 (2021-01-24)

## v0.10.1 (2021-01-24)

## v0.10.0 (2020-12-16)

## v0.9.0 (2020-12-15)

## v0.8.0 (2020-11-26)

## v0.7.0 (2020-11-18)

## v0.6.0 (2020-09-14)

## v0.5.0 (2020-09-12)

## v0.4.0 (2020-08-18)

## v0.3.0 (2020-08-12)

## v0.2.0 (2020-07-23)

## v0.1.0 (2020-07-12)

<csr-id-ab6f98b905f13ed2a7c0c483f34fab63141fbc5b/>

### Other

- <csr-id-ab6f98b905f13ed2a7c0c483f34fab63141fbc5b/> try-join with static typing works, but…
  …seems like a lot of effort. Probably not worth continuing here

