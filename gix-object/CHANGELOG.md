# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 4 calendar days.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
</details>

## 0.54.1 (2026-01-06)

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

## 0.54.0 (2025-12-31)

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

## 0.53.0 (2025-12-22)

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

 - 8 commits contributed to the release.
 - 29 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2277 from GitoxideLabs/dependabot/cargo/cargo-83105039d9 ([`f8435ea`](https://github.com/GitoxideLabs/gitoxide/commit/f8435ea4d12ea453ec750ce003ee70a040836feb))
    - Bump the cargo group with 7 updates ([`6bf895c`](https://github.com/GitoxideLabs/gitoxide/commit/6bf895c586bb557763c2335a48e56521b163a513))
    - Merge pull request #2253 from Pingasmaster/raw-email-attempt-fix ([`f471ac5`](https://github.com/GitoxideLabs/gitoxide/commit/f471ac5fece0baf3766d80482400f6b37db8edd0))
    - Refactor ([`6f7b23a`](https://github.com/GitoxideLabs/gitoxide/commit/6f7b23a4b85eb7b8c922753aa86849893d2a089d))
    - Expose raw commit/tag actor headers for round-tripping. ([`1955b5b`](https://github.com/GitoxideLabs/gitoxide/commit/1955b5b1a436518c036cfc9bf642170f9c7b6540))
    - Merge pull request #2275 from GitoxideLabs/dependabot/cargo/cargo-92eaa62a2e ([`93dd630`](https://github.com/GitoxideLabs/gitoxide/commit/93dd630ca6a2a4622ca74d7eaff42ece2750b6c5))
    - Bump the cargo group across 1 directory with 14 updates ([`703644c`](https://github.com/GitoxideLabs/gitoxide/commit/703644c8821aae161592d19495d3b3162133324f))
</details>

## 0.52.0 (2025-11-22)

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

## 0.51.1 (2025-10-23)

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

## 0.51.0 (2025-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 70 calendar days.
 - 70 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
    - Merge pull request #2208 from cruessler/add-commit-signing ([`8699d0f`](https://github.com/GitoxideLabs/gitoxide/commit/8699d0f010458155f7f5e19c8018ea5e58079963))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Use constant instead of hardcoded value ([`e9cd429`](https://github.com/GitoxideLabs/gitoxide/commit/e9cd4293a719d524f640f845292d06408ec14297))
    - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
    - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
</details>

## 0.50.2 (2025-08-13)

A maintenance release without user-facing changes.

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
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Update changelogs prior to `gix-ref` release ([`5315180`](https://github.com/GitoxideLabs/gitoxide/commit/53151807ec82ce3fbe1838c0885a4f9b71b82f23))
    - Merge pull request #2110 from jpgrayson/fix/gix-date-parse-raw ([`651f9fa`](https://github.com/GitoxideLabs/gitoxide/commit/651f9fa560d5df7260a45068b8440f72820a6ffd))
    - Release gix-date v0.10.5 ([`4289ae6`](https://github.com/GitoxideLabs/gitoxide/commit/4289ae635d94d713d247eaf6f87d0ba91a1a3826))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
</details>

## 0.50.1 (2025-08-03)

Make Commit parsing more lenient so it can read older commits with botched gpg signatures,
see [this commit](https://github.com/GitoxideLabs/gitoxide/commit/ef73c6b6631a3308b7bf5d15b69f4f14767a23d5)
for details.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 19 calendar days.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Prepare changelogs prior to release. ([`5e0122d`](https://github.com/GitoxideLabs/gitoxide/commit/5e0122df48392fb0ea6e16eb7b70d320b03244ca))
    - Merge pull request #2098 from dscho/fix-parsing-of-legacy-git.git-commit ([`ef73c6b`](https://github.com/GitoxideLabs/gitoxide/commit/ef73c6b6631a3308b7bf5d15b69f4f14767a23d5))
    - Refactor ([`8ad43f1`](https://github.com/GitoxideLabs/gitoxide/commit/8ad43f13012c7c8b05887d9f3d32f011d0c46274))
    - Allow empty-valued commit headers in more places ([`a0660fe`](https://github.com/GitoxideLabs/gitoxide/commit/a0660fe3b4227b206dc6df099f55ec5bb41edce0))
    - Work around parse errors of a bogus git.git commit ([`a86f67b`](https://github.com/GitoxideLabs/gitoxide/commit/a86f67b46edec7cd2789ceaaee395a8ecca5c5b0))
    - Merge pull request #2097 from GitoxideLabs/fix-gix-date ([`589d63e`](https://github.com/GitoxideLabs/gitoxide/commit/589d63ed21e5f2cd53ad2cac96fc387df3ea26e9))
    - Release gix-date v0.10.4 ([`007e3f6`](https://github.com/GitoxideLabs/gitoxide/commit/007e3f66246aaafc2374b85cbf77f89ec0b09512))
    - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
    - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.50.0 (2025-07-15)

### New Features

 - <csr-id-c9a312edce09f968ece7ed51a4cf68fdcab43e8a/> add convenience methods for common Git trailers

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2070 from GitoxideLabs/dependabot/cargo/cargo-827bceb7eb ([`dab97f7`](https://github.com/GitoxideLabs/gitoxide/commit/dab97f7618f160421b6e31de8f3e2f3d11dc2ef2))
    - Bump the cargo group across 1 directory with 68 updates ([`a9a8ea1`](https://github.com/GitoxideLabs/gitoxide/commit/a9a8ea1472532dde03bce4e0afdfa82924af1f96))
    - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
    - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Adapt `gix-config`/`gix-object` benches to changes in `criterion` ([`91aef25`](https://github.com/GitoxideLabs/gitoxide/commit/91aef25a9febd440a8c14ce9d73716b5b1de6257))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2030 from ralphmodales/trailer-conviences ([`3820a0f`](https://github.com/GitoxideLabs/gitoxide/commit/3820a0ff32cc543a5f389a20e2bc1c6efd1cd009))
    - Refactor ([`6c6dfd0`](https://github.com/GitoxideLabs/gitoxide/commit/6c6dfd0204b65a03bbe464c312d7decce78e7c98))
    - Add convenience methods for common Git trailers ([`c9a312e`](https://github.com/GitoxideLabs/gitoxide/commit/c9a312edce09f968ece7ed51a4cf68fdcab43e8a))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.49.1 (2025-04-26)

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

## 0.49.0 (2025-04-25)

### Bug Fixes (BREAKING)

<csr-id-a168807b2d354bdcd51f1c4758d2407dab98362f/>

 - <csr-id-c97d1d4e350db9d1499fb58db13afe8ffc9ebae6/> Store the raw time in Commit
 - <csr-id-5bfc2f540aac5be997159e93783d56ed147b0a2f/> Allow round-tripping for Trees
   Change the internal representation of `EntryMode` to fix a round-trip
   issue.
   
   Prior to this change, both `b"040000"` and `b"40000"` mapped to
   `0o40000u16`.
   
   After this change,
   * `b"040000"` is represented by `0o140000`
* `b"40000"` is represented by `0o40000`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 24 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1917](https://github.com/GitoxideLabs/gitoxide/issues/1917)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1917](https://github.com/GitoxideLabs/gitoxide/issues/1917)**
    - Refactor ([`103819d`](https://github.com/GitoxideLabs/gitoxide/commit/103819dbe2d86790a6ac4c6be3f21e5d4455dfd2))
 * **Uncategorized**
    - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
    - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
    - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
    - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Adapt to changes in `gix-date` and `gix-actor` ([`afdf1a5`](https://github.com/GitoxideLabs/gitoxide/commit/afdf1a5d5c9fb2645f481c17f580ad59d14d6095))
    - Apply feedback from discussion ([`70097c0`](https://github.com/GitoxideLabs/gitoxide/commit/70097c0feb481541ed96358842de96d6b1af24a9))
    - Store the raw time in Commit ([`c97d1d4`](https://github.com/GitoxideLabs/gitoxide/commit/c97d1d4e350db9d1499fb58db13afe8ffc9ebae6))
    - Merge pull request #1968 from GitoxideLabs/dependabot/cargo/cargo-bd18780e40 ([`46227e6`](https://github.com/GitoxideLabs/gitoxide/commit/46227e6d1ddc0879662730e5bb21a8597716b1ca))
    - Bump the cargo group with 40 updates ([`06bf1e1`](https://github.com/GitoxideLabs/gitoxide/commit/06bf1e1552de65ce692911bdc4c501d487bbc3d7))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1922 from pierrechevalier83/make_email_with_spaces_roundtrip ([`c13a403`](https://github.com/GitoxideLabs/gitoxide/commit/c13a403e8f306430220c209b1024d408c3c0a4f8))
    - Adapt to changes in gix-actor. ([`38e1055`](https://github.com/GitoxideLabs/gitoxide/commit/38e1055ae8b7d9ab630f70d5e1e50928ab7cda83))
    - Merge pull request #1917 from pierrechevalier83/issue_1887 ([`6307f57`](https://github.com/GitoxideLabs/gitoxide/commit/6307f571f969eb7ff2490e4c68dc7994fb2fecac))
    - Allow round-tripping for Trees ([`5bfc2f5`](https://github.com/GitoxideLabs/gitoxide/commit/5bfc2f540aac5be997159e93783d56ed147b0a2f))
    - Hide the internal representation of EntryMode ([`a168807`](https://github.com/GitoxideLabs/gitoxide/commit/a168807b2d354bdcd51f1c4758d2407dab98362f))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.48.0 (2025-04-04)

<csr-id-866affde8ef17f201884b8a4b36cc4c7f449d6fe/>

### New Features

 - <csr-id-bf851283f9d451a2a5c55a44c0474126baa2b8fb/> add `commit::ExtraHeaders::find_pos()`, and expose "gpgsig" header name.
   That way it it's easier to manipulate existing extra-header fields.
 - <csr-id-527fd3d19a307fe878fd8e3ca54ed10140ac6fa7/> add `CommitRef|TagRef|BlobRef::into_owned()` as shortcut.
   Otherwise one would have to know it converts into `Commit` via `From`.

### Bug Fixes

 - <csr-id-f69890cafd08db95231a02d08fbbcb02689e99c4/> Support empty tags with or without trailing NL
   When representing an annotated tag with an empty commit message, we used
   to only support a tag ending with two newlines (one after the tagger
   line + one after the empty commit message).
   
   This was due to a misconception that annotated tags ending with a single
   NL shouldn't exist in the wild since there isn't an obvious way to
   create them from with `git`.
   This misconception shows up in the discussion thread for
   [issue 603](https://github.com/GitoxideLabs/gitoxide/issues/603)
   
   It turns out that both encodings of empty annotated tags appear in the
   wild.
   
   We must be able to parse either and roundtrip for either.
   
   Before [PR 604](https://github.com/GitoxideLabs/gitoxide/pull/604), we
   used to special case the empty tag msg case and not add a NL.
   To be able to represent `b""`, `b"\n"`, `b"\n\n"`..., we special case
   any `message` that is a pure sequence of `b'\n'` and actually parse the
   `b'\n'` into the tag's message.
   
   This allows us to calculate the correct size that matches the number of
   bytes that git would produce, as well as round-trip to and from the
   commit encoding.
   
   The existing tests (in particular `round_trip` for `empty.txt` and
   `empty_missing_nl.txt`) convince me that the logic is sound.
   Also, the size being `139` bytes for `empty_missing_nl.txt` and `140`
   bytes for `empty.txt` matches the output of `cat file | wc -l` for each
   file.

### Other

 - <csr-id-866affde8ef17f201884b8a4b36cc4c7f449d6fe/> `Repository::commit()` now explains how to create a commit without ref updates.

### Changed (BREAKING)

 - <csr-id-a68f1156b4a24e49ceb165bdaa9f4fa8e4c70cd9/> drop migration shims for fallible hashing
   Since the APIs were already adjusted and all callers migrated, we
   only need to drop the migration shims.
 - <csr-id-b5ac93a9a02c97bd611d455091c60bfbd5f49efa/> make `gix_object::compute_hash` fallible
   `compute_stream_hash` is already fallible, so we don’t want to keep
   the `try_*` prefix on the fallible API.
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

 - 34 commits contributed to the release.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Drop migration shims for fallible hashing ([`a68f115`](https://github.com/GitoxideLabs/gitoxide/commit/a68f1156b4a24e49ceb165bdaa9f4fa8e4c70cd9))
    - Migrate `gix_object::{try_ =>}compute_hash` users ([`3d7e379`](https://github.com/GitoxideLabs/gitoxide/commit/3d7e379f26cbe53ddb430427b8e88ce0966be456))
    - Make `gix_object::compute_hash` fallible ([`b5ac93a`](https://github.com/GitoxideLabs/gitoxide/commit/b5ac93a9a02c97bd611d455091c60bfbd5f49efa))
    - Migrate hashing API users to fallible versions ([`fbf6cc8`](https://github.com/GitoxideLabs/gitoxide/commit/fbf6cc897cfeff5ed2a2d5946c060e0cebbd1afd))
    - Add fallible `gix_object::try_compute_hash` for migration ([`f2b07c0`](https://github.com/GitoxideLabs/gitoxide/commit/f2b07c0783ac98a08f648836e743cb0e6b4fdc7c))
    - Adjust error return types to handle collision detection ([`5095f44`](https://github.com/GitoxideLabs/gitoxide/commit/5095f44db58014f4a35ea8996a90d56d2ac19d45))
    - Use separate error type for I/O hashing operations ([`4f2b649`](https://github.com/GitoxideLabs/gitoxide/commit/4f2b6496109573237f82ee30058cbb93abf3e338))
    - Adjust hash verification return types for the common interface ([`54e5764`](https://github.com/GitoxideLabs/gitoxide/commit/54e57649f0e0b15c0bd1d3233e41524cb91a8cb9))
    - Migrate all hashing API users to `gix_hash::Hasher::finalize()` ([`4e935ce`](https://github.com/GitoxideLabs/gitoxide/commit/4e935ce167428581f7e0351768b705164f71179a))
    - Migrate all hashing API users to `gix_hash` ([`baa1430`](https://github.com/GitoxideLabs/gitoxide/commit/baa1430aed0dc8160a71cc675e2626780a2de052))
    - Use `gix_object::compute_hash` in a test and benchmark ([`7805ffe`](https://github.com/GitoxideLabs/gitoxide/commit/7805ffe7ac0af6a413b491efeacdd7e4616df86b))
    - Factor out private `gix_object::object_hasher` helper ([`cf261d5`](https://github.com/GitoxideLabs/gitoxide/commit/cf261d5b0ef647d389fce7c901d076cd9f9dd8e2))
    - Merge pull request #1908 from EliahKagan/run-ci/scripts ([`c8c42b4`](https://github.com/GitoxideLabs/gitoxide/commit/c8c42b4b86e8bf7d8f0f7130d2da98dfed246be9))
    - Fix a few ShellCheck warnings and stylistic inconsistencies ([`e5e2c6f`](https://github.com/GitoxideLabs/gitoxide/commit/e5e2c6fbf9337219edb79ce97b56b3be91bc14e5))
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Use raw literals for more strings with backslashes ([`01bd76d`](https://github.com/GitoxideLabs/gitoxide/commit/01bd76dcacb69d9c21f2fc6063e273a01aebf94f))
    - Merge pull request #1903 from pierrechevalier83/tag_end_of_line ([`6caee49`](https://github.com/GitoxideLabs/gitoxide/commit/6caee4922c96c864a28eb9b1b52a013d2c11cd4e))
    - Support empty tags with or without trailing NL ([`f69890c`](https://github.com/GitoxideLabs/gitoxide/commit/f69890cafd08db95231a02d08fbbcb02689e99c4))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1837 from GitoxideLabs/improvements ([`b4fe425`](https://github.com/GitoxideLabs/gitoxide/commit/b4fe425a1a7823790fab592c84aa8494d295640d))
    - Add `commit::ExtraHeaders::find_pos()`, and expose "gpgsig" header name. ([`bf85128`](https://github.com/GitoxideLabs/gitoxide/commit/bf851283f9d451a2a5c55a44c0474126baa2b8fb))
    - `Repository::commit()` now explains how to create a commit without ref updates. ([`866affd`](https://github.com/GitoxideLabs/gitoxide/commit/866affde8ef17f201884b8a4b36cc4c7f449d6fe))
    - Add `CommitRef|TagRef|BlobRef::into_owned()` as shortcut. ([`527fd3d`](https://github.com/GitoxideLabs/gitoxide/commit/527fd3d19a307fe878fd8e3ca54ed10140ac6fa7))
    - Merge pull request #1822 from epage/w7 ([`11ac79c`](https://github.com/GitoxideLabs/gitoxide/commit/11ac79c068181d4ed9f6a404e4875ad7c206520c))
    - Upgrade to Winnow 0.7 ([`fdc57e7`](https://github.com/GitoxideLabs/gitoxide/commit/fdc57e79af6f7922d91ad8d7796943821f637124))
    - Switch to ModalParser ([`0e71412`](https://github.com/GitoxideLabs/gitoxide/commit/0e71412e9b92166a5adcd2d9685d7c3ddbbd7014))
    - Resolve Winnow deprecations ([`3cd3e2a`](https://github.com/GitoxideLabs/gitoxide/commit/3cd3e2a71beb01591afe732ab4ae914ed62a4ecf))
    - Upgrade to Winnow 0.6.26 ([`783c4e6`](https://github.com/GitoxideLabs/gitoxide/commit/783c4e698234b8afaf8fbd25057aca11c5c66e75))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.47.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### New Features

 - <csr-id-9f2664533b0e44fdebe72102e6f2ab3410e1265d/> add conversion from `tree::Entry` to `EntryRef` with `From`
 - <csr-id-8141765fcfaebe7eab28cb41be5cae7bb948d46a/> add `TreeRefIter::offset_to_next_entry()`.
   Add a function to help resume the iterator without holding onto the data directly.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1746 from GitoxideLabs/status ([`af704f5`](https://github.com/GitoxideLabs/gitoxide/commit/af704f57bb9480c47cdd393465264d586f1d4562))
    - Add conversion from `tree::Entry` to `EntryRef` with `From` ([`9f26645`](https://github.com/GitoxideLabs/gitoxide/commit/9f2664533b0e44fdebe72102e6f2ab3410e1265d))
    - Merge pull request #1410 from GitoxideLabs/status ([`0ab4f64`](https://github.com/GitoxideLabs/gitoxide/commit/0ab4f64407b7fa0924830f7b7bd2f5b0ba1cc16e))
    - Add `TreeRefIter::offset_to_next_entry()`. ([`8141765`](https://github.com/GitoxideLabs/gitoxide/commit/8141765fcfaebe7eab28cb41be5cae7bb948d46a))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.46.1 (2024-12-22)

A maintenance release without user-facing changes.

But actually, `TreeRefIter` now has methods to lookup entries, also recursively.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1686 from cruessler/move-lookup-entry-to-gix-object ([`39227a9`](https://github.com/GitoxideLabs/gitoxide/commit/39227a90ca4590b08dc7d782728be2e9a3054618))
    - Refactors ([`d7f4991`](https://github.com/GitoxideLabs/gitoxide/commit/d7f49916037efb0c95cf1a4d58be215bee67eb0d))
    - Add test for entry that does not exist ([`54e399f`](https://github.com/GitoxideLabs/gitoxide/commit/54e399f38002689eb2694c2d59799fc4c2f019ff))
    - Match on complete entry ([`08a86dc`](https://github.com/GitoxideLabs/gitoxide/commit/08a86dc39edb5b57247faf069bbc332126f41f66))
    - Deduplicate tests ([`2de935d`](https://github.com/GitoxideLabs/gitoxide/commit/2de935dc1fbd22eb9f98eb145a3160c3b5d48b7f))
    - Replace FindExt by Find ([`579a6f2`](https://github.com/GitoxideLabs/gitoxide/commit/579a6f217290fb5b6b6d10b4c0414ea87012235e))
    - Move lookup_entry, lookup_entry_by_path to TreeRefIter ([`32632e9`](https://github.com/GitoxideLabs/gitoxide/commit/32632e94530b6fd6e10cd05d61d3bbbda5ccaa00))
    - Remove superfluous trait bound ([`1c6e4ae`](https://github.com/GitoxideLabs/gitoxide/commit/1c6e4ae103abc56352ff0e596b2236eab1809e60))
    - Don't recurse, use loop instead ([`7e66a4a`](https://github.com/GitoxideLabs/gitoxide/commit/7e66a4ab83d9532dba39e0d76e7d6dc98918d885))
    - Sketch recursive lookup ([`0cf6d3a`](https://github.com/GitoxideLabs/gitoxide/commit/0cf6d3a307caa411f34337a0384a104489d1490a))
    - Add lookup_entry, lookup_entry_by_path to TreeRef ([`d121fe7`](https://github.com/GitoxideLabs/gitoxide/commit/d121fe7ac850964aeafa8b27070b4514c5648016))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.46.0 (2024-11-24)

### New Features

 - <csr-id-cd61c25369d3e39b6160bac4b332b177dabddf4b/> octal Debug representation of `tree::EntryMode`.
   This makes it easier to reason about.
 - <csr-id-29aad45ec93b19aca6aca06bc525277e0fc0d728/> `FindExt` now supports empty trees and empty blobs natively.
 - <csr-id-e2ea398d1fb789cb180a1cf9d81f2d9fbb010927/> Add `tree::Editor::get()` to get entries directly from the editor.
   This is useful if in the middle of an edit you'd like to lookup what's there
   to choose a non-conflicting name, for example.

### New Features (BREAKING)

 - <csr-id-7325c584c3f0973975935b307630e29bc36ef5c7/> move `gix-odb::Write` trait to `gix-object::Write`.
   There it's far more useful and plumbing crates are enabled to write
   objects without pulling in `gix-odb` as dependency.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1661 from GitoxideLabs/merge ([`0b7abfb`](https://github.com/GitoxideLabs/gitoxide/commit/0b7abfbdebe8c5ab30b89499a70dd7727de41184))
    - Octal Debug representation of `tree::EntryMode`. ([`cd61c25`](https://github.com/GitoxideLabs/gitoxide/commit/cd61c25369d3e39b6160bac4b332b177dabddf4b))
    - Merge pull request #1690 from EliahKagan/gitattributes ([`700cfa5`](https://github.com/GitoxideLabs/gitoxide/commit/700cfa52e7f3008036881a99fbdeb04c9ab1f2f5))
    - Fix marking `gix-packetline-blocking` copy auto-generated ([`325588e`](https://github.com/GitoxideLabs/gitoxide/commit/325588e700ec159e5139beb149127c947164fb98))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
    - Adapt to changes in `gix-object` and `gix-odb` ([`96488f7`](https://github.com/GitoxideLabs/gitoxide/commit/96488f745506698e4183e8e544c4e0ecd1ef868b))
    - Move `gix-odb::Write` trait to `gix-object::Write`. ([`7325c58`](https://github.com/GitoxideLabs/gitoxide/commit/7325c584c3f0973975935b307630e29bc36ef5c7))
    - `FindExt` now supports empty trees and empty blobs natively. ([`29aad45`](https://github.com/GitoxideLabs/gitoxide/commit/29aad45ec93b19aca6aca06bc525277e0fc0d728))
    - Add `tree::Editor::get()` to get entries directly from the editor. ([`e2ea398`](https://github.com/GitoxideLabs/gitoxide/commit/e2ea398d1fb789cb180a1cf9d81f2d9fbb010927))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.45.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>

### New Features

 - <csr-id-7c485561582c583ce0d929680e7f9abf5ceeea60/> `TreeRef::to_owned()` and `Tree::into_owned()` for a convenient way to get a fully-owned `Tree`
 - <csr-id-14dfcf01dc4a6e48fbd363f4f1c8cf08c1e7f6e2/> add `tree::Editor`
   With it it's easy to alter existing trees or build entirely new ones,
   efficiently.
 - <csr-id-71bf80883e010f242460e9e36d5ea27a14e6494b/> add `Default` implementation to `Tree`

### Bug Fixes

 - <csr-id-4a6bbb1b79991f400be814c68fb45e8e5980a00c/> keep multi-line header value newlines verbatim
   Previously they were normalized to \n which made round-trips
   degenerate information.
   
   It's notable that we explicitly do NOT normalize the newline
   at the very last line of the multi-line header as its part
   of that line, even though it's also relevant to make the header field
   valid. The reason is that these *can* also be written with \r\n, and
   we don't want to degenerate that information.
 - <csr-id-cdac4a9b04959b0fc71009b5c828cdcb10a38828/> remove workspace lints from Cargo manifests of fuzz-projects.
   They are not part of the parent workspace.

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

 - <csr-id-a085b7e435317f566f6763f4155243ba3309cd82/> `Tree|TreeRef::write_to()` will now assure the most basic sanity of entry names.
   Previously, it would allow null-bytes in the name which would corrupt the written tree.
   Now this is forbidden.
   For some reason, it disallowed newlines, but that is now allowed as validation is should
   be handled on a higher level.
 - <csr-id-17573779688e755a786546d5e42ab533088cd726/> remove all workspace dependencies
   The problem is that with them, we don't notice anymore if the crate changes,
   because a dependency changes. That also means that older versions of the dependency
   may stay even though some other crates might pick up a newer version.
   
   Ultimately, this will lead to drift and subtle incompatibilities.
   
   We declare this breaking to enforce a proper re-release.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 36 commits contributed to the release over the course of 60 calendar days.
 - 60 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1542](https://github.com/GitoxideLabs/gitoxide/issues/1542)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1542](https://github.com/GitoxideLabs/gitoxide/issues/1542)**
    - Be lenient to missing timestamp in signature ([`059c9f9`](https://github.com/GitoxideLabs/gitoxide/commit/059c9f9ece4edf631e3ca5f8298ce2660cba946b))
    - Add test for tag missing timestamp in tagger ([`ce5d79c`](https://github.com/GitoxideLabs/gitoxide/commit/ce5d79c181babfdb8aa4a0c4cc1db29bfb1dc21a))
 * **Uncategorized**
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Merge pull request #1617 from Byron/commit-roundtrip ([`f35b109`](https://github.com/GitoxideLabs/gitoxide/commit/f35b1096c6db73842a55e089187d27d1287075ad))
    - Modernize integration tests ([`7fcf91a`](https://github.com/GitoxideLabs/gitoxide/commit/7fcf91aeeb05dc628979d3d0422922fd77592729))
    - Keep multi-line header value newlines verbatim ([`4a6bbb1`](https://github.com/GitoxideLabs/gitoxide/commit/4a6bbb1b79991f400be814c68fb45e8e5980a00c))
    - Reproduce roundtrip failure for commit that looks good otherwise ([`9db3f9c`](https://github.com/GitoxideLabs/gitoxide/commit/9db3f9c93edf9316c71a47a103cdf4047e9284b0))
    - Merge pull request #1593 from Byron/fix-fuzz ([`72daa46`](https://github.com/GitoxideLabs/gitoxide/commit/72daa46bad9d397ef2cc48a3cffda23f414ccd8a))
    - Remove workspace lints from Cargo manifests of fuzz-projects. ([`cdac4a9`](https://github.com/GitoxideLabs/gitoxide/commit/cdac4a9b04959b0fc71009b5c828cdcb10a38828))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - `TreeRef::to_owned()` and `Tree::into_owned()` for a convenient way to get a fully-owned `Tree` ([`7c48556`](https://github.com/GitoxideLabs/gitoxide/commit/7c485561582c583ce0d929680e7f9abf5ceeea60))
    - Use a standard HashMap instead of one based on SHA1 ([`39d8e03`](https://github.com/GitoxideLabs/gitoxide/commit/39d8e03047df09db0102246071015505652bec8b))
    - Add benchmarks for the tree editor and the tree-editor cursor ([`b5dc45f`](https://github.com/GitoxideLabs/gitoxide/commit/b5dc45f2854012981cb6071f511e5d987020a003))
    - `Tree|TreeRef::write_to()` will now assure the most basic sanity of entry names. ([`a085b7e`](https://github.com/GitoxideLabs/gitoxide/commit/a085b7e435317f566f6763f4155243ba3309cd82))
    - Add `tree::Editor::cursor()` to allow speedier creation of sub-trees. ([`91fa478`](https://github.com/GitoxideLabs/gitoxide/commit/91fa4786df7fe25f2da3d78ef0fc14d53b69b846))
    - Add `tree::Editor` ([`14dfcf0`](https://github.com/GitoxideLabs/gitoxide/commit/14dfcf01dc4a6e48fbd363f4f1c8cf08c1e7f6e2))
    - Add `Default` implementation to `Tree` ([`71bf808`](https://github.com/GitoxideLabs/gitoxide/commit/71bf80883e010f242460e9e36d5ea27a14e6494b))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Merge branch 'issue-1542' ([`c3f173c`](https://github.com/GitoxideLabs/gitoxide/commit/c3f173c94968907ecc685c5383f2fe24c02dcfbf))
    - Make binary-search based test more robust ([`1b600f3`](https://github.com/GitoxideLabs/gitoxide/commit/1b600f31db5b5061764e7a845ac24def57faf144))
    - Adjust expectation now that author parsing is more lenient. ([`bfd5009`](https://github.com/GitoxideLabs/gitoxide/commit/bfd50091d7ec5784f0af8581d1c14c463e9e9417))
    - Merge pull request #1540 from NobodyXu/patch-1 ([`f4898b9`](https://github.com/GitoxideLabs/gitoxide/commit/f4898b986741934d0f5366ef9cf175446f621cec))
    - Fix winnow version requirement in gix-object ([`0a3ce48`](https://github.com/GitoxideLabs/gitoxide/commit/0a3ce48a1749247c0aa78988e2876acc5f230dad))
    - Merge branch 'fixes' ([`46cd1ae`](https://github.com/GitoxideLabs/gitoxide/commit/46cd1aed7815d27cdc818edb87641b20b82ba048))
    - Remove all workspace dependencies ([`1757377`](https://github.com/GitoxideLabs/gitoxide/commit/17573779688e755a786546d5e42ab533088cd726))
</details>

## 0.44.0 (2024-08-22)

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

## 0.43.0 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 24 calendar days.
 - 49 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
    - Prepare changelogs prior to release ([`0f25841`](https://github.com/GitoxideLabs/gitoxide/commit/0f2584178ae88e425f1c629eb85b69f3b4310d9f))
    - Merge branch 'dependabot/github_actions/github-actions-c4bcf5a8e2' ([`2e00b5e`](https://github.com/GitoxideLabs/gitoxide/commit/2e00b5ef6e8a15e7f0a34d54739a5cd1c986b322))
    - Make `winnow` a workspace dependency ([`78a7e32`](https://github.com/GitoxideLabs/gitoxide/commit/78a7e32c34150dece4065e513cd177356619419f))
    - Merge branch 'improvements' ([`12313f2`](https://github.com/GitoxideLabs/gitoxide/commit/12313f2720bb509cb8fa5d7033560823beafb91c))
    - Thanks clippy ([`ae2b733`](https://github.com/GitoxideLabs/gitoxide/commit/ae2b733c1e4de33ddf442cd8044dae364307085d))
    - Merge branch 'upgrades' ([`1d37bf6`](https://github.com/GitoxideLabs/gitoxide/commit/1d37bf6a773d56eea9003aa626ced413e8e0eaa3))
    - Update all dependencies and fix deprecations ([`f5cd3ba`](https://github.com/GitoxideLabs/gitoxide/commit/f5cd3baf57676bfee0c40fee577d2779958bbe72))
    - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
    - Assure the next release is breaking ([`9fd1090`](https://github.com/GitoxideLabs/gitoxide/commit/9fd10905449a41cdda5eb2764e4d45d314de9c04))
</details>

## 0.42.3 (2024-07-03)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 42 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1438](https://github.com/GitoxideLabs/gitoxide/issues/1438)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1438](https://github.com/GitoxideLabs/gitoxide/issues/1438)**
    - Reproduce failure when parsing malformed commit ([`6c5861d`](https://github.com/GitoxideLabs/gitoxide/commit/6c5861dff0986ca68c474be65738eda47ca28927))
 * **Uncategorized**
    - Release gix-actor v0.31.4, gix-object v0.42.3 ([`bf3d82a`](https://github.com/GitoxideLabs/gitoxide/commit/bf3d82abc7c875109f9a5d6b6713ce68153b6456))
    - Prepare changelogs prior to release ([`255920e`](https://github.com/GitoxideLabs/gitoxide/commit/255920ecffd47f221702aaec29de966b120f8fc5))
    - Merge branch 'fix-1438' ([`9717a25`](https://github.com/GitoxideLabs/gitoxide/commit/9717a255b7c817c9f6cde44eedba232e309a6e0f))
    - Adapt to changes in `gix-actor` ([`8dca2d4`](https://github.com/GitoxideLabs/gitoxide/commit/8dca2d4de9f40839148fe545cf2f52d2fb9fa496))
    - Release gix-actor v0.31.3, gix-mailmap v0.23.4 ([`1e79c5c`](https://github.com/GitoxideLabs/gitoxide/commit/1e79c5cdf20fc0440e9a497c9d01b0c0ca3ce424))
    - Release gix-date v0.8.7, gix-mailmap v0.23.2 ([`c1d7c02`](https://github.com/GitoxideLabs/gitoxide/commit/c1d7c023d595eb04891b65295f001d85c9ba8074))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Regenerate archives for changed scripts ([`ea12fc2`](https://github.com/GitoxideLabs/gitoxide/commit/ea12fc234e898eb15013da40d2a82f69c2d20482))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
</details>

## 0.42.2 (2024-05-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 8 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Release gix-date v0.8.6 ([`d3588ca`](https://github.com/GitoxideLabs/gitoxide/commit/d3588ca4fe0364c88e42cdac24ceae548355d99d))
</details>

## 0.42.1 (2024-03-18)

### Bug Fixes

 - <csr-id-ab210836ac4558c88cc5f19fdddfb5c8eeb29754/> allow parsing of commits with broken timestamp.
   Instead, default to UTC just like Git.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1322](https://github.com/GitoxideLabs/gitoxide/issues/1322)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1322](https://github.com/GitoxideLabs/gitoxide/issues/1322)**
    - Allow parsing of commits with broken timestamp. ([`ab21083`](https://github.com/GitoxideLabs/gitoxide/commit/ab210836ac4558c88cc5f19fdddfb5c8eeb29754))
 * **Uncategorized**
    - Release gix-actor v0.31.1, gix-object v0.42.1, gix-index v0.31.1, gix-pathspec v0.7.2, gix-dir v0.3.0, gix-status v0.8.0, gix v0.61.0, safety bump 2 crates ([`155cc45`](https://github.com/GitoxideLabs/gitoxide/commit/155cc45730b259e662d7c4be42a469a3af3750e1))
    - Prepare changelog prior to release ([`129ba3d`](https://github.com/GitoxideLabs/gitoxide/commit/129ba3deccc9ada0dc571466458845939502763d))
    - Merge branch 'improvements-for-cargo' ([`41cd53e`](https://github.com/GitoxideLabs/gitoxide/commit/41cd53e2af76e35e047aac4eca6324774df4cb50))
</details>

## 0.42.0 (2024-03-14)

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

## 0.41.1 (2024-02-25)

### New Features

 - <csr-id-82e85fd192f59bc62bb27de13360a543971e8167/> Allow the creation of any `EntryMode`.
   This helps testing for one, but might also be useful in other contexts.
   After all, this is a plumbing crate.

### Bug Fixes

 - <csr-id-67833dfd18c49ed8073e7a3c0461579c0a5c2017/> `EntryMode` canonicalization
   This means that `mode.kind()` and `mode.is_*` will also work correctly
   if the underlying mode wasn't canonicalized, which could happen when
   looking at trees that were stored with very old versions of Git.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release over the course of 34 calendar days.
 - 36 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1259](https://github.com/GitoxideLabs/gitoxide/issues/1259)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1259](https://github.com/GitoxideLabs/gitoxide/issues/1259)**
    - Allow the creation of any `EntryMode`. ([`82e85fd`](https://github.com/GitoxideLabs/gitoxide/commit/82e85fd192f59bc62bb27de13360a543971e8167))
    - `EntryMode` canonicalization ([`67833df`](https://github.com/GitoxideLabs/gitoxide/commit/67833dfd18c49ed8073e7a3c0461579c0a5c2017))
 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'btoi' ([`5fc379d`](https://github.com/GitoxideLabs/gitoxide/commit/5fc379d1dc867d15a50cb086e30beefde2b42d86))
    - Refactor ([`c5c69bd`](https://github.com/GitoxideLabs/gitoxide/commit/c5c69bd355771a6fb3e4f6db0c5f49aa2bf7f42f))
    - Inline btoi code to reduce compile times ([`f26f298`](https://github.com/GitoxideLabs/gitoxide/commit/f26f2988f51f6c419ec7eff4ae6f4df0f4011663))
    - Merge pull request #1290 from epage/winnow ([`a663e9f`](https://github.com/GitoxideLabs/gitoxide/commit/a663e9fcdb5a3aedc9200da77ebae17d5c3e7135))
    - Update winnow to 0.6 ([`e175b20`](https://github.com/GitoxideLabs/gitoxide/commit/e175b20d431faa6859fbcc52f78400e50f91cad1))
    - Update winnow to 0.5.40 ([`516e105`](https://github.com/GitoxideLabs/gitoxide/commit/516e105db5f22e1483b4b8a886cc4f3929ad7f6a))
    - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
    - Adjust gitignore files with precious declarations ([`ae86a6a`](https://github.com/GitoxideLabs/gitoxide/commit/ae86a6a206074b85ff1eba32aea9c8b40c087b17))
    - Merge pull request #1267 from epage/winnow ([`69cb78b`](https://github.com/GitoxideLabs/gitoxide/commit/69cb78bd865a372c580b386766d7b61e5ca9303a))
    - Move off deprecated take_until[01] ([`52ede06`](https://github.com/GitoxideLabs/gitoxide/commit/52ede0634067de81d7db66728257d8242c14654e))
    - Update from winnow 0.5.31 to 0.5.36 ([`9470554`](https://github.com/GitoxideLabs/gitoxide/commit/94705546cf0e4c8e38bcc96999cfa79cd8ee1acd))
    - Merge branch 'entry-mode-fixes' ([`35d2083`](https://github.com/GitoxideLabs/gitoxide/commit/35d2083e242314d623c3e79f68fa8ac03bf4355a))
</details>

## 0.41.0 (2024-01-20)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 20 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
</details>

## 0.40.1 (2023-12-30)

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

## 0.40.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>
<csr-id-de400fd0c4959ed60009a2fd71a88d7bcf6d5191/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Other

- <csr-id-de400fd0c4959ed60009a2fd71a88d7bcf6d5191/> Add gix-object fuzzers

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 18 calendar days.
 - 22 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1185](https://github.com/GitoxideLabs/gitoxide/issues/1185)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1185](https://github.com/GitoxideLabs/gitoxide/issues/1185)**
    - Fix tree-parse issue found by fuzzer ([`7d81a76`](https://github.com/GitoxideLabs/gitoxide/commit/7d81a769f895b21716e92fb63298b9d547a5cca4))
 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Merge pull request #1199 from silvergasp/fuzz-git-object ([`7d21ce9`](https://github.com/GitoxideLabs/gitoxide/commit/7d21ce9fa0e10549b5340a92da32f70f9a3f7d62))
    - Add corpus builders for tree/tags ([`5a02529`](https://github.com/GitoxideLabs/gitoxide/commit/5a0252964d36aaa8a4a8cdbe7d17f59f3480d1b4))
    - Merge pull request #1192 from silvergasp/fuzz-git-object ([`e2f42dd`](https://github.com/GitoxideLabs/gitoxide/commit/e2f42dd93a2c749f1dbf7f9b7f35b37c9ea62cd4))
    - Add fuzz_commit seed corpus builder ([`2297ca1`](https://github.com/GitoxideLabs/gitoxide/commit/2297ca16bad699083d69f6a99d4fb6a51956df6b))
    - Merge branch 'fuzz-gix-ref' ([`8ee1194`](https://github.com/GitoxideLabs/gitoxide/commit/8ee119442b2ff4b846a34b57a222a6abdf6c95d8))
    - Let's have the latest versions of fuzzers automatically ([`5828f37`](https://github.com/GitoxideLabs/gitoxide/commit/5828f3796031de8d9f50bd6536697754da0b3d8c))
    - Merge branch 'fuzz_object' ([`7df919b`](https://github.com/GitoxideLabs/gitoxide/commit/7df919b356adfc1653bcd41137fa44a7054647be))
    - Add gix-object fuzzers ([`de400fd`](https://github.com/GitoxideLabs/gitoxide/commit/de400fd0c4959ed60009a2fd71a88d7bcf6d5191))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Check all git-lfs managed files into the repository ([`35439de`](https://github.com/GitoxideLabs/gitoxide/commit/35439defd2d71779d4b3795b7652cde18ff11150))
    - Git-lfs might fail early; let's rely on these caches to be recreated, where possible ([`b6f2b81`](https://github.com/GitoxideLabs/gitoxide/commit/b6f2b818f34e85edbdb0777a1df0cbf7fc9a0c98))
</details>

## 0.39.0 (2023-12-06)

### New Features

 - <csr-id-1838cbd36f4c19e07080529ec05ecf6c81218686/> support for querying only the header of an object with the `FindHeader` trait.
   That way one can know its decompressed size and its kind.
   
   We also add a `FindObjectOrHeader` trait for use as `dyn` trait object that
   can find objects and access their headers.
 - <csr-id-f984c4c58ca3a47469c1af66f9991c989f0d3e49/> Add `Find` and `Exists` trait and extension `FindExt` to easily find objects.
   This is more convenient than having to rely on closures all the time.
   Note that `Contains::contains` was renamed to `Exists::exists()`

### Bug Fixes

 - <csr-id-55729a5afe582100dc2ee0fa55f8b5ce313614df/> Report remaining input on error
 - <csr-id-8d05cae3027e3731602292c9e2d8c3cd53ebeec9/> adjust tree parser to deal with even more unusual trees.
 - <csr-id-5d78ab3d9e1f447f57a551561b0e773ea68b4d9a/> restore verbose error reporting capabilities when parsing of objects fails.
   When `verbose-object-parsing-errors` is enabled, it will now once again
   provide greater details as to where and why the parsing failed.
 - <csr-id-3542cf5f65dad92552e12cce84e47381aa58e228/> assure all kinds of trees can be parsed.
   Fix the slow and the fast-path tree-parsers to be able to cope
   with a greater variety of trees.
 - <csr-id-d25fbae8c81a3b40614d05041aafd7fbf06f849b/> Add `Copy` to `Data`, `BlobRef` and `TreeRefIter`

### Bug Fixes (BREAKING)

 - <csr-id-3ac5d0b7feed9554ddbf249748cb1591a7977d9b/> Keep verbatim tree-entry mode, but support discretization using the new `EntryKind`.
   Please note that `EntryKind` and `EntryMode` can be converted from and to each other.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 24 commits contributed to the release.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1096](https://github.com/GitoxideLabs/gitoxide/issues/1096)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1096](https://github.com/GitoxideLabs/gitoxide/issues/1096)**
    - Adjust tree parser to deal with even more unusual trees. ([`8d05cae`](https://github.com/GitoxideLabs/gitoxide/commit/8d05cae3027e3731602292c9e2d8c3cd53ebeec9))
    - Assure all kinds of trees can be parsed. ([`3542cf5`](https://github.com/GitoxideLabs/gitoxide/commit/3542cf5f65dad92552e12cce84e47381aa58e228))
 * **Uncategorized**
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - Upgrade to `winnow` 0.5.24 ([`abcfb65`](https://github.com/GitoxideLabs/gitoxide/commit/abcfb659786425ec09eff6b644cd2ad36b7d6bc4))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
    - Support for querying only the header of an object with the `FindHeader` trait. ([`1838cbd`](https://github.com/GitoxideLabs/gitoxide/commit/1838cbd36f4c19e07080529ec05ecf6c81218686))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'error' ([`c372321`](https://github.com/GitoxideLabs/gitoxide/commit/c372321dd6ea66a41c135d28c7319ab05a6d0942))
    - Merge branch 'fix-1096' ([`ff99a18`](https://github.com/GitoxideLabs/gitoxide/commit/ff99a18e9f9388542a9cbf17d61b413f34b1d533))
    - Keep verbatim tree-entry mode, but support discretization using the new `EntryKind`. ([`3ac5d0b`](https://github.com/GitoxideLabs/gitoxide/commit/3ac5d0b7feed9554ddbf249748cb1591a7977d9b))
    - Report remaining input on error ([`55729a5`](https://github.com/GitoxideLabs/gitoxide/commit/55729a5afe582100dc2ee0fa55f8b5ce313614df))
    - Refactor Errors to track remaining Input ([`5825250`](https://github.com/GitoxideLabs/gitoxide/commit/58252501f876bb7123b4f9bf2a75efc3011d7d37))
    - Refactor iterating parsing so error wrapping knows where left off ([`53309be`](https://github.com/GitoxideLabs/gitoxide/commit/53309be2b57f880316775df9e7f7eef3ebfa6534))
    - Refactor gix-object parsers to look more winnow-ish ([`f633f9e`](https://github.com/GitoxideLabs/gitoxide/commit/f633f9e443ec482fecb47ec7d3eccedba4fd053f))
    - Merge branch 'fix-1096' ([`48ef17e`](https://github.com/GitoxideLabs/gitoxide/commit/48ef17e4d169f2b08d119c2dba51fcfd9c06fe61))
    - Restore verbose error reporting capabilities when parsing of objects fails. ([`5d78ab3`](https://github.com/GitoxideLabs/gitoxide/commit/5d78ab3d9e1f447f57a551561b0e773ea68b4d9a))
    - Merge branch 'gix-object-find' ([`c8bd660`](https://github.com/GitoxideLabs/gitoxide/commit/c8bd66065316176dfbbfe7ecaa092a25cad1854b))
    - Add `Copy` to `Data`, `BlobRef` and `TreeRefIter` ([`d25fbae`](https://github.com/GitoxideLabs/gitoxide/commit/d25fbae8c81a3b40614d05041aafd7fbf06f849b))
    - Add `Find` and `Exists` trait and extension `FindExt` to easily find objects. ([`f984c4c`](https://github.com/GitoxideLabs/gitoxide/commit/f984c4c58ca3a47469c1af66f9991c989f0d3e49))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## 0.38.0 (2023-10-12)

### New Features

 - <csr-id-5bb55decbab55f0480165a756893b0af111e485c/> add `compute_stream_hash` method as lower-level way of hashing objects.

### Bug Fixes (BREAKING)

 - <csr-id-9283a9d2ec460c1380cf6c40d876f477cb552826/> `encode::loose_header()` now supports large objects even on 32 bit systems.
   Previously, larger than 4GB files wouldn't be supported, which causes problems when
   genrating hashes even when streaming data.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 8 calendar days.
 - 17 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Merge branch 'improvements' ([`429e7b2`](https://github.com/GitoxideLabs/gitoxide/commit/429e7b25f93c8a7947db19bafa74babf199a1aa6))
    - Add `compute_stream_hash` method as lower-level way of hashing objects. ([`5bb55de`](https://github.com/GitoxideLabs/gitoxide/commit/5bb55decbab55f0480165a756893b0af111e485c))
    - Fix docs ([`995bc84`](https://github.com/GitoxideLabs/gitoxide/commit/995bc840664cbd4aeb7f95592e3125dee63bdcd4))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - `encode::loose_header()` now supports large objects even on 32 bit systems. ([`9283a9d`](https://github.com/GitoxideLabs/gitoxide/commit/9283a9d2ec460c1380cf6c40d876f477cb552826))
</details>

## 0.37.0 (2023-09-24)

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

## 0.36.0 (2023-09-08)

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.35.0 (2023-08-22)

<csr-id-ef54aab9e5521add4154ee8d902d62612a9d8d4a/>
<csr-id-353b1a788a7c5a627ec73185f841ea4893a147a5/>
<csr-id-7649b185c93877e8c8b3cebedf685be8e94a0d9d/>

### Chore

- <csr-id-ef54aab9e5521add4154ee8d902d62612a9d8d4a/> switch `nom` to `winnow` in remaining uses in `gix-object`, `gix-ref`, and `gix-actor` for ~20% more performance.
  It's likely that over time, these parsers will get even faster due to improvements to `winnow`.
  Thanks, Ed Page, for single-handedly performing this transition.
- <csr-id-353b1a788a7c5a627ec73185f841ea4893a147a5/> add benchmarks to avoid parsing performance regressions
- <csr-id-7649b185c93877e8c8b3cebedf685be8e94a0d9d/> remove unused dependency: 'hex'

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release over the course of 13 calendar days.
 - 15 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Just fmt ([`0d258f4`](https://github.com/GitoxideLabs/gitoxide/commit/0d258f40afcd848509e2b0c7c264e9f346ed1726))
    - Switch `nom` to `winnow` in remaining uses in `gix-object`, `gix-ref`, and `gix-actor` for ~20% more performance. ([`ef54aab`](https://github.com/GitoxideLabs/gitoxide/commit/ef54aab9e5521add4154ee8d902d62612a9d8d4a))
    - Refactor and fixes ([`02587fc`](https://github.com/GitoxideLabs/gitoxide/commit/02587fc879c54b2b3e62ffbe1ab4c29591ea0d80))
    - Upgrade `winnow` to latest patch release ([`8c41848`](https://github.com/GitoxideLabs/gitoxide/commit/8c4184817e4e4364c34badc8ff0a71c6ae952efd))
    - Remove From<winnow> for (small) step towards stable API ([`be1f4b5`](https://github.com/GitoxideLabs/gitoxide/commit/be1f4b5d2158fc73d926394d22d62bb597c5e6ce))
    - Make clippy happy ([`62f7bc6`](https://github.com/GitoxideLabs/gitoxide/commit/62f7bc65c1eef812baa2ef2e5b406762e117b66a))
    - Switch off deprecated VerboseError ([`93fc441`](https://github.com/GitoxideLabs/gitoxide/commit/93fc441c6c82086e125ace5bdda3b9403d271618))
    - Switch errors to StrContext ([`df226dd`](https://github.com/GitoxideLabs/gitoxide/commit/df226dd31df2c591c6470ed70098202112e13dae))
    - Minor cleanup possible with 0.5 ([`a07590c`](https://github.com/GitoxideLabs/gitoxide/commit/a07590cb46423cb0422c18b9fc04b153c0fd53b1))
    - Upgrade to Winnow 0.5 ([`3f8c91f`](https://github.com/GitoxideLabs/gitoxide/commit/3f8c91fa463fbb53d54b2bf359e0dee7387afa00))
    - Simplify parsers ([`12f03db`](https://github.com/GitoxideLabs/gitoxide/commit/12f03db6475b92f492f5a14bda472c139c3511e0))
    - Resolve 0.4 not-quite deprecations ([`f0cbf81`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbf81a346e087a622b0e2a6a37593861d0010f))
    - Resolve 0.4 deprecations ([`9ed7df0`](https://github.com/GitoxideLabs/gitoxide/commit/9ed7df0a17deed08759dc29fc0089cdea100e433))
    - Upgrade to Winnow 0.4 ([`86ea47f`](https://github.com/GitoxideLabs/gitoxide/commit/86ea47f28079c51f874b0d662867040b92f88d14))
    - Parse explicitly in prep for 0.4 ([`b3f0418`](https://github.com/GitoxideLabs/gitoxide/commit/b3f041829881e881ad4eeeacaeea31064c523340))
    - Resolve remaining winnow 0.3 deprecations ([`fee441d`](https://github.com/GitoxideLabs/gitoxide/commit/fee441da875d52b1a0cb557d2fa58cee9c29e16a))
    - Prefer Parser inherent parsers ([`b37a909`](https://github.com/GitoxideLabs/gitoxide/commit/b37a909a5c344201a985262351e0fb67757572a4))
    - Prefer built-in Winnow parsers ([`ac0e81c`](https://github.com/GitoxideLabs/gitoxide/commit/ac0e81c41f8c8a33ede9a0d8b7bffcd04bb97dc3))
    - Simplify winnow ErrMode construction ([`86d7fd1`](https://github.com/GitoxideLabs/gitoxide/commit/86d7fd18487626d30f6d5478864819a3d7428085))
    - Switch gix to winnow 0.3 ([`ee75de1`](https://github.com/GitoxideLabs/gitoxide/commit/ee75de1e6035305fc23bdef2522ae5081272ac82))
    - Add benchmarks to avoid parsing performance regressions ([`353b1a7`](https://github.com/GitoxideLabs/gitoxide/commit/353b1a788a7c5a627ec73185f841ea4893a147a5))
    - Merge branch 'faster-hex' ([`4a4fa0f`](https://github.com/GitoxideLabs/gitoxide/commit/4a4fa0fcdaa6e14b51d3f03f5d7c5b65042667bf))
    - Remove unused dependency: 'hex' ([`7649b18`](https://github.com/GitoxideLabs/gitoxide/commit/7649b185c93877e8c8b3cebedf685be8e94a0d9d))
    - Merge branch 'extract-signatures' ([`b37affe`](https://github.com/GitoxideLabs/gitoxide/commit/b37affefecfb30a94431cd21dae6659004ca6244))
    - Refactor ([`fb95ead`](https://github.com/GitoxideLabs/gitoxide/commit/fb95eadb2da12c794d41b1f6212eba60ff3dd2dc))
    - Add commit signature extraction ([`2f9c0dd`](https://github.com/GitoxideLabs/gitoxide/commit/2f9c0ddb7c2191789b4c0a5e8f2bf9253ac58606))
</details>

## 0.34.0 (2023-08-07)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 3 calendar days.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-glob v0.10.2, gix-date v0.7.2, gix-validate v0.8.0, gix-object v0.34.0, gix-ref v0.34.0, gix-config v0.27.0, gix-commitgraph v0.18.2, gix-revwalk v0.5.0, gix-revision v0.19.0, gix-refspec v0.15.0, gix-submodule v0.1.0, safety bump 18 crates ([`4604f83`](https://github.com/GitoxideLabs/gitoxide/commit/4604f83ef238dc07c85aaeae097399b67f3cfd0c))
    - Prepare changelogs prior to release of `gix-submodule` ([`f3c4311`](https://github.com/GitoxideLabs/gitoxide/commit/f3c43110e8d5f16cf87e50821044d8b3edbae235))
    - Merge branch 'dev-on-linux' ([`6b4a303`](https://github.com/GitoxideLabs/gitoxide/commit/6b4a30330fe49fc97daa73f55bf56580cc0597aa))
    - Fix various tests to run properly on linux ([`ef8ccd9`](https://github.com/GitoxideLabs/gitoxide/commit/ef8ccd9d16143d37155d063747c69cade80f162d))
</details>

## 0.33.2 (2023-08-02)

<csr-id-7f7353e4525adc308e97b82d27bd301e48da6016/>

### Other

- <csr-id-7f7353e4525adc308e97b82d27bd301e48da6016/> fix test for struct size for 32-bit architectures
  The size of gix_object::Data is 24 bytes only on 64-bit architectures,
  on 32-bit architectures it's exactly half that (12 bytes).

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 11 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.24.2, gix-object v0.33.2, gix-ref v0.33.3, gix-config v0.26.2, gix-prompt v0.5.5, gix-odb v0.50.2, gix-transport v0.34.2, gix-protocol v0.37.0, gix-worktree v0.23.1, gix v0.51.0, safety bump 3 crates ([`231ac1c`](https://github.com/GitoxideLabs/gitoxide/commit/231ac1c6ad5ca9a84dbeb0dee14bfbf2fef1ae1e))
    - Prepare additional changelogs ([`db63815`](https://github.com/GitoxideLabs/gitoxide/commit/db6381522395a0de047118e81df5cd3cbeb862b9))
    - Merge branch 'decathorpe/main' ([`422747d`](https://github.com/GitoxideLabs/gitoxide/commit/422747d35724a56dade73c8d68b8ebcbaa4b9b25))
    - Prepare changelogs ([`e4d2890`](https://github.com/GitoxideLabs/gitoxide/commit/e4d2890a85bf60e9cdb4016dddfab3c4dccbe75e))
    - Merge branch 'fixes-and-improvements' ([`f8b1f55`](https://github.com/GitoxideLabs/gitoxide/commit/f8b1f553371f25b1bea6bce7cbb2ff1f01194856))
    - Fix test for struct size for 32-bit architectures ([`7f7353e`](https://github.com/GitoxideLabs/gitoxide/commit/7f7353e4525adc308e97b82d27bd301e48da6016))
    - Add another very special commit for parsing tests ([`f852243`](https://github.com/GitoxideLabs/gitoxide/commit/f852243831474a3fd311a0507d11fd43e0f6e4d3))
</details>

## 0.33.1 (2023-07-22)

### New Features

 - <csr-id-424d347bb2466ecdf8cfc9e0da779d604d1f9e4e/> `TreeRef::bisect_entry()` to correctly find an entry by name
 - <csr-id-437fedc59c2bad57beb3cf54e4c71a470bd22ee2/> `Copy` for `tree::EntryRef`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Prepare more changelogs ([`c4cc5f2`](https://github.com/GitoxideLabs/gitoxide/commit/c4cc5f261d29f712a101033a18293a97a9d4ae85))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Merge branch 'improvements-for-crates-index' ([`7734736`](https://github.com/GitoxideLabs/gitoxide/commit/7734736ce7e590b2ad60f3c1d8289854931d155c))
    - `TreeRef::bisect_entry()` to correctly find an entry by name ([`424d347`](https://github.com/GitoxideLabs/gitoxide/commit/424d347bb2466ecdf8cfc9e0da779d604d1f9e4e))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Merge branch 'improvements-for-crates-index' ([`3f914e8`](https://github.com/GitoxideLabs/gitoxide/commit/3f914e8840afc59441c3c463bdc89c53136d583e))
    - `Copy` for `tree::EntryRef` ([`437fedc`](https://github.com/GitoxideLabs/gitoxide/commit/437fedc59c2bad57beb3cf54e4c71a470bd22ee2))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.33.0 (2023-07-19)

### New Features

 - <csr-id-695de5679904776798b4d8b9dbee771306ff39ba/> Add classifier methods to `Kind` for easier object classification.
   This allows to avoid having to think about where to right type lives
   for comparisons or matches in simple cases that are about one specific object type.
 - <csr-id-437fedc59c2bad57beb3cf54e4c71a470bd22ee2/> `Copy` for `tree::EntryRef`
 - <csr-id-424d347bb2466ecdf8cfc9e0da779d604d1f9e4e/> `TreeRef::bisect_entry()` to correctly find an entry by name

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 19 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
    - Merge branch 'adjustments-for-crates-index' ([`b82868d`](https://github.com/GitoxideLabs/gitoxide/commit/b82868d5688d8d4849c47ed3d209a96ee59e69b3))
    - Add classifier methods to `Kind` for easier object classification. ([`695de56`](https://github.com/GitoxideLabs/gitoxide/commit/695de5679904776798b4d8b9dbee771306ff39ba))
</details>

## 0.32.0 (2023-06-29)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 6 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
    - Prepare changelogs prior to release ([`00f96fb`](https://github.com/GitoxideLabs/gitoxide/commit/00f96fb3110a8f81a1bd0d74c757c15b8773c6f6))
    - Merge branch 'i64-times' ([`b407461`](https://github.com/GitoxideLabs/gitoxide/commit/b407461d8991db67a5bdb2ab13f518f78a85ed40))
    - Add a test to see what happens if negative dates are used in commits ([`57a5cd1`](https://github.com/GitoxideLabs/gitoxide/commit/57a5cd1ca2f8153568c366cd1709be7d4ebec972))
</details>

## 0.31.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 11 calendar days.
 - 15 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - `just fmt` ([`871dd0b`](https://github.com/GitoxideLabs/gitoxide/commit/871dd0b977caf17159092a4739ba5408403cdb2c))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
    - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
    - Adapt to changes in `gix-actor` ([`4a80e86`](https://github.com/GitoxideLabs/gitoxide/commit/4a80e868f9530896616e649838e9be64b6d10036))
    - Adapt to changes in `gix-date` ([`d575336`](https://github.com/GitoxideLabs/gitoxide/commit/d575336c26e6026e463cd06d88266bb2bdd3e162))
</details>

## 0.30.0 (2023-06-06)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 12 calendar days.
 - 25 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - `just fmt` ([`ffc1276`](https://github.com/GitoxideLabs/gitoxide/commit/ffc1276e0c991ac33ce842f5dca0b45ac69680c0))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge pull request #878 from blinxen/main ([`67da689`](https://github.com/GitoxideLabs/gitoxide/commit/67da6894c8d8a24b982c732a1753a3e0a3300cc3))
    - Include missing changelog file in some crates ([`0269eed`](https://github.com/GitoxideLabs/gitoxide/commit/0269eedc08c21589b5381d9b7d7fcc7004160bf8))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Minor fixes ([`89a8cfe`](https://github.com/GitoxideLabs/gitoxide/commit/89a8cfe40e5c3a9d4a4181fa055e3f4a529a8081))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Auto-fix as many 'range-plus-one' lints as possible ([`4795fcf`](https://github.com/GitoxideLabs/gitoxide/commit/4795fcf6adb06b792592a1b11a3f071e9d47b1ac))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
</details>

## 0.29.2 (2023-05-11)

### Bug Fixes

 - <csr-id-ca37915fbb0c43c7f01ae600d1da63d2f21eeb63/> sorting of tree entries is now according to specificiation.
   This means tree entries are compared as if they had a / appended to it.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 15 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-object v0.29.2 ([`4f879bf`](https://github.com/GitoxideLabs/gitoxide/commit/4f879bf35653bdc8f9729d524c6e8e1fb3c6886b))
    - Merge branch 'fix-848' ([`e3b146f`](https://github.com/GitoxideLabs/gitoxide/commit/e3b146ffdd6a1150e6deb6ddd267b81fd3c217b2))
    - Sorting of tree entries is now according to specificiation. ([`ca37915`](https://github.com/GitoxideLabs/gitoxide/commit/ca37915fbb0c43c7f01ae600d1da63d2f21eeb63))
    - Refactor ([`d92a588`](https://github.com/GitoxideLabs/gitoxide/commit/d92a588249f3f507f0b3fc05543f819c51e49c5a))
</details>

## 0.29.1 (2023-04-26)

### New Features

 - <csr-id-ebf3fd49ff2a53c99f818f4e87fbfed57db696bb/> add `encode::compute_hash()` for an easy way to obtain object hashes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 6 calendar days.
 - 7 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - Merge branch 'fix-823' ([`6ebd61e`](https://github.com/GitoxideLabs/gitoxide/commit/6ebd61e548a36a04e413ac725a03e607a3588334))
    - Thanks clippy ([`14e64e7`](https://github.com/GitoxideLabs/gitoxide/commit/14e64e74649cfb1f2f99da87015939af98fae5c8))
    - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
    - Add `encode::compute_hash()` for an easy way to obtain object hashes. ([`ebf3fd4`](https://github.com/GitoxideLabs/gitoxide/commit/ebf3fd49ff2a53c99f818f4e87fbfed57db696bb))
</details>

## 0.29.0 (2023-04-19)

### New Features (BREAKING)

 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 6 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#814](https://github.com/GitoxideLabs/gitoxide/issues/814)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
    - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
 * **Uncategorized**
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Prepare changelog prior to release ([`7f06458`](https://github.com/GitoxideLabs/gitoxide/commit/7f064583bd0e1b078df89a7750f5a25deb70f516))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
</details>

## 0.28.0 (2023-03-04)

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
    - Release gix-features v0.28.0, gix-actor v0.19.0, gix-object v0.28.0, gix-diff v0.28.0, gix-traverse v0.24.0, gix-pack v0.32.0, safety bump 20 crates ([`0f411e9`](https://github.com/GitoxideLabs/gitoxide/commit/0f411e93ec812592bb9d3a52b751399dd86f76f7))
    - Prepare changelogs prior to release of `gix-pack` ([`6db30ef`](https://github.com/GitoxideLabs/gitoxide/commit/6db30ef6b5e931bbf12135507a3d922051de4d4b))
</details>

## 0.27.0 (2023-03-01)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
</details>

## 0.26.4 (2023-02-24)

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
    - Release gix-object v0.26.4, gix-diff v0.26.3, gix v0.37.2, gix-commitgraph v0.13.1, gitoxide-core v0.25.0, gitoxide v0.23.0 ([`9982949`](https://github.com/GitoxideLabs/gitoxide/commit/9982949cab401501d5ce3cba4e2ba900bc249c53))
    - Prepare changelog for release ([`13a1ec1`](https://github.com/GitoxideLabs/gitoxide/commit/13a1ec1803d677c2e94f3ea0461118c2426f8071))
    - Merge branch 'rename-tracking' ([`550144a`](https://github.com/GitoxideLabs/gitoxide/commit/550144a5fd37d501d86f4b1c4db2948d951d1c93))
    - Document why EntryRef has a different comparison method. ([`c9ba1f7`](https://github.com/GitoxideLabs/gitoxide/commit/c9ba1f7e5bd4a24cd1e32b126955b03957f01845))
</details>

## 0.26.3 (2023-02-20)

### New Features

 - <csr-id-c882ff0b2cbf3484520432593968716493ffdaa1/> add `tree::EntryMode::is_blob_or_link()`.
   Returns true if the entry points to a blob or a symlink.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-object v0.26.3, gix-diff v0.26.2, gix-traverse v0.22.2, gix v0.37.0, safety bump 3 crates ([`8b3e42f`](https://github.com/GitoxideLabs/gitoxide/commit/8b3e42f69fe97fe5083eb845c323f10d7ac087b2))
    - Merge branch 'rename-tracking' ([`35415c5`](https://github.com/GitoxideLabs/gitoxide/commit/35415c5061bf5ea90a04db80d06ac3622d0b0f1a))
    - Add `tree::EntryMode::is_blob_or_link()`. ([`c882ff0`](https://github.com/GitoxideLabs/gitoxide/commit/c882ff0b2cbf3484520432593968716493ffdaa1))
</details>

## 0.26.2 (2023-02-20)

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

## 0.26.1 (2023-02-17)

<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>
<csr-id-9c8a36f35b1e8dad66befb102e5a8670b4a6c2b2/>
<csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/>
<csr-id-3b9d66c932075feb08cdf2967f7698daef9fd3ff/>
<csr-id-ec3be19c8d007565b814b4757f17811ec0e9de2c/>
<csr-id-899c57927ce4ba2e1b5d8182f9e731c2a9094cba/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-322b2901382192d3b4ac0fbae32381a9abb49fa9/>

### Refactor (BREAKING)

- <csr-id-2290d006705ff47ad780b009fe58ee422b3285af/> move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.
- <csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/> move loose header manipulation from git-pack to git-object

### New Features (BREAKING)

 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `git-features` and `git-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.

### Chore (BREAKING)

- <csr-id-9c8a36f35b1e8dad66befb102e5a8670b4a6c2b2/> remove quick-error in favor of thiserror
  Some errors change shape which makes this a breaking change.

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`
 - <csr-id-fda2a8d2f5f8b7d80b4cc0177d08d6a061f1b8f1/> rename `commit::ref_iter::Token::into_id()` to `*::try_into_id()`
   This makes the method more consistent.

### Refactor

- <csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/> Use borrowed::Id in trees for full type safety
- <csr-id-3b9d66c932075feb08cdf2967f7698daef9fd3ff/> make reusing round-trip code easier
- <csr-id-ec3be19c8d007565b814b4757f17811ec0e9de2c/> Prefer integration level tests, but use unit-tests where appropriate

### Performance

 - <csr-id-83d591d536d1a43e8523082824ec0b95cca55d34/> parse entry mode into number instead of comparing it to byte strings
 - <csr-id-f9232acf8e52f8cd95520d122469e136eb07b39f/> ObjectID specific hashers, using the fact that object ids are hashes

### Other

- <csr-id-899c57927ce4ba2e1b5d8182f9e731c2a9094cba/> describe variants

### Bug Fixes

 - <csr-id-2df3c7395a791c751b22187a936facbce9f3f168/> don't panic if broken loose objects are encountered.
   Previously a loose object could advertise any object size and cause a
   panic if it was shorter than advertised.
   Now an error will be returned.
 - <csr-id-761695cd10222eccb720812fa41d5295d506fce9/> improve error messages related to object decoding

### New Features

 - <csr-id-14d613303b8663daac1ab1eed8413266594050c3/> `Object::try_into_blob_ref()` as alternative to consuming `*::try_into_blob()`
 - <csr-id-5095df949d30b57b09bd68355743d255a897365f/> add `tree::EntryMode::is_blob()` to quickly determine if an entry is a blob.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-4f4bba23a13f11050e35887da004e48148d68aea/> `tree::EntryMode::as_str()` to display itself as a string.
 - <csr-id-0d22ab459ce14bc57549270142595d8ebd98ea41/> `TagRefIter::tagger()`.
   Additionally ergonomics have been improved as the iterator is now
   `Copy`, similarly to the other iterators.
 - <csr-id-a39bf71531ee0a6c8db082758d3212c805ce2bf0/> support for trimming of whitespace around name and email
   It's separated from parsing to assure we can round-trip, but it's
   made easy to obtain trimmed results using new methods.
   
   This high-level git-repository will also trim by default now.
 - <csr-id-8f1b42b193098f6e116b3522a3df08d5d1b42a0a/> `CommitRef::time()`
   A shortcut to allow for fluid retrival of the `committer.time`.
 - <csr-id-c51a9252f8a038bf995d50cdbfc8356362020bbc/> `CommitRefIter::(author|committer)()`, better usability
   The methods returning an iterator are now consuming, which allows
   them to be nested by callers.
 - <csr-id-91065cd790f559f4a661ecd9c2d2f649922682e3/> `CommitRefIter::parent_ids()`
   Allocation-free parent retrieval.
 - <csr-id-5c573bf9abfc10227d5a900100e84d7f8f6c0504/> in-manifest and in-lib feature documentation
 - <csr-id-eb36a3dda83a46ad59078a904f4e277f298a24e1/> Add sorting mode to ancestor traversal  #270
 - <csr-id-a0bb6529216160d14e67b97206a51140099fd245/> add `Data` object
   This is typed data baked by a slice for conversion into parsed ObjectRef's
   for example.
   
   This is usually the result of a `Find` operation on an object database.

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-322b2901382192d3b4ac0fbae32381a9abb49fa9/> upgrade dependencies

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 574 commits contributed to the release.
 - 30 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#250](https://github.com/GitoxideLabs/gitoxide/issues/250), [#254](https://github.com/GitoxideLabs/gitoxide/issues/254), [#259](https://github.com/GitoxideLabs/gitoxide/issues/259), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#364](https://github.com/GitoxideLabs/gitoxide/issues/364), [#389](https://github.com/GitoxideLabs/gitoxide/issues/389), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#63](https://github.com/GitoxideLabs/gitoxide/issues/63), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691), [#79](https://github.com/GitoxideLabs/gitoxide/issues/79), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 17 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Fix stop-release-for-changelog logic and fix all affected changelogs ([`52b38bc`](https://github.com/GitoxideLabs/gitoxide/commit/52b38bc4856be5ba8b5372a3dd20f5d06504e7ed))
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Allow 'refactor' and 'other' in conventional messages if they have breaking changes ([`4eebaac`](https://github.com/GitoxideLabs/gitoxide/commit/4eebaac669e590beed112b622752997c64772ef1))
    - Rebuild all changelogs to assure properly ordered headlines ([`4a9a05f`](https://github.com/GitoxideLabs/gitoxide/commit/4a9a05f95930bad5938d4ce9c517ebf0e0b990f1))
    - Sort all commits by time, descending… ([`f536bad`](https://github.com/GitoxideLabs/gitoxide/commit/f536bad20ffbac4dc353dfeb1a917bb88becbb78))
    - Greatly reduce changelog size now that the traversal fix is applied ([`a0bc98c`](https://github.com/GitoxideLabs/gitoxide/commit/a0bc98c06c349de2fd6e0d4593606e68b98def72))
    - Generate changelogs with details ([`e1861ca`](https://github.com/GitoxideLabs/gitoxide/commit/e1861caa435d312953a9fea7ceff6d2e07b03443))
    - Update all changelogs with details ([`58ab2ae`](https://github.com/GitoxideLabs/gitoxide/commit/58ab2aee23ba70a536e9487b44fb04c610374d1a))
    - Update changelogs ([`c857d61`](https://github.com/GitoxideLabs/gitoxide/commit/c857d61ce3ce342012a2c4ba10a8327822aa530e))
    - Avoid adding newlines which make writing unstable ([`6b5c394`](https://github.com/GitoxideLabs/gitoxide/commit/6b5c394f49282a8d09c2a9ffece840e4683572db))
    - Fix section headline level ([`9d6f263`](https://github.com/GitoxideLabs/gitoxide/commit/9d6f263beef289d227dec1acc2d4240087cb9be6))
    - Write first version of changlogs thus far… ([`719b6bd`](https://github.com/GitoxideLabs/gitoxide/commit/719b6bdf543b8269ccafad9ad6b46e0c55efaa38))
    - Feat: `BodyRef::without_trailer()` for more obvious access than `*body` or `body.as_ref()` ([`f0ea526`](https://github.com/GitoxideLabs/gitoxide/commit/f0ea526775793c9104e4ae27dd5d92b5a1202c5f))
    - Refactor ([`ef3fc6d`](https://github.com/GitoxideLabs/gitoxide/commit/ef3fc6d92c1d751d0032e072834f41d37cbb9200))
    - Feat: `CommitRef::message_trailers()` as shortcut… ([`5324391`](https://github.com/GitoxideLabs/gitoxide/commit/5324391f581c5ad2c09604f0beeac7df852bfb33))
    - More tests for trailers iterator ([`c3b0161`](https://github.com/GitoxideLabs/gitoxide/commit/c3b0161eb76aaf806a7d82232ec7ac1a304052a3))
    - Feat: `BodyRef::trailers()` allows iterating trailer tokens and values ([`175e1cb`](https://github.com/GitoxideLabs/gitoxide/commit/175e1cbdfebfc6f01784fffdaf0859cd6c23377e))
    - Some tests and sketch for BodyRef parsing ([`3953c24`](https://github.com/GitoxideLabs/gitoxide/commit/3953c245461941c636ce5d755e6a469f7fa3eabe))
    - Feat: CommitRef::summary() and `MessageRef::body()` methods ([`1714d05`](https://github.com/GitoxideLabs/gitoxide/commit/1714d05df812aa373da485492b342e58e9e7c17d))
    - Refactor ([`7055dc8`](https://github.com/GitoxideLabs/gitoxide/commit/7055dc81e9db448da89ab2ee0ba2ffe07cd00cc2))
    - Another test for footer separation, simple version ([`b439186`](https://github.com/GitoxideLabs/gitoxide/commit/b4391862b67a09330476a82d520bfc3a698a4fbe))
    - Return to safety ([`35313b9`](https://github.com/GitoxideLabs/gitoxide/commit/35313b9f7c92edd1afdbe22d1f592baabc0abc9c))
    - Omg nom parsing works… ([`cd11704`](https://github.com/GitoxideLabs/gitoxide/commit/cd11704dd0d469cd23d7ee6905d6219b26ba4563))
    - FAIL: not really successful to continue down the 'fold' road ([`d9afc22`](https://github.com/GitoxideLabs/gitoxide/commit/d9afc22f161fb60195571be09d2d816d67050575))
    - Three tests failing with nom ([`13646e8`](https://github.com/GitoxideLabs/gitoxide/commit/13646e8dfe97d8503d0cef935c4c303f82271aa4))
    - Revert " FAIL: try to use nom-way of the previous body parsing…" ([`d1e6f62`](https://github.com/GitoxideLabs/gitoxide/commit/d1e6f621c2898ad9f03b2ee712019e6a10b44035))
    - FAIL: try to use nom-way of the previous body parsing… ([`909f668`](https://github.com/GitoxideLabs/gitoxide/commit/909f6682ac1de6be0eb8b66015b3f250daca17cd))
    - Sketch nom version of the message parser… ([`1ec47de`](https://github.com/GitoxideLabs/gitoxide/commit/1ec47ded5793cac1f2633262d59bfbae4a0e14be))
    - Fix build ([`d0a956f`](https://github.com/GitoxideLabs/gitoxide/commit/d0a956fdb5a822dbd116792bfbe70d1532a95ec9))
    - Refactor!: Use git_object::commit::MessageRef::summary()… ([`13e7c3a`](https://github.com/GitoxideLabs/gitoxide/commit/13e7c3ad5e079fe778d07d115c9e41c4c6eb038f))
    - Feat(commit): A summary for commit messages suitable for logs ([`cd3fc99`](https://github.com/GitoxideLabs/gitoxide/commit/cd3fc99968baa827302aa9c4f5d181bc9c4f9084))
    - More message parsing tests with windows line separators ([`001e8c2`](https://github.com/GitoxideLabs/gitoxide/commit/001e8c2a4ede5fc025572a4c39a771cc854f8b18))
    - A manual message parse impl and more tests ([`f4b8a0d`](https://github.com/GitoxideLabs/gitoxide/commit/f4b8a0da787f9a16eebd2a36b342f5a2a66edabd))
    - More message parsing tests, now with legit failure… ([`625be8d`](https://github.com/GitoxideLabs/gitoxide/commit/625be8dbd4204ea1a7131ade9b17f63dcc7e30d7))
    - Feat(commit): Add `message()` method and `MessageRef` type… ([`6150b2d`](https://github.com/GitoxideLabs/gitoxide/commit/6150b2db18034d5912029deac51d1affb38ae7b2))
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
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - Upgrade dependencies ([`322b290`](https://github.com/GitoxideLabs/gitoxide/commit/322b2901382192d3b4ac0fbae32381a9abb49fa9))
    - Fix docs ([`1bb4253`](https://github.com/GitoxideLabs/gitoxide/commit/1bb425347e4b502e1c048908cd5f3641d2b16896))
    - Move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait ([`2290d00`](https://github.com/GitoxideLabs/gitoxide/commit/2290d006705ff47ad780b009fe58ee422b3285af))
    - Add `Data` object ([`a0bb652`](https://github.com/GitoxideLabs/gitoxide/commit/a0bb6529216160d14e67b97206a51140099fd245))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Adapt to changes in git-hash ([`5eb0230`](https://github.com/GitoxideLabs/gitoxide/commit/5eb0230b58c25c0aa744eee0bd878dd91410dbe1))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - Fix tree-entry-ordering implementation ([`ea169a6`](https://github.com/GitoxideLabs/gitoxide/commit/ea169a6dbbba30d9464570cb86e5c990fcaf9ae8))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - In-manifest and in-lib feature documentation ([`5c573bf`](https://github.com/GitoxideLabs/gitoxide/commit/5c573bf9abfc10227d5a900100e84d7f8f6c0504))
    - Document all features related to serde1 ([`72b97f2`](https://github.com/GitoxideLabs/gitoxide/commit/72b97f2ae4dc7642b160f183c6d5df4502dc186f))
 * **[#364](https://github.com/GitoxideLabs/gitoxide/issues/364)**
    - Support for trimming of whitespace around name and email ([`a39bf71`](https://github.com/GitoxideLabs/gitoxide/commit/a39bf71531ee0a6c8db082758d3212c805ce2bf0))
    - `CommitRef::time()` ([`8f1b42b`](https://github.com/GitoxideLabs/gitoxide/commit/8f1b42b193098f6e116b3522a3df08d5d1b42a0a))
    - Full error handling for CommitRefIter ([`b94471a`](https://github.com/GitoxideLabs/gitoxide/commit/b94471a0ced50204156cf5d4126c676f0258a5eb))
    - More speedy access to author/committer ([`6129607`](https://github.com/GitoxideLabs/gitoxide/commit/61296077cebaaf2eb939fa6082121304bc6cf39b))
    - `CommitRefIter::(author|committer)()`, better usability ([`c51a925`](https://github.com/GitoxideLabs/gitoxide/commit/c51a9252f8a038bf995d50cdbfc8356362020bbc))
    - `CommitRefIter::parent_ids()` ([`91065cd`](https://github.com/GitoxideLabs/gitoxide/commit/91065cd790f559f4a661ecd9c2d2f649922682e3))
 * **[#389](https://github.com/GitoxideLabs/gitoxide/issues/389)**
    - `TagRefIter::tagger()`. ([`0d22ab4`](https://github.com/GitoxideLabs/gitoxide/commit/0d22ab459ce14bc57549270142595d8ebd98ea41))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - `tree::EntryMode::as_str()` to display itself as a string. ([`4f4bba2`](https://github.com/GitoxideLabs/gitoxide/commit/4f4bba23a13f11050e35887da004e48148d68aea))
    - Improve error messages related to object decoding ([`761695c`](https://github.com/GitoxideLabs/gitoxide/commit/761695cd10222eccb720812fa41d5295d506fce9))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
    - Fix docs ([`dad9cbe`](https://github.com/GitoxideLabs/gitoxide/commit/dad9cbeb853c0cc5128360b05c04b5a3da7ec75e))
    - Remove quick-error in favor of thiserror ([`9c8a36f`](https://github.com/GitoxideLabs/gitoxide/commit/9c8a36f35b1e8dad66befb102e5a8670b4a6c2b2))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#63](https://github.com/GitoxideLabs/gitoxide/issues/63)**
    - Use new `oid` where possible in git-odb ([`68a709e`](https://github.com/GitoxideLabs/gitoxide/commit/68a709e0337d4969138d30a5c25d60b7dbe51a73))
    - Refactor; better errors for invalid hash sizes ([`be84b36`](https://github.com/GitoxideLabs/gitoxide/commit/be84b36129694a2e89d1b81d932f2eba23aedf54))
    - Make ObjectId/oid happen! ([`ca78d15`](https://github.com/GitoxideLabs/gitoxide/commit/ca78d15373ec988d909be8f240baefe75555e077))
    - Remove all public exports of git-hash types in git-object ([`accf89d`](https://github.com/GitoxideLabs/gitoxide/commit/accf89d25560e5ded6f44a1c4a898ee65d14f8f6))
    - Remove re-export of git_object::borrowed::Id ([`a3f2816`](https://github.com/GitoxideLabs/gitoxide/commit/a3f28169c1268c1129852f279631d5a7f7540cdf))
    - Move git-hash::owned::Id into git-hash::Id ([`fdbe704`](https://github.com/GitoxideLabs/gitoxide/commit/fdbe704b6c9ace2b8f629f681a0580b24749a238))
    - Rename `git_hash::*::Digest` to `Id` ([`188d90a`](https://github.com/GitoxideLabs/gitoxide/commit/188d90ad463d342d715af701b03f0ed392c977fc))
 * **[#67](https://github.com/GitoxideLabs/gitoxide/issues/67)**
    - Describe variants ([`899c579`](https://github.com/GitoxideLabs/gitoxide/commit/899c57927ce4ba2e1b5d8182f9e731c2a9094cba))
    - Parse entry mode into number instead of comparing it to byte strings ([`83d591d`](https://github.com/GitoxideLabs/gitoxide/commit/83d591d536d1a43e8523082824ec0b95cca55d34))
    - ObjectID specific hashers, using the fact that object ids are hashes ([`f9232ac`](https://github.com/GitoxideLabs/gitoxide/commit/f9232acf8e52f8cd95520d122469e136eb07b39f))
    - Tree parsing now probably is twice as fast… ([`d1e2b89`](https://github.com/GitoxideLabs/gitoxide/commit/d1e2b8910b454dd798be8a9a43871f0b0644d503))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **[#79](https://github.com/GitoxideLabs/gitoxide/issues/79)**
    - Refactor; add test for empty tree iteration ([`6340296`](https://github.com/GitoxideLabs/gitoxide/commit/634029682da374f912068f5c8d5ec79d4837f7ea))
 * **[#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)**
    - Prepare changelogs prior to release ([`8c0bca3`](https://github.com/GitoxideLabs/gitoxide/commit/8c0bca37ff9fbaadbe55561fb2b0d649980c95b1))
 * **Uncategorized**
    - Release gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`e313112`](https://github.com/GitoxideLabs/gitoxide/commit/e31311257bd138b52042dea5fc40c3abab7f269b))
    - Release gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6efd0d3`](https://github.com/GitoxideLabs/gitoxide/commit/6efd0d31fbeca31ab7319aa2ac97bb31dc4ce055))
    - Release gix-date v0.4.2, gix-hash v0.10.2, gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6ccc88a`](https://github.com/GitoxideLabs/gitoxide/commit/6ccc88a8e4a56973b1a358cf72dc012ee3c75d56))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Fix git-object tests ([`fb92865`](https://github.com/GitoxideLabs/gitoxide/commit/fb92865fec8c76b710c432b9e2e08ff8e9538fcd))
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
    - Rename `git-object` to `gix-object` ([`2320358`](https://github.com/GitoxideLabs/gitoxide/commit/23203589e8c1df2e0eae9d9c2e75b0d0b24fd9b5))
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
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Optimize usage of `hex_to_id()` ([`6fa950d`](https://github.com/GitoxideLabs/gitoxide/commit/6fa950d0ab1991a5577c06385169be1b390dd88a))
    - Break cyclical dev dependencies ([`1fea18f`](https://github.com/GitoxideLabs/gitoxide/commit/1fea18f5f8b4189a23dc4fa3f041a672f6fbcfb3))
    - Release git-date v0.4.0, git-actor v0.17.0, git-object v0.26.0, git-traverse v0.22.0, git-index v0.12.0, safety bump 15 crates ([`0e3d0a5`](https://github.com/GitoxideLabs/gitoxide/commit/0e3d0a56d7e6a60c6578138f2690b4fa54a2072d))
    - Prepare changelogs prior to release ([`d679f5b`](https://github.com/GitoxideLabs/gitoxide/commit/d679f5b6f018633e858d3ebbdaf1cd5098bbc5e7))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Prepare changelogs prior to release ([`30d8ca1`](https://github.com/GitoxideLabs/gitoxide/commit/30d8ca19284049dcfbb0de2698cafae1d1a16b0c))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'pierrechevalier83/main' ([`a5b1d73`](https://github.com/GitoxideLabs/gitoxide/commit/a5b1d738d23d0a343bee1b72bcb72250b5fdae11))
    - Thanks clippy ([`0a6d888`](https://github.com/GitoxideLabs/gitoxide/commit/0a6d8882265eda833f4bf0252dc2c656aa05ca6f))
    - Restore original representation of `Tag` at the cost of some duplication ([`dd0a23d`](https://github.com/GitoxideLabs/gitoxide/commit/dd0a23d710be0eb6c7ea7f883aeb1400bcbc0709))
    - Unify pre-allocation of `parents` in `Commit` to match typical single-ancestor chains ([`7242d1f`](https://github.com/GitoxideLabs/gitoxide/commit/7242d1f4acb4ac0d2585295c11f36436a1215a7a))
    - Refactor ([`c02a6bd`](https://github.com/GitoxideLabs/gitoxide/commit/c02a6bdcc3669a48cd4b5b640280701fd089575d))
    - [refactor] Deduplicate Tag and TagRef ([`6003fa2`](https://github.com/GitoxideLabs/gitoxide/commit/6003fa22085b5031565c51b2b5a0a9feb1579fb0))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Merge branch 'pierrechevalier83/main' ([`f0dfa4c`](https://github.com/GitoxideLabs/gitoxide/commit/f0dfa4c17be4d4a15fb2117bfc47b2fea1bc48fb))
    - [git-object] Encode empty tags like git does ([`3bd3380`](https://github.com/GitoxideLabs/gitoxide/commit/3bd33802e072b06d278b93eeae8cfaed19795d6b))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index (upgrade to Rust 1.65) ([`5406630`](https://github.com/GitoxideLabs/gitoxide/commit/5406630466145990b5adbdadb59151036993060d))
    - Adapt in-memory size check to Rust 1.65 and below ([`1919e8e`](https://github.com/GitoxideLabs/gitoxide/commit/1919e8ec2f6bca8237a0356972b86f28c18da908))
    - `Object::try_into_blob_ref()` as alternative to consuming `*::try_into_blob()` ([`14d6133`](https://github.com/GitoxideLabs/gitoxide/commit/14d613303b8663daac1ab1eed8413266594050c3))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Merge branch 'main' into fetch-pack ([`93917cb`](https://github.com/GitoxideLabs/gitoxide/commit/93917cb6ecbb30daf3d20bb5a7c65e12211f084f))
    - Merge branch 'fix-loose-parsing' ([`1dc2a5d`](https://github.com/GitoxideLabs/gitoxide/commit/1dc2a5db9efa2cfb90a8b9fbeaca5200d35daed9))
    - Don't panic if broken loose objects are encountered. ([`2df3c73`](https://github.com/GitoxideLabs/gitoxide/commit/2df3c7395a791c751b22187a936facbce9f3f168))
    - Don't panic on too short loose object ([`c89c7ff`](https://github.com/GitoxideLabs/gitoxide/commit/c89c7fface26660d1855915b871dbf758150ab6f))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'main' into fix-odb-race ([`30712dc`](https://github.com/GitoxideLabs/gitoxide/commit/30712dc9ab952d29c528ec58bcc467b425489304))
    - Add `tree::EntryMode::is_blob()` to quickly determine if an entry is a blob. ([`5095df9`](https://github.com/GitoxideLabs/gitoxide/commit/5095df949d30b57b09bd68355743d255a897365f))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
    - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
    - Merge branch 'filter-refs' ([`e10554d`](https://github.com/GitoxideLabs/gitoxide/commit/e10554d2a3b9c027353a432b0c84f7d3797b7cae))
    - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
    - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
    - Merge branch 'main' into filter-refs-by-spec ([`1f6e5ab`](https://github.com/GitoxideLabs/gitoxide/commit/1f6e5ab15f5fd8d23719b13e6aea59cd231ac0fe))
    - Merge branch 'fix-522' ([`5869e9f`](https://github.com/GitoxideLabs/gitoxide/commit/5869e9ff2508d5a93c07635277af8764fcb57713))
    - Release git-hash v0.9.9 ([`da0716f`](https://github.com/GitoxideLabs/gitoxide/commit/da0716f8c27b4f29cfff0e5ce7fcb3d7240f4aeb))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-object v0.20.3, git-ref v0.15.4, git-config v0.7.1, git-diff v0.18.0, git-traverse v0.16.3, git-pack v0.22.0, git-odb v0.32.0, git-url v0.7.3, git-transport v0.19.3, git-protocol v0.19.1, git-refspec v0.1.1, git-repository v0.23.0, safety bump 6 crates ([`85a3bed`](https://github.com/GitoxideLabs/gitoxide/commit/85a3bedd68d2e5f36592a2f691c977dc55298279))
    - Refactor ([`867fc20`](https://github.com/GitoxideLabs/gitoxide/commit/867fc208e8542668adcc81f8c03dd4957e956ef7))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
    - Prepare changelogs prior to reelase ([`c06ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/c06ae1c606b6af9c2a12021103d99c2810750d60))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
    - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Prepare changelog prior to release ([`3c50625`](https://github.com/GitoxideLabs/gitoxide/commit/3c50625fa51350ec885b0f38ec9e92f9444df0f9))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`daa71c3`](https://github.com/GitoxideLabs/gitoxide/commit/daa71c3b753c6d76a3d652c29237906b3e28728f))
    - Thanks clippy ([`e1003d5`](https://github.com/GitoxideLabs/gitoxide/commit/e1003d5fdee5d4439c0cf0286c67dec9b5e34f53))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Erge branch 'fix-describe' ([`56d7ad7`](https://github.com/GitoxideLabs/gitoxide/commit/56d7ad7a2e7994545581ad5955c25feb9cefdf4e))
    - Release git-actor v0.9.0, git-object v0.18.0 ([`ef9242b`](https://github.com/GitoxideLabs/gitoxide/commit/ef9242bdb35c02afc36af7c59073d78091fbf504))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Thanks clippy ([`3079e11`](https://github.com/GitoxideLabs/gitoxide/commit/3079e114dc1d2552e023aa793dc10c28258b34da))
    - Merge branch 'main' into mailmap ([`b2df941`](https://github.com/GitoxideLabs/gitoxide/commit/b2df941feaf5ae9fa170fa49270189f3527f2eab))
    - Merge branch 'describe-rev' ([`77b7cd9`](https://github.com/GitoxideLabs/gitoxide/commit/77b7cd9a7813aaa1a15d035ea42c1e3fe4eef8dd))
    - Adapt to breaking changes in git-actor ([`40c48c3`](https://github.com/GitoxideLabs/gitoxide/commit/40c48c390eb796b427ebd516dde92e9538ce5fb7))
    - Upgrade document-features ([`c35e62e`](https://github.com/GitoxideLabs/gitoxide/commit/c35e62e0da9ac1f7dcb863f5f9c69108c728d32e))
    - Release git-actor v0.8.1 ([`08fe550`](https://github.com/GitoxideLabs/gitoxide/commit/08fe5508472f2eb209db8a5fc4e4942a9d7db93d))
    - Release git-object v0.17.1, git-pack v0.16.1 ([`e959af8`](https://github.com/GitoxideLabs/gitoxide/commit/e959af83fa92e8ed87edae6e2d1c6a797964c056))
    - Release git-hash v0.9.2, git-object v0.17.1, git-pack v0.16.1 ([`0db19b8`](https://github.com/GitoxideLabs/gitoxide/commit/0db19b8deaf11a4d4cbc03fa3ae40eea104bc302))
    - Update changelogs prior to git-pack release ([`b7e3a4a`](https://github.com/GitoxideLabs/gitoxide/commit/b7e3a4afdd6417a38aadad35c7f584617e7b47fa))
    - Merge branch 'index-verification' ([`ad3c803`](https://github.com/GitoxideLabs/gitoxide/commit/ad3c8032cee02052ef3940d1d7c950270a0a299a))
    - Release git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`8f57c29`](https://github.com/GitoxideLabs/gitoxide/commit/8f57c297d7d6ed68cf51415ea7ede4bf9263326e))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Merge branch 'oknozor-feat/traversal-sort-by-committer-date' ([`6add377`](https://github.com/GitoxideLabs/gitoxide/commit/6add3773c64a9155c236a36bd002099c218882eb))
    - Add sorting mode to ancestor traversal  #270 ([`eb36a3d`](https://github.com/GitoxideLabs/gitoxide/commit/eb36a3dda83a46ad59078a904f4e277f298a24e1))
    - Rename `commit::ref_iter::Token::into_id()` to `*::try_into_id()` ([`fda2a8d`](https://github.com/GitoxideLabs/gitoxide/commit/fda2a8d2f5f8b7d80b4cc0177d08d6a061f1b8f1))
    - Thanks clippy ([`7dd2313`](https://github.com/GitoxideLabs/gitoxide/commit/7dd2313d980fe7c058319ae66d313b3097e3ae5f))
    - Release git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0 ([`d3f9227`](https://github.com/GitoxideLabs/gitoxide/commit/d3f922781a81e8fbb81aa47afdbe9afeb06d666b))
    - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
    - Adjust changelogs prior to release ([`ec38950`](https://github.com/GitoxideLabs/gitoxide/commit/ec3895005d141abe79764eaff7c0f04153e38d73))
    - Merge branch 'pack-consistency' ([`5982406`](https://github.com/GitoxideLabs/gitoxide/commit/5982406b4e1b26fd383d9ec21a3cf652ec8ab25f))
    - Merge branch 'git-loose-objects' of https://github.com/xmo-odoo/gitoxide into xmo-odoo-git-loose-objects ([`ee737cd`](https://github.com/GitoxideLabs/gitoxide/commit/ee737cd237ad70bf9f2c5e0d3e4557909e495bca))
    - Release git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0 ([`f606fa9`](https://github.com/GitoxideLabs/gitoxide/commit/f606fa9a0ca338534252df8921cd5e9d3875bf94))
    - Better changelog descriptions. ([`f69b2d6`](https://github.com/GitoxideLabs/gitoxide/commit/f69b2d627099639bc144fd94fde678d84a10d6f7))
    - Adjusting changelogs prior to release of git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0, safety bump 5 crates ([`39b40c8`](https://github.com/GitoxideLabs/gitoxide/commit/39b40c8c3691029cc146b893fa0d8d25d56d0819))
    - Adjust changelogs prior to git-pack release ([`ac8015d`](https://github.com/GitoxideLabs/gitoxide/commit/ac8015de710142c2bedd0e4188e872e0cf1ceccc))
    - Move "loose object header" ser/de to git-object ([`3d1565a`](https://github.com/GitoxideLabs/gitoxide/commit/3d1565acfc336baf6487edccefd72d0226141a08))
    - Merge branch 'header-field-multi-improve' of https://github.com/xmo-odoo/gitoxide into xmo-odoo-header-field-multi-improve ([`d88e377`](https://github.com/GitoxideLabs/gitoxide/commit/d88e377c21e566bf86c274d5e87eff06100698b9))
    - Improve error handling of encode::header_field_multi_line & simplify ([`bab9fb5`](https://github.com/GitoxideLabs/gitoxide/commit/bab9fb567e47bb88d27b36f6ffa95c62c14ed80a))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Update changelogs just for fun ([`21541b3`](https://github.com/GitoxideLabs/gitoxide/commit/21541b3301de1e053fc0e84373be60d2162fbaae))
    - Merge branch 'changelog-generation' ([`bf0106e`](https://github.com/GitoxideLabs/gitoxide/commit/bf0106ea21734d4e59d190b424c22743c22da966))
    - Thanks clippy ([`d78d382`](https://github.com/GitoxideLabs/gitoxide/commit/d78d3828c7f80963c0b8803cb64e0ae5e08d0ba3))
    - Thanks clippy ([`4ea1126`](https://github.com/GitoxideLabs/gitoxide/commit/4ea11264296063278977c5539e2d68367475464a))
    - Thanks clippy ([`e56af5a`](https://github.com/GitoxideLabs/gitoxide/commit/e56af5a0846652f177a88771d495bff5973abde3))
    - [object #164] refactor ([`883343b`](https://github.com/GitoxideLabs/gitoxide/commit/883343bbbae431cfb8ffb16f0d39838b0d7636d7))
    - Bump git-object v0.14.0 ([`d4fc81f`](https://github.com/GitoxideLabs/gitoxide/commit/d4fc81f6390443f8c8561d91ac27ea4a6318fb62))
    - [repository #164] Prepare `commit()` for a possible less-allocating future ([`0fd01f7`](https://github.com/GitoxideLabs/gitoxide/commit/0fd01f7071c785c27c56d2c034aac8dcdf690677))
    - [repository #164] generic write_object() ([`c569f83`](https://github.com/GitoxideLabs/gitoxide/commit/c569f83363489dde03c8b9cd01e75d35f5e04dbc))
    - Thanks clippy ([`33a8fb3`](https://github.com/GitoxideLabs/gitoxide/commit/33a8fb34708407fd6b9a9ddabeaab51409aa1b03))
    - [object #164] Allow referenced objects to be serialized as well ([`a98d298`](https://github.com/GitoxideLabs/gitoxide/commit/a98d2985dae2259d72bb91a01548906862fee9f7))
    - Release git-object v0.13.1 ([`2c55ea7`](https://github.com/GitoxideLabs/gitoxide/commit/2c55ea759caa1d317f008966ae388b3cf0ce0f6d))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - [object #190] consistent method naming ([`c5de433`](https://github.com/GitoxideLabs/gitoxide/commit/c5de433e569c2cc8e78f3f84e368a11fe95f522a))
    - [object #190] More conversion methods for Object ([`78bacf9`](https://github.com/GitoxideLabs/gitoxide/commit/78bacf97d669f3adfebdb093054c162cfd5214fa))
    - [repository #190] A way to write objects and the empty tree specifically ([`7c559d6`](https://github.com/GitoxideLabs/gitoxide/commit/7c559d6e1b68bc89220bca426257f383bce586ae))
    - [object #177] cleanup CommitRefIter imports and git_object::Error ([`058f68a`](https://github.com/GitoxideLabs/gitoxide/commit/058f68a9e1cd79fd5a2a1235da42358bc92ed255))
    - [object #177] dissolve 'immutable' module ([`70e11c2`](https://github.com/GitoxideLabs/gitoxide/commit/70e11c21b0637cd250f54381d5490e9976880ad9))
    - [object #177] fix docs ([`2fd23ed`](https://github.com/GitoxideLabs/gitoxide/commit/2fd23ed9ad556b8e46cf650e23f0c6726e304708))
    - [object #177] resolve 'mutable' module ([`b201b32`](https://github.com/GitoxideLabs/gitoxide/commit/b201b3260e3eec98ed71716c1aab1ba4a06ab829))
    - [object #177] refactor ([`216dd0f`](https://github.com/GitoxideLabs/gitoxide/commit/216dd0f10add7a11ebdf96732ed7649d74815d64))
    - [object #177] refactor ([`472e13b`](https://github.com/GitoxideLabs/gitoxide/commit/472e13b27e97a196c644d716cad1801bd62fac71))
    - [object #177] Commit::write_to migration ([`60b9365`](https://github.com/GitoxideLabs/gitoxide/commit/60b936553bef3c9126d46ece9779f08b5eef9a95))
    - [object #177]  commit::RefIter -> CommitRefIter ([`e603306`](https://github.com/GitoxideLabs/gitoxide/commit/e603306e81f392af97aa5afd232653de56bf3ce9))
    - [object #177] migrate immutable::commit into crate::commit ([`45d3934`](https://github.com/GitoxideLabs/gitoxide/commit/45d393438eac2c7ecd47670922437dd0de4cd69b))
    - [object #177] refactor tag write_to ([`7f19559`](https://github.com/GitoxideLabs/gitoxide/commit/7f1955916ae9d7e17be971170c853487e3755169))
    - [object #177] tag::RefIter -> TagRefIter ([`28587c6`](https://github.com/GitoxideLabs/gitoxide/commit/28587c691eb74e5cb097afb2b63f9d9e2561c45d))
    - [object #177] into_mutable() -> into_owned() ([`7e701ce`](https://github.com/GitoxideLabs/gitoxide/commit/7e701ce49efe5d40327770a988aae88692d88219))
    - [object #177] fix docs ([`25d8e7b`](https://github.com/GitoxideLabs/gitoxide/commit/25d8e7b1862bd05489359b162a32c6ad45ecdf9a))
    - [object #177] move mutable objects to crate::* ([`c551c02`](https://github.com/GitoxideLabs/gitoxide/commit/c551c0236c64f3237cb9be7f35159f753d4b871f))
    - [object #177] migrate immutable::tree to crate::tree ([`fa5cd06`](https://github.com/GitoxideLabs/gitoxide/commit/fa5cd0648d5c855060ab2b75ee933851987c2dcf))
    - [object #177] fix docs ([`07be661`](https://github.com/GitoxideLabs/gitoxide/commit/07be6611d1742633815566443f71eef8b85ad5c0))
    - [object #177] move immutable::* to crate::*Ref, start `iter` adjustments ([`461dc53`](https://github.com/GitoxideLabs/gitoxide/commit/461dc53ba3bc07d55fdb4aad7570ba9176a8b360))
    - [object #177] rename immutable::* to immutable::*Ref ([`6deb012`](https://github.com/GitoxideLabs/gitoxide/commit/6deb01291fb382b7fb9206682e319afa81bacc05))
    - Release git-object v0.13.0 ([`708fc5a`](https://github.com/GitoxideLabs/gitoxide/commit/708fc5abd8af4dd7459f388c7092bf35915c6662))
    - Merge branch 'git-ref-refactor' ([`5dbf753`](https://github.com/GitoxideLabs/gitoxide/commit/5dbf753ce2035ffd07e4bce7ceb3bcd4e309c16e))
    - Release git-actor v0.5.0 ([`a684b0f`](https://github.com/GitoxideLabs/gitoxide/commit/a684b0ff96ebfc5e4b3ce78452dc21ce856a6869))
    - [actor #175] refactor ([`ec88c59`](https://github.com/GitoxideLabs/gitoxide/commit/ec88c5905194150cc94db4d4c20e9f4e2f6595c3))
    - Release git-actor v0.4.0 ([`16358c9`](https://github.com/GitoxideLabs/gitoxide/commit/16358c9bf03604857d51bfa4dbfd2fc8c5210da7))
    - [actor #173] fix docs ([`2d7956a`](https://github.com/GitoxideLabs/gitoxide/commit/2d7956a22511d73b767e443dac21b60e93f286dd))
    - [actor #173] rename immutable::Signature to SignatureRef! ([`96461ac`](https://github.com/GitoxideLabs/gitoxide/commit/96461ace776d6b351b313d4f2697f2d95b9e196e))
    - Merge branch 'Byron:main' into main ([`dc58eca`](https://github.com/GitoxideLabs/gitoxide/commit/dc58eca510e5a067acdeaad4b595a34b4598a0cd))
    - Upgrade to nom-7 ([`f0aa3e1`](https://github.com/GitoxideLabs/gitoxide/commit/f0aa3e1b5b407b2afd187c9cb622676fcddaf706))
    - [smart-release #162] use TreeRef capabilities to lookup path ([`51d1943`](https://github.com/GitoxideLabs/gitoxide/commit/51d19433e6704fabb6547a0ba1b5c32afce43d8b))
    - [repository #162] what could be a correct implementation of a tree path lookup ([`1f638ee`](https://github.com/GitoxideLabs/gitoxide/commit/1f638eee0aa5f6e1cc34c5bc59a18b5f22af4cbc))
    - Release git-object v0.12.2 ([`6e58edd`](https://github.com/GitoxideLabs/gitoxide/commit/6e58edde2d7c881a6b957b7efafb63e2f517c9b4))
    - [object] argh, remove these tests for now no time for this ([`13d627d`](https://github.com/GitoxideLabs/gitoxide/commit/13d627d19aae3bb9d44ed0e9304e373e90f51547))
    - [object] simply exclude the feature from testing for now… ([`adba3b9`](https://github.com/GitoxideLabs/gitoxide/commit/adba3b982c4b21889615afafcfcaa7ae1f91661d))
    - [object] fix magically smaller object size expectation ([`bf4d2d7`](https://github.com/GitoxideLabs/gitoxide/commit/bf4d2d7c0a33a3f35646f776edce4b829f086f66))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - Release git-object v0.12.1 ([`086baa2`](https://github.com/GitoxideLabs/gitoxide/commit/086baa261d0874e541e374d51d427727aa37a8ee))
    - Remove dev-dependency cycles by removing their version ([`c40faca`](https://github.com/GitoxideLabs/gitoxide/commit/c40faca41632cd2a226daf4ddf5293b65d1fdc82))
    - Release git-object v0.12.0 ([`7006150`](https://github.com/GitoxideLabs/gitoxide/commit/7006150ac314d19814608723f69f6e70a72f9262))
    - Release git-actor-0.3.1 ([`727087d`](https://github.com/GitoxideLabs/gitoxide/commit/727087dca243da4bc40bc87611a2f66234565be7))
    - (cargo-release) version 0.11.0 ([`a5be31c`](https://github.com/GitoxideLabs/gitoxide/commit/a5be31c4cf7c0b538a1ed4a52ff5c3a992c6feff))
    - (cargo-release) version 0.5.0 ([`bf15c2a`](https://github.com/GitoxideLabs/gitoxide/commit/bf15c2a2f285046b094093760c1969007ee75e25))
    - (cargo-release) version 0.3.0 ([`64efc05`](https://github.com/GitoxideLabs/gitoxide/commit/64efc0534ddc372b6e668b23c1e9d276098679c9))
    - (cargo-release) version 0.4.0 ([`70ef344`](https://github.com/GitoxideLabs/gitoxide/commit/70ef3442775b54ba9e4ee9ebfffb37af9804cc5b))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - (cargo-release) version 0.4.0 ([`0d5c8b9`](https://github.com/GitoxideLabs/gitoxide/commit/0d5c8b96dfdfb96e4fc82623f756f6c7f7046e90))
    - (cargo-release) version 0.2.0 ([`8ff5115`](https://github.com/GitoxideLabs/gitoxide/commit/8ff511583e6d859e43ffda0ef75e2fecce3ed03c))
    - Clippy on tests and thanks clippy ([`a77a71c`](https://github.com/GitoxideLabs/gitoxide/commit/a77a71cf02d328a2a964388928d6b2a235a0aa85))
    - Thanks clippy ([`e1964e4`](https://github.com/GitoxideLabs/gitoxide/commit/e1964e43979b3e32a5d4bfbe377a842d2c0b10ea))
    - [ref] fix build (bad find&replace) ([`467395f`](https://github.com/GitoxideLabs/gitoxide/commit/467395f19ce13ff8cd62499573d3cd4cb2e7797f))
    - [ref] refactor ([`e26c72f`](https://github.com/GitoxideLabs/gitoxide/commit/e26c72fb1bf9392932ffe42843f3dec52c7bbd7d))
    - Remove unnecessary pub(crate) exports ([`3d2456e`](https://github.com/GitoxideLabs/gitoxide/commit/3d2456e11709f0461b37c6df55ecc3861ca4cab5))
    - [ref] refactor ([`207a799`](https://github.com/GitoxideLabs/gitoxide/commit/207a799c1fcf490425f2e5dcf8274da83125af6f))
    - [ref] flexible and simple support for different hash lengths ([`9c2edd5`](https://github.com/GitoxideLabs/gitoxide/commit/9c2edd537fb86d2d7db874ec976d0cb1b8ec7c2e))
    - Thanks clippy ([`c437304`](https://github.com/GitoxideLabs/gitoxide/commit/c43730496ac5e1f7e66ee226099a4e99e151e97d))
    - [object] Add feature toggle for verbose errors… ([`4b63d8a`](https://github.com/GitoxideLabs/gitoxide/commit/4b63d8a8709a2674d287879c4d6538a74cd7869b))
    - [object] support for verbose errors for object parsing ([`8156f10`](https://github.com/GitoxideLabs/gitoxide/commit/8156f1037b87424864db73888108be12dedb5169))
    - [object] refactor ([`6f63983`](https://github.com/GitoxideLabs/gitoxide/commit/6f639835362224bade27dd4f45c275544a39625d))
    - [object] Generalize nom error handling and use nom-errors instead of custom ones ([`47c8a97`](https://github.com/GitoxideLabs/gitoxide/commit/47c8a97194c9e401ee311234a269f8b8f3650ba1))
    - [object] remove unused dependencies ([`2f01e46`](https://github.com/GitoxideLabs/gitoxide/commit/2f01e46a9b30f1231adf1e79c5923843e63cad86))
    - [object] cleanup parsing error handling by removing NomDetail ([`e91cb40`](https://github.com/GitoxideLabs/gitoxide/commit/e91cb405381d84bc1021c3ac30dfe6061788f9b1))
    - [object] refactor ([`1ddb5c0`](https://github.com/GitoxideLabs/gitoxide/commit/1ddb5c07b75aa2b9a9536125fbba1fc862b7fe34))
    - [object] replace custom context impl with the one by nom ([`9a6692d`](https://github.com/GitoxideLabs/gitoxide/commit/9a6692d034976dbcf011b587140c7f47fbc583e2))
    - [object] refactor ([`8205429`](https://github.com/GitoxideLabs/gitoxide/commit/8205429b2ac160525a7449e50edf04aaf027f12c))
    - [actor] git-object uses git-actor ([`d01dd2f`](https://github.com/GitoxideLabs/gitoxide/commit/d01dd2f9e9e8e2b81cdb1131a436d32b5819b731))
    - [actor] make signature parsing public, exposing nom :/ ([`a627972`](https://github.com/GitoxideLabs/gitoxide/commit/a627972ecc53d38210c826f851ea9c5fec17b9cb))
    - [refs] try to get structure in place for reflog parsing ([`727c66a`](https://github.com/GitoxideLabs/gitoxide/commit/727c66a2560c00cc8e01fbe47503ffbb67147c59))
    - Thanks clippy ([`6200ed9`](https://github.com/GitoxideLabs/gitoxide/commit/6200ed9ac5609c74de4254ab663c19cfe3591402))
    - (cargo-release) version 0.3.0 ([`87db688`](https://github.com/GitoxideLabs/gitoxide/commit/87db688f23475d7232731429d770848aea228492))
    - (cargo-release) version 0.3.0 ([`6b33678`](https://github.com/GitoxideLabs/gitoxide/commit/6b33678f83e6d261ca15c4a7634ff5b4e66d81dd))
    - (cargo-release) version 0.2.0 ([`3286e42`](https://github.com/GitoxideLabs/gitoxide/commit/3286e42547b59df6365087cbae9ce1c9c959faad))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - (cargo-release) version 0.2.0 ([`1327894`](https://github.com/GitoxideLabs/gitoxide/commit/132789475400abe660b30ef6d2c5ff57821dd2c4))
    - [git-object] use git-validate crate ([`4ba98e8`](https://github.com/GitoxideLabs/gitoxide/commit/4ba98e824417d1c58998fabee88549700a714bcf))
    - [git-object] refactor ([`d64d326`](https://github.com/GitoxideLabs/gitoxide/commit/d64d3266167ee224b651cc24c4dbc8d88c9c9876))
    - [git-ref] the first failing test ([`7e802a0`](https://github.com/GitoxideLabs/gitoxide/commit/7e802a0576230dfc666c253d484ea255f265f92f))
    - Switch to latest nom ([`859e57e`](https://github.com/GitoxideLabs/gitoxide/commit/859e57eae93c3490523b7ed98f7a606acbd87a2f))
    - [git-ref] clear it out and move existing functionality to git-object ([`fa548ce`](https://github.com/GitoxideLabs/gitoxide/commit/fa548ce94db3dd3969add494756fcc34e48985a3))
    - (cargo-release) version 0.5.0 ([`b6b5856`](https://github.com/GitoxideLabs/gitoxide/commit/b6b58560b7c3bc88e2b8b780be5ceb4cb508a346))
    - [pack-gen] refactor ([`61554e2`](https://github.com/GitoxideLabs/gitoxide/commit/61554e2effcbafef9cff0b407351c2fae0d2916c))
    - [pack-gen] tag support for tree traversal ([`28ed260`](https://github.com/GitoxideLabs/gitoxide/commit/28ed260a73554d261c9b00c8ae9a46e66f123e6f))
    - (cargo-release) version 0.10.0 ([`5d7ee6a`](https://github.com/GitoxideLabs/gitoxide/commit/5d7ee6a105abbb6efeed8624bade936bb59dbc55))
    - [pack-gen] more tests for Tag iterator ([`b69d6d6`](https://github.com/GitoxideLabs/gitoxide/commit/b69d6d6590ea9d8de4a50e31ab9f331a2e21faab))
    - [pack-gen] the first green test for Tag iterators ([`df5ef8a`](https://github.com/GitoxideLabs/gitoxide/commit/df5ef8a53cb4007058137890b414af510025fccf))
    - (cargo-release) version 0.9.0 ([`84897fd`](https://github.com/GitoxideLabs/gitoxide/commit/84897fd8e6e1b0269da0303d6a0de8f9e0eb58e5))
    - Allow empty trees when parsing them at once, fixes #79 ([`d34fd19`](https://github.com/GitoxideLabs/gitoxide/commit/d34fd19db5b3802ae9c677a6cf481f42e8a7e073))
    - Refactor ([`9870923`](https://github.com/GitoxideLabs/gitoxide/commit/9870923ce02d20beb98be5e4bb76ff8081804054))
    - [hours-demo] computation seems to work better now ([`26ecca2`](https://github.com/GitoxideLabs/gitoxide/commit/26ecca2133af287ddc9146fb3a3fc73dddc945e9))
    - Refactor ([`2d00c4e`](https://github.com/GitoxideLabs/gitoxide/commit/2d00c4ed6be5baa1c3389a61102e25eb7d543465))
    - [hours-demo] Maybe the pinnacle of performance… ([`f70c61a`](https://github.com/GitoxideLabs/gitoxide/commit/f70c61ab56b4153030d524da69a514a667c6abb7))
    - Remove debug-assert which doesn't hold - it's OK to have empty commit messages ([`13abc2d`](https://github.com/GitoxideLabs/gitoxide/commit/13abc2d70c9aa42fb76e71d44c9c711e2780a5ba))
    - And it's a wrap for git-diff docs for now ([`9e09dd5`](https://github.com/GitoxideLabs/gitoxide/commit/9e09dd560a23d52d0469ce4fc13de01f7efce227))
    - [traversal] first impl based on git-odb::traver ([`76a3017`](https://github.com/GitoxideLabs/gitoxide/commit/76a3017b60d41957f5fea56bf7b2b2bf41aae0d5))
    - A new crate: git-traverse ([`1a9af50`](https://github.com/GitoxideLabs/gitoxide/commit/1a9af50f1fca0e7e939f339b885c66dcb95e44e5))
    - (cargo-release) version 0.8.0 ([`a1ce210`](https://github.com/GitoxideLabs/gitoxide/commit/a1ce210003ff07bf11291018bb182cbc7913647b))
    - (cargo-release) version 0.3.0 ([`e9665c7`](https://github.com/GitoxideLabs/gitoxide/commit/e9665c784ae7e5cdaf662151395ee2355e9b57b6))
    - [traversal] add CommitIter::tree_id() convenience method ([`6affd9d`](https://github.com/GitoxideLabs/gitoxide/commit/6affd9d90d56d89774fcd4843638309a198815bf))
    - [traversal] trying to get things done with gitoxide shows some teeth… ([`3fee661`](https://github.com/GitoxideLabs/gitoxide/commit/3fee661af8d67e277e8657606383a670f17e7825))
    - Refactor; better iter error handling tests ([`9fe139b`](https://github.com/GitoxideLabs/gitoxide/commit/9fe139b85c350c8cbb78975a94f4548130764b1c))
    - [tree-diff] more tests for the tree iterator ([`91b5a02`](https://github.com/GitoxideLabs/gitoxide/commit/91b5a029337200a2873a21696020dcda08e335cb))
    - Test error handling of commit iteration ([`fcec4b4`](https://github.com/GitoxideLabs/gitoxide/commit/fcec4b43f7c1d72680431ec3522b0db94728507f))
    - Thanks clippy ([`41418ed`](https://github.com/GitoxideLabs/gitoxide/commit/41418ede15be9d0d18e49c34e4482c5701851404))
    - Fix serde support for commit iter token ([`3bfcb49`](https://github.com/GitoxideLabs/gitoxide/commit/3bfcb49814ed5f14dd66206a54a9b85f13edd9d9))
    - [tree-diff] all the tests for commit iter ([`7ebea87`](https://github.com/GitoxideLabs/gitoxide/commit/7ebea87b91c7cae3378fa3a5780d6c58e319c006))
    - [tree-diff] more tests ([`4f81450`](https://github.com/GitoxideLabs/gitoxide/commit/4f81450b13bfc14cede1bec3234d33ec0844ac3d))
    - [tree-diff] And there is a working commit iterator, needs more tests ([`d991847`](https://github.com/GitoxideLabs/gitoxide/commit/d99184782e9e79b517b7703ab41fefdc2424994e))
    - [tree-diff] A complete nearly working impl of a Commit iterator ([`4711821`](https://github.com/GitoxideLabs/gitoxide/commit/4711821dd54193737cff76ce904b18d29b518ac2))
    - Frame for Commit iterator ([`796b74a`](https://github.com/GitoxideLabs/gitoxide/commit/796b74a09cf1b4c8c4694d877a76da94d425bdc0))
    - First failing test for commit iterator; store two parents without alloc ([`8337514`](https://github.com/GitoxideLabs/gitoxide/commit/8337514378148d72dc7f6d7474d6d0b794759589))
    - [tree-diff] one more test green + refactor ([`bc5549d`](https://github.com/GitoxideLabs/gitoxide/commit/bc5549db2ad16222761219d8652caf64867a889f))
    - [tree-diff] refactor into iterator based model ([`29b527a`](https://github.com/GitoxideLabs/gitoxide/commit/29b527aaea101c9b4e885db1c6d3533ef2310c54))
    - [tree-diff] The least intrusive way to allow dealing with tree iterators ([`d41dd3c`](https://github.com/GitoxideLabs/gitoxide/commit/d41dd3c38ee34b250a4f5de120d7ae3e04e3212d))
    - [tree-diff] prototype an immutable tree iterator to avoid entry allocs ([`f38e5cd`](https://github.com/GitoxideLabs/gitoxide/commit/f38e5cdcd072873707a21f0b73c491ef1f1c7a8f))
    - [tree-diff] A step closer to handling additions in a directory ([`a11f210`](https://github.com/GitoxideLabs/gitoxide/commit/a11f210bec2c6c55bcf8cebe00e116e835306360))
    - Refactor ([`a4d5f99`](https://github.com/GitoxideLabs/gitoxide/commit/a4d5f99c8dc99bf814790928a3bf9649cd99486b))
    - Refactor ([`633cba7`](https://github.com/GitoxideLabs/gitoxide/commit/633cba7c1ff1f63c32613bedf963d1bd89afaee1))
    - First sketch of diff API ([`fc3f2b7`](https://github.com/GitoxideLabs/gitoxide/commit/fc3f2b7066538e31f8d4bb1053d70dcabd5fbab1))
    - Better ergonomics for accessing decoded objects ([`ae3eab6`](https://github.com/GitoxideLabs/gitoxide/commit/ae3eab6d6e4b96e207372fa8cb82f5ac9833e5e4))
    - Thanks clippy ([`8295548`](https://github.com/GitoxideLabs/gitoxide/commit/829554805520170f69cadc61e6be5e7255737cff))
    - Refactor ([`9d03843`](https://github.com/GitoxideLabs/gitoxide/commit/9d03843eeae388738d70b4251166081542893749))
    - Fix debug assert, thanks gitpython ([`fe954b9`](https://github.com/GitoxideLabs/gitoxide/commit/fe954b9f6d26bd8629f24a01bd2a06f9800deed0))
    - More explicit expectations towards entries in mutable Trees ([`d94f84c`](https://github.com/GitoxideLabs/gitoxide/commit/d94f84ceac637d2b6495be01dfc8eeb2494922f2))
    - Refactor ([`f19ea33`](https://github.com/GitoxideLabs/gitoxide/commit/f19ea33709f7c31873e46d896ed7b6d06607f1e7))
    - An even better name for decode errors ([`f270850`](https://github.com/GitoxideLabs/gitoxide/commit/f270850ff92eab15258023b8e59346ec200303bd))
    - Make clear it's a decode error we are using there ([`f45cb4b`](https://github.com/GitoxideLabs/gitoxide/commit/f45cb4b62122559e5701923e0a23dd5791ee2ced))
    - Rename git-object::(owned->mutable)|(borrowed|immutable) #(67) ([`91ee558`](https://github.com/GitoxideLabs/gitoxide/commit/91ee55893bf4b27a47d86d51bae6f99b59b69147))
    - The first basic traversal utility #(67) ([`ea6610b`](https://github.com/GitoxideLabs/gitoxide/commit/ea6610b9157d8d0dabd2ddd07c45dc6651b9cf84))
    - (cargo-release) version 0.7.0 ([`b900914`](https://github.com/GitoxideLabs/gitoxide/commit/b900914a00292217ba7b9bcac260591800395287))
    - (cargo-release) version 0.2.0 ([`4ec09f4`](https://github.com/GitoxideLabs/gitoxide/commit/4ec09f4d2239ea1d44f7145027e64191bf2c158c))
    - Thanks clippy ([`cefbf3e`](https://github.com/GitoxideLabs/gitoxide/commit/cefbf3e02edebd1875cd2762eb231e6c379b1ebb))
    - Upgrade depdendencies ([`e4a7711`](https://github.com/GitoxideLabs/gitoxide/commit/e4a77112ee4f5d0ab61d0678aab8ee090335740c))
    - Improved high-level docs for git-object ([`60036f2`](https://github.com/GitoxideLabs/gitoxide/commit/60036f20328600f0faaaf21ca30f1b9d0275d15f))
    - Add missing '.' at end of doc comments ([`7136854`](https://github.com/GitoxideLabs/gitoxide/commit/71368544f97369a4d371d43513607c4805bd0fd0))
    - All crates use git-hash::Kind and its types, sometimes through git-object ([`124c171`](https://github.com/GitoxideLabs/gitoxide/commit/124c171aaf546d8977e9913ff84e65383a80ee98))
    - First round of git-object doc proof reading ([`524ce51`](https://github.com/GitoxideLabs/gitoxide/commit/524ce51eb3e606b1225a23fce62df2199799d4f3))
    - (cargo-release) version 0.5.0 ([`fc7d600`](https://github.com/GitoxideLabs/gitoxide/commit/fc7d600ac2c438c8b6b91f67cb69b0ac5ec37675))
    - `deny(missing_docs)` for git-object ([`8525684`](https://github.com/GitoxideLabs/gitoxide/commit/8525684c6c69677f3e1b40a3673a817e111e9bff))
    - More docs for owned git-object ([`b79101d`](https://github.com/GitoxideLabs/gitoxide/commit/b79101d714f59a42a30eb47776486a212ec0f738))
    - A few more comments in git-object ([`171d269`](https://github.com/GitoxideLabs/gitoxide/commit/171d269e428f711b163f6644ebf0c44c1279d497))
    - Thanks clippy ([`ba9b3c2`](https://github.com/GitoxideLabs/gitoxide/commit/ba9b3c2345887353e02fc081be80733f1c5e22d9))
    - Refactor ([`d5d7cf9`](https://github.com/GitoxideLabs/gitoxide/commit/d5d7cf9d3f42d83652a7b81bc6e1ee6731396d6b))
    - More git-object docs ([`ba595f6`](https://github.com/GitoxideLabs/gitoxide/commit/ba595f6d4864eafc64f31460f7192cb48abd408a))
    - More docs of git-object::owned ([`0620dce`](https://github.com/GitoxideLabs/gitoxide/commit/0620dce7a3ac368354c73e3927eb96a6e4990f0c))
    - Docs for git-object::borrowed ([`68e524d`](https://github.com/GitoxideLabs/gitoxide/commit/68e524d079fe9042ebba1e33457f934a64018623))
    - Docs for git-object::borrowed::commit ([`c5c1df0`](https://github.com/GitoxideLabs/gitoxide/commit/c5c1df031aa391e0e65d0540f8414cbd1d1aa39f))
    - Merge branch 'commit-graph' into main ([`9cb09b2`](https://github.com/GitoxideLabs/gitoxide/commit/9cb09b248796f0ff5c9d3f3e857de4731324cfd5))
    - Add and use borrowed::Id::null_sha1() ([`c717492`](https://github.com/GitoxideLabs/gitoxide/commit/c717492d0038f55a6f21b48937b56a756890d214))
    - Updated `expect` message ([`e8d8d93`](https://github.com/GitoxideLabs/gitoxide/commit/e8d8d9351168b5423c44626ed8ac81cf7b013a02))
    - Update error message for type name ([`92cbb13`](https://github.com/GitoxideLabs/gitoxide/commit/92cbb1314657abaad560d068e7395a70769f0592))
    - Document borrowed odb objects ([`7626f7f`](https://github.com/GitoxideLabs/gitoxide/commit/7626f7f3af885f1b95801f9dbc71bee0bc77400e))
    - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Finish removal of rust 2018 idioms ([`0d1699e`](https://github.com/GitoxideLabs/gitoxide/commit/0d1699e0e0bc9052be0a74b1b3f3d3eeeec39e3e))
    - Refactor ([`e4bcfe6`](https://github.com/GitoxideLabs/gitoxide/commit/e4bcfe6406b14feffa63598c7cdcc8ecc73222bd))
    - (cargo-release) version 0.4.0 ([`0d7b60e`](https://github.com/GitoxideLabs/gitoxide/commit/0d7b60e856325009431172e1df742a1cd2165575))
    - (cargo-release) version 0.4.0 ([`f9dd225`](https://github.com/GitoxideLabs/gitoxide/commit/f9dd225afc4aafde1a8b8148943f56f2c547a9ea))
    - [clone] proper parsing of V1 refs ([`d262307`](https://github.com/GitoxideLabs/gitoxide/commit/d26230727ef795a819852bc82d6c2e9956809d8c))
    - [clone] Don't expose hex-error in public interfaces anymore ([`92dab30`](https://github.com/GitoxideLabs/gitoxide/commit/92dab3033890fe26fe2b901d87abe16abd065cce))
    - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
    - Refactor ([`a0bebd1`](https://github.com/GitoxideLabs/gitoxide/commit/a0bebd17500bccc08ed5a1c16e2ffcde89c71052))
    - Bump minor version to 0.3 ([`4351e28`](https://github.com/GitoxideLabs/gitoxide/commit/4351e2871c9dcf342b8471fffa74cae338a53269))
    - Update to quick-error 2.0 ([`4b1b784`](https://github.com/GitoxideLabs/gitoxide/commit/4b1b7849b47a54092b49821c39e864c86adda979))
    - Thanks clippy ([`62d2ff3`](https://github.com/GitoxideLabs/gitoxide/commit/62d2ff383c5f7fe884057c70868569a811a73e00))
    - Organize object type comparisons by probability… ([`19a5d94`](https://github.com/GitoxideLabs/gitoxide/commit/19a5d9465f7962cfcc39ea31a2c84be6235e40ed))
    - Don't cause re-allocs of the compression buffer ([`2bb6fd2`](https://github.com/GitoxideLabs/gitoxide/commit/2bb6fd26235825484a8f60a49455fee71f08236c))
    - Reduce memory consumption ([`6d1a7a1`](https://github.com/GitoxideLabs/gitoxide/commit/6d1a7a1292e8065d0a777cb6acd34776b1e23696))
    - Also read the pack trailer during iteration ([`98a8e17`](https://github.com/GitoxideLabs/gitoxide/commit/98a8e17e791b6bcd92149d7ff68cbc9d9ceee087))
    - Refactor; better tests ([`12d14bf`](https://github.com/GitoxideLabs/gitoxide/commit/12d14bfe2aa089723a395287c5100aad6e838935))
    - First step towards putting the index file into position ([`d994c74`](https://github.com/GitoxideLabs/gitoxide/commit/d994c74d7cd9c9c004bf27f0b2ac23558ce9c50d))
    - Improve looks of documentation ([`11a32eb`](https://github.com/GitoxideLabs/gitoxide/commit/11a32ebc2209d1a05eb4c4ec5131e85dfb43d9f6))
    - Finish Sink implementation ([`84f7908`](https://github.com/GitoxideLabs/gitoxide/commit/84f7908b1883ed6c484ca4e522ac530c8cc161d5))
    - Introduce hash kind, as this should be specified when writing an object ([`f5d0acf`](https://github.com/GitoxideLabs/gitoxide/commit/f5d0acf61ac5dd815bc5ece4462eb9a43dd9c44a))
    - (cargo-release) version 0.2.0 ([`76fe0ab`](https://github.com/GitoxideLabs/gitoxide/commit/76fe0ab5f0b58504a5ea5adb74b349b9d588e51e))
    - (cargo-release) version 0.2.0 ([`d350a13`](https://github.com/GitoxideLabs/gitoxide/commit/d350a13784685ea82b84646b18736986aeb68146))
    - Beautifully implement shared extra-header access ([`920d1ac`](https://github.com/GitoxideLabs/gitoxide/commit/920d1accc92d67019f0e654f8c4ab5c95d798925))
    - Roundtrip Rust repo in stress test; accept more diverse trees when parsing ([`0347cdb`](https://github.com/GitoxideLabs/gitoxide/commit/0347cdbf473d80c016745ffbaf582832fe2eba2a))
    - Make sure we write out trailing newlines properly in multi-line headers! ([`7f044c3`](https://github.com/GitoxideLabs/gitoxide/commit/7f044c36279aadfd7a2ebeecedd7f2c20b2b7b52))
    - Consume PGP signature in tags fully ([`ffd6c31`](https://github.com/GitoxideLabs/gitoxide/commit/ffd6c31aa3adecc2dea6357373d88a495d63ba0d))
    - Support for very special tree entry mode… ([`2be2c9d`](https://github.com/GitoxideLabs/gitoxide/commit/2be2c9d31563b147f0f2a5c1cd03279c79f1dd6b))
    - Make tagger signature optional ([`3358f9a`](https://github.com/GitoxideLabs/gitoxide/commit/3358f9ae539c7b7878d87a209d678d2f08f94b1b))
    - Remove now unused pgp_signature field - it's in extra-headers ([`c8c937c`](https://github.com/GitoxideLabs/gitoxide/commit/c8c937c505e455572544a1a9da1b991ef4662b97))
    - Proper support for extra-headers ([`d0feb2b`](https://github.com/GitoxideLabs/gitoxide/commit/d0feb2b5b30f9719bf3b40ac5b74f8a5a8515bc9))
    - Abiility to read mergetags (for now only these) as extra-headers ([`bd3a2db`](https://github.com/GitoxideLabs/gitoxide/commit/bd3a2db1068ce7509612ef1be0a108b7bb45ed49))
    - Switch to latest quick-error ([`9760856`](https://github.com/GitoxideLabs/gitoxide/commit/976085614ee13a19fc1347209259a3dcf36ef95b))
    - Fully implement --encode and --re-encode flags ([`a7cfac8`](https://github.com/GitoxideLabs/gitoxide/commit/a7cfac83ddd859d9c2c25e457c0d7043738792dc))
    - Empty trees are allowed, and they are special, too ([`6bed200`](https://github.com/GitoxideLabs/gitoxide/commit/6bed200ec1a528574edabf5783e9ebfb00add56d))
    - Refactor ([`56b66ac`](https://github.com/GitoxideLabs/gitoxide/commit/56b66ac069f24635a8fa74b4b2231dfb0a82a1ef))
    - Basic top-level object round-tripping ([`e851cbe`](https://github.com/GitoxideLabs/gitoxide/commit/e851cbe585525b3e50114eb8d2a0662149bf2019))
    - Refactor ([`ec5e50f`](https://github.com/GitoxideLabs/gitoxide/commit/ec5e50f607d59302d6db3944f6ea7b667f820927))
    - Implement blob ([`f30caf4`](https://github.com/GitoxideLabs/gitoxide/commit/f30caf4ff69fee36e65a2e404910b88e06d539bc))
    - Refactor ([`335e98a`](https://github.com/GitoxideLabs/gitoxide/commit/335e98ab3a2e9c05141f1cd218197079bb51cfa5))
    - Tree roundtrip ([`8b26a0e`](https://github.com/GitoxideLabs/gitoxide/commit/8b26a0e16c838270290cb3ac02b029100afe6b46))
    - Prepare for writing out owned trees ([`2b6eced`](https://github.com/GitoxideLabs/gitoxide/commit/2b6eced325057a884d56ed9db755a8699cbf8d00))
    - Manual deserialize implementation, for now ([`9f46efd`](https://github.com/GitoxideLabs/gitoxide/commit/9f46efd625d45a9ad947e9f7bc6f31f4426f3cfc))
    - Use borrowed::Id in trees for full type safety ([`5d57c1f`](https://github.com/GitoxideLabs/gitoxide/commit/5d57c1f7e3b9a84f7b46a4378015572155f3104b))
    - Refactor ([`f7b8826`](https://github.com/GitoxideLabs/gitoxide/commit/f7b8826ba144f54f3a3fe6096a5daafd29e25002))
    - Commit round-tripping works with multi-line signatures ([`b692b0a`](https://github.com/GitoxideLabs/gitoxide/commit/b692b0aa3f38507697096e671c990700d25933c8))
    - First attempts to roundtrip signatures shows I parse it wrongly :D ([`1b48367`](https://github.com/GitoxideLabs/gitoxide/commit/1b48367d02fde977ed4acab63e295c0c5feec357))
    - Prepare for allowing an owned, processed version of multi-line headers ([`f966e7f`](https://github.com/GitoxideLabs/gitoxide/commit/f966e7f26cbbe99e5508215adaacf073e108bf48))
    - First attempt to round-trip multi-line headers ([`645ef94`](https://github.com/GitoxideLabs/gitoxide/commit/645ef946653caf2eed571d83c61e8b7d7c1cf4b4))
    - Single-line header support ([`478c09e`](https://github.com/GitoxideLabs/gitoxide/commit/478c09e54cc73dcc5cce7aea6bc0702882c5f882))
    - The first basic version of commit serialization ([`5319f64`](https://github.com/GitoxideLabs/gitoxide/commit/5319f64036e09bce97285db19f30f988a5039761))
    - Make reusing round-trip code easier ([`3b9d66c`](https://github.com/GitoxideLabs/gitoxide/commit/3b9d66c932075feb08cdf2967f7698daef9fd3ff))
    - Refactor ([`987787e`](https://github.com/GitoxideLabs/gitoxide/commit/987787e3084bbfc948ed3ca464909a2912f7b653))
    - Fix tests on windows, by ignoring them ([`512ed6c`](https://github.com/GitoxideLabs/gitoxide/commit/512ed6c915b3db2cd3353ea23b945652ad1bef50))
    - Use borrowed::Id everywhere ([`9f876f0`](https://github.com/GitoxideLabs/gitoxide/commit/9f876f04feaa3fd3bba9729fff7667708dc0c4be))
    - Move git_object::Id into git_object::owned::Id - much better already! ([`50c7136`](https://github.com/GitoxideLabs/gitoxide/commit/50c71368a69f57b0a43061df105685e992ed384a))
    - Basic integration of borrowed Id; translate between owned and borrowed ([`84ff638`](https://github.com/GitoxideLabs/gitoxide/commit/84ff638a183567593ace8056de2a856304d29d1d))
    - Prepare to allow Id be owned and borrwed; abstract over hash type ([`d883c31`](https://github.com/GitoxideLabs/gitoxide/commit/d883c31dd14f253a3af153616007c9231fdf265a))
    - Introduce the notion of IdRef ([`7007361`](https://github.com/GitoxideLabs/gitoxide/commit/700736197b903cb6fe9ed60718e49e4be44199a7))
    - Use statically known borrowed arrays for perfect type safety! ([`3ead048`](https://github.com/GitoxideLabs/gitoxide/commit/3ead048bb999e6266831df2ca6c2022013529ab2))
    - Refactor ([`766f3e4`](https://github.com/GitoxideLabs/gitoxide/commit/766f3e491dc6ebcca20753cda3487545268721eb))
    - Tags can write signatures ([`a48275e`](https://github.com/GitoxideLabs/gitoxide/commit/a48275e65bee7f544c19bc81307660ed4f60b8fa))
    - Tags can write a message properly ([`b590b77`](https://github.com/GitoxideLabs/gitoxide/commit/b590b779a6f168db377bf5b4b796a4813bd19ccb))
    - Green tests as basic tags can now be serialied ([`62a02b4`](https://github.com/GitoxideLabs/gitoxide/commit/62a02b490055d9b95a5aae3cbe1641f42ff69df8))
    - More tests for signature serialization ([`5000f30`](https://github.com/GitoxideLabs/gitoxide/commit/5000f30bd0085c0afacf2c32d8a31224ec7337d0))
    - Time serialization ([`1eb1e36`](https://github.com/GitoxideLabs/gitoxide/commit/1eb1e36992f9973977b4d94d55348b7a3eecd3fb))
    - Prepare writing of time as part of signature ([`f560bc5`](https://github.com/GitoxideLabs/gitoxide/commit/f560bc50a2a2e4c9697c17b59ec5cf4122992fab))
    - Add new 'git-ref' crate; place ref name validation code there ([`1a0e84e`](https://github.com/GitoxideLabs/gitoxide/commit/1a0e84e627b17be1b1fb53b4dc98ab78e9cfb9a7))
    - Refactor ([`b4392e8`](https://github.com/GitoxideLabs/gitoxide/commit/b4392e880ed67464af9e8cfa2e343d10f23a949f))
    - Some more boilerplate to actually implement complete ref name checking ([`087857a`](https://github.com/GitoxideLabs/gitoxide/commit/087857a56654537fdfb5bfa6c66745184027517f))
    - Very basic first steps of validated serialization ([`d3fd5ff`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd5ffe10015e2a13200a1fef5bd903532f81af))
    - It's probably OK to consume the borrowed objects when converting them to owned ([`101ddd5`](https://github.com/GitoxideLabs/gitoxide/commit/101ddd586d4250aa5b3c8c6f8076456ae997faec))
    - Try basics of roundtrip without consuming the source object ([`581794e`](https://github.com/GitoxideLabs/gitoxide/commit/581794efcf4577c21f2ff078ba7844a71b47c0aa))
    - Refactor ([`bca1f16`](https://github.com/GitoxideLabs/gitoxide/commit/bca1f16a6f3da497e3488e333d5ebc99e39ee689))
    - First sketch of owned Tag in preparation for round-tripping ([`fa2745a`](https://github.com/GitoxideLabs/gitoxide/commit/fa2745a5d5f7b6c4e02177e4080db7df6603b9fc))
    - Refactor ([`90ae25d`](https://github.com/GitoxideLabs/gitoxide/commit/90ae25d39aa4540fc2785eb7cb189eee102895c0))
    - Refactor ([`256581b`](https://github.com/GitoxideLabs/gitoxide/commit/256581bad6692a458b331b712d16ce2d5143cb75))
    - 'data -> 'a as it's shorter and also more idiomatic ([`71821e9`](https://github.com/GitoxideLabs/gitoxide/commit/71821e938887f448f1458642eda2bac365f2aa85))
    - Refactor ([`dedd4dc`](https://github.com/GitoxideLabs/gitoxide/commit/dedd4dc91c26dfef368307345bb9e8d49637207c))
    - Apply cargo-diet (better late than never :D) ([`295fc81`](https://github.com/GitoxideLabs/gitoxide/commit/295fc81a2e0e31d6d83eba7e169dc4ed05038083))
    - Make crates publishable ([`5688a34`](https://github.com/GitoxideLabs/gitoxide/commit/5688a3427ff3673e1422d43106f4d685fa837aed))
    - Handle windows newlines in test suite for packs as well. ([`ebd5176`](https://github.com/GitoxideLabs/gitoxide/commit/ebd517633f099582dc2633e71f7bb7890acd14d1))
    - Fixup text file tests on windows ([`2288088`](https://github.com/GitoxideLabs/gitoxide/commit/22880881d5e442acdeb8dd0cfb5ecc4f62783951))
    - Add metadata to allow docs.rs build all featueres ([`10f9386`](https://github.com/GitoxideLabs/gitoxide/commit/10f9386a12decc1f13999aee72be484c8f6d48ce))
    - Git-odb with serde support ([`0da930c`](https://github.com/GitoxideLabs/gitoxide/commit/0da930cf23f215cc1e2bda8f7340a5d69370735a))
    - Cut back on onnecessary annnotations: serde(borrow) ([`759915c`](https://github.com/GitoxideLabs/gitoxide/commit/759915c75e473f65d35ba926aaca8303e5a77f9a))
    - Serde support for all git-object types, incl. test ([`1ae8f9c`](https://github.com/GitoxideLabs/gitoxide/commit/1ae8f9c8b6b699c3f4928905358f42187bef07a7))
    - Learn from the best: with-serde -> serde1 ([`d651c21`](https://github.com/GitoxideLabs/gitoxide/commit/d651c218bfb7af5fc67ca4b9763703fb29788017))
    - Commit to using bstr whenever something is not data bytes; remove miniserde ([`3183d1b`](https://github.com/GitoxideLabs/gitoxide/commit/3183d1b02c2d7bb3c750f8472c29bb161641ca7f))
    - Prepare centralization of bstr as optional component ([`aa857d9`](https://github.com/GitoxideLabs/gitoxide/commit/aa857d9df32dfc75f151154ca430ddfee907deed))
    - Add support for miniserde ([`f806647`](https://github.com/GitoxideLabs/gitoxide/commit/f80664769e4fdbab3d1ffa56510ba87e570ae9b0))
    - First gentle test of adding serde support selectively. ([`78d9bc0`](https://github.com/GitoxideLabs/gitoxide/commit/78d9bc0f54504dc809651aeb0d24e7ac6a3f0bb3))
    - Allow for more screen space when formatting ([`6794300`](https://github.com/GitoxideLabs/gitoxide/commit/67943002e7f4215b5383bd0538786ce2857f011e))
    - Pack offset by index ([`69e35b1`](https://github.com/GitoxideLabs/gitoxide/commit/69e35b1d8f24f366d675484a1bddbebd37b72e22))
    - Test V1 lookup ([`e9c7127`](https://github.com/GitoxideLabs/gitoxide/commit/e9c71271fa51d5420fcb205d2d3deb6d012f0d41))
    - Validate sha1 of pack objects, some work, some don't for some reason… ([`aa8799a`](https://github.com/GitoxideLabs/gitoxide/commit/aa8799a01b92c3c3b7d4347f745921bbb685c454))
    - Capability to write loose object headers, fast ([`de0aeff`](https://github.com/GitoxideLabs/gitoxide/commit/de0aeff518ebd218b73bf472558f278f6bcdc17c))
    - Simplify folder names ([`36fde1f`](https://github.com/GitoxideLabs/gitoxide/commit/36fde1f90e9034060b5ede8a923365474659085e))
    - Implement object Id as newtype for more convenenience and… ([`b7a4d17`](https://github.com/GitoxideLabs/gitoxide/commit/b7a4d17d886d555b080e4182d49dc648e201ad13))
    - Rename grit to 'gitoxide', CLI name is 'gio' ([`9d6007f`](https://github.com/GitoxideLabs/gitoxide/commit/9d6007f83b3b018d736d58aa0722b83b9cffb228))
    - Fix clippy ([`a9c5da7`](https://github.com/GitoxideLabs/gitoxide/commit/a9c5da7132eeaa6806b8190985a7aa25f9ef89d8))
    - More convenient access to our four object types ([`ecda6d2`](https://github.com/GitoxideLabs/gitoxide/commit/ecda6d23561dc176f7d7ad2565da8105efac614f))
    - Even better trait derives ([`e78f9f6`](https://github.com/GitoxideLabs/gitoxide/commit/e78f9f64c8d52402197b1f946bcd32f0d83e6c7d))
    - Better trait support for basic types ([`6617386`](https://github.com/GitoxideLabs/gitoxide/commit/6617386e37b69f6e036ab27280c946e271c99540))
    - Memory size checks for objects ([`ab51616`](https://github.com/GitoxideLabs/gitoxide/commit/ab51616bb250a62b5367e861c25c1d90ec60f720))
    - Make single-field objects blob and tree more explicit ([`1aef68f`](https://github.com/GitoxideLabs/gitoxide/commit/1aef68f7e979324eb94966d44c160ffe537ee4a8))
    - Add Blob type to parsed objects ([`d3e8e4b`](https://github.com/GitoxideLabs/gitoxide/commit/d3e8e4b24ecda84665b994ccad768774efdcdc90))
    - Fix imports ([`10f2967`](https://github.com/GitoxideLabs/gitoxide/commit/10f29675442c76b38e0a8deb757930a13af3a3bb))
    - Try pub use with rename. Not bad in the docs, but maybe a bit confusing ([`526f3f8`](https://github.com/GitoxideLabs/gitoxide/commit/526f3f8d3ca9fe9672b0518f1bc3b921f695c0d8))
    - Refactor ([`2ffd7fa`](https://github.com/GitoxideLabs/gitoxide/commit/2ffd7fa6c4e5a88042b7ee1d56ec8d8515f0991f))
    - Refacto ([`ffc0089`](https://github.com/GitoxideLabs/gitoxide/commit/ffc0089fd7f4ffd3eeb0d0411b6857a28b388001))
    - Refactor ([`b9a1647`](https://github.com/GitoxideLabs/gitoxide/commit/b9a16473ed028abc59fc5126db9530f2107498d8))
    - Test for parsing trees from loose dbs ([`4f48249`](https://github.com/GitoxideLabs/gitoxide/commit/4f4824971d62d165fd4c2bea869fd88986dc259f))
    - Refactor ([`9f9ccad`](https://github.com/GitoxideLabs/gitoxide/commit/9f9ccad37fea96954a2df9e314b6c154466dc0ca))
    - Move git-object tests to top-level for separation and cleanness ([`df42a01`](https://github.com/GitoxideLabs/gitoxide/commit/df42a012bcc489b78320126e51b1f121abe7c018))
    - Prefer integration level tests, but use unit-tests where appropriate ([`ec3be19`](https://github.com/GitoxideLabs/gitoxide/commit/ec3be19c8d007565b814b4757f17811ec0e9de2c))
    - Run previously unused method of Tree ([`0d159c2`](https://github.com/GitoxideLabs/gitoxide/commit/0d159c2b76f2a8fc3c391fd990d8e7a4eeffc913))
    - Actually use the Tree object ([`635e735`](https://github.com/GitoxideLabs/gitoxide/commit/635e735419af906579de681dbc27b36fd826406d))
    - Handle commits without newlines; make tag newlines optional ([`c0b54be`](https://github.com/GitoxideLabs/gitoxide/commit/c0b54bef5a2bcfce9b6deb90cdd27c7e0cc85810))
    - Handle tags without newline; document fixture processing step ([`344a562`](https://github.com/GitoxideLabs/gitoxide/commit/344a5622953047e6f2a543bfb0355fb060a4b1e8))
    - Don't assume newlines in trees anymore ([`45d7c36`](https://github.com/GitoxideLabs/gitoxide/commit/45d7c365072a9bada3a6e0b77ced7669fe5533a3))
    - Found huge issue with newlines polluting fixtures. ([`f182d22`](https://github.com/GitoxideLabs/gitoxide/commit/f182d22caf1dd9c262cdca6a1834478556a74f31))
    - First tree implementation, which seems to work well ([`9694fcb`](https://github.com/GitoxideLabs/gitoxide/commit/9694fcbeb7bea6ebf814119ba5757110ae33bc55))
    - Boilerplate for tree parsing ([`48c4c07`](https://github.com/GitoxideLabs/gitoxide/commit/48c4c07098d807b3d62e540e06459c66fef94355))
    - Refactor ([`d48cafa`](https://github.com/GitoxideLabs/gitoxide/commit/d48cafa7edc4c6d977c396df4a26d235a3bd662c))
    - Add conversion traits for Object<->Tag|Commit ([`7dcbd5d`](https://github.com/GitoxideLabs/gitoxide/commit/7dcbd5dc764a07685a66594e3ae5514a9df83082))
    - Make Commit available in borrowed object ([`b2d1b5d`](https://github.com/GitoxideLabs/gitoxide/commit/b2d1b5d684bdfda5f922b466cc13d4ce2d635cf8))
    - Use smallvec to save memory in the common case (single parent) ([`263835b`](https://github.com/GitoxideLabs/gitoxide/commit/263835b7e14e94bfb641067e8188e23d81bc9cac))
    - More tests ([`56248fe`](https://github.com/GitoxideLabs/gitoxide/commit/56248fe9a351572478cecda8520c25ec25664bc3))
    - Now gpg-signature parsing works correctly - thanks to peek(…) ([`7078dac`](https://github.com/GitoxideLabs/gitoxide/commit/7078dac0fc27594c63cd9550c8b8b4ac7a52a627))
    - First somewhat working version of single/multi-line signature parsing ([`dab5c65`](https://github.com/GitoxideLabs/gitoxide/commit/dab5c6581dc218ee9a7f049c5499975f762d81cf))
    - Support single-line gpg signatures ([`71330b5`](https://github.com/GitoxideLabs/gitoxide/commit/71330b526614a78e20e739aa8b1cd31b5cf2ce0e))
    - Support for commit encoding field ([`40bffe9`](https://github.com/GitoxideLabs/gitoxide/commit/40bffe9b36f5afcb9b3f147d47b94b5e82acaae8))
    - More commit tests, next up: encoding ([`ca4d3aa`](https://github.com/GitoxideLabs/gitoxide/commit/ca4d3aad8f91189890b8445680406fddb6544af4))
    - First successful parsing of commit ([`b44765a`](https://github.com/GitoxideLabs/gitoxide/commit/b44765ad08f53a7062def35ecb7fe7624827df85))
    - Parse BStr versions of hex-shas directly ([`e3a2b77`](https://github.com/GitoxideLabs/gitoxide/commit/e3a2b7782fa48f474c2e1d51a6b8c2ea2c561549))
    - Parse parents ([`696e0a3`](https://github.com/GitoxideLabs/gitoxide/commit/696e0a3c48e72373cb540d16b640ddb6fc2a2dcf))
    - Use BStr instead of Id to avoid parsing into something we might not use/need ([`7c97471`](https://github.com/GitoxideLabs/gitoxide/commit/7c97471c34362c9d3d56ccada252d3058aea6697))
    - Factor out hex sha parsing ([`d650dd2`](https://github.com/GitoxideLabs/gitoxide/commit/d650dd26a168ab5a8d679dfb4b93a7f2863a20f0))
    - Refactor ([`0104f4c`](https://github.com/GitoxideLabs/gitoxide/commit/0104f4c8a8449c2549bfcfeacfeb20f14b2ddc2d))
    - First stab at factoring header parsing into sub-parser ([`6f6ee8f`](https://github.com/GitoxideLabs/gitoxide/commit/6f6ee8f721df9f3caf4db54346e7653f341552e3))
    - First fixtures for commit parsing ([`551f2d1`](https://github.com/GitoxideLabs/gitoxide/commit/551f2d1f8e32e7e64a0d19e9e7d3b3ea63e9b449))
    - Avoid unnecessary allocation when creating SHA1 paths in loose ODB ([`09d8d3a`](https://github.com/GitoxideLabs/gitoxide/commit/09d8d3a12e161a7f6afb522dbe8900a9c09bce06))
    - Document existing use of unsafe, deny everywhere else ([`41f4bce`](https://github.com/GitoxideLabs/gitoxide/commit/41f4bce9d9a492f8e20a6eb5b3eaf5adc6d78329))
    - Cleanup integer parsing in loose object database ([`ecdce1a`](https://github.com/GitoxideLabs/gitoxide/commit/ecdce1a05d8c732afd53c6da6067bf591f96fa6a))
    - Add remaining tag tests, along with some fixes ([`06e22fb`](https://github.com/GitoxideLabs/gitoxide/commit/06e22fb8aea281676e53f786ddb808dd77d3b702))
    - Use bstr were possible ([`01dd4e2`](https://github.com/GitoxideLabs/gitoxide/commit/01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc))
    - The defining property is actually that the object is borrowing data ([`e0125fd`](https://github.com/GitoxideLabs/gitoxide/commit/e0125fdb0a41ed139364084f6d679932f08b7b4f))
    - Refactor ([`683360a`](https://github.com/GitoxideLabs/gitoxide/commit/683360a6932f7d5e216dc0fdafa5890c6708d1e8))
    - Move all tests into the top-level for nicer names basically :D ([`598901a`](https://github.com/GitoxideLabs/gitoxide/commit/598901a768fec768b2519e7925ac623cb66582d6))
    - Refactor ([`0f01e9f`](https://github.com/GitoxideLabs/gitoxide/commit/0f01e9fff39fb7f1234f57c6689c0e114d9aece7))
    - Refactor ([`87bbea4`](https://github.com/GitoxideLabs/gitoxide/commit/87bbea48d247b7938e74672e1a5cb1b8b5cc6a9f))
    - Refactor; add more signature parsing tests ([`ba9c7de`](https://github.com/GitoxideLabs/gitoxide/commit/ba9c7de7ca93ac42d3c57315d743b321f8f9e3b5))
    - Cleanup; all tests work! ([`7c96603`](https://github.com/GitoxideLabs/gitoxide/commit/7c9660354484869681356a8c4ef8057313e864f2))
    - Fix whitespace ([`ebaaa00`](https://github.com/GitoxideLabs/gitoxide/commit/ebaaa00d9508141746a7c20e5d25d877f38733e9))
    - First version of tag message parsing - it's actually changed now ([`74b2328`](https://github.com/GitoxideLabs/gitoxide/commit/74b2328fcbbcffab9981c23e903c4f4c5d085aff))
    - Implement parse_signature with nom, starting to like it ([`ebdf205`](https://github.com/GitoxideLabs/gitoxide/commit/ebdf205038b66108c0331aa590388431427493b7))
    - First part of parsing tagger signatures ([`5b43270`](https://github.com/GitoxideLabs/gitoxide/commit/5b432703cf1c44bbf29e8bf89b297ea29c959be6))
    - Generalize with Boxed error cause ([`824cd2c`](https://github.com/GitoxideLabs/gitoxide/commit/824cd2cfbfaef605e953f0af193a036ef74bcac7))
    - First seemingly awkward way of not discarding too much error information… ([`6f9a636`](https://github.com/GitoxideLabs/gitoxide/commit/6f9a636da5c2f33a612395a25e8b92e557d06e83))
    - Refactor ([`fb287af`](https://github.com/GitoxideLabs/gitoxide/commit/fb287af33fcb75c01ac25dd484f529cbb49f3e6f))
    - The first sketch of parsing a tag with Nom and half-decent errors ([`4498dff`](https://github.com/GitoxideLabs/gitoxide/commit/4498dff1cf63abe53ae17b59d3658ab52235630d))
    - Use git-object in git-odb ([`07f7c31`](https://github.com/GitoxideLabs/gitoxide/commit/07f7c318d55603e3731f08cb04d3da8ac2fcfea6))
    - Move all object related code into own crate… ([`605ef20`](https://github.com/GitoxideLabs/gitoxide/commit/605ef20ec5ccf66e4f42df6d0140e4087aa13053))
</details>

## 0.26.0 (2023-01-06)

A maintenance release without user-facing changes.

## 0.25.0 (2022-12-30)

A maintenance release without user-facing changes.

## 0.24.0 (2022-12-19)

A maintenance release without user-facing changes.

## 0.23.0 (2022-11-21)

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

## 0.22.1 (2022-11-06)

### New Features

 - <csr-id-14d613303b8663daac1ab1eed8413266594050c3/> `Object::try_into_blob_ref()` as alternative to consuming `*::try_into_blob()`

## 0.22.0 (2022-10-10)

### New Features

 - <csr-id-5095df949d30b57b09bd68355743d255a897365f/> add `tree::EntryMode::is_blob()` to quickly determine if an entry is a blob.

### Bug Fixes

 - <csr-id-2df3c7395a791c751b22187a936facbce9f3f168/> don't panic if broken loose objects are encountered.
   Previously a loose object could advertise any object size and cause a
   panic if it was shorter than advertised.
   Now an error will be returned.

## 0.21.0 (2022-09-20)

<csr-id-9c8a36f35b1e8dad66befb102e5a8670b4a6c2b2/>

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

### Chore (BREAKING)

- <csr-id-9c8a36f35b1e8dad66befb102e5a8670b4a6c2b2/> remove quick-error in favor of thiserror
  Some errors change shape which makes this a breaking change.

## 0.20.3 (2022-08-28)

Maintenance release without user-facing changes.

## 0.20.2 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

## 0.20.1 (2022-08-17)

### New Features

 - <csr-id-4f4bba23a13f11050e35887da004e48148d68aea/> `tree::EntryMode::as_str()` to display itself as a string.

## 0.20.0 (2022-07-22)

This is a maintenance release with no functional changes.

### Bug Fixes

 - <csr-id-761695cd10222eccb720812fa41d5295d506fce9/> improve error messages related to object decoding

## 0.19.0 (2022-05-18)

### New Features

 - <csr-id-0d22ab459ce14bc57549270142595d8ebd98ea41/> `TagRefIter::tagger()`.
   Additionally ergonomics have been improved as the iterator is now
   `Copy`, similarly to the other iterators.

## 0.18.0 (2022-04-03)

### New Features

 - <csr-id-a39bf71531ee0a6c8db082758d3212c805ce2bf0/> support for trimming of whitespace around name and email
   It's separated from parsing to assure we can round-trip, but it's
   made easy to obtain trimmed results using new methods.
   
   This high-level git-repository will also trim by default now.
 - <csr-id-8f1b42b193098f6e116b3522a3df08d5d1b42a0a/> `CommitRef::time()`
   A shortcut to allow for fluid retrieval of the `committer.time`.
 - <csr-id-c51a9252f8a038bf995d50cdbfc8356362020bbc/> `CommitRefIter::(author|committer)()`, better usability
   The methods returning an iterator are now consuming, which allows
   them to be nested by callers.
 - <csr-id-91065cd790f559f4a661ecd9c2d2f649922682e3/> `CommitRefIter::parent_ids()`
   Allocation-free parent retrieval.
 - <csr-id-5c573bf9abfc10227d5a900100e84d7f8f6c0504/> in-manifest and in-lib feature documentation

## 0.17.1 (2022-02-01)

A automated maintenance release without impact to the public API.

### Fixes

- Corrected the tree-entry ordering implementation.

## 0.17.0 (2022-01-23)

<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-322b2901382192d3b4ac0fbae32381a9abb49fa9/>

### Chore

- <csr-id-322b2901382192d3b4ac0fbae32381a9abb49fa9/> upgrade dependencies

### New Features

 - <csr-id-eb36a3dda83a46ad59078a904f4e277f298a24e1/> Add sorting mode to ancestor traversal  #270
 - <csr-id-a0bb6529216160d14e67b97206a51140099fd245/> add `Data` object
   This is typed data baked by a slice for conversion into parsed ObjectRef's
   for example.
   
   This is usually the result of a `Find` operation on an object database.

### Changed (BREAKING)

 - <csr-id-fda2a8d2f5f8b7d80b4cc0177d08d6a061f1b8f1/> rename `commit::ref_iter::Token::into_id()` to `*::try_into_id()`
   This makes the method more consistent.
- move gix_pack::data::Object to gix_object::Data, massively alter gix_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.

## 0.16.0 (2021-11-29)

<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>

Maintenance release due, which isn't really required but one now has to be careful what's committed at once.

## 0.15.1 (2021-11-16)

A maintenance release triggered by changes to gix-pack and changelog rewrites.

## v0.15.0 (2021-10-19)

A maintenance release to properly dealing with previously breaking changes in `gix-hash`.

## v0.14.1 (2021-10-15)

<csr-id-899c57927ce4ba2e1b5d8182f9e731c2a9094cba/>

This release greatly improves performance when decoding trees, a critical requirement when building packs
or generally trying to figure out what changed between commits.

### Other

- <csr-id-899c57927ce4ba2e1b5d8182f9e731c2a9094cba/> describe variants

### Performance

<csr-id-f9232acf8e52f8cd95520d122469e136eb07b39f/>

- Provide a new fast parser for tree objects which is used by the tree entry iterator.
 - <csr-id-83d591d536d1a43e8523082824ec0b95cca55d34/> parse entry mode into number instead of comparing it to byte strings

## v0.14.0 (2021-09-08)

## v0.13.1 (2021-09-07)

## v0.13.0 (2021-08-27)

## v0.12.2 (2021-08-17)

## v0.12.1 (2021-08-13)

## v0.12.0 (2021-08-12)

## v0.11.0 (2021-08-11)

## v0.10.0 (2021-08-10)

## v0.9.0 (2021-05-09)

## v0.8.0 (2021-04-30)

## v0.7.0 (2021-04-08)

## v0.6.0 (2020-12-16)

## v0.5.0 (2020-12-15)

## v0.4.0 (2020-09-12)

## v0.3.0 (2020-08-12)

<csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/>
<csr-id-3b9d66c932075feb08cdf2967f7698daef9fd3ff/>

### Refactor

- <csr-id-5d57c1f7e3b9a84f7b46a4378015572155f3104b/> Use borrowed::Id in trees for full type safety
- <csr-id-3b9d66c932075feb08cdf2967f7698daef9fd3ff/> make reusing round-trip code easier

## v0.1.0 (2020-07-12)

<csr-id-ec3be19c8d007565b814b4757f17811ec0e9de2c/>

### Refactor

- <csr-id-ec3be19c8d007565b814b4757f17811ec0e9de2c/> Prefer integration level tests, but use unit-tests where appropriate

