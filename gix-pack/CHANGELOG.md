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

## 0.64.1 (2026-01-06)

### Other

 - <csr-id-d99352bd3b7c75791826b40df20afdd4b0d8b26e/> Don't decompress the delta when decompressing base objects.
   It was already decompressed on line 285, so attempting to decompress it
   again turns the delta stream into garbage.
   
   Unfortunately I don't know enough about the git file format to make a
   test, the best I was able to do was to compare it to a reference
   implementation and see what was going wrong.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 5 calendar days.
 - 5 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2345 from matts1/push-utkmrmunzotv ([`c0cf38f`](https://github.com/GitoxideLabs/gitoxide/commit/c0cf38f98ac557c7ea89e505d2eb55f3b3c218bf))
    - Don't decompress the delta when decompressing base objects. ([`d99352b`](https://github.com/GitoxideLabs/gitoxide/commit/d99352bd3b7c75791826b40df20afdd4b0d8b26e))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.64.0 (2025-12-31)

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

## 0.63.0 (2025-12-22)

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

## 0.62.0 (2025-11-22)

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

## 0.61.1 (2025-10-23)

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

## 0.61.0 (2025-10-22)

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
    - Merge pull request #2155 from folkertdev/skip-flate2 ([`752d6dc`](https://github.com/GitoxideLabs/gitoxide/commit/752d6dc830f98980c96e9a1dc6ec8bf4432fb7f1))
    - Refactor ([`0e7aa81`](https://github.com/GitoxideLabs/gitoxide/commit/0e7aa815dbe10e9d19334a3f0b881c6d608a3b8e))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
    - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.60.0 (2025-07-15)

<csr-id-fce70950006892f51b32af233656be6fe5de9df3/>

### Other

 - <csr-id-fce70950006892f51b32af233656be6fe5de9df3/> delta application is a fallible operation

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 79 calendar days.
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
    - Merge pull request #2059 from bradlarsen/main ([`52906fb`](https://github.com/GitoxideLabs/gitoxide/commit/52906fb848e8ba088dc2662c4cd44a0a74148ceb))
    - Refactor ([`56ca4bf`](https://github.com/GitoxideLabs/gitoxide/commit/56ca4bf8e818bbdc14ee9dda68e39acded8f0287))
    - Delta application is a fallible operation ([`fce7095`](https://github.com/GitoxideLabs/gitoxide/commit/fce70950006892f51b32af233656be6fe5de9df3))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.59.1 (2025-04-26)

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

## 0.59.0 (2025-04-25)

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
    - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
    - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
    - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
    - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.58.0 (2025-04-04)

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
 - <csr-id-6c02b0a044edc123d95525ed8e8a6e2fb1e45213/> split index and pack checksum verification errors

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Split index and pack checksum verification errors ([`6c02b0a`](https://github.com/GitoxideLabs/gitoxide/commit/6c02b0a044edc123d95525ed8e8a6e2fb1e45213))
    - Migrate all hashing API users to `gix_hash::Hasher::finalize()` ([`4e935ce`](https://github.com/GitoxideLabs/gitoxide/commit/4e935ce167428581f7e0351768b705164f71179a))
    - Migrate all hashing API users to `gix_hash` ([`baa1430`](https://github.com/GitoxideLabs/gitoxide/commit/baa1430aed0dc8160a71cc675e2626780a2de052))
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Drop trailing `,` just before `)` on same line in function calls ([`66a5ae1`](https://github.com/GitoxideLabs/gitoxide/commit/66a5ae1b586d583066402c801213a55141e2aad6))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.57.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1752 from GitoxideLabs/git-shell ([`1ca480a`](https://github.com/GitoxideLabs/gitoxide/commit/1ca480aa4093328a7e047e770fdffdb8cc6d8e8d))
    - Thanks clippy ([`9193b05`](https://github.com/GitoxideLabs/gitoxide/commit/9193b05b2528f62d829447ccc50314bd4cffc415))
    - Merge pull request #1410 from GitoxideLabs/status ([`0ab4f64`](https://github.com/GitoxideLabs/gitoxide/commit/0ab4f64407b7fa0924830f7b7bd2f5b0ba1cc16e))
    - Adapt to changes in `gix-traverse` ([`1de4e70`](https://github.com/GitoxideLabs/gitoxide/commit/1de4e70569cd7c3bfcc9094b7591699b5b419608))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.56.0 (2024-12-22)

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

## 0.55.0 (2024-11-24)

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
    - Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0 ([`4000197`](https://github.com/GitoxideLabs/gitoxide/commit/4000197ecc8cf1a5d79361620e4c114f86476703))
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1700 from GitoxideLabs/reduce-memory ([`54ea266`](https://github.com/GitoxideLabs/gitoxide/commit/54ea266a5b57d3081c2ba6ed60dc0612059617ca))
    - Try to reduce memory consumption by avoiding over-capacity of possibly large vectors ([`664e28c`](https://github.com/GitoxideLabs/gitoxide/commit/664e28caa4304fe489d2b37a1a3328763960517f))
    - Merge pull request #1687 from EliahKagan/run-ci/32bit ([`aeaebec`](https://github.com/GitoxideLabs/gitoxide/commit/aeaebec7b1e07ce94429987c4f2466799c24cb67))
    - Deduplicate some repeated tests ([`dc0a73a`](https://github.com/GitoxideLabs/gitoxide/commit/dc0a73ab0ff3e4e3731cf7cc01b418a56bb5953c))
    - Use `<=` on 32-bit for some size assertions ([`fc13fc3`](https://github.com/GitoxideLabs/gitoxide/commit/fc13fc3eb950ebaff0c22c4d093a4d2300914f72))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1654 from EliahKagan/doctest-workspace ([`1411289`](https://github.com/GitoxideLabs/gitoxide/commit/141128942c26bd63fc6855e5137b98f8da814446))
    - Mark gix-pack vendored `izip!` doctest `ignore` ([`0783e9c`](https://github.com/GitoxideLabs/gitoxide/commit/0783e9c1648261ad4a9af1904d65abedf24821d9))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.54.0 (2024-10-22)

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

 - 22 commits contributed to the release.
 - 60 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0 ([`f1364dc`](https://github.com/GitoxideLabs/gitoxide/commit/f1364dcb8aa66e3d8730e38445b045c5b63c56e6))
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1637 from GitoxideLabs/improve-error-message ([`b36d7ef`](https://github.com/GitoxideLabs/gitoxide/commit/b36d7efb9743766338ac7bb7fb2399a06fae5e60))
    - Thanks clippy ([`2d1fbce`](https://github.com/GitoxideLabs/gitoxide/commit/2d1fbce2877e0d4c64557f7feb5327d695d114a3))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Adapt to changes in `gix-diff` ([`3fd9fab`](https://github.com/GitoxideLabs/gitoxide/commit/3fd9fabe0c0e74d54b5153c4f572eff76293f334))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Adapt to chagnes in `gix-diff` ([`5c1f010`](https://github.com/GitoxideLabs/gitoxide/commit/5c1f010851e40bf5b284ec82e6cd27cd41ab70bf))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - Adapt to changes in `gix-features` ([`ee80b2b`](https://github.com/GitoxideLabs/gitoxide/commit/ee80b2bd8c3be6f80f85987f8df32fa4a796247f))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
</details>

## 0.53.0 (2024-08-22)

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

## 0.52.0 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
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
</details>

## 0.51.1 (2024-07-23)

<csr-id-a2da5373fada6c1a90d139bba6db7b238ae6504c/>

### Bug Fixes

 - <csr-id-36d96f124bae7282da69883dc4c8e145261fb0f9/> assure duplicate packs and indices aren't overwriting existing ones.
   That way, on Windows there is no chance for access-denied errors.
   Maybe there are other advantages as well even for Unix.
   
   Since packs are usually written rarely, in comparison to loose objects,
   the extra file accesss seems acceptable.
 - <csr-id-36d1b69ae6d8765936ebb0dc590a14704dc41e64/> do not write empty packs and indices
   Failure can occour on Windows as it's likely such a pack or index is already opened
   during negotiation. Then, when an empty fetch via `--depth 1` is repeated,
   a temporary file would be renamed onto one an mmapped pack or index, causing
   a failure.
   
   Now packs or indices aren't written anymore if they are empty.

### Other

 - <csr-id-a2da5373fada6c1a90d139bba6db7b238ae6504c/> gate few deps

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release.
 - 62 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1404](https://github.com/GitoxideLabs/gitoxide/issues/1404)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1404](https://github.com/GitoxideLabs/gitoxide/issues/1404)**
    - Assure duplicate packs and indices aren't overwriting existing ones. ([`36d96f1`](https://github.com/GitoxideLabs/gitoxide/commit/36d96f124bae7282da69883dc4c8e145261fb0f9))
    - Do not write empty packs and indices ([`36d1b69`](https://github.com/GitoxideLabs/gitoxide/commit/36d1b69ae6d8765936ebb0dc590a14704dc41e64))
 * **Uncategorized**
    - Release gix-actor v0.31.5, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`6232824`](https://github.com/GitoxideLabs/gitoxide/commit/6232824301847a9786dea0b926796a3187493587))
    - Release gix-glob v0.16.4, gix-attributes v0.22.3, gix-command v0.3.8, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`a1b73a6`](https://github.com/GitoxideLabs/gitoxide/commit/a1b73a67c19d9102a2c5a7f574a7a53a86d0094c))
    - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
    - Prepare changelog prior to release ([`99c00cc`](https://github.com/GitoxideLabs/gitoxide/commit/99c00cc3ae9827555e2e1162328bc57038619d1f))
    - Release gix-path v0.10.9 ([`15f1cf7`](https://github.com/GitoxideLabs/gitoxide/commit/15f1cf76764221d14afa66d03a6528b19b9c30c9))
    - Release gix-actor v0.31.4, gix-object v0.42.3 ([`bf3d82a`](https://github.com/GitoxideLabs/gitoxide/commit/bf3d82abc7c875109f9a5d6b6713ce68153b6456))
    - Merge pull request #1430 from klensy/deps ([`ab02aa9`](https://github.com/GitoxideLabs/gitoxide/commit/ab02aa99842c17d68b8ee37e05e2f35720291e42))
    - Gate few deps ([`a2da537`](https://github.com/GitoxideLabs/gitoxide/commit/a2da5373fada6c1a90d139bba6db7b238ae6504c))
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'fix-1404' ([`9923542`](https://github.com/GitoxideLabs/gitoxide/commit/99235426058b208c59fe9d67051dc4fe26b45af6))
    - Merge branch 'feat/checkout-other-refs' ([`ecfde07`](https://github.com/GitoxideLabs/gitoxide/commit/ecfde07d0887322db34f5ea531891c92676e1ff4))
    - Thanks clippy ([`f36b9bd`](https://github.com/GitoxideLabs/gitoxide/commit/f36b9bd28052131401d048b5aa55c5ae1f9248db))
    - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
    - Thanks clippy ([`acc1331`](https://github.com/GitoxideLabs/gitoxide/commit/acc13318731fabac8f65d604baf7e47814f92ad4))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Regenerate archives for changed scripts ([`ea12fc2`](https://github.com/GitoxideLabs/gitoxide/commit/ea12fc234e898eb15013da40d2a82f69c2d20482))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
</details>

## 0.51.0 (2024-05-22)

### Bug Fixes (BREAKING)

 - <csr-id-b32a847e10b742834169fac56c284645c1ed28f3/> don't panic when unknown entry types are encountered.
   Related to https://github.com/helix-editor/helix/issues/10660
   which runs into object types that are unknown.
   
   I have looked into this and [couldn't find evidence of a new pack-entry type](https://github.com/git/git/blob/0f3415f1f8478b05e64db11eb8aaa2915e48fef6/packfile.c#L1303-L1358)
   in the Git codebase.
   
   It also looks like that Git [will never write packs that aren't V2](https://github.com/git/git/blob/0f3415f1f8478b05e64db11eb8aaa2915e48fef6/pack-write.c#L352)
   - initially I thought it might write V3 based on some other criteria.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 9 calendar days.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
    - Adapt to changes in `gix-pack` ([`bad5b48`](https://github.com/GitoxideLabs/gitoxide/commit/bad5b48e4f0d865b0b0937f136d9a0041aa88046))
    - Don't panic when unknown entry types are encountered. ([`b32a847`](https://github.com/GitoxideLabs/gitoxide/commit/b32a847e10b742834169fac56c284645c1ed28f3))
</details>

## 0.50.0 (2024-04-13)

### Bug Fixes (BREAKING)

 - <csr-id-2a9c178326b7f13ba6bc1f89fc2b9d9facbecf48/> Make `topo` more similar to `Ancestors`, but also rename `Ancestors` to `Simple`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 5 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
    - Merge branch 'add-topo-walk' ([`b590a9d`](https://github.com/GitoxideLabs/gitoxide/commit/b590a9d2b6a273f76f0320d2b9fe1f679c08f549))
    - Adapt to changes in `gix-traverse` ([`1cfeb11`](https://github.com/GitoxideLabs/gitoxide/commit/1cfeb11f1fe9ad9c7b9084840ed7f5c5877f2f9a))
    - Make `topo` more similar to `Ancestors`, but also rename `Ancestors` to `Simple` ([`2a9c178`](https://github.com/GitoxideLabs/gitoxide/commit/2a9c178326b7f13ba6bc1f89fc2b9d9facbecf48))
    - Adapt to changes in `gix-traverse` ([`6154bf3`](https://github.com/GitoxideLabs/gitoxide/commit/6154bf3a346d69f9749271d50e4f3aacdcbad4d0))
    - Thanks clippy ([`7f6bee5`](https://github.com/GitoxideLabs/gitoxide/commit/7f6bee5452ee01638f89a0cec2d4ee2a6f0d0136))
</details>

## 0.49.0 (2024-03-14)

<csr-id-39879af6eaf2bf4fe159a5c6371c98d516c4febe/>

### Chore

 - <csr-id-39879af6eaf2bf4fe159a5c6371c98d516c4febe/> remove repetitive words

### Bug Fixes

 - <csr-id-88061a176b2f4b5a377a4cff513979ddb1e306a1/> assure memory maps are created with `MAP_PRIVATE`
   That way, the mmap process should work under more circumstances.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 5 calendar days.
 - 18 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1312](https://github.com/GitoxideLabs/gitoxide/issues/1312)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1312](https://github.com/GitoxideLabs/gitoxide/issues/1312)**
    - Assure memory maps are created with `MAP_PRIVATE` ([`88061a1`](https://github.com/GitoxideLabs/gitoxide/commit/88061a176b2f4b5a377a4cff513979ddb1e306a1))
 * **Uncategorized**
    - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
    - Prepare changelogs prior to release ([`52c3bbd`](https://github.com/GitoxideLabs/gitoxide/commit/52c3bbd36b9e94a0f3a78b4ada84d0c08eba27f6))
    - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
    - Merge pull request #1314 from avoidalone/main ([`5722e3a`](https://github.com/GitoxideLabs/gitoxide/commit/5722e3aeeba5dd44e38a6cdbb70717a45345307e))
    - Remove repetitive words ([`39879af`](https://github.com/GitoxideLabs/gitoxide/commit/39879af6eaf2bf4fe159a5c6371c98d516c4febe))
    - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
    - Merge branch 'mmap-mode' ([`9e9b9fe`](https://github.com/GitoxideLabs/gitoxide/commit/9e9b9fe6f63759d3bf479a8ec2f9dd1fbef87a08))
</details>

## 0.48.0 (2024-02-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 30 calendar days.
 - 36 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Thanks clippy ([`13d5602`](https://github.com/GitoxideLabs/gitoxide/commit/13d5602faa58aa6f520ebc6003ed54bc9c844f2b))
    - Merge branch 'tempfile-permissions' ([`7b44c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7b44c7ff1dc0b8875214d2673c7f52948cf04ff0))
    - Release gix-tempfile v13.1.0, gix-lock v13.1.0, safety bump 12 crates ([`8430442`](https://github.com/GitoxideLabs/gitoxide/commit/84304427dfe4d170c7732161b126961719f70059))
    - Merge branch 'entryoom' ([`684fa5c`](https://github.com/GitoxideLabs/gitoxide/commit/684fa5caf82fc38dd238361d6482e77901ca0265))
    - Refactor ([`2a9ef4e`](https://github.com/GitoxideLabs/gitoxide/commit/2a9ef4e3299a28df8f9648e62eaec1ecb0011b99))
    - Merge branch 'chunks_exact' ([`d4d478b`](https://github.com/GitoxideLabs/gitoxide/commit/d4d478ba6a70fce34717e2164f8b3aa7a1521a2d))
    - Use chunks_exact where possible ([`2482023`](https://github.com/GitoxideLabs/gitoxide/commit/24820232f07ae55c80d95470228cc9e874830487))
    - Handle OOM when copying to buffers ([`0be338f`](https://github.com/GitoxideLabs/gitoxide/commit/0be338fdf80a11877599545cfa1b215092ce9afb))
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
</details>

## 0.47.0 (2024-01-20)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 16 calendar days.
 - 20 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1231](https://github.com/GitoxideLabs/gitoxide/issues/1231)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1231](https://github.com/GitoxideLabs/gitoxide/issues/1231)**
    - Improve usage of `unsafe` ([`4017e69`](https://github.com/GitoxideLabs/gitoxide/commit/4017e6934f7a68456c62bca78ecabc9c9151e843))
 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
    - Merge branch 'safety-invariants' ([`2a663a0`](https://github.com/GitoxideLabs/gitoxide/commit/2a663a0ee408935004b2cc9fe282ce87c0637905))
    - Put Tree in its own module ([`c85a981`](https://github.com/GitoxideLabs/gitoxide/commit/c85a981f39e2589821e5e502103a24265e11204a))
    - Properly track safety invariants ([`6587eea`](https://github.com/GitoxideLabs/gitoxide/commit/6587eea0043423320d64c806ed226dba1e1c32f0))
    - Demote `ItemSliceMut::get_mut()` bound-check to debug-code only. ([`911c05f`](https://github.com/GitoxideLabs/gitoxide/commit/911c05f7c714511a5eb4228ba41096ff1047cf71))
    - Make ItemSliceSync::get_mut() check that the index is in range ([`ef21617`](https://github.com/GitoxideLabs/gitoxide/commit/ef21617941d9faa6679269fb9bafd6ac097fefb9))
</details>

## 0.46.1 (2023-12-30)

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

## 0.46.0 (2023-12-29)

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

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Adapt to changes in `gix-hash` ([`859a092`](https://github.com/GitoxideLabs/gitoxide/commit/859a092bb6ad04ee9bab424cf5b56f299a9645d3))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Check all git-lfs managed files into the repository ([`35439de`](https://github.com/GitoxideLabs/gitoxide/commit/35439defd2d71779d4b3795b7652cde18ff11150))
    - Release gix-hash v0.13.3, gix-index v0.27.1 ([`98b08f4`](https://github.com/GitoxideLabs/gitoxide/commit/98b08f4d0d9237be0e0c2caa9bf5c13ae8bbf9d8))
</details>

## 0.45.0 (2023-12-06)

### Changed (BREAKING)

 - <csr-id-d6e720146c633bdc5400425b6097a3f5de93a51b/> Use `gix-object::Find` trait

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 29 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`d3fd11e`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd11ec3783843d4e49081e1d14359ed9714b5f))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge pull request #1137 from martinvonz/push-mruzqpwzzwmy ([`115c993`](https://github.com/GitoxideLabs/gitoxide/commit/115c993f7919b4f7edcf04e92bb8293957c028a5))
    - Give `Node` an unsafe constructor ([`c4807de`](https://github.com/GitoxideLabs/gitoxide/commit/c4807deffa170062c97e8452f4e20f73fb3a8aff))
    - Remove `Clone` impl of `ItemSliceSend` and make it `Sync` instead ([`43bca11`](https://github.com/GitoxideLabs/gitoxide/commit/43bca114ae4de2e84fdcd3c1bdcb736417d0f4e6))
    - Make delta `Node` private to `resolve.rs` ([`7301ca1`](https://github.com/GitoxideLabs/gitoxide/commit/7301ca13f89f2c0aafb145aedee4b32481920620))
    - Move delta `Node` into `resolve.rs` ([`d655ee1`](https://github.com/GitoxideLabs/gitoxide/commit/d655ee13779c40c28e518056f539e38d1b52ac8c))
    - Extract a `ItemSliceSend::get_mut()` from `Node::into_child_iter()` ([`d27857d`](https://github.com/GitoxideLabs/gitoxide/commit/d27857d7a6fb5d4b858e0398f3f1f2b5656e70d0))
    - Rename an `Item`-typed variable from `node` to `item` ([`d6b8977`](https://github.com/GitoxideLabs/gitoxide/commit/d6b8977ce1662b75d9b0a82978b387dc7faf3979))
    - Merge pull request #1131 from alexanderkjall/bump-memmap2-to-0.9 ([`f528ae8`](https://github.com/GitoxideLabs/gitoxide/commit/f528ae864b9f5f2a88325d89e2596f3b66b706d4))
    - Upgrade memmap2 from 0.7.1 to 0.9.0 ([`f4c3380`](https://github.com/GitoxideLabs/gitoxide/commit/f4c338077688d7026feaaf3b95c03a8409c071ba))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'sh-on-windows' ([`2b80d84`](https://github.com/GitoxideLabs/gitoxide/commit/2b80d8424196088d4ccc36914c87e320e4416ea1))
    - Remove special handling in favor of allowing shell-avoidance. ([`a0cc80d`](https://github.com/GitoxideLabs/gitoxide/commit/a0cc80d21e74a43d5770cf08a221ef92f39920bb))
    - Merge pull request #1116 from martinvonz/push-rlxsnxnlnkmp ([`613f018`](https://github.com/GitoxideLabs/gitoxide/commit/613f0184cac2b74a3fac80e4cefbc2503d4580cf))
    - Make ItemSliceSend and Node only crate-visible ([`4f5ab36`](https://github.com/GitoxideLabs/gitoxide/commit/4f5ab3650250159bc3b5521e79cdc390aeb3f4ae))
    - Add a phantom lifetime to ItemSliceSend ([`6e5ef3b`](https://github.com/GitoxideLabs/gitoxide/commit/6e5ef3b2e60be4eb38544362e08b6a787788bfdf))
    - Fix pointer aliasing in ItemSliceSend ([`17a7c6a`](https://github.com/GitoxideLabs/gitoxide/commit/17a7c6aa71e007e5110ee0680b871c5be79554bc))
    - Give ItemSliceSend a constructor and hide its field ([`c684b26`](https://github.com/GitoxideLabs/gitoxide/commit/c684b269f6ae71f8b469fe127b83a180e879a7f1))
    - Merge branch 'fix-1096' ([`ff99a18`](https://github.com/GitoxideLabs/gitoxide/commit/ff99a18e9f9388542a9cbf17d61b413f34b1d533))
    - Adapt to changes in `gix-object` ([`203d69c`](https://github.com/GitoxideLabs/gitoxide/commit/203d69c8890acc716bd4f7a7b1b2b91a8c828bde))
    - Merge branch 'gix-object-find' ([`c8bd660`](https://github.com/GitoxideLabs/gitoxide/commit/c8bd66065316176dfbbfe7ecaa092a25cad1854b))
    - Thanks clippy ([`82b01c2`](https://github.com/GitoxideLabs/gitoxide/commit/82b01c28bbbcd3b8ce346d1977fe7d8587273be6))
    - Use `gix-object::Find` trait ([`d6e7201`](https://github.com/GitoxideLabs/gitoxide/commit/d6e720146c633bdc5400425b6097a3f5de93a51b))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## 0.44.0 (2023-10-12)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 13 calendar days.
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
    - Thanks clippy ([`345712d`](https://github.com/GitoxideLabs/gitoxide/commit/345712dcdfddcccc630bbfef2ed4f461b21550d3))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Adapt to `gix-features` ([`9e7c3e1`](https://github.com/GitoxideLabs/gitoxide/commit/9e7c3e12528833023bf37ada4e2e90989d848e99))
</details>

## 0.43.0 (2023-09-24)

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

## 0.42.0 (2023-09-08)

<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

### New Features

 - <csr-id-0357b6cdb11b098fd54cd0c3df6c617d0b44a1c2/> Add `generate` and `streaming-input` feature toggles.
   That way, it's possible to not compile a bunch of code in `gix` if
   the writing of packs isn't required.

### Bug Fixes

 - <csr-id-96a07e08e6090222cf398b46aa8d46b56f81f14d/> Use `Vec::resize()` instead of set_len()
   Otherwise it's possible for uninitialized memory to be used as if it was initialized,
   which can lead to strange behaviour.
   
   As the buffer is re-used, it's not actually zeroing that much memory either.

### Chore (BREAKING)

- <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
  It makes proper usage of `Progress` types easier and allows them to be used
  as `dyn` traits as well.

### New Features (BREAKING)

 - <csr-id-24dd870919ba444aa8099c63a78ea120d47ec28e/> use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress`
 - <csr-id-4b74996b19176cb0f00860b3db5a27819d63e7d0/> Make usage of decompression context explicit.
   That way, the context can be reused which is more efficient than recreating
   it from scratch for every little delta to decompress.
   
   This leads to a performance gain of 1.3%.

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch 'optimizations' ([`6135a5e`](https://github.com/GitoxideLabs/gitoxide/commit/6135a5ea8709646f01da62939a59dd3a9750e007))
    - Add `generate` and `streaming-input` feature toggles. ([`0357b6c`](https://github.com/GitoxideLabs/gitoxide/commit/0357b6cdb11b098fd54cd0c3df6c617d0b44a1c2))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress` ([`24dd870`](https://github.com/GitoxideLabs/gitoxide/commit/24dd870919ba444aa8099c63a78ea120d47ec28e))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Switch the last crate (gix-package-tests) to edition 2021 ([`04494c6`](https://github.com/GitoxideLabs/gitoxide/commit/04494c65df05eef6b24acb68faae939a3d85f510))
    - Merge branch 'perf-and-safety' ([`9ad9c5b`](https://github.com/GitoxideLabs/gitoxide/commit/9ad9c5b1cfa3afff5273558b6ef98ca4714d4272))
    - Use `Vec::resize()` instead of set_len() ([`96a07e0`](https://github.com/GitoxideLabs/gitoxide/commit/96a07e08e6090222cf398b46aa8d46b56f81f14d))
    - Make usage of decompression context explicit. ([`4b74996`](https://github.com/GitoxideLabs/gitoxide/commit/4b74996b19176cb0f00860b3db5a27819d63e7d0))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.41.0 (2023-08-22)

<csr-id-93feea269eebd114e866e6f29f4a73c0096df9e0/>

### Chore

- <csr-id-93feea269eebd114e866e6f29f4a73c0096df9e0/> split tests off into their own crate to allow feature toggles.
  That way we can test with the `parallel` feature and won't have to
  create bogus feature toggles that are only used for testing, yet visbible
  to users.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 15 calendar days.
 - 29 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0 ([`6c62e74`](https://github.com/GitoxideLabs/gitoxide/commit/6c62e748240ac0980fc23fdf30f8477dea8b9bc3))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
    - More cleanup of test crates ([`73c685a`](https://github.com/GitoxideLabs/gitoxide/commit/73c685a67debcfa26a940f37bbca69cb3a4af57e))
    - Split tests off into their own crate to allow feature toggles. ([`93feea2`](https://github.com/GitoxideLabs/gitoxide/commit/93feea269eebd114e866e6f29f4a73c0096df9e0))
    - Release gix-glob v0.10.2, gix-date v0.7.2, gix-validate v0.8.0, gix-object v0.34.0, gix-ref v0.34.0, gix-config v0.27.0, gix-commitgraph v0.18.2, gix-revwalk v0.5.0, gix-revision v0.19.0, gix-refspec v0.15.0, gix-submodule v0.1.0, safety bump 18 crates ([`4604f83`](https://github.com/GitoxideLabs/gitoxide/commit/4604f83ef238dc07c85aaeae097399b67f3cfd0c))
</details>

## 0.40.2 (2023-07-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 day passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-archive v0.2.1, gix-ref v0.33.2, gix-pack v0.40.2, gix v0.50.1 ([`13883e5`](https://github.com/GitoxideLabs/gitoxide/commit/13883e5528385f892ee402e911298121e0c297c0))
    - Prepare changelogs ([`735c206`](https://github.com/GitoxideLabs/gitoxide/commit/735c2062625aaeffbdbca3c1395dbcf075661e3a))
    - Does this fix CI? ([`a4e6e28`](https://github.com/GitoxideLabs/gitoxide/commit/a4e6e28cf18766c625bfb03ed2eb607694b1371a))
</details>

## 0.40.1 (2023-07-22)

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

## 0.40.0 (2023-07-19)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 10 calendar days.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`4aca8c2`](https://github.com/GitoxideLabs/gitoxide/commit/4aca8c2ae2ec588fb65ec4faa0c07c19d219569f))
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
    - Merge branch 'refactor-pack-streaming' ([`8a46a7e`](https://github.com/GitoxideLabs/gitoxide/commit/8a46a7ef3efc22241b67f3447223b2505e205442))
    - Simplify decompressor and hash handling when streaming packs ([`33f95ba`](https://github.com/GitoxideLabs/gitoxide/commit/33f95ba45885b10c8ca73d8180caa1b303dc091c))
    - Thanks clippy ([`3ef32af`](https://github.com/GitoxideLabs/gitoxide/commit/3ef32af9bf477cbc60d24da8bb3f15d20976e9e0))
    - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
    - Assure we run all tests in nextest ([`3821089`](https://github.com/GitoxideLabs/gitoxide/commit/3821089b6b02c933770705b19fc3126d61beb5a7))
</details>

## 0.39.1 (2023-06-29)

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

## 0.39.0 (2023-06-29)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 6 calendar days.
 - 6 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
    - Prepare changelogs prior to release ([`00f96fb`](https://github.com/GitoxideLabs/gitoxide/commit/00f96fb3110a8f81a1bd0d74c757c15b8773c6f6))
    - Upgrade memmap2 and fastrand dependencies ([`6fc7497`](https://github.com/GitoxideLabs/gitoxide/commit/6fc74971ac6838cbfd9c869ba3746713001d7a38))
</details>

## 0.38.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### New Features

 - <csr-id-3cffa268460eb2d41bd6a30d45778b88db4ec602/> provide basic `tracing` spans for common operations.
   This is just the beginning and more crates will integrate with it over time.

### Bug Fixes

 - <csr-id-b9eb40730b53f788d2e4bffe4ef6d9028440782e/> revert 3a2d5286084597d4c68549903709cda77dda4357 to fix 'incorrect data check' error.
   This error could occour in heavily threaded code for unknown reason.
   But maybe it's due to threads somehow not cleaning up their reused decompressor properly
   (maybe related to the zlib-ng version). It's strange and sad as this really costs performnace
   for no good reason.
 - <csr-id-8817c248dd7c6453ced654d4df304f98ff18ecda/> don't crash when object validation failed during verification.
   When objects can't be serialized, they will trigger an error that manifests as IO error.
   Previously we didn't think of the possibility that writing to an im-memory buffer could fail
   would indeed panic during verification.
   
   This is now fixed.
 - <csr-id-cd747f9292fb034e7fd5ee6c5a94a4e53df42e86/> ScopedJoinHandle is not always available

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 10 calendar days.
 - 12 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - Try tree-traversal without thread_local! ([`5a9a7a3`](https://github.com/GitoxideLabs/gitoxide/commit/5a9a7a31c1537aa54cd81a5145ca319a040b9951))
    - Revert 3a2d5286084597d4c68549903709cda77dda4357 to fix 'incorrect data check' error. ([`b9eb407`](https://github.com/GitoxideLabs/gitoxide/commit/b9eb40730b53f788d2e4bffe4ef6d9028440782e))
    - Merge branch 'gix-corpus' ([`5861afb`](https://github.com/GitoxideLabs/gitoxide/commit/5861afb45f32c16eefcd8e7b7480309bf44b6edc))
    - Don't crash when object validation failed during verification. ([`8817c24`](https://github.com/GitoxideLabs/gitoxide/commit/8817c248dd7c6453ced654d4df304f98ff18ecda))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Provide basic `tracing` spans for common operations. ([`3cffa26`](https://github.com/GitoxideLabs/gitoxide/commit/3cffa268460eb2d41bd6a30d45778b88db4ec602))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - ScopedJoinHandle is not always available ([`cd747f9`](https://github.com/GitoxideLabs/gitoxide/commit/cd747f9292fb034e7fd5ee6c5a94a4e53df42e86))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
</details>

## 0.37.0 (2023-06-10)

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
    - Release gix-attributes v0.13.1, gix-diff v0.30.1, gix-revwalk v0.1.0, gix-traverse v0.27.0, gix-index v0.18.0, gix-revision v0.15.2, gix-negotiate v0.2.1, gix-pack v0.37.0, gix-odb v0.47.0, gix-protocol v0.33.2, gix-worktree v0.19.0, gix v0.46.0, safety bump 7 crates ([`2560a2c`](https://github.com/GitoxideLabs/gitoxide/commit/2560a2cc3e1d8c60cd812e15696fa4761d036e19))
    - Prepare changelogs prior to release ([`298f3d7`](https://github.com/GitoxideLabs/gitoxide/commit/298f3d7359c5b183314d8c584e45dcdd559d88b3))
    - Merge branch 'walk-with-commitgraph' ([`fdee9a2`](https://github.com/GitoxideLabs/gitoxide/commit/fdee9a22873a13ae644d3dc92f8fe93f8f0266c0))
    - Adapt to changes in `gix-traverse` ([`1f682fd`](https://github.com/GitoxideLabs/gitoxide/commit/1f682fd991b9b76a8d37e6852567ff239c0ac0db))
</details>

## 0.36.0 (2023-06-06)

### New Features

 - <csr-id-3a2d5286084597d4c68549903709cda77dda4357/> improve performance by reducing the trashing of `zlib::Inflate`.
   Previously, for every zlib inflate operation, we would allocate memory
   due to the creation of a decompressor.
   
   This is now avoided by reusing thread-local copies of such a decompressor,
   which is reused and reset instead.
 - <csr-id-14e7ea0217af8a04ed2b50ff7b13c28335c29022/> improve performance by avoiding zeroying buffers.
   Previously we would use `resize(new_len, 0)` to resize buffers, even though
   these values would then be overwritten (or the buffer isn't available).
   
   Now we use `set_len(new_len)` after calling `reserve` to do the same, but safe
   a memset.

### Bug Fixes

 - <csr-id-977e135bfa00bdbc1a8f8324f85347ec9078c84f/> static linked list delta cache with memory cap
   Previously, the 64 slot big LRU cache for pack deltas didn't
   use any memory limit which could lead to memory exhaustion in the
   face of untypical, large objects.
   
   Now we add a generous default limit to do *better* in such situations.
   It's worth noting though that that even without any cache, the working
   set of buffers to do delta resolution takes considerable memory, despite
   trying to keep it minimal.
   
   Note that for bigger objects, the cache is now not used at all, which probably
   leads to terrible performance as not even the base object can be cached.
 - <csr-id-f89cbc675b0acc67322e289e7b3a17288b9eae27/> check for interrupt more often
   Previously when traversing a pack it could appear to hang as checks
   were only performed on chunk or base (of a delta-tree) level.
   
   Now interrupt checks are performed more often to stop all work much quicker.
 - <csr-id-923692bcda698b45d3d1ad518b29f6d30b23fbc0/> memory capped hashmap as pack delta cache won't trash memory as much.
   Previously it would take a buffer from the free-list, copy data into it, and
   when exceeding the capacity loose it entirely.
   Now the freelist is handled correctly.

### New Features (BREAKING)

 - <csr-id-d22dd8fcc22e8dbe30524a1bdddc09bc841db341/> index-backed tree traversal with a form of work-stealing.
   When delta-trees are unbalanced, in pathological cases it's possible that that one thread
   ends up with more than half of the work. In this case it's required that it manages to
   spawn its own threads to parallelize the work it has.
 - <csr-id-0fa04bcbdf3102c5435e64cfef894a1bfc8d6e7b/> make current thread-count accessible in slice-workers.
   Threads started for working on an entry in a slice can now see the amount
   of threads left for use (and manipulate that variable) which effectively
   allows them to implement their own parallelization on top of the current one.
   
   This is useful if there is there is very imbalanced work within the slice itself.
   
   While at it, we also make consumer functions mutable as they exsit per thread.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 29 commits contributed to the release over the course of 25 calendar days.
 - 40 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#851](https://github.com/GitoxideLabs/gitoxide/issues/851)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#851](https://github.com/GitoxideLabs/gitoxide/issues/851)**
    - Fix performance regression in pack traversal ([`de1f6c2`](https://github.com/GitoxideLabs/gitoxide/commit/de1f6c22dfccf22f34e2c2c1b4890ca095ced7ac))
    - Index-backed tree traversal with a form of work-stealing. ([`d22dd8f`](https://github.com/GitoxideLabs/gitoxide/commit/d22dd8fcc22e8dbe30524a1bdddc09bc841db341))
    - Improve performance by avoiding zeroying buffers. ([`14e7ea0`](https://github.com/GitoxideLabs/gitoxide/commit/14e7ea0217af8a04ed2b50ff7b13c28335c29022))
    - Make current thread-count accessible in slice-workers. ([`0fa04bc`](https://github.com/GitoxideLabs/gitoxide/commit/0fa04bcbdf3102c5435e64cfef894a1bfc8d6e7b))
    - Try to get improved hitrate for delta-cache by using it more, and fail ([`969cc77`](https://github.com/GitoxideLabs/gitoxide/commit/969cc77ec7855fc8c23c2b50353813e6a04b779d))
    - Static linked list delta cache with memory cap ([`977e135`](https://github.com/GitoxideLabs/gitoxide/commit/977e135bfa00bdbc1a8f8324f85347ec9078c84f))
    - Check for interrupt more often ([`f89cbc6`](https://github.com/GitoxideLabs/gitoxide/commit/f89cbc675b0acc67322e289e7b3a17288b9eae27))
    - Memory capped hashmap as pack delta cache won't trash memory as much. ([`923692b`](https://github.com/GitoxideLabs/gitoxide/commit/923692bcda698b45d3d1ad518b29f6d30b23fbc0))
 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - `just fmt` ([`ffc1276`](https://github.com/GitoxideLabs/gitoxide/commit/ffc1276e0c991ac33ce842f5dca0b45ac69680c0))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Thanks clippy ([`9525ac8`](https://github.com/GitoxideLabs/gitoxide/commit/9525ac822aa902f5325f17e7b08ffb60b683e0e7))
    - Merge branch 'fix-alloc' ([`d9d9bc0`](https://github.com/GitoxideLabs/gitoxide/commit/d9d9bc01b34ac75b28a5f1b75f40123aa6d83c60))
    - Improve performance by reducing the trashing of `zlib::Inflate`. ([`3a2d528`](https://github.com/GitoxideLabs/gitoxide/commit/3a2d5286084597d4c68549903709cda77dda4357))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Minor fixes ([`89a8cfe`](https://github.com/GitoxideLabs/gitoxide/commit/89a8cfe40e5c3a9d4a4181fa055e3f4a529a8081))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Auto-fix clippy to remove explicit iter looping ([`3eff567`](https://github.com/GitoxideLabs/gitoxide/commit/3eff567c683b5c650c14792b68968cbdbc90ec5c))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Further optimize buffer usage when traversing packs ([`78d28a7`](https://github.com/GitoxideLabs/gitoxide/commit/78d28a72a9ed4773b86b36b23bf200269406384e))
    - Fix incorrect naming of progress of traversal threads ([`996ba6b`](https://github.com/GitoxideLabs/gitoxide/commit/996ba6b20a0e13cd6f448b41797deee97922a3e4))
    - Merge branch 'fix-851' ([`2f275d5`](https://github.com/GitoxideLabs/gitoxide/commit/2f275d5d3cb49b3b8ba53b30e4b4386fac32662b))
    - Adjust to changes in `gix-features` ([`c48bbe3`](https://github.com/GitoxideLabs/gitoxide/commit/c48bbe330e5e99fa357a87a4aa210317ab7c8143))
    - Release gix-object v0.29.2 ([`4f879bf`](https://github.com/GitoxideLabs/gitoxide/commit/4f879bf35653bdc8f9729d524c6e8e1fb3c6886b))
</details>

## 0.35.0 (2023-04-27)

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

## 0.34.0 (2023-04-26)

### New Features (BREAKING)

 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 14 calendar days.
 - 31 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#814](https://github.com/GitoxideLabs/gitoxide/issues/814)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
    - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
 * **Uncategorized**
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`d7173b2`](https://github.com/GitoxideLabs/gitoxide/commit/d7173b2d2cb79685fdf7f618c31c576db24fa648))
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`e4df557`](https://github.com/GitoxideLabs/gitoxide/commit/e4df5574c0813a0236319fa6e8b3b41bab179fc8))
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - Merge branch 'fix-823' ([`6ebd61e`](https://github.com/GitoxideLabs/gitoxide/commit/6ebd61e548a36a04e413ac725a03e607a3588334))
    - Thanks clippy ([`14e64e7`](https://github.com/GitoxideLabs/gitoxide/commit/14e64e74649cfb1f2f99da87015939af98fae5c8))
    - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
    - Adjust to improvements in `gix-object` ([`efeb29d`](https://github.com/GitoxideLabs/gitoxide/commit/efeb29daaf4199748702f42966470a41ded3c6de))
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
</details>

## 0.33.2 (2023-03-26)

A maintenance release without any user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v5.0.2, gix-validate v0.7.4, gix-config v0.20.0, gix-prompt v0.3.3, gix-diff v0.28.1, gix-discover v0.16.1, gix-pack v0.33.2, gix-transport v0.29.1, gix-protocol v0.30.1, gix-revision v0.12.1, gix-worktree v0.15.1, gix v0.43.0, safety bump gix v0.43.0 ([`5dc1f9f`](https://github.com/GitoxideLabs/gitoxide/commit/5dc1f9f2bcb8b3e147115fcb6f76558e8f48ffef))
    - Prepare changelogs prior to release ([`3016a28`](https://github.com/GitoxideLabs/gitoxide/commit/3016a285f566bdfe7de2774fa6f2254c1b1a2c51))
    - Finally fix typos detected by `typos` tool. ([`a69f873`](https://github.com/GitoxideLabs/gitoxide/commit/a69f873ea53b073a0b86ade095596f21709b233b))
</details>

## 0.33.1 (2023-03-14)

A maintenance release without any user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.28.1, gix-tempfile v5.0.1, gix-ref v0.27.1, gix-pack v0.33.1, gix-packetline v0.15.0, gix-transport v0.29.0, gix-protocol v0.30.0, gix v0.42.0, safety bump 3 crates ([`c1f1bfb`](https://github.com/GitoxideLabs/gitoxide/commit/c1f1bfb8dc0e73993678353e4492d0614b642ed1))
    - Prepare changelogs prior to release ([`c66e298`](https://github.com/GitoxideLabs/gitoxide/commit/c66e2982577e4cd9faef63798986b8cf8ece93a2))
    - Make fmt ([`3836cc0`](https://github.com/GitoxideLabs/gitoxide/commit/3836cc0c9c3e1158b56142b924483c8a77217d53))
    - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
</details>

## 0.33.0 (2023-03-10)

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

## 0.32.0 (2023-03-04)

### Bug Fixes

 - <csr-id-ce182d6dae7e277d104893f0eec3285709946233/> don't over-estimate the number of objects to be received in the indexing phase.
   Previously, it would work with a number that is pessimistically duplicated as each object
   could in theory be a ref-delta, which then causes another base to be added to the stream,
   duplicating the amount of objects we need to account for in the acceleration data structure,
   which unfortunatelly cannot actually grow dynamically without violating invariants.
   
   Now we simply don't try to estimata the number of ref-deltas (which don't exist here anymore)
   at this stage.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.28.0, gix-actor v0.19.0, gix-object v0.28.0, gix-diff v0.28.0, gix-traverse v0.24.0, gix-pack v0.32.0, safety bump 20 crates ([`0f411e9`](https://github.com/GitoxideLabs/gitoxide/commit/0f411e93ec812592bb9d3a52b751399dd86f76f7))
    - Prepare changelogs prior to release of `gix-pack` ([`6db30ef`](https://github.com/GitoxideLabs/gitoxide/commit/6db30ef6b5e931bbf12135507a3d922051de4d4b))
    - Don't over-estimate the number of objects to be received in the indexing phase. ([`ce182d6`](https://github.com/GitoxideLabs/gitoxide/commit/ce182d6dae7e277d104893f0eec3285709946233))
</details>

## 0.31.0 (2023-03-01)

<csr-id-3ba25202240d13fdda998581297616afe06422ca/>

### Chore

- <csr-id-3ba25202240d13fdda998581297616afe06422ca/> remove `dashmap` in favor of own sharded concurrent hashmap.
  This speeds up multi-threaded counting greatly, and despite it
  using shared memory which makes it quite wasteful, it is possible to
  outperform `git` with it if enough cores are thrown at the problem.

  Single-threaded performance is still lacking though, ultimately it needs
  caches to accelerate the counting stage to hopefully be competitive.

### New Features

 - <csr-id-f0e40ecddaf1211f76ed60ef30cf03dcfd53a7ab/> add `wasm` feature toggle to allow compilation to wasm32-unknown-unknown
 - <csr-id-3ba25202240d13fdda998581297616afe06422ca/> remove `dashmap` in favor of own sharded concurrent hashmap.
   This speeds up multi-threaded counting greatly, and despite it
   using shared memory which makes it quite wasteful, it is possible to
   outperform `git` with it if enough cores are thrown at the problem.
   
   Single-threaded performance is still lacking though, ultimately it needs
   caches to accelerate the counting stage to hopefully be competitive.

### Bug Fixes

 - <csr-id-e14dc7d475373d2c266e84ff8f1826c68a34ab92/> note that crates have been renamed from `git-*` to `gix-*`.
   This also means that the `git-*` prefixed crates of the `gitoxide` project
   are effectively unmaintained.
   Use the crates with the `gix-*` prefix instead.
   
   If you were using `git-repository`, then `gix` is its substitute.

### New Features (BREAKING)

 - <csr-id-6c4c196c9bc6c2171dc4dc58b69bd5ef53226e29/> add `wasm` feature toggle to let parts of `git-pack` build on wasm32.
   It's a breaking change because we also start using the `dep:` syntax for declaring
   references to optional dependencies, which will prevent them from being automatically
   available as features.
   
   Besides that, it adds the `wasm` feature toggle to allow compiling to `wasm32` targets.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 2 calendar days.
 - 8 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v4.1.0, gix-lock v4.0.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0, safety bump 6 crates ([`ea9fd1d`](https://github.com/GitoxideLabs/gitoxide/commit/ea9fd1d9b60e1e9e17042e9e37c06525823c40a5))
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Remove versions from dev-dependencies to workspace crates. ([`3cfbf89`](https://github.com/GitoxideLabs/gitoxide/commit/3cfbf89f8630dfc71c9085eee6ca286a5c96ad84))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
    - Merge branch 'adjustments-for-cargo' ([`d686d94`](https://github.com/GitoxideLabs/gitoxide/commit/d686d94e1030a8591ba074757d56927a346c8351))
    - Remove `dashmap` in favor of own sharded concurrent hashmap. ([`3ba2520`](https://github.com/GitoxideLabs/gitoxide/commit/3ba25202240d13fdda998581297616afe06422ca))
    - Adapt to changes in `gix-tempfile` ([`bfcd1e3`](https://github.com/GitoxideLabs/gitoxide/commit/bfcd1e3a8f2d05d0d6d1f5cf06d369ac9e6e4cdf))
    - Adapt to changes in `gix-features` ([`a0ed614`](https://github.com/GitoxideLabs/gitoxide/commit/a0ed6142c9a7ce7ed9fb6576117bb46e5497839c))
    - Adjust to changes in `gix-features`, use `process::count_with_decimals()` ([`3968133`](https://github.com/GitoxideLabs/gitoxide/commit/3968133f60609d75806783234253fb8b3972f10e))
    - Prepare for git-tempfile release ([`56c005b`](https://github.com/GitoxideLabs/gitoxide/commit/56c005b13c44376f71e61781e73c0bf93416d0e4))
</details>

## 0.30.3 (2023-02-20)

### Bug Fixes

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

## 0.30.2 (2023-02-17)

<csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/>
<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>
<csr-id-b46347fd3d50886eeca500e31e1e12b354711309/>
<csr-id-591afd56d9862a6348ef8b3af61798004b36aa19/>
<csr-id-2f2d856efe733d3cf81110c0e0607d2e7c40d968/>
<csr-id-9b9f10ad862b5e097c836c51df1eb98607df5ae1/>
<csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/>
<csr-id-71c628d46088ab455b54eb2330d24dcff96c911d/>
<csr-id-8fe461281842b58aa11437445637c6e587bedd63/>
<csr-id-e6ff1a885889cf88f6b34b1193aa03d8bce16af5/>
<csr-id-f48630ba8f745c2ec61a1e3c51fa63a1789a088c/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-25209454d3f7e27e12e8ddca92e43b1ff01d58aa/>
<csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/>

### Refactor (BREAKING)

- <csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/> remove pack-cache from `Find::try_find(…)`
  With the new architecture this can be an implementation detail without
  forcing it to be Sync.
- <csr-id-2290d006705ff47ad780b009fe58ee422b3285af/> move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.
- <csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/> move loose header manipulation from git-pack to git-object

### Other (BREAKING)

- <csr-id-b46347fd3d50886eeca500e31e1e12b354711309/> `index::write::Outcome::index_kind` -> `::index_version`.
- <csr-id-591afd56d9862a6348ef8b3af61798004b36aa19/> `bundle::write::Options::index_kind` -> `::index_version`.
- <csr-id-2f2d856efe733d3cf81110c0e0607d2e7c40d968/> Avoid duplicate module paths in 'tree' and 'commit'

### Bug Fixes (BREAKING)

 - <csr-id-5a75afe0467e4a84323ea10172eed835cc7fae4c/> Reading and writing empty packs is explicitly allowed.
   This can happen when sending packs that don't actually contain changes,
   but need to be sent to conform to the protocol.

### New Features (BREAKING)

 - <csr-id-6c4c196c9bc6c2171dc4dc58b69bd5ef53226e29/> add `wasm` feature toggle to let parts of `git-pack` build on wasm32.
   It's a breaking change because we also start using the `dep:` syntax for declaring
   references to optional dependencies, which will prevent them from being automatically
   available as features.
   
   Besides that, it adds the `wasm` feature toggle to allow compiling to `wasm32` targets.
 - <csr-id-95210cb2ba85f75148b4ef48ccea9d9f8a0a0114/> Provide optional `candidates` for ambigious entries during `lookup_prefix()`
   The candidate entries are all entries matching a given prefix.

### Changed (BREAKING)

 - <csr-id-37f3a675d7fa931f5d3f38e91df74ec0b517422b/> move `data::ResolveBase` into `data::decode_entry::`; unify `decode_entry|decode_header::Error` into `decode::Error`.
 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`
 - <csr-id-8c5ae77f06a64c57df9a9ad1190266896a223dbe/> Remove deprecated compound and linked object databases
   The dynamic/general store is the only maintained can-do-it-all
   DB now.
 - <csr-id-a79a7fb638b45df88af0d0d5fc9ada6d824bc328/> Improve method signatures of `cache::Tree::*`
 - <csr-id-91d047658b114f372735116c9d8e6962a3873137/> cleanup and unify `verify_integrity()` method signature
   Previously they used many different ways of handling their parameters
   despite all boiling down to calling the same 'index::File::traverse()`
   method.
   
   This allows for more reuse of `Options` structs and generally makes
   clearer how these optinos are used.
 - <csr-id-2cf7727228e1d8094ffd2eec6746006348c39eab/> `index::File::traverse()` now returns an `Outcome` struct instead of tuple of 3 fields
 - <csr-id-bf04644ab75ed1969507f957dc8d4868790d462d/> remove `Option<impl Progress>` in favor of `impl Progress`
 - <csr-id-6829e5e5d6aed1e6c87647144e2dd76a1e4b9f1f/> multi-index integrity check; use `integrity::Outcome` for various integrity checks
 - <csr-id-d851bede97801096d188ff6af06c98a79fe276db/> remove unnecessary `Arc` around `should_interrupt` flag
 - <csr-id-c2679a03358b9c19d63ed1af1cd57324c6381447/> remove Sha1 mentions in `index::verify::Mode::*` variants
   The hash is repository defined and not hard-coded
 - <csr-id-80b120d3278e46429f848df7af3db13413c36649/> introduce `index::File::verify_integrity(…, pack: Option<PackContext>, …)`, replacing tuple
   This allows for more documentation on what input is required there and
   generally makes for an easier to use API.
 - <csr-id-79dc0d5ba6fa31ddd5c075693ffdc6496c1eaded/> rename `oid::try_from()` to `try_from_bytes()`, add `from_bytes_unchecked()`
   This change was done in the name of consistency, as `from_bytes()` is
   used in many other git-* crates
 - <csr-id-2ef9a8424af51310db8c1e6df31dde9953ed3d21/> Change accessors named `hash_kind()` to `object_hash()` for consistency
 - <csr-id-b76f6be6c5baa6cf613a174241f007e92bf5ba36/> consistently use `object_hash` instead of `hash_kind`
 - <csr-id-629412b4cb192614b7eff08dbf203e3448c902c1/> data::Entry::from_read() now takes a hash lengths as parameter
   That way ref-deltas can be interepreted without hard-coding SHA1
 - <csr-id-851dc2c52fa8e204ba2d5ced8fb0959a889869d8/> data::Entry::from_bytes(…, hash_len) takes new parameter
   The hash-len tells it how to interpret ref-delta objects, which
   store the complete hash of the base object.
   
   This is now entirely configurable.
 - <csr-id-db8c8c41b4ced0fc296d3877883d801e77d550ae/> `index::File::at()` with git_hash::Kind parameter
   It will allow to assume different hashes even in the index file format
   which isn't yet capable of storing this information.
 - <csr-id-e6a3c9f72332b524b143bc94ee9df0a6db11e864/> `data::File::at()` and `Bundle::at()` now have `hash_kind` parameter
   It's used to configure the kind of hash to assume when reading packs and
   indices.
 - <csr-id-82b9b33bd5f4c3c1721a5093de2cedc62cb10565/> move `bundle::Location` to `data::entry::Location`
   The latter place best describes its purpose.
 - <csr-id-3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7/> remove `make_object_cache` parameter from `git_pack::data::output::count::objects()`
   It now is an implementation detail of the Find trait.
 - <csr-id-e7526b2a7b51cbac4018e1ab3b623a85987fadc2/> parallel utilities now use `Send + Clone` insted of `Send + Sync`
   This helps to assure that thread-local computations always work with the
   kind of types we provide. The ones that are carrying out actions are
   notably not `Sync` anymore.
   
   We cater to that by defining our bounds accordingly, but for those
   who want to use other utilities that need Sync, using types like
   `Repository` and `thread_local!()` is the only way to make this
   work.
 - <csr-id-e8b091943f0c9a26317da0003f7fcdf5a56ef21a/> Rename gix->ein and gixp->gix

### Refactor

- <csr-id-9b9f10ad862b5e097c836c51df1eb98607df5ae1/> remove unnecessary unsafe by using `chunks_mut()`
  This was probably a left-over from times where there was a static
  requirement on the chunks processing. Maybe… .
- <csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/> replace bare u32 `data::Id` typedef
- <csr-id-71c628d46088ab455b54eb2330d24dcff96c911d/> Use 'cache::Object' trait where it matters
- <csr-id-8fe461281842b58aa11437445637c6e587bedd63/> split data::output::count::objects into files

### Performance

 - <csr-id-f9232acf8e52f8cd95520d122469e136eb07b39f/> ObjectID specific hashers, using the fact that object ids are hashes

### Other

- <csr-id-e6ff1a885889cf88f6b34b1193aa03d8bce16af5/> :File uses its hash_len parameter
- <csr-id-f48630ba8f745c2ec61a1e3c51fa63a1789a088c/> :Find implementation for Rc

### Bug Fixes

 - <csr-id-ec58bef84d620360dc52e34e173ace1310f74c85/> reduce memory usage for pack resolution by 18%…
   …compared to where it started out before.
   
   This is the result of using u32 intead of usize where possible,
   leveraging the object limit of u32 in packs.
   
   This change makes the biggest difference by not keeping the decompressed
   memory of leaf nodes alive for longer than needed, at the cost of
   some code duplication which could be extracted into a function if one
   was inclined to deal with the boilerplate and lots of generics.
 - <csr-id-49d168a8859c6b5a9e7ef58cd836093212c2c7ad/> Allow verification of empty packs and indices.
   Empty packs are fine, even though the implementation should probably
   assure to not write them, or remove them after having been written.
 - <csr-id-a745512185fb0a46e35daaa6d28829aec05edb55/> increase pack-receive performance using a BufWriter
   Previously the NamedTempFile would receive every little write request
   for millions of objects, consuming considerable amounts of time.
   
   Now a buf writer alleviates this issue entirely.
 - <csr-id-0b6ed60f842f0a36f61f187651080540a358758e/> `bundle::write::Error` is now publicly available
 - <csr-id-d9451e8d7fc39c252042f9d2447061262c16ae7a/> downgrade dashmap to 4.0 to avoid unsoundness.
   See https://github.com/xacrimon/dashmap/issues/167 for tracking
   progress on resolving the issue.
 - <csr-id-42e0487286c1f745837c0ce337ed7c9d86b14516/> support Rust 1.52
 - <csr-id-84ade1d23060f10bf6c8529f8f693d06660b4f4e/> Allow resolution of in-pack ref-deltas
   This finally allows delta tree caches to be used on typical small packs
   returned by GitHub.
 - <csr-id-ba92cc09ba41fe4c9a9097bfeb8d18016408fcdf/> don't try to short-cut detection of large offsets when writing index files
   The code incorrectly assumed that the input is sorted by offsets, with
   the largest offset being last, even though by all means that's not the
   case.
 - <csr-id-6d3f52dc13d7243a6bce6dab89a985114a75d94b/> Avoid the dashmap being cloned for each thread
   Intead, share it by reference, it's sync after all.
   
   This issue was introduced when switching to a `Send + Clone` model,
   instead of `Send + Sync`, to allow thread-local caches in database
   handles of all kinds.
 - <csr-id-b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e/> linked::Store now assures unique IDs across compound stores
 - <csr-id-20b3994206aa5bc5e35cbbc9c8f8f99187077f79/> Adjust size-hints of resolving entries iterator and use the upper bound in delta tree
   The delta-tree is a data structure that actually heavily relies on
   favorable allocation and a known amount of objects in order to
   provide front and back buffers. However, this is an implementation
   detail and they don't have to stay consistent at all especially
   after growing the buffer by pushing to it.
   
   Interestingly, the VecDeque internally over-allocates as well which
   definitely helps the example of `as_mut_slices()`, otherwise
   it could also suffer from the assertions that trigger here.
 - <csr-id-d8fe8141e80a9e9a433b5e1a072b850325c806c8/> don't put more objects into the pack cache than needed
   Previously when accessing a packed object, it would store the base
   object into the pack cache (if it wasn't retrieved from there)
   which is great if that operation is free.
   
   Since it isn't, it's better not to stress the cache with puts
   and trash more objects than necessary.
   
   Now only the last decompressed object will be put into the LRU cache.
 - <csr-id-faf6f813927720c5adf62102f9ce46606ff2617c/> don't include submodules in count…
   …to avoid dealing with missing objects.
   
   It's still a good idea to handle these gracefully though, git itself
   seems to ignore them.

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-25209454d3f7e27e12e8ddca92e43b1ff01d58aa/> upgrade dashmap to 5.1.0 (with security fix)
- <csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/> remove unused dependencies

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### New Features

 - <csr-id-d792ea543246632bf1ca8d0e1d239bbe7f07e219/> use enumerations to advertise progress ids publicly.
   Previously these were an implementation detail which also means they
   couldn't be relied upon.
   
   Thanks to an intermediate enumeration, they become part of the public API
   and their actual value is not exposed.
 - <csr-id-f0e40ecddaf1211f76ed60ef30cf03dcfd53a7ab/> add `wasm` feature toggle to allow compilation to wasm32-unknown-unknown
 - <csr-id-a85dcddba29a453bbea87ac865b8aedc97f41aed/> add `data::File::resolve_header()` to obtain object information without decompressing it.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.
 - <csr-id-450257eb596465352fb363a5c8c514f544e7f9ac/> `Bundle::write…` also produces a `.keep` file
   These files are placed before moving the coresponding pack and index
   files into place to prevent them from being picked up for collection
   while their refs are still being placed.
   
   The caller is responsible for deleting them once refs integrate the
   contained objects into the commit graph.
 - <csr-id-6836cb148cbadf919bdac2e0e77bdec34caa9307/> more efficient distribution of tasks to threads during pack resolution.
   This clearly is a peak-performance optimization as it will now
   efficiently distribut tree-root nodes to threads one at a time by
   means of shared memory, which is accessed mutably yet safely.
   
   This change can also further reduce peak memory usage as it will not
   keep buffers of more resolved deltas then necessary, as it only handles
   one at a time per thread.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-503b1a1f8d4f39b44c166209d7a8ba8d74137859/> `index::File::lookup_prefix(…)`
 - <csr-id-cb83beedd1aa389f6774e2296f79273e8c8f14f4/> git-hash::Prefix::from_id()
   A way to obtain a prefix of an object id, with all non-prefix
   bytes set to zero.
 - <csr-id-16208306ab49ade30d8ffd6b067ebd8eefd84cd4/> in-manifest and in-lib documentation of feature toggles
 - <csr-id-b80dec2323b81fb2172df76c7d897a4b5e6bdfea/> zero-objects check for index and multi-index integrity validation
 - <csr-id-56fc99fb9c1cab61abd03c10e1b4af0d6e491bbf/> support for fan-checking in index and multi-index integrity verification
 - <csr-id-28e3ea8612112f6a04cfaff591565eca5a1ffba2/> introduce type for entry indices within an index or multi-index
   That way it's a littl emore descriptive than a bare u32.
 - <csr-id-58c2edb76755ab71e10eef4cd9a51533825c291f/> git_pack::Find::try_find_cached(…, pack_cache)
   With this method it's easier to bypass local caches and control
   the cache oneself entirely.
 - <csr-id-e25f4eadec679406aad6df10026e27e4832c2482/> A simplified version of the `Find` trait
   It's meant for the next generation of object db handles which keep a
   local cache of all the details of the actual object database.
 - <csr-id-60c9fad8002b4e3f6b9607bba6361871752f4d3d/> control pack and object cache size in megabytes
 - <csr-id-50cf610e8939812c3d2268c48835e2dac67d0c31/> cache::Object trait for caching and retrieving whole objects
 - <csr-id-5a8c2da6cb1e2accf7cfdccc16bc3a1d0b2a7dbc/> object cache size is configurable
 - <csr-id-d6c44e6ab8f436020d4fb235e423b018fd1e7a9f/> dynamically sized full-object speeds up diff-based object counting…
   …which is what happens when counting objects for fetches where only
   changed objects should be sent.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 717 commits contributed to the release.
 - 75 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 27 unique issues were worked on: [#164](https://github.com/GitoxideLabs/gitoxide/issues/164), [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#247](https://github.com/GitoxideLabs/gitoxide/issues/247), [#250](https://github.com/GitoxideLabs/gitoxide/issues/250), [#254](https://github.com/GitoxideLabs/gitoxide/issues/254), [#259](https://github.com/GitoxideLabs/gitoxide/issues/259), [#260](https://github.com/GitoxideLabs/gitoxide/issues/260), [#263](https://github.com/GitoxideLabs/gitoxide/issues/263), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#287](https://github.com/GitoxideLabs/gitoxide/issues/287), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#333](https://github.com/GitoxideLabs/gitoxide/issues/333), [#364](https://github.com/GitoxideLabs/gitoxide/issues/364), [#384](https://github.com/GitoxideLabs/gitoxide/issues/384), [#422](https://github.com/GitoxideLabs/gitoxide/issues/422), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 26 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#164](https://github.com/GitoxideLabs/gitoxide/issues/164)**
    - Avoid duplicate module paths in 'tree' and 'commit' ([`2f2d856`](https://github.com/GitoxideLabs/gitoxide/commit/2f2d856efe733d3cf81110c0e0607d2e7c40d968))
 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Fix stop-release-for-changelog logic and fix all affected changelogs ([`52b38bc`](https://github.com/GitoxideLabs/gitoxide/commit/52b38bc4856be5ba8b5372a3dd20f5d06504e7ed))
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Allow 'refactor' and 'other' in conventional messages if they have breaking changes ([`4eebaac`](https://github.com/GitoxideLabs/gitoxide/commit/4eebaac669e590beed112b622752997c64772ef1))
    - Rebuild all changelogs to assure properly ordered headlines ([`4a9a05f`](https://github.com/GitoxideLabs/gitoxide/commit/4a9a05f95930bad5938d4ce9c517ebf0e0b990f1))
    - Sort all commits by time, descending… ([`f536bad`](https://github.com/GitoxideLabs/gitoxide/commit/f536bad20ffbac4dc353dfeb1a917bb88becbb78))
    - Greatly reduce changelog size now that the traversal fix is applied ([`a0bc98c`](https://github.com/GitoxideLabs/gitoxide/commit/a0bc98c06c349de2fd6e0d4593606e68b98def72))
    - Don't put more objects into the pack cache than needed ([`d8fe814`](https://github.com/GitoxideLabs/gitoxide/commit/d8fe8141e80a9e9a433b5e1a072b850325c806c8))
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
 * **[#247](https://github.com/GitoxideLabs/gitoxide/issues/247)**
    - Rename gix->ein and gixp->gix ([`e8b0919`](https://github.com/GitoxideLabs/gitoxide/commit/e8b091943f0c9a26317da0003f7fcdf5a56ef21a))
 * **[#250](https://github.com/GitoxideLabs/gitoxide/issues/250)**
    - Address FIXME related to git_pack::data::Object ([`96386fd`](https://github.com/GitoxideLabs/gitoxide/commit/96386fd1379b32ce2333baf34f81133cb9817364))
    - Move loose header manipulation from git-pack to git-object ([`598698b`](https://github.com/GitoxideLabs/gitoxide/commit/598698b88c194bc0e6ef69539f9fa7246ebfab70))
 * **[#254](https://github.com/GitoxideLabs/gitoxide/issues/254)**
    - Adjust changelogs prior to git-pack release ([`6776a3f`](https://github.com/GitoxideLabs/gitoxide/commit/6776a3ff9fa5a283da06c9ec5723d13023a0b267))
    - Minor refactor ([`227c8b1`](https://github.com/GitoxideLabs/gitoxide/commit/227c8b1859a6cbf96d48fd8564e575ef7e201db1))
    - Adjust size-hints of resolving entries iterator and use the upper bound in delta tree ([`20b3994`](https://github.com/GitoxideLabs/gitoxide/commit/20b3994206aa5bc5e35cbbc9c8f8f99187077f79))
 * **[#259](https://github.com/GitoxideLabs/gitoxide/issues/259)**
    - Sketch a little more how packs could be accessed ([`3fce8f2`](https://github.com/GitoxideLabs/gitoxide/commit/3fce8f2b35ec6c2076f66fdde16a5f99a68326ac))
    - Unify trait bounds for parallel code: prefer Clone over Sync ([`c805d0b`](https://github.com/GitoxideLabs/gitoxide/commit/c805d0b231cf4d2f51dae7705bfbbc6562f86c32))
    - Remove trait bounds to allow single-threaded applications to exist ([`3c790e0`](https://github.com/GitoxideLabs/gitoxide/commit/3c790e01de0dbd3ffa2683d5cf060723d11d64a5))
    - Turns out the new `PolicyStore` can co-exist with existing one… ([`5e9250f`](https://github.com/GitoxideLabs/gitoxide/commit/5e9250f5027e4b2c701ceae72a6038ac2a4a2093))
 * **[#260](https://github.com/GitoxideLabs/gitoxide/issues/260)**
    - Linked::Store now assures unique IDs across compound stores ([`b605c1f`](https://github.com/GitoxideLabs/gitoxide/commit/b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e))
 * **[#263](https://github.com/GitoxideLabs/gitoxide/issues/263)**
    - Fmt ([`fbeddeb`](https://github.com/GitoxideLabs/gitoxide/commit/fbeddebcab999f4898f768a3184906091f8ce0b8))
    - Parallel utilities now use `Send + Clone` insted of `Send + Sync` ([`e7526b2`](https://github.com/GitoxideLabs/gitoxide/commit/e7526b2a7b51cbac4018e1ab3b623a85987fadc2))
    - A mad attempt to use thread-local everywhere and avoid Sync… ([`0af5077`](https://github.com/GitoxideLabs/gitoxide/commit/0af5077e1f028c1c69bbdc098bb567e486282c37))
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - Remove unused dependencies ([`c800fdd`](https://github.com/GitoxideLabs/gitoxide/commit/c800fdd331e6d7a0b8d756ba822915259f26e9e8))
    - Upgrade dashmap to latest version ([`52d4fe5`](https://github.com/GitoxideLabs/gitoxide/commit/52d4fe55b6dd88f72479abd4015cab063ddaaf97))
    - Refactor ([`b88f253`](https://github.com/GitoxideLabs/gitoxide/commit/b88f253e46e7ad0a50b670b96c1bfa09eaaecaef))
    - Refactor ([`52a4dcd`](https://github.com/GitoxideLabs/gitoxide/commit/52a4dcd3a6969fa8f423ab39c875f98f9d210e95))
    - Make single-threaded programs possible to use with git-repository ([`dde5c6b`](https://github.com/GitoxideLabs/gitoxide/commit/dde5c6ba76ff849f69f742c985b4bc65ca830883))
    - Use new odb in place of the old one and it works ([`8ad25c5`](https://github.com/GitoxideLabs/gitoxide/commit/8ad25c581bc79041545a72baf57b0a469d99cc30))
    - Make find::Entry self-contained ([`ad36fb9`](https://github.com/GitoxideLabs/gitoxide/commit/ad36fb9b800c17931ce358ac262bef40d43dcfb3))
    - Remove iterator access in favor of fully owned data ([`62d3f10`](https://github.com/GitoxideLabs/gitoxide/commit/62d3f106437e597a41aae592da28f48e8736b143))
    - Remove CRC32 check entirely as it doesn't seem to be important in the big picture ([`22d35bd`](https://github.com/GitoxideLabs/gitoxide/commit/22d35bdbc271ccada8d68a1450d9a2533fc739ee))
    - Notes about multi-pack indices in the current data::entry::location ([`7eff6bf`](https://github.com/GitoxideLabs/gitoxide/commit/7eff6bf525ea48fa913149911ea4c8fe742a25a3))
    - Move `bundle::Location` to `data::entry::Location` ([`82b9b33`](https://github.com/GitoxideLabs/gitoxide/commit/82b9b33bd5f4c3c1721a5093de2cedc62cb10565))
    - Use existing git_features facilities ([`ed0c266`](https://github.com/GitoxideLabs/gitoxide/commit/ed0c2662d95b74b4abc09b42fc24cb56219dd511))
    - Adjust pack-create to changes in git-pack ([`12db899`](https://github.com/GitoxideLabs/gitoxide/commit/12db899a72da6decccd82931637d074059b578f5))
    - Remove `make_object_cache` parameter from `git_pack::data::output::count::objects()` ([`3f05fea`](https://github.com/GitoxideLabs/gitoxide/commit/3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7))
    - :Find implementation for Rc ([`f48630b`](https://github.com/GitoxideLabs/gitoxide/commit/f48630ba8f745c2ec61a1e3c51fa63a1789a088c))
    - MultiPackIndex compatible pack::Find trait definition ([`5fa1a9d`](https://github.com/GitoxideLabs/gitoxide/commit/5fa1a9dce59c2654374a532d024c8de5959d4d0f))
    - Git_pack::Find::try_find_cached(…, pack_cache) ([`58c2edb`](https://github.com/GitoxideLabs/gitoxide/commit/58c2edb76755ab71e10eef4cd9a51533825c291f))
    - Refactor ([`3310d8f`](https://github.com/GitoxideLabs/gitoxide/commit/3310d8f271f74fc6084e33dd9bd4c5f01b54e432))
    - Remove pack-cache from `Find::try_find(…)` ([`ebc7f47`](https://github.com/GitoxideLabs/gitoxide/commit/ebc7f47708a63c3df4415ba0e702660d976dfb3e))
    - Fix docs ([`1bb4253`](https://github.com/GitoxideLabs/gitoxide/commit/1bb425347e4b502e1c048908cd5f3641d2b16896))
    - Move git_pack::data::Object to git_object::Data, massively alter git_odb::Find trait ([`2290d00`](https://github.com/GitoxideLabs/gitoxide/commit/2290d006705ff47ad780b009fe58ee422b3285af))
    - A simplified version of the `Find` trait ([`e25f4ea`](https://github.com/GitoxideLabs/gitoxide/commit/e25f4eadec679406aad6df10026e27e4832c2482))
    - Add 'contains()' method to Find ([`dfdd6fb`](https://github.com/GitoxideLabs/gitoxide/commit/dfdd6fb2c83e5d09c3a56936723bc6749ac4b99a))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Add a less thorough and faster way of verifying multi-indices ([`7517482`](https://github.com/GitoxideLabs/gitoxide/commit/75174825e1012cfb4c34c18391c681b49c2f0d29))
    - Refactor ([`91e6d38`](https://github.com/GitoxideLabs/gitoxide/commit/91e6d382bb2e2430d5d3325a390b7d9bdc0034d6))
    - Allow interrupting multi-index creation more often ([`f223ecb`](https://github.com/GitoxideLabs/gitoxide/commit/f223ecb6c69358ed8e38d796aca9bef21173cc92))
    - Also test pack-creation with multi-index repo ([`235a27a`](https://github.com/GitoxideLabs/gitoxide/commit/235a27a925e9b5f6729056ac44e8107dcba55cfd))
    - Better multi-pack verification progress ([`2e16f13`](https://github.com/GitoxideLabs/gitoxide/commit/2e16f1321bdccc2cef688d27efd9cc9be1360c31))
    - Handle large multi-pack indices correctly ([`4f6b030`](https://github.com/GitoxideLabs/gitoxide/commit/4f6b0308f06b7705163ff624a98694e1d928fee1))
    - Fix progress and handling of large of multi-pack index offsets ([`5dc1f81`](https://github.com/GitoxideLabs/gitoxide/commit/5dc1f813ead64ad13edb2b5ed9bd660d198c7ddb))
    - Add missing docs ([`4137327`](https://github.com/GitoxideLabs/gitoxide/commit/41373274fc7f23e3fed17dc52e3e3e94c2e9e41a))
    - Write progress for multi-pack writing ([`1bea1d4`](https://github.com/GitoxideLabs/gitoxide/commit/1bea1d47908d3ec44c83b2e39a5b67134ad51ee0))
    - Adapt to changes in git-features ([`542c0df`](https://github.com/GitoxideLabs/gitoxide/commit/542c0df9f7498a53a4561e4286b8fdb888565cd3))
    - Progress for chunk writing ([`50fde01`](https://github.com/GitoxideLabs/gitoxide/commit/50fde01b44a0a720ccb874bc23a818334238c6e0))
    - Multi-pack index writing complete with large-offset support ([`f7d5c7f`](https://github.com/GitoxideLabs/gitoxide/commit/f7d5c7f815dbf52c668444b316ae2e1485463bcb))
    - Write pack-ids and offsets ([`bfc8069`](https://github.com/GitoxideLabs/gitoxide/commit/bfc8069e6da2ec6d87fa40bbaaca247c1e247d5f))
    - Add chunk for oids ([`565a7ae`](https://github.com/GitoxideLabs/gitoxide/commit/565a7aea9341a0f0005a41bc6687fbaacb0c0b97))
    - Write the fanout table ([`6a68ed7`](https://github.com/GitoxideLabs/gitoxide/commit/6a68ed7708bdbb29c40bcea0dc7cf681c0aff75b))
    - Refactor ([`93dc660`](https://github.com/GitoxideLabs/gitoxide/commit/93dc660aa34c18b5186c57c6a3fad547a63d5eec))
    - Write multi-index header along with path-names chunk ([`2fc6751`](https://github.com/GitoxideLabs/gitoxide/commit/2fc67512f8be2860ab06dc5a282f4f6550c3fddb))
    - Sketch all the chunk-write API and use it from multi-index write ([`5457761`](https://github.com/GitoxideLabs/gitoxide/commit/545776180f75cba87f7119f9bd862d39f081f1bd))
    - Add frame for writing a multi-pack index ([`9ce1e7f`](https://github.com/GitoxideLabs/gitoxide/commit/9ce1e7f2d8c7133590f571919850eaa763f789e3))
    - `index::File::traverse()` now returns an `Outcome` struct instead of tuple of 3 fields ([`2cf7727`](https://github.com/GitoxideLabs/gitoxide/commit/2cf7727228e1d8094ffd2eec6746006348c39eab))
    - Refactor ([`c361ee3`](https://github.com/GitoxideLabs/gitoxide/commit/c361ee399e9c435b087387c1542b3838c21fad03))
    - Multi-index verification now matches that of git itself ([`3a76a28`](https://github.com/GitoxideLabs/gitoxide/commit/3a76a28e6af11950e8a808d09c36c2ee8b655944))
    - Zero-objects check for index and multi-index integrity validation ([`b80dec2`](https://github.com/GitoxideLabs/gitoxide/commit/b80dec2323b81fb2172df76c7d897a4b5e6bdfea))
    - Support for fan-checking in index and multi-index integrity verification ([`56fc99f`](https://github.com/GitoxideLabs/gitoxide/commit/56fc99fb9c1cab61abd03c10e1b4af0d6e491bbf))
    - More detailed multi-index verification ([`8f9a55b`](https://github.com/GitoxideLabs/gitoxide/commit/8f9a55bb31af32b266d7c53426bc925361a627b2))
    - Even nicer printing ([`d2bea27`](https://github.com/GitoxideLabs/gitoxide/commit/d2bea270787597d6aef48ffe023ff49969c33bd9))
    - Nicer printing of index verification results ([`e3dfa12`](https://github.com/GitoxideLabs/gitoxide/commit/e3dfa123b368e66f39567bd2a8f5d7d9c09d4fe6))
    - Very first experimental support for multi-pack index verification ([`bb35c69`](https://github.com/GitoxideLabs/gitoxide/commit/bb35c6994765ec3bbbcfde247911d1ffe711a23d))
    - Remove `Option<impl Progress>` in favor of `impl Progress` ([`bf04644`](https://github.com/GitoxideLabs/gitoxide/commit/bf04644ab75ed1969507f957dc8d4868790d462d))
    - Multi-index integrity check; use `integrity::Outcome` for various integrity checks ([`6829e5e`](https://github.com/GitoxideLabs/gitoxide/commit/6829e5e5d6aed1e6c87647144e2dd76a1e4b9f1f))
    - Remove unnecessary `Arc` around `should_interrupt` flag ([`d851bed`](https://github.com/GitoxideLabs/gitoxide/commit/d851bede97801096d188ff6af06c98a79fe276db))
    - Remove Sha1 mentions in `index::verify::Mode::*` variants ([`c2679a0`](https://github.com/GitoxideLabs/gitoxide/commit/c2679a03358b9c19d63ed1af1cd57324c6381447))
    - Introduce `index::File::verify_integrity(…, pack: Option<PackContext>, …)`, replacing tuple ([`80b120d`](https://github.com/GitoxideLabs/gitoxide/commit/80b120d3278e46429f848df7af3db13413c36649))
    - Multi-index verify checksum ([`853d468`](https://github.com/GitoxideLabs/gitoxide/commit/853d4683aae5f4dd4667b452932bd57f99f6afab))
    - Fix docs ([`ce044ef`](https://github.com/GitoxideLabs/gitoxide/commit/ce044ef146e3d67483bed382f5dd5c484699534e))
    - Introduce type for entry indices within an index or multi-index ([`28e3ea8`](https://github.com/GitoxideLabs/gitoxide/commit/28e3ea8612112f6a04cfaff591565eca5a1ffba2))
    - Replace bare u32 `data::Id` typedef ([`e0b8636`](https://github.com/GitoxideLabs/gitoxide/commit/e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c))
    - Adjust to changes in git-odb ([`710780c`](https://github.com/GitoxideLabs/gitoxide/commit/710780cd355793ea638767213f250e026997a530))
    - Add remaining docs for multi-index ([`10a24c1`](https://github.com/GitoxideLabs/gitoxide/commit/10a24c1860e63935b435e985900797b2d4c707a8))
    - Docs for multi_index::chunk ([`73fbc91`](https://github.com/GitoxideLabs/gitoxide/commit/73fbc915847b7c458a17bdfbb7fa1de3f31ab437))
    - Refactor ([`eafdff4`](https://github.com/GitoxideLabs/gitoxide/commit/eafdff405b3f408aa5203f40c7f0a570ce20655d))
    - Multi-index iteration ([`1c99903`](https://github.com/GitoxideLabs/gitoxide/commit/1c999035cc3649ab9db02bd82644fb54c408f6d2))
    - Access pack-indices and pack-offsets of multi-pack indices ([`c2a6918`](https://github.com/GitoxideLabs/gitoxide/commit/c2a69189f88c53ab555158245ce647fcd33fca6a))
    - Oid lookup for multi-pack indices ([`254f618`](https://github.com/GitoxideLabs/gitoxide/commit/254f618ee410be4a2787f599529a6cca1284a0fb))
    - Add basic oid by multi-index file index ([`a54f552`](https://github.com/GitoxideLabs/gitoxide/commit/a54f552741aed315b21112576d6e5b704a9439d4))
    - Rename `oid::try_from()` to `try_from_bytes()`, add `from_bytes_unchecked()` ([`79dc0d5`](https://github.com/GitoxideLabs/gitoxide/commit/79dc0d5ba6fa31ddd5c075693ffdc6496c1eaded))
    - Change accessors named `hash_kind()` to `object_hash()` for consistency ([`2ef9a84`](https://github.com/GitoxideLabs/gitoxide/commit/2ef9a8424af51310db8c1e6df31dde9953ed3d21))
    - Adapt to changes in git-hash ([`754a663`](https://github.com/GitoxideLabs/gitoxide/commit/754a66344ff2cfcfc4a7a3d72f1240e939c48055))
    - Remove unnecessary `Default` implementation for user of Tree::traverse ([`9da20e9`](https://github.com/GitoxideLabs/gitoxide/commit/9da20e92c96e4ce8dd75e141c24143e4ea1141a7))
    - Remove unnecessary Default bound for data in Tree nodes ([`d548f72`](https://github.com/GitoxideLabs/gitoxide/commit/d548f726013df409b0e1a5fb0e39c15ff445228d))
    - Adjust to changes in git-hash ([`9bf25cc`](https://github.com/GitoxideLabs/gitoxide/commit/9bf25cc4f2e44821f93e85997677bc4e86a67bd4))
    - Consistently use `object_hash` instead of `hash_kind` ([`b76f6be`](https://github.com/GitoxideLabs/gitoxide/commit/b76f6be6c5baa6cf613a174241f007e92bf5ba36))
    - Adjust to changes in git-hash ([`ca35246`](https://github.com/GitoxideLabs/gitoxide/commit/ca35246a91888ae41805d71082055c98d2ff7f0b))
    - Adjust to changes in git-hash and git-pack ([`0cae25b`](https://github.com/GitoxideLabs/gitoxide/commit/0cae25b1bb3c902ec323f17a1d9743e42fe213d0))
    - Data::Entry::from_read() now takes a hash lengths as parameter ([`629412b`](https://github.com/GitoxideLabs/gitoxide/commit/629412b4cb192614b7eff08dbf203e3448c902c1))
    - Data::Entry::from_bytes(…, hash_len) takes new parameter ([`851dc2c`](https://github.com/GitoxideLabs/gitoxide/commit/851dc2c52fa8e204ba2d5ced8fb0959a889869d8))
    - Refactor ([`7331e99`](https://github.com/GitoxideLabs/gitoxide/commit/7331e99cb88df19f7b1e04b1468584e9c7c79913))
    - Adjust to changes in git-hash ([`07aa1bc`](https://github.com/GitoxideLabs/gitoxide/commit/07aa1bca225c30b168a597f920bda392b2cb2713))
    - :File uses its hash_len parameter ([`e6ff1a8`](https://github.com/GitoxideLabs/gitoxide/commit/e6ff1a885889cf88f6b34b1193aa03d8bce16af5))
    - `index::File::at()` with git_hash::Kind parameter ([`db8c8c4`](https://github.com/GitoxideLabs/gitoxide/commit/db8c8c41b4ced0fc296d3877883d801e77d550ae))
    - `data::File::at()` and `Bundle::at()` now have `hash_kind` parameter ([`e6a3c9f`](https://github.com/GitoxideLabs/gitoxide/commit/e6a3c9f72332b524b143bc94ee9df0a6db11e864))
    - Remove unnecessary dev-depednency ([`b71ea6a`](https://github.com/GitoxideLabs/gitoxide/commit/b71ea6a89d11d6cac01b7d9e9b1101f4d637617c))
    - Adapt to changes in git-hash ([`82fec95`](https://github.com/GitoxideLabs/gitoxide/commit/82fec95e9ed4b924849bfcc84b5b2691a925a5b3))
    - Calculate trailer offset instead of storing it ([`bf62067`](https://github.com/GitoxideLabs/gitoxide/commit/bf62067c690e407e2ace66220337359542e1846a))
    - Make pessimistic size-estimation instead of an optimistic one ([`69f1d2a`](https://github.com/GitoxideLabs/gitoxide/commit/69f1d2a2063cfebae3ea70979d950f8ab7751eac))
    - Refactor ([`8b8b4c5`](https://github.com/GitoxideLabs/gitoxide/commit/8b8b4c538823fb4d2c37be80340d843080f08d19))
    - Refactor ([`8c9c7fc`](https://github.com/GitoxideLabs/gitoxide/commit/8c9c7fc3bc46afa9c8567a8bc8079cac12ed8422))
    - Adapt to changes in git-chunk ([`44ea5c3`](https://github.com/GitoxideLabs/gitoxide/commit/44ea5c3c334399bc03d92fa20171d2c0c3afdf49))
    - Refactor ([`ac46765`](https://github.com/GitoxideLabs/gitoxide/commit/ac4676534573e3ccfa219765e645526797c6d71b))
    - Adapt to latest changes to git-chunk ([`743d696`](https://github.com/GitoxideLabs/gitoxide/commit/743d6967d6236a4bb6a9c8817f957e7604bc9264))
    - Provide multi-index checksum ([`a363de9`](https://github.com/GitoxideLabs/gitoxide/commit/a363de9b8271986385b1d57e61a6c103c20a4055))
    - Update changelog prior to release ([`6ae49e3`](https://github.com/GitoxideLabs/gitoxide/commit/6ae49e39b2251ad70b72a8f3b3840ebb9334ffd9))
    - Completely validate and parse multi-index file ([`e7e40c3`](https://github.com/GitoxideLabs/gitoxide/commit/e7e40c30dea082d004e8781ef7d36bde0afdd8a7))
    - Read and validate index names contained in the multi-pack index ([`24a9790`](https://github.com/GitoxideLabs/gitoxide/commit/24a979036df515f0616738825e669ec9c8dab1f1))
    - Read and validate fanout chunk ([`3ca04e3`](https://github.com/GitoxideLabs/gitoxide/commit/3ca04e355a413975e55adf8b204d6962a9341d32))
    - Read all mandatory and optional chunks ([`99023bb`](https://github.com/GitoxideLabs/gitoxide/commit/99023bbde027be82e9217868df7f73ecd09bf705))
    - Load chunk index of midx file ([`fac8efa`](https://github.com/GitoxideLabs/gitoxide/commit/fac8efacb31935c2143717ebe82003a0916f233f))
    - Frame for git-chunk crate to share among git-pack and git-commitgraph ([`b2d2ae2`](https://github.com/GitoxideLabs/gitoxide/commit/b2d2ae221d43cc14aa169ada3c471e2bd2adadf4))
    - Basic midx header parsing ([`edf02ae`](https://github.com/GitoxideLabs/gitoxide/commit/edf02ae46ce6f3f981acd99310878e1d4a00d23b))
    - First pieces of header parsing; allow to respect multi-index desired hash kind in git-odb ([`1a2a049`](https://github.com/GitoxideLabs/gitoxide/commit/1a2a04930ab56ba778091e10b15cecf415f5058d))
    - Frame for instantiation of multi-pack-index ([`5e085ec`](https://github.com/GitoxideLabs/gitoxide/commit/5e085ecbea913e0b0191d8267e548fe859bdd5d9))
 * **[#287](https://github.com/GitoxideLabs/gitoxide/issues/287)**
    - Way nicer progress messages for repo verification ([`4b4f9f8`](https://github.com/GitoxideLabs/gitoxide/commit/4b4f9f81879ad181744022eb0d7dc02392a5e91e))
    - Upgrade to prodash 17 ([`47860b7`](https://github.com/GitoxideLabs/gitoxide/commit/47860b7e2769260cfb8522ae455c491605093423))
    - Refactor ([`831397c`](https://github.com/GitoxideLabs/gitoxide/commit/831397c99fee9f2d6758124d993386cca5534f7b))
    - Allow resolution of in-pack ref-deltas ([`84ade1d`](https://github.com/GitoxideLabs/gitoxide/commit/84ade1d23060f10bf6c8529f8f693d06660b4f4e))
    - Refactor ([`38426a1`](https://github.com/GitoxideLabs/gitoxide/commit/38426a171844014201282a441ebfc7d1f4cfff94))
    - Test to reproduce ref-delta forward references and the issue it poses for index traversal ([`7db7195`](https://github.com/GitoxideLabs/gitoxide/commit/7db7195953954ded32a410e8d11f07f4c5b61687))
    - Very rough version of repository verification ([`80a4a7a`](https://github.com/GitoxideLabs/gitoxide/commit/80a4a7add688d16376b9bf2ed7f1c7f655b7c912))
    - Refactor ([`6c06659`](https://github.com/GitoxideLabs/gitoxide/commit/6c066597f310b1bd5eb5611c1147b48846bc0ac0))
    - Improve method signatures of `cache::Tree::*` ([`a79a7fb`](https://github.com/GitoxideLabs/gitoxide/commit/a79a7fb638b45df88af0d0d5fc9ada6d824bc328))
    - Cleanup and unify `verify_integrity()` method signature ([`91d0476`](https://github.com/GitoxideLabs/gitoxide/commit/91d047658b114f372735116c9d8e6962a3873137))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - Fix docs, again ([`7b2ab26`](https://github.com/GitoxideLabs/gitoxide/commit/7b2ab263b9dbb2ad33a4dddfe82f4cd7f3187271))
    - Fix build ([`e3977fe`](https://github.com/GitoxideLabs/gitoxide/commit/e3977fe033550bfd3297cdd674934e40476aa38b))
    - Use InOrderIter from git-features ([`7721b5f`](https://github.com/GitoxideLabs/gitoxide/commit/7721b5fc7cba86d785e0936fdfab2ea41163219f))
    - Basic IEOT parsing ([`35bdee4`](https://github.com/GitoxideLabs/gitoxide/commit/35bdee4bf77787bcbe6c3dd715a677e2e46a8ad1))
    - Assure we are right about the leb64 buffer needed for a 64 bit int ([`7558844`](https://github.com/GitoxideLabs/gitoxide/commit/7558844b40b6c9af5038fea6b8a4e81583c46bde))
    - Adapt to changes in git-features: use var-int decoding from there ([`52e3c6f`](https://github.com/GitoxideLabs/gitoxide/commit/52e3c6f6f4cd1bf677c9189fb59db16173954669))
    - Remove byteorder from git-pack ([`4122306`](https://github.com/GitoxideLabs/gitoxide/commit/41223061a2b919fd190066315b419ea17cabfde3))
    - Git-pack uses `memmap2` instead of `filebuffer` ([`d9011c7`](https://github.com/GitoxideLabs/gitoxide/commit/d9011c71048ff34201917b0693586290c23b3ddf))
 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Restrict signature changes to 'Ancestores::sorting()` ([`d71bd9d`](https://github.com/GitoxideLabs/gitoxide/commit/d71bd9ded1e5e5a61a27be3d55f4b85ee4049bcf))
    - Adjust to changes in git-traverse ([`8240622`](https://github.com/GitoxideLabs/gitoxide/commit/824062215865e6ec12afeb2d51b3c63f15291244))
    - `index::File::lookup_prefix(…)` ([`503b1a1`](https://github.com/GitoxideLabs/gitoxide/commit/503b1a1f8d4f39b44c166209d7a8ba8d74137859))
    - Support MSRV ([`d09fd9b`](https://github.com/GitoxideLabs/gitoxide/commit/d09fd9b37557f2dc199e8a4651c56b3b63423136))
    - Add documentation for lookup_prefix along with missing test ([`927b2ac`](https://github.com/GitoxideLabs/gitoxide/commit/927b2ace875cdda63ce312eb7ad5329f2159608d))
    - Lookup_prefix() seems to work now ([`b558f11`](https://github.com/GitoxideLabs/gitoxide/commit/b558f111520381e25a9500d3b2401fdd337db6f6))
    - A stab at implementing lookup_prefix - to no avail ([`69cb6d1`](https://github.com/GitoxideLabs/gitoxide/commit/69cb6d1dd6b8df74fee1ead1ce15bcf0b51d7232))
    - Refactor ([`cff6f9f`](https://github.com/GitoxideLabs/gitoxide/commit/cff6f9fc90e58c409e367912d0b38860fae9a205))
    - Refactor ([`5bc548e`](https://github.com/GitoxideLabs/gitoxide/commit/5bc548ed500045491012ab0a93bcbe13e78b0dc8))
    - Prefix now validates all constraints and errors on violation ([`75efa79`](https://github.com/GitoxideLabs/gitoxide/commit/75efa79f62efc29b343d2d2f53eaf001eef176df))
    - Git-hash::Prefix::from_id() ([`cb83bee`](https://github.com/GitoxideLabs/gitoxide/commit/cb83beedd1aa389f6774e2296f79273e8c8f14f4))
    - Sketch for abbreviated method lookup ([`467453a`](https://github.com/GitoxideLabs/gitoxide/commit/467453a7e625a3bc8e3e381ce50f24f1be8ba605))
    - Use hash_hasher based hash state for better keys/less collisions ([`814de07`](https://github.com/GitoxideLabs/gitoxide/commit/814de079f4226f42efa49ad334a348bce67184e4))
    - Upgrade parking_lot and cargo_toml ([`f95c1a0`](https://github.com/GitoxideLabs/gitoxide/commit/f95c1a0d9c19bcc6feb9b8739a09d86f9970a0e0))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
    - Adapt to changes in git-path ([`cc2d810`](https://github.com/GitoxideLabs/gitoxide/commit/cc2d81012d107da7a61bf4de5b28342dea5083b7))
    - Use `git-path` crate instead of `git_features::path` ([`47e607d`](https://github.com/GitoxideLabs/gitoxide/commit/47e607dc256a43a3411406c645eb7ff04239dd3a))
    - Salvage an alternative parallelization approach which might be good for index-creation ([`7e76796`](https://github.com/GitoxideLabs/gitoxide/commit/7e76796d5c2956961bd998286bec05fca1ba8fc4))
    - Refactor ([`f86eacc`](https://github.com/GitoxideLabs/gitoxide/commit/f86eacc5cfaf6d88ead4f8dbd65989d32674c213))
    - Switch index checkout to chunk-based operation ([`e5f6943`](https://github.com/GitoxideLabs/gitoxide/commit/e5f69433e4a6cc7866b666e0baccfa32efb92a7f))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - In-manifest and in-lib documentation of feature toggles ([`1620830`](https://github.com/GitoxideLabs/gitoxide/commit/16208306ab49ade30d8ffd6b067ebd8eefd84cd4))
    - Document all features related to serde1 ([`72b97f2`](https://github.com/GitoxideLabs/gitoxide/commit/72b97f2ae4dc7642b160f183c6d5df4502dc186f))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Adapt to changes in git_features::path to deal with Result ([`bba4c68`](https://github.com/GitoxideLabs/gitoxide/commit/bba4c680c627a418efbd25f14bd168df19b8dedd))
 * **[#333](https://github.com/GitoxideLabs/gitoxide/issues/333)**
    - Use git_features::path everywhere where there is a path conversion ([`2e1437c`](https://github.com/GitoxideLabs/gitoxide/commit/2e1437cb0b5dc77f2317881767f71eaf9b009ebf))
    - Gitoxide-core without os-str-bytes ([`909aa14`](https://github.com/GitoxideLabs/gitoxide/commit/909aa1402c82c3128052023613a297b213716e3d))
    - Remove os_str_bytes from git-pack ([`86f6e50`](https://github.com/GitoxideLabs/gitoxide/commit/86f6e5054ea11b7aeb9c85321913de090f71e3a1))
 * **[#364](https://github.com/GitoxideLabs/gitoxide/issues/364)**
    - Add some precaution to avoid strange interactions with packs ([`b052a9a`](https://github.com/GitoxideLabs/gitoxide/commit/b052a9a3e9127fd9a4029594ea9de6e436db03c6))
    - Fix build ([`9c8e449`](https://github.com/GitoxideLabs/gitoxide/commit/9c8e449e928b3190e5845606f79b12c529dede55))
 * **[#384](https://github.com/GitoxideLabs/gitoxide/issues/384)**
    - Prevent line-ending conversions for shell scripts on windows ([`96bb4d4`](https://github.com/GitoxideLabs/gitoxide/commit/96bb4d460db420e18dfd0f925109c740e971820d))
    - No need to isolate archives by crate name ([`19d46f3`](https://github.com/GitoxideLabs/gitoxide/commit/19d46f35440419b9911b6e2bca2cfc975865dce9))
    - Add archive files via git-lfs ([`7202a1c`](https://github.com/GitoxideLabs/gitoxide/commit/7202a1c4734ad904c026ee3e4e2143c0461d51a2))
    - Auto-set commit.gpgsign=false when executing git ([`c23feb6`](https://github.com/GitoxideLabs/gitoxide/commit/c23feb64ad157180cfba8a11c882b829733ea8f6))
 * **[#422](https://github.com/GitoxideLabs/gitoxide/issues/422)**
    - Prepare changelog ([`de2d587`](https://github.com/GitoxideLabs/gitoxide/commit/de2d5874b8d75c53165a9fc3ed35e2b37142bf52))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - Make fmt ([`4b320e7`](https://github.com/GitoxideLabs/gitoxide/commit/4b320e773368ac5e8c38dd8a779ef3d6d2d024ec))
    - Refactor ([`c28404b`](https://github.com/GitoxideLabs/gitoxide/commit/c28404b3656ba6714db6898ce8151c0bcf4448e0))
    - Assure index ambiguous object range can represent 'none found' ([`5ffe54f`](https://github.com/GitoxideLabs/gitoxide/commit/5ffe54ff88f026139474658fb470742751126119))
    - Avoid allocating index entries in case of ambiguity by using a range ([`4db4754`](https://github.com/GitoxideLabs/gitoxide/commit/4db47547fa405542efd38b475e3e430548b9d160))
    - Fix build ([`8a63076`](https://github.com/GitoxideLabs/gitoxide/commit/8a6307617580f72457d3f008e4fe40ae7dcfd360))
    - More efficient and clear addition of entries to candidates ([`a76a0e1`](https://github.com/GitoxideLabs/gitoxide/commit/a76a0e161ec289d289e5683e541cff45d393756d))
    - Properly order ambiguous entries, ascending ([`0340896`](https://github.com/GitoxideLabs/gitoxide/commit/0340896b49fe4f40a9dba1c947ec9a2050d8f9b8))
    - Provide optional `candidates` for ambigious entries during `lookup_prefix()` ([`95210cb`](https://github.com/GitoxideLabs/gitoxide/commit/95210cb2ba85f75148b4ef48ccea9d9f8a0a0114))
    - Refactor ([`a2f9fa4`](https://github.com/GitoxideLabs/gitoxide/commit/a2f9fa431a820093cb34158e71cb30dc21db1151))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Less noisy way of writing trait bounds ([`b593806`](https://github.com/GitoxideLabs/gitoxide/commit/b593806ca3571d680801130ad528f266d3eab83e))
    - Upgrade to `prodash` v21 ([`a0655dc`](https://github.com/GitoxideLabs/gitoxide/commit/a0655dc7bc5dff388bc69a648e7f16b44fd1abd9))
    - Prefix created pack files with `pack-` ([`e489b10`](https://github.com/GitoxideLabs/gitoxide/commit/e489b10878c85289e0a9869804abee2418de6989))
    - Increase pack-receive performance using a BufWriter ([`a745512`](https://github.com/GitoxideLabs/gitoxide/commit/a745512185fb0a46e35daaa6d28829aec05edb55))
    - `index::write::Outcome::index_kind` -> `::index_version`. ([`b46347f`](https://github.com/GitoxideLabs/gitoxide/commit/b46347fd3d50886eeca500e31e1e12b354711309))
    - Add test to show that empty packs won't be written as expected behaviour. ([`72ce7fd`](https://github.com/GitoxideLabs/gitoxide/commit/72ce7fdced10b8359e74daea3bb35ab73b29e7c0))
    - `bundle::write::Error` is now publicly available ([`0b6ed60`](https://github.com/GitoxideLabs/gitoxide/commit/0b6ed60f842f0a36f61f187651080540a358758e))
    - `bundle::write::Options::index_kind` -> `::index_version`. ([`591afd5`](https://github.com/GitoxideLabs/gitoxide/commit/591afd56d9862a6348ef8b3af61798004b36aa19))
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#67](https://github.com/GitoxideLabs/gitoxide/issues/67)**
    - Use an even faster way of counting ([`3877920`](https://github.com/GitoxideLabs/gitoxide/commit/387792085542ebc8277ac0dcaf9e3dc3b522a69a))
    - Avoid the dashmap being cloned for each thread ([`6d3f52d`](https://github.com/GitoxideLabs/gitoxide/commit/6d3f52dc13d7243a6bce6dab89a985114a75d94b))
    - Properly count total objects during pack creation ([`bcb3d37`](https://github.com/GitoxideLabs/gitoxide/commit/bcb3d37a900a40fd70b7be7bad8b2d5db292d2af))
    - ObjectID specific hashers, using the fact that object ids are hashes ([`f9232ac`](https://github.com/GitoxideLabs/gitoxide/commit/f9232acf8e52f8cd95520d122469e136eb07b39f))
    - Use a custom hasher for 'seen' objects hashset… ([`70179e2`](https://github.com/GitoxideLabs/gitoxide/commit/70179e2cf8d15ba4e1cf8e94a9915bf5b02cf755))
    - Don't include submodules in count… ([`faf6f81`](https://github.com/GitoxideLabs/gitoxide/commit/faf6f813927720c5adf62102f9ce46606ff2617c))
    - Control pack and object cache size in megabytes ([`60c9fad`](https://github.com/GitoxideLabs/gitoxide/commit/60c9fad8002b4e3f6b9607bba6361871752f4d3d))
    - Use 'cache::Object' trait where it matters ([`71c628d`](https://github.com/GitoxideLabs/gitoxide/commit/71c628d46088ab455b54eb2330d24dcff96c911d))
    - Split data::output::count::objects into files ([`8fe4612`](https://github.com/GitoxideLabs/gitoxide/commit/8fe461281842b58aa11437445637c6e587bedd63))
    - Cache::Object trait for caching and retrieving whole objects ([`50cf610`](https://github.com/GitoxideLabs/gitoxide/commit/50cf610e8939812c3d2268c48835e2dac67d0c31))
    - Object cache size is configurable ([`5a8c2da`](https://github.com/GitoxideLabs/gitoxide/commit/5a8c2da6cb1e2accf7cfdccc16bc3a1d0b2a7dbc))
    - Dynamically sized full-object speeds up diff-based object counting… ([`d6c44e6`](https://github.com/GitoxideLabs/gitoxide/commit/d6c44e6ab8f436020d4fb235e423b018fd1e7a9f))
    - Count ref-deltas in thin packs as well ([`80c6994`](https://github.com/GitoxideLabs/gitoxide/commit/80c6994149d19917c25e36e1bdf0dc8c9678365e))
    - Add '--thin' flag to pack-create and pass it on ([`2664d73`](https://github.com/GitoxideLabs/gitoxide/commit/2664d73f531a4b1f4bc784c1fe3a991711c86475))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
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
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
    - Rename `git-pack` to `gix-pack` ([`134d70d`](https://github.com/GitoxideLabs/gitoxide/commit/134d70dbb7fe9c7fcd4201a7bb028b22bca39a30))
    - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
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
    - Merge branch 'git-pack-wasm' ([`4bc19d1`](https://github.com/GitoxideLabs/gitoxide/commit/4bc19d104233a3e3d3d2768c0e9b9ad027cc34c0))
    - Add `wasm` feature toggle to let parts of `git-pack` build on wasm32. ([`6c4c196`](https://github.com/GitoxideLabs/gitoxide/commit/6c4c196c9bc6c2171dc4dc58b69bd5ef53226e29))
    - Add `wasm` feature toggle to allow compilation to wasm32-unknown-unknown ([`f0e40ec`](https://github.com/GitoxideLabs/gitoxide/commit/f0e40ecddaf1211f76ed60ef30cf03dcfd53a7ab))
    - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
    - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
    - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
    - Prepare changelogs prior to release ([`7c846d2`](https://github.com/GitoxideLabs/gitoxide/commit/7c846d2102dc767366771925212712ef8cc9bf07))
    - Merge branch 'Lioness100/main' ([`1e544e8`](https://github.com/GitoxideLabs/gitoxide/commit/1e544e82455bf9ecb5e3c2146280eaf7ecd81f16))
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
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Merge branch 'odb-iteration' ([`693a469`](https://github.com/GitoxideLabs/gitoxide/commit/693a46977e2b57b93ee921320e008c8ad1beb81b))
    - Assure deltas are counted correctly, even if the base is out of pack. ([`ddaf47f`](https://github.com/GitoxideLabs/gitoxide/commit/ddaf47f023970e8acfb98e8874da22f2604a92d9))
    - Merge branch 'read-header' ([`3d01252`](https://github.com/GitoxideLabs/gitoxide/commit/3d0125271ec7bd606734bd74757a7e31a18c7ce5))
    - Tweaks to immprove header decode performance ([`04df898`](https://github.com/GitoxideLabs/gitoxide/commit/04df89835395d43e92ce3f0fee922f1799687aa4))
    - Move `data::ResolveBase` into `data::decode_entry::`; unify `decode_entry|decode_header::Error` into `decode::Error`. ([`37f3a67`](https://github.com/GitoxideLabs/gitoxide/commit/37f3a675d7fa931f5d3f38e91df74ec0b517422b))
    - Add `data::File::resolve_header()` to obtain object information without decompressing it. ([`a85dcdd`](https://github.com/GitoxideLabs/gitoxide/commit/a85dcddba29a453bbea87ac865b8aedc97f41aed))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - Thanks clippy ([`f1160fb`](https://github.com/GitoxideLabs/gitoxide/commit/f1160fb42acf59b37cbeda546a7079af3c9bc050))
    - Adjust to changes in `git-testtools` ([`4eb842c`](https://github.com/GitoxideLabs/gitoxide/commit/4eb842c7150b980e1c2637217e1f9657a671cea7))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Upgrade clru, remove it from git-repository dependencies (unused) ([`7e7547d`](https://github.com/GitoxideLabs/gitoxide/commit/7e7547d995afc16192a1ee08add5a87560197fc9))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Merge branch 'optimize_hashtables' ([`95ad56c`](https://github.com/GitoxideLabs/gitoxide/commit/95ad56c11489bc46d6eb2b2f48cf0bf01e954c58))
    - Use newly added git-hashtable ([`50cb436`](https://github.com/GitoxideLabs/gitoxide/commit/50cb4362010e1a5799fe782df36ac5fcdb48dd8a))
    - Switch to custom Hasher implementation ([`269d59e`](https://github.com/GitoxideLabs/gitoxide/commit/269d59e0bee1f072096667b143800a0d85b18403))
    - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
    - Upgrade to `prodash 21.1` and add `Ids` to all progress instances. ([`c8835c6`](https://github.com/GitoxideLabs/gitoxide/commit/c8835c6edae784c9ffcb69a674c0a6545dbb2af3))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'main' into http-config ([`7c5b37d`](https://github.com/GitoxideLabs/gitoxide/commit/7c5b37d28e98f59a6847368a0d0166d2dbb4acc1))
    - Release git-diff v0.22.0, git-index v0.7.1, git-pack v0.26.0, git-odb v0.36.0, git-transport v0.21.2, git-repository v0.27.0, safety bump 6 crates ([`f0cab31`](https://github.com/GitoxideLabs/gitoxide/commit/f0cab317bb0c2799fa80d16f3ae1b89d6aee4284))
    - Prepare changelogs prior to release ([`f5f3a9e`](https://github.com/GitoxideLabs/gitoxide/commit/f5f3a9edd038a89c8c6c4da02054e5439bcc0071))
    - Fix spelling where -> were ([`57355c7`](https://github.com/GitoxideLabs/gitoxide/commit/57355c7b345d864aaf80fdbd9deec50ca78d94fd))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Merge branch 'main' into gix-clone ([`3b48317`](https://github.com/GitoxideLabs/gitoxide/commit/3b48317d6a9f41765d4f2a9f0a49c31afcdb68b6))
    - Thanks clippy ([`93e7691`](https://github.com/GitoxideLabs/gitoxide/commit/93e7691be421e40cc72e3e2e0506584a2fbd4857))
    - `Bundle::write…` also produces a `.keep` file ([`450257e`](https://github.com/GitoxideLabs/gitoxide/commit/450257eb596465352fb363a5c8c514f544e7f9ac))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Merge branch 'fix-smart-release' ([`aa80b60`](https://github.com/GitoxideLabs/gitoxide/commit/aa80b606e5570f327660cca42ea81581a6e9d5e3))
    - Make fmt ([`7b9c065`](https://github.com/GitoxideLabs/gitoxide/commit/7b9c06547b75929e3e5bf4240f43c7e9bc7d54e0))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Merge branch 'main' into clone ([`acb0738`](https://github.com/GitoxideLabs/gitoxide/commit/acb07382a9306d6962bea60e8977d83d021743f4))
    - Merge branch 'delta-tree-parallelization' ([`cca2ad5`](https://github.com/GitoxideLabs/gitoxide/commit/cca2ad5ee9483d7da968658e0a4d610dbc4ad4d6))
    - Don't enforce Send bounds in serial version of `in_parallel_with_slice()` ([`dda661e`](https://github.com/GitoxideLabs/gitoxide/commit/dda661e1b7cc0ace6cd9504233f20980e1e52387))
    - Thanks clippy ([`583f2ca`](https://github.com/GitoxideLabs/gitoxide/commit/583f2cacbdaa79149b3741c1f08cabcc73a1a03f))
    - More efficient distribution of tasks to threads during pack resolution. ([`6836cb1`](https://github.com/GitoxideLabs/gitoxide/commit/6836cb148cbadf919bdac2e0e77bdec34caa9307))
    - Refactor ([`0e2ca47`](https://github.com/GitoxideLabs/gitoxide/commit/0e2ca47fab99fec72cf8c8a327f5c3627a916add))
    - Try to use counters directly in delta-tree resolution ([`c6ded26`](https://github.com/GitoxideLabs/gitoxide/commit/c6ded26aa2513fefda0c19784a70ce5b046910f1))
    - Reduce memory usage for pack resolution by 18%… ([`ec58bef`](https://github.com/GitoxideLabs/gitoxide/commit/ec58bef84d620360dc52e34e173ace1310f74c85))
    - Merge branch 'reduce-pack-resolve-mem-consumption' ([`a8ced57`](https://github.com/GitoxideLabs/gitoxide/commit/a8ced575ef935b9a2078d6f2b15aff746a862317))
    - Minor refactor after trying more memory foot-print related changes ([`5852334`](https://github.com/GitoxideLabs/gitoxide/commit/58523341d1203ff55144a981d7dd3b276bd47b31))
    - Simplify size-related tests ([`fb2b2a8`](https://github.com/GitoxideLabs/gitoxide/commit/fb2b2a88193247d55f74c3e39cd8a12f3862b6d6))
    - Cut the size of bytes needed to store children in half for a lower footprint ([`38c96f2`](https://github.com/GitoxideLabs/gitoxide/commit/38c96f2ab1fc2c3a91958e2b4ab213574ec9e8f0))
    - Merge branch 'crates-index-diff-fixes' ([`b09bad7`](https://github.com/GitoxideLabs/gitoxide/commit/b09bad7b5702838c1119d052b322d90c9b5968bb))
    - Thanks clippy ([`851bc79`](https://github.com/GitoxideLabs/gitoxide/commit/851bc798a76160701387d25119b101aa8014c93f))
    - Allow verification of empty packs and indices. ([`49d168a`](https://github.com/GitoxideLabs/gitoxide/commit/49d168a8859c6b5a9e7ef58cd836093212c2c7ad))
    - Refactor ([`38a7452`](https://github.com/GitoxideLabs/gitoxide/commit/38a74522b5c1ca5c255b8b132c25b90fc7fa85f9))
    - Reading and writing empty packs is explicitly allowed. ([`5a75afe`](https://github.com/GitoxideLabs/gitoxide/commit/5a75afe0467e4a84323ea10172eed835cc7fae4c))
    - Make fmt ([`53acf25`](https://github.com/GitoxideLabs/gitoxide/commit/53acf2565743eff7cead7a42011107b2fc8d7e0e))
    - Merge branch 'fetch-pack' ([`f47c891`](https://github.com/GitoxideLabs/gitoxide/commit/f47c89129732bcb06fe76a4696fe38ab1151fb0c))
    - Merge branch 'fetch-pack' ([`3c49400`](https://github.com/GitoxideLabs/gitoxide/commit/3c49400809c7c2120f4ce704c19a0421545b5acd))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0 ([`f5c36d8`](https://github.com/GitoxideLabs/gitoxide/commit/f5c36d85755d1f0f503b77d9a565fad6aecf6728))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
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
    - Adjust to changes in `git-diff` ([`54954ee`](https://github.com/GitoxideLabs/gitoxide/commit/54954ee5022a900f9f97baec63e9a073eca514e9))
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
    - Assure document-features are available in all 'usable' and 'early' crates ([`238581c`](https://github.com/GitoxideLabs/gitoxide/commit/238581cc46c7288691eed37dc7de5069e3d86721))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Update changelogs prior to release ([`bb424f5`](https://github.com/GitoxideLabs/gitoxide/commit/bb424f51068b8a8e762696890a55ab48900ab980))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Release git-path v0.1.2, git-sec v0.1.1, git-config v0.4.0, git-discover v0.1.1, git-pack v0.19.1, git-repository v0.18.0, cargo-smart-release v0.10.0, safety bump 2 crates ([`ceb6dff`](https://github.com/GitoxideLabs/gitoxide/commit/ceb6dff13362a2b4318a551893217c1d11643b9f))
    - Thanks clippy ([`409a95b`](https://github.com/GitoxideLabs/gitoxide/commit/409a95b6505db8568bfea24bc62c92640a5c3cbf))
    - Merge branch 'davidkna-discover-ceiling' ([`66944ba`](https://github.com/GitoxideLabs/gitoxide/commit/66944ba986114ece2d3b31440c721d0e84b4f267))
    - Thanks clippy ([`4979d20`](https://github.com/GitoxideLabs/gitoxide/commit/4979d2071cbd1a98f3d81aacd60dd99f07d3f746))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0 ([`349c590`](https://github.com/GitoxideLabs/gitoxide/commit/349c5904b0dac350838a896759d51576b66880a7))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Merge branch 'refs-and-worktrees' ([`8131227`](https://github.com/GitoxideLabs/gitoxide/commit/8131227ddff6f36919b6a0f7b33792ebde0f8ae9))
    - Cleanup ([`199583a`](https://github.com/GitoxideLabs/gitoxide/commit/199583abbec1ff58204c3cf90dd8be88730fc112))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Merge branch 'main' into repo-status ([`0eb2372`](https://github.com/GitoxideLabs/gitoxide/commit/0eb23721dca78f6e6bf864c5c3a3e44df8b419f0))
    - Merge branch 'test-archive-support' ([`350df01`](https://github.com/GitoxideLabs/gitoxide/commit/350df01042d6ca8b93f8737fa101e69b50535a0f))
    - Fix clippy - many false positives this time ([`045e6fa`](https://github.com/GitoxideLabs/gitoxide/commit/045e6fae17077555c3e115992905c8046f2c5d0b))
    - Fix clippy - many false positives this time ([`099bd5b`](https://github.com/GitoxideLabs/gitoxide/commit/099bd5b86fb80b26a73863b80ad60a0394458b6d))
    - Release git-config v0.2.1, git-diff v0.15.0, git-traverse v0.14.0, git-pack v0.18.0, git-odb v0.28.0, git-ref v0.12.1, git-revision v0.1.0, git-repository v0.16.0, gitoxide-core v0.14.0, gitoxide v0.12.0, safety bump 6 crates ([`b612021`](https://github.com/GitoxideLabs/gitoxide/commit/b612021683ba709b693bd48aef3e2e3c2f5b9ead))
    - Remove deprecated compound and linked object databases ([`8c5ae77`](https://github.com/GitoxideLabs/gitoxide/commit/8c5ae77f06a64c57df9a9ad1190266896a223dbe))
    - Release git-diff v0.14.0, git-bitmap v0.1.0, git-index v0.2.0, git-tempfile v2.0.1, git-lock v2.0.0, git-mailmap v0.1.0, git-traverse v0.13.0, git-pack v0.17.0, git-quote v0.2.0, git-odb v0.27.0, git-packetline v0.12.4, git-url v0.4.0, git-transport v0.16.0, git-protocol v0.15.0, git-ref v0.12.0, git-worktree v0.1.0, git-repository v0.15.0, cargo-smart-release v0.9.0, safety bump 5 crates ([`e58dc30`](https://github.com/GitoxideLabs/gitoxide/commit/e58dc3084cf17a9f618ae3a6554a7323e44428bf))
    - Merge branch 'for-onefetch' ([`8e5cb65`](https://github.com/GitoxideLabs/gitoxide/commit/8e5cb65da75036a13ed469334e7ae6c527d9fff6))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Make fmt ([`7cf3545`](https://github.com/GitoxideLabs/gitoxide/commit/7cf354509b545f7e7c99e159b5989ddfbe86273d))
    - Remove unnecessary unsafe by using `chunks_mut()` ([`9b9f10a`](https://github.com/GitoxideLabs/gitoxide/commit/9b9f10ad862b5e097c836c51df1eb98607df5ae1))
    - Merge branch 'short-id' ([`5849d5b`](https://github.com/GitoxideLabs/gitoxide/commit/5849d5b326b83f98a16cf1d956c720c7f0fd4445))
    - Thanks clippy ([`48be1ee`](https://github.com/GitoxideLabs/gitoxide/commit/48be1ee666a88f1416896c5e8073d4d86dae7b8c))
    - Merge branch 'svetli-n-path_value' ([`e8383ca`](https://github.com/GitoxideLabs/gitoxide/commit/e8383caf6db211beb57d70019fe4ad13ce9066ee))
    - Release git-tempfile v2.0.0, safety bump 6 crates ([`90b1c42`](https://github.com/GitoxideLabs/gitoxide/commit/90b1c42d5487904a9f329362d185b035d0ddb975))
    - Upgrade document-features ([`c35e62e`](https://github.com/GitoxideLabs/gitoxide/commit/c35e62e0da9ac1f7dcb863f5f9c69108c728d32e))
    - Merge branch 'unify-path-encoding' ([`566ff8a`](https://github.com/GitoxideLabs/gitoxide/commit/566ff8a3597b889899d41ca15e5b9af7e05f1a4b))
    - Upgrade dashmap to 5.1.0 (with security fix) ([`2520945`](https://github.com/GitoxideLabs/gitoxide/commit/25209454d3f7e27e12e8ddca92e43b1ff01d58aa))
    - Release git-pack v0.16.1 ([`d4a8f9f`](https://github.com/GitoxideLabs/gitoxide/commit/d4a8f9f73bb829bcc83fa68b6b5a7495fcba6b19))
    - Release git-object v0.17.1, git-pack v0.16.1 ([`e959af8`](https://github.com/GitoxideLabs/gitoxide/commit/e959af83fa92e8ed87edae6e2d1c6a797964c056))
    - Release git-hash v0.9.2, git-object v0.17.1, git-pack v0.16.1 ([`0db19b8`](https://github.com/GitoxideLabs/gitoxide/commit/0db19b8deaf11a4d4cbc03fa3ae40eea104bc302))
    - Update changelogs prior to git-pack release ([`b7e3a4a`](https://github.com/GitoxideLabs/gitoxide/commit/b7e3a4afdd6417a38aadad35c7f584617e7b47fa))
    - Downgrade dashmap to 4.0 to avoid unsoundness. ([`d9451e8`](https://github.com/GitoxideLabs/gitoxide/commit/d9451e8d7fc39c252042f9d2447061262c16ae7a))
    - Release git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`1b76119`](https://github.com/GitoxideLabs/gitoxide/commit/1b76119259b8168aeb99cbbec233f7ddaa2d7d2c))
    - Release git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`8f57c29`](https://github.com/GitoxideLabs/gitoxide/commit/8f57c297d7d6ed68cf51415ea7ede4bf9263326e))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Fix git-pack changelog to be stable ([`fd5b616`](https://github.com/GitoxideLabs/gitoxide/commit/fd5b616d6ce8f353bd96b2c4994af9ba9c878b3e))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Support Rust 1.52 ([`42e0487`](https://github.com/GitoxideLabs/gitoxide/commit/42e0487286c1f745837c0ce337ed7c9d86b14516))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Thanks clippy ([`d8925f5`](https://github.com/GitoxideLabs/gitoxide/commit/d8925f5bd7ac8ef2c98f0e57a1373e5ffba8ce23))
    - Thanks clippy ([`5a68d2f`](https://github.com/GitoxideLabs/gitoxide/commit/5a68d2feffc551ad5f07e90efb2307e966d2636b))
    - Thanks clippy ([`1e051dc`](https://github.com/GitoxideLabs/gitoxide/commit/1e051dc23fb298b0bfe3e9ffb85a95ecb9c0f93f))
    - Don't try to short-cut detection of large offsets when writing index files ([`ba92cc0`](https://github.com/GitoxideLabs/gitoxide/commit/ba92cc09ba41fe4c9a9097bfeb8d18016408fcdf))
    - Refactor ([`e7fbd9f`](https://github.com/GitoxideLabs/gitoxide/commit/e7fbd9f3700496ad7bb7e71226c4d25429f0ccd5))
    - Merge branch 'use-midx-in-store' ([`338521b`](https://github.com/GitoxideLabs/gitoxide/commit/338521b0443b9dc1007581de42ef6a950f6e0bbf))
    - Thanks clippy ([`533a532`](https://github.com/GitoxideLabs/gitoxide/commit/533a532c86bcf0dae27558e66b1a5cd2e52983df))
    - Thanks clippy ([`7bd3ad3`](https://github.com/GitoxideLabs/gitoxide/commit/7bd3ad3ab9f17eaf94490bea04a9b1297fa5fe64))
    - Release git-chunk v0.2.0, safety bump 4 crates ([`b792fab`](https://github.com/GitoxideLabs/gitoxide/commit/b792fabf9f5f93ab906ac5a5bb3e4f01c179290a))
    - Thanks clippy ([`35cf46f`](https://github.com/GitoxideLabs/gitoxide/commit/35cf46f87ecc42cf033ca001acf1b5918b3fea1b))
    - Refactor ([`0032223`](https://github.com/GitoxideLabs/gitoxide/commit/003222365bb2f8ce7d915240db6ff84ccbca6db4))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`4ca9e07`](https://github.com/GitoxideLabs/gitoxide/commit/4ca9e07c7ac062d48d64ad7b516274e32dbc51c6))
    - Make fmt ([`066f3ff`](https://github.com/GitoxideLabs/gitoxide/commit/066f3ffb8740f242c1b03e680c3c5c1a0e4c36c3))
    - Thanks clippy ([`7dd2313`](https://github.com/GitoxideLabs/gitoxide/commit/7dd2313d980fe7c058319ae66d313b3097e3ae5f))
    - Release git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0 ([`d3f9227`](https://github.com/GitoxideLabs/gitoxide/commit/d3f922781a81e8fbb81aa47afdbe9afeb06d666b))
    - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
    - Adjust changelogs prior to release ([`ec38950`](https://github.com/GitoxideLabs/gitoxide/commit/ec3895005d141abe79764eaff7c0f04153e38d73))
    - Merge branch 'pack-consistency' ([`5982406`](https://github.com/GitoxideLabs/gitoxide/commit/5982406b4e1b26fd383d9ec21a3cf652ec8ab25f))
    - Merge branch 'git-loose-objects' of https://github.com/xmo-odoo/gitoxide into xmo-odoo-git-loose-objects ([`ee737cd`](https://github.com/GitoxideLabs/gitoxide/commit/ee737cd237ad70bf9f2c5e0d3e4557909e495bca))
    - Release git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0 ([`f606fa9`](https://github.com/GitoxideLabs/gitoxide/commit/f606fa9a0ca338534252df8921cd5e9d3875bf94))
    - Adjusting changelogs prior to release of git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0, safety bump 5 crates ([`39b40c8`](https://github.com/GitoxideLabs/gitoxide/commit/39b40c8c3691029cc146b893fa0d8d25d56d0819))
    - Adjust changelogs prior to git-pack release ([`ac8015d`](https://github.com/GitoxideLabs/gitoxide/commit/ac8015de710142c2bedd0e4188e872e0cf1ceccc))
    - Move "loose object header" ser/de to git-object ([`3d1565a`](https://github.com/GitoxideLabs/gitoxide/commit/3d1565acfc336baf6487edccefd72d0226141a08))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Make fmt, but now it picked up some parts that usually don't get altered… ([`01f7b72`](https://github.com/GitoxideLabs/gitoxide/commit/01f7b729337bd2c99498321c479a9a13b1858e3e))
    - Update changelogs just for fun ([`21541b3`](https://github.com/GitoxideLabs/gitoxide/commit/21541b3301de1e053fc0e84373be60d2162fbaae))
    - Merge branch 'changelog-generation' ([`bf0106e`](https://github.com/GitoxideLabs/gitoxide/commit/bf0106ea21734d4e59d190b424c22743c22da966))
    - Bump git-traverse v0.9.0, safety bump 8 crates ([`d39fabb`](https://github.com/GitoxideLabs/gitoxide/commit/d39fabb8757369aa19452a457f610fe21dc13a14))
    - Bump git-pack v0.11.0 ([`5ae6ff5`](https://github.com/GitoxideLabs/gitoxide/commit/5ae6ff52cd2cd1ccd1e26bb987c154eb19603696))
    - Bump git-object v0.14.0 ([`d4fc81f`](https://github.com/GitoxideLabs/gitoxide/commit/d4fc81f6390443f8c8561d91ac27ea4a6318fb62))
    - [repository #164] generic write_object() ([`c569f83`](https://github.com/GitoxideLabs/gitoxide/commit/c569f83363489dde03c8b9cd01e75d35f5e04dbc))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - Bump git-pack v0.10.0 ([`e5e3c80`](https://github.com/GitoxideLabs/gitoxide/commit/e5e3c8024e1c2e5e90cee83abbdae41d58eee156))
    - [repository #190] first shot at ancestor iteration… ([`85f1a48`](https://github.com/GitoxideLabs/gitoxide/commit/85f1a48ea39f3b224e8d0ba3728dd75e03a6edc3))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - [repository #185] refactor ([`7604935`](https://github.com/GitoxideLabs/gitoxide/commit/7604935b12eacb26a98bedc5f77636b5583629a5))
    - [repository #174] adjust various changelogs ([`081faf5`](https://github.com/GitoxideLabs/gitoxide/commit/081faf5c3a21b34b7068b44d8206fb5770c392f5))
    - [pack #179] refactor ([`76e66d1`](https://github.com/GitoxideLabs/gitoxide/commit/76e66d1b9d24bb25a9f681d9612e52c8ccd60e2c))
    - [pack #179] move Tree traversal cache private ([`34e45d7`](https://github.com/GitoxideLabs/gitoxide/commit/34e45d745cb8756831c56dc441695a25cd0069a9))
    - [pack #179] refactor ([`5a3677d`](https://github.com/GitoxideLabs/gitoxide/commit/5a3677dd3f3dcab26a3d9270b6184fd0fe18c54e))
    - [pack #179] refactor bundle ([`420dca2`](https://github.com/GitoxideLabs/gitoxide/commit/420dca29bccca6e7d759880d8342f23b33eead0d))
    - [pack #179] fix docs ([`7ad7a44`](https://github.com/GitoxideLabs/gitoxide/commit/7ad7a4428d0e38f2ff776f7efab6996505d2bba2))
    - [pack #179] refactor ([`ab6554b`](https://github.com/GitoxideLabs/gitoxide/commit/ab6554b0cd5838f1ea4e82f6b5019798288076fa))
    - [pack #179] refactor ([`620d8a5`](https://github.com/GitoxideLabs/gitoxide/commit/620d8a54db5cd8367ec85c8b837cab710c509e3e))
    - [pack #179] add changelog ([`2102569`](https://github.com/GitoxideLabs/gitoxide/commit/210256932a338038adb55c5475d8f90560aa4c12))
    - Bump git-traverse v0.8.0 ([`54f3541`](https://github.com/GitoxideLabs/gitoxide/commit/54f3541f1448a8afa044d3958fa1be5b074e4445))
    - Bump git-diff v0.9.0 ([`2e2e798`](https://github.com/GitoxideLabs/gitoxide/commit/2e2e7983178b3af7e5684995de68ed5d020927ec))
    - [object #177] cleanup CommitRefIter imports and git_object::Error ([`058f68a`](https://github.com/GitoxideLabs/gitoxide/commit/058f68a9e1cd79fd5a2a1235da42358bc92ed255))
    - [object #177] dissolve 'immutable' module ([`70e11c2`](https://github.com/GitoxideLabs/gitoxide/commit/70e11c21b0637cd250f54381d5490e9976880ad9))
    - [object #177]  commit::RefIter -> CommitRefIter ([`e603306`](https://github.com/GitoxideLabs/gitoxide/commit/e603306e81f392af97aa5afd232653de56bf3ce9))
    - [object #177] migrate immutable::commit into crate::commit ([`45d3934`](https://github.com/GitoxideLabs/gitoxide/commit/45d393438eac2c7ecd47670922437dd0de4cd69b))
    - [object #177] tag::RefIter -> TagRefIter ([`28587c6`](https://github.com/GitoxideLabs/gitoxide/commit/28587c691eb74e5cb097afb2b63f9d9e2561c45d))
    - [object #177] into_mutable() -> into_owned() ([`7e701ce`](https://github.com/GitoxideLabs/gitoxide/commit/7e701ce49efe5d40327770a988aae88692d88219))
    - [object #177] fix docs ([`25d8e7b`](https://github.com/GitoxideLabs/gitoxide/commit/25d8e7b1862bd05489359b162a32c6ad45ecdf9a))
    - [object #177] move mutable objects to crate::* ([`c551c02`](https://github.com/GitoxideLabs/gitoxide/commit/c551c0236c64f3237cb9be7f35159f753d4b871f))
    - [object #177] migrate immutable::tree to crate::tree ([`fa5cd06`](https://github.com/GitoxideLabs/gitoxide/commit/fa5cd0648d5c855060ab2b75ee933851987c2dcf))
    - [object #177] fix docs ([`07be661`](https://github.com/GitoxideLabs/gitoxide/commit/07be6611d1742633815566443f71eef8b85ad5c0))
    - [object #177] move immutable::* to crate::*Ref, start `iter` adjustments ([`461dc53`](https://github.com/GitoxideLabs/gitoxide/commit/461dc53ba3bc07d55fdb4aad7570ba9176a8b360))
    - [object #177] rename immutable::* to immutable::*Ref ([`6deb012`](https://github.com/GitoxideLabs/gitoxide/commit/6deb01291fb382b7fb9206682e319afa81bacc05))
    - Release git-object v0.13.0 ([`708fc5a`](https://github.com/GitoxideLabs/gitoxide/commit/708fc5abd8af4dd7459f388c7092bf35915c6662))
    - [pack #172] A note about empty packs in Bundle writer ([`09a777f`](https://github.com/GitoxideLabs/gitoxide/commit/09a777f1da5e792c5eb4c8ff9e83504ad8d19c5c))
    - Merge pull request #172 from mellowagain/main ([`61aebbf`](https://github.com/GitoxideLabs/gitoxide/commit/61aebbfff02eb87e0e8c49438a093a21b1134baf))
    - [actor #173] fix docs ([`2d7956a`](https://github.com/GitoxideLabs/gitoxide/commit/2d7956a22511d73b767e443dac21b60e93f286dd))
    - [actor #173] rename immutable::Signature to SignatureRef! ([`96461ac`](https://github.com/GitoxideLabs/gitoxide/commit/96461ace776d6b351b313d4f2697f2d95b9e196e))
    - Release git-tempfile v1.0.0 ([`1238535`](https://github.com/GitoxideLabs/gitoxide/commit/123853539dc30ddea2d822ab177ee09b191bdf1b))
    - Merge branch 'main' into stability ([`11bae43`](https://github.com/GitoxideLabs/gitoxide/commit/11bae437e473fef6ed09c178d54ad11eee001b1d))
    - Cleanup imports ([`e669303`](https://github.com/GitoxideLabs/gitoxide/commit/e6693032f1391416fd704c21617051ddfb862a3a))
    - Allow creation of empty indices ([`d122fc7`](https://github.com/GitoxideLabs/gitoxide/commit/d122fc79cc9b9a52a2817bdd46d3215c10e61129))
    - [pack #170] there can only be one ([`dce4f97`](https://github.com/GitoxideLabs/gitoxide/commit/dce4f97a84aa6a73e31e7397501cfce27241c5b8))
    - [pack #170] clru allows for free lists, reducing allocation pressure... ([`4d820d2`](https://github.com/GitoxideLabs/gitoxide/commit/4d820d2f94dc3afc062bbd25e969c87410212c3a))
    - [pack #170] basic progress for resolution ([`ada0b96`](https://github.com/GitoxideLabs/gitoxide/commit/ada0b96e3707c06d7d6f7e4002907e12b45f7419))
    - [pack #170] Basic entry resolution without progress ([`7461f31`](https://github.com/GitoxideLabs/gitoxide/commit/7461f31f03d67ecc9fdf398adf3cb6d4eb365412))
    - [pack #170] first step towards resolving in multi-threaded mode… ([`f3c21f9`](https://github.com/GitoxideLabs/gitoxide/commit/f3c21f99594ab4080b8aa1ffed9ea8a33e18fabd))
    - [pack #170] Don't double-lookup trees during traversal… ([`7b06829`](https://github.com/GitoxideLabs/gitoxide/commit/7b068296fe5ca10af212d8fe2662940188b7359c))
    - Revert "[pack #67] Don't pre-fetch packed objects during counting" ([`811bb54`](https://github.com/GitoxideLabs/gitoxide/commit/811bb54991636f7e517087b62cf0c8c8cc2ad9e6))
    - [pack #67] Don't pre-fetch packed objects during counting ([`d08b673`](https://github.com/GitoxideLabs/gitoxide/commit/d08b6739d8e9294b795aba75e9c7f9f20645af2b))
    - Release git-pack v0.9.0 ([`7fbc961`](https://github.com/GitoxideLabs/gitoxide/commit/7fbc9617da97d4ba4bb3784f41d4163c0839c03c))
    - [pack #67] refactor ([`14717f6`](https://github.com/GitoxideLabs/gitoxide/commit/14717f6132672a5d271832a68de0b323b73abb2a))
    - [pack #67] Add cache debugging capabilities to git-features ([`8776c98`](https://github.com/GitoxideLabs/gitoxide/commit/8776c9834ac4622b3057f5db464a9817ed9acdb0))
    - [pack #167] Use custom uluru version to avoid a lot of allocations… ([`7bd3671`](https://github.com/GitoxideLabs/gitoxide/commit/7bd3671ad949d62f84147ef7ff3fde59937fee54))
    - [pack #164] fix docs ([`08ee674`](https://github.com/GitoxideLabs/gitoxide/commit/08ee674c55cef6ab76520de2f836b246c907888c))
    - Merge branch 'main' into 162-repo-design-sketch ([`e63b634`](https://github.com/GitoxideLabs/gitoxide/commit/e63b63412c02db469fbdb17da82cd1e9fda1ef0f))
    - Revert "[pack #167] Use custom uluru version to avoid a lot of allocations…" ([`4c2ea21`](https://github.com/GitoxideLabs/gitoxide/commit/4c2ea212bbffb0ba3c21ba388dfc79cc7a1c4734))
    - [pack #167] Use custom uluru version to avoid a lot of allocations… ([`8d49976`](https://github.com/GitoxideLabs/gitoxide/commit/8d499762b74c08437d901bb98806e0a1fc6f93bb))
    - [pack #167] a single-threaded special case for counting… ([`65e29de`](https://github.com/GitoxideLabs/gitoxide/commit/65e29de45a92c82cebd832634ab194db19a1b590))
    - [pack #167] generalize over immutable insertions… ([`169f000`](https://github.com/GitoxideLabs/gitoxide/commit/169f000087aab18f0257fb0c61dc3b3901e97505))
    - [pack #167] refactor ([`6bf0f7e`](https://github.com/GitoxideLabs/gitoxide/commit/6bf0f7e86312b2a4d262c80979c61c94519bd4b0))
    - [pack #167] progress is handled by reducer… ([`a22f8e1`](https://github.com/GitoxideLabs/gitoxide/commit/a22f8e171e705bc42fcf290789e8e05423bd72d1))
    - [pack #167] Error handling for object input ([`0aac40c`](https://github.com/GitoxideLabs/gitoxide/commit/0aac40c88a5c26f7c295db8433b510b168f15ca3))
    - Thanks clippy ([`d689599`](https://github.com/GitoxideLabs/gitoxide/commit/d689599d1b819c18a3be60075170dbe00462e216))
    - [pack #167] remove iterator based count objects impl… ([`7ec2f2b`](https://github.com/GitoxideLabs/gitoxide/commit/7ec2f2b40e83aaa218360a8b5989792cd67de2ed))
    - [pack] A non-iterator version of parallel object counting… ([`04fe855`](https://github.com/GitoxideLabs/gitoxide/commit/04fe855a37577d3da5bbd619807b44e449947893))
    - Release git-pack v0.8.2 ([`39a3f71`](https://github.com/GitoxideLabs/gitoxide/commit/39a3f71ba5997ac26d9994cdc7c2145af3220f64))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - Release git-pack v0.8.1 ([`045eb09`](https://github.com/GitoxideLabs/gitoxide/commit/045eb094691324a398120f6039bbfa34b4fda1af))
    - Remove dev-dependency cycles by removing their version ([`c40faca`](https://github.com/GitoxideLabs/gitoxide/commit/c40faca41632cd2a226daf4ddf5293b65d1fdc82))
    - Release git-diff v0.8.0, git-odb v0.20.0, git-pack v0.8.0, git-traverse v0.7.0 ([`f123f69`](https://github.com/GitoxideLabs/gitoxide/commit/f123f69c7a4f9fd1c98bd2f60ebc953a6739fe04))
    - Release git-diff v0.7.0, git-odb v0.19.0, git-pack v0.7.0, git-traverse v0.6.0 ([`c67291f`](https://github.com/GitoxideLabs/gitoxide/commit/c67291ff9bcdff9a747d87241f6a71015607af05))
    - Release git-object v0.12.0 ([`7006150`](https://github.com/GitoxideLabs/gitoxide/commit/7006150ac314d19814608723f69f6e70a72f9262))
    - (cargo-release) version 0.18.0 ([`b327590`](https://github.com/GitoxideLabs/gitoxide/commit/b327590d02fec5536c380b2d39dd7be089ca7c40))
    - (cargo-release) version 0.6.0 ([`d704bca`](https://github.com/GitoxideLabs/gitoxide/commit/d704bca7de0a6591f35345c842d6418b36ecd206))
    - (cargo-release) version 0.6.0 ([`4b71e15`](https://github.com/GitoxideLabs/gitoxide/commit/4b71e15c3ba4a17ff2da5a5ef79986a2832fa3f2))
    - (cargo-release) version 0.5.0 ([`e21142b`](https://github.com/GitoxideLabs/gitoxide/commit/e21142ba1a113b2afc4725d4d4225dff519c513a))
    - (cargo-release) version 0.17.0 ([`c52a491`](https://github.com/GitoxideLabs/gitoxide/commit/c52a49176bd294bb36db74b4293cdb684a2ab7f6))
    - (cargo-release) version 0.5.0 ([`c2f94a5`](https://github.com/GitoxideLabs/gitoxide/commit/c2f94a51bce287be301090450cb00cde57e92f76))
    - (cargo-release) version 0.4.0 ([`d69d0ac`](https://github.com/GitoxideLabs/gitoxide/commit/d69d0ac21989243fdafa514fa41579fd51bc2558))
    - (cargo-release) version 0.6.0 ([`d58f37e`](https://github.com/GitoxideLabs/gitoxide/commit/d58f37e3b5a000fbe069aa869bd84f66d5c3210b))
    - (cargo-release) version 0.5.0 ([`1687e59`](https://github.com/GitoxideLabs/gitoxide/commit/1687e599be98d97925fbab594f31cf5558e9d2b1))
    - (cargo-release) version 0.4.0 ([`28e58f6`](https://github.com/GitoxideLabs/gitoxide/commit/28e58f6b43a44e010da749a5618df02441f0d2e8))
    - (cargo-release) version 0.11.0 ([`a5be31c`](https://github.com/GitoxideLabs/gitoxide/commit/a5be31c4cf7c0b538a1ed4a52ff5c3a992c6feff))
    - (cargo-release) version 0.4.0 ([`70ef344`](https://github.com/GitoxideLabs/gitoxide/commit/70ef3442775b54ba9e4ee9ebfffb37af9804cc5b))
    - [utils #154] refactor: bool.then(||this) - neat ([`1dec1c4`](https://github.com/GitoxideLabs/gitoxide/commit/1dec1c49032c8acb449e463fde41f403cb640e45))
    - Revert "break more dev-depedency cycles up to git-odb" ([`22337ce`](https://github.com/GitoxideLabs/gitoxide/commit/22337ce23995eee474e7dfb2e37fb56814522942))
    - (cargo-release) version 0.3.1 ([`8b24197`](https://github.com/GitoxideLabs/gitoxide/commit/8b241977b31720e7f08809bca0b277267b29102e))
    - Break more dev-depedency cycles up to git-odb ([`7ee278b`](https://github.com/GitoxideLabs/gitoxide/commit/7ee278bf5b04adc5e4ab82cb83a3519f93587176))
    - (cargo-release) version 0.3.0 ([`0e9c73a`](https://github.com/GitoxideLabs/gitoxide/commit/0e9c73abd17e0dd21952275077ae53ad7e7aa1af))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - (cargo-release) version 0.16.0 ([`1231dbd`](https://github.com/GitoxideLabs/gitoxide/commit/1231dbd16dacefb39adec8e067c312d313a82e3c))
    - (cargo-release) version 0.5.0 ([`0e11e98`](https://github.com/GitoxideLabs/gitoxide/commit/0e11e98f0562c7baa9c90e18db6240731d165217))
    - [pack #153] finish transitioning to git-tempfile ([`38173fc`](https://github.com/GitoxideLabs/gitoxide/commit/38173fcf62c04b485c4b309bdf7e6b7afacfcd58))
    - Thanks clippy ([`e1964e4`](https://github.com/GitoxideLabs/gitoxide/commit/e1964e43979b3e32a5d4bfbe377a842d2c0b10ea))
    - [ref #139] add missing docs ([`5422ec8`](https://github.com/GitoxideLabs/gitoxide/commit/5422ec8923a5f3c284f7094894a952a392812e63))
    - [pack] refactor ([`581fb51`](https://github.com/GitoxideLabs/gitoxide/commit/581fb51a84567e341d315e6bacee8e681718f7a7))
    - [pack] refactor ([`b19f6b9`](https://github.com/GitoxideLabs/gitoxide/commit/b19f6b9b1fcd5ebbc5b1f2a4bef0543b1c693bd1))
    - [pack] fix docs ([`e7b9d96`](https://github.com/GitoxideLabs/gitoxide/commit/e7b9d9613874cd1ebaf740dc08db467c461a4751))
    - [pack] fix build ([`98dd557`](https://github.com/GitoxideLabs/gitoxide/commit/98dd557b963acfe1c4e717451d222c187c46a5da))
    - [pack] update CRC values when changing entries to satisfy all consistency checks ([`990ea48`](https://github.com/GitoxideLabs/gitoxide/commit/990ea4866be2d22ae2043da2dcd9577b748de255))
    - [pack] fix trailer of last entry to match expected recomputed pack hash… ([`8d0ec7d`](https://github.com/GitoxideLabs/gitoxide/commit/8d0ec7d7c0afb6112e66518a2987907d2e4d29e3))
    - [pack] refactor ([`1852e3e`](https://github.com/GitoxideLabs/gitoxide/commit/1852e3ea98a462958862ab05f110649e3b06e2b5))
    - [pack] all tests running for now, but… ([`aec8439`](https://github.com/GitoxideLabs/gitoxide/commit/aec8439683c639f7b6e344cb76bf1dd9fc769d17))
    - [pack] hacky proof of concept that this actually works… ([`6085a92`](https://github.com/GitoxideLabs/gitoxide/commit/6085a9201ecbd9285547c1d17c9834f09e22fef9))
    - [pack] on the way to 'quickly' get a proof of concept ([`cdc7582`](https://github.com/GitoxideLabs/gitoxide/commit/cdc7582ab7e35ec1daac44401bf7cb62e0b592a2))
    - [pack] refactor ([`685cce6`](https://github.com/GitoxideLabs/gitoxide/commit/685cce612eec99ed9f15d86d5ce2a7e6c270ae0d))
    - [pack] refactor ([`f822ebb`](https://github.com/GitoxideLabs/gitoxide/commit/f822ebb9e899bd52d5baec8179a843c47d073e44))
    - Thanks clippy ([`96ef0b0`](https://github.com/GitoxideLabs/gitoxide/commit/96ef0b036c3c94a45f3ab882a8b32bfcc1250653))
    - [pack] a quickly made iterator that writes input::Entries ([`116bdc4`](https://github.com/GitoxideLabs/gitoxide/commit/116bdc4ba879da9785877ebca56ab3c57b9cfd98))
    - [pack] prepare a custom writing iterator for input::Entries… ([`a4d2764`](https://github.com/GitoxideLabs/gitoxide/commit/a4d27648b4021bcf65c95dc5bcfa2b3d11f538fd))
    - Thanks clippy ([`bd517d6`](https://github.com/GitoxideLabs/gitoxide/commit/bd517d6374f20670086eedce2776a8ecf7d0d22b))
    - [pack] prepare bundle writer for yet another iterator wrapper… ([`33be1a1`](https://github.com/GitoxideLabs/gitoxide/commit/33be1a1ffba34a64eeb04b4479790fec2f50bcba))
    - [pack] refactor ([`50861e6`](https://github.com/GitoxideLabs/gitoxide/commit/50861e6266a6e1800607eb19288e040846325c06))
    - [pack] refactor ([`dc07225`](https://github.com/GitoxideLabs/gitoxide/commit/dc07225d7eea04e0cfe61c87b56009e06491726c))
    - [pack] another todo down, the last one ([`3fc8c8f`](https://github.com/GitoxideLabs/gitoxide/commit/3fc8c8ff5ab1c49b55e3b9e1af3fa2f0aee68b94))
    - [pack] one more todo down, it should work now, right?… ([`69a9ff1`](https://github.com/GitoxideLabs/gitoxide/commit/69a9ff17b3fe16de782ffabb76b87510e8a5b74e))
    - [pack] fix thin pack support test… ([`4bdebdd`](https://github.com/GitoxideLabs/gitoxide/commit/4bdebddd3791ba71f3f6b4182229a1c48c5a4a95))
    - [pack] definitely not working yet ([`690d9b7`](https://github.com/GitoxideLabs/gitoxide/commit/690d9b7fbc34b7d2393649d39290071f81cb8bb1))
    - [pack] a step closer, new cases show up ([`75eaba3`](https://github.com/GitoxideLabs/gitoxide/commit/75eaba36072cf29e76a97fbbd425f0861eb657e2))
    - [pack] refactor ([`a8512f8`](https://github.com/GitoxideLabs/gitoxide/commit/a8512f89a4e0dd7492fa208c1da41eed9d6a208f))
    - [pack] improved test to validate a fix ([`e3eeeb1`](https://github.com/GitoxideLabs/gitoxide/commit/e3eeeb146a0ba3dbe701b2e4da560309ff181753))
    - [pack] attempt to get a more realistic test, but… ([`2890737`](https://github.com/GitoxideLabs/gitoxide/commit/2890737c7e074d31f3bb55acb63664a2da93faaa))
    - [pack] refactor ([`cabc1e5`](https://github.com/GitoxideLabs/gitoxide/commit/cabc1e5858d52806542ee8d9266bac36e5d39c96))
    - [pack] first succeeding test ([`f5da439`](https://github.com/GitoxideLabs/gitoxide/commit/f5da439dce93cc203dacb4a5e9d0ae68a87b9be4))
    - [pack] first reasonably failing test showing that offset computation is indeed wrong ([`df1bc2f`](https://github.com/GitoxideLabs/gitoxide/commit/df1bc2f66ff9e7046898b6937c5ad239313a70dc))
    - [pack] the first test for the lookup ref deltas iter ([`b162f9e`](https://github.com/GitoxideLabs/gitoxide/commit/b162f9eb37f09f49e363376dc3f0c6c126442bbf))
    - [pack] Make use of thin-pack resolver when writing bundles… ([`9f43bf0`](https://github.com/GitoxideLabs/gitoxide/commit/9f43bf029624f7c94346646465e366609b89e2e1))
    - [pack] handle the same ref-base correctly ([`2f94854`](https://github.com/GitoxideLabs/gitoxide/commit/2f948545a935d2cb7c5a252ec74764440a9ff595))
    - [pack] thin pack resolver which might actually work ([`54f055a`](https://github.com/GitoxideLabs/gitoxide/commit/54f055a53e888156459340e8ab160650a198ab13))
    - [pack] first sketch of resolver for thin pack entries ([`ee428e0`](https://github.com/GitoxideLabs/gitoxide/commit/ee428e07bcc3df9bc795d06068a444beed71f2d0))
    - [pack] refactor ([`a8fd70f`](https://github.com/GitoxideLabs/gitoxide/commit/a8fd70fdbff871779ad5a9ba491162ae49605c9f))
    - [pack] thanks clippy ([`7c2fc89`](https://github.com/GitoxideLabs/gitoxide/commit/7c2fc89c70aa6de9cb0707799918e623267326a8))
    - [pack] actually, this is how it works, so this code should be unreachable ([`8f359e1`](https://github.com/GitoxideLabs/gitoxide/commit/8f359e1fc8cb99fcf0003eaab1d97cdeaac20876))
    - [pack] first step towards fixing bad-objects properly ([`3c96507`](https://github.com/GitoxideLabs/gitoxide/commit/3c965070a7c799f0507f9e7faae2896346bc9e65))
    - [pack] discard bad-object tracking in favor of delayed handling ([`31ce008`](https://github.com/GitoxideLabs/gitoxide/commit/31ce008208cdd3bc4f093abab6fabf4c8074c130))
    - Revert "[pack] fix race to finally make pack-gen missing objects…" ([`ad0d2a8`](https://github.com/GitoxideLabs/gitoxide/commit/ad0d2a8e4e92d11351225db0115de0ed1210f9e3))
    - [pack] fix race to finally make pack-gen missing objects… ([`73394db`](https://github.com/GitoxideLabs/gitoxide/commit/73394db1b048d3dc87b8b4934737f27b6a8a0d3c))
    - [pack] it seems git is just skipping bad objects during pack-gen ([`0f29b82`](https://github.com/GitoxideLabs/gitoxide/commit/0f29b82b48f45f509016eb16ea92af7f6dbf65a6))
    - Revert "[pack] FAIL: See if not looking up the pack location speeds up counting…" ([`d03fe97`](https://github.com/GitoxideLabs/gitoxide/commit/d03fe9732b69c6ca3b7a6df96097233661e53a05))
    - [pack] FAIL: See if not looking up the pack location speeds up counting… ([`48c4930`](https://github.com/GitoxideLabs/gitoxide/commit/48c49300a55e6443d5e4d94632979b6d07f2bc5a))
    - Revert "[pack] FAIL: speedup with Mutex<HashSet>" ([`df98edf`](https://github.com/GitoxideLabs/gitoxide/commit/df98edf48c49717136a6e8e5d9b1f64aeda17db2))
    - [pack] FAIL: speedup with Mutex<HashSet> ([`f8aca03`](https://github.com/GitoxideLabs/gitoxide/commit/f8aca03c2d126574541c136019df4e51b52a5b10))
    - [pack] In single-threaded mode, use a huge cache for some speedup ([`aec8a9b`](https://github.com/GitoxideLabs/gitoxide/commit/aec8a9b4b9deb102b06390a19727eab7660621f9))
    - [pack] fix offset index properly by using chunk-absolute offsets ([`461c1ee`](https://github.com/GitoxideLabs/gitoxide/commit/461c1eefe9214b07cd80a37292b23744846383d3))
    - [pack] forcefully fix issue with incorrect partition point ([`290bd65`](https://github.com/GitoxideLabs/gitoxide/commit/290bd65f10f5a64de6735b09119b7bbffc44254b))
    - [pack] test for parital pack without thin pack allowance… ([`1f48d3b`](https://github.com/GitoxideLabs/gitoxide/commit/1f48d3b58a1151a1fefce9bf4af5649837309a37))
    - [pack] pack-create with immediate counting and traversing… ([`b74a98f`](https://github.com/GitoxideLabs/gitoxide/commit/b74a98fc87a92a8ccbaec59aeea5284731e2fe49))
    - [pack] entry writer now supports deltas and it seems to work even ([`fcda6f0`](https://github.com/GitoxideLabs/gitoxide/commit/fcda6f096f95a6322122229ac364a2dd5ea0ce6b))
    - Thanks clippy ([`cc61f82`](https://github.com/GitoxideLabs/gitoxide/commit/cc61f82f597d9a0ab43efaaccc2cb568b9aa746f))
    - [pack] on-demand cache for pack-offset to id lookup ([`0bfdea8`](https://github.com/GitoxideLabs/gitoxide/commit/0bfdea843606673005ecab6a482a9fce89a4cb69))
    - [pack] refactor ([`4bb3ce4`](https://github.com/GitoxideLabs/gitoxide/commit/4bb3ce4f2e89dd817c284ed8ae9e2559ed60f9a2))
    - [pack] thin pack offset to index lookup ([`121aca4`](https://github.com/GitoxideLabs/gitoxide/commit/121aca45ecb1acce3496b1b2ac003aa95851f247))
    - [pack] refactor ([`372b9ce`](https://github.com/GitoxideLabs/gitoxide/commit/372b9cee78a6b49eb7ebb5cf452a324e07775d98))
    - [pack] a way to obtain whole bundles for offset-to-index lookup ([`15fcbe2`](https://github.com/GitoxideLabs/gitoxide/commit/15fcbe254b75e8f74652711cc339ae5ade74d24c))
    - [pack] refactor ([`64b1dcd`](https://github.com/GitoxideLabs/gitoxide/commit/64b1dcdb0fb53749ce73017d0dc1e053689d17d4))
    - [pack] refactor ([`1d713b4`](https://github.com/GitoxideLabs/gitoxide/commit/1d713b482264ddb0aba6a98e3918f8236ce12c80))
    - [pack] refactor ([`cdf020a`](https://github.com/GitoxideLabs/gitoxide/commit/cdf020a3b29bc59062d3ccf56672e9c18201c67c))
    - [pack] refactor ([`2ccefb2`](https://github.com/GitoxideLabs/gitoxide/commit/2ccefb2832b326966a24d0cbcfd79ca5309f91aa))
    - [pack] refactor; entry-iterator now produces delta-objects ([`5dc370b`](https://github.com/GitoxideLabs/gitoxide/commit/5dc370ba01d25a6e8b7f4bfa03259c83e6b1d758))
    - [pack] rough version of obtaining object indices for deltas ([`a58e270`](https://github.com/GitoxideLabs/gitoxide/commit/a58e270ef96011ffd2434539e3099cbe27aed3f3))
    - [pack] refactor ([`8cfa414`](https://github.com/GitoxideLabs/gitoxide/commit/8cfa414482a4318ed385f42582ec885fb73134e3))
    - [pack] pass all data to where it belongs to… ([`af5cb1f`](https://github.com/GitoxideLabs/gitoxide/commit/af5cb1f4b809ac268ca3d878896854c966dcea97))
    - [pack] add the notion of thin-packs to the pack generator ([`a289bba`](https://github.com/GitoxideLabs/gitoxide/commit/a289bbaa36546109d3371a8fcd7a6dc3c363861f))
    - [pack] build an index of pack ranges as well ([`4d6ab7b`](https://github.com/GitoxideLabs/gitoxide/commit/4d6ab7b74c325820a3760361faace380f958572f))
    - [pack] bundle::Location with pack offset; order counts by that… ([`f92f285`](https://github.com/GitoxideLabs/gitoxide/commit/f92f285167c6b5bc4d86f255e360c4534e38bb29))
    - [pack] better identify the currently implemented pack generation mode. ([`f9e3b3c`](https://github.com/GitoxideLabs/gitoxide/commit/f9e3b3ca3bbf063e8d71c62fe607b812c745a969))
    - [pack] refactor ([`f3dc3da`](https://github.com/GitoxideLabs/gitoxide/commit/f3dc3da492e1dda5dd9e43fddc57da6a118081b3))
    - [pack] refactor ([`9ee1e22`](https://github.com/GitoxideLabs/gitoxide/commit/9ee1e22fa5c5d97ff626f0dfc44706272433bfef))
    - [pack] refactor ([`78d46c1`](https://github.com/GitoxideLabs/gitoxide/commit/78d46c13d0510ee3e2e2f33cd60d624d63e85900))
    - [pack] refactor ([`69af352`](https://github.com/GitoxideLabs/gitoxide/commit/69af3526b0fcfa8a270238f3e2cf59d332bd187e))
    - Change wording ([`6c82a16`](https://github.com/GitoxideLabs/gitoxide/commit/6c82a16d340acb9b11c5cf56c917c9fe6f2cdf0e))
    - Bump uluru from 2.1.1 to 2.2.0 ([`52e274f`](https://github.com/GitoxideLabs/gitoxide/commit/52e274fe985948b6b742ff7066fcb9831e427ba3))
    - Don't use ASM on windows for Sha1 as it fails to build there. ([`ba1fb7a`](https://github.com/GitoxideLabs/gitoxide/commit/ba1fb7ab5bc03f5a23ece32ff1e144544e1eaeae))
    - Merge branch 'remove-unnecessary-unsafe' ([`7a3c5c1`](https://github.com/GitoxideLabs/gitoxide/commit/7a3c5c14dc56d8711548d1b219a969836693cbaa))
    - Remove unnecessary unsafe code ([`83e207a`](https://github.com/GitoxideLabs/gitoxide/commit/83e207a44aece0ff4870e57990bd5aaf43f38e22))
    - Remove unnecessary pub(crate) exports ([`3d2456e`](https://github.com/GitoxideLabs/gitoxide/commit/3d2456e11709f0461b37c6df55ecc3861ca4cab5))
    - Bump thiserror from 1.0.25 to 1.0.26 ([`9682590`](https://github.com/GitoxideLabs/gitoxide/commit/9682590095dc3a502b0c84ccd206ca4797635092))
    - Thanks clippy ([`6200ed9`](https://github.com/GitoxideLabs/gitoxide/commit/6200ed9ac5609c74de4254ab663c19cfe3591402))
    - Fix build ([`dbfa49a`](https://github.com/GitoxideLabs/gitoxide/commit/dbfa49acf58b2c0763c5e98e5276860b43dfb27b))
    - Fix everything up so that… ([`5930563`](https://github.com/GitoxideLabs/gitoxide/commit/5930563601d6c2148cf39e109f69f8b7c7dfcb36))
    - A first attempt to make intrerupt tools work, but… ([`8fb8d37`](https://github.com/GitoxideLabs/gitoxide/commit/8fb8d374ecfeffa3ae1bd07bf9bc5014351730f5))
    - Fix pack tests ([`7968467`](https://github.com/GitoxideLabs/gitoxide/commit/7968467cc0d392e3d223811ed36ae777531a5a36))
    - The last occurrence of the global git-features::interrupt usage gone ([`6820724`](https://github.com/GitoxideLabs/gitoxide/commit/6820724be83ebf48c7ccf6a65a3d6383f766c9de))
    - Another one ([`0a8ed0e`](https://github.com/GitoxideLabs/gitoxide/commit/0a8ed0ecc078d76dc3a5fe13518cf43bfbb121f0))
    - And another one down ([`abce75e`](https://github.com/GitoxideLabs/gitoxide/commit/abce75eefff44b9538c112b60ad5e0596482e89c))
    - Refactor ([`7f9be36`](https://github.com/GitoxideLabs/gitoxide/commit/7f9be36ea909ee67555591287bcb140fdc54c801))
    - And one less usage of the global interrupt handler… ([`5da57a3`](https://github.com/GitoxideLabs/gitoxide/commit/5da57a3b0efef75ad82cb4d1cd496fc7fc0f1c23))
    - Thanks clippy ([`3b2e765`](https://github.com/GitoxideLabs/gitoxide/commit/3b2e7650d8afe2c0e246e005ab1c321a157cbd44))
    - Make most interrupts local to the method or function ([`4588993`](https://github.com/GitoxideLabs/gitoxide/commit/458899306a3f3c8578f185d7ecbf1ade2a7142dd))
    - [features] sketch of iterator to auto-check for interruptions ([`61d3a15`](https://github.com/GitoxideLabs/gitoxide/commit/61d3a15c66b4c1be1d98715b8a60705a3a314455))
    - [pack] refactor ([`25f04ba`](https://github.com/GitoxideLabs/gitoxide/commit/25f04baa100bd1996f48fbeb4c87e40ff1b27d90))
    - [pack] refactor ([`18cabb8`](https://github.com/GitoxideLabs/gitoxide/commit/18cabb8618ffc324412302bfda208948abffb61f))
    - [pack] also put counts in order for stable packs ([`f299160`](https://github.com/GitoxideLabs/gitoxide/commit/f299160cafd00f0fea00a2402901570f5ddf27d5))
    - [pack] fix run of 'cargo test --all' ([`e7ecdc1`](https://github.com/GitoxideLabs/gitoxide/commit/e7ecdc195d03fa9a29ad1e44464b42e3ca6fb6a4))
    - [pack] a working in-order iterator ([`5fea926`](https://github.com/GitoxideLabs/gitoxide/commit/5fea926803bcc7b2ef7d8f156e3d31a503831091))
    - [pack] tests for error handling of in-order iterator ([`44892cc`](https://github.com/GitoxideLabs/gitoxide/commit/44892cca9309c4cca0eaa30dbedc65422a2699d1))
    - [pack] ground work for ordering in produced chunks ([`9680649`](https://github.com/GitoxideLabs/gitoxide/commit/96806494d32243bd1798a89c094e220dbe050d68))
    - [pack] also run multi-threaded tests as part of unit-tests ([`5d3006a`](https://github.com/GitoxideLabs/gitoxide/commit/5d3006a5d075bce9011b20920a84404952624c45))
    - Bump uluru from 2.0.0 to 2.1.1 ([`b6ac506`](https://github.com/GitoxideLabs/gitoxide/commit/b6ac506ba2df0f82eaae64eaf023cc0c0376ddff))
    - [pack] hopefully fix tests on CI; verify determinism of pack ([`51dec8b`](https://github.com/GitoxideLabs/gitoxide/commit/51dec8b3c661ba9071306ab89796aa93d9a25b65))
    - [pack] deterministic single-threaded pack generation ([`ddb6442`](https://github.com/GitoxideLabs/gitoxide/commit/ddb6442fd6681a2dd3890a8a415003ec770c7d64))
    - [pack] refactor ([`cfdf802`](https://github.com/GitoxideLabs/gitoxide/commit/cfdf8021ea1448ac4844b1f3bf252fefde2572fa))
    - [pack] basic statistics for entries ([`37229a6`](https://github.com/GitoxideLabs/gitoxide/commit/37229a650ceb0155aa7ca87b499fe188ac4bb565))
    - Thanks clippy ([`18b2113`](https://github.com/GitoxideLabs/gitoxide/commit/18b2113b1e3c372145bc9037ee6a9de7efe4e506))
    - [pack] write packs to a directory with the proper name ([`3fbca7d`](https://github.com/GitoxideLabs/gitoxide/commit/3fbca7dd62752a7dd752b83a39ec8dfd7b2f2ea8))
    - [pack] refactor ([`f10adea`](https://github.com/GitoxideLabs/gitoxide/commit/f10adea76d92eada3ca204fe69e7b5f81a06d8cc))
    - [pack] fix docs ([`6ba471d`](https://github.com/GitoxideLabs/gitoxide/commit/6ba471d228c45a3821b4984905a4b4ecaff5b0b0))
    - [pack] fix build ([`81ee633`](https://github.com/GitoxideLabs/gitoxide/commit/81ee633c7f482746bc28a2a43d74ebbaded7af5f))
    - [pack] statistics for counting objects seemingly work… ([`4e3deb1`](https://github.com/GitoxideLabs/gitoxide/commit/4e3deb1364dd1bef0af79d6aa97086a95b4983bc))
    - [pack] actual counts statistics ([`3a9f6d8`](https://github.com/GitoxideLabs/gitoxide/commit/3a9f6d8a53da3235bde4a3f32859381d4843cb7e))
    - [pack] aggregate the count outcome ([`c7ac0e6`](https://github.com/GitoxideLabs/gitoxide/commit/c7ac0e60a5d69f3a948d47c3acc3060cddbafb98))
    - [pack] use statistics reducer ([`0974ab1`](https://github.com/GitoxideLabs/gitoxide/commit/0974ab176777bfa02ac0ea32915f6d9c46e3ddeb))
    - [pack] count object reducer sketch ([`ea45692`](https://github.com/GitoxideLabs/gitoxide/commit/ea4569282e2f63042869dd47205874c161bfecfe))
    - [pack] refactor ([`fdf485a`](https://github.com/GitoxideLabs/gitoxide/commit/fdf485afa66af20abca586b04f588a33c167310f))
    - [pack] refactor ([`0514f1d`](https://github.com/GitoxideLabs/gitoxide/commit/0514f1df113c5f6bf1c934b15741ca8ea47316ae))
    - [pack] refactor ([`37922d1`](https://github.com/GitoxideLabs/gitoxide/commit/37922d12765c221e747fad4ca813597490525279))
    - (cargo-release) version 0.3.0 ([`6b33678`](https://github.com/GitoxideLabs/gitoxide/commit/6b33678f83e6d261ca15c4a7634ff5b4e66d81dd))
    - (cargo-release) version 0.2.0 ([`3286e42`](https://github.com/GitoxideLabs/gitoxide/commit/3286e42547b59df6365087cbae9ce1c9c959faad))
    - Refactor ([`a25a774`](https://github.com/GitoxideLabs/gitoxide/commit/a25a774675e2e9db1c891351077d3af2fd5c72ed))
    - [git-transport] Show how to use blocking git-pack code in non-blocking transports ([`de2ba3c`](https://github.com/GitoxideLabs/gitoxide/commit/de2ba3c4919d454894911c54fd4bb0e0a4665723))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - [git-repository] towards git-repository as one stop shop ([`aea6cc5`](https://github.com/GitoxideLabs/gitoxide/commit/aea6cc536f438050cc0e02223de7702cd7912e75))
    - [git-ref] the first failing test ([`7e802a0`](https://github.com/GitoxideLabs/gitoxide/commit/7e802a0576230dfc666c253d484ea255f265f92f))
    - (cargo-release) version 0.2.0 ([`b213628`](https://github.com/GitoxideLabs/gitoxide/commit/b213628feeb8dfa87dab489c7d3155a60e6a236d))
    - [git-odb] prep release ([`4984ce3`](https://github.com/GitoxideLabs/gitoxide/commit/4984ce3e19b60b89a4337f90ac4b9c44c42558a0))
    - [git-odb] refactor ([`2958145`](https://github.com/GitoxideLabs/gitoxide/commit/2958145a0ae1ef582bbf88352f5567d5c2b5eaf0))
    - [git-pack] fix docs ([`efd20d4`](https://github.com/GitoxideLabs/gitoxide/commit/efd20d4e1afbfbe573d620dea4761c06f948a296))
    - [git-pack] refactor ([`ea2b3de`](https://github.com/GitoxideLabs/gitoxide/commit/ea2b3deab78882943e11270e4166ca7c340b03e1))
    - [git-pack] refactor ([`bc4b7b1`](https://github.com/GitoxideLabs/gitoxide/commit/bc4b7b18a04506a3d08d66d1222d706b82a2f6e7))
    - [git-pack] refactor ([`157b6ff`](https://github.com/GitoxideLabs/gitoxide/commit/157b6ff7b55ba2b7f8f90f66864212906426f8d7))
    - [git-pack] refactor ([`49c1c3e`](https://github.com/GitoxideLabs/gitoxide/commit/49c1c3ea67379c5a122a8c3921d8ff713e14d371))
    - (cargo-release) version 0.16.0 ([`769c649`](https://github.com/GitoxideLabs/gitoxide/commit/769c649c00c009bf5a3f7c0611a7b999618f2938))
    - [git-pack] refactor ([`be6ddaa`](https://github.com/GitoxideLabs/gitoxide/commit/be6ddaa98fc1dcaf77dc0fd9c9d67754e74927e4))
    - [git-pack] used by git-odb ([`5d6ee07`](https://github.com/GitoxideLabs/gitoxide/commit/5d6ee07a8dec64fe5f68c14c418d922077fad3df))
    - [git-pack] refactor ([`1b2a245`](https://github.com/GitoxideLabs/gitoxide/commit/1b2a245aa494c0f9cacc2ad6b8ca02e9891fdb4c))
    - [git-pack] move hash-writer to git-features as it's quite general purpose ([`80e5640`](https://github.com/GitoxideLabs/gitoxide/commit/80e5640169363910b4189fda58bb495c6677eaaa))
    - [git-pack] the very first version… ([`8c06cdb`](https://github.com/GitoxideLabs/gitoxide/commit/8c06cdb14269e798b7ff771ea3864f85fa673ed7))
</details>

## 0.30.1 (2023-01-10)

A maintenance release without user-facing changes.

## 0.30.0 (2023-01-09)

A maintenance release without user-facing changes.

## 0.29.0 (2022-12-30)

A maintenance release without user-facing changes.

## 0.28.0 (2022-12-19)

### New Features

 - <csr-id-a85dcddba29a453bbea87ac865b8aedc97f41aed/> add `data::File::resolve_header()` to obtain object information without decompressing it.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.

### Changed (BREAKING)

 - <csr-id-37f3a675d7fa931f5d3f38e91df74ec0b517422b/> move `data::ResolveBase` into `data::decode_entry::`; unify `decode_entry|decode_header::Error` into `decode::Error`.

## 0.27.0 (2022-11-21)

A maintenance release without user facing changes.

## 0.26.0 (2022-11-08)

A maintenance release without user-facing changes.

## 0.25.0 (2022-11-06)

### New Features

 - <csr-id-450257eb596465352fb363a5c8c514f544e7f9ac/> `Bundle::write…` also produces a `.keep` file
   These files are placed before moving the corresponding pack and index
   files into place to prevent them from being picked up for collection
   while their refs are still being placed.
   
   The caller is responsible for deleting them once refs integrate the
   contained objects into the commit graph.

## 0.24.0 (2022-10-10)

<csr-id-b46347fd3d50886eeca500e31e1e12b354711309/>
<csr-id-591afd56d9862a6348ef8b3af61798004b36aa19/>

### New Features

 - <csr-id-6836cb148cbadf919bdac2e0e77bdec34caa9307/> more efficient distribution of tasks to threads during pack resolution.
   This clearly is a peak-performance optimization as it will now
   efficiently distribute tree-root nodes to threads one at a time by
   means of shared memory, which is accessed mutably yet safely.
   
   This change can also further reduce peak memory usage as it will not
   keep buffers of more resolved deltas then necessary, as it only handles
   one at a time per thread.

### Bug Fixes

 - <csr-id-ec58bef84d620360dc52e34e173ace1310f74c85/> reduce memory usage for pack resolution by 18%…
   …compared to where it started out before.
   
   This is the result of using u32 instead of usize where possible,
   leveraging the object limit of u32 in packs.
   
   This change makes the biggest difference by not keeping the decompressed
   memory of leaf nodes alive for longer than needed, at the cost of
   some code duplication which could be extracted into a function if one
   was inclined to deal with the boilerplate and lots of generics.
 - <csr-id-49d168a8859c6b5a9e7ef58cd836093212c2c7ad/> Allow verification of empty packs and indices.
   Empty packs are fine, even though the implementation should probably
   assure to not write them, or remove them after having been written.
 - <csr-id-a745512185fb0a46e35daaa6d28829aec05edb55/> increase pack-receive performance using a BufWriter
   Previously the NamedTempFile would receive every little write request
   for millions of objects, consuming considerable amounts of time.
   
   Now a buf writer alleviates this issue entirely.
 - <csr-id-0b6ed60f842f0a36f61f187651080540a358758e/> `bundle::write::Error` is now publicly available

### Bug Fixes (BREAKING)

 - <csr-id-5a75afe0467e4a84323ea10172eed835cc7fae4c/> Reading and writing empty packs is explicitly allowed.
   This can happen when sending packs that don't actually contain changes,
   but need to be sent to conform to the protocol.

### Other (BREAKING)

- <csr-id-b46347fd3d50886eeca500e31e1e12b354711309/> `index::write::Outcome::index_kind` -> `::index_version`.
- <csr-id-591afd56d9862a6348ef8b3af61798004b36aa19/> `bundle::write::Options::index_kind` -> `::index_version`.

## 0.23.0 (2022-09-20)

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

## 0.22.0 (2022-08-28)

Maintenance release without user-facing changes.

## 0.21.2 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

## 0.21.1 (2022-08-17)

A maintenance release without user facing changes.

## 0.21.0 (2022-07-22)

This is a maintenance release with no functional changes.

### New Features (BREAKING)

 - <csr-id-95210cb2ba85f75148b4ef48ccea9d9f8a0a0114/> Provide optional `candidates` for ambiguous entries during `lookup_prefix()`
   The candidate entries are all entries matching a given prefix.

## 0.20.0 (2022-06-13)

A maintenance release without user-facing changes.

## 0.19.1 (2022-05-21)

A maintenance release without user-facing changes.

## 0.19.0 (2022-05-18)

A maintenance release without user-facing changes.

## 0.18.0 (2022-04-05)

### Changed (BREAKING)

 - <csr-id-8c5ae77f06a64c57df9a9ad1190266896a223dbe/> Remove deprecated compound and linked object databases
   The dynamic/general store is the only maintained can-do-it-all
   DB now.

## 0.17.0 (2022-04-03)

<csr-id-25209454d3f7e27e12e8ddca92e43b1ff01d58aa/>
<csr-id-9b9f10ad862b5e097c836c51df1eb98607df5ae1/>

### Chore

- <csr-id-25209454d3f7e27e12e8ddca92e43b1ff01d58aa/> upgrade dashmap to 5.1.0 (with security fix)

### New Features

 - <csr-id-503b1a1f8d4f39b44c166209d7a8ba8d74137859/> `index::File::lookup_prefix(…)`
 - <csr-id-cb83beedd1aa389f6774e2296f79273e8c8f14f4/> gix-hash::Prefix::from_id()
   A way to obtain a prefix of an object id, with all non-prefix
   bytes set to zero.
 - <csr-id-16208306ab49ade30d8ffd6b067ebd8eefd84cd4/> in-manifest and in-lib documentation of feature toggles

### Bug Fixes

 - <csr-id-42e0487286c1f745837c0ce337ed7c9d86b14516/> support Rust 1.52

### Refactor

- <csr-id-9b9f10ad862b5e097c836c51df1eb98607df5ae1/> remove unnecessary unsafe by using `chunks_mut()`
  This was probably a left-over from times where there was a static
  requirement on the chunks processing. Maybe… .

## 0.16.1 (2022-02-01)

### Bug Fixes

 - <csr-id-d9451e8d7fc39c252042f9d2447061262c16ae7a/> downgrade dashmap to 4.0 to avoid unsoundness.
   See https://github.com/xacrimon/dashmap/issues/167 for tracking
   progress on resolving the issue.

## 0.16.0 (2022-01-23)

<csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/>
<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>
<csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/>
<csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/>
<csr-id-e6ff1a885889cf88f6b34b1193aa03d8bce16af5/>
<csr-id-f48630ba8f745c2ec61a1e3c51fa63a1789a088c/>

### Refactor

- <csr-id-e0b8636f96e4bfe1bc72b5aa6ad4c4c8538ff92c/> replace bare u32 `data::Id` typedef

### Other

- <csr-id-e6ff1a885889cf88f6b34b1193aa03d8bce16af5/> :File uses its hash_len parameter
- <csr-id-f48630ba8f745c2ec61a1e3c51fa63a1789a088c/> :Find implementation for Rc

### Chore

- <csr-id-c800fdd331e6d7a0b8d756ba822915259f26e9e8/> remove unused dependencies

### New Features

 - <csr-id-b80dec2323b81fb2172df76c7d897a4b5e6bdfea/> zero-objects check for index and multi-index integrity validation
 - <csr-id-56fc99fb9c1cab61abd03c10e1b4af0d6e491bbf/> support for fan-checking in index and multi-index integrity verification
 - <csr-id-28e3ea8612112f6a04cfaff591565eca5a1ffba2/> introduce type for entry indices within an index or multi-index
   That way it's a little more descriptive than a bare u32.
 - <csr-id-58c2edb76755ab71e10eef4cd9a51533825c291f/> gix_pack::Find::try_find_cached(…, pack_cache)
   With this method it's easier to bypass local caches and control
   the cache oneself entirely.
 - <csr-id-e25f4eadec679406aad6df10026e27e4832c2482/> A simplified version of the `Find` trait
   It's meant for the next generation of object db handles which keep a
   local cache of all the details of the actual object database.

### Bug Fixes

 - <csr-id-42e0487286c1f745837c0ce337ed7c9d86b14516/> support Rust 1.52
 - <csr-id-84ade1d23060f10bf6c8529f8f693d06660b4f4e/> Allow resolution of in-pack ref-deltas
   This finally allows delta tree caches to be used on typical small packs
   returned by GitHub.
 - <csr-id-ba92cc09ba41fe4c9a9097bfeb8d18016408fcdf/> don't try to short-cut detection of large offsets when writing index files
   The code incorrectly assumed that the input is sorted by offsets, with
   the largest offset being last, even though by all means that's not the
   case.
 - <csr-id-6d3f52dc13d7243a6bce6dab89a985114a75d94b/> Avoid the dashmap being cloned for each thread
   Instead, share it by reference, it's sync after all.
   
   This issue was introduced when switching to a `Send + Clone` model,
   instead of `Send + Sync`, to allow thread-local caches in database
   handles of all kinds.
 - <csr-id-b605c1fa0494b10872d3c2e6ecce0e39f1a90a9e/> linked::Store now assures unique IDs across compound stores

### Changed (BREAKING)

 - <csr-id-a79a7fb638b45df88af0d0d5fc9ada6d824bc328/> Improve method signatures of `cache::Tree::*`
 - <csr-id-91d047658b114f372735116c9d8e6962a3873137/> cleanup and unify `verify_integrity()` method signature
   Previously they used many different ways of handling their parameters
   despite all boiling down to calling the same 'index::File::traverse()`
   method.
   
   This allows for more reuse of `Options` structs and generally makes
   clearer how these options are used.
 - <csr-id-2cf7727228e1d8094ffd2eec6746006348c39eab/> `index::File::traverse()` now returns an `Outcome` struct instead of tuple of 3 fields
 - <csr-id-bf04644ab75ed1969507f957dc8d4868790d462d/> remove `Option<impl Progress>` in favor of `impl Progress`
 - <csr-id-6829e5e5d6aed1e6c87647144e2dd76a1e4b9f1f/> multi-index integrity check; use `integrity::Outcome` for various integrity checks
 - <csr-id-d851bede97801096d188ff6af06c98a79fe276db/> remove unnecessary `Arc` around `should_interrupt` flag
 - <csr-id-c2679a03358b9c19d63ed1af1cd57324c6381447/> remove Sha1 mentions in `index::verify::Mode::*` variants
   The hash is repository defined and not hard-coded
 - <csr-id-80b120d3278e46429f848df7af3db13413c36649/> introduce `index::File::verify_integrity(…, pack: Option<PackContext>, …)`, replacing tuple
   This allows for more documentation on what input is required there and
   generally makes for an easier to use API.
 - <csr-id-79dc0d5ba6fa31ddd5c075693ffdc6496c1eaded/> rename `oid::try_from()` to `try_from_bytes()`, add `from_bytes_unchecked()`
   This change was done in the name of consistency, as `from_bytes()` is
   used in many other git-* crates
 - <csr-id-2ef9a8424af51310db8c1e6df31dde9953ed3d21/> Change accessors named `hash_kind()` to `object_hash()` for consistency
 - <csr-id-b76f6be6c5baa6cf613a174241f007e92bf5ba36/> consistently use `object_hash` instead of `hash_kind`
 - <csr-id-629412b4cb192614b7eff08dbf203e3448c902c1/> data::Entry::from_read() now takes a hash lengths as parameter
   That way ref-deltas can be interpreted without hard-coding SHA1
 - <csr-id-851dc2c52fa8e204ba2d5ced8fb0959a889869d8/> data::Entry::from_bytes(…, hash_len) takes new parameter
   The hash-len tells it how to interpret ref-delta objects, which
   store the complete hash of the base object.
   
   This is now entirely configurable.
 - <csr-id-db8c8c41b4ced0fc296d3877883d801e77d550ae/> `index::File::at()` with gix_hash::Kind parameter
   It will allow to assume different hashes even in the index file format
   which isn't yet capable of storing this information.
 - <csr-id-e6a3c9f72332b524b143bc94ee9df0a6db11e864/> `data::File::at()` and `Bundle::at()` now have `hash_kind` parameter
   It's used to configure the kind of hash to assume when reading packs and
   indices.
 - <csr-id-3f05fea55dc8acce1ed62ecbe4e0a1394f2720b7/> remove `make_object_cache` parameter from `gix_pack::data::output::count::objects()`
   It now is an implementation detail of the Find trait.
 - <csr-id-82b9b33bd5f4c3c1721a5093de2cedc62cb10565/> move `bundle::Location` to `data::entry::Location`
   The latter place best describes its purpose.
- remove pack-cache from `Find::try_find(…)`
  With the new architecture this can be an implementation detail without
  forcing it to be Sync.
- move gix_pack::data::Object to gix_object::Data, massively alter gix_odb::Find trait
  This will break a lot, but has to happen to prepare these traits for the
  next generation of object databases.

## 0.15.0 (2021-11-29)

<csr-id-598698b88c194bc0e6ef69539f9fa7246ebfab70/>

### Changed (BREAKING)

 - <csr-id-e7526b2a7b51cbac4018e1ab3b623a85987fadc2/> parallel utilities now use `Send + Clone` instead of `Send + Sync`.
   
   This helps to assure that thread-local computations always work with the
   kind of types we provide. The ones that are carrying out actions are
   notably not `Sync` anymore.
   
   We cater to that by defining our bounds accordingly, but for those
   who want to use other utilities that need Sync, using types like
   `Repository` and `thread_local!()` is the only way to make this
   work.

## 0.14.0 (2021-11-16)

<csr-id-e8b091943f0c9a26317da0003f7fcdf5a56ef21a/>

An important bugfix to prevent assertion failures when writing thin packs.

### Bug Fixes

 - <csr-id-20b3994206aa5bc5e35cbbc9c8f8f99187077f79/> Adjust size-hints of resolving entries iterator and use the upper bound in delta tree.
   
   The delta-tree is a data structure that actually heavily relies on
   favorable allocation and a known amount of objects in order to
   provide front and back buffers. However, this is an implementation
   detail and they don't have to stay consistent at all especially
   after growing the buffer by pushing to it.
   
   Interestingly, the VecDeque internally over-allocates as well which
   definitely helps the example of `as_mut_slices()`, otherwise
   it could also suffer from the assertions that trigger here.

## v0.13.0 (2021-10-19)

A maintenance release to properly dealing with previously breaking changes in `gix-hash`.

## v0.12.0 (2021-10-15)

<csr-id-d8fe8141e80a9e9a433b5e1a072b850325c806c8/>
<csr-id-faf6f813927720c5adf62102f9ce46606ff2617c/>
<csr-id-2f2d856efe733d3cf81110c0e0607d2e7c40d968/>
<csr-id-71c628d46088ab455b54eb2330d24dcff96c911d/>
<csr-id-8fe461281842b58aa11437445637c6e587bedd63/>

This release contains bugfixes and features, but is considered breaking as `gix-traverse`
signalled a breaking change which is one of our dependencies.

### Refactor

- <csr-id-71c628d46088ab455b54eb2330d24dcff96c911d/> Use 'cache::Object' trait where it matters
- <csr-id-8fe461281842b58aa11437445637c6e587bedd63/> split data::output::count::objects into files

### New Features

 - <csr-id-d6c44e6ab8f436020d4fb235e423b018fd1e7a9f/> dynamically sized full-object speeds up diff-based object counting
   which is what happens when counting objects for fetches where only changed objects should be sent.
 - <csr-id-50cf610e8939812c3d2268c48835e2dac67d0c31/> `cache::Object` trait for caching and retrieving whole objects
 - <csr-id-60c9fad8002b4e3f6b9607bba6361871752f4d3d/> control pack and object cache size in megabytes
 - <csr-id-5a8c2da6cb1e2accf7cfdccc16bc3a1d0b2a7dbc/> object cache size is configurable

### Bug Fixes

 - <csr-id-d8fe8141e80a9e9a433b5e1a072b850325c806c8/> don't put more objects into the pack cache than needed.
   
   Previously when accessing a packed object, it would store the base
   object into the pack cache (if it wasn't retrieved from there)
   which is great if that operation is free.
   
   Since it isn't, it's better not to stress the cache with puts
   and trash more objects than necessary.
   
   Now only the last decompressed object will be put into the LRU cache.
 - <csr-id-faf6f813927720c5adf62102f9ce46606ff2617c/> don't include submodules in count,
   which avoids dealing with missing objects entirely. Those ominous missing objects where just git submodules after all.
   
   It's still a good idea to handle these gracefully though, git itself
   seems to ignore them, too, and so do we at least for now.

### Performance

 - <csr-id-f9232acf8e52f8cd95520d122469e136eb07b39f/> ObjectID specific hashers, using the fact that object ids are hashes

## v0.11.0 (2021-09-08)

- manual bump for safety as its dependencies have breaking changes

## v0.10.0 (2021-09-07)

- **renames**
    - `data::Object::into_commit_iter()` -> `data::Object::try_into_commit_iter()`
    - `data::Object::into_tree_iter()` -> `data::Object::try_into_tree_iter()`
    - `data::Object::into_tag_iter()` -> `data::Object::try_into_tag_iter()`

## v0.9.0 (2021-08-27)

- **renames / moves / visibility**
    - `find::Find`  and `find::FindExt` only in `Find` and `FindExt` (not in `find` anymore)
    - `data::output::count::Count` -> `data::output::Count`
    - `data::output::entry::Entry` -> `data::output::Entry`
    - `Find::find_existing_*` -> `Find::find_*`
    - `Find::find_existing_*` -> `Find::find_*`
    - `Find::find()-> `Find::try_find()`
    - `bundle::Bundle` -> `Bundle`
    - `bundle::Error` -> `bundle::init::Error`
    - `pub tree::` -> `pub(crate) cache::delta::`
    - `data::object::Object` -> `data::Object`
    - `data::entry::Entry` -> `data::Entry`

* **new methods**
    - `Find::find_tag_iter()`

## v0.8.2 (2021-08-17)

## v0.8.1 (2021-08-13)

## v0.8.0 (2021-08-12)

## v0.6.0 (2021-08-11)

## v0.5.0 (2021-08-11)

## v0.3.1 (2021-08-10)

## v0.3.0 (2021-08-10)

## v0.2.0 (2021-05-25)

## v0.1.0 (2021-05-24)

