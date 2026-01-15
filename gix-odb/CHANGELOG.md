# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2341 from GitoxideLabs/dependabot/cargo/cargo-cf4a2135ae ([`d914d95`](https://github.com/GitoxideLabs/gitoxide/commit/d914d9533ed2243658d51ba05e68dd444b75a748))
    - Bump the cargo group across 1 directory with 51 updates ([`4edc5dd`](https://github.com/GitoxideLabs/gitoxide/commit/4edc5dda7ca39cc8249cb98dc39ca46c7d00eb44))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.74.0 (2025-12-31)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
</details>

## 0.73.0 (2025-12-22)

### Bug Fixes (BREAKING)

 - <csr-id-1955b5b1a436518c036cfc9bf642170f9c7b6540/> expose raw commit/tag actor headers for round-tripping.
   Note that this means you have to call `CommitRef::commiter|author()?` and `TagRef::tagger()?` instead
   of assuming pre-parsed fields.
   
   This PR makes signature handling truly lossless for "creative" emails and other info.
   We now stash the raw name <email> slice on IdentityRef/SignatureRef and fall back to it when rewriting,
   so even commits with embedded angle brackets round-trip cleanly (might want to expand to other malformed characters before merging?
   Parsing and serialization honor that flag but still keep strict validation for normal input.
   I also added regression coverage for these scenarios.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 21 calendar days.
 - 29 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2253 from Pingasmaster/raw-email-attempt-fix ([`f471ac5`](https://github.com/GitoxideLabs/gitoxide/commit/f471ac5fece0baf3766d80482400f6b37db8edd0))
    - Expose raw commit/tag actor headers for round-tripping. ([`1955b5b`](https://github.com/GitoxideLabs/gitoxide/commit/1955b5b1a436518c036cfc9bf642170f9c7b6540))
</details>

## 0.72.0 (2025-11-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
</details>

## 0.71.1 (2025-10-23)

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

## 0.71.0 (2025-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release over the course of 99 calendar days.
 - 99 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0 ([`49f8d53`](https://github.com/GitoxideLabs/gitoxide/commit/49f8d5373501c82699fec76809f4d445c3327c5a))
    - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
    - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2110 from jpgrayson/fix/gix-date-parse-raw ([`651f9fa`](https://github.com/GitoxideLabs/gitoxide/commit/651f9fa560d5df7260a45068b8440f72820a6ffd))
    - Release gix-date v0.10.5 ([`4289ae6`](https://github.com/GitoxideLabs/gitoxide/commit/4289ae635d94d713d247eaf6f87d0ba91a1a3826))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2097 from GitoxideLabs/fix-gix-date ([`589d63e`](https://github.com/GitoxideLabs/gitoxide/commit/589d63ed21e5f2cd53ad2cac96fc387df3ea26e9))
    - Release gix-date v0.10.4 ([`007e3f6`](https://github.com/GitoxideLabs/gitoxide/commit/007e3f66246aaafc2374b85cbf77f89ec0b09512))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.70.0 (2025-07-15)

### Bug Fixes

 - <csr-id-a773854a798bb2315dcce347d86b856b8fac8dc9/> improve error message for when there is too many packs.
   Affects https://github.com/jj-vcs/jj/issues/6906

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Improve error message for when there is too many packs. ([`a773854`](https://github.com/GitoxideLabs/gitoxide/commit/a773854a798bb2315dcce347d86b856b8fac8dc9))
    - Merge pull request #2041 from cruessler/add-blame-extraction ([`dd5f0a4`](https://github.com/GitoxideLabs/gitoxide/commit/dd5f0a4811bc738051f7af164b8d2815aaa23220))
    - Thanks clippy ([`554ce13`](https://github.com/GitoxideLabs/gitoxide/commit/554ce134bc4b514b52a935f17f57f76ebf23ab97))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #1975 from GitoxideLabs/improvements ([`28935a5`](https://github.com/GitoxideLabs/gitoxide/commit/28935a56ff91f1fc2c17a7d23b057cf7119144e9))
    - Thanks clippy ([`dbf65c9`](https://github.com/GitoxideLabs/gitoxide/commit/dbf65c95644e6a134e7f9b75e7871479720b4deb))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.69.1 (2025-04-26)

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

## 0.69.0 (2025-04-25)

### Bug Fixes

 - <csr-id-b07f907ba2e01849744c72df35dac57b624f2f85/> Adapt to changes in gix-actor
   Use the committer date and author date that are now backed by bytes and
   interpret these bytes into a `gix_date::Time` on demand.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
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
    - Adapt to changes in `gix-date` and `gix-actor` ([`afdf1a5`](https://github.com/GitoxideLabs/gitoxide/commit/afdf1a5d5c9fb2645f481c17f580ad59d14d6095))
    - Adapt to changes in gix-actor ([`b07f907`](https://github.com/GitoxideLabs/gitoxide/commit/b07f907ba2e01849744c72df35dac57b624f2f85))
    - Merge pull request #1853 from GitoxideLabs/odb-issue ([`cd1a777`](https://github.com/GitoxideLabs/gitoxide/commit/cd1a77777f8a5b162b14ec4d0d3f476a0387d7f1))
    - Various minor changes ([`5fd0ab9`](https://github.com/GitoxideLabs/gitoxide/commit/5fd0ab90e3fbdc7c82b390520e007e258d7d0ce8))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1944 from GitoxideLabs/dependabot/cargo/cargo-ea4584273b ([`79dabb0`](https://github.com/GitoxideLabs/gitoxide/commit/79dabb0a062a795b1dc5c9c840d67474dc6b343b))
    - Bump crossbeam-channel from 0.5.14 to 0.5.15 in the cargo group ([`1792178`](https://github.com/GitoxideLabs/gitoxide/commit/1792178cbed5abf4782370bc46207a8c5d113615))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.68.0 (2025-04-04)

### Changed (BREAKING)

 - <csr-id-5095f44db58014f4a35ea8996a90d56d2ac19d45/> adjust error return types to handle collision detection
   This does mean a lot of churn across the tree, but the change is
   usually just an adjustment to variants of an existing error type,
   so I expect that most downstream users will require little to no
   adaption for this change.
 - <csr-id-4f2b6496109573237f82ee30058cbb93abf3e338/> use separate error type for I/O hashing operations
   Prepare for hashing becoming fallible.
 - <csr-id-54e57649f0e0b15c0bd1d3233e41524cb91a8cb9/> adjust hash verification return types for the common interface
   This mostly just affects return types – using
   `git_hash::verify::Error` instead of bespoke duplicated versions
   thereof, and occasionally returning an `ObjectId` instead of `()`
   for convenience.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`d248e3d`](https://github.com/GitoxideLabs/gitoxide/commit/d248e3d87d45ca3983cb9fd7c6143dacbd8301cc))
    - Release gix-sec v0.10.12, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`ada5a94`](https://github.com/GitoxideLabs/gitoxide/commit/ada5a9447dc3c210afbd8866fe939c3f3a024226))
    - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
    - Update changelogs prior to release ([`38dff41`](https://github.com/GitoxideLabs/gitoxide/commit/38dff41d09b6841ff52435464e77cd012dce7645))
    - Merge pull request #1915 from emilazy/push-qvyqmopsoltr ([`4660f7a`](https://github.com/GitoxideLabs/gitoxide/commit/4660f7a6f71873311f68f170b0f1f6659a02829d))
    - Refactor ([`4501086`](https://github.com/GitoxideLabs/gitoxide/commit/4501086adc544e675b3043c4c23b78a6c6711d8b))
    - Migrate `gix_object::{try_ =>}compute_hash` users ([`3d7e379`](https://github.com/GitoxideLabs/gitoxide/commit/3d7e379f26cbe53ddb430427b8e88ce0966be456))
    - Migrate hashing API users to fallible versions ([`fbf6cc8`](https://github.com/GitoxideLabs/gitoxide/commit/fbf6cc897cfeff5ed2a2d5946c060e0cebbd1afd))
    - Adjust error return types to handle collision detection ([`5095f44`](https://github.com/GitoxideLabs/gitoxide/commit/5095f44db58014f4a35ea8996a90d56d2ac19d45))
    - Use separate error type for I/O hashing operations ([`4f2b649`](https://github.com/GitoxideLabs/gitoxide/commit/4f2b6496109573237f82ee30058cbb93abf3e338))
    - Adjust hash verification return types for the common interface ([`54e5764`](https://github.com/GitoxideLabs/gitoxide/commit/54e57649f0e0b15c0bd1d3233e41524cb91a8cb9))
    - Migrate all hashing API users to `gix_hash::Hasher::finalize()` ([`4e935ce`](https://github.com/GitoxideLabs/gitoxide/commit/4e935ce167428581f7e0351768b705164f71179a))
    - Migrate all hashing API users to `gix_hash` ([`baa1430`](https://github.com/GitoxideLabs/gitoxide/commit/baa1430aed0dc8160a71cc675e2626780a2de052))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.67.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Bug Fixes

 - <csr-id-724601d474ce4acd30ee482e77fd96be2fe3425b/> improve handling of overflows when there are more pack than we can hold.
   Internally, there is a statically allocated vec which holds opened packs and
   indices.
   When reconciling the disk-state with what's currently loaded, it was possible
   to get into a situation where there were more files than we could fit into
   the slotmap and got into an invalid state.
   
   The system now generously trashes existing slots to be able to load more rencent ones,
   which seems to help in this situation.
   It's probably still leading to strange situations where not all objects can be read.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1750 from GitoxideLabs/odb-issue ([`e4fb21e`](https://github.com/GitoxideLabs/gitoxide/commit/e4fb21eb73e7cdb43e30df49eb72512e2836dad1))
    - Improve handling of overflows when there are more pack than we can hold. ([`724601d`](https://github.com/GitoxideLabs/gitoxide/commit/724601d474ce4acd30ee482e77fd96be2fe3425b))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1410 from GitoxideLabs/status ([`0ab4f64`](https://github.com/GitoxideLabs/gitoxide/commit/0ab4f64407b7fa0924830f7b7bd2f5b0ba1cc16e))
    - Adapt to changes in `gix-hash` ([`201e853`](https://github.com/GitoxideLabs/gitoxide/commit/201e853b151983502377b1f04d821c07d159014f))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.66.0 (2024-12-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0 ([`beb0ea8`](https://github.com/GitoxideLabs/gitoxide/commit/beb0ea8c4ff94c64b7773772a9d388ccb403f3c1))
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.65.0 (2024-11-24)

### New Features (BREAKING)

 - <csr-id-7325c584c3f0973975935b307630e29bc36ef5c7/> move `gix-odb::Write` trait to `gix-object::Write`.
   There it's far more useful and plumbing crates are enabled to write
   objects without pulling in `gix-odb` as dependency.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0 ([`4000197`](https://github.com/GitoxideLabs/gitoxide/commit/4000197ecc8cf1a5d79361620e4c114f86476703))
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
    - Adapt to changes in `gix-object` and `gix-odb` ([`96488f7`](https://github.com/GitoxideLabs/gitoxide/commit/96488f745506698e4183e8e544c4e0ecd1ef868b))
    - Move `gix-odb::Write` trait to `gix-object::Write`. ([`7325c58`](https://github.com/GitoxideLabs/gitoxide/commit/7325c584c3f0973975935b307630e29bc36ef5c7))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.64.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>

### New Features

 - <csr-id-ca5294af0971da372c5d5a5c7a569810dd53de93/> add `InMemoryPassThrough` implementation.
   An implementation of `Header`, `Write` and `Find`, that can optionally
   write everything to an in-memory store, and if enabled, also read
   objects back from there.
   
   That way it can present a consistent view to objects from two locations.

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 60 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0 ([`f1364dc`](https://github.com/GitoxideLabs/gitoxide/commit/f1364dcb8aa66e3d8730e38445b045c5b63c56e6))
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Merge pull request #1617 from Byron/commit-roundtrip ([`f35b109`](https://github.com/GitoxideLabs/gitoxide/commit/f35b1096c6db73842a55e089187d27d1287075ad))
    - Adapt to changes in `gix-object` ([`528f549`](https://github.com/GitoxideLabs/gitoxide/commit/528f549a3572eabb2ad137707a2ef5051d6414a4))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - Add `InMemoryPassThrough` implementation. ([`ca5294a`](https://github.com/GitoxideLabs/gitoxide/commit/ca5294af0971da372c5d5a5c7a569810dd53de93))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
</details>

## 0.63.0 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.32.0, gix-object v0.44.0, gix-filter v0.13.0, gix-revwalk v0.15.0, gix-traverse v0.41.0, gix-worktree-stream v0.15.0, gix-archive v0.15.0, gix-ref v0.47.0, gix-config v0.40.0, gix-index v0.35.0, gix-worktree v0.36.0, gix-diff v0.46.0, gix-discover v0.35.0, gix-dir v0.8.0, gix-mailmap v0.24.0, gix-negotiate v0.15.0, gix-pack v0.53.0, gix-odb v0.63.0, gix-revision v0.29.0, gix-refspec v0.25.0, gix-status v0.13.0, gix-submodule v0.14.0, gix-worktree-state v0.13.0, gix v0.66.0, gix-fsck v0.6.0, gitoxide-core v0.41.0, gitoxide v0.38.0, safety bump 26 crates ([`b3ff033`](https://github.com/GitoxideLabs/gitoxide/commit/b3ff033b602f303433f0b2e4daa2dba90b619c9e))
    - Prepare changelog prior to (yet another) release ([`209b6de`](https://github.com/GitoxideLabs/gitoxide/commit/209b6de0329dbaaf61b929d32d9d54cf13fe241e))
</details>

## 0.62.0 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 24 calendar days.
 - 30 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`4fe330e`](https://github.com/GitoxideLabs/gitoxide/commit/4fe330e68d10d51b0a7116a7ef8b9ea3b48a224c))
    - Release gix-attributes v0.22.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`f2b522d`](https://github.com/GitoxideLabs/gitoxide/commit/f2b522df2ddad07f065f43c2dbad49aa726714dd))
    - Release gix-glob v0.16.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`a65a17f`](https://github.com/GitoxideLabs/gitoxide/commit/a65a17fc396ef49663b0a75cf7b5502d370db269))
    - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
    - Prepare changelogs prior to release ([`0f25841`](https://github.com/GitoxideLabs/gitoxide/commit/0f2584178ae88e425f1c629eb85b69f3b4310d9f))
    - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
    - Assure the next release is breaking ([`9fd1090`](https://github.com/GitoxideLabs/gitoxide/commit/9fd10905449a41cdda5eb2764e4d45d314de9c04))
</details>

## 0.61.1 (2024-07-23)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 55 calendar days.
 - 62 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.31.5, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`6232824`](https://github.com/GitoxideLabs/gitoxide/commit/6232824301847a9786dea0b926796a3187493587))
    - Release gix-glob v0.16.4, gix-attributes v0.22.3, gix-command v0.3.8, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`a1b73a6`](https://github.com/GitoxideLabs/gitoxide/commit/a1b73a67c19d9102a2c5a7f574a7a53a86d0094c))
    - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
    - Prepare changelog prior to release ([`99c00cc`](https://github.com/GitoxideLabs/gitoxide/commit/99c00cc3ae9827555e2e1162328bc57038619d1f))
    - Release gix-path v0.10.9 ([`15f1cf7`](https://github.com/GitoxideLabs/gitoxide/commit/15f1cf76764221d14afa66d03a6528b19b9c30c9))
    - Release gix-actor v0.31.4, gix-object v0.42.3 ([`bf3d82a`](https://github.com/GitoxideLabs/gitoxide/commit/bf3d82abc7c875109f9a5d6b6713ce68153b6456))
    - Merge branch 'heredocs' ([`7330844`](https://github.com/GitoxideLabs/gitoxide/commit/73308446e5ffee053af35b108e3d49c71db31e99))
    - Regenerate archives ([`a4bb7b9`](https://github.com/GitoxideLabs/gitoxide/commit/a4bb7b9b7f15992644171bb06865637e18e1141f))
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Release gix-date v0.8.7, gix-mailmap v0.23.2 ([`c1d7c02`](https://github.com/GitoxideLabs/gitoxide/commit/c1d7c023d595eb04891b65295f001d85c9ba8074))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'feat/checkout-other-refs' ([`ecfde07`](https://github.com/GitoxideLabs/gitoxide/commit/ecfde07d0887322db34f5ea531891c92676e1ff4))
    - Thanks clippy ([`f36b9bd`](https://github.com/GitoxideLabs/gitoxide/commit/f36b9bd28052131401d048b5aa55c5ae1f9248db))
    - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
    - Re-add tests that were previously 'lost' due to an incorrect feature toggle ([`ca31249`](https://github.com/GitoxideLabs/gitoxide/commit/ca31249e9322b14868af4c3324a8cef991a1b552))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Regenerate archives for changed scripts ([`ea12fc2`](https://github.com/GitoxideLabs/gitoxide/commit/ea12fc234e898eb15013da40d2a82f69c2d20482))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
    - Release gix-fs v0.11.1, gix-glob v0.16.3 ([`2cefe77`](https://github.com/GitoxideLabs/gitoxide/commit/2cefe77203131878d0d8f5346f20f0e25b76cbea))
</details>

## 0.61.0 (2024-05-22)

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
 - <csr-id-7b3dc92d04e5240a7bdbcdaa4019ac93cec6eaa3/> use `tempfile` permissions support to set the correct mode on unix.
   Previousoly it would make an additional syscall to change permissions, which
   is slower than necessary.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 10 calendar days.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Release gix-date v0.8.6 ([`d3588ca`](https://github.com/GitoxideLabs/gitoxide/commit/d3588ca4fe0364c88e42cdac24ceae548355d99d))
    - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
    - Adapt to changes in `gix-pack` ([`bad5b48`](https://github.com/GitoxideLabs/gitoxide/commit/bad5b48e4f0d865b0b0937f136d9a0041aa88046))
    - More robustness in the face of a trampling-herd of threads loading a single index. ([`addf446`](https://github.com/GitoxideLabs/gitoxide/commit/addf446f052ff74edcdb083f2b2968b313daa940))
    - Use `tempfile` permissions support to set the correct mode on unix. ([`7b3dc92`](https://github.com/GitoxideLabs/gitoxide/commit/7b3dc92d04e5240a7bdbcdaa4019ac93cec6eaa3))
</details>

## 0.60.0 (2024-04-13)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
</details>

## 0.59.0 (2024-03-14)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 4 calendar days.
 - 18 days passed between releases.
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

## 0.58.0 (2024-02-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 30 calendar days.
 - 36 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'tempfile-permissions' ([`7b44c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7b44c7ff1dc0b8875214d2673c7f52948cf04ff0))
    - Release gix-tempfile v13.1.0, gix-lock v13.1.0, safety bump 12 crates ([`8430442`](https://github.com/GitoxideLabs/gitoxide/commit/84304427dfe4d170c7732161b126961719f70059))
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
</details>

## 0.57.0 (2024-01-20)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 20 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Merge branch 'dirwalk' ([`5d176fc`](https://github.com/GitoxideLabs/gitoxide/commit/5d176fc5ab82bfc7c194b4d929e73da9659ae8b8))
    - Use `gix_fs::current_dir(precompose_unicode)`. ([`7d8d167`](https://github.com/GitoxideLabs/gitoxide/commit/7d8d1678c9dd185a369e996551a6ed89fbb391f5))
    - Adapt to changes in `gix-features` ([`eacb5a4`](https://github.com/GitoxideLabs/gitoxide/commit/eacb5a4ae2fd94b095005cfbc0a8b2aa67539e52))
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
</details>

## 0.56.1 (2023-12-30)

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

## 0.56.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 19 calendar days.
 - 22 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Thanks clippy ([`d38d1cc`](https://github.com/GitoxideLabs/gitoxide/commit/d38d1cc1aa3402629a0f182324e3310e730ce3f2))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Check all git-lfs managed files into the repository ([`35439de`](https://github.com/GitoxideLabs/gitoxide/commit/35439defd2d71779d4b3795b7652cde18ff11150))
    - Release gix-hash v0.13.3, gix-index v0.27.1 ([`98b08f4`](https://github.com/GitoxideLabs/gitoxide/commit/98b08f4d0d9237be0e0c2caa9bf5c13ae8bbf9d8))
</details>

## 0.55.0 (2023-12-06)

### Changed (BREAKING)

 - <csr-id-fad298228ca91298abe42af41defb8f6c89c3cc9/> Remove `Find` trait (as it's now living in `gix-odb`)
   This makes it easier to access objects via trait from other crates
   without having to pull in all of `gix-odb`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`d3fd11e`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd11ec3783843d4e49081e1d14359ed9714b5f))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
    - Adapt to changes in `gix-object` (Find::try_header()) ([`336d161`](https://github.com/GitoxideLabs/gitoxide/commit/336d161a4a291b5cb03c08e637c6f47a05731996))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'fix-1096' ([`ff99a18`](https://github.com/GitoxideLabs/gitoxide/commit/ff99a18e9f9388542a9cbf17d61b413f34b1d533))
    - Adapt to changes in `gix-object` ([`203d69c`](https://github.com/GitoxideLabs/gitoxide/commit/203d69c8890acc716bd4f7a7b1b2b91a8c828bde))
    - Merge branch 'gix-object-find' ([`c8bd660`](https://github.com/GitoxideLabs/gitoxide/commit/c8bd66065316176dfbbfe7ecaa092a25cad1854b))
    - Adapt to changes in `gix-pack` ([`40e7440`](https://github.com/GitoxideLabs/gitoxide/commit/40e74404e87e44b0b2a5fe3d294f3ddc5ed2ba88))
    - Remove `Find` trait (as it's now living in `gix-odb`) ([`fad2982`](https://github.com/GitoxideLabs/gitoxide/commit/fad298228ca91298abe42af41defb8f6c89c3cc9))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## 0.54.0 (2023-10-12)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 8 calendar days.
 - 17 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Fix docs ([`995bc84`](https://github.com/GitoxideLabs/gitoxide/commit/995bc840664cbd4aeb7f95592e3125dee63bdcd4))
    - Thanks clippy ([`345712d`](https://github.com/GitoxideLabs/gitoxide/commit/345712dcdfddcccc630bbfef2ed4f461b21550d3))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Adapt to changes in `gix-object` ([`ffcb110`](https://github.com/GitoxideLabs/gitoxide/commit/ffcb110135e4597d8953b97da3db9ecc3cf12e34))
</details>

## 0.53.0 (2023-09-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 16 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
</details>

## 0.52.0 (2023-09-08)

<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

### Chore (BREAKING)

- <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
  It makes proper usage of `Progress` types easier and allows them to be used
  as `dyn` traits as well.

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch 'optimizations' ([`6135a5e`](https://github.com/GitoxideLabs/gitoxide/commit/6135a5ea8709646f01da62939a59dd3a9750e007))
    - Adapt to changes in features of `gix-pack` ([`6b27ffa`](https://github.com/GitoxideLabs/gitoxide/commit/6b27ffa18f0049321e7c1837acc5467f0966fbb5))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Merge branch 'perf-and-safety' ([`9ad9c5b`](https://github.com/GitoxideLabs/gitoxide/commit/9ad9c5b1cfa3afff5273558b6ef98ca4714d4272))
    - Adapt to changes in `gix-pack` ([`46a4d4d`](https://github.com/GitoxideLabs/gitoxide/commit/46a4d4d873610ba88d51bce865404b904b2b7cb3))
    - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
    - Release gix-date v0.7.4, gix-index v0.23.0, safety bump 5 crates ([`3be2b1c`](https://github.com/GitoxideLabs/gitoxide/commit/3be2b1ccfe30eeae45711c64b88efc522a2b51b7))
    - Thanks clippy ([`5044c3b`](https://github.com/GitoxideLabs/gitoxide/commit/5044c3b87456cf58ebfbbd00f23c9ba671cb290c))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.51.0 (2023-08-22)

<csr-id-01c6ef62e260246f1623bdf05f49c03eff14ac69/>

### Chore

- <csr-id-01c6ef62e260246f1623bdf05f49c03eff14ac69/> reorganize tests to get rid of feature toggles that are for internal testing only

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 18 calendar days.
 - 19 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0 ([`6c62e74`](https://github.com/GitoxideLabs/gitoxide/commit/6c62e748240ac0980fc23fdf30f8477dea8b9bc3))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Remove version specifications from dev dependencies ([`e80ed04`](https://github.com/GitoxideLabs/gitoxide/commit/e80ed04f1cc065c5f4ddf196e780a8a8511a8069))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
    - Reorganize tests to get rid of feature toggles that are for internal testing only ([`01c6ef6`](https://github.com/GitoxideLabs/gitoxide/commit/01c6ef62e260246f1623bdf05f49c03eff14ac69))
    - Release gix-glob v0.10.2, gix-date v0.7.2, gix-validate v0.8.0, gix-object v0.34.0, gix-ref v0.34.0, gix-config v0.27.0, gix-commitgraph v0.18.2, gix-revwalk v0.5.0, gix-revision v0.19.0, gix-refspec v0.15.0, gix-submodule v0.1.0, safety bump 18 crates ([`4604f83`](https://github.com/GitoxideLabs/gitoxide/commit/4604f83ef238dc07c85aaeae097399b67f3cfd0c))
    - Merge branch 'dev-on-linux' ([`6b4a303`](https://github.com/GitoxideLabs/gitoxide/commit/6b4a30330fe49fc97daa73f55bf56580cc0597aa))
    - Fix various tests to run properly on linux ([`ef8ccd9`](https://github.com/GitoxideLabs/gitoxide/commit/ef8ccd9d16143d37155d063747c69cade80f162d))
</details>

## 0.50.2 (2023-08-02)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 10 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.24.2, gix-object v0.33.2, gix-ref v0.33.3, gix-config v0.26.2, gix-prompt v0.5.5, gix-odb v0.50.2, gix-transport v0.34.2, gix-protocol v0.37.0, gix-worktree v0.23.1, gix v0.51.0, safety bump 3 crates ([`231ac1c`](https://github.com/GitoxideLabs/gitoxide/commit/231ac1c6ad5ca9a84dbeb0dee14bfbf2fef1ae1e))
    - Prepare additional changelogs ([`db63815`](https://github.com/GitoxideLabs/gitoxide/commit/db6381522395a0de047118e81df5cd3cbeb862b9))
    - Prepare changelogs ([`e4d2890`](https://github.com/GitoxideLabs/gitoxide/commit/e4d2890a85bf60e9cdb4016dddfab3c4dccbe75e))
    - Merge branch 'fixes-and-improvements' ([`f8b1f55`](https://github.com/GitoxideLabs/gitoxide/commit/f8b1f553371f25b1bea6bce7cbb2ff1f01194856))
    - Add note go ODB initialization in preparation for dealing with many small packs. ([`090357e`](https://github.com/GitoxideLabs/gitoxide/commit/090357ee7b36ead5c622b13b1fabab5174499d82))
</details>

## 0.50.1 (2023-07-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0 ([`0062971`](https://github.com/GitoxideLabs/gitoxide/commit/00629710dffeb10fda340665530353703cf5d129))
    - Release gix-tempfile v7.0.2, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0 ([`107a64e`](https://github.com/GitoxideLabs/gitoxide/commit/107a64e734580ad9e2c4142db96394529d8072df))
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Prepare more changelogs ([`c4cc5f2`](https://github.com/GitoxideLabs/gitoxide/commit/c4cc5f261d29f712a101033a18293a97a9d4ae85))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.50.0 (2023-07-19)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`4aca8c2`](https://github.com/GitoxideLabs/gitoxide/commit/4aca8c2ae2ec588fb65ec4faa0c07c19d219569f))
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
</details>

## 0.49.1 (2023-06-29)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-glob v0.9.1, gix-attributes v0.14.1, gix-config-value v0.12.3, gix-ref v0.32.1, gix-sec v0.8.3, gix-config v0.25.1, gix-url v0.20.1, gix-credentials v0.16.1, gix-discover v0.21.1, gix-ignore v0.4.1, gix-pack v0.39.1, gix-odb v0.49.1, gix-worktree v0.21.1, gix v0.48.0 ([`69c6a36`](https://github.com/GitoxideLabs/gitoxide/commit/69c6a36ba14cbef129deebda9fd8870005fefa17))
    - Prepare changelogs prior to release ([`c143cf4`](https://github.com/GitoxideLabs/gitoxide/commit/c143cf48ee1885467e3e9262a3f8823a1247bfe0))
    - Align usage of `gix-path` across all crates ([`73c1292`](https://github.com/GitoxideLabs/gitoxide/commit/73c1292be393986c4a1adde1400abf551e850da0))
</details>

## 0.49.0 (2023-06-29)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 6 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
    - Prepare changelogs prior to release ([`00f96fb`](https://github.com/GitoxideLabs/gitoxide/commit/00f96fb3110a8f81a1bd0d74c757c15b8773c6f6))
</details>

## 0.48.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### New Features

 - <csr-id-3cffa268460eb2d41bd6a30d45778b88db4ec602/> provide basic `tracing` spans for common operations.
   This is just the beginning and more crates will integrate with it over time.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 11 calendar days.
 - 12 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - `just fmt` ([`871dd0b`](https://github.com/GitoxideLabs/gitoxide/commit/871dd0b977caf17159092a4739ba5408403cdb2c))
    - Merge branch 'gix-corpus' ([`5861afb`](https://github.com/GitoxideLabs/gitoxide/commit/5861afb45f32c16eefcd8e7b7480309bf44b6edc))
    - Add more tasks to gather a little more information ([`891a061`](https://github.com/GitoxideLabs/gitoxide/commit/891a06107883b4a21796facf046a0cd697dc2134))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Provide basic `tracing` spans for common operations. ([`3cffa26`](https://github.com/GitoxideLabs/gitoxide/commit/3cffa268460eb2d41bd6a30d45778b88db4ec602))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
    - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
    - Adapt to changes in `gix-actor` ([`4a80e86`](https://github.com/GitoxideLabs/gitoxide/commit/4a80e868f9530896616e649838e9be64b6d10036))
    - Adapt to changes in `gix-date` ([`d575336`](https://github.com/GitoxideLabs/gitoxide/commit/d575336c26e6026e463cd06d88266bb2bdd3e162))
</details>

## 0.47.0 (2023-06-10)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-attributes v0.13.1, gix-diff v0.30.1, gix-revwalk v0.1.0, gix-traverse v0.27.0, gix-index v0.18.0, gix-revision v0.15.2, gix-negotiate v0.2.1, gix-pack v0.37.0, gix-odb v0.47.0, gix-protocol v0.33.2, gix-worktree v0.19.0, gix v0.46.0, safety bump 7 crates ([`2560a2c`](https://github.com/GitoxideLabs/gitoxide/commit/2560a2cc3e1d8c60cd812e15696fa4761d036e19))
    - Prepare changelogs prior to release ([`298f3d7`](https://github.com/GitoxideLabs/gitoxide/commit/298f3d7359c5b183314d8c584e45dcdd559d88b3))
</details>

## 0.46.0 (2023-06-06)

### New Features

 - <csr-id-8c72a236dbeb71a4aead45bf82010f1c89829540/> add `Store::alternate_db_paths()`.
   Provides a way to learn about loose database paths that are provided by
   git alternates.
 - <csr-id-3db18c45e8b26243907521ffd11156afed28a0a3/> implement `Clone` for `Sink`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 25 calendar days.
 - 40 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Add `Store::alternate_db_paths()`. ([`8c72a23`](https://github.com/GitoxideLabs/gitoxide/commit/8c72a236dbeb71a4aead45bf82010f1c89829540))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Apply -W clippy::cloned-instead-of-copied ([`150463c`](https://github.com/GitoxideLabs/gitoxide/commit/150463c26f0d2e1c2b5facba731ccba29cf23228))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Auto-fix clippy to remove explicit iter looping ([`3eff567`](https://github.com/GitoxideLabs/gitoxide/commit/3eff567c683b5c650c14792b68968cbdbc90ec5c))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Merge branch 'fix-851' ([`2f275d5`](https://github.com/GitoxideLabs/gitoxide/commit/2f275d5d3cb49b3b8ba53b30e4b4386fac32662b))
    - Implement `Clone` for `Sink`. ([`3db18c4`](https://github.com/GitoxideLabs/gitoxide/commit/3db18c45e8b26243907521ffd11156afed28a0a3))
    - Release gix-object v0.29.2 ([`4f879bf`](https://github.com/GitoxideLabs/gitoxide/commit/4f879bf35653bdc8f9729d524c6e8e1fb3c6886b))
</details>

## 0.45.0 (2023-04-27)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.8.0, gix-glob v0.7.0, gix-attributes v0.12.0, gix-config-value v0.12.0, gix-ref v0.29.0, gix-sec v0.8.0, gix-config v0.22.0, gix-prompt v0.5.0, gix-url v0.18.0, gix-credentials v0.14.0, gix-discover v0.18.0, gix-ignore v0.2.0, gix-pack v0.35.0, gix-odb v0.45.0, gix-transport v0.31.0, gix-protocol v0.32.0, gix-refspec v0.10.1, gix-worktree v0.17.0, gix v0.44.1 ([`7ebc9f7`](https://github.com/GitoxideLabs/gitoxide/commit/7ebc9f734ec4371dd27daa568c0244185bb49eb5))
    - Prepare changelogs prior to release ([`0135158`](https://github.com/GitoxideLabs/gitoxide/commit/013515897215400539bfd53c25548bd054186ba6))
    - Bump gix-path v0.8.0, safety bump 20 crates (gix set to 0.44.1 manually) ([`43ebaf2`](https://github.com/GitoxideLabs/gitoxide/commit/43ebaf267557218865862538ffc7bdf00558492f))
</details>

## 0.44.0 (2023-04-26)

### Bug Fixes

 - <csr-id-07e11cf210682337f777f1cbbc0d25794c1179ca/> set permissions of newly written loose objects to be similar to `git`.
   Note that the current implementation lacks all of the sophistication that git
   applies, and doing this properly definitely takes more work as we would need
   to support `core.sharedRepository`.
   
   Further, our tempfile implementation doesn't allow the setup of file modes
   right when it matters, so that could mean quite some work to either workaround
   or contribute.
 - <csr-id-416ceccf7eaf1946efed5a2c95461a2690ae367a/> collisions of newly written object's don't trigger collisions anymore.
   It's solved by special-casing windows and assume that certain kinds of filesystem errors
   are the result of a collision (with some degree of concurrency/contention).

### New Features (BREAKING)

 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 14 calendar days.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#814](https://github.com/GitoxideLabs/gitoxide/issues/814), [#819](https://github.com/GitoxideLabs/gitoxide/issues/819)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
    - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
 * **[#819](https://github.com/GitoxideLabs/gitoxide/issues/819)**
    - Collisions of newly written object's don't trigger collisions anymore. ([`416cecc`](https://github.com/GitoxideLabs/gitoxide/commit/416ceccf7eaf1946efed5a2c95461a2690ae367a))
 * **Uncategorized**
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`d7173b2`](https://github.com/GitoxideLabs/gitoxide/commit/d7173b2d2cb79685fdf7f618c31c576db24fa648))
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`e4df557`](https://github.com/GitoxideLabs/gitoxide/commit/e4df5574c0813a0236319fa6e8b3b41bab179fc8))
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - Merge branch 'fix-823' ([`6ebd61e`](https://github.com/GitoxideLabs/gitoxide/commit/6ebd61e548a36a04e413ac725a03e607a3588334))
    - Thanks clippy ([`14e64e7`](https://github.com/GitoxideLabs/gitoxide/commit/14e64e74649cfb1f2f99da87015939af98fae5c8))
    - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
    - Set permissions of newly written loose objects to be similar to `git`. ([`07e11cf`](https://github.com/GitoxideLabs/gitoxide/commit/07e11cf210682337f777f1cbbc0d25794c1179ca))
    - Make empty `sink` module non-public. ([`45a0ac1`](https://github.com/GitoxideLabs/gitoxide/commit/45a0ac192dd15500d577838e850edb56187f42c2))
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
</details>

## 0.43.1 (2023-03-30)

### Documentation

 - <csr-id-02c4659984fa6423bc76cc4980a143edaba8ace0/> fix minor typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.7.3, gix-config-value v0.10.2, gix-config v0.20.1, gix-discover v0.16.2, gix-index v0.15.1, gix-odb v0.43.1, gix-packetline v0.15.1, gix-protocol v0.30.2, gix-worktree v0.15.2, gix v0.43.1 ([`38eed1d`](https://github.com/GitoxideLabs/gitoxide/commit/38eed1d06e7cbb8fbcd54b2cad3163ca45e0baf1))
    - Fix minor typos ([`02c4659`](https://github.com/GitoxideLabs/gitoxide/commit/02c4659984fa6423bc76cc4980a143edaba8ace0))
    - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
</details>

## 0.43.0 (2023-03-10)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 6 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v5.0.0, gix-lock v5.0.0, gix-ref v0.27.0, gix-config v0.19.0, gix-url v0.16.0, gix-credentials v0.12.0, gix-discover v0.16.0, gix-index v0.15.0, gix-pack v0.33.0, gix-odb v0.43.0, gix-transport v0.28.0, gix-protocol v0.29.0, gix-worktree v0.15.0, gix v0.41.0, safety bump 12 crates ([`29a0870`](https://github.com/GitoxideLabs/gitoxide/commit/29a087043d1feb2f127b065341c8028d0bd0301e))
    - Prepare changelogs prior to release ([`e06f5f5`](https://github.com/GitoxideLabs/gitoxide/commit/e06f5f523e83f4da390eddbebcb9a2d58674587b))
</details>

## 0.42.0 (2023-03-04)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-attributes v0.10.0, gix-ref v0.26.0, gix-config v0.18.0, gix-url v0.15.0, gix-credentials v0.11.0, gix-discover v0.15.0, gix-index v0.14.0, gix-mailmap v0.11.0, gix-odb v0.42.0, gix-transport v0.27.0, gix-protocol v0.28.0, gix-revision v0.12.0, gix-refspec v0.9.0, gix-worktree v0.14.0, gix v0.39.0 ([`93e75fe`](https://github.com/GitoxideLabs/gitoxide/commit/93e75fed454ed8b342231bde4638db90e407ce52))
    - Prepare changelogs prior to release ([`895e482`](https://github.com/GitoxideLabs/gitoxide/commit/895e482badf01e953bb9144001eebd5e1b1c4d84))
    - Release gix-features v0.28.0, gix-actor v0.19.0, gix-object v0.28.0, gix-diff v0.28.0, gix-traverse v0.24.0, gix-pack v0.32.0, safety bump 20 crates ([`0f411e9`](https://github.com/GitoxideLabs/gitoxide/commit/0f411e93ec812592bb9d3a52b751399dd86f76f7))
</details>

## 0.41.0 (2023-03-01)

### Bug Fixes

 - <csr-id-e14dc7d475373d2c266e84ff8f1826c68a34ab92/> note that crates have been renamed from `git-*` to `gix-*`.
   This also means that the `git-*` prefixed crates of the `gitoxide` project
   are effectively unmaintained.
   Use the crates with the `gix-*` prefix instead.
   
   If you were using `git-repository`, then `gix` is its substitute.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 2 calendar days.
 - 11 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v4.1.0, gix-lock v4.0.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0, safety bump 6 crates ([`ea9fd1d`](https://github.com/GitoxideLabs/gitoxide/commit/ea9fd1d9b60e1e9e17042e9e37c06525823c40a5))
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
    - Merge branch 'adjustments-for-cargo' ([`d686d94`](https://github.com/GitoxideLabs/gitoxide/commit/d686d94e1030a8591ba074757d56927a346c8351))
    - Remove `num_cpus` dependency in favor of `std::thread::available_parallelism()` ([`9567102`](https://github.com/GitoxideLabs/gitoxide/commit/9567102d4e1a729f7f9882f688365784b9813ac6))
    - Prepare for git-tempfile release ([`56c005b`](https://github.com/GitoxideLabs/gitoxide/commit/56c005b13c44376f71e61781e73c0bf93416d0e4))
</details>

## 0.40.2 (2023-02-17)

<csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/>
<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>
<csr-id-46636e64c9a48ec0e85e014ac0cc8b48846d8462/>
<csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/>
<csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/>
<csr-id-47ca6ab2ff0cbf8801d0a82cebbbeb8c4f62cdae/>
<csr-id-2d6960f886c1165f0bdb6f2d653388e1e0b57a2d/>
<csr-id-424c9b3a2b467f5a1e339700257cd4ab72e2e692/>
<csr-id-b1c82a7959fba1541642fc8dfae46b27848f2ba3/>
<csr-id-9235106986e14551a28693bfe4ea92f046c65406/>
<csr-id-747a13e9a1fe5200c53055dd961507c9fef667e1/>
<csr-id-4c77e4c97641ab3b02b56aaa702a7d2ca5bced7c/>
<csr-id-d53c4b0f91f1b29769c9430f2d1c0bcab1170c75/>
<csr-id-b317200b72096573d511d229c6e61e74e7ba14db/>
<csr-id-eaae9c1bc723209d793eb93f5587fa2604d5cd92/>
<csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/>
<csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/>
<csr-id-9945eba749afb020e0deaaa5bb01fda6ff9ccd84/>
<csr-id-cfd8a25f9125c48afe4b66eab6b6ecf71097c486/>
<csr-id-1525f36d29574699d2fcb16b70678121030fd109/>
<csr-id-4ff21686c32a6edc84ea041c3040f33ae24f9519/>
<csr-id-91c8fc1f0c50af55d7cb233bbe813c6d12fe11bc/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/>

### Refactor (BREAKING)

- <csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/> remove pack-cache from `Find::try_find(…)`
  With the new architecture this can be an implementation detail without
  forcing it to be Sync.
- <csr-id-2290d006705ff47ad780b009fe58ee422b3285af/> move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.
- <csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/> move loose header manipulation from git-pack to git-object

### Bug Fixes (BREAKING)

 - <csr-id-1fabdc51b9468ba2c6b8cf74509ad5aa2a0b86f4/> `alternate::resolve(…)` now takes the current_dir as argument.
   That way it's more consistent with similar low-level functions and it's
   possible to avoid multiple calls to `std::env::current_dir()`.
   
   Furthermore, the usage of `current_dir()` is made explicit when
   instantiating a store to allow it to be resued.

### New Features (BREAKING)

 - <csr-id-d9d05b0db6b4453e7385117d466bf7c2e8de81fa/> add `Store::try_header()` for obtaining object information quickly.
   Note that this feature also comes with various refactorings related to the error
   type used by various methods in order to get away from a 'one error fits all' kind
   of situation.
 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `git-features` and `git-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.
 - <csr-id-95210cb2ba85f75148b4ef48ccea9d9f8a0a0114/> Provide optional `candidates` for ambigious entries during `lookup_prefix()`
   The candidate entries are all entries matching a given prefix.
 - <csr-id-92d8be1101a7e76e70cd90db6a943b9e31e20802/> `loose::Db` and `Store` can return all candidate objects for a single prefix
   This is the first step towards auto-disambiguating objects in rev-parse.
 - <csr-id-bf73a94b43288b6634dbb33f2433656987a73baf/> `Cache::inner` removed in favor of `Deref/Mut` and `into_inner()`
   Making the `inner` field available allows changing it, which would make
   it potentially incompatible with existing caches. The new
   implementation makes it essentially read-only while allowing more
   convenient access to methods on `inner`.

### Changed (BREAKING)

<csr-id-ab4e726fcec65871a81056a9c69af8ea3f56b2a3/>
<csr-id-8bb5c9a75cd91ae0d888bc8e93707cfc9cc08090/>
<csr-id-580e96c1b2d9782a2e8cf9d1123f6d53a5376a3d/>
<csr-id-3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7/>

 - <csr-id-8c5ae77f06a64c57df9a9ad1190266896a223dbe/> Remove deprecated compound and linked object databases
   The dynamic/general store is the only maintained can-do-it-all
   DB now.
 - <csr-id-91d047658b114f372735116c9d8e6962a3873137/> cleanup and unify `verify_integrity()` method signature
   Previously they used many different ways of handling their parameters
   despite all boiling down to calling the same 'index::File::traverse()`
   method.
   
   This allows for more reuse of `Options` structs and generally makes
   clearer how these optinos are used.
 - <csr-id-2ef9a8424af51310db8c1e6df31dde9953ed3d21/> Change accessors named `hash_kind()` to `object_hash()` for consistency
 - <csr-id-49998cce419a27f3928ec4ac39da5e3b500e5cb2/> consistently use `object_hash` instead of `hash_kind`
 - <csr-id-67c42fbf5f88f8dc42a9ebd7c6276d57ba1d4624/> remove `Write::*(…, hash_kind)`
   The `hash_kind` is now intrinsic to the implementation of the write
   trait and thus isn't passed along anymore in parameters.
   
   The `sink()` function now takes the kind of hash as parameter.
 - <csr-id-ad1b9ea17eb4b98ebd2fddebe82a8fee1d63e9dd/> various changes to the `loose::Store`
   - Change `path` field to read-only `path()` method

### Refactor

- <csr-id-46636e64c9a48ec0e85e014ac0cc8b48846d8462/> flatten errors into one
  By adding one variant, one can remove the previous 'sub-error', for which
  there is no precedent in the codebase yet.
- <csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/> replace bare u32 `data::Id` typedef
- <csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/> Use borrowed::Id in trees for full type safety
- <csr-id-47ca6ab2ff0cbf8801d0a82cebbbeb8c4f62cdae/> a simpler implementation to skip the header

### Other

- <csr-id-2d6960f886c1165f0bdb6f2d653388e1e0b57a2d/> try LRU-like contains implementation
  Which unfortunately isn't really faster at all even though it totally
  should be.
- <csr-id-424c9b3a2b467f5a1e339700257cd4ab72e2e692/> Try to make Handle usable for pack creation
  It's nearly there, but for some reason the boxed dyn traits don't get to
  be Send even though it's specified.
- <csr-id-b1c82a7959fba1541642fc8dfae46b27848f2ba3/> :Find for Arc and Rc
- <csr-id-9235106986e14551a28693bfe4ea92f046c65406/> :Find implementation for linked::Store
- <csr-id-747a13e9a1fe5200c53055dd961507c9fef667e1/> :borrowed::Object => git-odb::data::Object
- <csr-id-4c77e4c97641ab3b02b56aaa702a7d2ca5bced7c/> :Db::init() with a few tests
- <csr-id-d53c4b0f91f1b29769c9430f2d1c0bcab1170c75/> add link to simplified/polonius version in the docs
- <csr-id-b317200b72096573d511d229c6e61e74e7ba14db/> Only check alternates for objects not found in packs or loose
  This matches the behavior of git.
- <csr-id-eaae9c1bc723209d793eb93f5587fa2604d5cd92/> Avoid double-lookup in packs without polonius
  Split object lookup into two steps: looking up the object index, and
  looking up the object itself given the index. This avoids passing in the
  buffer (and thus looking like an unconditional borrow to non-polonius)
  until we're committed to returning from the loop.
- <csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/> try to get rid of tree-traversal Boxed error…
  …which really complicates things downstream as these now have to deal
  with another type argument, or of to try to turn it into a Box anyway.

  The latter seems to be…troubling so I can't make it compile.
- <csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/> See if tree compaction saves considerable amounts of memory
  No, it's not worth it.
- <csr-id-9945eba749afb020e0deaaa5bb01fda6ff9ccd84/> try to use a customized version of just pieces of Miniz-oxide
- <csr-id-cfd8a25f9125c48afe4b66eab6b6ecf71097c486/> fanout table, but slowly I get it :D
- <csr-id-1525f36d29574699d2fcb16b70678121030fd109/> discard idea of making traversal even more generic
- <csr-id-4ff21686c32a6edc84ea041c3040f33ae24f9519/> first silly attempt to randomly remove an allocation
- <csr-id-91c8fc1f0c50af55d7cb233bbe813c6d12fe11bc/> get rid of failure crate in favor of quick-error

### Bug Fixes

 - <csr-id-4fffa9a9198cf3012fa8215796aab3d456519ff3/> remove panic-assertions in `loose` `lookup_prefix`
 - <csr-id-1ce3190000f6211ce31468c7603d491bb5b90293/> Disable tag.gpgSign in test scripts
   This is done for the same reason that commit.gpgsign is disabled for test
   scripts. It prevents test failures if the user has tag.gpgsign enabled in
   their global git config when invoking tests.
 - <csr-id-41d494365d281056c5e9466860db808bd85143e9/> improve error messages when objects aren't found
 - <csr-id-9c14de391a1a9f1055922164d1757c9aa9720807/> support Rust 1.52
 - <csr-id-b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e/> linked::Store now assures unique IDs across compound stores

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/> remove unused dependencies

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### New Features

 - <csr-id-d792ea543246632bf1ca8d0e1d239bbe7f07e219/> use enumerations to advertise progress ids publicly.
   Previously these were an implementation detail which also means they
   couldn't be relied upon.
   
   Thanks to an intermediate enumeration, they become part of the public API
   and their actual value is not exposed.
 - <csr-id-e9d1f45e944e91bb9715a3ee89a4f28b09250411/> support for pack-order when iterating objects.
 - <csr-id-7f19bd7e63d78e3151e43d5094ae9d35cbe34f46/> add `loose::Store::try_header()` to obtain loose object information without content.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-81e1a9d38aac9e6dd0618266ff826593e038cce8/> Add `Cache::has_object_cache()` and `Cache::has_pack_cache()` methods.
   That way it's possible to conditionally set or change the cache size.
 - <csr-id-84ec54e904378c5b3d7da9efff66b02e88b16916/> Handle::packed_object_count()
   Provide packed objects numbers and cache the value
   for fast access later on.
 - <csr-id-996bfb3061fd9ee2cf38c93f39e0d4c7c6163386/> loose::Store::lookup_prefix(…)
 - <csr-id-58c2edb76755ab71e10eef4cd9a51533825c291f/> git_pack::Find::try_find_cached(…, pack_cache)
   With this method it's easier to bypass local caches and control
   the cache oneself entirely.
 - <csr-id-36fde720c34e02429a810ddd43b894a37516f51a/> add linked::Store::rc_iter()
   For completeness in case of single-threaded operations
 - <csr-id-a81b33359a4394a66f854195445f8f9aa0a46179/> linked::Store sorts bundles by modification date, newest first
 - <csr-id-e25f4eadec679406aad6df10026e27e4832c2482/> A simplified version of the `Find` trait
   It's meant for the next generation of object db handles which keep a
   local cache of all the details of the actual object database.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1569 commits contributed to the release.
 - 60 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 26 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#250](https://github.com/GitoxideLabs/gitoxide/issues/250), [#254](https://github.com/GitoxideLabs/gitoxide/issues/254), [#259](https://github.com/GitoxideLabs/gitoxide/issues/259), [#260](https://github.com/GitoxideLabs/gitoxide/issues/260), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#287](https://github.com/GitoxideLabs/gitoxide/issues/287), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#333](https://github.com/GitoxideLabs/gitoxide/issues/333), [#364](https://github.com/GitoxideLabs/gitoxide/issues/364), [#384](https://github.com/GitoxideLabs/gitoxide/issues/384), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#536](https://github.com/GitoxideLabs/gitoxide/issues/536), [#59](https://github.com/GitoxideLabs/gitoxide/issues/59), [#63](https://github.com/GitoxideLabs/gitoxide/issues/63), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691), [#724](https://github.com/GitoxideLabs/gitoxide/issues/724), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 40 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Adjust all changelogs to fulfil requirements for publishing ([`04b9ca0`](https://github.com/GitoxideLabs/gitoxide/commit/04b9ca025a1667529b2221ab4280bd3c8dae01cf))
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Allow 'refactor' and 'other' in conventional messages if they have breaking changes ([`4eebaac`](https://github.com/GitoxideLabs/gitoxide/commit/4eebaac669e590beed112b622752997c64772ef1))
    - Rebuild all changelogs to assure properly ordered headlines ([`4a9a05f`](https://github.com/GitoxideLabs/gitoxide/commit/4a9a05f95930bad5938d4ce9c517ebf0e0b990f1))
    - Sort all commits by time, descending… ([`f536bad`](https://github.com/GitoxideLabs/gitoxide/commit/f536bad20ffbac4dc353dfeb1a917bb88becbb78))
    - Greatly reduce changelog size now that the traversal fix is applied ([`a0bc98c`](https://github.com/GitoxideLabs/gitoxide/commit/a0bc98c06c349de2fd6e0d4593606e68b98def72))
    - Fixup remaining changelogs… ([`2f75db2`](https://github.com/GitoxideLabs/gitoxide/commit/2f75db294fcf20c325555822f65629611be52971))
    - Generate changelogs with details ([`e1861ca`](https://github.com/GitoxideLabs/gitoxide/commit/e1861caa435d312953a9fea7ceff6d2e07b03443))
    - Update all changelogs with details ([`58ab2ae`](https://github.com/GitoxideLabs/gitoxide/commit/58ab2aee23ba70a536e9487b44fb04c610374d1a))
    - Update changelogs ([`c857d61`](https://github.com/GitoxideLabs/gitoxide/commit/c857d61ce3ce342012a2c4ba10a8327822aa530e))
    - Avoid adding newlines which make writing unstable ([`6b5c394`](https://github.com/GitoxideLabs/gitoxide/commit/6b5c394f49282a8d09c2a9ffece840e4683572db))
    - Fix section headline level ([`9d6f263`](https://github.com/GitoxideLabs/gitoxide/commit/9d6f263beef289d227dec1acc2d4240087cb9be6))
    - Write first version of changlogs thus far… ([`719b6bd`](https://github.com/GitoxideLabs/gitoxide/commit/719b6bdf543b8269ccafad9ad6b46e0c55efaa38))
    - Parse more user generated section content, adapt existing changelogs to work correctly ([`2f43a54`](https://github.com/GitoxideLabs/gitoxide/commit/2f43a54298e7ecfff2334627df149fe0882b5d1d))
 * **[#222](https://github.com/GitoxideLabs/gitoxide/issues/222)**
    - Update changelogs prior to release ([`9a493d0`](https://github.com/GitoxideLabs/gitoxide/commit/9a493d0651b0b6d71cf230dc510a658be7f8cb19))
    - Stabilize changelogs ([`920e832`](https://github.com/GitoxideLabs/gitoxide/commit/920e83219911df1c440d3fe42fd5ec3a295b0bb8))
    - Update changelogs prior to release ([`b3e2252`](https://github.com/GitoxideLabs/gitoxide/commit/b3e2252f7461a003d9a4612da60ba931dd8c0bef))
 * **[#250](https://github.com/GitoxideLabs/gitoxide/issues/250)**
    - Move loose header manipulation from git-pack to git-object ([`598698b`](https://github.com/GitoxideLabs/gitoxide/commit/598698b88c194bc0e6ef69539f9fa7246ebfab70))
 * **[#254](https://github.com/GitoxideLabs/gitoxide/issues/254)**
    - Adjust changelogs prior to git-pack release ([`6776a3f`](https://github.com/GitoxideLabs/gitoxide/commit/6776a3ff9fa5a283da06c9ec5723d13023a0b267))
 * **[#259](https://github.com/GitoxideLabs/gitoxide/issues/259)**
    - Btree/hashmap free lookup of packs in store, keeping things more bundled ([`a88981b`](https://github.com/GitoxideLabs/gitoxide/commit/a88981b6f38b86624588f0c8ff200d17f38d0263))
 * **[#260](https://github.com/GitoxideLabs/gitoxide/issues/260)**
    - Linked::Store now assures unique IDs across compound stores ([`b605c1f`](https://github.com/GitoxideLabs/gitoxide/commit/b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e))
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - More explicit information about how much garbaged is in the slotmap ([`cfd36ee`](https://github.com/GitoxideLabs/gitoxide/commit/cfd36ee60172a13d147dea06c9ad5db76501053a))
    - Assure stable handles can actually access the indices hey need ([`9474a43`](https://github.com/GitoxideLabs/gitoxide/commit/9474a439dd884d4d7a54e87beab212a0b0ced030))
    - A failing test to show the handle-stability doesn't quite work yet ([`5562e88`](https://github.com/GitoxideLabs/gitoxide/commit/5562e8888cd8ac8fc3d89a41f8e8cc5cec7b8ca6))
    - Refactor ([`c499843`](https://github.com/GitoxideLabs/gitoxide/commit/c499843485a8af102cb4d3594c4e6014976c5aa0))
    - Docs for dynamic object store ([`2c2a2e9`](https://github.com/GitoxideLabs/gitoxide/commit/2c2a2e9d69e917983518ee91c66c96e90d34850e))
    - Default handle refresh mode is the least surprising, with option to configure ([`1b74c14`](https://github.com/GitoxideLabs/gitoxide/commit/1b74c14c99a3076753f166dc1a6a4451bca490d2))
    - Remove unused dependencies ([`c800fdd`](https://github.com/GitoxideLabs/gitoxide/commit/c800fdd331e6d7a0b8d756ba822915259f26e9e8))
    - Refactor ([`b88f253`](https://github.com/GitoxideLabs/gitoxide/commit/b88f253e46e7ad0a50b670b96c1bfa09eaaecaef))
    - Refactor ([`52a4dcd`](https://github.com/GitoxideLabs/gitoxide/commit/52a4dcd3a6969fa8f423ab39c875f98f9d210e95))
    - Refactor ([`3da91ce`](https://github.com/GitoxideLabs/gitoxide/commit/3da91ce78ae582989cc1da136a1884dfe21de2b3))
    - Move `sink::Sink` to the top-level exclusively ([`ab4e726`](https://github.com/GitoxideLabs/gitoxide/commit/ab4e726fcec65871a81056a9c69af8ea3f56b2a3))
    - Dynamic store module cleanu ([`494772c`](https://github.com/GitoxideLabs/gitoxide/commit/494772cec65a5c29d0c45e4f130cc9a8ee2023b9))
    - Adapt to changes in git-odb ([`a44dd4b`](https://github.com/GitoxideLabs/gitoxide/commit/a44dd4b5d1910856d7a21e156e7bca3138c04484))
    - Move `loose::iter::Iter` to `loose::Iter` ([`8bb5c9a`](https://github.com/GitoxideLabs/gitoxide/commit/8bb5c9a75cd91ae0d888bc8e93707cfc9cc08090))
    - Minor improvements to module layout, docs ([`0364f48`](https://github.com/GitoxideLabs/gitoxide/commit/0364f48ff4cd25bd4e73524651b68e65694bb5f7))
    - Fix docs ([`360bf9d`](https://github.com/GitoxideLabs/gitoxide/commit/360bf9d759e724b164a38f8f59423bf0b63d2d0c))
    - Make single-threaded programs possible to use with git-repository ([`dde5c6b`](https://github.com/GitoxideLabs/gitoxide/commit/dde5c6ba76ff849f69f742c985b4bc65ca830883))
    - A more suitable iterator implementation for general store ([`af0cc5f`](https://github.com/GitoxideLabs/gitoxide/commit/af0cc5f27488df497bc929492bb13386aa88cc2a))
    - A quick and dirty version index iteration ([`0384007`](https://github.com/GitoxideLabs/gitoxide/commit/0384007cd9e813cf4bfb13642adef8a602d219ad))
    - Use new store in git-repository ([`2f9e342`](https://github.com/GitoxideLabs/gitoxide/commit/2f9e342b63f9e5c925d8e85ebc0a0be693ca0901))
    - Use new odb in place of the old one and it works ([`8ad25c5`](https://github.com/GitoxideLabs/gitoxide/commit/8ad25c581bc79041545a72baf57b0a469d99cc30))
    - Remaining methods of git-pack::Find ([`92b9764`](https://github.com/GitoxideLabs/gitoxide/commit/92b97644e9828ab84727135d42fdab5101340a3f))
    - Make find::Entry self-contained ([`ad36fb9`](https://github.com/GitoxideLabs/gitoxide/commit/ad36fb9b800c17931ce358ac262bef40d43dcfb3))
    - Remove iterator access in favor of fully owned data ([`62d3f10`](https://github.com/GitoxideLabs/gitoxide/commit/62d3f106437e597a41aae592da28f48e8736b143))
    - It shows that we can't return anything referenced from the interior-mutable handle ([`b9f308b`](https://github.com/GitoxideLabs/gitoxide/commit/b9f308b6f3e5961b7ebeffbbc412cc9398fa0e2a))
    - Refactor ([`6cb474e`](https://github.com/GitoxideLabs/gitoxide/commit/6cb474ec2e50e0a7cc5bfaff49a1e866279e8269))
    - Prepare implementation of location-dependent methods ([`5de29f4`](https://github.com/GitoxideLabs/gitoxide/commit/5de29f4440bb0258bf2262ee210e1a511b74a4d3))
    - Refactor ([`2c23f42`](https://github.com/GitoxideLabs/gitoxide/commit/2c23f42dc8b965aca5043652355aecb978084fa5))
    - Impl git_odb::Write for general::Handle ([`b7a6ab7`](https://github.com/GitoxideLabs/gitoxide/commit/b7a6ab71788643039313193869e96b4e9364d7ba))
    - Cleanup ([`a4f3670`](https://github.com/GitoxideLabs/gitoxide/commit/a4f36704e0243555c3d021f2925fbb15c396a026))
    - A way to predict the amount of slots needed for smooth operation ([`a3a16d6`](https://github.com/GitoxideLabs/gitoxide/commit/a3a16d6abfb245654353aaf1df0c9d932b250bf3))
    - Finding objects and dynamically loading packs seems to work ([`8f58c30`](https://github.com/GitoxideLabs/gitoxide/commit/8f58c30e3a6251b318514e468fc03d743d85e4f3))
    - First sketch of looking up objects with pack cache ([`a0aae84`](https://github.com/GitoxideLabs/gitoxide/commit/a0aae843d32bc0ebf672e5fe268e4fd1fd1fc51e))
    - Add contains checks for libgit2 ([`c64a45a`](https://github.com/GitoxideLabs/gitoxide/commit/c64a45a6453d72227260839cd93b8e8fa54d7357))
    - Add MRU to contains() even faster contains checks ([`6525847`](https://github.com/GitoxideLabs/gitoxide/commit/65258476dfc4ce35c30dbb48066f510f3de08073))
    - Load an index right after refreshing items from disk ([`0c40eb3`](https://github.com/GitoxideLabs/gitoxide/commit/0c40eb3f73c10eb58f7704e2a3f1fb3171bc4300))
    - Some more assertions for contains() regarding refresh mode ([`3f8c540`](https://github.com/GitoxideLabs/gitoxide/commit/3f8c540625a14c33f26eece6f1b89997db5abab9))
    - Try reusing mappings, but no LRU ([`bb602e8`](https://github.com/GitoxideLabs/gitoxide/commit/bb602e8ab22bfb48043e26d2ecfd5380056582c5))
    - Try LRU-like contains implementation ([`2d6960f`](https://github.com/GitoxideLabs/gitoxide/commit/2d6960f886c1165f0bdb6f2d653388e1e0b57a2d))
    - Don't abort run into assertion unnecessarily… ([`4e87a56`](https://github.com/GitoxideLabs/gitoxide/commit/4e87a56cef4644e592d93999e064adbc436e3cec))
    - Adjust object-acess to test new contains method ([`8488b41`](https://github.com/GitoxideLabs/gitoxide/commit/8488b41651751d9177f53a23233b7ddd655dd696))
    - Looks like 'contains()' is implemented well enough ([`c24015a`](https://github.com/GitoxideLabs/gitoxide/commit/c24015a076963ed0feceadf9269c52f7910ad033))
    - First successful loading of indices… ([`8cbef57`](https://github.com/GitoxideLabs/gitoxide/commit/8cbef57aa2ad82591521dd9ff7e529e3171f6153))
    - First stab at loading indices while dealing with inherent raciness ([`94be3a0`](https://github.com/GitoxideLabs/gitoxide/commit/94be3a0f3e51a3377e8a263e4e3b638c9b64cf0e))
    - Assure loops can't happen anymore ([`f04ff80`](https://github.com/GitoxideLabs/gitoxide/commit/f04ff8011198b7f6c45c2094530903316c6e91ea))
    - The first green test for loose object lookup ([`0c6b7b1`](https://github.com/GitoxideLabs/gitoxide/commit/0c6b7b13dd532350d649d932e93ff016170a1860))
    - More clarity around generations; actually trash slots or unset them ([`7bce101`](https://github.com/GitoxideLabs/gitoxide/commit/7bce101ac4e085b3d035069f51ea99f945961990))
    - Create a new index snapshot and store it ([`41d91f9`](https://github.com/GitoxideLabs/gitoxide/commit/41d91f96cfc6ca41902a268537b6865f7df867fa))
    - Sort out race condition around slots that change identity ([`6e678e7`](https://github.com/GitoxideLabs/gitoxide/commit/6e678e705172c988de2a849352b4b83f898eeaa4))
    - Uncover slightly disturbing races which make it hard to ever release/unload maps ([`cbf2d13`](https://github.com/GitoxideLabs/gitoxide/commit/cbf2d13afaee11cd835872c60063bac885829751))
    - Remaining comments about multi-pack-index handling ([`569c40b`](https://github.com/GitoxideLabs/gitoxide/commit/569c40b7b8c468901266e24c4b8dbde0d0365912))
    - Handling of multi-pack index slot-map moves as they change ([`456f1e7`](https://github.com/GitoxideLabs/gitoxide/commit/456f1e7f1bb72a28c31aa3ff80e488cd81888900))
    - Prepare correct handling of multi-pack indices when the time comes ([`6388ba2`](https://github.com/GitoxideLabs/gitoxide/commit/6388ba2f82adced3f3f268f248063b9cb5a6d269))
    - More thoughts about how to continue a search… ([`eac8c45`](https://github.com/GitoxideLabs/gitoxide/commit/eac8c45ae5961cc6a19d803b589c5f816fddcc5e))
    - Get closer to actually setting up slots ([`55645ae`](https://github.com/GitoxideLabs/gitoxide/commit/55645ae8a9d3f7666f5c6f9d28302b7afeb355ed))
    - Add load-pack method frame to not forget ([`e1ec535`](https://github.com/GitoxideLabs/gitoxide/commit/e1ec535ed16010c7d8d4dec659d0b3d6ab8863e5))
    - Some steps towards recording the disk state in the slot-map ([`5074f4c`](https://github.com/GitoxideLabs/gitoxide/commit/5074f4ce711e443cc456ddb7e6ce978610331ded))
    - Better errors for disk consolidation; setup loose object dbs ([`4c13e14`](https://github.com/GitoxideLabs/gitoxide/commit/4c13e149bdd111cf7ddb171881c8ac95c88f3808))
    - Detail the load-indices flow more ([`11d3325`](https://github.com/GitoxideLabs/gitoxide/commit/11d3325dec84b50dab41ed12ba52a1b939ac3003))
    - Make slot-count configurable ([`c910af5`](https://github.com/GitoxideLabs/gitoxide/commit/c910af59806aa65e2b56a81357fd3b21e877791c))
    - Provide handle with a snapshot of the store's state ([`6e0cd6d`](https://github.com/GitoxideLabs/gitoxide/commit/6e0cd6d38c5df874990ace6c2c3c0b39342c4d05))
    - Refactor ([`d5565da`](https://github.com/GitoxideLabs/gitoxide/commit/d5565daf9ece760af99e6b6c7c711440b95cd359))
    - Support for metrics in general store handle ([`11b98b8`](https://github.com/GitoxideLabs/gitoxide/commit/11b98b8d2a0ecafaf6b57efafa3ed8ac40b00298))
    - First test to trigger all major code-paths ([`25b56c5`](https://github.com/GitoxideLabs/gitoxide/commit/25b56c5257230db296453a2ac74f73e694d0af77))
    - More trustworthy state-id hashing ([`4eb43d0`](https://github.com/GitoxideLabs/gitoxide/commit/4eb43d03e4954fa6d4ab4afb5f13ad5e801ba58a))
    - Use handle registration to avoid unloading packs; fix state-id hash ([`a1070de`](https://github.com/GitoxideLabs/gitoxide/commit/a1070de8b750acbbaf14925d827574465ea69804))
    - Handle registration ([`df4e4eb`](https://github.com/GitoxideLabs/gitoxide/commit/df4e4ebd116d0d80e6a8b09bb5ea30ecfdbfbae9))
    - Bring in the slotmap ([`3a5cb5f`](https://github.com/GitoxideLabs/gitoxide/commit/3a5cb5ffd326dba8e1f8d6ca725c4d2815864a9b))
    - Put down more types for loading of indices and refresh logic ([`9909eaf`](https://github.com/GitoxideLabs/gitoxide/commit/9909eaf363e8681454ea1dd2b953fe8757c23463))
    - Add all types the handle would have to store ([`e2f0cb0`](https://github.com/GitoxideLabs/gitoxide/commit/e2f0cb07b9c23360ee3cc68b7093fc742ecef16d))
    - Rename `Handle` to `Cache` ([`580e96c`](https://github.com/GitoxideLabs/gitoxide/commit/580e96c1b2d9782a2e8cf9d1123f6d53a5376a3d))
    - First sketch of general store ([`fc1b640`](https://github.com/GitoxideLabs/gitoxide/commit/fc1b6409380256b73cf271c105802f4494dbb8c5))
    - More affirmative notes about multi-pack indices ([`dceaea2`](https://github.com/GitoxideLabs/gitoxide/commit/dceaea2486afcfce053eac1ad41851cddce9c5e7))
    - Remove CRC32 check entirely as it doesn't seem to be important in the big picture ([`22d35bd`](https://github.com/GitoxideLabs/gitoxide/commit/22d35bdbc271ccada8d68a1450d9a2533fc739ee))
    - Notes about multi-pack indices in the current data::entry::location ([`7eff6bf`](https://github.com/GitoxideLabs/gitoxide/commit/7eff6bf525ea48fa913149911ea4c8fe742a25a3))
    - Adjust to new name/place of `bundle::Location` ([`1f8954d`](https://github.com/GitoxideLabs/gitoxide/commit/1f8954d7b7567990055fe7d1752599c6a0a90931))
    - Add docs for handle-related functions ([`cf1b1e6`](https://github.com/GitoxideLabs/gitoxide/commit/cf1b1e6d82f691ab17975e4f1479d93720368803))
    - Impl Write for Arc, Rc and shared borrows ([`5cdc27d`](https://github.com/GitoxideLabs/gitoxide/commit/5cdc27df2f5a25668f5c8bce4cbe1dcb0262ccc1))
    - Adapt to changes in git-repository ([`3ab9b03`](https://github.com/GitoxideLabs/gitoxide/commit/3ab9b03eee7d449b7bb87cb7dcbf164fdbe4ca48))
    - Also use object cache if there is no pack-cache ([`3e1ae25`](https://github.com/GitoxideLabs/gitoxide/commit/3e1ae256d380eca1cb66f9348287bcf9bdfcefca))
    - Remove `make_object_cache` parameter from `git_pack::data::output::count::objects()` ([`3f05fea`](https://github.com/GitoxideLabs/gitoxide/commit/3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7))
    - Cache-creators are indeed shared across threads, must be sync ([`c326cb3`](https://github.com/GitoxideLabs/gitoxide/commit/c326cb35cc684a5751e007c0ece3f02edf162ecc))
    - Try to make Handle usable for pack creation ([`424c9b3`](https://github.com/GitoxideLabs/gitoxide/commit/424c9b3a2b467f5a1e339700257cd4ab72e2e692))
    - Make all handle caches and cache creators optional ([`3c30769`](https://github.com/GitoxideLabs/gitoxide/commit/3c307694708532a9903ce7ff7138d0d4ab17dba5))
    - Make odb handle more general ([`2be6725`](https://github.com/GitoxideLabs/gitoxide/commit/2be6725b76b5f4a10dbadc73b2a7dfc3c1537223))
    - :Find for Arc and Rc ([`b1c82a7`](https://github.com/GitoxideLabs/gitoxide/commit/b1c82a7959fba1541642fc8dfae46b27848f2ba3))
    - MultiPackIndex compatible pack::Find trait definition ([`5fa1a9d`](https://github.com/GitoxideLabs/gitoxide/commit/5fa1a9dce59c2654374a532d024c8de5959d4d0f))
    - Git_pack::Find::try_find_cached(…, pack_cache) ([`58c2edb`](https://github.com/GitoxideLabs/gitoxide/commit/58c2edb76755ab71e10eef4cd9a51533825c291f))
    - Construct a handle from a linked store ([`9702ed4`](https://github.com/GitoxideLabs/gitoxide/commit/9702ed492114a64c8dab0fedc3e251c9e059a753))
    - Refactor ([`ab14f70`](https://github.com/GitoxideLabs/gitoxide/commit/ab14f70202a5a80369a640eff809003925bccb58))
    - A first tiny stab at Handle for linked store ([`ef08f7f`](https://github.com/GitoxideLabs/gitoxide/commit/ef08f7fd186dca5ff0d02e8b57c8d14e6b584e65))
    - Refactor ([`1361c31`](https://github.com/GitoxideLabs/gitoxide/commit/1361c31d43438cb14723003194236fe7d1fdc201))
    - Refactor ([`3310d8f`](https://github.com/GitoxideLabs/gitoxide/commit/3310d8f271f74fc6084e33dd9bd4c5f01b54e432))
    - :Find implementation for linked::Store ([`9235106`](https://github.com/GitoxideLabs/gitoxide/commit/9235106986e14551a28693bfe4ea92f046c65406))
    - Add linked::Store::rc_iter() ([`36fde72`](https://github.com/GitoxideLabs/gitoxide/commit/36fde720c34e02429a810ddd43b894a37516f51a))
    - Use `Deref` instead of Borrow in linked ODB iterator ([`a96b1e5`](https://github.com/GitoxideLabs/gitoxide/commit/a96b1e5e626f58bc4d79d44132b4f03bd3a7cfd1))
    - Remove pack-cache from `Find::try_find(…)` ([`ebc7f47`](https://github.com/GitoxideLabs/gitoxide/commit/ebc7f47708a63c3df4415ba0e702660d976dfb3e))
    - Linked::Store sorts bundles by modification date, newest first ([`a81b333`](https://github.com/GitoxideLabs/gitoxide/commit/a81b33359a4394a66f854195445f8f9aa0a46179))
    - Move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait ([`2290d00`](https://github.com/GitoxideLabs/gitoxide/commit/2290d006705ff47ad780b009fe58ee422b3285af))
    - A simplified version of the `Find` trait ([`e25f4ea`](https://github.com/GitoxideLabs/gitoxide/commit/e25f4eadec679406aad6df10026e27e4832c2482))
    - Add 'contains()' method to Find ([`dfdd6fb`](https://github.com/GitoxideLabs/gitoxide/commit/dfdd6fb2c83e5d09c3a56936723bc6749ac4b99a))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Add missing docs ([`4137327`](https://github.com/GitoxideLabs/gitoxide/commit/41373274fc7f23e3fed17dc52e3e3e94c2e9e41a))
    - Very first experimental support for multi-pack index verification ([`bb35c69`](https://github.com/GitoxideLabs/gitoxide/commit/bb35c6994765ec3bbbcfde247911d1ffe711a23d))
    - Multi-index verify checksum ([`853d468`](https://github.com/GitoxideLabs/gitoxide/commit/853d4683aae5f4dd4667b452932bd57f99f6afab))
    - Another test to validate index stability ([`1fb08df`](https://github.com/GitoxideLabs/gitoxide/commit/1fb08dfb00e951c0851aaf8b0d9b64bcdf5a3e3e))
    - Refactor ([`a4ad5bf`](https://github.com/GitoxideLabs/gitoxide/commit/a4ad5bf948f405bdf00847859a95aa50082e0615))
    - Handle 'move' of multi-pack indices ([`aec0895`](https://github.com/GitoxideLabs/gitoxide/commit/aec0895c1d4a47727e530b1e80b5289db966ab91))
    - Trigger last portion of multi-index logic ([`b46979b`](https://github.com/GitoxideLabs/gitoxide/commit/b46979b9aa57b718ed38acbaf9ca5cd7b775a6fc))
    - Load slots early to avoid races with the 'generation' field ([`10bc3ab`](https://github.com/GitoxideLabs/gitoxide/commit/10bc3ab6c4407fa1fdb966d3fc880aa772c490d3))
    - Only open multi-pack indices if the mtime changed ([`efe2579`](https://github.com/GitoxideLabs/gitoxide/commit/efe2579b70c078f42b65077461a63abe1f1b6950))
    - Add remaining Store docs ([`e440bcd`](https://github.com/GitoxideLabs/gitoxide/commit/e440bcdd7525e7f79a889b55610f64ff00827a2c))
    - First odb lookup with multi-index works ([`a8773df`](https://github.com/GitoxideLabs/gitoxide/commit/a8773dfd728a0e996a5d4853508e5560e3bbb03d))
    - Generalize intra-pack offset lookup ([`dff05ef`](https://github.com/GitoxideLabs/gitoxide/commit/dff05ef0c242a05e234d441d044f9b294ebefe0c))
    - Replace bare u32 `data::Id` typedef ([`e0b8636`](https://github.com/GitoxideLabs/gitoxide/commit/e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c))
    - A rough implementation of everything multi-index support would need… ([`56f174f`](https://github.com/GitoxideLabs/gitoxide/commit/56f174f37e1a0fa671bc54be2a8d16878eab75a6))
    - Make opened multi-pack indices representable ([`28e648d`](https://github.com/GitoxideLabs/gitoxide/commit/28e648d642c89422fdc23292582d601b79601262))
    - First sketch towards reading in multi-indices ([`25eb157`](https://github.com/GitoxideLabs/gitoxide/commit/25eb157ccb9f02b404e66f0d953228cd56424436))
    - `Cache::inner` removed in favor of `Deref/Mut` and `into_inner()` ([`bf73a94`](https://github.com/GitoxideLabs/gitoxide/commit/bf73a94b43288b6634dbb33f2433656987a73baf))
    - Cargo fmt ([`8b9da35`](https://github.com/GitoxideLabs/gitoxide/commit/8b9da35b3e0d3458efcac150f7062c9d7382a6c4))
    - Change accessors named `hash_kind()` to `object_hash()` for consistency ([`2ef9a84`](https://github.com/GitoxideLabs/gitoxide/commit/2ef9a8424af51310db8c1e6df31dde9953ed3d21))
    - Consistently use `object_hash` instead of `hash_kind` ([`49998cc`](https://github.com/GitoxideLabs/gitoxide/commit/49998cce419a27f3928ec4ac39da5e3b500e5cb2))
    - Refactor ([`7331e99`](https://github.com/GitoxideLabs/gitoxide/commit/7331e99cb88df19f7b1e04b1468584e9c7c79913))
    - Adapt to changes in git-pack ([`28dba20`](https://github.com/GitoxideLabs/gitoxide/commit/28dba20d0ba6197d02e1c9b665279392dad8d707))
    - Adjust to changes in git-pack ([`b8f109e`](https://github.com/GitoxideLabs/gitoxide/commit/b8f109efa3bf3b9f37033bddd841fdf55e450dd4))
    - Refactor ([`9b5451a`](https://github.com/GitoxideLabs/gitoxide/commit/9b5451a071ea303216c750dc94f2f646fc3c366f))
    - Loose object iteration respects hash kind ([`72eb9da`](https://github.com/GitoxideLabs/gitoxide/commit/72eb9da4e2aa543ea902c49e0477274bd631c7c3))
    - Remove `Write::*(…, hash_kind)` ([`67c42fb`](https://github.com/GitoxideLabs/gitoxide/commit/67c42fbf5f88f8dc42a9ebd7c6276d57ba1d4624))
    - Various changes to the `loose::Store` ([`ad1b9ea`](https://github.com/GitoxideLabs/gitoxide/commit/ad1b9ea17eb4b98ebd2fddebe82a8fee1d63e9dd))
    - Loose odb doesn't hard-code sha1 anymore for `find()` and `contains()` ([`68f1031`](https://github.com/GitoxideLabs/gitoxide/commit/68f1031732cfb8f50fa582ced834f7d4fd9da2ae))
    - First pieces of header parsing; allow to respect multi-index desired hash kind in git-odb ([`1a2a049`](https://github.com/GitoxideLabs/gitoxide/commit/1a2a04930ab56ba778091e10b15cecf415f5058d))
    - Respect `core.multiPackIndex` option ([`1495efc`](https://github.com/GitoxideLabs/gitoxide/commit/1495efcc914449f9680f9141805d60b1f3188001))
    - Only load multi-pack indices if allowed ([`b22e146`](https://github.com/GitoxideLabs/gitoxide/commit/b22e14663dd0237a0ee1f5283782953ec8442a16))
 * **[#287](https://github.com/GitoxideLabs/gitoxide/issues/287)**
    - Basic output for 'repo verify' json only ([`9f8d61f`](https://github.com/GitoxideLabs/gitoxide/commit/9f8d61f164fb3fbdb76cc44fbd634ca5db35b3b8))
    - Way nicer progress messages for repo verification ([`4b4f9f8`](https://github.com/GitoxideLabs/gitoxide/commit/4b4f9f81879ad181744022eb0d7dc02392a5e91e))
    - Upgrade to prodash 17 ([`47860b7`](https://github.com/GitoxideLabs/gitoxide/commit/47860b7e2769260cfb8522ae455c491605093423))
    - Better verify progress printing ([`4a464f2`](https://github.com/GitoxideLabs/gitoxide/commit/4a464f26bb4dae236bcc845e353e25dd5d936f8f))
    - Refactor ([`831397c`](https://github.com/GitoxideLabs/gitoxide/commit/831397c99fee9f2d6758124d993386cca5534f7b))
    - Refactor ([`38426a1`](https://github.com/GitoxideLabs/gitoxide/commit/38426a171844014201282a441ebfc7d1f4cfff94))
    - Very rough version of repository verification ([`80a4a7a`](https://github.com/GitoxideLabs/gitoxide/commit/80a4a7add688d16376b9bf2ed7f1c7f655b7c912))
    - Support for loose object statistics in odb store ([`53d835a`](https://github.com/GitoxideLabs/gitoxide/commit/53d835a4a8a2830c56a354a3e7f5c1790ee40ad1))
    - Bare-bones loose object integrity check ([`3dfec81`](https://github.com/GitoxideLabs/gitoxide/commit/3dfec817e4980b19a3d0eb1301d8f33296b2fbcf))
    - Frame for loose-db validation ([`a24307d`](https://github.com/GitoxideLabs/gitoxide/commit/a24307dfd0b7322472f85ec83687a04488d28cff))
    - First basic validation of all packs within an odb store ([`d63176f`](https://github.com/GitoxideLabs/gitoxide/commit/d63176f2cf0a869285a7434311a8adfdfbf552c7))
    - Don't reset generations, instead make them match the current one ([`1d995ef`](https://github.com/GitoxideLabs/gitoxide/commit/1d995ef672e7bfcb142a369ac034a802e02ad124))
    - Frame for interity check on object store ([`b5dd059`](https://github.com/GitoxideLabs/gitoxide/commit/b5dd059343f0545c77129595f3801a946cfe8c5f))
    - Cleanup and unify `verify_integrity()` method signature ([`91d0476`](https://github.com/GitoxideLabs/gitoxide/commit/91d047658b114f372735116c9d8e6962a3873137))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - Refactor ([`9b28b18`](https://github.com/GitoxideLabs/gitoxide/commit/9b28b18262c763608d60fba65e91fcb9ca3ddb3e))
 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Handle::packed_object_count() ([`84ec54e`](https://github.com/GitoxideLabs/gitoxide/commit/84ec54e904378c5b3d7da9efff66b02e88b16916))
    - Docs ([`a45f378`](https://github.com/GitoxideLabs/gitoxide/commit/a45f3789696078848e2e96ddb8a55570c941dd53))
    - Implement ODB::disambiguate_prefix(…) ([`7d4d281`](https://github.com/GitoxideLabs/gitoxide/commit/7d4d2818395cfe0c31117f8736471d4a707e3feb))
    - Add missing docs ([`c7cd4ab`](https://github.com/GitoxideLabs/gitoxide/commit/c7cd4ab9a2ba2674be2dcd97a16f549e14679149))
    - Refactor ([`4455af3`](https://github.com/GitoxideLabs/gitoxide/commit/4455af376a583fe524855d9199faa0a374b2ab31))
    - Refactor ([`fe87704`](https://github.com/GitoxideLabs/gitoxide/commit/fe87704d037ae4cef5d48abd7dfc152f37a43c97))
    - Lookup_prefix for ODB ([`a4ccd18`](https://github.com/GitoxideLabs/gitoxide/commit/a4ccd18fb3662f9fa38f73693abe9364c77df3d3))
    - Base for testing git_odb::Handle::lookup_prefix() ([`6244c06`](https://github.com/GitoxideLabs/gitoxide/commit/6244c06900a1c66b2d62bd12015aaae86ac1c842))
    - Basics for ODB lookup prefix ([`5c228e1`](https://github.com/GitoxideLabs/gitoxide/commit/5c228e1596b0ace28a1261ca271c3f1d93eb3970))
    - Add docs to `loose::Store::lookup_prefix(…)` ([`9fa4817`](https://github.com/GitoxideLabs/gitoxide/commit/9fa4817f24ad1d31e20da92e307c853cdae758c1))
    - Loose::Store::lookup_prefix(…) ([`996bfb3`](https://github.com/GitoxideLabs/gitoxide/commit/996bfb3061fd9ee2cf38c93f39e0d4c7c6163386))
    - Simplify error of loose::Iter ([`622abd7`](https://github.com/GitoxideLabs/gitoxide/commit/622abd756c4d950076974e91dbe1a144b61ca3b1))
    - Upgrade parking_lot and cargo_toml ([`f95c1a0`](https://github.com/GitoxideLabs/gitoxide/commit/f95c1a0d9c19bcc6feb9b8739a09d86f9970a0e0))
    - Support Rust 1.52 ([`9c14de3`](https://github.com/GitoxideLabs/gitoxide/commit/9c14de391a1a9f1055922164d1757c9aa9720807))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
    - Adapt to changes in git-path ([`cc2d810`](https://github.com/GitoxideLabs/gitoxide/commit/cc2d81012d107da7a61bf4de5b28342dea5083b7))
    - Use `git-path` crate instead of `git_features::path` ([`47e607d`](https://github.com/GitoxideLabs/gitoxide/commit/47e607dc256a43a3411406c645eb7ff04239dd3a))
    - Make fmt ([`50ff7aa`](https://github.com/GitoxideLabs/gitoxide/commit/50ff7aa7fa86e5e2a94fb15aab86470532ac3f51))
    - Adapt to changes in git-quote ([`ba48629`](https://github.com/GitoxideLabs/gitoxide/commit/ba486297114154c334e0c38cd883504608973f3c))
    - Use git-quote crate in git-odb alternate parsing ([`8e49aa6`](https://github.com/GitoxideLabs/gitoxide/commit/8e49aa6090c1c361e3ddd44798754c44c179ab49))
    - All path-related tests are green ([`81d2bf2`](https://github.com/GitoxideLabs/gitoxide/commit/81d2bf2ec5f571245d56eb853306d07ede3010a2))
    - Update git-odb changelog to include information about bugfix ([`055b117`](https://github.com/GitoxideLabs/gitoxide/commit/055b1170b9a96c9c6067a588fcd9679b618d9530))
    - Fix the first race-condition around initialization in ODB ([`a891315`](https://github.com/GitoxideLabs/gitoxide/commit/a89131517fd4805211c4037396d9411ee41363d1))
    - Conversions from Rc to arc for Handle ([`c19331e`](https://github.com/GitoxideLabs/gitoxide/commit/c19331e001e587e4fca74f3e9fec28a7df922c0a))
    - Less restrictive ref-delta-base resolution ([`917480b`](https://github.com/GitoxideLabs/gitoxide/commit/917480b6626363555ba818c8e1c4e18cb944aa40))
    - More safety around recursion and invariants when resolving ref-deltas ([`dddb4a5`](https://github.com/GitoxideLabs/gitoxide/commit/dddb4a51f417ff84a53da64959ad668ab26ebd93))
    - Allow delta base objects to be out-of-pack in general odb ([`d4f1590`](https://github.com/GitoxideLabs/gitoxide/commit/d4f1590a6afe25fbd6659002c420098c57e1824a))
    - Elaborate odb info and simple entries printing ([`0f65282`](https://github.com/GitoxideLabs/gitoxide/commit/0f65282fd2719234f745473e33bd42637be5fd3b))
    - A first sketch of access odb information using a sub-command ([`89b628a`](https://github.com/GitoxideLabs/gitoxide/commit/89b628ab5b833a34f0b426b3a399bb182e63f3f4))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - Document all features related to serde1 ([`72b97f2`](https://github.com/GitoxideLabs/gitoxide/commit/72b97f2ae4dc7642b160f183c6d5df4502dc186f))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Adjustments due to breaking changes in `git_path` ([`4420ae9`](https://github.com/GitoxideLabs/gitoxide/commit/4420ae932d5b20a9662a6d36353a27111b5cd672))
    - Adapt to changes in git_features::path to deal with Result ([`bba4c68`](https://github.com/GitoxideLabs/gitoxide/commit/bba4c680c627a418efbd25f14bd168df19b8dedd))
 * **[#333](https://github.com/GitoxideLabs/gitoxide/issues/333)**
    - Use git_features::path everywhere where there is a path conversion ([`2e1437c`](https://github.com/GitoxideLabs/gitoxide/commit/2e1437cb0b5dc77f2317881767f71eaf9b009ebf))
 * **[#364](https://github.com/GitoxideLabs/gitoxide/issues/364)**
    - A statement about replacement objects ([`2d32f4d`](https://github.com/GitoxideLabs/gitoxide/commit/2d32f4d77efc6861323ec7de827cfe8db2c3c7a1))
    - Implement object replacement ([`b16d5e9`](https://github.com/GitoxideLabs/gitoxide/commit/b16d5e9a5dcc5d3ad8275a8c793df50bb112eb3e))
    - Add some precaution to avoid strange interactions with packs ([`b052a9a`](https://github.com/GitoxideLabs/gitoxide/commit/b052a9a3e9127fd9a4029594ea9de6e436db03c6))
    - An API and a test for replacement configuration ([`f2d6db1`](https://github.com/GitoxideLabs/gitoxide/commit/f2d6db16f89bc70f1d167975cbd88937c4d38cfb))
    - Initial test to assure we don't replace objects by default ([`6cb9ecc`](https://github.com/GitoxideLabs/gitoxide/commit/6cb9ecc4ef2602606a90880880ea7deccaaa0729))
 * **[#384](https://github.com/GitoxideLabs/gitoxide/issues/384)**
    - No need to isolate archives by crate name ([`19d46f3`](https://github.com/GitoxideLabs/gitoxide/commit/19d46f35440419b9911b6e2bca2cfc975865dce9))
    - Add archive files via git-lfs ([`7202a1c`](https://github.com/GitoxideLabs/gitoxide/commit/7202a1c4734ad904c026ee3e4e2143c0461d51a2))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - Make fmt ([`4b320e7`](https://github.com/GitoxideLabs/gitoxide/commit/4b320e773368ac5e8c38dd8a779ef3d6d2d024ec))
    - Add `Cache::has_object_cache()` and `Cache::has_pack_cache()` methods. ([`81e1a9d`](https://github.com/GitoxideLabs/gitoxide/commit/81e1a9d38aac9e6dd0618266ff826593e038cce8))
    - Improve error messages when objects aren't found ([`41d4943`](https://github.com/GitoxideLabs/gitoxide/commit/41d494365d281056c5e9466860db808bd85143e9))
    - Fix: incorrect desired object kind if retrieved object doesn't have the expected kind ([`87f974e`](https://github.com/GitoxideLabs/gitoxide/commit/87f974eea2cf7c6e3405b2816d3ef2bd058fc3dc))
    - Assure index ambiguous object range can represent 'none found' ([`5ffe54f`](https://github.com/GitoxideLabs/gitoxide/commit/5ffe54ff88f026139474658fb470742751126119))
    - Avoid allocating index entries in case of ambiguity by using a range ([`4db4754`](https://github.com/GitoxideLabs/gitoxide/commit/4db47547fa405542efd38b475e3e430548b9d160))
    - Provide optional `candidates` for ambigious entries during `lookup_prefix()` ([`95210cb`](https://github.com/GitoxideLabs/gitoxide/commit/95210cb2ba85f75148b4ef48ccea9d9f8a0a0114))
    - `loose::Db` and `Store` can return all candidate objects for a single prefix ([`92d8be1`](https://github.com/GitoxideLabs/gitoxide/commit/92d8be1101a7e76e70cd90db6a943b9e31e20802))
    - The first successful disambiguation test ([`6bc6337`](https://github.com/GitoxideLabs/gitoxide/commit/6bc6337037708243346afeee07ad24a02565894b))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#536](https://github.com/GitoxideLabs/gitoxide/issues/536)**
    - Try even harder to find the original index during recursive base object resolution. ([`bae3ea9`](https://github.com/GitoxideLabs/gitoxide/commit/bae3ea954af560de871fa7d506146e70041b5fca))
    - Try to recreate multi-threaded panic, without success. ([`53b5086`](https://github.com/GitoxideLabs/gitoxide/commit/53b50866bcf9785904f1d8f809375edae9cee6f1))
    - Only run single-threaded these when respective toggle is set (when multi-threading is requested) ([`730384d`](https://github.com/GitoxideLabs/gitoxide/commit/730384d6c8908ab8b44e9b499f3f7d678143e8ed))
 * **[#59](https://github.com/GitoxideLabs/gitoxide/issues/59)**
    - Fix initializing pack bundles in compound db ([`5a48e08`](https://github.com/GitoxideLabs/gitoxide/commit/5a48e085d49a191a85a9b043e34d844389c8342b))
    - Add failing test ([`d629339`](https://github.com/GitoxideLabs/gitoxide/commit/d629339834479553ceef27c15e5115e820b875ee))
    - Move pack fixtures into place which resembles an actual object db ([`fb5cea4`](https://github.com/GitoxideLabs/gitoxide/commit/fb5cea4b9a98997f105a6ccb9729371be994af3c))
 * **[#63](https://github.com/GitoxideLabs/gitoxide/issues/63)**
    - Impl == and != for common combinations of ObjectId/oid ([`2455178`](https://github.com/GitoxideLabs/gitoxide/commit/24551781cee4fcf312567ca9270d54a95bc4d7ae))
    - Git-protocol uses `oid` type ([`3930a6f`](https://github.com/GitoxideLabs/gitoxide/commit/3930a6ff508f5bb2249fb2c2f21e00b74fecda22))
    - Use new `oid` where possible in git-odb ([`68a709e`](https://github.com/GitoxideLabs/gitoxide/commit/68a709e0337d4969138d30a5c25d60b7dbe51a73))
    - Refactor; better errors for invalid hash sizes ([`be84b36`](https://github.com/GitoxideLabs/gitoxide/commit/be84b36129694a2e89d1b81d932f2eba23aedf54))
    - Make ObjectId/oid happen! ([`ca78d15`](https://github.com/GitoxideLabs/gitoxide/commit/ca78d15373ec988d909be8f240baefe75555e077))
    - Remove all public exports of git-hash types in git-object ([`accf89d`](https://github.com/GitoxideLabs/gitoxide/commit/accf89d25560e5ded6f44a1c4a898ee65d14f8f6))
    - Remove re-export of git_object::borrowed::Id ([`a3f2816`](https://github.com/GitoxideLabs/gitoxide/commit/a3f28169c1268c1129852f279631d5a7f7540cdf))
 * **[#67](https://github.com/GitoxideLabs/gitoxide/issues/67)**
    - Assure pack-ids are actually unique, the simple way… ([`0509b4f`](https://github.com/GitoxideLabs/gitoxide/commit/0509b4fb5a78a3e4bfcacbeb661d262f8592884a))
    - The very first version of complete pack writing ([`4d76d53`](https://github.com/GitoxideLabs/gitoxide/commit/4d76d53aabb956ed7c8a45c883486ec5596bcaa3))
    - A sketch of the pack::generation function signature ([`21b0aab`](https://github.com/GitoxideLabs/gitoxide/commit/21b0aab81e7304da964dbef90c806134073ccef3))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **[#724](https://github.com/GitoxideLabs/gitoxide/issues/724)**
    - Fix typo proper ([`ffc99b8`](https://github.com/GitoxideLabs/gitoxide/commit/ffc99b8edaacdb20c940624549acb4a4ee7f8c66))
 * **[#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)**
    - Prepare changelogs prior to release ([`8c0bca3`](https://github.com/GitoxideLabs/gitoxide/commit/8c0bca37ff9fbaadbe55561fb2b0d649980c95b1))
 * **Uncategorized**
    - Release gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`59e9fac`](https://github.com/GitoxideLabs/gitoxide/commit/59e9fac67d1b353e124300435b55f6b5468d7deb))
    - Release gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`48f5bd2`](https://github.com/GitoxideLabs/gitoxide/commit/48f5bd2014fa3dda6fbd60d091065c5537f69453))
    - Release gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`a5869e0`](https://github.com/GitoxideLabs/gitoxide/commit/a5869e0b223406820bca836e3e3a7fae2bfd9b04))
    - Release gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`41d57b9`](https://github.com/GitoxideLabs/gitoxide/commit/41d57b98964094fc1528adb09f69ca824229bf25))
    - Release gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`e313112`](https://github.com/GitoxideLabs/gitoxide/commit/e31311257bd138b52042dea5fc40c3abab7f269b))
    - Release gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6efd0d3`](https://github.com/GitoxideLabs/gitoxide/commit/6efd0d31fbeca31ab7319aa2ac97bb31dc4ce055))
    - Release gix-date v0.4.2, gix-hash v0.10.2, gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6ccc88a`](https://github.com/GitoxideLabs/gitoxide/commit/6ccc88a8e4a56973b1a358cf72dc012ee3c75d56))
    - Apparently some fixtures were changed, so here are the archived ([`e49aed9`](https://github.com/GitoxideLabs/gitoxide/commit/e49aed9d1d264030c73e431e7cd291d27546927b))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
    - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
    - Rename `git-odb` to `gix-odb` ([`93047bb`](https://github.com/GitoxideLabs/gitoxide/commit/93047bb8bbb09982cd23ae6d78a0a90a9ec5c2a9))
    - Adjust to renaming of `git-index` to `gix-index` ([`86db5e0`](https://github.com/GitoxideLabs/gitoxide/commit/86db5e09fc58ce66b252dc13b8d7e2c48e4d5062))
    - Adjust to renaming of `git-diff` to `gix-diff` ([`49a163e`](https://github.com/GitoxideLabs/gitoxide/commit/49a163ec8b18f0e5fcd05a315de16d5d8be7650e))
    - Adjust to renaming of `git-commitgraph` to `gix-commitgraph` ([`f1dd0a3`](https://github.com/GitoxideLabs/gitoxide/commit/f1dd0a3366e31259af029da73228e8af2f414244))
    - Adjust to renaming of `git-mailmap` to `gix-mailmap` ([`2e28c56`](https://github.com/GitoxideLabs/gitoxide/commit/2e28c56bb9f70de6f97439818118d3a25859698f))
    - Adjust to renaming of `git-discover` to `gix-discover` ([`53adfe1`](https://github.com/GitoxideLabs/gitoxide/commit/53adfe1c34e9ea3b27067a97b5e7ac80b351c441))
    - Adjust to renaming of `git-lfs` to `gix-lfs` ([`b9225c8`](https://github.com/GitoxideLabs/gitoxide/commit/b9225c830daf1388484ee7e05f727990fdeff43c))
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
    - Adjust to renaming of `git-glob` to `gix-glob` ([`35b2a3a`](https://github.com/GitoxideLabs/gitoxide/commit/35b2a3acbc8f2a03f151bc0a3863163844e0ca86))
    - Adjust to renaming of `git-sec` to `gix-sec` ([`eabbb92`](https://github.com/GitoxideLabs/gitoxide/commit/eabbb923bd5a32fc80fa80f96cfdc2ab7bb2ed17))
    - Adapt to renaming of `git-path` to `gix-path` ([`d3bbcfc`](https://github.com/GitoxideLabs/gitoxide/commit/d3bbcfccad80fc44ea8e7bf819f23adaca06ba2d))
    - Adjust to rename of `git-config-value` to `gix-config-value` ([`622b3e1`](https://github.com/GitoxideLabs/gitoxide/commit/622b3e1d0bffa0f8db73697960f9712024fac430))
    - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
    - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
    - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
    - Prepare changelogs prior to release ([`7c846d2`](https://github.com/GitoxideLabs/gitoxide/commit/7c846d2102dc767366771925212712ef8cc9bf07))
    - Merge branch 'Lioness100/main' ([`1e544e8`](https://github.com/GitoxideLabs/gitoxide/commit/1e544e82455bf9ecb5e3c2146280eaf7ecd81f16))
    - Undo typo-fix which reversed the meaning of the word. ([`eb232ff`](https://github.com/GitoxideLabs/gitoxide/commit/eb232ffa7c6c939eb8286e07eed295d585afbe37))
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Make fmt ([`e22080e`](https://github.com/GitoxideLabs/gitoxide/commit/e22080e4a29d0bad15a99d565a5e3e304a8743ec))
    - Merge branch 'adjustments-for-cargo' ([`7bba270`](https://github.com/GitoxideLabs/gitoxide/commit/7bba2709488b7eb999b8136dbab03af977241678))
    - Break cyclical dev dependencies ([`1fea18f`](https://github.com/GitoxideLabs/gitoxide/commit/1fea18f5f8b4189a23dc4fa3f041a672f6fbcfb3))
    - Use enumerations to advertise progress ids publicly. ([`d792ea5`](https://github.com/GitoxideLabs/gitoxide/commit/d792ea543246632bf1ca8d0e1d239bbe7f07e219))
    - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
    - Prepare changelogs prior to release ([`93bef97`](https://github.com/GitoxideLabs/gitoxide/commit/93bef97b3c0c75d4bf7119fdd787516e1efc77bf))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`9e04685`](https://github.com/GitoxideLabs/gitoxide/commit/9e04685dd3f109bfb27663f9dc7c04102e660bf2))
    - Release git-ref v0.23.0, git-config v0.15.0, git-command v0.2.2, git-diff v0.26.0, git-discover v0.12.0, git-mailmap v0.9.0, git-pack v0.30.0, git-odb v0.40.0, git-transport v0.25.2, git-protocol v0.26.1, git-revision v0.10.0, git-refspec v0.7.0, git-worktree v0.12.0, git-repository v0.32.0 ([`ffb5b6a`](https://github.com/GitoxideLabs/gitoxide/commit/ffb5b6a21cb415315db6fd5294940c7c6deb4538))
    - Prepare changelogs prior to release ([`4381a03`](https://github.com/GitoxideLabs/gitoxide/commit/4381a03a34c305f31713cce234c2afbf8ac60f01))
    - Release git-date v0.4.0, git-actor v0.17.0, git-object v0.26.0, git-traverse v0.22.0, git-index v0.12.0, safety bump 15 crates ([`0e3d0a5`](https://github.com/GitoxideLabs/gitoxide/commit/0e3d0a56d7e6a60c6578138f2690b4fa54a2072d))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Prepare changelogs prior to release ([`30d8ca1`](https://github.com/GitoxideLabs/gitoxide/commit/30d8ca19284049dcfbb0de2698cafae1d1a16b0c))
    - Make fmt ([`511ed00`](https://github.com/GitoxideLabs/gitoxide/commit/511ed0000397a5b268530c8f5362e7d25b7c1594))
    - Release git-features v0.25.1, git-url v0.12.2, git-odb v0.38.1, git-transport v0.24.2, git-repository v0.30.2 ([`bb0a07b`](https://github.com/GitoxideLabs/gitoxide/commit/bb0a07b5edd5f980989d1a92e74df7f183febe87))
    - Add test to verify we don't panic on a corrupt loose object ([`391adeb`](https://github.com/GitoxideLabs/gitoxide/commit/391adeb69a73310baa3f08afa2c0a9aea7cfaf7c))
    - Merge branch 'loose-find-panic' ([`95cccdd`](https://github.com/GitoxideLabs/gitoxide/commit/95cccddd3c181eb2a85b12823c27beb054adf5d8))
    - Flatten errors into one ([`46636e6`](https://github.com/GitoxideLabs/gitoxide/commit/46636e64c9a48ec0e85e014ac0cc8b48846d8462))
    - Refactor ([`d305a3a`](https://github.com/GitoxideLabs/gitoxide/commit/d305a3a8af77e7857c3f9b9866d103960ec024e1))
    - Remove panic-assertions in `loose` `lookup_prefix` ([`4fffa9a`](https://github.com/GitoxideLabs/gitoxide/commit/4fffa9a9198cf3012fa8215796aab3d456519ff3))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Merge branch 'odb-iteration' ([`693a469`](https://github.com/GitoxideLabs/gitoxide/commit/693a46977e2b57b93ee921320e008c8ad1beb81b))
    - Assure deltas are counted correctly, even if the base is out of pack. ([`ddaf47f`](https://github.com/GitoxideLabs/gitoxide/commit/ddaf47f023970e8acfb98e8874da22f2604a92d9))
    - Support for pack-order when iterating objects. ([`e9d1f45`](https://github.com/GitoxideLabs/gitoxide/commit/e9d1f45e944e91bb9715a3ee89a4f28b09250411))
    - Merge branch 'read-header' ([`3d01252`](https://github.com/GitoxideLabs/gitoxide/commit/3d0125271ec7bd606734bd74757a7e31a18c7ce5))
    - Adjust to changes in `git-odb` ([`50ea7fb`](https://github.com/GitoxideLabs/gitoxide/commit/50ea7fba30c752f86609fabf579a8a038b505c17))
    - Add `Store::try_header()` for obtaining object information quickly. ([`d9d05b0`](https://github.com/GitoxideLabs/gitoxide/commit/d9d05b0db6b4453e7385117d466bf7c2e8de81fa))
    - Adapt to changes in `git-pack` ([`b1724ef`](https://github.com/GitoxideLabs/gitoxide/commit/b1724efab49f6e656531e540b68315822ddafd22))
    - Add `loose::Store::try_header()` to obtain loose object information without content. ([`7f19bd7`](https://github.com/GitoxideLabs/gitoxide/commit/7f19bd7e63d78e3151e43d5094ae9d35cbe34f46))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - Thanks clippy ([`f1160fb`](https://github.com/GitoxideLabs/gitoxide/commit/f1160fb42acf59b37cbeda546a7079af3c9bc050))
    - Adapt to changes in `git-features::fs`. ([`35f7d59`](https://github.com/GitoxideLabs/gitoxide/commit/35f7d5960210738d88d35aef9c1ed3480681c481))
    - Adjust to changes in `git-testtools` ([`4eb842c`](https://github.com/GitoxideLabs/gitoxide/commit/4eb842c7150b980e1c2637217e1f9657a671cea7))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Adapt to changes in `git-transport` ([`527c62e`](https://github.com/GitoxideLabs/gitoxide/commit/527c62ef034a961a7e2b1dd1868cf8f81cc2eedc))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
    - Upgrade to `prodash 21.1` and add `Ids` to all progress instances. ([`c8835c6`](https://github.com/GitoxideLabs/gitoxide/commit/c8835c6edae784c9ffcb69a674c0a6545dbb2af3))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'cwd-consistency' ([`ea7c6a3`](https://github.com/GitoxideLabs/gitoxide/commit/ea7c6a3b069c9e13905b51b87538c57ba9182dca))
    - `alternate::resolve(…)` now takes the current_dir as argument. ([`1fabdc5`](https://github.com/GitoxideLabs/gitoxide/commit/1fabdc51b9468ba2c6b8cf74509ad5aa2a0b86f4))
    - Merge branch 'jpgrayson/main' ([`b242853`](https://github.com/GitoxideLabs/gitoxide/commit/b242853abd790e5234b2f18b4aaeddb8f6f4d36f))
    - Remove `git config` statements from fixtures that didn't need them (anymore). ([`578ea79`](https://github.com/GitoxideLabs/gitoxide/commit/578ea799e9ec10f7142a7fc207d43ef301308c6d))
    - Disable tag.gpgSign in test scripts ([`1ce3190`](https://github.com/GitoxideLabs/gitoxide/commit/1ce3190000f6211ce31468c7603d491bb5b90293))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Merge branch 'main' into http-config ([`7c5b37d`](https://github.com/GitoxideLabs/gitoxide/commit/7c5b37d28e98f59a6847368a0d0166d2dbb4acc1))
    - Release git-diff v0.22.0, git-index v0.7.1, git-pack v0.26.0, git-odb v0.36.0, git-transport v0.21.2, git-repository v0.27.0, safety bump 6 crates ([`f0cab31`](https://github.com/GitoxideLabs/gitoxide/commit/f0cab317bb0c2799fa80d16f3ae1b89d6aee4284))
    - Prepare changelogs prior to release ([`f5f3a9e`](https://github.com/GitoxideLabs/gitoxide/commit/f5f3a9edd038a89c8c6c4da02054e5439bcc0071))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index (upgrade to Rust 1.65) ([`5406630`](https://github.com/GitoxideLabs/gitoxide/commit/5406630466145990b5adbdadb59151036993060d))
    - Thanks clippy ([`04cfa63`](https://github.com/GitoxideLabs/gitoxide/commit/04cfa635a65ae34ad6d22391f2febd2ca7eabca9))
    - Adjust memory-size expectations to deal with Rust 1.65 and below ([`a93c470`](https://github.com/GitoxideLabs/gitoxide/commit/a93c4703699ea61a646c82b861c9345715a6c057))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Merge branch 'main' into fetch-pack ([`d686020`](https://github.com/GitoxideLabs/gitoxide/commit/d6860205db847b8a474756e92578195e1022481c))
    - Thanks clippy ([`b9937ad`](https://github.com/GitoxideLabs/gitoxide/commit/b9937adc2c31095dde63397be7d56f1ea559b0f7))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'fix-odb-race' ([`b862fc5`](https://github.com/GitoxideLabs/gitoxide/commit/b862fc52dd2409e912c892c7f428a571f565b64a))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0 ([`f5c36d8`](https://github.com/GitoxideLabs/gitoxide/commit/f5c36d85755d1f0f503b77d9a565fad6aecf6728))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
    - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
    - Merge branch 'main' into filter-refs-by-spec ([`1f6e5ab`](https://github.com/GitoxideLabs/gitoxide/commit/1f6e5ab15f5fd8d23719b13e6aea59cd231ac0fe))
    - Merge branch 'fix-522' ([`5869e9f`](https://github.com/GitoxideLabs/gitoxide/commit/5869e9ff2508d5a93c07635277af8764fcb57713))
    - Release git-hash v0.9.9 ([`da0716f`](https://github.com/GitoxideLabs/gitoxide/commit/da0716f8c27b4f29cfff0e5ce7fcb3d7240f4aeb))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Release git-path v0.4.2, git-config-value v0.7.0 ([`c48fb31`](https://github.com/GitoxideLabs/gitoxide/commit/c48fb3107d29f9a06868b0c6de40567063a656d1))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-object v0.20.3, git-ref v0.15.4, git-config v0.7.1, git-diff v0.18.0, git-traverse v0.16.3, git-pack v0.22.0, git-odb v0.32.0, git-url v0.7.3, git-transport v0.19.3, git-protocol v0.19.1, git-refspec v0.1.1, git-repository v0.23.0, safety bump 6 crates ([`85a3bed`](https://github.com/GitoxideLabs/gitoxide/commit/85a3bedd68d2e5f36592a2f691c977dc55298279))
    - Release git-features v0.22.3, git-revision v0.4.4 ([`c2660e2`](https://github.com/GitoxideLabs/gitoxide/commit/c2660e2503323531ba02519eaa51124ee22fec51))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Release git-path v0.4.1 ([`5e82346`](https://github.com/GitoxideLabs/gitoxide/commit/5e823462b3deb904f5d6154a7bf114cef1988224))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
    - Prepare changelogs prior to reelase ([`c06ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/c06ae1c606b6af9c2a12021103d99c2810750d60))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
    - Merge branch 'main' into write-index-v2 ([`a938986`](https://github.com/GitoxideLabs/gitoxide/commit/a938986877302c197d1aed087594c5605416fe5f))
    - Merge branch 'main' into remote-ls-refs ([`de61c4d`](https://github.com/GitoxideLabs/gitoxide/commit/de61c4db7855d6925d66961f62ae3d12cc4acf78))
    - Thanks clippy ([`4bd747c`](https://github.com/GitoxideLabs/gitoxide/commit/4bd747cb3e126fe5b1d540270cfbd731cffd42ef))
    - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Merge branch 'kianmeng-fix-typos' ([`4e7b343`](https://github.com/GitoxideLabs/gitoxide/commit/4e7b34349c0a01ad8686bbb4eb987e9338259d9c))
    - Fix typos ([`e9fcb70`](https://github.com/GitoxideLabs/gitoxide/commit/e9fcb70e429edb2974afa3f58d181f3ef14c3da3))
    - Release git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0 ([`aa639d8`](https://github.com/GitoxideLabs/gitoxide/commit/aa639d8c43f3098cc4a5b50614c5ae94a8156928))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Prepare changelog prior to release ([`3c50625`](https://github.com/GitoxideLabs/gitoxide/commit/3c50625fa51350ec885b0f38ec9e92f9444df0f9))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Make fmt ([`0700b09`](https://github.com/GitoxideLabs/gitoxide/commit/0700b09d6828849fa2470df89af1f75a67bfb27d))
    - Assure document-features are available in all 'usable' and 'early' crates ([`238581c`](https://github.com/GitoxideLabs/gitoxide/commit/238581cc46c7288691eed37dc7de5069e3d86721))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`0e9df36`](https://github.com/GitoxideLabs/gitoxide/commit/0e9df364c4cddf006b1de18b8d167319b7cc1186))
    - Use git_path::realpath in all places that allow it right now ([`229dc91`](https://github.com/GitoxideLabs/gitoxide/commit/229dc917fc7d9241b85e5818260a6fbdd3a5daaa))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Update changelogs prior to release ([`bb424f5`](https://github.com/GitoxideLabs/gitoxide/commit/bb424f51068b8a8e762696890a55ab48900ab980))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0 ([`349c590`](https://github.com/GitoxideLabs/gitoxide/commit/349c5904b0dac350838a896759d51576b66880a7))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Merge branch 'main' into repo-status ([`0eb2372`](https://github.com/GitoxideLabs/gitoxide/commit/0eb23721dca78f6e6bf864c5c3a3e44df8b419f0))
    - Merge branch 'test-archive-support' ([`350df01`](https://github.com/GitoxideLabs/gitoxide/commit/350df01042d6ca8b93f8737fa101e69b50535a0f))
    - Merge branch 'main' into repo-status ([`4086335`](https://github.com/GitoxideLabs/gitoxide/commit/40863353a739ec971b49410fbc2ba048b2762732))
    - Merge branch 'worktree-stack' ([`e90d3fd`](https://github.com/GitoxideLabs/gitoxide/commit/e90d3fd0a9764511e6280596f21d3a0494ed7021))
    - Release git-config v0.2.1, git-diff v0.15.0, git-traverse v0.14.0, git-pack v0.18.0, git-odb v0.28.0, git-ref v0.12.1, git-revision v0.1.0, git-repository v0.16.0, gitoxide-core v0.14.0, gitoxide v0.12.0, safety bump 6 crates ([`b612021`](https://github.com/GitoxideLabs/gitoxide/commit/b612021683ba709b693bd48aef3e2e3c2f5b9ead))
    - Remove deprecated compound and linked object databases ([`8c5ae77`](https://github.com/GitoxideLabs/gitoxide/commit/8c5ae77f06a64c57df9a9ad1190266896a223dbe))
    - Release git-diff v0.14.0, git-bitmap v0.1.0, git-index v0.2.0, git-tempfile v2.0.1, git-lock v2.0.0, git-mailmap v0.1.0, git-traverse v0.13.0, git-pack v0.17.0, git-quote v0.2.0, git-odb v0.27.0, git-packetline v0.12.4, git-url v0.4.0, git-transport v0.16.0, git-protocol v0.15.0, git-ref v0.12.0, git-worktree v0.1.0, git-repository v0.15.0, cargo-smart-release v0.9.0, safety bump 5 crates ([`e58dc30`](https://github.com/GitoxideLabs/gitoxide/commit/e58dc3084cf17a9f618ae3a6554a7323e44428bf))
    - Merge branch 'for-onefetch' ([`8e5cb65`](https://github.com/GitoxideLabs/gitoxide/commit/8e5cb65da75036a13ed469334e7ae6c527d9fff6))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Make fmt ([`7cf3545`](https://github.com/GitoxideLabs/gitoxide/commit/7cf354509b545f7e7c99e159b5989ddfbe86273d))
    - Merge branch 'main' into mailmap ([`b2df941`](https://github.com/GitoxideLabs/gitoxide/commit/b2df941feaf5ae9fa170fa49270189f3527f2eab))
    - Merge branch 'describe-rev' ([`77b7cd9`](https://github.com/GitoxideLabs/gitoxide/commit/77b7cd9a7813aaa1a15d035ea42c1e3fe4eef8dd))
    - Adapt to breaking changes in git-actor ([`40c48c3`](https://github.com/GitoxideLabs/gitoxide/commit/40c48c390eb796b427ebd516dde92e9538ce5fb7))
    - Merge branch 'short-id' ([`5849d5b`](https://github.com/GitoxideLabs/gitoxide/commit/5849d5b326b83f98a16cf1d956c720c7f0fd4445))
    - Thanks clippy ([`4618f8a`](https://github.com/GitoxideLabs/gitoxide/commit/4618f8aa7648c0553a8e1b023fceb6738654e38b))
    - Merge branch 'svetli-n-path_value' ([`e8383ca`](https://github.com/GitoxideLabs/gitoxide/commit/e8383caf6db211beb57d70019fe4ad13ce9066ee))
    - Release git-tempfile v2.0.0, safety bump 6 crates ([`90b1c42`](https://github.com/GitoxideLabs/gitoxide/commit/90b1c42d5487904a9f329362d185b035d0ddb975))
    - Merge branch 'unify-path-encoding' ([`566ff8a`](https://github.com/GitoxideLabs/gitoxide/commit/566ff8a3597b889899d41ca15e5b9af7e05f1a4b))
    - Release git-hash v0.9.2, git-object v0.17.1, git-pack v0.16.1 ([`0db19b8`](https://github.com/GitoxideLabs/gitoxide/commit/0db19b8deaf11a4d4cbc03fa3ae40eea104bc302))
    - Merge branch 'index-verification' ([`ad3c803`](https://github.com/GitoxideLabs/gitoxide/commit/ad3c8032cee02052ef3940d1d7c950270a0a299a))
    - Implement `git_odb::Find` for `git_odb::Handle` ([`3522aef`](https://github.com/GitoxideLabs/gitoxide/commit/3522aef37bcaa285e14d7a84cdca67321f9125bb))
    - Release git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`42ebb53`](https://github.com/GitoxideLabs/gitoxide/commit/42ebb536cd6086f096b8422291776c9720fa0948))
    - Release git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`1b76119`](https://github.com/GitoxideLabs/gitoxide/commit/1b76119259b8168aeb99cbbec233f7ddaa2d7d2c))
    - Release git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`8f57c29`](https://github.com/GitoxideLabs/gitoxide/commit/8f57c297d7d6ed68cf51415ea7ede4bf9263326e))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Prepare changelogs for release ([`674ec73`](https://github.com/GitoxideLabs/gitoxide/commit/674ec73b0816baa2c63b4ef1b40b7a41849c5e95))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Merge branch 'use-midx-in-store' ([`338521b`](https://github.com/GitoxideLabs/gitoxide/commit/338521b0443b9dc1007581de42ef6a950f6e0bbf))
    - Release git-chunk v0.2.0, safety bump 4 crates ([`b792fab`](https://github.com/GitoxideLabs/gitoxide/commit/b792fabf9f5f93ab906ac5a5bb3e4f01c179290a))
    - Refactor ([`c09a44d`](https://github.com/GitoxideLabs/gitoxide/commit/c09a44db8e2bf6f45ebcd7423ab7438308557c49))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`bf4694c`](https://github.com/GitoxideLabs/gitoxide/commit/bf4694c895ac7e73f22e6424808269b91f17003f))
    - Thanks clippy ([`17af184`](https://github.com/GitoxideLabs/gitoxide/commit/17af184b7e24e39ce78b6c07dbced5d530ef3d6d))
    - Thanks clippy ([`123a95e`](https://github.com/GitoxideLabs/gitoxide/commit/123a95ed2a7eef30fc2071769d96469dc17b2195))
    - Thanks clippy ([`4ca9e07`](https://github.com/GitoxideLabs/gitoxide/commit/4ca9e07c7ac062d48d64ad7b516274e32dbc51c6))
    - Make fmt ([`066f3ff`](https://github.com/GitoxideLabs/gitoxide/commit/066f3ffb8740f242c1b03e680c3c5c1a0e4c36c3))
    - Release git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0 ([`d3f9227`](https://github.com/GitoxideLabs/gitoxide/commit/d3f922781a81e8fbb81aa47afdbe9afeb06d666b))
    - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
    - Adjust changelogs prior to release ([`ec38950`](https://github.com/GitoxideLabs/gitoxide/commit/ec3895005d141abe79764eaff7c0f04153e38d73))
    - Merge branch 'pack-consistency' ([`5982406`](https://github.com/GitoxideLabs/gitoxide/commit/5982406b4e1b26fd383d9ec21a3cf652ec8ab25f))
    - Merge branch 'git-loose-objects' of https://github.com/xmo-odoo/gitoxide into xmo-odoo-git-loose-objects ([`ee737cd`](https://github.com/GitoxideLabs/gitoxide/commit/ee737cd237ad70bf9f2c5e0d3e4557909e495bca))
    - Release git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0 ([`f606fa9`](https://github.com/GitoxideLabs/gitoxide/commit/f606fa9a0ca338534252df8921cd5e9d3875bf94))
    - Better changelog descriptions. ([`f69b2d6`](https://github.com/GitoxideLabs/gitoxide/commit/f69b2d627099639bc144fd94fde678d84a10d6f7))
    - Adjusting changelogs prior to release of git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0, safety bump 5 crates ([`39b40c8`](https://github.com/GitoxideLabs/gitoxide/commit/39b40c8c3691029cc146b893fa0d8d25d56d0819))
    - Move "loose object header" ser/de to git-object ([`3d1565a`](https://github.com/GitoxideLabs/gitoxide/commit/3d1565acfc336baf6487edccefd72d0226141a08))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Don't let 'a' leak out of the tempdir to fix #212 ([`682dff8`](https://github.com/GitoxideLabs/gitoxide/commit/682dff83520fa7a09c88ce83fd1d6439c377a480))
    - More proper fix for #212 ([`dfc1493`](https://github.com/GitoxideLabs/gitoxide/commit/dfc1493de348c166cd11eb6af3c145c994126472))
    - Naive fix for #212 ([`b31863c`](https://github.com/GitoxideLabs/gitoxide/commit/b31863cc643833db5a7ac2edfe38b942dd15790c))
    - Update changelogs just for fun ([`21541b3`](https://github.com/GitoxideLabs/gitoxide/commit/21541b3301de1e053fc0e84373be60d2162fbaae))
    - Merge branch 'changelog-generation' ([`bf0106e`](https://github.com/GitoxideLabs/gitoxide/commit/bf0106ea21734d4e59d190b424c22743c22da966))
    - Bump git-traverse v0.9.0, safety bump 8 crates ([`d39fabb`](https://github.com/GitoxideLabs/gitoxide/commit/d39fabb8757369aa19452a457f610fe21dc13a14))
    - Release git-odb v0.21.3 ([`223f930`](https://github.com/GitoxideLabs/gitoxide/commit/223f93075a28dd49f44505c039cfeae5a7296914))
    - [smart-release #195] fix docs ([`8d7e132`](https://github.com/GitoxideLabs/gitoxide/commit/8d7e132d055d8c87ea3e45de15593964a61b0608))
    - Release git-odb v0.21.2 ([`d443644`](https://github.com/GitoxideLabs/gitoxide/commit/d44364445cfbae861ce45df8bddec1b34e03f454))
    - Bump git-pack v0.11.0 ([`5ae6ff5`](https://github.com/GitoxideLabs/gitoxide/commit/5ae6ff52cd2cd1ccd1e26bb987c154eb19603696))
    - Bump git-object v0.14.0 ([`d4fc81f`](https://github.com/GitoxideLabs/gitoxide/commit/d4fc81f6390443f8c8561d91ac27ea4a6318fb62))
    - [repository #164] Prepare `commit()` for a possible less-allocating future ([`0fd01f7`](https://github.com/GitoxideLabs/gitoxide/commit/0fd01f7071c785c27c56d2c034aac8dcdf690677))
    - [repository #164] generic write_object() ([`c569f83`](https://github.com/GitoxideLabs/gitoxide/commit/c569f83363489dde03c8b9cd01e75d35f5e04dbc))
    - [repository #164] Support for refreshing the object database ([`46e10f8`](https://github.com/GitoxideLabs/gitoxide/commit/46e10f863e1fea419483a7b086022c16cd0ca226))
    - [odb #164] Add refresh() functionality ([`ee16d04`](https://github.com/GitoxideLabs/gitoxide/commit/ee16d041941a5777c8f6495a28f7633c327cbd6b))
    - Release git-odb v0.21.1 ([`cb33c2f`](https://github.com/GitoxideLabs/gitoxide/commit/cb33c2f71c2e1e228ba0d2299fb531cf2a5f3b23))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - [odb #190] Read all eligble packed refs, no "pack-" prefix needed ([`ab250f7`](https://github.com/GitoxideLabs/gitoxide/commit/ab250f76b356c0937ada959591dc4df3872acf8f))
    - Bump git-pack v0.10.0 ([`e5e3c80`](https://github.com/GitoxideLabs/gitoxide/commit/e5e3c8024e1c2e5e90cee83abbdae41d58eee156))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - [repository #174] adjust various changelogs ([`081faf5`](https://github.com/GitoxideLabs/gitoxide/commit/081faf5c3a21b34b7068b44d8206fb5770c392f5))
    - [odb #180] fix docs ([`bd50752`](https://github.com/GitoxideLabs/gitoxide/commit/bd50752dd9188acd92b8503db53cc2ce8112c61f))
    - [odb #180] refactor ([`eff21da`](https://github.com/GitoxideLabs/gitoxide/commit/eff21dae1083042412f45cd2f7a0faaf7d6400e6))
    - Bump git-odb v0.21.0 ([`7b9854f`](https://github.com/GitoxideLabs/gitoxide/commit/7b9854fb35e86958a5ca827ec9a55b1168f38395))
    - [odb #180] add changelog ([`acf1193`](https://github.com/GitoxideLabs/gitoxide/commit/acf1193e6b72433d4b74ec9fd39de148529224c5))
    - [pack #179] refactor bundle ([`420dca2`](https://github.com/GitoxideLabs/gitoxide/commit/420dca29bccca6e7d759880d8342f23b33eead0d))
    - [pack #179] refactor ([`ab6554b`](https://github.com/GitoxideLabs/gitoxide/commit/ab6554b0cd5838f1ea4e82f6b5019798288076fa))
    - [object #177] move mutable objects to crate::* ([`c551c02`](https://github.com/GitoxideLabs/gitoxide/commit/c551c0236c64f3237cb9be7f35159f753d4b871f))
    - [object #177] migrate immutable::tree to crate::tree ([`fa5cd06`](https://github.com/GitoxideLabs/gitoxide/commit/fa5cd0648d5c855060ab2b75ee933851987c2dcf))
    - [object #177] move immutable::* to crate::*Ref, start `iter` adjustments ([`461dc53`](https://github.com/GitoxideLabs/gitoxide/commit/461dc53ba3bc07d55fdb4aad7570ba9176a8b360))
    - [object #177] rename immutable::* to immutable::*Ref ([`6deb012`](https://github.com/GitoxideLabs/gitoxide/commit/6deb01291fb382b7fb9206682e319afa81bacc05))
    - Release git-object v0.13.0 ([`708fc5a`](https://github.com/GitoxideLabs/gitoxide/commit/708fc5abd8af4dd7459f388c7092bf35915c6662))
    - [actor #173] rename immutable::Signature to SignatureRef! ([`96461ac`](https://github.com/GitoxideLabs/gitoxide/commit/96461ace776d6b351b313d4f2697f2d95b9e196e))
    - Release git-pack v0.9.0 ([`7fbc961`](https://github.com/GitoxideLabs/gitoxide/commit/7fbc9617da97d4ba4bb3784f41d4163c0839c03c))
    - [pack #67] refactor ([`14717f6`](https://github.com/GitoxideLabs/gitoxide/commit/14717f6132672a5d271832a68de0b323b73abb2a))
    - Release git-odb v0.20.2 ([`6fb8bbb`](https://github.com/GitoxideLabs/gitoxide/commit/6fb8bbb11e87911424c95001ce851bc4665920e9))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - Release git-odb v0.20.1 ([`ca3f736`](https://github.com/GitoxideLabs/gitoxide/commit/ca3f736ae3e6a0a5541223364db874a8e31dd3ec))
    - Remove dev-dependency cycles by removing their version ([`c40faca`](https://github.com/GitoxideLabs/gitoxide/commit/c40faca41632cd2a226daf4ddf5293b65d1fdc82))
    - Release git-diff v0.8.0, git-odb v0.20.0, git-pack v0.8.0, git-traverse v0.7.0 ([`f123f69`](https://github.com/GitoxideLabs/gitoxide/commit/f123f69c7a4f9fd1c98bd2f60ebc953a6739fe04))
    - Release git-diff v0.7.0, git-odb v0.19.0, git-pack v0.7.0, git-traverse v0.6.0 ([`c67291f`](https://github.com/GitoxideLabs/gitoxide/commit/c67291ff9bcdff9a747d87241f6a71015607af05))
    - Release git-object v0.12.0 ([`7006150`](https://github.com/GitoxideLabs/gitoxide/commit/7006150ac314d19814608723f69f6e70a72f9262))
    - Release git-actor-0.3.1 ([`727087d`](https://github.com/GitoxideLabs/gitoxide/commit/727087dca243da4bc40bc87611a2f66234565be7))
    - (cargo-release) version 0.18.0 ([`b327590`](https://github.com/GitoxideLabs/gitoxide/commit/b327590d02fec5536c380b2d39dd7be089ca7c40))
    - (cargo-release) version 0.6.0 ([`d704bca`](https://github.com/GitoxideLabs/gitoxide/commit/d704bca7de0a6591f35345c842d6418b36ecd206))
    - (cargo-release) version 0.17.0 ([`c52a491`](https://github.com/GitoxideLabs/gitoxide/commit/c52a49176bd294bb36db74b4293cdb684a2ab7f6))
    - (cargo-release) version 0.5.0 ([`c2f94a5`](https://github.com/GitoxideLabs/gitoxide/commit/c2f94a51bce287be301090450cb00cde57e92f76))
    - (cargo-release) version 0.4.0 ([`d69d0ac`](https://github.com/GitoxideLabs/gitoxide/commit/d69d0ac21989243fdafa514fa41579fd51bc2558))
    - (cargo-release) version 0.11.0 ([`a5be31c`](https://github.com/GitoxideLabs/gitoxide/commit/a5be31c4cf7c0b538a1ed4a52ff5c3a992c6feff))
    - (cargo-release) version 0.3.0 ([`64efc05`](https://github.com/GitoxideLabs/gitoxide/commit/64efc0534ddc372b6e668b23c1e9d276098679c9))
    - (cargo-release) version 0.4.0 ([`70ef344`](https://github.com/GitoxideLabs/gitoxide/commit/70ef3442775b54ba9e4ee9ebfffb37af9804cc5b))
    - [utils #154] refactor: bool.then(||this) - neat ([`1dec1c4`](https://github.com/GitoxideLabs/gitoxide/commit/1dec1c49032c8acb449e463fde41f403cb640e45))
    - (cargo-release) version 0.16.1 ([`8cd173b`](https://github.com/GitoxideLabs/gitoxide/commit/8cd173b32138a136e6109518c179bf7738fe6866))
    - (cargo-release) version 0.3.0 ([`0e9c73a`](https://github.com/GitoxideLabs/gitoxide/commit/0e9c73abd17e0dd21952275077ae53ad7e7aa1af))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - (cargo-release) version 0.16.0 ([`1231dbd`](https://github.com/GitoxideLabs/gitoxide/commit/1231dbd16dacefb39adec8e067c312d313a82e3c))
    - (cargo-release) version 0.2.0 ([`8ff5115`](https://github.com/GitoxideLabs/gitoxide/commit/8ff511583e6d859e43ffda0ef75e2fecce3ed03c))
    - Clippy on tests and thanks clippy ([`a77a71c`](https://github.com/GitoxideLabs/gitoxide/commit/a77a71cf02d328a2a964388928d6b2a235a0aa85))
    - Thanks clippy ([`e1964e4`](https://github.com/GitoxideLabs/gitoxide/commit/e1964e43979b3e32a5d4bfbe377a842d2c0b10ea))
    - [pack] a way to obtain whole bundles for offset-to-index lookup ([`15fcbe2`](https://github.com/GitoxideLabs/gitoxide/commit/15fcbe254b75e8f74652711cc339ae5ade74d24c))
    - [pack] refactor ([`64b1dcd`](https://github.com/GitoxideLabs/gitoxide/commit/64b1dcdb0fb53749ce73017d0dc1e053689d17d4))
    - [pack] bundle::Location with pack offset; order counts by that… ([`f92f285`](https://github.com/GitoxideLabs/gitoxide/commit/f92f285167c6b5bc4d86f255e360c4534e38bb29))
    - Don't use ASM on windows for Sha1 as it fails to build there. ([`ba1fb7a`](https://github.com/GitoxideLabs/gitoxide/commit/ba1fb7ab5bc03f5a23ece32ff1e144544e1eaeae))
    - Remove unnecessary pub(crate) exports ([`3d2456e`](https://github.com/GitoxideLabs/gitoxide/commit/3d2456e11709f0461b37c6df55ecc3861ca4cab5))
    - Bump thiserror from 1.0.25 to 1.0.26 ([`9682590`](https://github.com/GitoxideLabs/gitoxide/commit/9682590095dc3a502b0c84ccd206ca4797635092))
    - [actor] fix gix hours ([`b4e95fd`](https://github.com/GitoxideLabs/gitoxide/commit/b4e95fdbb6664adcb2603d9cb6e6a69182de050f))
    - [actor] git-object uses git-actor ([`d01dd2f`](https://github.com/GitoxideLabs/gitoxide/commit/d01dd2f9e9e8e2b81cdb1131a436d32b5819b731))
    - Thanks clippy ([`3f7e27b`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e27b91e2c7d66959f5f4c1a667f3315111cd6))
    - (cargo-release) version 0.3.0 ([`6b33678`](https://github.com/GitoxideLabs/gitoxide/commit/6b33678f83e6d261ca15c4a7634ff5b4e66d81dd))
    - (cargo-release) version 0.2.0 ([`3286e42`](https://github.com/GitoxideLabs/gitoxide/commit/3286e42547b59df6365087cbae9ce1c9c959faad))
    - Thanks clippy ([`c5b4de8`](https://github.com/GitoxideLabs/gitoxide/commit/c5b4de8c7675da47b5d6325d2993f4ebce4a8f0c))
    - [git-odb] linked::Store can now check if an object exists ([`bb95c79`](https://github.com/GitoxideLabs/gitoxide/commit/bb95c7917a272bfe7eb04bea66685d6a1196dc25))
    - Refactor ([`a25a774`](https://github.com/GitoxideLabs/gitoxide/commit/a25a774675e2e9db1c891351077d3af2fd5c72ed))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - [git-ref] the first failing test ([`7e802a0`](https://github.com/GitoxideLabs/gitoxide/commit/7e802a0576230dfc666c253d484ea255f265f92f))
    - [git-odb] fix test compiilation ([`639bc10`](https://github.com/GitoxideLabs/gitoxide/commit/639bc10e1698beb4c9e7902c2545dd0a3e90e698))
    - [git-odb] much better docs; cleanup exposed API ([`3d5b229`](https://github.com/GitoxideLabs/gitoxide/commit/3d5b229c2605060f2cac9695ff2479777deabdd0))
    - (cargo-release) version 0.2.0 ([`b213628`](https://github.com/GitoxideLabs/gitoxide/commit/b213628feeb8dfa87dab489c7d3155a60e6a236d))
    - [git-odb] refactor ([`2958145`](https://github.com/GitoxideLabs/gitoxide/commit/2958145a0ae1ef582bbf88352f5567d5c2b5eaf0))
    - [git-odb] refactor ([`1eab15d`](https://github.com/GitoxideLabs/gitoxide/commit/1eab15dfb42c819050b0277c4cb6a1045d2fd58d))
    - [git-odb] refactor ([`4967c22`](https://github.com/GitoxideLabs/gitoxide/commit/4967c22340679e953ec6e6319b671455456f93bc))
    - [git-odb] refactor ([`2e68e0c`](https://github.com/GitoxideLabs/gitoxide/commit/2e68e0c9296977eaaf239b8f0ede6e285b13d06c))
    - [git-odb] fix docs ([`936cfd3`](https://github.com/GitoxideLabs/gitoxide/commit/936cfd3af731ed822464765f532dd49a206f596d))
    - [git-pack] compilation ([`b392a55`](https://github.com/GitoxideLabs/gitoxide/commit/b392a55b97a30b10ac0db94a96230e22ea7ab0dc))
    - [git-pack] refactor ([`ea2b3de`](https://github.com/GitoxideLabs/gitoxide/commit/ea2b3deab78882943e11270e4166ca7c340b03e1))
    - [git-pack] refactor ([`5ca2547`](https://github.com/GitoxideLabs/gitoxide/commit/5ca25477c44ff6c606901080e25df57371d9ec9c))
    - [git-pack] refactor ([`157b6ff`](https://github.com/GitoxideLabs/gitoxide/commit/157b6ff7b55ba2b7f8f90f66864212906426f8d7))
    - (cargo-release) version 0.16.0 ([`769c649`](https://github.com/GitoxideLabs/gitoxide/commit/769c649c00c009bf5a3f7c0611a7b999618f2938))
    - [git-pack] refactor ([`be6ddaa`](https://github.com/GitoxideLabs/gitoxide/commit/be6ddaa98fc1dcaf77dc0fd9c9d67754e74927e4))
    - [git-pack] refactor ([`1fab6af`](https://github.com/GitoxideLabs/gitoxide/commit/1fab6af317fd42662c59f82b476917da29cd3c60))
    - [git-pack] refactor ([`e5b00ee`](https://github.com/GitoxideLabs/gitoxide/commit/e5b00ee257b712477413f48448b0bccf9a06bfaf))
    - [git-pack] the world compiles again ([`f0c0e36`](https://github.com/GitoxideLabs/gitoxide/commit/f0c0e36a1fb15d44776678567162ac754fdd26c0))
    - [git-pack] used by git-odb ([`5d6ee07`](https://github.com/GitoxideLabs/gitoxide/commit/5d6ee07a8dec64fe5f68c14c418d922077fad3df))
    - [git-features] refactor to help understand a zlib-related logic bug ([`ae826e8`](https://github.com/GitoxideLabs/gitoxide/commit/ae826e8c3240efd14939beedd33a06695a6c112b))
    - [git-features] a first step towards supporting a pure rust zlib backend ([`040cab7`](https://github.com/GitoxideLabs/gitoxide/commit/040cab7f27de83b283957189244d523d71ca1457))
    - [git-odb] refactor ([`e07478c`](https://github.com/GitoxideLabs/gitoxide/commit/e07478c7b212e4d1d21ce151d9eb26d0fae422a8))
    - [git-odb] fix docs ([`05347d4`](https://github.com/GitoxideLabs/gitoxide/commit/05347d4154d43d4657839a9cadcebeb1f44ec728))
    - [git-odb] refactor ([`721303d`](https://github.com/GitoxideLabs/gitoxide/commit/721303db232f87857aae58e12b342e5fb0139306))
    - [git-odb] refactor ([`ea224e9`](https://github.com/GitoxideLabs/gitoxide/commit/ea224e9ee5f7efcbf4942a2a6fc7e4d790b2be50))
    - [git-odb] refactor ([`6a1b16a`](https://github.com/GitoxideLabs/gitoxide/commit/6a1b16ae98edc9a694b945a12a7866eb17fc6be3))
    - [git-odb] refactor ([`bae3980`](https://github.com/GitoxideLabs/gitoxide/commit/bae3980e01131e7da38146aa510d1243e558a01a))
    - [git-odb] refactor ([`6b7400b`](https://github.com/GitoxideLabs/gitoxide/commit/6b7400bdcfc793d598f2102576939e55a5a3fc43))
    - [git-odb] refactor ([`19ab0cb`](https://github.com/GitoxideLabs/gitoxide/commit/19ab0cba168cd037107e5cc16a360884d40bd775))
    - [git-odb] refactor ([`47c4042`](https://github.com/GitoxideLabs/gitoxide/commit/47c4042f16a0e0e6a536bab7150b7cb21958a7ed))
    - [pack-gen] refactor ([`b5618ca`](https://github.com/GitoxideLabs/gitoxide/commit/b5618cad9f2a2403058b9b73ff1ada53ba85e8d0))
    - (cargo-release) version 0.15.0 ([`d69d9fb`](https://github.com/GitoxideLabs/gitoxide/commit/d69d9fb0931f8257cef96ef14a89da9340ad9738))
    - Put prodash behind a feature toggle, too ([`966058d`](https://github.com/GitoxideLabs/gitoxide/commit/966058d611c548e90c050462de52e36f1925e775))
    - Put 'walkdir' behind a feature flag/make it optional. ([`1a3cc5b`](https://github.com/GitoxideLabs/gitoxide/commit/1a3cc5bea1868ed3ae015403fbe0cdec788be749))
    - Put 'sha1' behind a feature toggle ([`4f326bc`](https://github.com/GitoxideLabs/gitoxide/commit/4f326bc261c4e7f0d5510df74ad4215da3580696))
    - Put crc functionality behind a feature toggle ([`458fa6e`](https://github.com/GitoxideLabs/gitoxide/commit/458fa6ec726ec7901c1f6d970cbb1c1ea975dded))
    - Revert "[pack-gen] quick hack for obtaining the entry size more quickly" ([`4c36f92`](https://github.com/GitoxideLabs/gitoxide/commit/4c36f92880d52886b1fb2c37cf2f98e6b9a327a0))
    - [pack-gen] quick hack for obtaining the entry size more quickly ([`ad6d007`](https://github.com/GitoxideLabs/gitoxide/commit/ad6d00701d28befda006f41f85bbbb6fc3508e1e))
    - Revert "[pack-gen] remove tree-diff as traversal option." ([`2907a5f`](https://github.com/GitoxideLabs/gitoxide/commit/2907a5facb08a7decbdfa652e76eb0ebd5e29dcf))
    - [pack-gen] remove tree-diff as traversal option. ([`8373671`](https://github.com/GitoxideLabs/gitoxide/commit/8373671fd4f3f7e9d78c480e9f68c0a7ae423c69))
    - [pack-gen] fix docs ([`2548b48`](https://github.com/GitoxideLabs/gitoxide/commit/2548b4813f52409bc1b214485854e5fceb78b534))
    - [pack-gen] basic progress for entry generation ([`953190d`](https://github.com/GitoxideLabs/gitoxide/commit/953190d70a5df22b54dc1fffe78d41dc7d81cc61))
    - [pack-gen] the first barely working progress ([`5b89a0e`](https://github.com/GitoxideLabs/gitoxide/commit/5b89a0e4203d405a50bc2e8de9d87b79e545412d))
    - [pack-gen] tests are green ([`34b6a2e`](https://github.com/GitoxideLabs/gitoxide/commit/34b6a2e94949b24bf0bbaeb169b4baa0fa45c965))
    - [pack-gen] thanks clippy ([`3f948a4`](https://github.com/GitoxideLabs/gitoxide/commit/3f948a44857b5ff21c85e71ab0c10538862d3d26))
    - [pack-gen] the basics to get the program going ([`03b67b0`](https://github.com/GitoxideLabs/gitoxide/commit/03b67b09e4127ae4bd791501d74794d9360f7ef6))
    - [pack-gen] Use more light-weight lookups for all blobs ([`80ce34d`](https://github.com/GitoxideLabs/gitoxide/commit/80ce34d82aebf9a075dde5e77be8af56d22117c7))
    - [pack-gen] refactor ([`e0caf8d`](https://github.com/GitoxideLabs/gitoxide/commit/e0caf8df5f2d6009a0ef10aa160c7c0bb5682560))
    - [pack-gen] a way to get the pack location by ID right away ([`5619efb`](https://github.com/GitoxideLabs/gitoxide/commit/5619efb368176809d550dc9d43d820b05a87a700))
    - [pack-gen] refactor ([`fcb9c5f`](https://github.com/GitoxideLabs/gitoxide/commit/fcb9c5fad04429b7797d400c2a9661a149b5bf66))
    - [pack-gen] refactor ([`11ce2d8`](https://github.com/GitoxideLabs/gitoxide/commit/11ce2d84c55ef8ffe5ac0a3cf43a48a74ff3185f))
    - [pack-gen] And the fix - all green ([`202c704`](https://github.com/GitoxideLabs/gitoxide/commit/202c7046283acd65ae3ae6ab5ff0b20b1020e360))
    - [pack-gen] with the addition of write-oack checks it actually fails ([`a9e46a6`](https://github.com/GitoxideLabs/gitoxide/commit/a9e46a64fc09dabf1290aeafa309bf86cfd496fe))
    - [pack-gen] it compiles and all tests are green, with tests for all parts ([`b3a0344`](https://github.com/GitoxideLabs/gitoxide/commit/b3a0344db0f10a6208793087a9a9a9bcf39ab47e))
    - [pack-gen] minor but relevant differences between 'existing' and 'existing_object' ([`5f18124`](https://github.com/GitoxideLabs/gitoxide/commit/5f18124694dd767e378ff6b4e77c71db642b50a2))
    - [pack-gen] very close to a basic impl of count + entries-gen… ([`c927429`](https://github.com/GitoxideLabs/gitoxide/commit/c9274295e62f59cd8db06a307cc4a69d096a006e))
    - [pack-gen] Fill the relevant information for later ([`932b439`](https://github.com/GitoxideLabs/gitoxide/commit/932b43998849e5d755f6fd8d19f1e942080e7bbd))
    - [pack-gen] the first test for object counts ([`67b1512`](https://github.com/GitoxideLabs/gitoxide/commit/67b1512db8c3bdb2ea946d0de96190146be9ed18))
    - [pack-gen] first sketch of how counting could look like ([`6ef0072`](https://github.com/GitoxideLabs/gitoxide/commit/6ef00723b134d2ce730a288a89858db2ff568c3b))
    - [pack-gen] prep for counting stage ([`93fd425`](https://github.com/GitoxideLabs/gitoxide/commit/93fd4251885e6a13f0026b96c6688da0e68f9cbf))
    - [pack-gen] tag handling for diff based traversal ([`e55906c`](https://github.com/GitoxideLabs/gitoxide/commit/e55906c07d9d6f2fbfa5607a2337e586f94beabe))
    - [pack-gen] tag support for tree traversal ([`28ed260`](https://github.com/GitoxideLabs/gitoxide/commit/28ed260a73554d261c9b00c8ae9a46e66f123e6f))
    - (cargo-release) version 0.10.0 ([`5d7ee6a`](https://github.com/GitoxideLabs/gitoxide/commit/5d7ee6a105abbb6efeed8624bade936bb59dbc55))
    - [pack-gen] the first green test for Tag iterators ([`df5ef8a`](https://github.com/GitoxideLabs/gitoxide/commit/df5ef8a53cb4007058137890b414af510025fccf))
    - [pack-gen] A test to see we can handle tag objects ([`1898319`](https://github.com/GitoxideLabs/gitoxide/commit/189831944d60217a3cd7383a00550d581259f638))
    - Refactor ([`9f0a8cc`](https://github.com/GitoxideLabs/gitoxide/commit/9f0a8cc1561589088f44a1775832110449a4f1ab))
    - [pack-gen] Finally traversal based pack gen works too ([`086422b`](https://github.com/GitoxideLabs/gitoxide/commit/086422bbea50bba01060937420ab737e469e11da))
    - [pack-gen] diff-based traversal now finds all reachable objects ([`e819c92`](https://github.com/GitoxideLabs/gitoxide/commit/e819c92234a1c2b182dd269d0858f003ffcc2cb0))
    - Thanks clippy ([`760febf`](https://github.com/GitoxideLabs/gitoxide/commit/760febf6a025891957b1afea1dd44a4ed0c4b1ca))
    - [pack-gen] add more objects during diff traversal ([`bc2ef19`](https://github.com/GitoxideLabs/gitoxide/commit/bc2ef193af15a1414d987b9cc780b2cd3a93e9f4))
    - [pack-gen] pickup more trees ([`2da57bd`](https://github.com/GitoxideLabs/gitoxide/commit/2da57bd02672d1d4effc940bcf81720fc63f02bc))
    - [pack-gen] Specific tests show that something is off in the changes case… ([`b131c9e`](https://github.com/GitoxideLabs/gitoxide/commit/b131c9e68c7ac062cd9abbd0541afdb9c69e8649))
    - [pack-gen] central object synchronization for diff based algo as well ([`6de3558`](https://github.com/GitoxideLabs/gitoxide/commit/6de3558e4becbf4d3cf0640b8eceff40b82f55d3))
    - [pack-gen] have to keep track of all seen objects ([`dc645c6`](https://github.com/GitoxideLabs/gitoxide/commit/dc645c62a1b05e6b160c8355a71452467ccb6d38))
    - [pack-gen] updating tests to verify something shows that objects are duplicated ([`cef1a58`](https://github.com/GitoxideLabs/gitoxide/commit/cef1a58cf6cc40fd0a53a9c46ef996f753d7d7d4))
    - [pack-gen] tree diff based pack generation passes the trivial test ([`ad0e345`](https://github.com/GitoxideLabs/gitoxide/commit/ad0e345af0654ce40afce713de9286f06cf1d05c))
    - [pack-gen] refactor ([`cac002a`](https://github.com/GitoxideLabs/gitoxide/commit/cac002a94427c10a9f87901a861a9d764126f8e5))
    - [git-traverse] accept tree iterators instead of object ids ([`f343dad`](https://github.com/GitoxideLabs/gitoxide/commit/f343dad60d34dfd88247a14c7e3de906a761cf2d))
    - [pack-gen] Most of changes based pack gen ([`9ade04c`](https://github.com/GitoxideLabs/gitoxide/commit/9ade04c47b3d4cad29a754f15f21df7e1b266325))
    - [pack-gen] prepare diff based pack-gen ([`fa2ae2c`](https://github.com/GitoxideLabs/gitoxide/commit/fa2ae2c924b295a4c25f41ba9ecbcf5c45b77e85))
    - [git-diff] refactor ([`087e853`](https://github.com/GitoxideLabs/gitoxide/commit/087e85367c27bb3684c6ad543c7eae46762e5e44))
    - [git-traverse] refactor ([`85de287`](https://github.com/GitoxideLabs/gitoxide/commit/85de2874087f64fc166797a3219eeb26be460619))
    - (cargo-release) version 0.3.0 ([`684de4b`](https://github.com/GitoxideLabs/gitoxide/commit/684de4b376ecd4cc5330f7ac8643352ea9580ed3))
    - [pack-gen] Speed up tree-traversal :D ([`90b4093`](https://github.com/GitoxideLabs/gitoxide/commit/90b4093aa6076c97f751013de4c25934fef764b8))
    - Thanks clippy ([`009a342`](https://github.com/GitoxideLabs/gitoxide/commit/009a3425d24d4c9f476ff1c32da9b279cb170350))
    - [pack-gen] Probably a valid impl of tree traversal ([`4c72a17`](https://github.com/GitoxideLabs/gitoxide/commit/4c72a171d50d08d4b35209fcef107b9a85a6c648))
    - [pack-gen] quite a bit closer to tree-traversal for pack gen ([`ecd37ee`](https://github.com/GitoxideLabs/gitoxide/commit/ecd37eea0154791bf9192d1225828e7d9b5ad530))
    - [pack-gen] refactor ([`325d63e`](https://github.com/GitoxideLabs/gitoxide/commit/325d63efe6855c8d14d564d8b3cbce9a9e144d14))
    - [pack-gen] a test for upcoming traversal modes ([`8d1e9ac`](https://github.com/GitoxideLabs/gitoxide/commit/8d1e9ace79bbbe41ea4fac70c13522d7d6091a81))
    - [pack-gen] refactor ([`08b136f`](https://github.com/GitoxideLabs/gitoxide/commit/08b136f0cf35f8b275feee9830bfab4555d40a99))
    - (cargo-release) version 0.15.0 ([`d91b241`](https://github.com/GitoxideLabs/gitoxide/commit/d91b2412381e3c8c1f24c38469e821c3c3960e34))
    - (cargo-release) version 0.9.0 ([`84897fd`](https://github.com/GitoxideLabs/gitoxide/commit/84897fd8e6e1b0269da0303d6a0de8f9e0eb58e5))
    - Merge branch 'patch-2' ([`f01dc54`](https://github.com/GitoxideLabs/gitoxide/commit/f01dc54010683b232c5f5813bd5370e93f1681f5))
    - Refactor ([`a9e4feb`](https://github.com/GitoxideLabs/gitoxide/commit/a9e4feb0a81894568be730603446e2d061dd558f))
    - Fix formatting ([`a341995`](https://github.com/GitoxideLabs/gitoxide/commit/a341995e6014b6ed0e43ae94fa1152aed6fcfd89))
    - Merge branch 'patch-1' ([`5edc076`](https://github.com/GitoxideLabs/gitoxide/commit/5edc0762524112bb6716b3afcf23b2a4a0f5efd3))
    - Use Seek to skip large objects during indexing. ([`95e2af7`](https://github.com/GitoxideLabs/gitoxide/commit/95e2af74574af998294265b6a3de833dbe4dcedb))
    - Remove almost all unsafe code from Tree. ([`42b6033`](https://github.com/GitoxideLabs/gitoxide/commit/42b6033f3c367ccce37c82356183d165d37ae881))
    - Thanks clippy ([`17258cc`](https://github.com/GitoxideLabs/gitoxide/commit/17258cc58767caa6e71227898decd160ad0cdf13))
    - Thanks clippy ([`09decde`](https://github.com/GitoxideLabs/gitoxide/commit/09decde782e0b9e794a740d4fa654af73398d80a))
    - Convenience methods for iterators ([`aa6c9e6`](https://github.com/GitoxideLabs/gitoxide/commit/aa6c9e699a09b6b2b4f55aa75a1dd6f648eead09))
    - Refactor ([`d9783b9`](https://github.com/GitoxideLabs/gitoxide/commit/d9783b94b0149c584690a1a50f029c9424de08c3))
    - A sketch of convenient finding of commits ([`db21062`](https://github.com/GitoxideLabs/gitoxide/commit/db210622b95d5f1f24d815cb35db5d46aa8a09e3))
    - Refactor ([`3af7b7b`](https://github.com/GitoxideLabs/gitoxide/commit/3af7b7b2bc0082298faa7ff2bd4413e80bee1107))
    - Sketch of convenience function for `Find` trait. ([`2bf4958`](https://github.com/GitoxideLabs/gitoxide/commit/2bf4958dd7d1ad0a2f9f8c5754be88c3efb524a4))
    - Refactor ([`bd4d21e`](https://github.com/GitoxideLabs/gitoxide/commit/bd4d21e7003801319a62887e3d39467b2ee1ad0d))
    - Refactor ([`8b10434`](https://github.com/GitoxideLabs/gitoxide/commit/8b1043483cb46fd1b7f47a90c9dce24a65d58d1b))
    - Fix order of operations in git-odb::hash::Write ([`a31d8c7`](https://github.com/GitoxideLabs/gitoxide/commit/a31d8c75e7821b68b49f017010646a8232ecc6cc))
    - (cargo-release) version 0.14.0 ([`a760f8c`](https://github.com/GitoxideLabs/gitoxide/commit/a760f8c013e13ba82daa1acf1a4a57e0818a008d))
    - (cargo-release) version 0.14.0 ([`d9514ee`](https://github.com/GitoxideLabs/gitoxide/commit/d9514eec64579ef77c9f2ac5dfe87cd302180eb9))
    - Rename 'Locate' to 'Find' - shorter and just as good ([`60f72f5`](https://github.com/GitoxideLabs/gitoxide/commit/60f72f573a7696323e09bf4add80d5fbce22c99d))
    - (cargo-release) version 0.13.0 ([`5c791af`](https://github.com/GitoxideLabs/gitoxide/commit/5c791af217fac6a171d174ad9f4ee5f4d5282892))
    - [traversal] remove git-odb::traversal (now git-traverse::iter) ([`35b74d2`](https://github.com/GitoxideLabs/gitoxide/commit/35b74d2f046426d99bb5431f8aea42ac453ac101))
    - Prepare test utilities for release… ([`d35e654`](https://github.com/GitoxideLabs/gitoxide/commit/d35e654747f96cec93bdecd1314ce325129cbc44))
    - (cargo-release) version 0.8.0 ([`a1ce210`](https://github.com/GitoxideLabs/gitoxide/commit/a1ce210003ff07bf11291018bb182cbc7913647b))
    - (cargo-release) version 0.3.0 ([`e9665c7`](https://github.com/GitoxideLabs/gitoxide/commit/e9665c784ae7e5cdaf662151395ee2355e9b57b6))
    - [traversal] all the caching ([`0890403`](https://github.com/GitoxideLabs/gitoxide/commit/0890403cce658ea90c593a6ca21e24f02ddf5a93))
    - [tree-diff] first prototype of traversal experiment ([`ece43b4`](https://github.com/GitoxideLabs/gitoxide/commit/ece43b4b0bf054d798685461d2b96daaafd8a408))
    - Thanks clippy ([`2d5e205`](https://github.com/GitoxideLabs/gitoxide/commit/2d5e20520499d1a87808db508548b408e3777d0e))
    - [tree-diff] more tests for the tree iterator ([`91b5a02`](https://github.com/GitoxideLabs/gitoxide/commit/91b5a029337200a2873a21696020dcda08e335cb))
    - [tree-diff] Now the commit graph traversal works with zero-allocations ([`8078910`](https://github.com/GitoxideLabs/gitoxide/commit/8078910b27149df10b6b236b9311ebee31523710))
    - Make it easy to get a commit iterator ([`33213f3`](https://github.com/GitoxideLabs/gitoxide/commit/33213f30abbb6619d41663a4baf3078af3284085))
    - [tree-diff] refactor into iterator based model ([`29b527a`](https://github.com/GitoxideLabs/gitoxide/commit/29b527aaea101c9b4e885db1c6d3533ef2310c54))
    - [tree-diff] The least intrusive way to allow dealing with tree iterators ([`d41dd3c`](https://github.com/GitoxideLabs/gitoxide/commit/d41dd3c38ee34b250a4f5de120d7ae3e04e3212d))
    - Refactor ([`a4d5f99`](https://github.com/GitoxideLabs/gitoxide/commit/a4d5f99c8dc99bf814790928a3bf9649cd99486b))
    - Refactor ([`03ee510`](https://github.com/GitoxideLabs/gitoxide/commit/03ee510a5f9c24b6acddaec1d30ea3ad39174603))
    - Better ergonomics for accessing decoded objects ([`ae3eab6`](https://github.com/GitoxideLabs/gitoxide/commit/ae3eab6d6e4b96e207372fa8cb82f5ac9833e5e4))
    - Refactor ([`c1013dd`](https://github.com/GitoxideLabs/gitoxide/commit/c1013dddbc221b366b91d186cfd1732f1d72be10))
    - Refactor ([`f37c31f`](https://github.com/GitoxideLabs/gitoxide/commit/f37c31f04bf8cf531284abe380db77d6196bd711))
    - Refactor ([`ac70651`](https://github.com/GitoxideLabs/gitoxide/commit/ac706518fff2e92ade3589dea4a6c81fca57aec2))
    - Refactor ([`77764f3`](https://github.com/GitoxideLabs/gitoxide/commit/77764f3b9c3e8202119bb9327e150089c3ecbb9b))
    - Refactor ([`3185cc9`](https://github.com/GitoxideLabs/gitoxide/commit/3185cc9de1f7d3e52d088b60fcaae0ac91a72fe1))
    - Thanks, cargo audit ([`4f293f5`](https://github.com/GitoxideLabs/gitoxide/commit/4f293f5036c44a69ccacf102d35202adad83bbe0))
    - Refactor ([`edf7d38`](https://github.com/GitoxideLabs/gitoxide/commit/edf7d382148aa139485c8279c2a50dc6c86d481d))
    - Refactor ([`ca98221`](https://github.com/GitoxideLabs/gitoxide/commit/ca98221d5a512dabf683cc1da56d40a17285f2fb))
    - Refactor ([`b4027e3`](https://github.com/GitoxideLabs/gitoxide/commit/b4027e3df187931a263798b255c80b272910aef7))
    - Refacto ([`6e328da`](https://github.com/GitoxideLabs/gitoxide/commit/6e328da9f8a73ac8e699aea55b1250a433f5ecd9))
    - Fix docs ([`a54bab4`](https://github.com/GitoxideLabs/gitoxide/commit/a54bab40a5881873eb2b1c591fa9f05d8034ac6d))
    - Refactor ([`3f2ee4c`](https://github.com/GitoxideLabs/gitoxide/commit/3f2ee4cda6db14902639f7fc3a9fbee05508086f))
    - Refactor ([`d6ab581`](https://github.com/GitoxideLabs/gitoxide/commit/d6ab581db66c1d09578ed2af9db34929995e2cb9))
    - Refactor ([`d490b65`](https://github.com/GitoxideLabs/gitoxide/commit/d490b65ebbc6666cd59d88f8677dc1c52bfe1e1c))
    - Pack V2 writing (base objects only) seems to work now #(67) ([`e68dd84`](https://github.com/GitoxideLabs/gitoxide/commit/e68dd84df7d13efcc7882644d3d9347b3722785a))
    - The first more thorough and indirect validation of the newly written pack… ([`d43687e`](https://github.com/GitoxideLabs/gitoxide/commit/d43687ed9093224e0caba4063063705b9473afd0))
    - Refactor ([`08fafaa`](https://github.com/GitoxideLabs/gitoxide/commit/08fafaa03144fc3ddea9120a4a1943e18c454ae8))
    - Test newly written pack data alone ([`01fdd70`](https://github.com/GitoxideLabs/gitoxide/commit/01fdd70395a662612309ece730c2a75303e2155e))
    - Write pack data entries #(67) ([`722202e`](https://github.com/GitoxideLabs/gitoxide/commit/722202edce0d5700a9df9eff6208ad5d7c6554fb))
    - Refactor ([`eed1e3c`](https://github.com/GitoxideLabs/gitoxide/commit/eed1e3c21658ee152d224622599cd5a4c65df126))
    - Write pack data header #(67) ([`717726b`](https://github.com/GitoxideLabs/gitoxide/commit/717726b30e80f0ca56927f31e823ec48470fbeb2))
    - Refactor ([`28cbeb3`](https://github.com/GitoxideLabs/gitoxide/commit/28cbeb3ca1d6c5f6aa7664255d1d44fdb49f116b))
    - Refactor ([`4261c7d`](https://github.com/GitoxideLabs/gitoxide/commit/4261c7dea7666cfc3a867bca2bbdb0827487be00))
    - All logic needed to write a valid pack within an iterator ([`775ab29`](https://github.com/GitoxideLabs/gitoxide/commit/775ab295531875ec93e57d30422b88e03e48313e))
    - Sketch of pack data write API ([`dfeda87`](https://github.com/GitoxideLabs/gitoxide/commit/dfeda87de13c6f05a39732d3f0518bb76695be9a))
    - Refactor ([`f33fa10`](https://github.com/GitoxideLabs/gitoxide/commit/f33fa10224d46539c94a2042014c14042d7dc968))
    - [experiment/object-access] allow bare repositories ([`401690d`](https://github.com/GitoxideLabs/gitoxide/commit/401690dbc6c10b2e7144bf3362c4b2e71435e801))
    - Thanks clippy ([`c86823a`](https://github.com/GitoxideLabs/gitoxide/commit/c86823a5cce91a12738c25313ae15eec7751af46))
    - Refactor zlib ([`4587b82`](https://github.com/GitoxideLabs/gitoxide/commit/4587b8244c5ba85aa899e998214119aadb948862))
    - Refactor zlib ([`496e6bb`](https://github.com/GitoxideLabs/gitoxide/commit/496e6bb86ba1bcf66ffaf250b026c12bd3e830c5))
    - Refactor ([`3a4469c`](https://github.com/GitoxideLabs/gitoxide/commit/3a4469c20c44dd649a442c7f6c2902325c744394))
    - First basic pack entry generation (base only) works… ([`75cb32b`](https://github.com/GitoxideLabs/gitoxide/commit/75cb32baed75c23b47d6422569be630c6fd412f7))
    - Refactor ([`d4bf8ae`](https://github.com/GitoxideLabs/gitoxide/commit/d4bf8aea9b8f811b57b943be16ea4bb2eabccca4))
    - Refactor ([`2d89222`](https://github.com/GitoxideLabs/gitoxide/commit/2d89222b1d48cf63544ceb4bf8d3067a49adb792))
    - Refactor ([`eb3a8da`](https://github.com/GitoxideLabs/gitoxide/commit/eb3a8da7a246355f1ef0de20226abaaf38b01126))
    - Allow calling 'finalize()' on the entries iterator ([`3c617bc`](https://github.com/GitoxideLabs/gitoxide/commit/3c617bc2ae59adbb12c254308269e745149d462b))
    - Refactor ([`b7d0323`](https://github.com/GitoxideLabs/gitoxide/commit/b7d03235b1eb42e98cfc7620dea9d41b0e87d208))
    - Reduce size of data::Object ([`7aa783a`](https://github.com/GitoxideLabs/gitoxide/commit/7aa783ab4e06d3e33f918c0cd084dd8d89f3d768))
    - First pack-to-pack copy and crc32 verification ([`37619f0`](https://github.com/GitoxideLabs/gitoxide/commit/37619f0ea71216ef7a0b9e512e3987fead08c9b9))
    - It's possible to get entry data within pack generation ([`a2a5927`](https://github.com/GitoxideLabs/gitoxide/commit/a2a59272116029e9328fb2798e5c72e9fc9b3b32))
    - Git-odb without cargo warnings due to using the same test twice ([`8945f95`](https://github.com/GitoxideLabs/gitoxide/commit/8945f95364b489e7a639d74dd0f28b17e82e70f3))
    - A way to obtain entry information using previous lookup information ([`a55d113`](https://github.com/GitoxideLabs/gitoxide/commit/a55d113ded8d6aeee78f9041f13167dc54243254))
    - Refactor ([`95ab11b`](https://github.com/GitoxideLabs/gitoxide/commit/95ab11bd3014c81ab35437ba9d1c6b84caf6ba76))
    - A probably more secure way of accessing pack data ([`7a01bd8`](https://github.com/GitoxideLabs/gitoxide/commit/7a01bd8b120fa34566b0f97ca9b35e1d8a97aefa))
    - Sketch of pack-entry retrieval api ([`d1e9248`](https://github.com/GitoxideLabs/gitoxide/commit/d1e92486c9b716c11cf75eccd9829738d3b94ca0))
    - Refactor ([`08cf90a`](https://github.com/GitoxideLabs/gitoxide/commit/08cf90a2e33e3a0cdbb249ffcc46ef3a46685145))
    - Let's be a bit more conservative with this information for now ([`efef417`](https://github.com/GitoxideLabs/gitoxide/commit/efef417e52caf12a2090b6d4a96e0633e77471dd))
    - Objects know their pack location ([`73f1c66`](https://github.com/GitoxideLabs/gitoxide/commit/73f1c668b629055d8b0bffc1a6cc31c54037a6da))
    - Chunks based iteration for pack generation ([`23c2694`](https://github.com/GitoxideLabs/gitoxide/commit/23c2694c86eb397f5063f248c95cd164bae2120a))
    - Some notes about how to treat defaults of file versions ([`cfa5cf6`](https://github.com/GitoxideLabs/gitoxide/commit/cfa5cf6146d4de028a31b5eb8ad756477e37111b))
    - Run git-odb tests in parallel, too; improved threaded error handling ([`40802fd`](https://github.com/GitoxideLabs/gitoxide/commit/40802fd8bbb15b8a61249522d67f3a5b28da64b3))
    - The first test for pack generation ([`2a2fdde`](https://github.com/GitoxideLabs/gitoxide/commit/2a2fdde2e5365e83faf99999ea1c640159f5c4b9))
    - Refactor ([`385f52d`](https://github.com/GitoxideLabs/gitoxide/commit/385f52d4ea99230839bb447e2993bad741ce0cae))
    - Refactor  Please enter the commit message for your changes. Lines starting ([`f65c68c`](https://github.com/GitoxideLabs/gitoxide/commit/f65c68c3c7c4c838ea77494ecc0ce17f6d5d719b))
    - Fix doc links ([`ec35743`](https://github.com/GitoxideLabs/gitoxide/commit/ec35743cc4062f2b6dbfc85b7f5aadfa68f598a7))
    - Thanks clippy ([`563e445`](https://github.com/GitoxideLabs/gitoxide/commit/563e4452aae5c6dc1323e0f6759315e73f3a2c89))
    - The first seemingly working iteration over all objects in an odb #(67) ([`6b34f62`](https://github.com/GitoxideLabs/gitoxide/commit/6b34f62cc4e6f9ee6030590d8b3f185dda3bc568))
    - Refactor ([`01d9d91`](https://github.com/GitoxideLabs/gitoxide/commit/01d9d91d1bce6217b8a48ab1f0a7ba4e17508279))
    - Impl size_hint for linked db iter ([`ada259b`](https://github.com/GitoxideLabs/gitoxide/commit/ada259b4fe441728682521e9138ed9f6ef1c13f4))
    - Refactor ([`82c2f42`](https://github.com/GitoxideLabs/gitoxide/commit/82c2f428e22c3cda79913c9ca2f092c377d692aa))
    - Refactor ([`7a6b514`](https://github.com/GitoxideLabs/gitoxide/commit/7a6b514a5b9b93bf574cd3a114f27ad5967e89ac))
    - First sketch of object iterator for linked::Db ([`a316eed`](https://github.com/GitoxideLabs/gitoxide/commit/a316eed4bf4634d4776d153528cb28254b847bdd))
    - Set environment in testtools to freeze repositories generation scripts ([`eaad3ab`](https://github.com/GitoxideLabs/gitoxide/commit/eaad3ab69338115439a553ba1062160dc3a08082))
    - Faster repeated tests if fixtures don't change ([`792277f`](https://github.com/GitoxideLabs/gitoxide/commit/792277f241446086dd6c9b78f688363d4e66e5a7))
    - Refactor ([`e1a92ad`](https://github.com/GitoxideLabs/gitoxide/commit/e1a92adbedcb017a9e35049389ef86fca34fa44c))
    - Allow the use of shared test utilities across crates ([`b117626`](https://github.com/GitoxideLabs/gitoxide/commit/b117626df6da714c24d2b7914301678e89d2d0cb))
    - Refactor ([`40b86a7`](https://github.com/GitoxideLabs/gitoxide/commit/40b86a78367fbd7cd9c8e5447c9b97fa685cc43e))
    - Refactor ([`8b00094`](https://github.com/GitoxideLabs/gitoxide/commit/8b0009466b820b934a2244a98360007336180246))
    - Fix doc links ([`7479071`](https://github.com/GitoxideLabs/gitoxide/commit/747907145e001a093c8dc84e80d879f4d18c84d5))
    - Thanks clippy ([`6f901f5`](https://github.com/GitoxideLabs/gitoxide/commit/6f901f5daa868c1a0e9cea113abe13beb65d8f35))
    - Ancestor iterator is now generic over Locate trait ([`bbfd616`](https://github.com/GitoxideLabs/gitoxide/commit/bbfd616ae023aae9d92ebd9d873a9be02423e820))
    - [fail] try to abstract ancestor::Iter over Locate trait ([`f8c0375`](https://github.com/GitoxideLabs/gitoxide/commit/f8c0375bbafffc81938998a8ff8aa2faac9253e1))
    - Refactor ([`5ef1f22`](https://github.com/GitoxideLabs/gitoxide/commit/5ef1f22c1e12ff8d607663d4dfbbbfe426a29e0f))
    - Improve interface for building packs based on Locate trait #(67) ([`5b66b6e`](https://github.com/GitoxideLabs/gitoxide/commit/5b66b6e729c858068a31e4817db63a5f6ba5032b))
    - A version of the Locate trait we can do today #(67) ([`d752be2`](https://github.com/GitoxideLabs/gitoxide/commit/d752be2931e3403c16fea8d804c8759c56bb1fd4))
    - [git-odb] Associated types with lifetimes also don't seem to work ([`0e68a9d`](https://github.com/GitoxideLabs/gitoxide/commit/0e68a9d095eb038da0e9139fe1d649d593d72010))
    - [git-odb] Trying to use offical traits won't work with our kind of object ([`29a5054`](https://github.com/GitoxideLabs/gitoxide/commit/29a5054740fd0c7958376c603fd6214421f7772f))
    - :borrowed::Object => git-odb::data::Object ([`747a13e`](https://github.com/GitoxideLabs/gitoxide/commit/747a13e9a1fe5200c53055dd961507c9fef667e1))
    - An even better name for decode errors ([`f270850`](https://github.com/GitoxideLabs/gitoxide/commit/f270850ff92eab15258023b8e59346ec200303bd))
    - Make clear it's a decode error we are using there ([`f45cb4b`](https://github.com/GitoxideLabs/gitoxide/commit/f45cb4b62122559e5701923e0a23dd5791ee2ced))
    - Rename git-object::(owned->mutable)|(borrowed|immutable) #(67) ([`91ee558`](https://github.com/GitoxideLabs/gitoxide/commit/91ee55893bf4b27a47d86d51bae6f99b59b69147))
    - Bump git-odb minor version ([`5c833ce`](https://github.com/GitoxideLabs/gitoxide/commit/5c833ce64babd00b7ced3e3a1c9ed3dbd260c9f4))
    - Thanks clippy ([`547af6e`](https://github.com/GitoxideLabs/gitoxide/commit/547af6e3965112c8eea69cae173a6996249b77c5))
    - Fix test breakage for loose object reading ([`222c7a2`](https://github.com/GitoxideLabs/gitoxide/commit/222c7a276efdc65da4f9490f53b82e58f8e878c1))
    - Fix docs #(67) ([`01db10a`](https://github.com/GitoxideLabs/gitoxide/commit/01db10a27431ad89a68ed3e4eabae810748a6f29))
    - Thanks clippy ([`60a7689`](https://github.com/GitoxideLabs/gitoxide/commit/60a7689d7493d29103775ce358999314af9257c8))
    - Refactor ([`ef674ff`](https://github.com/GitoxideLabs/gitoxide/commit/ef674ffde5af3c19a9538d99112f414144b921cd))
    - Remove loose::Object entirely #(67) ([`5cf4840`](https://github.com/GitoxideLabs/gitoxide/commit/5cf4840b10a3fac43266bc9defa72977a004bf8c))
    - Start using loose::Db::locate2 - definitely still bugs in there ([`d6f07b7`](https://github.com/GitoxideLabs/gitoxide/commit/d6f07b7709fb2291484859477b54371ef3108a57))
    - An alternative version of loose::Db::locate() for use with borrowed::Object ([`5b40a32`](https://github.com/GitoxideLabs/gitoxide/commit/5b40a32c017f264d80b8babb293470a4a47a45b4))
    - Refactor ([`bad3ce4`](https://github.com/GitoxideLabs/gitoxide/commit/bad3ce417dd7b4cdbcf45c95fbdc44b245b0762f))
    - Replace loose::Object::stream() with *::data() #(67) ([`040b347`](https://github.com/GitoxideLabs/gitoxide/commit/040b347d1b020ef17a8862c4cb792e267d674c8a))
    - Sketch loose::Object::data() to start refactoring #(67) ([`ee1701f`](https://github.com/GitoxideLabs/gitoxide/commit/ee1701f681af4a6acfd6809fe439a3fa1912f259))
    - Sketch of trait for locating objects #(67) ([`31445d7`](https://github.com/GitoxideLabs/gitoxide/commit/31445d778864c430d363bea86c51286f5f9f69a1))
    - Refactor ([`2754dd6`](https://github.com/GitoxideLabs/gitoxide/commit/2754dd6078608a600ec20a5d1c9307c2d746e6c5))
    - Refactor ([`3e908bd`](https://github.com/GitoxideLabs/gitoxide/commit/3e908bd4b4077c4a5d113cefc113f9d71f249133))
    - Refactor ([`409d763`](https://github.com/GitoxideLabs/gitoxide/commit/409d763d2fca974a647487c72d15f568a9b62ccb))
    - Refactor ([`896ab94`](https://github.com/GitoxideLabs/gitoxide/commit/896ab940bcd475d026e4009b3aa2fa6a025c14bc))
    - Remove unsafe interface for stepped computation #(67) ([`c856613`](https://github.com/GitoxideLabs/gitoxide/commit/c856613a35aea7dea1d093bfcfe1ddbde93fdf26))
    - Show that with custom iterators, Arc's are natively supported #(67) ([`0c49007`](https://github.com/GitoxideLabs/gitoxide/commit/0c490073c53cf1f2df9fe2cd7612a1531e1430a7))
    - Thanks clippy ([`405dd9d`](https://github.com/GitoxideLabs/gitoxide/commit/405dd9d4cb7980a4925b19562e02a9fb7f0f5ab6))
    - Multi-tip support #(67) ([`2254ecc`](https://github.com/GitoxideLabs/gitoxide/commit/2254ecc4b1927867f02fe03db4a027d8c1e47ee9))
    - Cache support for traversal #(67) ([`1e9300a`](https://github.com/GitoxideLabs/gitoxide/commit/1e9300ac53b1d3e96352ce466f2c7d27a93ade2a))
    - Cycle and duplicate check #(67) ([`334a72d`](https://github.com/GitoxideLabs/gitoxide/commit/334a72d4a2ec2718d92b9c0843c4f6722a909f5e))
    - A new failing test ([`86b6c24`](https://github.com/GitoxideLabs/gitoxide/commit/86b6c2497cfa17bf3f822792e3afe406f7968ee7))
    - Refactor ([`5408b62`](https://github.com/GitoxideLabs/gitoxide/commit/5408b6258c5c5aea26c91e4ec7e7d56e8a3cc8f8))
    - The first basic traversal utility #(67) ([`ea6610b`](https://github.com/GitoxideLabs/gitoxide/commit/ea6610b9157d8d0dabd2ddd07c45dc6651b9cf84))
    - Fix iteration signature due to shadowed naming ([`fe8b459`](https://github.com/GitoxideLabs/gitoxide/commit/fe8b459fc406d5fee39d7dd333ff0afad78a0c38))
    - Thanks clippy ([`a463a43`](https://github.com/GitoxideLabs/gitoxide/commit/a463a438c69a96ac0f291298113c7814b6d51ec4))
    - Sketch of machinery for producing pack entries #(67) ([`ac8e7fb`](https://github.com/GitoxideLabs/gitoxide/commit/ac8e7fb6c8ae4ac42f56482d9d7744aa66132702))
    - A step towards using SteppedReduce #(67) ([`0d5595a`](https://github.com/GitoxideLabs/gitoxide/commit/0d5595a2269314d9aa2a76b2b5d650506a51f58e))
    - (cargo-release) version 0.13.0 ([`ac2eddb`](https://github.com/GitoxideLabs/gitoxide/commit/ac2eddb06eb3d8a9a3dcdcd796eb54a7e45ab935))
    - Allow parallel reducers to produce something during 'feed()' #(67) ([`6c04fcd`](https://github.com/GitoxideLabs/gitoxide/commit/6c04fcd643083d9db633edd3bb838b4f5de8f0db))
    - Allow more fine-grained stepping over the pack generator #(67) ([`22eb892`](https://github.com/GitoxideLabs/gitoxide/commit/22eb892e7e66f6a5e3e35094a657a8469a163e26))
    - Allow to obtain object information without fetching the data #(67) ([`6553850`](https://github.com/GitoxideLabs/gitoxide/commit/6553850aacbf815989af297bc95fe15d915f62ec))
    - Sketch a version to abstract object data retrieval #(67) ([`ad90446`](https://github.com/GitoxideLabs/gitoxide/commit/ad90446da913f1f0b9833a393c5f33ae2638ae30))
    - Implement `Write` trait for linked::Db ([`21362c3`](https://github.com/GitoxideLabs/gitoxide/commit/21362c388026837ad78891945cfeac8cea27e0db))
    - Docs for `linked::Db` ([`9d936de`](https://github.com/GitoxideLabs/gitoxide/commit/9d936dea06b8b28a46e8a0a466ea9f4d618595b1))
    - Support for caches within linked::Db ([`3635a3e`](https://github.com/GitoxideLabs/gitoxide/commit/3635a3e8629143c6b96ed80eb7d7a10f011afceb))
    - `locate()` for `linked::Db` without cache for now ([`014bc3c`](https://github.com/GitoxideLabs/gitoxide/commit/014bc3c74a8b566608091f0decfe79439ab5d6f9))
    - Refactor ([`7b443d1`](https://github.com/GitoxideLabs/gitoxide/commit/7b443d1563e8231ca8bf88752eac28f441801d52))
    - Refactor ([`d077ead`](https://github.com/GitoxideLabs/gitoxide/commit/d077ead603ce87f891e83e83cbcffeb4c79dd1f0))
    - :Db::init() with a few tests ([`4c77e4c`](https://github.com/GitoxideLabs/gitoxide/commit/4c77e4c97641ab3b02b56aaa702a7d2ca5bced7c))
    - Frame for linked::Db ([`e64d148`](https://github.com/GitoxideLabs/gitoxide/commit/e64d148a0984fb6dba3f788f8cc99c37914fd930))
    - Make cycles in alternate object chains fatal ([`67e679a`](https://github.com/GitoxideLabs/gitoxide/commit/67e679a6d7b56c2754f422e5cce3f8cf0784e506))
    - Resolve alternates as paths, not as repositories ([`73352c3`](https://github.com/GitoxideLabs/gitoxide/commit/73352c346d4a408eb657f1862996525982c16db6))
    - Remove support for Polonius in preparation for a new repo type ([`871c803`](https://github.com/GitoxideLabs/gitoxide/commit/871c803d9c5be6e786338b549c243ad50d057df5))
    - (cargo-release) version 0.11.0 ([`fd698e3`](https://github.com/GitoxideLabs/gitoxide/commit/fd698e334e44d5c478c162f98d09afd9ce7a6895))
    - Introduce pack_id for use in pack cache, preventing (most collisions) ([`ad04ad3`](https://github.com/GitoxideLabs/gitoxide/commit/ad04ad3b8ac54e78bee307dd44c85c1389edced2))
    - Fix benchmark to get valid test results ([`20abb3a`](https://github.com/GitoxideLabs/gitoxide/commit/20abb3a4fc9769f23b9a86d2e8d49f53290b36f4))
    - First use of memory-cap based LRU cache for object access ([`b057494`](https://github.com/GitoxideLabs/gitoxide/commit/b0574945039881c6d5d8be4107c1c987ed3bbaf6))
    - Add hash-map based LRU to allow bigger/more effective object caches ([`5affdd5`](https://github.com/GitoxideLabs/gitoxide/commit/5affdd5df0c4d01f3130fc1be259c72f601a1f71))
    - Feature toggle for uluru based Lru cache ([`98eec48`](https://github.com/GitoxideLabs/gitoxide/commit/98eec4837d605a408b026a859e53a7e2eae8e4da))
    - Refactor ([`d624d09`](https://github.com/GitoxideLabs/gitoxide/commit/d624d097784eed216f8d0e94544d8b62ef6c3010))
    - Improve docs to prevent people to 'misuse' the Lru cache. ([`fff62ed`](https://github.com/GitoxideLabs/gitoxide/commit/fff62ed708153e1c9313930bf36877faad5cd777))
    - LruCache with const-generics ([`93618d1`](https://github.com/GitoxideLabs/gitoxide/commit/93618d107e9defadb603209251f77948caddc121))
    - [experiment] cached version of compound::locate() ([`ec988dc`](https://github.com/GitoxideLabs/gitoxide/commit/ec988dc21320b211f3da9327b63101f954db307e))
    - (cargo-release) version 0.10.0 ([`3161777`](https://github.com/GitoxideLabs/gitoxide/commit/316177729e42f8d000a40ab01b9b97621e7179e8))
    - (cargo-release) version 0.7.0 ([`b900914`](https://github.com/GitoxideLabs/gitoxide/commit/b900914a00292217ba7b9bcac260591800395287))
    - (cargo-release) version 0.12.0 ([`3b71e7e`](https://github.com/GitoxideLabs/gitoxide/commit/3b71e7e8416e550b47e5aed2259c1181497ac9e8))
    - (cargo-release) version 0.2.0 ([`4ec09f4`](https://github.com/GitoxideLabs/gitoxide/commit/4ec09f4d2239ea1d44f7145027e64191bf2c158c))
    - Greatly reduce compound::Object size ([`afa8156`](https://github.com/GitoxideLabs/gitoxide/commit/afa8156c37c6ea93bad7553e5a373fc333398d9b))
    - The git-odb compound Object clearly is too large ([`8f0e813`](https://github.com/GitoxideLabs/gitoxide/commit/8f0e8138ed3313b79b4e358854b9fda5e981f652))
    - Add link to simplified/polonius version in the docs ([`d53c4b0`](https://github.com/GitoxideLabs/gitoxide/commit/d53c4b0f91f1b29769c9430f2d1c0bcab1170c75))
    - Only check alternates for objects not found in packs or loose ([`b317200`](https://github.com/GitoxideLabs/gitoxide/commit/b317200b72096573d511d229c6e61e74e7ba14db))
    - Avoid double-lookup in packs without polonius ([`eaae9c1`](https://github.com/GitoxideLabs/gitoxide/commit/eaae9c1bc723209d793eb93f5587fa2604d5cd92))
    - Thanks clippy ([`0c5f404`](https://github.com/GitoxideLabs/gitoxide/commit/0c5f4043da4615820cb180804a81c2d4fe75fe5e))
    - Remove locate(…) -> Option<Result<…>> in favor of Result<Option<…>> ([`40ee743`](https://github.com/GitoxideLabs/gitoxide/commit/40ee7438a98c4094c0fd04977cd4904668087512))
    - A locate returning Result<Option<Object>> for compound DB ([`a1dafa6`](https://github.com/GitoxideLabs/gitoxide/commit/a1dafa64b4e26dd1456d38f94d58eaadf19abfd3))
    - Use Result<Option<Object>> in Bundle::locate() ([`2dfef8f`](https://github.com/GitoxideLabs/gitoxide/commit/2dfef8f71da456c5c494e1530040589582a046b1))
    - A trial for Result<Option<Object>>  for loose object databases ([`3842859`](https://github.com/GitoxideLabs/gitoxide/commit/3842859c5bddb8b4583443685c26dcae3f8db558))
    - Assure loose objects are actually not found when opening ([`7a4f2bf`](https://github.com/GitoxideLabs/gitoxide/commit/7a4f2bf2cb31407422be2e563b3df210bbf8bfd0))
    - Add feature toggle for polonius and make it part of the test suite ([`c825c11`](https://github.com/GitoxideLabs/gitoxide/commit/c825c11e2d17141b38654d30b37e043dfae383f3))
    - (cargo-release) version 0.9.1 ([`e0feb28`](https://github.com/GitoxideLabs/gitoxide/commit/e0feb28b50ce55be71b24ea5238a760f5b1f8d3b))
    - (cargo-release) version 0.9.0 ([`efc8983`](https://github.com/GitoxideLabs/gitoxide/commit/efc898381d830e44487c62e35a665d3ccd0a2d39))
    - Thanks clippy ([`0fc239c`](https://github.com/GitoxideLabs/gitoxide/commit/0fc239cf9b773f72928b7c42344b578c6ff5d19f))
    - Refactor ([`f2e9add`](https://github.com/GitoxideLabs/gitoxide/commit/f2e9add3cb5803426a2e36a3b462f823e8cef44b))
    - Upgrade depdendencies ([`e4a7711`](https://github.com/GitoxideLabs/gitoxide/commit/e4a77112ee4f5d0ab61d0678aab8ee090335740c))
    - Fix compile warnings produced by +nightly build ([`e387d2c`](https://github.com/GitoxideLabs/gitoxide/commit/e387d2c148e231321f88e5fb1b2988437475d2c0))
    - Merge pull request #50 from Byron/edward-shen/odb-zlib-ng ([`acb90d7`](https://github.com/GitoxideLabs/gitoxide/commit/acb90d755fb02c37f8a5a431778abcbe143fb5e5))
    - Conform imports ([`fd73731`](https://github.com/GitoxideLabs/gitoxide/commit/fd737317379af80f8e0ba9a9a8fc505fb60fd177))
    - Fix error type argument order and spell fields out ([`819568e`](https://github.com/GitoxideLabs/gitoxide/commit/819568e9c5be14cec1e9e1cdc915b4c286c2ed00))
    - [git-odb] Return error on invalid packs ([`88de64d`](https://github.com/GitoxideLabs/gitoxide/commit/88de64d433b44996d5f8be733b50e1949c71e42d))
    - [git-odb] Fix Inflate::once ([`36f6bbd`](https://github.com/GitoxideLabs/gitoxide/commit/36f6bbd451a5474cb6dac0259904e4aed7fd11d9))
    - [git-odb] Remove unnecessary tests ([`ebe41ca`](https://github.com/GitoxideLabs/gitoxide/commit/ebe41cadc4acb38326e59d193fd3b1e501146943))
    - [gix] Use flate2 by default ([`f1158a1`](https://github.com/GitoxideLabs/gitoxide/commit/f1158a1a4bc8e13913461db4d4851e32d57816ff))
    - [gix] Add optional zlib feature ([`f1f9665`](https://github.com/GitoxideLabs/gitoxide/commit/f1f96658a6cd6165ba9c9d7acb809fcaf2c46f9c))
    - [git-odb] Add feature flag for zlib-ng ([`96b3810`](https://github.com/GitoxideLabs/gitoxide/commit/96b3810995f9e7b0164234dcb9f3b28b0c9b5224))
    - (cargo-release) version 0.8.0 ([`1ccfdcd`](https://github.com/GitoxideLabs/gitoxide/commit/1ccfdcdb96b59c6415e7fbc800371d594b2ef7a1))
    - (cargo-release) version 0.7.1 ([`2c38ff9`](https://github.com/GitoxideLabs/gitoxide/commit/2c38ff909cd5ed39995d4ac3b5af3e0da2f3b76d))
    - (cargo-release) version 0.11.0 ([`1aa1f5e`](https://github.com/GitoxideLabs/gitoxide/commit/1aa1f5e84a07427d5d7f3231735fe9c1923f506f))
    - Require latest version of git-features in git-odb ([`e664e93`](https://github.com/GitoxideLabs/gitoxide/commit/e664e93e960564c43a5510d32bf5ff45624af8ee))
    - Remove usage of gitfeatures::fs in organize subcommand ([`b567d37`](https://github.com/GitoxideLabs/gitoxide/commit/b567d3709a74e9fdafef54b0fe58ca82721cd773))
    - Refactor; planning ([`5df492c`](https://github.com/GitoxideLabs/gitoxide/commit/5df492c7d7322bde2b268deaf590f1ba012a6b8e))
    - Thanks clippy ([`343ab9a`](https://github.com/GitoxideLabs/gitoxide/commit/343ab9adb62da1dde495fc209c179137bbe59a10))
    - Refactor ([`5b1328f`](https://github.com/GitoxideLabs/gitoxide/commit/5b1328fc48deab321f81d25b5dc8e9ba55840e2c))
    - Add missing '.' at end of doc comments ([`7136854`](https://github.com/GitoxideLabs/gitoxide/commit/71368544f97369a4d371d43513607c4805bd0fd0))
    - Fix git-odb tests ([`35c1209`](https://github.com/GitoxideLabs/gitoxide/commit/35c1209164b5baaa68d1c566344ac73ee6dfae79))
    - All crates use git-hash::Kind and its types, sometimes through git-object ([`124c171`](https://github.com/GitoxideLabs/gitoxide/commit/124c171aaf546d8977e9913ff84e65383a80ee98))
    - Use git-hash in git-features ([`5b307e0`](https://github.com/GitoxideLabs/gitoxide/commit/5b307e076f6f5975592c8b177c122c91c1d809c6))
    - (cargo-release) version 0.6.0 ([`27f5955`](https://github.com/GitoxideLabs/gitoxide/commit/27f5955e047f35e21a86789eb46bfd89e1c99b44))
    - (cargo-release) version 0.9.0 ([`a89fdb9`](https://github.com/GitoxideLabs/gitoxide/commit/a89fdb98f64bb0ca070fa79a1f58f1232bb14090))
    - (cargo-release) version 0.5.0 ([`fc7d600`](https://github.com/GitoxideLabs/gitoxide/commit/fc7d600ac2c438c8b6b91f67cb69b0ac5ec37675))
    - (cargo-release) version 0.5.0 ([`c767e07`](https://github.com/GitoxideLabs/gitoxide/commit/c767e07ccfc58a28e3e8ec22b590afdf0d92b9f2))
    - More docs for owned git-object ([`b79101d`](https://github.com/GitoxideLabs/gitoxide/commit/b79101d714f59a42a30eb47776486a212ec0f738))
    - Thanks clippy ([`ba9b3c2`](https://github.com/GitoxideLabs/gitoxide/commit/ba9b3c2345887353e02fc081be80733f1c5e22d9))
    - Refactor ([`d5d7cf9`](https://github.com/GitoxideLabs/gitoxide/commit/d5d7cf9d3f42d83652a7b81bc6e1ee6731396d6b))
    - More docs of git-object::owned ([`0620dce`](https://github.com/GitoxideLabs/gitoxide/commit/0620dce7a3ac368354c73e3927eb96a6e4990f0c))
    - (cargo-release) version 0.8.0 ([`47c00c2`](https://github.com/GitoxideLabs/gitoxide/commit/47c00c2228cf25c79e1fa3eb4229c7ab24de91e5))
    - Cargo clippy Rust 1.48 ([`475a68c`](https://github.com/GitoxideLabs/gitoxide/commit/475a68ce33b895de911939c51afa159df534f7b8))
    - Finish refactoring git-odb ([`ec282ae`](https://github.com/GitoxideLabs/gitoxide/commit/ec282ae1a3d9f16eb9c89a44e17259112d097a41))
    - (cargo-release) version 0.7.0 ([`7fa7bae`](https://github.com/GitoxideLabs/gitoxide/commit/7fa7baeb3e7d008a25e4d714eff908e2516c828b))
    - (cargo-release) version 0.4.2 ([`173c957`](https://github.com/GitoxideLabs/gitoxide/commit/173c957032761705edc61a0ded1f963cac73c320))
    - Minor fixes to git-odb docs ([`3788512`](https://github.com/GitoxideLabs/gitoxide/commit/37885125d7c4d1dba7aaff37b5d39a7c249bf794))
    - Complete docs for git-odb ([`0cf8496`](https://github.com/GitoxideLabs/gitoxide/commit/0cf84967feed768bc29de29f65f6dc4622008464))
    - Prefer empty doc strings for modules over [allow missing docs] ([`9b3f04f`](https://github.com/GitoxideLabs/gitoxide/commit/9b3f04f4247d6d2a139f813ea2555203d374962a))
    - Add remaining doc strings for git-odb ([`428f0ad`](https://github.com/GitoxideLabs/gitoxide/commit/428f0ad2072148416b54b050add9a50868e7e5d0))
    - Some more docs ([`2d87124`](https://github.com/GitoxideLabs/gitoxide/commit/2d87124344af845a34d17693f5ef04c9fb3323e1))
    - Try to document all the bits an pieces of git-odb ([`1b353fa`](https://github.com/GitoxideLabs/gitoxide/commit/1b353fa95723a7fe4ddef0a70486a74957e727cd))
    - Finish docs on top-level traversal method ([`2ef1c99`](https://github.com/GitoxideLabs/gitoxide/commit/2ef1c99a48c39cb9f3362a5ea493b5e90e4593c9))
    - Start describing how pack traversal works ([`5e990f2`](https://github.com/GitoxideLabs/gitoxide/commit/5e990f20dee6005d23ebc5a56389f09d9d7f8782))
    - Refactor ([`a681335`](https://github.com/GitoxideLabs/gitoxide/commit/a681335b51c10ff56ddd2fe80ec24449a771abd2))
    - Document pack::index::write ([`f5edc60`](https://github.com/GitoxideLabs/gitoxide/commit/f5edc602cb3e570ce154a3ba3d692fcbcf8d55c0))
    - Dependency update ([`bc336d9`](https://github.com/GitoxideLabs/gitoxide/commit/bc336d9bb22d13a6d2407b44b297fcb770cdaac6))
    - Refactor ([`6b909a2`](https://github.com/GitoxideLabs/gitoxide/commit/6b909a22cf981b33060cb6f1324ec3231146d159))
    - Refactor ([`b511a2b`](https://github.com/GitoxideLabs/gitoxide/commit/b511a2b1d9b6d55b1937ad3f4bbbb331b5cdd9a3))
    - Document index integrity checking ([`9336ab9`](https://github.com/GitoxideLabs/gitoxide/commit/9336ab9f9675ba5d33eacefc585d745e1b0bcc18))
    - Docs for index access ([`996acbf`](https://github.com/GitoxideLabs/gitoxide/commit/996acbf67fde183a0e5f553ecad9b2361eecf18b))
    - Docs for top level pack index module ([`d2dd72f`](https://github.com/GitoxideLabs/gitoxide/commit/d2dd72fe2d230ecdd88343535ecdbfbd8ae1b143))
    - Document pack data verification ([`27962ca`](https://github.com/GitoxideLabs/gitoxide/commit/27962ca9019d0b4971fa76afedaf1d85f451665b))
    - Document pack entry iteration ([`c869ee9`](https://github.com/GitoxideLabs/gitoxide/commit/c869ee93c6f042ce3de4962229e2caa4377af62b))
    - Docs for pack header ([`9505b40`](https://github.com/GitoxideLabs/gitoxide/commit/9505b401a87c3107ac1e5775ff6c10e8a808ba25))
    - Some more pack data file documentation ([`05e05f4`](https://github.com/GitoxideLabs/gitoxide/commit/05e05f46a38bcc068b564409d92310dd93ca5527))
    - Docs for Bundle::write_* ([`ac41253`](https://github.com/GitoxideLabs/gitoxide/commit/ac41253067803796e5623184d7dee790aa597809))
    - Remove special Error with just one variant… ([`d05a416`](https://github.com/GitoxideLabs/gitoxide/commit/d05a416dc43164f4c9fb2ee00884107fdbd13f90))
    - Docs for Bundle::locate ([`066787c`](https://github.com/GitoxideLabs/gitoxide/commit/066787c12e3142732d3ba65b233c836f89745543))
    - Some more docs for 'pack' module ([`c32850d`](https://github.com/GitoxideLabs/gitoxide/commit/c32850d4b6f94dd636d09b6222d2aa7ee6a85c82))
    - Some more documentation ([`201f67c`](https://github.com/GitoxideLabs/gitoxide/commit/201f67ce52e39dde3a79ff8a1f05bbaf30deec15))
    - Merge branch 'commit-graph' into main ([`9cb09b2`](https://github.com/GitoxideLabs/gitoxide/commit/9cb09b248796f0ff5c9d3f3e857de4731324cfd5))
    - Specify the hash to create with 'hash::bytes_of_file' ([`c000294`](https://github.com/GitoxideLabs/gitoxide/commit/c000294423ae0759b978399db3b69ac07c20578d))
    - Move 'git_odb::hash::bytes_of_file' into git_features::hash ([`c5f6b45`](https://github.com/GitoxideLabs/gitoxide/commit/c5f6b4587ee4042a080c0505613b0c72fdfe5273))
    - The daily commit (single handedly) ([`b528c2e`](https://github.com/GitoxideLabs/gitoxide/commit/b528c2e1bf0a3211491535427c4bd36212711a0f))
    - Document `loose::Object` entirely ([`d5eef9c`](https://github.com/GitoxideLabs/gitoxide/commit/d5eef9cdd06910eeaf1f1c4114b97638a29c7327))
    - Thanks clippy ([`b9e0a87`](https://github.com/GitoxideLabs/gitoxide/commit/b9e0a87996b8f3c4531a392607c353a1f0824ce6))
    - Docs for Sink ([`e7a09f0`](https://github.com/GitoxideLabs/gitoxide/commit/e7a09f0628b44ae0c6b564ef41f044e51866f2df))
    - Docs for compound object databases ([`813df71`](https://github.com/GitoxideLabs/gitoxide/commit/813df7115eb643742158f975975eb7469443cc07))
    - Document borrowed odb objects ([`7626f7f`](https://github.com/GitoxideLabs/gitoxide/commit/7626f7f3af885f1b95801f9dbc71bee0bc77400e))
    - Document alternates implementation ([`60666e8`](https://github.com/GitoxideLabs/gitoxide/commit/60666e86316c81f3bb63ee151e457af78dbefc00))
    - Docs for git-odb crate (top-level) ([`71af366`](https://github.com/GitoxideLabs/gitoxide/commit/71af366c84e1bd00125b4582d80799a6d927324a))
    - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
    - Merge branch 'main' into commit-graph ([`ca5b801`](https://github.com/GitoxideLabs/gitoxide/commit/ca5b80174b73cc9ac162b3f33b5d3721ef936cb1))
    - Thanks clippy ([`e355b4a`](https://github.com/GitoxideLabs/gitoxide/commit/e355b4ad133075152312816816af5ce72cf79cff))
    - Refactor ([`5a1cbf2`](https://github.com/GitoxideLabs/gitoxide/commit/5a1cbf299f2d5c1c07143d14ee3ded95d6a44a20))
    - And octal values unquoting works too ([`5effc7b`](https://github.com/GitoxideLabs/gitoxide/commit/5effc7b6daf6ff49b6d51af09f8da148602c7322))
    - All explicit escapes ([`1841544`](https://github.com/GitoxideLabs/gitoxide/commit/18415445caaee6e9e54aabddb88bdcd2f5602508))
    - First bunch of simple unescapes ([`a45c594`](https://github.com/GitoxideLabs/gitoxide/commit/a45c5941cf426537710842917c0e715cf4c74863))
    - Prepare for actual unescaping ([`284da44`](https://github.com/GitoxideLabs/gitoxide/commit/284da449cae62d12ea4eea8c31f1225699c5e52e))
    - Basic infrastructure for unquoting c-style strings ([`f81bb03`](https://github.com/GitoxideLabs/gitoxide/commit/f81bb038bfc8ea0d9b61012d6effae084c89335a))
    - Fix incorrect cycle detection, which worked on MacOS by accident ([`a6e7765`](https://github.com/GitoxideLabs/gitoxide/commit/a6e77650a886ac33b23af8892182c9832a86e997))
    - Also use alternates for looking up objects… ([`ade929d`](https://github.com/GitoxideLabs/gitoxide/commit/ade929df38e619850f73389178a2c53e1c43fa45))
    - Prepare for unquoting c-strings ([`47e2fa0`](https://github.com/GitoxideLabs/gitoxide/commit/47e2fa03a1e2fe163c5c019d52bbb0ddbdb80bf0))
    - Read multiple alternates from single file; ignore comments ([`1f8d367`](https://github.com/GitoxideLabs/gitoxide/commit/1f8d36705c4568b1036b0d62b3a80ae6ec20a99c))
    - Support for relateive alternates ([`b20e9ee`](https://github.com/GitoxideLabs/gitoxide/commit/b20e9eea423ced275781d410217110c85ddb587c))
    - Ignore all cycles and be happy if we have found at least one actual odb ([`1effdfd`](https://github.com/GitoxideLabs/gitoxide/commit/1effdfda703d5eb9cd1333a7bae21075ef9e53cc))
    - Prepare for multi-line parsing and all the bells and whistles ([`08f9ec4`](https://github.com/GitoxideLabs/gitoxide/commit/08f9ec41feee56fe0ff2b057bb50391100bdb84e))
    - Make compound DB initialization less lazy… ([`6dc57b3`](https://github.com/GitoxideLabs/gitoxide/commit/6dc57b31d0bc5abfca100ab1d4b5dff68852aad8))
    - Use parallel walkdir (via jwalk) when parallel feature is enabled ([`f444c85`](https://github.com/GitoxideLabs/gitoxide/commit/f444c859f5b215ea70a46d5493a2babbf7a98235))
    - Alternate now handles cycles ([`71167e4`](https://github.com/GitoxideLabs/gitoxide/commit/71167e4e50efa8a097c3b09a249004e97aeaf2c8))
    - First simple alternate tests ([`7372118`](https://github.com/GitoxideLabs/gitoxide/commit/73721185cfd646c6e83b2548280fad8f480f8324))
    - Test for circular alternates ([`fc92709`](https://github.com/GitoxideLabs/gitoxide/commit/fc927091d69196a930c0cea4611af8d96b7b84d8))
    - Thanks clippy ([`4ddc64f`](https://github.com/GitoxideLabs/gitoxide/commit/4ddc64fd71d3d1260e001f89c379c46fe157e5ce))
    - Actually resolve alternates when creating a compound DB ([`9be7aed`](https://github.com/GitoxideLabs/gitoxide/commit/9be7aed7bd4b939d98b9a8d1db8a6ffc85044ca9))
    - Refactor ([`c1eff58`](https://github.com/GitoxideLabs/gitoxide/commit/c1eff58cd28e45a2d5f46481551724b81735ede3))
    - First sketch of alternate resolution ([`6cc8a94`](https://github.com/GitoxideLabs/gitoxide/commit/6cc8a947df776aeeb031de627f84b7bc85207235))
    - Remove quickerror dependency from git-odb ([`7e27495`](https://github.com/GitoxideLabs/gitoxide/commit/7e2749521b6c873766a2f6f96e6c91a0c6a9dbf3))
    - Refactor ([`7874c35`](https://github.com/GitoxideLabs/gitoxide/commit/7874c35bccb74ae7670335e633efa7eaebc72630))
    - Refactor ([`3ec99dc`](https://github.com/GitoxideLabs/gitoxide/commit/3ec99dc7360c01b4f3c4593ff5049361e7043254))
    - Refactor ([`519dd12`](https://github.com/GitoxideLabs/gitoxide/commit/519dd12f2bf58dd3048bc12e5b058236ad727853))
    - Refacator ([`7ac2153`](https://github.com/GitoxideLabs/gitoxide/commit/7ac21536b3cee6708489011731216b9b831509e4))
    - Refactor ([`d4f288c`](https://github.com/GitoxideLabs/gitoxide/commit/d4f288ceb2436b292993df74ed07d4d7e578edf1))
    - Refactor ([`3a8fb61`](https://github.com/GitoxideLabs/gitoxide/commit/3a8fb61067c210d4db6d515f21b2e28425c52e8c))
    - Refactor ([`98b3f4a`](https://github.com/GitoxideLabs/gitoxide/commit/98b3f4a9cc65e76aa09280065ab1d151f637e692))
    - Refactor ([`127b8b2`](https://github.com/GitoxideLabs/gitoxide/commit/127b8b2949476b38ef6f8ea0fb68bae6d473adcc))
    - Refactor ([`669b726`](https://github.com/GitoxideLabs/gitoxide/commit/669b726da305ce4520c792d62b4344b04fe5f996))
    - Refactor ([`7bc321e`](https://github.com/GitoxideLabs/gitoxide/commit/7bc321e96ecce0aae5063eb7008ecbac7d2ca31c))
    - Refactor ([`0752b45`](https://github.com/GitoxideLabs/gitoxide/commit/0752b45e95dd5378b7fca5b70bd11b9100ba2946))
    - (cargo-release) version 0.4.1 ([`60ac8b0`](https://github.com/GitoxideLabs/gitoxide/commit/60ac8b0a7545fff6ef293fd48716e9a19175517c))
    - Refactor ([`ad17bfd`](https://github.com/GitoxideLabs/gitoxide/commit/ad17bfdc07e1301693fdfa3d09df3b39f675a36f))
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Refactor ([`91d9f78`](https://github.com/GitoxideLabs/gitoxide/commit/91d9f78a9af04b44b2cead30d6e1cbaaeb76a522))
    - Refactor ([`6ebb5d1`](https://github.com/GitoxideLabs/gitoxide/commit/6ebb5d1839cd5ab4d8aff78acbccebaa66f439c7))
    - Refactor ([`8877b77`](https://github.com/GitoxideLabs/gitoxide/commit/8877b776bda8d1f202e86ac471ea30b595cff41b))
    - Refactor ([`4a0d034`](https://github.com/GitoxideLabs/gitoxide/commit/4a0d0342a20f519f30fe8b84d51ebb2bdea23752))
    - Refactor ([`485aa91`](https://github.com/GitoxideLabs/gitoxide/commit/485aa91c7412c55c0215e33cc4f906dd62e440a8))
    - Refactor ([`c1d2f41`](https://github.com/GitoxideLabs/gitoxide/commit/c1d2f41938211985a6cdb7a0fde6bcb51a7944de))
    - Refactor ([`07aff14`](https://github.com/GitoxideLabs/gitoxide/commit/07aff14a8c2ceca3202b0506b3bd4286550ac3a0))
    - Refactor ([`57d463f`](https://github.com/GitoxideLabs/gitoxide/commit/57d463ffeb5861270abaaf72f662b14c9c262052))
    - Refactor ([`c6be43d`](https://github.com/GitoxideLabs/gitoxide/commit/c6be43de3493566cedd98ce49fb2c8af7714a61c))
    - Refactor ([`524d0fe`](https://github.com/GitoxideLabs/gitoxide/commit/524d0fec17c356c846f0c62f87f2637a7a6fa56b))
    - Refactor ([`a8f4cd7`](https://github.com/GitoxideLabs/gitoxide/commit/a8f4cd7b9c31e59c3329cc649aca8378cd34a597))
    - Checksum verification for compound object ([`3be08b0`](https://github.com/GitoxideLabs/gitoxide/commit/3be08b09cd71e5e5eb21bdd81d6a07d2c232e6e8))
    - Refactor ([`59d989a`](https://github.com/GitoxideLabs/gitoxide/commit/59d989a9c86789d6572c9a3dfd8a3652bd4a7a1b))
    - More methods for compound object ([`84d2b0e`](https://github.com/GitoxideLabs/gitoxide/commit/84d2b0ec53f7def1470fbadff45fbe266bceb71a))
    - Refactor ([`e5a9343`](https://github.com/GitoxideLabs/gitoxide/commit/e5a9343f3f5304de4c9f614cdb260cf0fcfbbbfb))
    - Refactor ([`6a84f13`](https://github.com/GitoxideLabs/gitoxide/commit/6a84f137754cddfdcb9b1fec3e353762ebb3ce2b))
    - Refactor ([`4e89c3b`](https://github.com/GitoxideLabs/gitoxide/commit/4e89c3bc0f14cf9581348ae2c1620ade9dc1db8f))
    - Document why we won't use nightly for fixing NLL issue ([`ca29368`](https://github.com/GitoxideLabs/gitoxide/commit/ca29368b42b902fe7fe14dd4bff1b35e266c96a8))
    - Revert "Fix NLL issue by using nightly" ([`6864a55`](https://github.com/GitoxideLabs/gitoxide/commit/6864a55001f1d01839f948618355928d666602ee))
    - Fix NLL issue by using nightly ([`8c5bd09`](https://github.com/GitoxideLabs/gitoxide/commit/8c5bd095539042d7db0e611460803cdbf172beb0))
    - Update tasks, prepare for NLL fix ([`52af8d1`](https://github.com/GitoxideLabs/gitoxide/commit/52af8d1089dc85cff19aee276bd831393f1b84b3))
    - Thanks clippy ([`6c4d1ec`](https://github.com/GitoxideLabs/gitoxide/commit/6c4d1ec33d37b99b38698dfd91d38216ab4a2ef2))
    - This works, but locates twice… ([`4e709f6`](https://github.com/GitoxideLabs/gitoxide/commit/4e709f6029cf98f8c6ff204598706e2b6a1775eb))
    - Also the imperative version doesn't borrowcheck… ([`c5720f1`](https://github.com/GitoxideLabs/gitoxide/commit/c5720f1e4dc790539980fa81e940be6c6e15b50a))
    - Looks like the functional approach to locate(…) doesn't borrowcheck ([`5df6867`](https://github.com/GitoxideLabs/gitoxide/commit/5df6867a2b9fa7ba3fe2cdcd3bb9766faba1ae1b))
    - Refactor ([`9e68c6b`](https://github.com/GitoxideLabs/gitoxide/commit/9e68c6bcd1d07ea73730ce5ff59d7883152f894d))
    - Refactor ([`f219d5a`](https://github.com/GitoxideLabs/gitoxide/commit/f219d5a25efb7e258249ca3a4d39382136fe4229))
    - Sketch compound::Db::locate; sort packs by size ([`6609a53`](https://github.com/GitoxideLabs/gitoxide/commit/6609a534f45fc1ffc9d80a60a6a9793cbf54131c))
    - Refactor ([`4a09754`](https://github.com/GitoxideLabs/gitoxide/commit/4a09754f6cd17d7f39f8a71b7de44d517294ffc5))
    - Implement Write in terms of writing to the loose object DB ([`02b88c2`](https://github.com/GitoxideLabs/gitoxide/commit/02b88c28304ff6d8c1fbad6fdcfa36f3b1f9dafc))
    - First sketch of compound Db ([`9bf2279`](https://github.com/GitoxideLabs/gitoxide/commit/9bf227914d9281bfbdfc902edc3c1cc21c7fa3cd))
    - Refactor ([`203ba99`](https://github.com/GitoxideLabs/gitoxide/commit/203ba995c9e237ac63bc2ecebda18171e90fcb47))
    - (cargo-release) version 0.6.0 ([`9ef184e`](https://github.com/GitoxideLabs/gitoxide/commit/9ef184e35712f938fb4f9f6da7390a8777a9284e))
    - Revert "FAIL: try to get rid of tree-traversal Boxed error…" ([`1b42b31`](https://github.com/GitoxideLabs/gitoxide/commit/1b42b3141dded644a17c8d23057c987e2bac4f80))
    - Try to get rid of tree-traversal Boxed error… ([`13159eb`](https://github.com/GitoxideLabs/gitoxide/commit/13159eb972ed78ce4ebee2313b288023cec91c47))
    - Parameterize traversal error with Processor error ([`1513a13`](https://github.com/GitoxideLabs/gitoxide/commit/1513a13179bedefd12fc08da07a05c7f07ed4ef6))
    - Switch to prodash 10 and safe a lot of trait bounds in the process ([`e2fb1d9`](https://github.com/GitoxideLabs/gitoxide/commit/e2fb1d944b4d803a11c91f868b831d406fb5e35f))
    - (cargo-release) version 0.4.0 ([`2272fa4`](https://github.com/GitoxideLabs/gitoxide/commit/2272fa4bcacdaf1898e4cd8b791232fc1321227f))
    - (cargo-release) version 0.4.0 ([`0d7b60e`](https://github.com/GitoxideLabs/gitoxide/commit/0d7b60e856325009431172e1df742a1cd2165575))
    - (cargo-release) version 0.5.0 ([`82b7313`](https://github.com/GitoxideLabs/gitoxide/commit/82b73131b79ec3c42a712dad1c0766a72209d737))
    - [clone] All it took was a an intermediary to call 'read' as expected ([`7c8ecb7`](https://github.com/GitoxideLabs/gitoxide/commit/7c8ecb78e988f7752cea6fe2022ccf9739b86fd4))
    - [clone] minor refactor; it's definitely the read() that doesn't work… ([`406829b`](https://github.com/GitoxideLabs/gitoxide/commit/406829b951164673c0b8152d1e9de76f1318df0a))
    - [clone] This actually works: first MVP of retrieving packs via clone ([`c06d819`](https://github.com/GitoxideLabs/gitoxide/commit/c06d8194173f9ec468ddd0faf72dd6d8dbf7d35d))
    - [clone] support for progress that can handle writing pack files ([`46e0055`](https://github.com/GitoxideLabs/gitoxide/commit/46e0055eab47e402807b15c63b6a4577f5c0b7bb))
    - Use fast-reset for miniz oxide to gain about 4s when resolving the kernel pack ([`e5b6ce4`](https://github.com/GitoxideLabs/gitoxide/commit/e5b6ce440073c1db32ed4afc8e1db21b32f62863))
    - Fix build ([`6178133`](https://github.com/GitoxideLabs/gitoxide/commit/6178133fd1e08af6abb90ba7d1a4c22970da850c))
    - Refactor ([`174baa7`](https://github.com/GitoxideLabs/gitoxide/commit/174baa7733d34d1dbb2d47f4163ca39fb4a4c473))
    - Bump git-features to 0.4 to allow publishes after breaking changes ([`9d6b879`](https://github.com/GitoxideLabs/gitoxide/commit/9d6b8790e2edd7fa01b3239adff86a7cd2393f10))
    - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
    - Refactor ([`b4a6e16`](https://github.com/GitoxideLabs/gitoxide/commit/b4a6e16364822c0dccb56f98dbfb0ca4c8007069))
    - Remove tree compaction code ([`dfc6c7d`](https://github.com/GitoxideLabs/gitoxide/commit/dfc6c7dde9014e79eb4a752d81bc3c77ad36c366))
    - See if tree compaction saves considerable amounts of memory ([`0092c25`](https://github.com/GitoxideLabs/gitoxide/commit/0092c256b3bfaf2818566540e660cdefcf68d246))
    - Bump minor version to 0.3 ([`4351e28`](https://github.com/GitoxideLabs/gitoxide/commit/4351e2871c9dcf342b8471fffa74cae338a53269))
    - Thanks clippy ([`6725104`](https://github.com/GitoxideLabs/gitoxide/commit/6725104d2841e6518db641d06e3e107cf4f40f96))
    - Also run file hashing in indexed mode in parallel (like with lookup) ([`8f8d14f`](https://github.com/GitoxideLabs/gitoxide/commit/8f8d14f4606e99dc710eb352a985db48c00ea4f4))
    - First step towards parallelizing file hashes and traversal! ([`9573836`](https://github.com/GitoxideLabs/gitoxide/commit/95738369e0d3accf7f6239c8cd966a7f5c36825a))
    - Allow hashing to be interrupted ([`df4dfd7`](https://github.com/GitoxideLabs/gitoxide/commit/df4dfd7ec1be608cec1117f56303c528fb8f7ba7))
    - Unify file based file verification of data and index ([`e1b4105`](https://github.com/GitoxideLabs/gitoxide/commit/e1b4105308963cfe9102c2dee461c7dd948ee942))
    - Update to quick-error 2.0 ([`4b1b784`](https://github.com/GitoxideLabs/gitoxide/commit/4b1b7849b47a54092b49821c39e864c86adda979))
    - Haha, funny, silly me… ([`a4a1244`](https://github.com/GitoxideLabs/gitoxide/commit/a4a1244af2d386e75ebd55909d4675b475ccd905))
    - Better usability for units ([`b226253`](https://github.com/GitoxideLabs/gitoxide/commit/b226253636d8146a084a7bcd7c0c320e37f9d2fb))
    - Better progress for Sha1 of pack and index ([`310a59e`](https://github.com/GitoxideLabs/gitoxide/commit/310a59ee99ce78a4f14326c0058ea0c543a1d24c))
    - Make obvious that interrupt request was received ([`34b2373`](https://github.com/GitoxideLabs/gitoxide/commit/34b23737f560fe52d4f98fb886eba754652f9a5e))
    - Conditionally use an eager iterator… ([`e9b5511`](https://github.com/GitoxideLabs/gitoxide/commit/e9b5511568f4e64968596994855783f19672d678))
    - Reduce progress information for threads ([`e9a1b31`](https://github.com/GitoxideLabs/gitoxide/commit/e9a1b310fd99675dc87e56c6277d57259a6415a0))
    - Revert "A test to see how much time can be saved by not zeroing zlib buffers" ([`3d51d59`](https://github.com/GitoxideLabs/gitoxide/commit/3d51d595469d8451867331089e75b808f9361912))
    - A test to see how much time can be saved by not zeroing zlib buffers ([`fd41a51`](https://github.com/GitoxideLabs/gitoxide/commit/fd41a51de2261f425262ee7dee7a24fd87d87432))
    - Implement optionally keeping the compressed bytes ([`fc26914`](https://github.com/GitoxideLabs/gitoxide/commit/fc26914a57b6e89c703e1b04d6a4d8d31005ddbc))
    - First step towards more control over allocation in iterator ([`cacf76c`](https://github.com/GitoxideLabs/gitoxide/commit/cacf76cd69996073894e400e65322d3547493789))
    - Never keep decompressed bytes while streaming… ([`65c3856`](https://github.com/GitoxideLabs/gitoxide/commit/65c38569219134ccd412a8a3ee7ec618d866c941))
    - Only keep base objects, not the deltas (like originally intended) ([`fc8334b`](https://github.com/GitoxideLabs/gitoxide/commit/fc8334b8d196425f2b766ebb9772a12483ef0f8d))
    - Reduce footprint of sha1 when writing the index ([`12aa454`](https://github.com/GitoxideLabs/gitoxide/commit/12aa4549bee0d9ea00bb0723acefa8187f5119a9))
    - First successful test of moving the streaming iterator into its own thread ([`c9fcb68`](https://github.com/GitoxideLabs/gitoxide/commit/c9fcb68c644c96a15cb9956a754bec7b65bb5fbd))
    - First sketch of order-destroying eager iterator ([`20fca45`](https://github.com/GitoxideLabs/gitoxide/commit/20fca4515f6e9ea320d0bf21c15cd6d2c3cff742))
    - Add object size progress when resolving with index ([`b2f8c9e`](https://github.com/GitoxideLabs/gitoxide/commit/b2f8c9e85dfac63f70ca7b0e91af697b801b4131))
    - Add decompression progress ([`0e5c534`](https://github.com/GitoxideLabs/gitoxide/commit/0e5c534d7c6e661a1f6c1cdb59ad1c9ffade642d))
    - Print read throughput automatically ([`0a71b48`](https://github.com/GitoxideLabs/gitoxide/commit/0a71b482310a129aa8757475290b3b24a200b702))
    - Allow 'read' progress to go out of scope while keeping it accessible! ([`d7a7828`](https://github.com/GitoxideLabs/gitoxide/commit/d7a782899ca841291e240bad822bb8184d6f5083))
    - Fix throughput display of otherwise stepped progress indicators ([`399f81d`](https://github.com/GitoxideLabs/gitoxide/commit/399f81daadb8c111b9cad958945924e0eed2c2ad))
    - Unify used ranges for line renderer amond pretty and lean interface ([`f59f66e`](https://github.com/GitoxideLabs/gitoxide/commit/f59f66e189732f567414f68c7463364e510f41c4))
    - Add percentage and throughput to tasks that matter ([`763d7ca`](https://github.com/GitoxideLabs/gitoxide/commit/763d7caa4c70111b7cb3ef5733d2c3c697758c28))
    - Upgrade to latest iteration of prodash ([`3a4faec`](https://github.com/GitoxideLabs/gitoxide/commit/3a4faecab56e37670c553e6563f11a46d740c333))
    - First part of migration to prodash 8.0, but… ([`6901a09`](https://github.com/GitoxideLabs/gitoxide/commit/6901a098641820c8d974ce56a24d6cdca779730d))
    - Fix various issues related to 64bit offset support when writing indices… ([`da31694`](https://github.com/GitoxideLabs/gitoxide/commit/da31694ee13022bcc52ed06389469d65b4e37daa))
    - Fix unit tests: actually sort the directory entries :D ([`b69717a`](https://github.com/GitoxideLabs/gitoxide/commit/b69717af368510347a550012f4ed97ba24d36ffd))
    - Add convenience method to get a new bundle for the index/data just written ([`a6d74ad`](https://github.com/GitoxideLabs/gitoxide/commit/a6d74ad7b65cdc293c8504dae73ea1c717e5bfca))
    - Bundle write with a given directory ([`7f29c73`](https://github.com/GitoxideLabs/gitoxide/commit/7f29c73d35b8717c8834beac259ed71eebcc2058))
    - First unit test for bundle writing ([`74bda39`](https://github.com/GitoxideLabs/gitoxide/commit/74bda3963af7fe4b97e7f04f0bb9e150df8b7fa7))
    - Journey tests for restore functionality ([`1aa63e4`](https://github.com/GitoxideLabs/gitoxide/commit/1aa63e419736960915c03c29827a57c18261e04d))
    - Refactor ([`fc42567`](https://github.com/GitoxideLabs/gitoxide/commit/fc4256788f7c3d3c4a05f240eee4d71a716cafce))
    - Refactor ([`cf3ebe0`](https://github.com/GitoxideLabs/gitoxide/commit/cf3ebe00619d16e957166578038520b2bf080411))
    - Refactor ([`72ca435`](https://github.com/GitoxideLabs/gitoxide/commit/72ca435a90e470797ae59dd10640c36b84bb4f41))
    - More flexible error types for processors - anything goes ([`be3a947`](https://github.com/GitoxideLabs/gitoxide/commit/be3a947ba6197319fea0b38e48008850cc971bf6))
    - Refactor ([`c7dd581`](https://github.com/GitoxideLabs/gitoxide/commit/c7dd581348a05146d7a79f7622bf30a08d34f474))
    - Refactor ([`aae8e79`](https://github.com/GitoxideLabs/gitoxide/commit/aae8e79a89261a548f088454ca6082a34c2063ce))
    - Refactor ([`0e27763`](https://github.com/GitoxideLabs/gitoxide/commit/0e27763995e135fc1bca56e6084b5c81825dba22))
    - Make lookup based algorithm gracefully interruptible ([`8d2e649`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e649c754d713e6dd48315cd043204ffda4a7b))
    - Write about user interfaces and the use/non-use of async ([`91ba045`](https://github.com/GitoxideLabs/gitoxide/commit/91ba0457745f860b7a68cb38b13e69754747e8d9))
    - Use pack hash for index file as well :D ([`2106c64`](https://github.com/GitoxideLabs/gitoxide/commit/2106c64484c2162ee4e715efc592db14da602327))
    - Support for interruptible operations ([`a025593`](https://github.com/GitoxideLabs/gitoxide/commit/a02559378f9165df97a217f24834a851be719b08))
    - Thanks clippy ([`62d2ff3`](https://github.com/GitoxideLabs/gitoxide/commit/62d2ff383c5f7fe884057c70868569a811a73e00))
    - Organize object type comparisons by probability… ([`19a5d94`](https://github.com/GitoxideLabs/gitoxide/commit/19a5d9465f7962cfcc39ea31a2c84be6235e40ed))
    - Count object types as well ([`e04a8d1`](https://github.com/GitoxideLabs/gitoxide/commit/e04a8d16fda3712663d8d9220f3a017e668b6283))
    - Revert "Less memory for look up mode, faster start" - too slow ([`584350a`](https://github.com/GitoxideLabs/gitoxide/commit/584350af91f533db4cf980327d530445384c6b5a))
    - Less memory for look up mode, faster start ([`395c7e7`](https://github.com/GitoxideLabs/gitoxide/commit/395c7e78ef344ee56cf3d4ef49828942a09094bc))
    - Remove petgraph entirely ([`70ba33a`](https://github.com/GitoxideLabs/gitoxide/commit/70ba33a23a3ef887323ee29c248422f1997af6be))
    - Compute statistics for indexed pack verify ([`3d31c23`](https://github.com/GitoxideLabs/gitoxide/commit/3d31c235edaf7f88eb954cffc6864777566b3ef1))
    - Prepare for computing indexed statistics ([`082c246`](https://github.com/GitoxideLabs/gitoxide/commit/082c2467f2ab46aeb285504abcf2d8945dac4ce5))
    - Refactor ([`bfbae90`](https://github.com/GitoxideLabs/gitoxide/commit/bfbae905d3e8a0c5f30779c1723163a947de355e))
    - Keep all metadata per object needed to compute the usual statistics ([`961b85e`](https://github.com/GitoxideLabs/gitoxide/commit/961b85efec1ce84beacaa35720746752f687413a))
    - Make 'level' available to support statistics ([`f7ba51c`](https://github.com/GitoxideLabs/gitoxide/commit/f7ba51c93b04ef2e98f2436cf72e8c28b89b2448))
    - Refactor ([`6277318`](https://github.com/GitoxideLabs/gitoxide/commit/6277318f2ea71451b023a11fc9f74149d11fe9a9))
    - Support for error handling in traversal callbacks ([`c1d5bf6`](https://github.com/GitoxideLabs/gitoxide/commit/c1d5bf628db5f0c79aaf9af9740b990fc78aa4d5))
    - Indexed traversal now works, in theory, but needs error handling ([`86f8400`](https://github.com/GitoxideLabs/gitoxide/commit/86f8400a5e74e75fe7dab24911215a3f820b64b1))
    - Support for progress ([`62108fd`](https://github.com/GitoxideLabs/gitoxide/commit/62108fda164ae35903147eb1808c951bb90dac85))
    - Support for thread local storage in callbacks ([`1dad088`](https://github.com/GitoxideLabs/gitoxide/commit/1dad088a7d0be10a83cae0f119d42501887043e3))
    - Support for learning about the objects slice in the pack ([`faec782`](https://github.com/GitoxideLabs/gitoxide/commit/faec78276c4814edc7bbde150f8379fa73abc364))
    - And even more caapbilities are required to make tree traversal work natively ([`90523bb`](https://github.com/GitoxideLabs/gitoxide/commit/90523bb983e2cac70dad822531b7d66b7196cefc))
    - Refactor ([`2bbfd82`](https://github.com/GitoxideLabs/gitoxide/commit/2bbfd82f909ebc30cfb276bf40c7dbaa424a62f8))
    - Refactor ([`efa7cd8`](https://github.com/GitoxideLabs/gitoxide/commit/efa7cd843f7e93a8c4beba20597ff6d914bd6a33))
    - First steps towards actually using the new tree traversal during verification ([`785b0ff`](https://github.com/GitoxideLabs/gitoxide/commit/785b0ff02c0e00f6e7dea3a9c41a32f4129659e6))
    - Thanks clippy ([`44b20de`](https://github.com/GitoxideLabs/gitoxide/commit/44b20deafeac85151d57ecf4c0f5d889e9fe32f7))
    - Refactor ([`afe5e44`](https://github.com/GitoxideLabs/gitoxide/commit/afe5e445617c79b29d519257042b85d9533d40b0))
    - Refactor ([`fcc660d`](https://github.com/GitoxideLabs/gitoxide/commit/fcc660dee6f70b40364c70c73fc6b436929df4cd))
    - Reduce memory usage for index considerably ([`aa802be`](https://github.com/GitoxideLabs/gitoxide/commit/aa802be3402ad26a2907711cd5d1476b0caeec03))
    - And now it works! ([`f14e10e`](https://github.com/GitoxideLabs/gitoxide/commit/f14e10e9cfe1f4a4b477fcfc9459e49b439b0217))
    - Use new traversal in index writing, but it doesn't work yet ([`0dd5570`](https://github.com/GitoxideLabs/gitoxide/commit/0dd5570a1c615192f3c9382dfb7ffb1d817924db))
    - Refactor ([`4ff69c6`](https://github.com/GitoxideLabs/gitoxide/commit/4ff69c6281ba8d9af29a9f4407e9b2fa72f6550c))
    - Refactor ([`6cbb7cc`](https://github.com/GitoxideLabs/gitoxide/commit/6cbb7ccfc3e1fde4febfe652c25f5566937d3ad2))
    - Generalized tree traversal can theoretically work ([`64158e0`](https://github.com/GitoxideLabs/gitoxide/commit/64158e095f348ffa15139a9fa586074dad4d648b))
    - Make traversal part of the tree for greater ease of use ([`6629e30`](https://github.com/GitoxideLabs/gitoxide/commit/6629e3043786e5caf7d2b6fedc9350cd9e7bc6fb))
    - Prepare flexible traversal on decompressed objects ([`7707ea6`](https://github.com/GitoxideLabs/gitoxide/commit/7707ea6cf99d5ee93e4d6eea57adf00190d79d87))
    - Refactor ([`deea36c`](https://github.com/GitoxideLabs/gitoxide/commit/deea36c090fdd57ef8fc900744bbf17bd6e70097))
    - Refactor ([`83a0102`](https://github.com/GitoxideLabs/gitoxide/commit/83a01024d324123234776c8200ec3a3ae5f3c54e))
    - Refactor ([`b77d148`](https://github.com/GitoxideLabs/gitoxide/commit/b77d148ed1c5aec31cb0493b4f1e0f2d82d7e641))
    - Generalize tree iteration ([`fdc06de`](https://github.com/GitoxideLabs/gitoxide/commit/fdc06de2af8e7c9d2000177ce4f99ac68b5335be))
    - Also index using the new tree impl during verify (prepare replacement) ([`92039b0`](https://github.com/GitoxideLabs/gitoxide/commit/92039b038653cf97029e06f3f9b80892035d8c87))
    - Refactor ([`e3ff6af`](https://github.com/GitoxideLabs/gitoxide/commit/e3ff6af014cfbdbb53fe9498ff75b7f49fa5beb7))
    - Support for building a tree from offsets ([`95858bc`](https://github.com/GitoxideLabs/gitoxide/commit/95858bcbad01138240512731ec0e6dbdaed6c9fe))
    - Refactor ([`8cfe025`](https://github.com/GitoxideLabs/gitoxide/commit/8cfe0257de05b08a1278f78f6bdf3b5d65447686))
    - Refactor ([`bb9e518`](https://github.com/GitoxideLabs/gitoxide/commit/bb9e518b71ee1b4e1ab24d1369b879e047009294))
    - Count sorting into the progress, 7.5 mio entries takes a moment ([`2fc4cd8`](https://github.com/GitoxideLabs/gitoxide/commit/2fc4cd8dcac50f21491b5d297237acf97b2759fa))
    - Use bigger buffers when reading from disk. ([`e76e4eb`](https://github.com/GitoxideLabs/gitoxide/commit/e76e4ebb2261351bfe2af42b5782f0058f15edc6))
    - Only keep decompressed bytes of base objects… ([`b39ad89`](https://github.com/GitoxideLabs/gitoxide/commit/b39ad8976ee853229f87bbf962ada9557c7bbd32))
    - Remove memory mode entirely (and some complexity with it) ([`8812e91`](https://github.com/GitoxideLabs/gitoxide/commit/8812e916a21983868a37c4aade10f79a1dc9b926))
    - Turns out you never want to keep deltas in memory ([`657aa2c`](https://github.com/GitoxideLabs/gitoxide/commit/657aa2c38673cf10174f42bcb97039ac37b2926e))
    - Remove support for keeping compressed memory to reduce the index size ([`1e2ec7e`](https://github.com/GitoxideLabs/gitoxide/commit/1e2ec7e9d0ef2f2a4908860672080e411e945bff))
    - Don't cause re-allocs of the compression buffer ([`2bb6fd2`](https://github.com/GitoxideLabs/gitoxide/commit/2bb6fd26235825484a8f60a49455fee71f08236c))
    - Revert "FAIL: try to use a customized version of just pieces of Miniz-oxide" ([`ea0fdb3`](https://github.com/GitoxideLabs/gitoxide/commit/ea0fdb3c9ae42fcbd97f9319e90873c053d4ab71))
    - Try to use a customized version of just pieces of Miniz-oxide ([`9945eba`](https://github.com/GitoxideLabs/gitoxide/commit/9945eba749afb020e0deaaa5bb01fda6ff9ccd84))
    - Dependency upgrade + update ([`c6692c6`](https://github.com/GitoxideLabs/gitoxide/commit/c6692c6d494fe2bf1f9b924cf27da5908b74d62b))
    - Refactor ([`133e3ba`](https://github.com/GitoxideLabs/gitoxide/commit/133e3bafea772028f4bfd0fcc28a3e9bc3507701))
    - Let go of another handbreak - decompression is much faster now ([`ae9dc16`](https://github.com/GitoxideLabs/gitoxide/commit/ae9dc165b72893216e7337bf0726705adce69cd8))
    - Thanks clippy ([`393067d`](https://github.com/GitoxideLabs/gitoxide/commit/393067ddcf19424381ad2703c9c987d0f99587cd))
    - Use call to produce the resolver, allowing to delay opening a file mapping… ([`dd30e8d`](https://github.com/GitoxideLabs/gitoxide/commit/dd30e8d3c8b6754bd90e2777ec0153e158d4a708))
    - Fix actual memory violation (thanks to unsafe code) ([`c44c5e1`](https://github.com/GitoxideLabs/gitoxide/commit/c44c5e1890bc26ced920eb484e8708456d69df15))
    - Thanks clippy ([`1083a0b`](https://github.com/GitoxideLabs/gitoxide/commit/1083a0b75298454d19c2bdabaf0e195c78543792))
    - Reduce memory consumption ([`6d1a7a1`](https://github.com/GitoxideLabs/gitoxide/commit/6d1a7a1292e8065d0a777cb6acd34776b1e23696))
    - Unfortunately working with an optional for data is unwieldy, let's use default ([`12bbca0`](https://github.com/GitoxideLabs/gitoxide/commit/12bbca0b2dd780c3f6d4117a6bd0420fec0823bc))
    - Tree can now be used as sole data structure, collecting all results ([`3e52d6f`](https://github.com/GitoxideLabs/gitoxide/commit/3e52d6f89cb0ff0ab7e5a7fdb5aa892b498eef29))
    - Preparation for allowing reuse of the tree data structure ([`f565512`](https://github.com/GitoxideLabs/gitoxide/commit/f565512c6d37c0532d0d138dd1db0456903a0d2a))
    - Refactor ([`9c4bc0a`](https://github.com/GitoxideLabs/gitoxide/commit/9c4bc0a98bd024ca0a6e3d3f86f491dd92b880ac))
    - And it works! The new algorithm is sleeker, and really wants to be backported… ([`8e025b1`](https://github.com/GitoxideLabs/gitoxide/commit/8e025b1177db12e0e4f2387e44e58815e703a054))
    - Thanks, clippy… ([`079ce9c`](https://github.com/GitoxideLabs/gitoxide/commit/079ce9c07409ceb9acfc0eae900e73a4ae51fc58))
    - Basis for re-implementing core algorithm using new Tree data structure ([`be6caf4`](https://github.com/GitoxideLabs/gitoxide/commit/be6caf4caf73fb61f23a4ea42617c3ca61b44569))
    - Refactor ([`290c29a`](https://github.com/GitoxideLabs/gitoxide/commit/290c29ade648c7bb850c2e0629f8cc10967758fb))
    - Incorporate proper filtering of bases ([`0880998`](https://github.com/GitoxideLabs/gitoxide/commit/08809986ac50081d91a9dbe8fd28c3452bf54e69))
    - Overhauled iterator logic, still missing 'is_root' filter ([`2bfbae1`](https://github.com/GitoxideLabs/gitoxide/commit/2bfbae145e7d2256d41ed0a69e03d1e002534a49))
    - First impl of the Iterator shows it's 'unknown' what a root node is ([`3f32938`](https://github.com/GitoxideLabs/gitoxide/commit/3f329380f6d13ab6ab991a5bb82e4cb38b37a52f))
    - Sketch on how children access could look like ([`16a35df`](https://github.com/GitoxideLabs/gitoxide/commit/16a35dfcee905a672b2c1a0741320a51b3cf67d7))
    - How a referenced version would look like… ([`e36021d`](https://github.com/GitoxideLabs/gitoxide/commit/e36021df6b6b872be1249dbddd96a2d678c3bcc3))
    - Refactor ([`62a01fe`](https://github.com/GitoxideLabs/gitoxide/commit/62a01fee56b45ef83b4e3efb018af8ebb1db22ac))
    - More experimentation towards a safe tree data structure… ([`d907ce8`](https://github.com/GitoxideLabs/gitoxide/commit/d907ce8f34ff488bc6a70f17d3c99df82b7ef41b))
    - First stab at new Tree datastructure… ([`85d7579`](https://github.com/GitoxideLabs/gitoxide/commit/85d7579bf9c2f82f941b983ea4d54e16c6661c9b))
    - Safety for handling base pack offsets ([`17d8375`](https://github.com/GitoxideLabs/gitoxide/commit/17d837514ad0f28771d67a64f74a30ef460fc3d1))
    - …but there seem to be issues with the kernel pack… ([`cc147bc`](https://github.com/GitoxideLabs/gitoxide/commit/cc147bc60066c4ef31353a499958edadc960a9c4))
    - Quick and dirty impl of gitoxide layer for bundle writing, aka index-pack ([`e78386b`](https://github.com/GitoxideLabs/gitoxide/commit/e78386b824010c5ca8efca87604c339d40b545ae))
    - Cargo clippy ([`586ba7a`](https://github.com/GitoxideLabs/gitoxide/commit/586ba7af016f9a510b4ffeecc1aff6de0a569627))
    - Implement in-memory mode; refactor ([`0c195b9`](https://github.com/GitoxideLabs/gitoxide/commit/0c195b92b59892c9f5369e28acd8f99d25f42c0c))
    - Refactor ([`c9d9298`](https://github.com/GitoxideLabs/gitoxide/commit/c9d92980fd15e8e3568c82243d5eedb5e6e13f10))
    - Use monomorphic calls only at the expense of code siz ([`40b28d1`](https://github.com/GitoxideLabs/gitoxide/commit/40b28d18736b09bf3af1a70e9854e98e94bd09fc))
    - Refactor ([`150d0bc`](https://github.com/GitoxideLabs/gitoxide/commit/150d0bcc3ab39061be1add3f98da299e95edbbd5))
    - Also implement the 'no output directory' branch ([`5a3240f`](https://github.com/GitoxideLabs/gitoxide/commit/5a3240fae2211924ac2eb03c9f57d2234de4f26f))
    - Refactor ([`68e52f8`](https://github.com/GitoxideLabs/gitoxide/commit/68e52f8ce144f2daf2db407e66b3684a7d96d58d))
    - For the first time, writing an index could work with persistence ([`16e045c`](https://github.com/GitoxideLabs/gitoxide/commit/16e045c3cd0f6e003a6b6e547360acdf99a06585))
    - Don't write pack to file if everything is kept in memory ([`f3ddda6`](https://github.com/GitoxideLabs/gitoxide/commit/f3ddda6434824845e9abffab1d851c067428d8c7))
    - Allow data file to be optional in preparation for in-memory operation ([`95af105`](https://github.com/GitoxideLabs/gitoxide/commit/95af105298e1073b71e3edcbbe3c9f3179ecf78e))
    - Refactor ([`413968d`](https://github.com/GitoxideLabs/gitoxide/commit/413968dfee5e5a66ed9e63823f6bda5a5a22753e))
    - Refactor ([`5d27cdb`](https://github.com/GitoxideLabs/gitoxide/commit/5d27cdb98b85baa5c544bc326ad50d1d7664116a))
    - Optional pack lookup depending on the settings ([`2b509de`](https://github.com/GitoxideLabs/gitoxide/commit/2b509deefe7e09e59bd69937044337c8ac327f5f))
    - Write-through the pack file as we receive it and move it into place ([`6180e39`](https://github.com/GitoxideLabs/gitoxide/commit/6180e3995426e99400364b04af36e0265ad779aa))
    - Receive progress information when reading packs in bundle ([`759091d`](https://github.com/GitoxideLabs/gitoxide/commit/759091d3c6696b427d7b5aab1b6da05a0d268c04))
    - Start supporting writing packs to disk right away ([`f2203e0`](https://github.com/GitoxideLabs/gitoxide/commit/f2203e0ebefaf254008f4ad4628218c42f1a2208))
    - Refactor ([`75c333c`](https://github.com/GitoxideLabs/gitoxide/commit/75c333c121c402201ed4abf82ea7f14481d3f55b))
    - Prepare for implementing the bundle with various write modes ([`de420e4`](https://github.com/GitoxideLabs/gitoxide/commit/de420e4515c6e4953a6e8cf6c632e3561873caca))
    - Bundle thread progress underneath reducer progress ([`76b1b2b`](https://github.com/GitoxideLabs/gitoxide/commit/76b1b2b3015183129638b1f122a54fb8df8a1ac7))
    - Prevent deadlock, interestingly ([`ca02901`](https://github.com/GitoxideLabs/gitoxide/commit/ca02901ad0eff63c3d9105a385c0ada6179ae71a))
    - Refactor ([`ea254c0`](https://github.com/GitoxideLabs/gitoxide/commit/ea254c095465c880383d47a5284994a5a68a8769))
    - Rough progress for writing the index ([`f1a7f9b`](https://github.com/GitoxideLabs/gitoxide/commit/f1a7f9b9ec71f2ae2de2c9bbe57f5118c76fa3dd))
    - Initial batch of progress usage for index creation… ([`b10e5c6`](https://github.com/GitoxideLabs/gitoxide/commit/b10e5c664be9bd1bdb2b72b858ebaf35c1ed4cb4))
    - Refactor ([`77b3c21`](https://github.com/GitoxideLabs/gitoxide/commit/77b3c213922c2f264722fc2423dbc22d0988c507))
    - Refactor ([`fb23d15`](https://github.com/GitoxideLabs/gitoxide/commit/fb23d156c276484038761394a054a96d6f9ed087))
    - Refactor ([`7da7e08`](https://github.com/GitoxideLabs/gitoxide/commit/7da7e080241c36b3a743e9bc01b61db5758246e5))
    - Refactor ([`5a3ad3a`](https://github.com/GitoxideLabs/gitoxide/commit/5a3ad3a59da297c56ea47450b2c90dd24f542d40))
    - Refactor ([`785a23d`](https://github.com/GitoxideLabs/gitoxide/commit/785a23d9b0ef3529ca4f655ed122a5e0c783b945))
    - Header encoding works now! As well as index writing :)! ([`024b854`](https://github.com/GitoxideLabs/gitoxide/commit/024b854b07720f219fe12eefa94a166820523c9c))
    - Initial version of a complete header encoding impl, but… ([`ce6b46b`](https://github.com/GitoxideLabs/gitoxide/commit/ce6b46b1bdcdf5ff5047d3288dc6fddb5bf62f77))
    - Looks like CRCs are not correct ([`3c4e4a0`](https://github.com/GitoxideLabs/gitoxide/commit/3c4e4a0a61fe552913ec72c569d9a2095646b69a))
    - Cargo clippy ([`a5596fb`](https://github.com/GitoxideLabs/gitoxide/commit/a5596fb71fd268b6faaa3b19c8b78d3608070012))
    - Fanout writing works now… ([`93a7ba9`](https://github.com/GitoxideLabs/gitoxide/commit/93a7ba913fa29f734b98fe5723d01e2a7593ae2c))
    - It's a good idea to remove old code from time to time… ([`9e47f1b`](https://github.com/GitoxideLabs/gitoxide/commit/9e47f1b04a5bbd4c0f13da5d55ae6302ae941d35))
    - Fanout table, but slowly I get it :D ([`cfd8a25`](https://github.com/GitoxideLabs/gitoxide/commit/cfd8a25f9125c48afe4b66eab6b6ecf71097c486))
    - Fix decompression; fanout table is still wrong though ([`77fac1a`](https://github.com/GitoxideLabs/gitoxide/commit/77fac1a01d8c15f9f772c3e14a430a890ff50899))
    - Despite writing the CRC32 now, it doesn't work yet ([`ecd12b9`](https://github.com/GitoxideLabs/gitoxide/commit/ecd12b90aadd6bf6cdf551802918823670a45466))
    - First stab at streaming pack header encoding ([`3c6e78b`](https://github.com/GitoxideLabs/gitoxide/commit/3c6e78bec9cbd4df842919cc8dc3c575414ed002))
    - Refactor ([`5925d46`](https://github.com/GitoxideLabs/gitoxide/commit/5925d4615216dea70b5bc737b70f898e81e540e2))
    - Simplify offset handling in favor of allocating less ([`ce4ec62`](https://github.com/GitoxideLabs/gitoxide/commit/ce4ec62e66a7fd9ff720633f531156ed51d610fe))
    - Only allocate memory for offsets if needed ([`72e0642`](https://github.com/GitoxideLabs/gitoxide/commit/72e06421ae386dd15b34ce6dcf5e1cf666e70c3a))
    - First complete implementation of index writing… ([`826f996`](https://github.com/GitoxideLabs/gitoxide/commit/826f996b9a9d877b84d286e18f5501eaec73d6f1))
    - Reduce contention by using the shared cache only once ([`c370e13`](https://github.com/GitoxideLabs/gitoxide/commit/c370e133f4626a59eadbc8b70b4b5df39a34ad71))
    - Optimize CRC handling - no need to assign it after the fact ([`ffcc03d`](https://github.com/GitoxideLabs/gitoxide/commit/ffcc03de5768c26c25dafbfbe523ca3bd4422336))
    - Assure we can deltas store theyr resolved buffer ([`d2a81d9`](https://github.com/GitoxideLabs/gitoxide/commit/d2a81d912cdd9ec22ed8351b2a8395d85de46aa5))
    - And it does seem to work! Awesome! ([`71cd982`](https://github.com/GitoxideLabs/gitoxide/commit/71cd9824bece6215745b02d9df001ae202fe2597))
    - Delta-application could work if we handle our buffer better ([`ac6100b`](https://github.com/GitoxideLabs/gitoxide/commit/ac6100b094842f0a472be9789c024fc45939ff06))
    - Refactor ([`400a2a9`](https://github.com/GitoxideLabs/gitoxide/commit/400a2a91edd72394de0aba55628154c16bca98bc))
    - One step before applying deltas ([`a074193`](https://github.com/GitoxideLabs/gitoxide/commit/a07419303b3b9a24acb580a8653da952a5fa9964))
    - Prepare for delta application ([`9a9fb7a`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fb7a53fbda1b77d013f9806bd383a06135741))
    - Cargo clippy ([`d69c973`](https://github.com/GitoxideLabs/gitoxide/commit/d69c973626fc554d34326b7ba37243b5389d2193))
    - Parse pack header before trying to decompress :D ([`9d1b44a`](https://github.com/GitoxideLabs/gitoxide/commit/9d1b44ad98bb4cac55749ce25af5e444bc14d4ab))
    - Refactor ([`772e9ce`](https://github.com/GitoxideLabs/gitoxide/commit/772e9cef82b1d58a1d7c9ad23dda570ec97bcc0b))
    - Consumer can resolve entries ([`13adce6`](https://github.com/GitoxideLabs/gitoxide/commit/13adce6e18a4efb9da30dfc86c22a74dbc9026aa))
    - Refactor ([`c87f770`](https://github.com/GitoxideLabs/gitoxide/commit/c87f77036eb8f8b095997afcf5200b165d9ddf2f))
    - Refactor ([`d9d406d`](https://github.com/GitoxideLabs/gitoxide/commit/d9d406d77531bdfe5b33ee8ed17bccd431e85f9b))
    - First version of resolver to copy from a memory map ([`506b8fd`](https://github.com/GitoxideLabs/gitoxide/commit/506b8fd94478ab259d18f4226c4b25bd080f775d))
    - Rethink resolver into something even simpler ([`4388c6c`](https://github.com/GitoxideLabs/gitoxide/commit/4388c6c1ccbfbee7d5abb064eab3569a1aebf6a0))
    - Use parking_lot where possible ([`367874e`](https://github.com/GitoxideLabs/gitoxide/commit/367874e91a2ca79d17c90d8ebaace1ee23efb4d9))
    - Consumers can fail gracefully ([`9082080`](https://github.com/GitoxideLabs/gitoxide/commit/9082080a43e4db43378abc5555ad6f8084fdc111))
    - Refactor ([`1b4cad0`](https://github.com/GitoxideLabs/gitoxide/commit/1b4cad01c6fb99e06db51415557c555ffb06b9f7))
    - Refactor ([`4ce13bb`](https://github.com/GitoxideLabs/gitoxide/commit/4ce13bbe65403a2a9a320fb439ae797b19921862))
    - Support for decompression in case compressed bytes are stored ([`c1fcf28`](https://github.com/GitoxideLabs/gitoxide/commit/c1fcf28f1069b605191652a2bd1556445e3b9833))
    - Computing hashes for bases from decompressed in-memory store works ([`7c19fe6`](https://github.com/GitoxideLabs/gitoxide/commit/7c19fe6aec0cdb425d77cf13349e2f7f687c63e3))
    - Show that all data can be passed for processing in threads ([`a95ce9c`](https://github.com/GitoxideLabs/gitoxide/commit/a95ce9c83920f29689ded1e374a224bef2d2b7cb))
    - A cache usable from threads ([`1d4879a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4879aee75a2c2ccbefdd48a2c2d339db38a23b))
    - Re-associate CRC32 with the correctly sorted ID output ([`037e1e5`](https://github.com/GitoxideLabs/gitoxide/commit/037e1e5a92c430689674e2cb7e96f9738a92fde5))
    - Refactor ([`b3a365d`](https://github.com/GitoxideLabs/gitoxide/commit/b3a365d179301f315d24884717c2dc09e34c3087))
    - Refactor ([`97eb524`](https://github.com/GitoxideLabs/gitoxide/commit/97eb524ffa4cbf04113d3a622aca3a76606f0d96))
    - Use chunked input and calculate 'optimal' chunk and thread sizes ([`0cc74d7`](https://github.com/GitoxideLabs/gitoxide/commit/0cc74d7982577866c6fa6d7b0f56073979142bf0))
    - Generalize chunk iterator ([`905e85e`](https://github.com/GitoxideLabs/gitoxide/commit/905e85e0910650b139a845c7e7bae97a7ae5b215))
    - First rough cut of in_parallel invocation ([`8f16081`](https://github.com/GitoxideLabs/gitoxide/commit/8f160810f6baf0fca5590001dd89895fccae0bbe))
    - Prepare for parallelization ([`cb36596`](https://github.com/GitoxideLabs/gitoxide/commit/cb36596d3059700deaf87a26df344f5dbb87f1f4))
    - Simplify indexing step ([`070899c`](https://github.com/GitoxideLabs/gitoxide/commit/070899cd8cb86ac3761255ccba72225ffd6c518e))
    - Resolver look ups may now turn out empty… ([`a991923`](https://github.com/GitoxideLabs/gitoxide/commit/a9919230896b9129fe91b5e12dc6e0f03547b5e9))
    - Allow us to stop searching for bases early when resolving ([`e7874da`](https://github.com/GitoxideLabs/gitoxide/commit/e7874dad3982829d82d3e708926e2965eca3ef4e))
    - This should be the interface for building indices from packs directly ([`f5295d0`](https://github.com/GitoxideLabs/gitoxide/commit/f5295d09592753089569543b843a352fd91df201))
    - Got a good idea on how this will work! ([`7bb229f`](https://github.com/GitoxideLabs/gitoxide/commit/7bb229fbb17fb8cc8251c49b681511519a9a6b9c))
    - Keep track of the pack trailer ([`cdba61e`](https://github.com/GitoxideLabs/gitoxide/commit/cdba61ea90a5e4f4e64ca2fe7777da540dbbf09c))
    - Now I understand why there is a separate resolution phase… ([`1c2bcbd`](https://github.com/GitoxideLabs/gitoxide/commit/1c2bcbd3f510dddfc43e3f02a7987890306d8db7))
    - Fix tests ([`b9866b6`](https://github.com/GitoxideLabs/gitoxide/commit/b9866b683687e210279b88b5409f01d52659f550))
    - Prepare a way to gradually implement V2 index writing ([`92a4986`](https://github.com/GitoxideLabs/gitoxide/commit/92a4986fcec21870abfbe8a7886fa428d5d47941))
    - Refactor ([`feba75b`](https://github.com/GitoxideLabs/gitoxide/commit/feba75b9f04459abc341bd2482a393a69602b054))
    - We can now restore (possibly half-written) packs ([`b1daa46`](https://github.com/GitoxideLabs/gitoxide/commit/b1daa465c40ea8c7c9de69a18e467d69459d911e))
    - Prepare ability to restore pack files ([`76583e5`](https://github.com/GitoxideLabs/gitoxide/commit/76583e58ad8a4a4269fb857364b213ae12d4ea9b))
    - Support for pack trailer verification when iterating ([`f37f131`](https://github.com/GitoxideLabs/gitoxide/commit/f37f131cf6904780147371746ff5bf56dbc21356))
    - Also read the pack trailer during iteration ([`98a8e17`](https://github.com/GitoxideLabs/gitoxide/commit/98a8e17e791b6bcd92149d7ff68cbc9d9ceee087))
    - Only take as many objects as we are allowed (without 'take(…)') ([`86f5853`](https://github.com/GitoxideLabs/gitoxide/commit/86f585344f968ba86a19b58129fe3bd2a058730c))
    - Refactor ([`e15bde4`](https://github.com/GitoxideLabs/gitoxide/commit/e15bde409cd1eae30a3a4b45624f52025144a10a))
    - Shift thin pack resolution to another work bucket; test for index writing ([`2592361`](https://github.com/GitoxideLabs/gitoxide/commit/25923611663a244908198c4dc656ac73cc16c841))
    - Refactor; better tests ([`12d14bf`](https://github.com/GitoxideLabs/gitoxide/commit/12d14bfe2aa089723a395287c5100aad6e838935))
    - Refactor ([`bd66a85`](https://github.com/GitoxideLabs/gitoxide/commit/bd66a8592d3d2f5c6a7393c261f19023d14d2f37))
    - Now keeping track of read bytes works ([`d32d921`](https://github.com/GitoxideLabs/gitoxide/commit/d32d9210133ab339cece3b8811958eadb8428587))
    - An attempt to intercept bytes read from bufread - FAIL ([`8db04f6`](https://github.com/GitoxideLabs/gitoxide/commit/8db04f66fe4a4c5d0dba1c2a0c82723b4487f5bf))
    - Refactor ([`2d817d7`](https://github.com/GitoxideLabs/gitoxide/commit/2d817d7b3fcb939067b7b94fa7aeac20382effc8))
    - Refactor ([`893f65b`](https://github.com/GitoxideLabs/gitoxide/commit/893f65b63b424922b8cdc496a9e798acc498c1c6))
    - Refactor ([`12816bc`](https://github.com/GitoxideLabs/gitoxide/commit/12816bc715a0d0bad338a00c394c4cc503b20c3e))
    - Refactor ([`56f763a`](https://github.com/GitoxideLabs/gitoxide/commit/56f763a44538e053b4f674543720720fcc1af5d4))
    - Associate HashKind with the kind of pack ([`d66d139`](https://github.com/GitoxideLabs/gitoxide/commit/d66d1391a3edee0572e07cb421527a57d90de9d9))
    - Move all pack-related file handling to bundle; big refactor ([`f8b6e75`](https://github.com/GitoxideLabs/gitoxide/commit/f8b6e7524b6d73406dc6ff7b8e9c7e22322efd78))
    - First step towards putting the index file into position ([`d994c74`](https://github.com/GitoxideLabs/gitoxide/commit/d994c74d7cd9c9c004bf27f0b2ac23558ce9c50d))
    - Initial interface trial for writing pack index files ([`936bdcc`](https://github.com/GitoxideLabs/gitoxide/commit/936bdcc29e5531026c1b0e83d9084501fc6ded9c))
    - Refactor; more thorough tests ([`82d87ce`](https://github.com/GitoxideLabs/gitoxide/commit/82d87ce35b1e68a07057807d28afffa7acc03b7f))
    - Cargo clippy ([`b768b56`](https://github.com/GitoxideLabs/gitoxide/commit/b768b56db4274b7cc313e8a6c09f3c46a48a2829))
    - At least make it configurable if to keep decompressed bytes or not ([`28ebcae`](https://github.com/GitoxideLabs/gitoxide/commit/28ebcae69e95c768e4d9567ec6cc8adacd8d520b))
    - And streaming iteration works, even though we are forced to allocate… ([`27d624d`](https://github.com/GitoxideLabs/gitoxide/commit/27d624d920a0ea92cf506363a505517676ced770))
    - Yes, this really cannot work: StreamingIterator ([`b4df430`](https://github.com/GitoxideLabs/gitoxide/commit/b4df430b96561c63d20bb5de442582eca79768f1))
    - In the moment we tried to actually return Entry<'a>, it didn't let me :D ([`8367955`](https://github.com/GitoxideLabs/gitoxide/commit/836795514f19a9d43039be228c5183061db4a404))
    - First steps towards making the InflateReader reusable ([`83a97d4`](https://github.com/GitoxideLabs/gitoxide/commit/83a97d462e16d6e28151c2bf6eb7b201f4982dce))
    - Better error handling in iterator, fuse yourself ([`5ebacc4`](https://github.com/GitoxideLabs/gitoxide/commit/5ebacc491a5148d31bf5ebe2746ea3d5c562b407))
    - The next() impl shows that we should be less lenient ([`4521cab`](https://github.com/GitoxideLabs/gitoxide/commit/4521cab497757c34501b8eefd3b2d7d36b4df32b))
    - Provide entries which borrow from iterator ([`86eea13`](https://github.com/GitoxideLabs/gitoxide/commit/86eea1326a48cf55c8a17505d2cf7c44a110a878))
    - Provide a lifetime for iterator (and possibly its entries) ([`7852bd1`](https://github.com/GitoxideLabs/gitoxide/commit/7852bd193ad5659f07fc8759ca3597b037ad0255))
    - First version of expected iterated data types ([`d5e7d31`](https://github.com/GitoxideLabs/gitoxide/commit/d5e7d311f38ffff0a31f85feaab692f078a75bb5))
    - Improved iterator constructors ([`fb71f04`](https://github.com/GitoxideLabs/gitoxide/commit/fb71f0463519c886d2e5ab30a32d546e70fb0606))
    - Better handling of pack headers ([`0030bdb`](https://github.com/GitoxideLabs/gitoxide/commit/0030bdbe3d476f6dac9c98f273d72666e2a9b7eb))
    - Frame for a pack iterator ([`07d1096`](https://github.com/GitoxideLabs/gitoxide/commit/07d109652a6ccb93d166296cd1f91babbd1ae0aa))
    - Some more tests ([`9095728`](https://github.com/GitoxideLabs/gitoxide/commit/9095728dff0f5ae221dcf3345e81cfb54300e03d))
    - Verification for pack objects ([`17bd95e`](https://github.com/GitoxideLabs/gitoxide/commit/17bd95ec43ca2814165823026fd85a776208fe21))
    - Refactor ([`3ee947e`](https://github.com/GitoxideLabs/gitoxide/commit/3ee947e241404cdac3225824b1434c2b270236da))
    - 'stream()' now assures all data is decompressed ([`32e994c`](https://github.com/GitoxideLabs/gitoxide/commit/32e994c60f58f1be839c6dc07d819ac31f30af1d))
    - It looks like something is wrong with the object stream implementation ([`d187b5a`](https://github.com/GitoxideLabs/gitoxide/commit/d187b5a769b62ec706c1265e0db8403327d8e92d))
    - Loose object verifycation - but it doesn't seem to work as expected ([`9dd5676`](https://github.com/GitoxideLabs/gitoxide/commit/9dd56761ae75eac691449cd86a1be04c11c0fecb))
    - Refactor ([`37cfd9b`](https://github.com/GitoxideLabs/gitoxide/commit/37cfd9ba14726d6fd38b5ba6eabb3b17be263779))
    - Refactor ([`8e3b9fc`](https://github.com/GitoxideLabs/gitoxide/commit/8e3b9fc23a139c8307e052afa8d1d6f6f562ca1d))
    - Prepare full 'verify' implementation ([`ee45c7f`](https://github.com/GitoxideLabs/gitoxide/commit/ee45c7f47b95fc406cc5922a322c8fd6c0f52775))
    - Refactor ([`0a33b24`](https://github.com/GitoxideLabs/gitoxide/commit/0a33b24f5b61ccdf1358f1e9adcf0f6fd4099c1c))
    - Always compress values when using a sink when exploding packs ([`70562fa`](https://github.com/GitoxideLabs/gitoxide/commit/70562fa123faf51bd72a4aedb12acb0d3247e4e2))
    - Support for compression even when using sink ([`105c845`](https://github.com/GitoxideLabs/gitoxide/commit/105c84551361bd93ec549a07ab377a7f1ae97332))
    - Another stab at fixing stress tests :) ([`7db6a33`](https://github.com/GitoxideLabs/gitoxide/commit/7db6a33bc8bdaccf9091acc2ca48eb26f8a8c1fa))
    - Fix stress test; improve progress messages ([`37ccd92`](https://github.com/GitoxideLabs/gitoxide/commit/37ccd92bbc4eb9917c1916e39f626ecddbf85064))
    - Ignore decode errors (if configured) at the right spot ([`e53141d`](https://github.com/GitoxideLabs/gitoxide/commit/e53141dd5e319d29de15ab73a783ce21158ed54a))
    - Tests for relaxed error handling ([`93c0e26`](https://github.com/GitoxideLabs/gitoxide/commit/93c0e2664ccc259747543845186c4211ae139008))
    - Nice error message on failure ([`adbc82c`](https://github.com/GitoxideLabs/gitoxide/commit/adbc82c31450681fcb38233eeb8095efc5e52a18))
    - Inform about deleted files using progress ([`a3ee516`](https://github.com/GitoxideLabs/gitoxide/commit/a3ee5160093c9326006fcedbf1f507d8978a97c2))
    - Fix error display - certainly something to watch out for ([`38eff2c`](https://github.com/GitoxideLabs/gitoxide/commit/38eff2c3f0bb6170a253b4c96f01077c1358bc40))
    - The first 'explode' implementation… ([`0d31ad1`](https://github.com/GitoxideLabs/gitoxide/commit/0d31ad1b61997fa0d0692c5919fb8032ffaaa35b))
    - Support for skipping various safety checks during traversal ([`0416666`](https://github.com/GitoxideLabs/gitoxide/commit/0416666d3492ddd031188f750371248f5f67d598))
    - Prepare for configuration of safety checks ([`06638d0`](https://github.com/GitoxideLabs/gitoxide/commit/06638d0f9ce50782e2897d76c742c526758889d1))
    - Cargo clippy ([`95e02c9`](https://github.com/GitoxideLabs/gitoxide/commit/95e02c951ace19f6ace49a9190607674d98c970d))
    - Restore original verification functionality ([`0e3c1b9`](https://github.com/GitoxideLabs/gitoxide/commit/0e3c1b9bb9841ae4bb0ef1df2e72e950f7a7fd33))
    - Nearly there! Interesting that anyhow errors must be sync! ([`eaee77e`](https://github.com/GitoxideLabs/gitoxide/commit/eaee77ea4ce10f5c85b42a33452eef996adac3bf))
    - Finally it compiles with returning Boxed errors! Ouch… ([`1fc8252`](https://github.com/GitoxideLabs/gitoxide/commit/1fc8252a24b75faa88065838a3e9ffa13e6f7f54))
    - First sketch of new verify expressed in terms of traversal ([`4cb570f`](https://github.com/GitoxideLabs/gitoxide/commit/4cb570f96ddd7ee2faa62e54927afd78ba7822af))
    - Refactor ([`f2832a8`](https://github.com/GitoxideLabs/gitoxide/commit/f2832a840d0bc69e7ee0817e3617ac0b3d40e4fd))
    - Finally a progress can be passed to the delegate… ([`a9f4de0`](https://github.com/GitoxideLabs/gitoxide/commit/a9f4de0783a87b0693f87da98283e30ec72f3737))
    - Refactor ([`bbb3e1e`](https://github.com/GitoxideLabs/gitoxide/commit/bbb3e1efd309bbcdb3adda84308a3fc644389e43))
    - Pass all arguments (but progress) to processor ([`1e87922`](https://github.com/GitoxideLabs/gitoxide/commit/1e87922299762dc0b2cf0800e1ff1e0a61467ce5))
    - Call a bare version of the traversal processor ([`95a5cea`](https://github.com/GitoxideLabs/gitoxide/commit/95a5cead30fa4e7904b28158a747ac28adadf01e))
    - Preparation for abstracting the 'process object (stateful)' function ([`fe400f5`](https://github.com/GitoxideLabs/gitoxide/commit/fe400f572accb396def704f7853d5e81a42839de))
    - Discard idea of making traversal even more generic ([`1525f36`](https://github.com/GitoxideLabs/gitoxide/commit/1525f36d29574699d2fcb16b70678121030fd109))
    - Initial step towards separating verification from traversal ([`d14b4fc`](https://github.com/GitoxideLabs/gitoxide/commit/d14b4fc7fd09bf1a96b16d583c1a8df102517650))
    - Refactor ([`bae7781`](https://github.com/GitoxideLabs/gitoxide/commit/bae7781ab549f0daa73980a29d18d64320601470))
    - Rename verify-pack to pack-verify (keeping it more formal) ([`ec8c48a`](https://github.com/GitoxideLabs/gitoxide/commit/ec8c48a8fcbcd748c9c764734d881b5f0217e1e4))
    - Refactor ([`f580441`](https://github.com/GitoxideLabs/gitoxide/commit/f5804410eb80fa406294fb83e161b09a4f3bf1a2))
    - Fast implementation for buffered input ([`c50b150`](https://github.com/GitoxideLabs/gitoxide/commit/c50b150adc7c5379017237c0914c294aad1fdc7c))
    - Respect object size to be 64 bit where applicable… ([`61c8aba`](https://github.com/GitoxideLabs/gitoxide/commit/61c8aba769a52d11de549505f6b4cbca1d949758))
    - Better errors for writing disk objects ([`f7bc137`](https://github.com/GitoxideLabs/gitoxide/commit/f7bc1372d6b445f5c078632c4f3ad7786f98e6a9))
    - Try to use HashKind where possible ([`b32e01d`](https://github.com/GitoxideLabs/gitoxide/commit/b32e01dbfd257d123a461380df5dcfcb88c77e1e))
    - Refactor ([`a3777ed`](https://github.com/GitoxideLabs/gitoxide/commit/a3777edb2612b50de7a12da4ecbf707638d23ac3))
    - Clippy happy ([`a938c70`](https://github.com/GitoxideLabs/gitoxide/commit/a938c7002b4c4905694d97dc682dd77cd6780cff))
    - And writing of loose objects works ([`bbfe7bf`](https://github.com/GitoxideLabs/gitoxide/commit/bbfe7bf2be3ab04dd27b9a23381ced9838fc292e))
    - This seems to be a working deflate write implementation ([`0acce38`](https://github.com/GitoxideLabs/gitoxide/commit/0acce381912059c06df55955e45245c5eeb6d4b3))
    - The first succesful inflate run for small input ([`94e1c5a`](https://github.com/GitoxideLabs/gitoxide/commit/94e1c5a69d22ee56d697e6e59a6a367ceb5e0c6f))
    - What seems to be a reasonable write implementation for deflate ([`45a28d2`](https://github.com/GitoxideLabs/gitoxide/commit/45a28d259c2f08b8c3c96b0bf0092261d0bd17a3))
    - Another test to understand the deflate streamer better ([`4256038`](https://github.com/GitoxideLabs/gitoxide/commit/4256038e65bc68222c60c1b25273a3d066991970))
    - Refactor ([`dd463df`](https://github.com/GitoxideLabs/gitoxide/commit/dd463df3b4a48d117596d78e050bf425db350b27))
    - Refactor ([`0b42237`](https://github.com/GitoxideLabs/gitoxide/commit/0b42237ead90a1582b5ddb936d30fbf75da8b6b1))
    - Refactor ([`5b0bb84`](https://github.com/GitoxideLabs/gitoxide/commit/5b0bb841bbc5f2e267238e4cdec69029a8344a31))
    - Put down a few tests to understand how deflate wants to be fed ([`178a018`](https://github.com/GitoxideLabs/gitoxide/commit/178a01814b344c7b2ae7a7470c33808dca7e3a38))
    - Refactor ([`0d8d7fe`](https://github.com/GitoxideLabs/gitoxide/commit/0d8d7fee0fde7f24c91fb147745ed8474f40e834))
    - Improve looks of documentation ([`11a32eb`](https://github.com/GitoxideLabs/gitoxide/commit/11a32ebc2209d1a05eb4c4ec5131e85dfb43d9f6))
    - Fix tests for now… ([`79ab945`](https://github.com/GitoxideLabs/gitoxide/commit/79ab9453264562488e8c5bc6ead7dd1c1fe46cba))
    - Refactor ([`0cd7bb7`](https://github.com/GitoxideLabs/gitoxide/commit/0cd7bb74e379483116afb1ab618081ef1bfef67a))
    - Complete and unoptimized disk writer for objects, but… ([`9d0c3f1`](https://github.com/GitoxideLabs/gitoxide/commit/9d0c3f16413d437fa893524c1cdf4a899fc3c921))
    - Refactor ([`62e75bc`](https://github.com/GitoxideLabs/gitoxide/commit/62e75bca7de17782dc5b7cbae29c8ce8e63b8d02))
    - Make use of HashKind in Write trait ([`0304dd0`](https://github.com/GitoxideLabs/gitoxide/commit/0304dd0f44cd55af07796c3aacca0f116ffd181b))
    - Make our Sink API similar to std::io::sink() ([`a03ae0f`](https://github.com/GitoxideLabs/gitoxide/commit/a03ae0f064cbf63bc4cb352ccec25333ec1843e6))
    - Finish Sink implementation ([`84f7908`](https://github.com/GitoxideLabs/gitoxide/commit/84f7908b1883ed6c484ca4e522ac530c8cc161d5))
    - First steps towards serialization tests for sink ([`e8d52c6`](https://github.com/GitoxideLabs/gitoxide/commit/e8d52c6997997688220959b096d46aaa641d14a1))
    - Introduce hash kind, as this should be specified when writing an object ([`f5d0acf`](https://github.com/GitoxideLabs/gitoxide/commit/f5d0acf61ac5dd815bc5ece4462eb9a43dd9c44a))
    - A simple trait for writing owned objects and streams ([`68b7d7d`](https://github.com/GitoxideLabs/gitoxide/commit/68b7d7defdb07b3a100bc16a9167ee957647f5cb))
    - (cargo-release) version 0.2.0 ([`76fe0ab`](https://github.com/GitoxideLabs/gitoxide/commit/76fe0ab5f0b58504a5ea5adb74b349b9d588e51e))
    - (cargo-release) version 0.2.0 ([`0bb8314`](https://github.com/GitoxideLabs/gitoxide/commit/0bb831480d8657e1bb29ee7009aeac673471403e))
    - Use 'optimized' chunk size for 'less-time' algorithm ([`c8c23c0`](https://github.com/GitoxideLabs/gitoxide/commit/c8c23c0fb9ab0174dd33299ddd3f257f7b2dde78))
    - Incorporate dynamic chunking into 'less-time' algorithm ([`295aa2f`](https://github.com/GitoxideLabs/gitoxide/commit/295aa2f01dc596a8880cd2f68a8d83bc6913ce48))
    - Integrate new chunk size code into lookup code ([`a8422cf`](https://github.com/GitoxideLabs/gitoxide/commit/a8422cf0b0c9ff4d3275cc17a68a74811b5bd01f))
    - Simplify progress code using `inc()` ([`9e8df59`](https://github.com/GitoxideLabs/gitoxide/commit/9e8df59d9a6349c49dd80447cbdbde95090e1f04))
    - Add 'inc()' convenience methods to progress ([`2e46c9b`](https://github.com/GitoxideLabs/gitoxide/commit/2e46c9b72a2a5b90bcdac249de07ffbc124cfb04))
    - Run clippy first; pacify clippy ([`0a5b883`](https://github.com/GitoxideLabs/gitoxide/commit/0a5b883c22f2df8a6d51f75c5e09bdfdf276fee4))
    - Use faster algorithm by default ([`bb45c3d`](https://github.com/GitoxideLabs/gitoxide/commit/bb45c3d8a2aabf87231981000240f0444abf6fc4))
    - Properly compute delta chain length by default ([`a93b894`](https://github.com/GitoxideLabs/gitoxide/commit/a93b89464e4484bc7100d5934f14a7321f3ca7a4))
    - Remove hits_to_live ([`3a3fae9`](https://github.com/GitoxideLabs/gitoxide/commit/3a3fae9a8f637481d526d28a695c3f411c1a89a8))
    - Attempt to auto-remove unusable deltas… ([`5dd8243`](https://github.com/GitoxideLabs/gitoxide/commit/5dd8243ceafbb2a89964708f5f9b2783953677aa))
    - Now with cache (and due to that, incorrect statistics for now) ([`efd28d2`](https://github.com/GitoxideLabs/gitoxide/commit/efd28d21acd97709f68ff9404131123cda527cbd))
    - Make chunk statistics independent of traversal method ([`6225f36`](https://github.com/GitoxideLabs/gitoxide/commit/6225f36cc4735dd41b0c01d7c7ce6ed61f384e9a))
    - First working version of alternate object traversal, without cache ([`51b5eb6`](https://github.com/GitoxideLabs/gitoxide/commit/51b5eb6c3a91e323c92e3e8f4069a12cda904354))
    - Initial state for indexed lookup ([`acbcd79`](https://github.com/GitoxideLabs/gitoxide/commit/acbcd79942e9783ca60ac41010a73ef98031d3e9))
    - Refactor; tests now fail with more than just not-implemented ([`310a2f7`](https://github.com/GitoxideLabs/gitoxide/commit/310a2f7f5498ed48777eec53b830b9f7dece33c3))
    - Speedup entry sorting a little; use less memory ([`b4df372`](https://github.com/GitoxideLabs/gitoxide/commit/b4df37258734e55d4679870c639f993305ada73c))
    - Better index entries sorting progress ([`b4d7038`](https://github.com/GitoxideLabs/gitoxide/commit/b4d7038ae729c2631277b0d5ca842a20c609abe9))
    - Prepare sharing even more code ([`61c76cf`](https://github.com/GitoxideLabs/gitoxide/commit/61c76cf6f856f79fd2c77e8ed9cf8940b29d6a50))
    - Make use of shared reducer in upcoming indexed verify implementation ([`290eae1`](https://github.com/GitoxideLabs/gitoxide/commit/290eae115a1df277c4331bb7f2994265da117656))
    - Use shared reduce implementation in lookup based algorithm ([`10fc88d`](https://github.com/GitoxideLabs/gitoxide/commit/10fc88d492821cf67de4cea9beefef4b77d4452b))
    - Prepare for integration of general reducer ([`c37832e`](https://github.com/GitoxideLabs/gitoxide/commit/c37832eb8a6b08cf965c287a104bfdead02776d2))
    - Refactor; enable testing of reverse-delta lookup ([`512daf9`](https://github.com/GitoxideLabs/gitoxide/commit/512daf94038f675353271c930694e0577ac746b4))
    - Revert "Move deallocation off into own thread" - not worth it! ([`051da15`](https://github.com/GitoxideLabs/gitoxide/commit/051da1572a8ed8a99108a337f802ae5f7cc9491e))
    - Move deallocation off into own thread ([`90230f1`](https://github.com/GitoxideLabs/gitoxide/commit/90230f1c0cdd1c9091a3f5e6d9393e05b6c0abb5))
    - Implement more cache-friendly pack offset v2 retrieval ([`00cf84b`](https://github.com/GitoxideLabs/gitoxide/commit/00cf84baeee9932196288c8641f18621610d47a9))
    - Refactor ([`3c25c67`](https://github.com/GitoxideLabs/gitoxide/commit/3c25c6778b3d4fbba9906e0f5b37acbce6c69c61))
    - Initial refactor of DeltaTree, but… ([`6384649`](https://github.com/GitoxideLabs/gitoxide/commit/63846499367a3f106cf668cb84606ca355ad7a3d))
    - Measuring performance of sorting index offsets is quite revealing ([`4b16336`](https://github.com/GitoxideLabs/gitoxide/commit/4b163366cbf5b8e314e1913e24a1d19179e25611))
    - Properly handle the BufReader to make indexing work; FAST ([`57e95cf`](https://github.com/GitoxideLabs/gitoxide/commit/57e95cf79c78285283be88ca9e7baf56c1ad58c0))
    - Avoid seek in favor of skimming a file read in bursts ([`01ae405`](https://github.com/GitoxideLabs/gitoxide/commit/01ae4053ee57f35875d843f00d390acc19e56849))
    - Some performance information in progress ([`20aef2c`](https://github.com/GitoxideLabs/gitoxide/commit/20aef2cf0e0212d5d79a6a4b7ece328adffbdf23))
    - Nodes now provide access to the pack offset ([`61c1497`](https://github.com/GitoxideLabs/gitoxide/commit/61c1497547ee2789f5a90735b72b06186030c3d3))
    - Basic tree access for the entry graph ([`c5e5c77`](https://github.com/GitoxideLabs/gitoxide/commit/c5e5c77aea3981d4f3b0ad528ae25eccdc58ae85))
    - Fix clippy ([`ec40e09`](https://github.com/GitoxideLabs/gitoxide/commit/ec40e093d72f93d86168f39ebaca5b122ca0bec3))
    - Hookup new indexing step ([`313064f`](https://github.com/GitoxideLabs/gitoxide/commit/313064f1875fea6165f9d7feeb31ce0183959044))
    - Frame for running the new streaming code on bigger packs ([`e0b34eb`](https://github.com/GitoxideLabs/gitoxide/commit/e0b34eb87bbf29b31c87d298cdb68e6e0fa5349b))
    - Refactor ([`fdfab40`](https://github.com/GitoxideLabs/gitoxide/commit/fdfab408c38087c5afcdd028e988089c56311baf))
    - Refactor ([`1fbeb35`](https://github.com/GitoxideLabs/gitoxide/commit/1fbeb35cb1a0e66d7e12d678f351fecedc7978dd))
    - Refactor ([`385e935`](https://github.com/GitoxideLabs/gitoxide/commit/385e9356f49fb9e1e87f13137ee270b34527fc0b))
    - Now it works :D ([`008b4de`](https://github.com/GitoxideLabs/gitoxide/commit/008b4defcfbccdd61bc7f5f2c9a8e939f817095d))
    - Initial (failing) implementation of building an index tree ([`25dc83d`](https://github.com/GitoxideLabs/gitoxide/commit/25dc83d660c832cc68306395c7bd303ae806ac07))
    - Easy access to sorted offsets in pack index files ([`d93540f`](https://github.com/GitoxideLabs/gitoxide/commit/d93540fe2a6d4bb70248e82d039d6a2665354ef3))
    - Refactor ([`cb8d561`](https://github.com/GitoxideLabs/gitoxide/commit/cb8d56101bdc4cd7e3fa95ac79f82c1cda99871c))
    - Refactor ([`c7ae705`](https://github.com/GitoxideLabs/gitoxide/commit/c7ae7056eb6a33656b0db31bf1c1012b7ffa2ca8))
    - Refactor ([`2fc449c`](https://github.com/GitoxideLabs/gitoxide/commit/2fc449cdefab1d10a446f83bd2462d1034808d97))
    - Change course and do pack streaming first ([`bcb275e`](https://github.com/GitoxideLabs/gitoxide/commit/bcb275e91cfd6f0a71b3cb59a2b706b60608a594))
    - Roundtrip Rust repo in stress test; accept more diverse trees when parsing ([`0347cdb`](https://github.com/GitoxideLabs/gitoxide/commit/0347cdbf473d80c016745ffbaf582832fe2eba2a))
    - Allow some very special trees not to be round-trippable ([`8fe1358`](https://github.com/GitoxideLabs/gitoxide/commit/8fe1358aa9375bbe63f1ee64174b9e663d140a05))
    - Consume PGP signature in tags fully ([`ffd6c31`](https://github.com/GitoxideLabs/gitoxide/commit/ffd6c31aa3adecc2dea6357373d88a495d63ba0d))
    - Make tagger signature optional ([`3358f9a`](https://github.com/GitoxideLabs/gitoxide/commit/3358f9ae539c7b7878d87a209d678d2f08f94b1b))
    - Remove now unused pgp_signature field - it's in extra-headers ([`c8c937c`](https://github.com/GitoxideLabs/gitoxide/commit/c8c937c505e455572544a1a9da1b991ef4662b97))
    - Proper support for extra-headers ([`d0feb2b`](https://github.com/GitoxideLabs/gitoxide/commit/d0feb2b5b30f9719bf3b40ac5b74f8a5a8515bc9))
    - Switch to latest quick-error ([`9760856`](https://github.com/GitoxideLabs/gitoxide/commit/976085614ee13a19fc1347209259a3dcf36ef95b))
    - Fully implement --encode and --re-encode flags ([`a7cfac8`](https://github.com/GitoxideLabs/gitoxide/commit/a7cfac83ddd859d9c2c25e457c0d7043738792dc))
    - Refactor ([`56b66ac`](https://github.com/GitoxideLabs/gitoxide/commit/56b66ac069f24635a8fa74b4b2231dfb0a82a1ef))
    - Prepare for re-encoding each pack object ([`afae684`](https://github.com/GitoxideLabs/gitoxide/commit/afae684c72e5dc4b718976056dd5d34ed61de72a))
    - Fix build with rustc 1.45 ([`8c2a1ee`](https://github.com/GitoxideLabs/gitoxide/commit/8c2a1ee853c5354117fc0a1b6719108785633915))
    - Refactor ([`ec5e50f`](https://github.com/GitoxideLabs/gitoxide/commit/ec5e50f607d59302d6db3944f6ea7b667f820927))
    - Prepare for writing out owned trees ([`2b6eced`](https://github.com/GitoxideLabs/gitoxide/commit/2b6eced325057a884d56ed9db755a8699cbf8d00))
    - Use borrowed::Id in trees for full type safety ([`5d57c1f`](https://github.com/GitoxideLabs/gitoxide/commit/5d57c1f7e3b9a84f7b46a4378015572155f3104b))
    - Refactor ([`f7b8826`](https://github.com/GitoxideLabs/gitoxide/commit/f7b8826ba144f54f3a3fe6096a5daafd29e25002))
    - Fix odb test ([`a792f44`](https://github.com/GitoxideLabs/gitoxide/commit/a792f44fec60d63aaa16538bf06fb29277e78433))
    - Prepare for allowing an owned, processed version of multi-line headers ([`f966e7f`](https://github.com/GitoxideLabs/gitoxide/commit/f966e7f26cbbe99e5508215adaacf073e108bf48))
    - Use borrowed::Id everywhere ([`9f876f0`](https://github.com/GitoxideLabs/gitoxide/commit/9f876f04feaa3fd3bba9729fff7667708dc0c4be))
    - Move git_object::Id into git_object::owned::Id - much better already! ([`50c7136`](https://github.com/GitoxideLabs/gitoxide/commit/50c71368a69f57b0a43061df105685e992ed384a))
    - Basic integration of borrowed Id; translate between owned and borrowed ([`84ff638`](https://github.com/GitoxideLabs/gitoxide/commit/84ff638a183567593ace8056de2a856304d29d1d))
    - Prepare to allow Id be owned and borrwed; abstract over hash type ([`d883c31`](https://github.com/GitoxideLabs/gitoxide/commit/d883c31dd14f253a3af153616007c9231fdf265a))
    - Introduce the notion of IdRef ([`7007361`](https://github.com/GitoxideLabs/gitoxide/commit/700736197b903cb6fe9ed60718e49e4be44199a7))
    - Use statically known borrowed arrays for perfect type safety! ([`3ead048`](https://github.com/GitoxideLabs/gitoxide/commit/3ead048bb999e6266831df2ca6c2022013529ab2))
    - Refactor ([`766f3e4`](https://github.com/GitoxideLabs/gitoxide/commit/766f3e491dc6ebcca20753cda3487545268721eb))
    - Refactor ([`bca1f16`](https://github.com/GitoxideLabs/gitoxide/commit/bca1f16a6f3da497e3488e333d5ebc99e39ee689))
    - 'data -> 'a as it's shorter and also more idiomatic ([`71821e9`](https://github.com/GitoxideLabs/gitoxide/commit/71821e938887f448f1458642eda2bac365f2aa85))
    - Refactor ([`dedd4dc`](https://github.com/GitoxideLabs/gitoxide/commit/dedd4dc91c26dfef368307345bb9e8d49637207c))
    - Refactor ([`de0bc3c`](https://github.com/GitoxideLabs/gitoxide/commit/de0bc3cb4a32bf4cd02ce7c8420bc008e469b779))
    - Refactor ([`e5391d3`](https://github.com/GitoxideLabs/gitoxide/commit/e5391d36d192c3c12426102f734d2e227c568a08))
    - Refactor ([`163909b`](https://github.com/GitoxideLabs/gitoxide/commit/163909b593e21860d0a292c6e45daee93fb270fb))
    - Refactor ([`49f64db`](https://github.com/GitoxideLabs/gitoxide/commit/49f64db88fc0643e0bff215efdb9b1b429b648ba))
    - Refactor ([`9f825b8`](https://github.com/GitoxideLabs/gitoxide/commit/9f825b849d14494a2d58a09eb6499fd86fd05af3))
    - Refactor ([`2fbc2e1`](https://github.com/GitoxideLabs/gitoxide/commit/2fbc2e1f76f972758b0c880d3eabdf75586749e7))
    - Fix naming change, which was introduced accidentally ([`fbb9f98`](https://github.com/GitoxideLabs/gitoxide/commit/fbb9f98508ec722e192466e28ded47aef2fb78b3))
    - Make it easier to validate bundles, for completeness ([`8ea05de`](https://github.com/GitoxideLabs/gitoxide/commit/8ea05de8d1ae49e09465a66354cf69dd7c7a2e05))
    - Refactor ([`34e85f2`](https://github.com/GitoxideLabs/gitoxide/commit/34e85f2242b12ec1560b8e50bc9ab447cd1805fc))
    - Refactor ([`b3bde87`](https://github.com/GitoxideLabs/gitoxide/commit/b3bde870054cb4001c1e2ea8a81b1a4b1d83405b))
    - Refactor ([`0b540c2`](https://github.com/GitoxideLabs/gitoxide/commit/0b540c236b3459b340f13908ca52c82e40378e13))
    - Refactor ([`2888f1b`](https://github.com/GitoxideLabs/gitoxide/commit/2888f1b10a2baf40155544e667ddd461f3ddc938))
    - Refactor ([`0817b24`](https://github.com/GitoxideLabs/gitoxide/commit/0817b24fae6106db2d9e3fcfcdcb10b9a182911d))
    - Refactor ([`dcacd3b`](https://github.com/GitoxideLabs/gitoxide/commit/dcacd3b06d7a4532c600dfdf62e03561e8ed55ef))
    - Refactor ([`b113da9`](https://github.com/GitoxideLabs/gitoxide/commit/b113da945715f9611eb0fb79925d1239eaf1569c))
    - Refactor ([`6659174`](https://github.com/GitoxideLabs/gitoxide/commit/66591745f08d15f3756a352f4041c807ea92fc6f))
    - Refactor ([`bed5dc8`](https://github.com/GitoxideLabs/gitoxide/commit/bed5dc80c5b307c6d35f7b4405693dce1f7f6d71))
    - Refactor ([`4867740`](https://github.com/GitoxideLabs/gitoxide/commit/486774096a86f6eb001d812e6ac9ab0b29791148))
    - Refactor ([`f6cc80e`](https://github.com/GitoxideLabs/gitoxide/commit/f6cc80e5f0ae966c83345be64219fa7ebe0e1db2))
    - Refactor ([`8b416d4`](https://github.com/GitoxideLabs/gitoxide/commit/8b416d4b8417c04ea5d3527a88190d867dc8b7c2))
    - Refactor ([`23e05d7`](https://github.com/GitoxideLabs/gitoxide/commit/23e05d78c73b2bfce3025b3e34746d48026b34ed))
    - Refactor ([`d3b36f4`](https://github.com/GitoxideLabs/gitoxide/commit/d3b36f4ad8a5cb9266542ee997941c879121be96))
    - More tests for various object types ([`f4703e0`](https://github.com/GitoxideLabs/gitoxide/commit/f4703e047834d13748f21db861fd0a753d5b1233))
    - Refactor ([`86fa00f`](https://github.com/GitoxideLabs/gitoxide/commit/86fa00f0967dba5453f7226125123ef398e48790))
    - Basic decode implementation ([`7ff02cb`](https://github.com/GitoxideLabs/gitoxide/commit/7ff02cb84469f5aa4a3be1489927344b45385a45))
    - Support for in-pack object lookup in Bundle::locate ([`7e3d6be`](https://github.com/GitoxideLabs/gitoxide/commit/7e3d6be5136d9c3816bedd3b9797186457aeb476))
    - First dummy implementation of borrowing a buffer provided by the user ([`9c31fcb`](https://github.com/GitoxideLabs/gitoxide/commit/9c31fcb7c25be5c75e3dad1e940683b8ae42b935))
    - Make it easy to learn that objects couldn't be located by using options ([`a916f36`](https://github.com/GitoxideLabs/gitoxide/commit/a916f367d329927369b127a5f2fba63e8d4d9d88))
    - Mild refactor - need combined pack + index ([`6bf8ed4`](https://github.com/GitoxideLabs/gitoxide/commit/6bf8ed470803ab58737b119d892b7eabb77fd8b9))
    - Respect thread limit in 'in_parallel' ([`babfd84`](https://github.com/GitoxideLabs/gitoxide/commit/babfd84cba77ef7a0f541ba921b31ebd3f3c50e3))
    - Apply cargo diet ([`79b9b73`](https://github.com/GitoxideLabs/gitoxide/commit/79b9b7398be608de1f439f56b057dc08d421081f))
    - Add missing license description ([`2b80181`](https://github.com/GitoxideLabs/gitoxide/commit/2b80181ad428a9bf267a9660886f347a850fc76f))
    - Make crates publishable ([`5688a34`](https://github.com/GitoxideLabs/gitoxide/commit/5688a3427ff3673e1422d43106f4d685fa837aed))
    - Cargo clippy (from CI) ([`0a28857`](https://github.com/GitoxideLabs/gitoxide/commit/0a288579545345c0dffdfa814b052959baec0a34))
    - Proper implementation of line renderer into 'lean' CLI ([`e98e7c2`](https://github.com/GitoxideLabs/gitoxide/commit/e98e7c280d73e9d9ebd13202afb93a56cb2f7c9c))
    - Handle windows newlines in test suite for packs as well. ([`ebd5176`](https://github.com/GitoxideLabs/gitoxide/commit/ebd517633f099582dc2633e71f7bb7890acd14d1))
    - Add metadata to allow docs.rs build all featueres ([`10f9386`](https://github.com/GitoxideLabs/gitoxide/commit/10f9386a12decc1f13999aee72be484c8f6d48ce))
    - Update tasks ([`269280a`](https://github.com/GitoxideLabs/gitoxide/commit/269280a0e00c9eee54d591b96c3ed0e9d4202489))
    - Allow to limit the logging depth for less cluttered output ([`fce7035`](https://github.com/GitoxideLabs/gitoxide/commit/fce703531d7006f7d961d6ffa66f51f6c9bc0efc))
    - Looks like this performs much better already, but… ideally subprogress isn't shown ([`3b96d18`](https://github.com/GitoxideLabs/gitoxide/commit/3b96d18483a845f7692f94cc40c28871fd96e479))
    - Finally speed up logging progress properly - needs input throttling ([`1a550c6`](https://github.com/GitoxideLabs/gitoxide/commit/1a550c6458b10fad2e42b641899216c5517c6e26))
    - Provide average throughput per second ([`5b23d17`](https://github.com/GitoxideLabs/gitoxide/commit/5b23d171102ad859258b9673bf35561ef9e8f246))
    - Git-odb with serde support ([`0da930c`](https://github.com/GitoxideLabs/gitoxide/commit/0da930cf23f215cc1e2bda8f7340a5d69370735a))
    - Remove dependency to git-object from git-features - it better remains free ([`67c3a6a`](https://github.com/GitoxideLabs/gitoxide/commit/67c3a6ab4cc32358a1406c2f863e26a4c2929867))
    - Commit to using bstr whenever something is not data bytes; remove miniserde ([`3183d1b`](https://github.com/GitoxideLabs/gitoxide/commit/3183d1b02c2d7bb3c750f8472c29bb161641ca7f))
    - Prepare centralization of bstr as optional component ([`aa857d9`](https://github.com/GitoxideLabs/gitoxide/commit/aa857d9df32dfc75f151154ca430ddfee907deed))
    - \#[forbid(unsafe)] for all crates ([`afda803`](https://github.com/GitoxideLabs/gitoxide/commit/afda8039259b7a30cfed5dbcdd9caf4773b4c234))
    - Allow for more screen space when formatting ([`6794300`](https://github.com/GitoxideLabs/gitoxide/commit/67943002e7f4215b5383bd0538786ce2857f011e))
    - Prepare next task ([`74bcbb5`](https://github.com/GitoxideLabs/gitoxide/commit/74bcbb506585aa9e0955253d07bab111f83f014e))
    - Display object throughput per second, even though it won't be visible in TUI… ([`53b4513`](https://github.com/GitoxideLabs/gitoxide/commit/53b4513f6a8bb2f2e5b07fa72a3085e620cee24c))
    - Disable LRU cache if we have to get statistics ([`befba3b`](https://github.com/GitoxideLabs/gitoxide/commit/befba3b769195fb592d714afe12194a61ae4a330))
    - Wonderful statistics on compression efficiency! ([`1bb09c5`](https://github.com/GitoxideLabs/gitoxide/commit/1bb09c509dae4e493ab05022bbf51c0b1786d479))
    - Count objects per chain level ([`209d53f`](https://github.com/GitoxideLabs/gitoxide/commit/209d53f531ec9bcffbb04ba060447bee59ad26f6))
    - Pass average stats through to the top level ([`5b4979c`](https://github.com/GitoxideLabs/gitoxide/commit/5b4979c1dfeb9a29974dd4e6529ae5da074d0b1a))
    - Refactor ([`4dd9fd4`](https://github.com/GitoxideLabs/gitoxide/commit/4dd9fd4a2c48380bda9a865ef704e7fdfa7e5b89))
    - Closer to actually producing statistics ([`5f087ec`](https://github.com/GitoxideLabs/gitoxide/commit/5f087ec30a50775ad8bb67f21e352fe9ee1ccc9f))
    - Refactor ([`7add82c`](https://github.com/GitoxideLabs/gitoxide/commit/7add82c39169e3c2fff76c48cdd318fe6040d7bc))
    - Also average statistics on chunk level ([`3b927e5`](https://github.com/GitoxideLabs/gitoxide/commit/3b927e50173e3feae72cde8a226cee524275403a))
    - Provide more detailed information when decoding an entry ([`80c5da9`](https://github.com/GitoxideLabs/gitoxide/commit/80c5da9bd88e1f329292f3f93ba53c8ff8324a20))
    - No need to say 'begin' before doing something, it's primarily for logging ([`13eba3a`](https://github.com/GitoxideLabs/gitoxide/commit/13eba3a3484068939436996352fe5585aa221bca))
    - Throughput for pack ([`81f5c33`](https://github.com/GitoxideLabs/gitoxide/commit/81f5c335b224dd85062c9208cee2bb288ad3e833))
    - Print performance stats at the end of hashing the index ([`9c94417`](https://github.com/GitoxideLabs/gitoxide/commit/9c9441709c9a759a3a0916402921c7beeb735d75))
    - Assure hashing progress is dropped when done ([`db6e067`](https://github.com/GitoxideLabs/gitoxide/commit/db6e067c5dd90311d174881546d8df8f521eb552))
    - First implementation of logging per thread ([`477dd90`](https://github.com/GitoxideLabs/gitoxide/commit/477dd90ce5e102875b19489bf8ae9877522ef9c8))
    - Support for providing progress to threads ([`2815858`](https://github.com/GitoxideLabs/gitoxide/commit/2815858adf7ac0f7b4cbc88cf05df0ea6aef4116))
    - Properly count objects ([`d398e7e`](https://github.com/GitoxideLabs/gitoxide/commit/d398e7e68ad893d21a088ec6ac727dc8577317fc))
    - First very basic progress implementation ([`b820717`](https://github.com/GitoxideLabs/gitoxide/commit/b8207177daee8a9ffa23c7c052cf9ca651b15804))
    - Pass progress everywhere, for now just to discard it ([`da3ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/da3ae1c82cd726b8fae9b8d26069719930e9ba99))
    - Control which hashing crates to use from the top-level as well. ([`dfe9b20`](https://github.com/GitoxideLabs/gitoxide/commit/dfe9b203b2e877a7e345b4f2942bf5a1582ab43e))
    - Use git-features to toggle 'parallel' mode from the 'gitoxide' level ([`d944fbf`](https://github.com/GitoxideLabs/gitoxide/commit/d944fbf181acc5fb83a841613174702af1e074d6))
    - Sketch out `Progress` trait; don't forget to write docs at some point ([`534b3c7`](https://github.com/GitoxideLabs/gitoxide/commit/534b3c73101fd1b885de630523ab706bd06a327b))
    - Refactor ([`baeb4ef`](https://github.com/GitoxideLabs/gitoxide/commit/baeb4ef1a9680c212ce9d1010e2c34eedafcd246))
    - Refactor ([`e12bfd6`](https://github.com/GitoxideLabs/gitoxide/commit/e12bfd645bb2f707a1b5077190d9f37393a8e315))
    - Make `in_parallel` trait bound more loose: Clone instead of copy ([`3e91b05`](https://github.com/GitoxideLabs/gitoxide/commit/3e91b0512919c02899324564b8f571ce534955d9))
    - Using all cores actually does speed things up ([`ed944b9`](https://github.com/GitoxideLabs/gitoxide/commit/ed944b9480ae647c7e75a7a07a9c59885725b3a0))
    - Also run index+pack validation in parallel; only parallelize bigger packs ([`dc15b26`](https://github.com/GitoxideLabs/gitoxide/commit/dc15b2652cc6b9e94f80aebfeec8f879ae5a529f))
    - Avoid running anything in parallel for small packs ([`c2df183`](https://github.com/GitoxideLabs/gitoxide/commit/c2df183943e8b533c4cd5f5833f61ad94942943d))
    - Don't send every single entry, instead send reasonably sized chunks ([`56298a6`](https://github.com/GitoxideLabs/gitoxide/commit/56298a62ea8cc9c6fef7f682ffb8ddda5404ca9b))
    - Refactor (down to 6 minutes for big pack verification) ([`4157b51`](https://github.com/GitoxideLabs/gitoxide/commit/4157b5196936e9f5f884a645f7e1c37ba6b13b52))
    - First working version of actually parallel `in_parallel` ([`145ee39`](https://github.com/GitoxideLabs/gitoxide/commit/145ee399e2c057aec3330e26bafb7910ca7dc56d))
    - First implementation of 'parallel' without threads. How will scoped fare? ([`735744e`](https://github.com/GitoxideLabs/gitoxide/commit/735744e1960a3055b836767c85613ba9d147cdd4))
    - A sketch of a minimal helper for parallel work ([`377252a`](https://github.com/GitoxideLabs/gitoxide/commit/377252a3b4869952059e832ce32656e2cf2a674c))
    - Refactor ([`be4795f`](https://github.com/GitoxideLabs/gitoxide/commit/be4795f00d7b693cb52f93857ac3b4b65340053f))
    - Refactor ([`3e2efff`](https://github.com/GitoxideLabs/gitoxide/commit/3e2efffc945a0737c2d8b820a93b013e6ffa45e2))
    - Bigger LRU caches are better, but with this one we can't go too large ([`5e1f7ae`](https://github.com/GitoxideLabs/gitoxide/commit/5e1f7aedc970552d3ec4ab3358757af790ce6628))
    - First implementation of an LRU cache - it gets hit, let's see how it fares! ([`5a21031`](https://github.com/GitoxideLabs/gitoxide/commit/5a21031415a6e2ca43cb828492fd5517d2a98e9e))
    - Also set the cache with bases and deltas ([`915a3fb`](https://github.com/GitoxideLabs/gitoxide/commit/915a3fb21c950dd35a97a735375a144bbc59e3b1))
    - First sketch of cache implementation - get() is there, next is put() ([`ce54756`](https://github.com/GitoxideLabs/gitoxide/commit/ce547565de23e89212bf6197178191ddf5b11fd3))
    - Allow delta base resolution to fail (similar to how lookups can fail) ([`b721424`](https://github.com/GitoxideLabs/gitoxide/commit/b7214241dfbb85c3115e230fa502f790133e2192))
    - Allow in-pack lookups for V1 packs ([`2e51bbb`](https://github.com/GitoxideLabs/gitoxide/commit/2e51bbbab4a47001ef725d0bf8bf5714d0c37e70))
    - Add CRC32 reading at index ([`268f855`](https://github.com/GitoxideLabs/gitoxide/commit/268f855da9db5f694bedb073493778147d646271))
    - Pack offset by index ([`69e35b1`](https://github.com/GitoxideLabs/gitoxide/commit/69e35b1d8f24f366d675484a1bddbebd37b72e22))
    - V2 pack lookup ([`9e56902`](https://github.com/GitoxideLabs/gitoxide/commit/9e56902bdb7702181809c6a4c2280750ddd64044))
    - Test V1 lookup ([`e9c7127`](https://github.com/GitoxideLabs/gitoxide/commit/e9c71271fa51d5420fcb205d2d3deb6d012f0d41))
    - Add CRC32 check during pack verification ([`04ff1a0`](https://github.com/GitoxideLabs/gitoxide/commit/04ff1a0bf9aa164e9cff262ec521eab76c2e4688))
    - Prepare for CRC32 check - needs understanding of size of bytes in packed object ([`3ab2df1`](https://github.com/GitoxideLabs/gitoxide/commit/3ab2df1e00eb41e8a222b208131f63ba3e065df5))
    - Refactor ([`dd2d623`](https://github.com/GitoxideLabs/gitoxide/commit/dd2d6238771ff86df6a412a6d817aa92a5e5ed43))
    - Finally delta-objects can be read as expected. ([`81f2f54`](https://github.com/GitoxideLabs/gitoxide/commit/81f2f547bad33c414f6e12d16df4922274b06758))
    - Definitely an improvement to the way add-deltas are applied… ([`c6cdb12`](https://github.com/GitoxideLabs/gitoxide/commit/c6cdb12b47f6d5f4e3f02895acb2de08a7df00cc))
    - Fix one issue with Trees being declared as tags ([`ada66cd`](https://github.com/GitoxideLabs/gitoxide/commit/ada66cdbab0fec4765428ce815c0868d34d5babf))
    - Validate sha1 of pack objects, some work, some don't for some reason… ([`aa8799a`](https://github.com/GitoxideLabs/gitoxide/commit/aa8799a01b92c3c3b7d4347f745921bbb685c454))
    - Capability to write loose object headers, fast ([`de0aeff`](https://github.com/GitoxideLabs/gitoxide/commit/de0aeff518ebd218b73bf472558f278f6bcdc17c))
    - Refactor ([`5364bbe`](https://github.com/GitoxideLabs/gitoxide/commit/5364bbe0415c37f684066e22eb017fe5d7ca7c64))
    - Fix another implicit assumption that doesn't hold: deltas are NOT… ([`093637d`](https://github.com/GitoxideLabs/gitoxide/commit/093637da964b807fa767009732e9b93002e35fab))
    - Finish delta-application to take into account the biggest possible result… ([`0ee2b69`](https://github.com/GitoxideLabs/gitoxide/commit/0ee2b696014012864b0645bd1b9da508cb1e465c))
    - First stab at dealing with bigger-than-expected intermediate result sizes… ([`8027ff4`](https://github.com/GitoxideLabs/gitoxide/commit/8027ff4de7ffe6126cf1ade4938baa08899cb938))
    - First simple implementation of fetching all objects in a pack (without validation) ([`053045b`](https://github.com/GitoxideLabs/gitoxide/commit/053045bb23e2a85e2a1d16eeb65c399dfabba5b4))
    - Support for verifying pack files and index files ([`b09b4e1`](https://github.com/GitoxideLabs/gitoxide/commit/b09b4e1f35c3802dfd3418bda42b96828acd9ec8))
    - Simple index file verification (internal) ([`1d27050`](https://github.com/GitoxideLabs/gitoxide/commit/1d27050f21ee1c8f492d38e14c294fa31a7b48a1))
    - Refactor ([`4023b02`](https://github.com/GitoxideLabs/gitoxide/commit/4023b0260b0b139853f8dc1b9260045a8dac6e47))
    - Refactor ([`855a769`](https://github.com/GitoxideLabs/gitoxide/commit/855a769026f81739f28b38507c0bef7b59e97a8b))
    - Refact[r ([`c84410b`](https://github.com/GitoxideLabs/gitoxide/commit/c84410b2b0e66c10c30fc70c3674c971b270204d))
    - Refactor ([`c24c79d`](https://github.com/GitoxideLabs/gitoxide/commit/c24c79d65b947625a5a9ab73dbd3afdef060fa12))
    - Test --no-default-features for git-odb ([`2394bca`](https://github.com/GitoxideLabs/gitoxide/commit/2394bca4a76247c420fe06c59d0d76819c6e978b))
    - Refactor; prevent trailing bytes to become part of the digets ([`043813c`](https://github.com/GitoxideLabs/gitoxide/commit/043813cd2e49b358e17ad78d975ef255924c78fa))
    - Try a version that doesn't rely on memory mapped files for throughput… ([`d59ddfc`](https://github.com/GitoxideLabs/gitoxide/commit/d59ddfcf50edd0bfc8252e6c7a68c86fe27b5a9f))
    - Try to speed it up with prefetching - not really :D ([`8485185`](https://github.com/GitoxideLabs/gitoxide/commit/8485185bcb7895461dc4347f25b9f0b0bab54594))
    - Simplify folder names ([`36fde1f`](https://github.com/GitoxideLabs/gitoxide/commit/36fde1f90e9034060b5ede8a923365474659085e))
    - Fix LSB parsing code with python based code written 6 years ago :D ([`c12fdad`](https://github.com/GitoxideLabs/gitoxide/commit/c12fdadbf839ce6f8a638fe25667d870a8f6b808))
    - Improved packed header parsing… it works a little better now it seems, but… ([`ca779ed`](https://github.com/GitoxideLabs/gitoxide/commit/ca779edc457f1f1baed05e8c64bb2994f6b12945))
    - Refactor; and figured out what the header parsing issue is ([`d364049`](https://github.com/GitoxideLabs/gitoxide/commit/d3640493e509b782589b4c0680962e6e1f2ae665))
    - Some more tests ([`85e541f`](https://github.com/GitoxideLabs/gitoxide/commit/85e541f36fd7795c53d0dc3d07d5b76a6725c889))
    - Refactor; better error handling ([`031df11`](https://github.com/GitoxideLabs/gitoxide/commit/031df11a3c3767330c9f13cab0e55c2559a72e9b))
    - First very rough version of full-object decompression without allocation ([`7c704a7`](https://github.com/GitoxideLabs/gitoxide/commit/7c704a71e51607149a7a6a1293a401f4c7ecb610))
    - Refactor ([`dcb1997`](https://github.com/GitoxideLabs/gitoxide/commit/dcb19971841d3330df63c67f73793f0a45b6c74f))
    - Refactor ([`baaf06e`](https://github.com/GitoxideLabs/gitoxide/commit/baaf06e36605f9b79ef09dd7cbdbb42fb16b64be))
    - Refactor ([`3edaaec`](https://github.com/GitoxideLabs/gitoxide/commit/3edaaec2fad6594049a0f10a4bf921dc3c485ac0))
    - Finish Object Reader implementation, now for in-memory objects, too ([`35e69b8`](https://github.com/GitoxideLabs/gitoxide/commit/35e69b87521eef89705012a7170517670ee20e7c))
    - A simpler implementation to skip the header ([`47ca6ab`](https://github.com/GitoxideLabs/gitoxide/commit/47ca6ab2ff0cbf8801d0a82cebbbeb8c4f62cdae))
    - Allow skipping the header when decompressing files (streaming) ([`ff35032`](https://github.com/GitoxideLabs/gitoxide/commit/ff350323e4a424df8c17a9dca53cc8967e45e960))
    - First step towards supporting skipping the header in the stream ([`8e45f53`](https://github.com/GitoxideLabs/gitoxide/commit/8e45f5370516b0df9df4e984d29161d399697fdd))
    - Fix stream decoding - it seems to work, but we need to deal with the header ([`f10ed75`](https://github.com/GitoxideLabs/gitoxide/commit/f10ed75a74c183edeb2a5bd665e5649a5b282e93))
    - Tests for streamed reading of bigger objects (FAIL) ([`b4a6b72`](https://github.com/GitoxideLabs/gitoxide/commit/b4a6b7233ff4f4154d1dd46a29a88787746899f8))
    - Refactor ([`80aad4b`](https://github.com/GitoxideLabs/gitoxide/commit/80aad4b97b76b26050c87eac483b8af1fcfb61ed))
    - Add missing parts to implement Read, need refactoring to make it work though ([`13d4cdb`](https://github.com/GitoxideLabs/gitoxide/commit/13d4cdb32fe197d1517270183d9547ddf1aa381e))
    - First step towards streaming of ZLIB deflated content ([`a870f7a`](https://github.com/GitoxideLabs/gitoxide/commit/a870f7a5bca9f57374e7c9582866473fbbce6e5e))
    - Cleanup ([`a2f0a5d`](https://github.com/GitoxideLabs/gitoxide/commit/a2f0a5dec0b183712e03397e8b4340fed77ce008))
    - Fix clippy ([`a9c5da7`](https://github.com/GitoxideLabs/gitoxide/commit/a9c5da7132eeaa6806b8190985a7aa25f9ef89d8))
    - Make decompression of bigger objects work (on the fly) ([`7e4f5a9`](https://github.com/GitoxideLabs/gitoxide/commit/7e4f5a9594b31c67a49a1d2d6a063241ab8821d9))
    - It becomes obvious that this way of decompressing things won't work ([`1818bda`](https://github.com/GitoxideLabs/gitoxide/commit/1818bda0acc83354b093c39831e2844d48eb5637))
    - Don't do so much logic if we already decompressed everything ([`26cb36c`](https://github.com/GitoxideLabs/gitoxide/commit/26cb36ce3717a63ca7934e7bbc35052208227056))
    - Refactor ([`423b885`](https://github.com/GitoxideLabs/gitoxide/commit/423b8857f1dc580d64ec4075f955d34524979269))
    - More convenient access to our four object types ([`ecda6d2`](https://github.com/GitoxideLabs/gitoxide/commit/ecda6d23561dc176f7d7ad2565da8105efac614f))
    - It's proably OK to make parsed pack entries avaialble, why not ([`8a64e10`](https://github.com/GitoxideLabs/gitoxide/commit/8a64e10ae5206e10f487fbde88412037c165e583))
    - Refactor ([`13f0e77`](https://github.com/GitoxideLabs/gitoxide/commit/13f0e77c0d67f8078bfaf96c3bb735f8c3161a3f))
    - Memory size checks for objects ([`ab51616`](https://github.com/GitoxideLabs/gitoxide/commit/ab51616bb250a62b5367e861c25c1d90ec60f720))
    - Reduce loose Object memory footprint ([`38a81b0`](https://github.com/GitoxideLabs/gitoxide/commit/38a81b0fc3ef1bff54779f0cf531ea2e0f82ebd8))
    - First Blob test for blobs that are already in memory ([`f503324`](https://github.com/GitoxideLabs/gitoxide/commit/f503324b33fd7289782fe642b1f566e9d101ceab))
    - Make single-field objects blob and tree more explicit ([`1aef68f`](https://github.com/GitoxideLabs/gitoxide/commit/1aef68f7e979324eb94966d44c160ffe537ee4a8))
    - Add Blob type to parsed objects ([`d3e8e4b`](https://github.com/GitoxideLabs/gitoxide/commit/d3e8e4b24ecda84665b994ccad768774efdcdc90))
    - See 'parsed' blobs as in-memory representations… ([`6a6e105`](https://github.com/GitoxideLabs/gitoxide/commit/6a6e105b3e438380b55f9e9566f0acd76c5efffd))
    - Make clear that not all objects can be parsed at the expense of convenience ([`ce3031d`](https://github.com/GitoxideLabs/gitoxide/commit/ce3031da8ba1eb3e66d72474a8efc65c2990bc99))
    - Don't conflate errors with 'there is no suitable object' to parse ([`b9b796f`](https://github.com/GitoxideLabs/gitoxide/commit/b9b796f69ced726167d72615e5628263a3158a35))
    - Fix imports ([`10f2967`](https://github.com/GitoxideLabs/gitoxide/commit/10f29675442c76b38e0a8deb757930a13af3a3bb))
    - Try pub use with rename. Not bad in the docs, but maybe a bit confusing ([`526f3f8`](https://github.com/GitoxideLabs/gitoxide/commit/526f3f8d3ca9fe9672b0518f1bc3b921f695c0d8))
    - Refactor ([`b9a1647`](https://github.com/GitoxideLabs/gitoxide/commit/b9a16473ed028abc59fc5126db9530f2107498d8))
    - Integrate Commit object into Loose DB ([`7e9fe50`](https://github.com/GitoxideLabs/gitoxide/commit/7e9fe505f08def0378c967514a9389da9e46301d))
    - Test for parsing trees from loose dbs ([`4f48249`](https://github.com/GitoxideLabs/gitoxide/commit/4f4824971d62d165fd4c2bea869fd88986dc259f))
    - Refactor ([`9f9ccad`](https://github.com/GitoxideLabs/gitoxide/commit/9f9ccad37fea96954a2df9e314b6c154466dc0ca))
    - Refactor ([`427c480`](https://github.com/GitoxideLabs/gitoxide/commit/427c48007016e95b13d8750df8b6ac1620f465ac))
    - Refactor loose db ([`6ea4f53`](https://github.com/GitoxideLabs/gitoxide/commit/6ea4f5331f8d4279025e3f912315af50f0eedbdc))
    - Handle commits without newlines; make tag newlines optional ([`c0b54be`](https://github.com/GitoxideLabs/gitoxide/commit/c0b54bef5a2bcfce9b6deb90cdd27c7e0cc85810))
    - Make Commit available in borrowed object ([`b2d1b5d`](https://github.com/GitoxideLabs/gitoxide/commit/b2d1b5d684bdfda5f922b466cc13d4ce2d635cf8))
    - Avoid unnecessary allocation when creating SHA1 paths in loose ODB ([`09d8d3a`](https://github.com/GitoxideLabs/gitoxide/commit/09d8d3a12e161a7f6afb522dbe8900a9c09bce06))
    - First silly attempt to randomly remove an allocation ([`4ff2168`](https://github.com/GitoxideLabs/gitoxide/commit/4ff21686c32a6edc84ea041c3040f33ae24f9519))
    - Document existing use of unsafe, deny everywhere else ([`41f4bce`](https://github.com/GitoxideLabs/gitoxide/commit/41f4bce9d9a492f8e20a6eb5b3eaf5adc6d78329))
    - Cleanup integer parsing in loose object database ([`ecdce1a`](https://github.com/GitoxideLabs/gitoxide/commit/ecdce1a05d8c732afd53c6da6067bf591f96fa6a))
    - The defining property is actually that the object is borrowing data ([`e0125fd`](https://github.com/GitoxideLabs/gitoxide/commit/e0125fdb0a41ed139364084f6d679932f08b7b4f))
    - Fix cargo fmt ([`642dd13`](https://github.com/GitoxideLabs/gitoxide/commit/642dd13afa77ea9c0f4a20d59f54b84bf6ca3333))
    - Cleanup; all tests work! ([`7c96603`](https://github.com/GitoxideLabs/gitoxide/commit/7c9660354484869681356a8c4ef8057313e864f2))
    - First version of tag message parsing - it's actually changed now ([`74b2328`](https://github.com/GitoxideLabs/gitoxide/commit/74b2328fcbbcffab9981c23e903c4f4c5d085aff))
    - Remove itertools in favor of vendoring the little code we need ([`8340508`](https://github.com/GitoxideLabs/gitoxide/commit/834050878b43bae677287767332adc746a8aa2ed))
    - Optimize macro usage ([`0c9960b`](https://github.com/GitoxideLabs/gitoxide/commit/0c9960b1a9404ec8db62ffeeedb3e482eba81c77))
    - Optimize dependencies ([`3ea2195`](https://github.com/GitoxideLabs/gitoxide/commit/3ea2195090728f17ae425e4816405f10b7eb8a14))
    - Use git-object in git-odb ([`07f7c31`](https://github.com/GitoxideLabs/gitoxide/commit/07f7c318d55603e3731f08cb04d3da8ac2fcfea6))
    - Add the latest nom, hoping it will be come out of alpha… ([`85958f1`](https://github.com/GitoxideLabs/gitoxide/commit/85958f1771b521f905528ca426404b846244e122))
    - Refactor; use pretty-assertions for massively more readable test-failures ([`ea8d311`](https://github.com/GitoxideLabs/gitoxide/commit/ea8d3113c32fff85c02d8ff2217adc6b42153137))
    - Switch everything parsed to BStr ([`62ae90a`](https://github.com/GitoxideLabs/gitoxide/commit/62ae90a37d0dea33a23eb7d026cdf9b719692078))
    - Refactor ([`9a86f63`](https://github.com/GitoxideLabs/gitoxide/commit/9a86f6352ccd5178198ad87df44d88358b475d1a))
    - Use btoi to parse all integers, directly from ascii-bytes ([`4f6ef42`](https://github.com/GitoxideLabs/gitoxide/commit/4f6ef42a0b871096f81bd0cb9759aa651a1943d0))
    - Refactor ([`2990902`](https://github.com/GitoxideLabs/gitoxide/commit/299090296fb3a2074c74289c4645b79d3f736ed0))
    - Move parsing tests close to actual parsing ([`3ca2c59`](https://github.com/GitoxideLabs/gitoxide/commit/3ca2c592d91c9aa8fab8ed749871d6d96f2ef4e2))
    - Move examples into demos, having their very own dependencies; optimize tests ([`b757712`](https://github.com/GitoxideLabs/gitoxide/commit/b757712f82de1d75ed813e744f979c1c652350e6))
    - Fix (untested) extraction of delta object information ([`55a56b7`](https://github.com/GitoxideLabs/gitoxide/commit/55a56b70b7b5a80089fde2edfff3ab3743d61cdd))
    - Parallelize git-conut, optimize for speed ([`debd044`](https://github.com/GitoxideLabs/gitoxide/commit/debd0445ba482d7b4424e53c45c0b6acf8b1de37))
    - Refactor ([`9fc9fc0`](https://github.com/GitoxideLabs/gitoxide/commit/9fc9fc0e706eaced2a4f04ae082f9f5acdde1fc0))
    - Fix big-pack 64 bit offset handling in index v2 ([`3b485b5`](https://github.com/GitoxideLabs/gitoxide/commit/3b485b57062765b7ea476feaed328f4f94fc3478))
    - Make refactor ([`cd6a18a`](https://github.com/GitoxideLabs/gitoxide/commit/cd6a18ace5c07b542475518f6cfb506d34547013))
    - Cargo clippy first pass ([`8b0a2a8`](https://github.com/GitoxideLabs/gitoxide/commit/8b0a2a8b0665cb4bd7c32e46bec9dc33114e4985))
    - Finally remove failure and equip example with anyhow ([`f5e4ec5`](https://github.com/GitoxideLabs/gitoxide/commit/f5e4ec5804efec4966ab1ca7fbf6e1a757f2f8c2))
    - Remove failure from Index ([`55034a7`](https://github.com/GitoxideLabs/gitoxide/commit/55034a7a22404b2d6a117c7242852480f42b84ab))
    - And one more module without failure ([`d0575bf`](https://github.com/GitoxideLabs/gitoxide/commit/d0575bf39e6eebd0337bb8712eda1141b5766e92))
    - A big step towards removing failure ([`d862bd8`](https://github.com/GitoxideLabs/gitoxide/commit/d862bd87a4d5bcafce83eed6c49c15a093972416))
    - Refactor ([`87c8a2e`](https://github.com/GitoxideLabs/gitoxide/commit/87c8a2e288140b04e163fe85266d040d039ec69c))
    - Get rid of failure crate in favor of quick-error ([`91c8fc1`](https://github.com/GitoxideLabs/gitoxide/commit/91c8fc1f0c50af55d7cb233bbe813c6d12fe11bc))
    - Get rid of nightly requirement, just parse tags differently soon ([`f037c4d`](https://github.com/GitoxideLabs/gitoxide/commit/f037c4d982f2158cf173dce898c8dda1aea14106))
    - Cargo fmt ([`2aa0857`](https://github.com/GitoxideLabs/gitoxide/commit/2aa085752aa3e99b51034a3dec882aea8c27ad94))
    - Reorganize repository a bit; use different contact email address ([`cb9fa28`](https://github.com/GitoxideLabs/gitoxide/commit/cb9fa2848476e30767deb9d9807c649e0bc366da))
</details>

## 0.40.1 (2023-01-10)

A maintenance release without user-facing changes.

## 0.40.0 (2023-01-09)

A maintenance release without user-facing changes.

## 0.39.0 (2022-12-30)

A maintenance release without user-facing changes.

## 0.38.1 (2022-12-26)

<csr-id-46636e64c9a48ec0e85e014ac0cc8b48846d8462/>

### Bug Fixes

 - <csr-id-4fffa9a9198cf3012fa8215796aab3d456519ff3/> remove panic-assertions in `loose` `lookup_prefix`

### Refactor

- <csr-id-46636e64c9a48ec0e85e014ac0cc8b48846d8462/> flatten errors into one
  By adding one variant, one can remove the previous 'sub-error', for which
  there is no precedent in the codebase yet.

## 0.38.0 (2022-12-19)

### New Features

 - <csr-id-e9d1f45e944e91bb9715a3ee89a4f28b09250411/> support for pack-order when iterating objects.
 - <csr-id-7f19bd7e63d78e3151e43d5094ae9d35cbe34f46/> add `loose::Store::try_header()` to obtain loose object information without content.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.

### New Features (BREAKING)

 - <csr-id-d9d05b0db6b4453e7385117d466bf7c2e8de81fa/> add `Store::try_header()` for obtaining object information quickly.
   Note that this feature also comes with various refactorings related to the error
   type used by various methods in order to get away from a 'one error fits all' kind
   of situation.

## 0.37.0 (2022-11-21)

### Bug Fixes

 - <csr-id-1ce3190000f6211ce31468c7603d491bb5b90293/> Disable tag.gpgSign in test scripts
   This is done for the same reason that commit.gpgsign is disabled for test
   scripts. It prevents test failures if the user has tag.gpgsign enabled in
   their global git config when invoking tests.

### New Features (BREAKING)

 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `gix-features` and `gix-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.

### Bug Fixes (BREAKING)

 - <csr-id-1fabdc51b9468ba2c6b8cf74509ad5aa2a0b86f4/> `alternate::resolve(…)` now takes the current_dir as argument.
   That way it's more consistent with similar low-level functions and it's
   possible to avoid multiple calls to `std::env::current_dir()`.
   
   Furthermore, the usage of `current_dir()` is made explicit when
   instantiating a store to allow it to be reused.

## 0.36.0 (2022-11-08)

A maintenance release without user-facing changes.

## 0.35.0 (2022-11-06)

A maintenance release without user-facing changes.

## 0.34.0 (2022-10-10)

Maintenance release without user-facing changes.

## 0.33.0 (2022-09-20)

Maintenance release without observable changes.

## 0.32.0 (2022-08-28)

Maintenance release without user-facing changes.

## 0.31.2 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

## 0.31.1 (2022-08-17)

### New Features

 - <csr-id-81e1a9d38aac9e6dd0618266ff826593e038cce8/> Add `Cache::has_object_cache()` and `Cache::has_pack_cache()` methods.
   That way it's possible to conditionally set or change the cache size.

### Bug Fixes

 - <csr-id-41d494365d281056c5e9466860db808bd85143e9/> improve error messages when objects aren't found
 - <csr-id-87f974eea2cf7c6e3405b2816d3ef2bd058fc3dc/> incorrect desired object kind if retrieved object doesn't have the

## 0.31.0 (2022-07-22)

This is a maintenance release with no functional changes.

### New Features (BREAKING)

 - <csr-id-95210cb2ba85f75148b4ef48ccea9d9f8a0a0114/> Provide optional `candidates` for ambiguous entries during `lookup_prefix()`
   The candidate entries are all entries matching a given prefix.
 - <csr-id-92d8be1101a7e76e70cd90db6a943b9e31e20802/> `loose::Db` and `Store` can return all candidate objects for a single prefix
   This is the first step towards auto-disambiguating objects in rev-parse.

## 0.30.0 (2022-06-13)

A maintenance release without user-facing changes.

## 0.29.0 (2022-05-18)

A maintenance release without user-facing changes.

## 0.28.0 (2022-04-05)

### New Features

 - <csr-id-84ec54e904378c5b3d7da9efff66b02e88b16916/> Handle::packed_object_count()
   Provide packed objects numbers and cache the value
   for fast access later on.

### Changed (BREAKING)

 - <csr-id-8c5ae77f06a64c57df9a9ad1190266896a223dbe/> Remove deprecated compound and linked object databases
   The dynamic/general store is the only maintained can-do-it-all
   DB now.

## 0.27.0 (2022-04-03)

- fixes a race condition around the first initialization of an ODB, which could leave the loosing
  thread without any packs are loose databases.
- Adds support for lookups by prefix.

### New Features

 - <csr-id-996bfb3061fd9ee2cf38c93f39e0d4c7c6163386/> loose::Store::lookup_prefix(…)

### Bug Fixes

 - <csr-id-9c14de391a1a9f1055922164d1757c9aa9720807/> support Rust 1.52

## 0.26.0 (2022-01-23)

<csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/>
<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/>
<csr-id-2d6960f886c1165f0bdb6f2d653388e1e0b57a2d/>
<csr-id-424c9b3a2b467f5a1e339700257cd4ab72e2e692/>
<csr-id-b1c82a7959fba1541642fc8dfae46b27848f2ba3/>
<csr-id-9235106986e14551a28693bfe4ea92f046c65406/>
<csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/>

### Refactor

- <csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/> replace bare u32 `data::Id` typedef

### Other

- <csr-id-2d6960f886c1165f0bdb6f2d653388e1e0b57a2d/> try LRU-like contains implementation
  Which unfortunately isn't really faster at all even though it totally
  should be.
- <csr-id-424c9b3a2b467f5a1e339700257cd4ab72e2e692/> Try to make Handle usable for pack creation
  It's nearly there, but for some reason the boxed dyn traits don't get to
  be Send even though it's specified.
- <csr-id-b1c82a7959fba1541642fc8dfae46b27848f2ba3/> :Find for Arc and Rc
- <csr-id-9235106986e14551a28693bfe4ea92f046c65406/> :Find implementation for linked::Store

### Chore

- <csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/> remove unused dependencies

### New Features

 - <csr-id-58c2edb76755ab71e10eef4cd9a51533825c291f/> gix_pack::Find::try_find_cached(…, pack_cache)
   With this method it's easier to bypass local caches and control
   the cache oneself entirely.
 - <csr-id-36fde720c34e02429a810ddd43b894a37516f51a/> add linked::Store::rc_iter()
   For completeness in case of single-threaded operations
 - <csr-id-a81b33359a4394a66f854195445f8f9aa0a46179/> linked::Store sorts bundles by modification date, newest first
 - <csr-id-e25f4eadec679406aad6df10026e27e4832c2482/> A simplified version of the `Find` trait
   It's meant for the next generation of object db handles which keep a
   local cache of all the details of the actual object database.

### Bug Fixes

 - <csr-id-9c14de391a1a9f1055922164d1757c9aa9720807/> support Rust 1.52
 - <csr-id-b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e/> linked::Store now assures unique IDs across compound stores

### Changed (BREAKING)

 - <csr-id-91d047658b114f372735116c9d8e6962a3873137/> cleanup and unify `verify_integrity()` method signature
   Previously they used many different ways of handling their parameters
   despite all boiling down to calling the same 'index::File::traverse()`
   method.
   
   This allows for more reuse of `Options` structs and generally makes
   clearer how these options are used.
 - <csr-id-2ef9a8424af51310db8c1e6df31dde9953ed3d21/> Change accessors named `hash_kind()` to `object_hash()` for consistency
 - <csr-id-49998cce419a27f3928ec4ac39da5e3b500e5cb2/> consistently use `object_hash` instead of `hash_kind`
 - <csr-id-67c42fbf5f88f8dc42a9ebd7c6276d57ba1d4624/> remove `Write::*(…, hash_kind)`
   The `hash_kind` is now intrinsic to the implementation of the write
   trait and thus isn't passed along anymore in parameters.
   
   The `sink()` function now takes the kind of hash as parameter.
 - <csr-id-ad1b9ea17eb4b98ebd2fddebe82a8fee1d63e9dd/> various changes to the `loose::Store`.
   
   Change `path` field to read-only `path()` method
   add `hash_kind` parameter to `loose::Store::at(…, hash_kind)`
 - <csr-id-ab4e726fcec65871a81056a9c69af8ea3f56b2a3/> move `sink::Sink` to the top-level exclusively
 - <csr-id-8bb5c9a75cd91ae0d888bc8e93707cfc9cc08090/> move `loose::iter::Iter` to `loose::Iter`
 - <csr-id-3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7/> remove `make_object_cache` parameter from `gix_pack::data::output::count::objects()`
   It now is an implementation detail of the Find trait.
 - <csr-id-580e96c1b2d9782a2e8cf9d1123f6d53a5376a3d/> Rename `Handle` to `Cache`
   Because this is exactly what it is effectively.
   Also add some basic instantiation for the new object store.
- remove pack-cache from `Find::try_find(…)`
  With the new architecture this can be an implementation detail without
  forcing it to be Sync.
- move gix_pack::data::Object to gix_object::Data, massively alter gix_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.

### New Features (BREAKING)

 - <csr-id-bf73a94b43288b6634dbb33f2433656987a73baf/> `Cache::inner` removed in favor of `Deref/Mut` and `into_inner()`
   Making the `inner` field available allows changing it, which would make
   it potentially incompatible with existing caches. The new
   implementation makes it essentially read-only while allowing more
   convenient access to methods on `inner`.

## 0.25.0 (2021-11-29)

<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>

Maintenance release due, which isn't really required but one now has to be careful what's committed at once.

### Refactor (BREAKING)

- move loose header manipulation from gix-pack to gix-object

## 0.24.0 (2021-11-16)

A maintenance release triggered by changes to gix-pack and changelog rewrites.

## v0.23.0 (2021-10-19)

A maintenance release to properly dealing with previously breaking changes in `gix-hash`.

## v0.22.0 (2021-10-15)

### Dependency Upgrade (BREAKING)

* `gix-traverse` saw a breaking change moving to v0.9, which triggered this crate to signal a breaking change, too.

### Improvements

* pack-ids as generated when instantiating a compound database are now sequential. That way, they never clash which
  was possible before when it was using a CRC32 over the filename.

  The latter was done to allow deduplicating packs, but it turns out not to be necessary just yet.

## v0.21.3 (2021-09-10)

## v0.21.2 (2021-09-08)

## v0.21.1 (2021-09-07)

## v0.21.0 (2021-08-27)

## v0.20.2 (2021-08-17)

## v0.20.1 (2021-08-13)

## v0.20.0 (2021-08-12)

## v0.18.0 (2021-08-11)

## v0.17.0 (2021-08-11)

## v0.16.1 (2021-08-10)

## v0.16.0 (2021-08-10)

## v0.15.0 (2021-05-09)

## v0.14.0 (2021-05-02)

## v0.12.0 (2021-04-30)

<csr-id-747a13e9a1fe5200c53055dd961507c9fef667e1/>
<csr-id-4c77e4c97641ab3b02b56aaa702a7d2ca5bced7c/>

### Other

- <csr-id-747a13e9a1fe5200c53055dd961507c9fef667e1/> :borrowed::Object => gix-odb::data::Object
- <csr-id-4c77e4c97641ab3b02b56aaa702a7d2ca5bced7c/> :Db::init() with a few tests

## v0.10.0 (2021-04-08)

<csr-id-d53c4b0f91f1b29769c9430f2d1c0bcab1170c75/>
<csr-id-b317200b72096573d511d229c6e61e74e7ba14db/>
<csr-id-eaae9c1bc723209d793eb93f5587fa2604d5cd92/>

### Other

- <csr-id-d53c4b0f91f1b29769c9430f2d1c0bcab1170c75/> add link to simplified/polonius version in the docs
- <csr-id-b317200b72096573d511d229c6e61e74e7ba14db/> Only check alternates for objects not found in packs or loose
  This matches the behavior of git.
- <csr-id-eaae9c1bc723209d793eb93f5587fa2604d5cd92/> Avoid double-lookup in packs without polonius
  Split object lookup into two steps: looking up the object index, and
  looking up the object itself given the index. This avoids passing in the
  buffer (and thus looking like an unconditional borrow to non-polonius)
  until we're committed to returning from the loop.

## v0.9.1 (2021-04-03)

## v0.9.0 (2021-03-29)

## v0.8.0 (2021-01-24)

## v0.7.1 (2021-01-24)

## v0.7.0 (2020-12-16)

## v0.6.0 (2020-12-15)

## v0.5.0 (2020-12-15)

## v0.4.2 (2020-11-18)

<csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/>

### Other

- <csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/> try to get rid of tree-traversal Boxed error…
  …which really complicates things downstream as these now have to deal
  with another type argument, or of to try to turn it into a Box anyway.

  The latter seems to be…troubling so I can't make it compile.

## v0.4.1 (2020-09-18)

<csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/>
<csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/>

### Other

- <csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/> See if tree compaction saves considerable amounts of memory
  No, it's not worth it.

### Other

- <csr-id-13159eb972ed78ce4ebee2313b288023cec91c47/> try to get rid of tree-traversal Boxed error…
  …which really complicates things downstream as these now have to deal
  with another type argument, or of to try to turn it into a Box anyway.

  The latter seems to be…troubling so I can't make it compile.

## v0.4.0 (2020-09-12)

<csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/>

### Other

- <csr-id-0092c256b3bfaf2818566540e660cdefcf68d246/> See if tree compaction saves considerable amounts of memory
  No, it's not worth it.

## v0.3.0 (2020-08-12)

<csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/>
<csr-id-9945eba749afb020e0deaaa5bb01fda6ff9ccd84/>
<csr-id-cfd8a25f9125c48afe4b66eab6b6ecf71097c486/>
<csr-id-1525f36d29574699d2fcb16b70678121030fd109/>

### Refactor

- <csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/> Use borrowed::Id in trees for full type safety

### Other

- <csr-id-9945eba749afb020e0deaaa5bb01fda6ff9ccd84/> try to use a customized version of just pieces of Miniz-oxide
- <csr-id-cfd8a25f9125c48afe4b66eab6b6ecf71097c486/> fanout table, but slowly I get it :D
- <csr-id-1525f36d29574699d2fcb16b70678121030fd109/> discard idea of making traversal even more generic

## v0.1.0 (2020-07-12)

<csr-id-47ca6ab2ff0cbf8801d0a82cebbbeb8c4f62cdae/>
<csr-id-4ff21686c32a6edc84ea041c3040f33ae24f9519/>
<csr-id-91c8fc1f0c50af55d7cb233bbe813c6d12fe11bc/>

### Refactor

- <csr-id-47ca6ab2ff0cbf8801d0a82cebbbeb8c4f62cdae/> a simpler implementation to skip the header

### Other

- <csr-id-4ff21686c32a6edc84ea041c3040f33ae24f9519/> first silly attempt to randomly remove an allocation
- <csr-id-91c8fc1f0c50af55d7cb233bbe813c6d12fe11bc/> get rid of failure crate in favor of quick-error

