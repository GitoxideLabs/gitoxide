# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.24.0 (2025-12-31)

### Bug Fixes

 - <csr-id-33ff24bfbcc3ddcefd45d2c109474f65d2e11412/> handle `read_link` errors instead of panicking
   Replace unwrap() with proper error handling when reading symlink targets
   in stream_worktree_file. This prevents panics when read_link fails due to
   permissions, race conditions, or I/O errors.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
    - Merge pull request #2312 from GitoxideLabs/copilot/adapt-script-for-gix-test-suite ([`0b29bd3`](https://github.com/GitoxideLabs/gitoxide/commit/0b29bd3d68f8b3a4ae0dbe16866e31d2fb66dfce))
    - Refactor ([`ab67301`](https://github.com/GitoxideLabs/gitoxide/commit/ab67301b29b36d8ddc1bef2d8f533309017525bd))
    - Handle `read_link` errors instead of panicking ([`33ff24b`](https://github.com/GitoxideLabs/gitoxide/commit/33ff24bfbcc3ddcefd45d2c109474f65d2e11412))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
</details>

## 0.23.0 (2025-12-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 29 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
</details>

## 0.22.0 (2025-11-22)

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

## 0.21.1 (2025-10-23)

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

## 0.21.0 (2025-10-22)

### New Features (BREAKING)

 - <csr-id-38b28c6e778a9a35472b3703e72e2e5e1ade3c7c/> Make all conflict entries available in index-worktree status.
   That way it's possible to rely on the status information without the need
   to access the index separately.
   
   Note that to make this work, `PartialOrd|Ord|PartialEq|Eq` had to be removed from the type.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 99 calendar days.
 - 99 days passed between releases.
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
    - Merge pull request #2119 from GitoxideLabs/improvements ([`f3be6e3`](https://github.com/GitoxideLabs/gitoxide/commit/f3be6e380450d6b1e178d2fda0446674429bdfe6))
    - Make all conflict entries available in index-worktree status. ([`38b28c6`](https://github.com/GitoxideLabs/gitoxide/commit/38b28c6e778a9a35472b3703e72e2e5e1ade3c7c))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.20.0 (2025-07-15)

### Bug Fixes

 - <csr-id-376ed0cb602e4df457b0b6c87fe16af027bdff48/> if `core.symlinks=false`, don't misclassify actual symlinks as files.
   Thus, prefer the actual observation over the stored and maybe incorrect filesystem settings.
   This avoids false-positives when checking for changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2016 from GitoxideLabs/improvements ([`7ae3797`](https://github.com/GitoxideLabs/gitoxide/commit/7ae3797f19cf2dd3bc3e02a6437643e5f50ed338))
    - If `core.symlinks=false`, don't misclassify actual symlinks as files. ([`376ed0c`](https://github.com/GitoxideLabs/gitoxide/commit/376ed0cb602e4df457b0b6c87fe16af027bdff48))
    - Merge pull request #2014 from GitoxideLabs/zip ([`648022b`](https://github.com/GitoxideLabs/gitoxide/commit/648022b44e12f597cae55cc45830d0a19b87eb4c))
    - Release gix-glob v0.20.1, gix-attributes v0.26.1, gix-command v0.6.1, gix-filter v0.19.2, gix-worktree-stream v0.21.2, gix-archive v0.21.2 ([`f0ed2cc`](https://github.com/GitoxideLabs/gitoxide/commit/f0ed2cc0046f866e67944bff9aef0579c12d5852))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.19.1 (2025-04-26)

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

## 0.19.0 (2025-04-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.18.0 (2025-04-04)

### New Features

 - <csr-id-9db196b97d1da6005ccf67cb439c1a708e20a77a/> add `SymlinkCheck::verified_path_allow_nonexisting()`
 - <csr-id-7dcba9f0a4fa2cd22d6f356c5f2e2684946ff25a/> make `SymlinkCheck::inner` available.
   That way one can obtain more information about the stack state.

### Changed (BREAKING)

 - <csr-id-4f2b6496109573237f82ee30058cbb93abf3e338/> use separate error type for I/O hashing operations
   Prepare for hashing becoming fallible.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

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
    - Use separate error type for I/O hashing operations ([`4f2b649`](https://github.com/GitoxideLabs/gitoxide/commit/4f2b6496109573237f82ee30058cbb93abf3e338))
    - Merge pull request #1909 from cruessler/take-to-components-in-fs-stack ([`5cb5337`](https://github.com/GitoxideLabs/gitoxide/commit/5cb5337efd7679d8a2ab4bd5e6a5da8c366f7f1a))
    - Use `gix_fs::stack::ToNormalPathComponents` everywhere. ([`1f98edb`](https://github.com/GitoxideLabs/gitoxide/commit/1f98edbaa51caaf152eda289b769388676259a06))
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Drop trailing `,` just before `)` on same line in function calls ([`66a5ae1`](https://github.com/GitoxideLabs/gitoxide/commit/66a5ae1b586d583066402c801213a55141e2aad6))
    - Merge pull request #1898 from GitoxideLabs/improvements ([`7255a5f`](https://github.com/GitoxideLabs/gitoxide/commit/7255a5fc0aa790b54e3176e8ecf066457acd9eef))
    - Add `SymlinkCheck::verified_path_allow_nonexisting()` ([`9db196b`](https://github.com/GitoxideLabs/gitoxide/commit/9db196b97d1da6005ccf67cb439c1a708e20a77a))
    - Improve documentation of a field that one can easily get wrong otherwise. ([`5a1b3d6`](https://github.com/GitoxideLabs/gitoxide/commit/5a1b3d66b161a00c47f35cb5ad92f1c40554e538))
    - Merge pull request #1884 from GitoxideLabs/improvements ([`0bf1d5b`](https://github.com/GitoxideLabs/gitoxide/commit/0bf1d5b9f0b0971b9f25a8e44b7818e37c78d68e))
    - Make `SymlinkCheck::inner` available. ([`7dcba9f`](https://github.com/GitoxideLabs/gitoxide/commit/7dcba9f0a4fa2cd22d6f356c5f2e2684946ff25a))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.17.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>
<csr-id-c4e87454bfd4b92bcf2c8a47a031940cab6c2e18/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Other

 - <csr-id-c4e87454bfd4b92bcf2c8a47a031940cab6c2e18/> inform about tree to index "status"

### Bug Fixes (BREAKING)

 - <csr-id-77ecdb5888b4e0ba20862fb606f8f897900ccf5c/> let the `Change::Type` carry the new type.
   Previously it would just discard that information.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1773 from GitoxideLabs/improvements ([`c2d1a5d`](https://github.com/GitoxideLabs/gitoxide/commit/c2d1a5d1050212ffae8072314f90613c10200b31))
    - Assure that the recently added `worktree_mode` actually represents it. ([`fc2021f`](https://github.com/GitoxideLabs/gitoxide/commit/fc2021fbfc1051b666ea9a2add6225dd9843e772))
    - Merge pull request #1768 from GitoxideLabs/improvements ([`34fa6bb`](https://github.com/GitoxideLabs/gitoxide/commit/34fa6bbcdaafa9a690dd7504c42d286e4dce0fd1))
    - Let the `Change::Type` carry the new type. ([`77ecdb5`](https://github.com/GitoxideLabs/gitoxide/commit/77ecdb5888b4e0ba20862fb606f8f897900ccf5c))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1749 from GitoxideLabs/status ([`8d84818`](https://github.com/GitoxideLabs/gitoxide/commit/8d84818240d44e1f5fe78a231b5d9bffd0283918))
    - Add a `core.autocrlf` (filter) test ([`b67c39d`](https://github.com/GitoxideLabs/gitoxide/commit/b67c39d47350df0eb7dc9a0a2c709b0ae57daae9))
    - Merge pull request #1410 from GitoxideLabs/status ([`0ab4f64`](https://github.com/GitoxideLabs/gitoxide/commit/0ab4f64407b7fa0924830f7b7bd2f5b0ba1cc16e))
    - Inform about tree to index "status" ([`c4e8745`](https://github.com/GitoxideLabs/gitoxide/commit/c4e87454bfd4b92bcf2c8a47a031940cab6c2e18))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.16.0 (2024-12-22)

<csr-id-c06a57e698ee21d6c6d8a35bbb37323ceef1e143/>

### Bug Fixes

 - <csr-id-f3b76d0f52da8b4ea0f4d03490fc0c18fd7c3a96/> Replacing a directory with non-directory will not cause an error anymore.
 - <csr-id-3614c21da2357cb58b7e8572ca49b52656780594/> `status` retrieval now deals with non-file entries by ignoring them when possible.

### Other

 - <csr-id-c06a57e698ee21d6c6d8a35bbb37323ceef1e143/> Clarify and expand descriptions of `NonFile`s
   Discussed in:
   https://github.com/GitoxideLabs/gitoxide/pull/1730#discussion_r1894381449
   
   At least for now, they remain called `NonFile`s (and sometimes
   referred to as "non-files" in text), but more specifically defined.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1735](https://github.com/GitoxideLabs/gitoxide/issues/1735)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1735](https://github.com/GitoxideLabs/gitoxide/issues/1735)**
    - Replacing a directory with non-directory will not cause an error anymore. ([`f3b76d0`](https://github.com/GitoxideLabs/gitoxide/commit/f3b76d0f52da8b4ea0f4d03490fc0c18fd7c3a96))
 * **Uncategorized**
    - Release gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0 ([`beb0ea8`](https://github.com/GitoxideLabs/gitoxide/commit/beb0ea8c4ff94c64b7773772a9d388ccb403f3c1))
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1736 from GitoxideLabs/fix-status ([`3af94f9`](https://github.com/GitoxideLabs/gitoxide/commit/3af94f9ad53404fe053fac47e3a6cf849b8287d9))
    - Merge pull request #1734 from EliahKagan/nonfiles ([`ad6b9b6`](https://github.com/GitoxideLabs/gitoxide/commit/ad6b9b66aa3e3561e413d04d00f6dbf832d63353))
    - Reword "non-files" in documentation comments ([`154b21f`](https://github.com/GitoxideLabs/gitoxide/commit/154b21f0e9beb0e5b6615f091f5f0716df6a3f7b))
    - Adapt to changes in `gix-dir` ([`329a734`](https://github.com/GitoxideLabs/gitoxide/commit/329a734e0f5d23de2bd9af87f6da85cb6154a97c))
    - Clarify and expand descriptions of `NonFile`s ([`c06a57e`](https://github.com/GitoxideLabs/gitoxide/commit/c06a57e698ee21d6c6d8a35bbb37323ceef1e143))
    - Merge pull request #1730 from GitoxideLabs/fix-1729 ([`6822689`](https://github.com/GitoxideLabs/gitoxide/commit/6822689fca04c15e309f9ca41d610bca9cb93e3b))
    - `status` retrieval now deals with non-file entries by ignoring them when possible. ([`3614c21`](https://github.com/GitoxideLabs/gitoxide/commit/3614c21da2357cb58b7e8572ca49b52656780594))
    - Adapt to changes in `gix-dir` ([`a7c4100`](https://github.com/GitoxideLabs/gitoxide/commit/a7c41008705b3c2777b467c60e5b5881021c5abf))
    - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
    - Adapt to changes in `gix-diff` ([`960773e`](https://github.com/GitoxideLabs/gitoxide/commit/960773e5526d02e1f2294224859c821ed86a3463))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.15.0 (2024-11-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge pull request #1652 from EliahKagan/run-ci/chmod ([`8e99eba`](https://github.com/GitoxideLabs/gitoxide/commit/8e99eba2a284b35b5e9bcb97e47bfbbafc3df5d1))
    - Regenerate gix-status fixture archives ([`9ec034f`](https://github.com/GitoxideLabs/gitoxide/commit/9ec034f2abbd5b0a169ac1095001d11e73d6db0a))
    - Set +x in index in other gix-status fixtures ([`c50d730`](https://github.com/GitoxideLabs/gitoxide/commit/c50d73058c64809c7863c1be3bff88a0e91f86d8))
    - Set +x in index in status_changed.sh fixture ([`9b12a78`](https://github.com/GitoxideLabs/gitoxide/commit/9b12a783d6d8460b5095393f2e6bbac472cf94f8))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.14.0 (2024-10-22)

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 60 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
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
    - Adapt to changes in `gix-diff` ([`2bf1e5f`](https://github.com/GitoxideLabs/gitoxide/commit/2bf1e5f15a60ef57c7c15279124d3eb227e585e0))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
</details>

## 0.13.0 (2024-08-22)

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

## 0.12.0 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 28 calendar days.
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
    - Merge branch 'fix-clean' ([`33eacfb`](https://github.com/GitoxideLabs/gitoxide/commit/33eacfbaace2021043e2b5d72dcb9293af6c4020))
    - Adapt to changes in `gix-dir` ([`37c2852`](https://github.com/GitoxideLabs/gitoxide/commit/37c2852a1fdbc13ff9e76db6f60224b4cb5f75ab))
</details>

## 0.11.0 (2024-07-23)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 23 commits contributed to the release over the course of 55 calendar days.
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
    - Merge pull request #1455 from joshtriplett/portable-atomic-only-when-needed ([`6cd8b46`](https://github.com/GitoxideLabs/gitoxide/commit/6cd8b4665bb7582f744c3244abaef812be39ec35))
    - Centralize conditionals for `AtomicU64` import in one place ([`27653cf`](https://github.com/GitoxideLabs/gitoxide/commit/27653cf0e6da98f5b22aef59080119abb41eabcd))
    - Depend on portable-atomic only on platforms without 64-bit atomics ([`42916f6`](https://github.com/GitoxideLabs/gitoxide/commit/42916f6945e302b2b20ca566ac88eac3e21091b0))
    - Merge pull request #1454 from Fabian-Gruenbichler/mr/atomics ([`9ff9642`](https://github.com/GitoxideLabs/gitoxide/commit/9ff96429606e6ad833fe984e756c9186d89e0371))
    - Use portable-atomic for AtomicU64 where needed ([`3639678`](https://github.com/GitoxideLabs/gitoxide/commit/36396782198969ce96ceed3edeae71cf6df6851f))
    - Merge branch 'fix-windows-tests' ([`c2753b8`](https://github.com/GitoxideLabs/gitoxide/commit/c2753b8425c285c6b53f46eba9bc3584aa85eb01))
    - Allow tests to work with symlinks on Windows ([`f48ed0c`](https://github.com/GitoxideLabs/gitoxide/commit/f48ed0c123db7ac6e1b03d012a6eec963873393b))
    - Release gix-actor v0.31.4, gix-object v0.42.3 ([`bf3d82a`](https://github.com/GitoxideLabs/gitoxide/commit/bf3d82abc7c875109f9a5d6b6713ce68153b6456))
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
    - Thanks clippy ([`acc1331`](https://github.com/GitoxideLabs/gitoxide/commit/acc13318731fabac8f65d604baf7e47814f92ad4))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Regenerate archives for changed scripts ([`ea12fc2`](https://github.com/GitoxideLabs/gitoxide/commit/ea12fc234e898eb15013da40d2a82f69c2d20482))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
    - Release gix-fs v0.11.1, gix-glob v0.16.3 ([`2cefe77`](https://github.com/GitoxideLabs/gitoxide/commit/2cefe77203131878d0d8f5346f20f0e25b76cbea))
</details>

## 0.10.0 (2024-05-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 38 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Merge branch 'various-fixes' ([`d6cd449`](https://github.com/GitoxideLabs/gitoxide/commit/d6cd44930fb204b06e2b70fc6965e7705530c47a))
    - Merge pull request from GHSA-7w47-3wg8-547c ([`79dce79`](https://github.com/GitoxideLabs/gitoxide/commit/79dce79c62f6072aa2653780d590dc3993dfa401))
    - Address review comments ([`fcc3b69`](https://github.com/GitoxideLabs/gitoxide/commit/fcc3b69867db1628f6a44d0e0dad8f7417f566bc))
    - Adapt to changes in `gix-worktree` ([`1ca6a3c`](https://github.com/GitoxideLabs/gitoxide/commit/1ca6a3ce22887c7eb42ec3e0a19f6e1202715745))
    - Merge branch 'cargo-fixes' ([`977346e`](https://github.com/GitoxideLabs/gitoxide/commit/977346ee61de6207c66f3de003db6e8c722fb81c))
    - Release gix-index v0.32.1, gix-pathspec v0.7.4, gix-worktree v0.33.1, gix-dir v0.4.1 ([`54ac559`](https://github.com/GitoxideLabs/gitoxide/commit/54ac55946bb04635cd74582a1ce2e4bee70f2e60))
    - Merge pull request #1345 from EliahKagan/shell-scripts ([`fe24c89`](https://github.com/GitoxideLabs/gitoxide/commit/fe24c89e326670deaa3aaa643276d612d866072e))
    - Add missing +x bit on scripts that are run and not sourced ([`41bf65a`](https://github.com/GitoxideLabs/gitoxide/commit/41bf65adef6f7d2cdd28fede262173ec7ba10822))
</details>

## 0.9.0 (2024-04-13)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 9 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
    - Merge pull request #1341 from szepeviktor/typos ([`55f379b`](https://github.com/GitoxideLabs/gitoxide/commit/55f379bc47065822d078393d83d30c0835a89782))
    - Fix typos ([`f72ecce`](https://github.com/GitoxideLabs/gitoxide/commit/f72ecce45babcad2a0c9b73c79d01ff502907a57))
    - Merge branch 'status' ([`45edd2e`](https://github.com/GitoxideLabs/gitoxide/commit/45edd2ea66035adf526cb2f617873dcba60a2a9a))
    - Adapt to changes in `gix-index` ([`1e1fce1`](https://github.com/GitoxideLabs/gitoxide/commit/1e1fce11a968ebbcede1135ccbd0b03e749a1267))
</details>

## 0.8.0 (2024-03-18)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.31.1, gix-object v0.42.1, gix-index v0.31.1, gix-pathspec v0.7.2, gix-dir v0.3.0, gix-status v0.8.0, gix v0.61.0, safety bump 2 crates ([`155cc45`](https://github.com/GitoxideLabs/gitoxide/commit/155cc45730b259e662d7c4be42a469a3af3750e1))
    - Prepare changelog prior to release ([`129ba3d`](https://github.com/GitoxideLabs/gitoxide/commit/129ba3deccc9ada0dc571466458845939502763d))
    - Merge branch 'improvements-for-cargo' ([`41cd53e`](https://github.com/GitoxideLabs/gitoxide/commit/41cd53e2af76e35e047aac4eca6324774df4cb50))
    - Adapt to changes in `gix-dir` ([`e48ba08`](https://github.com/GitoxideLabs/gitoxide/commit/e48ba081ae1c76c05215725647399e0d060b7134))
</details>

## 0.7.0 (2024-03-14)

### New Features

 - <csr-id-2dc373f54988fb38b3a8f1e6100c46ece53b9f95/> provide `Entry::worktree_summary()`
   That way it's possible to more easily and straight-forwardly understand
   the status of an entry, comparing index to worktree.
 - <csr-id-80a6cb80fdb3a49c3461d286d62baf68755aa733/> add index-to-worktree status with rename tracking

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 4 calendar days.
 - 18 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
    - Prepare changelogs prior to release ([`52c3bbd`](https://github.com/GitoxideLabs/gitoxide/commit/52c3bbd36b9e94a0f3a78b4ada84d0c08eba27f6))
    - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
    - Provide `Entry::worktree_summary()` ([`2dc373f`](https://github.com/GitoxideLabs/gitoxide/commit/2dc373f54988fb38b3a8f1e6100c46ece53b9f95))
    - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
    - Add index-to-worktree status with rename tracking ([`80a6cb8`](https://github.com/GitoxideLabs/gitoxide/commit/80a6cb80fdb3a49c3461d286d62baf68755aa733))
</details>

## 0.6.0 (2024-02-25)

### New Features (BREAKING)

 - <csr-id-0bef316696fcf11e9f7dc1910819a60e4229f766/> improve `index_as_worktree()` function signature.
   This is accomplished by providing a context which now contains only
   statically known types, among them also a pathspec search. This allows
   the attribute stack to be passed and can thus cheaply be cloned, instead
   of being created internally.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 30 calendar days.
 - 36 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'status' ([`bb48c4c`](https://github.com/GitoxideLabs/gitoxide/commit/bb48c4ce22650b8c76af3b147e252ebe7cedb205))
    - Improve `index_as_worktree()` function signature. ([`0bef316`](https://github.com/GitoxideLabs/gitoxide/commit/0bef316696fcf11e9f7dc1910819a60e4229f766))
    - Merge branch 'tempfile-permissions' ([`7b44c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7b44c7ff1dc0b8875214d2673c7f52948cf04ff0))
    - Release gix-tempfile v13.1.0, gix-lock v13.1.0, safety bump 12 crates ([`8430442`](https://github.com/GitoxideLabs/gitoxide/commit/84304427dfe4d170c7732161b126961719f70059))
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
</details>

## 0.5.0 (2024-01-20)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 20 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
</details>

## 0.4.1 (2023-12-30)

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

## 0.4.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

 - <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
   Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 19 calendar days.
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
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Check all git-lfs managed files into the repository ([`35439de`](https://github.com/GitoxideLabs/gitoxide/commit/35439defd2d71779d4b3795b7652cde18ff11150))
    - Release gix-hash v0.13.3, gix-index v0.27.1 ([`98b08f4`](https://github.com/GitoxideLabs/gitoxide/commit/98b08f4d0d9237be0e0c2caa9bf5c13ae8bbf9d8))
</details>

## v0.3.0 (2023-12-06)

### Bug Fixes

 - <csr-id-13ab6291eca79b2a5b538d923f7138c3d755d18d/> remove unused dependency and improve documentation slightly

### Changed (BREAKING)

 - <csr-id-7407fec0f73a132e3273c643ede264a401227de1/> use `gix-object::Find` trait

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Adapt to changes in `gix-diff` ([`1706e23`](https://github.com/GitoxideLabs/gitoxide/commit/1706e2394380c35cd98d0e106eb0985ae1912da0))
    - Merge branch 'gix-status' ([`c87f2cc`](https://github.com/GitoxideLabs/gitoxide/commit/c87f2cc7a499cbd354c03c40f9923c80845fc56c))
    - Remove unused dependency and improve documentation slightly ([`13ab629`](https://github.com/GitoxideLabs/gitoxide/commit/13ab6291eca79b2a5b538d923f7138c3d755d18d))
    - Adapt to changes in `gix-index` ([`8134767`](https://github.com/GitoxideLabs/gitoxide/commit/81347676707d30d414b3126c5714c005cca576c8))
    - Merge branch 'gix-object-find' ([`c8bd660`](https://github.com/GitoxideLabs/gitoxide/commit/c8bd66065316176dfbbfe7ecaa092a25cad1854b))
    - Use `gix-object::Find` trait ([`7407fec`](https://github.com/GitoxideLabs/gitoxide/commit/7407fec0f73a132e3273c643ede264a401227de1))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## v0.2.0 (2023-10-12)

### New Features

 - <csr-id-de66b4c26a937a4f6462dff5ec58275dae01813a/> `status` now supports filters.
   This is important as it allows to streaming-read from the worktree and
   correctly change, for example, `git-lfs` files back into their manifests,
   and to arrive at the correct hash.
 - <csr-id-0e10b62557fbd5b33a4aebab24e442e23304ac0a/> a way for `status` to stop early.
   That way, 'is_dirty()` scenarios can be done without wasting too much time.

### New Features (BREAKING)

 - <csr-id-60c948f55ec432ab40b826a9ce8cb3d8fe15a543/> replace `conflict` marker with detailed decoding of stages.
   We also adjust the returned data structure to allow the input to be immutable,
   which delegates entry updates to the caller.
   
   This also paves the way for rename tracking, which requires free access to entries
   for searching renames among the added and removed items, and/or copies among the added ones.
 - <csr-id-b55a8d58b8bd9e1ba2f9049668c166e75fb0a360/> add entries-relative index to each change.
   That way it's possible to lookup other, surrounding entries in case
   of conflicts or easily find entries that didn't change.
 - <csr-id-0d01eb28ebb2305873726ba1892204fd151f4c4f/> provide statistics at the end of a index status operation
 - <csr-id-53de126ea571ef9ed911e66c26a4c36cfdc7e0dd/> add support for submodule status
   Previously, submodules where ignored. Now they are treated correctly
   as 'directory' which is compared to what's in the worktree.
   
   We also simplify blob handling.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 8 calendar days.
 - 17 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.37.1, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0 ([`14ddbd4`](https://github.com/GitoxideLabs/gitoxide/commit/14ddbd4c15128b1d5631a2388a00e024842b7b83))
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Merge branch 'improvements' ([`429e7b2`](https://github.com/GitoxideLabs/gitoxide/commit/429e7b25f93c8a7947db19bafa74babf199a1aa6))
    - Adapt to changes in `gix-object` ([`f712aeb`](https://github.com/GitoxideLabs/gitoxide/commit/f712aeb1fe14ab60c58da5317410e397115f8d35))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Replace `conflict` marker with detailed decoding of stages. ([`60c948f`](https://github.com/GitoxideLabs/gitoxide/commit/60c948f55ec432ab40b826a9ce8cb3d8fe15a543))
    - Add entries-relative index to each change. ([`b55a8d5`](https://github.com/GitoxideLabs/gitoxide/commit/b55a8d58b8bd9e1ba2f9049668c166e75fb0a360))
    - `status` now supports filters. ([`de66b4c`](https://github.com/GitoxideLabs/gitoxide/commit/de66b4c26a937a4f6462dff5ec58275dae01813a))
    - A way for `status` to stop early. ([`0e10b62`](https://github.com/GitoxideLabs/gitoxide/commit/0e10b62557fbd5b33a4aebab24e442e23304ac0a))
    - Provide statistics at the end of a index status operation ([`0d01eb2`](https://github.com/GitoxideLabs/gitoxide/commit/0d01eb28ebb2305873726ba1892204fd151f4c4f))
    - Add support for submodule status ([`53de126`](https://github.com/GitoxideLabs/gitoxide/commit/53de126ea571ef9ed911e66c26a4c36cfdc7e0dd))
    - Add symlink checking for `gix status` ([`c044919`](https://github.com/GitoxideLabs/gitoxide/commit/c044919383b14f9355cf279add64297c2acedeed))
</details>

## v0.1.0 (2023-09-24)

<csr-id-93feea269eebd114e866e6f29f4a73c0096df9e0/>

### Chore

 - <csr-id-93feea269eebd114e866e6f29f4a73c0096df9e0/> split tests off into their own crate to allow feature toggles.
   That way we can test with the `parallel` feature and won't have to
   create bogus feature toggles that are only used for testing, yet visbible
   to users.

### New Features (BREAKING)

 - <csr-id-2d011253c64aaeede546a2e4cdd143142689044b/> various improvements

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 35 calendar days.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Fix gix-status dev dependencies - can't use versions ([`f692809`](https://github.com/GitoxideLabs/gitoxide/commit/f69280929c4ebc3a7677dc3039a61fee6bcfe428))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
    - Merge branch 'reset' ([`54a8495`](https://github.com/GitoxideLabs/gitoxide/commit/54a849545140f7f1c0c7564c418071c0a76a34e7))
    - Various improvements ([`2d01125`](https://github.com/GitoxideLabs/gitoxide/commit/2d011253c64aaeede546a2e4cdd143142689044b))
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
    - Release gix-index v0.23.1 ([`11b9c71`](https://github.com/GitoxideLabs/gitoxide/commit/11b9c71311df978ebb20cca0d765cf249c8eedcf))
    - Release gix-date v0.7.4, gix-index v0.23.0, safety bump 5 crates ([`3be2b1c`](https://github.com/GitoxideLabs/gitoxide/commit/3be2b1ccfe30eeae45711c64b88efc522a2b51b7))
    - Merge branch 'fixes' ([`4bfd1cc`](https://github.com/GitoxideLabs/gitoxide/commit/4bfd1cc8f7922a8c4de6b9d078d54b93e78f51ff))
    - Adapt to changes in `gix-index` and pass skip-hash through for performance.. ([`713cd59`](https://github.com/GitoxideLabs/gitoxide/commit/713cd59f0b1eff6397b80f1e1fceec278532fd59))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
    - More cleanup of test crates ([`73c685a`](https://github.com/GitoxideLabs/gitoxide/commit/73c685a67debcfa26a940f37bbca69cb3a4af57e))
    - Split tests off into their own crate to allow feature toggles. ([`93feea2`](https://github.com/GitoxideLabs/gitoxide/commit/93feea269eebd114e866e6f29f4a73c0096df9e0))
    - Merge branch 'worktree-organization' ([`8d0d8e0`](https://github.com/GitoxideLabs/gitoxide/commit/8d0d8e005d7f11924a6717954d892aae5cec45e7))
    - Adapt to changes in `gix-worktree` ([`e5717e1`](https://github.com/GitoxideLabs/gitoxide/commit/e5717e1d12c49285d31a90b03b7f8e9cbc6c1108))
    - Create new `gix-status` crate to capture `git-status` like functionality ([`be9af32`](https://github.com/GitoxideLabs/gitoxide/commit/be9af327c75d693658a2427ee9a711e631a8da7d))
</details>

