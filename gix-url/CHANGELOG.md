# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Bug Fixes

 - <csr-id-6fd1241427f115bc553f22f20fbf4538cb3d66dd/> Implement percent-decoding in HTTP URLs for username, password and paths, and add whitespace rejection.
 - <csr-id-62c1407daf42fe13b755a87efccdd5cedcc9be4f/> Detect and fail if HTTP URLs contain whitespace for username, password and path
 - <csr-id-454b33000e3ee959a00987f78ae8e3b6b66d779b/> disallow spaces in hosts

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 18 calendar days.
 - 18 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2355](https://github.com/GitoxideLabs/gitoxide/issues/2355)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2355](https://github.com/GitoxideLabs/gitoxide/issues/2355)**
    - Disallow spaces in hosts ([`454b330`](https://github.com/GitoxideLabs/gitoxide/commit/454b33000e3ee959a00987f78ae8e3b6b66d779b))
 * **Uncategorized**
    - Merge pull request #2358 from GitoxideLabs/copilot/decode-url-in-simple-url ([`f56770f`](https://github.com/GitoxideLabs/gitoxide/commit/f56770f25547f43a1f2067e4a137a3c3bb141683))
    - Refactor ([`a3e1823`](https://github.com/GitoxideLabs/gitoxide/commit/a3e1823f72a247b13cbfcd32d365671066f82463))
    - Implement percent-decoding in HTTP URLs for username, password and paths, and add whitespace rejection. ([`6fd1241`](https://github.com/GitoxideLabs/gitoxide/commit/6fd1241427f115bc553f22f20fbf4538cb3d66dd))
    - Merge pull request #2357 from GitoxideLabs/copilot/validate-url-invariants ([`9081b8f`](https://github.com/GitoxideLabs/gitoxide/commit/9081b8f79ca977c653cf7e92162be5d079aece6e))
    - Refactor ([`5c99c68`](https://github.com/GitoxideLabs/gitoxide/commit/5c99c6824a1d1c89d6a83e8a6261f6dd8a15a785))
    - Detect and fail if HTTP URLs contain whitespace for username, password and path ([`62c1407`](https://github.com/GitoxideLabs/gitoxide/commit/62c1407daf42fe13b755a87efccdd5cedcc9be4f))
    - Merge pull request #2356 from GitoxideLabs/copilot/fix-failing-test-issue-2355 ([`1f1a3d1`](https://github.com/GitoxideLabs/gitoxide/commit/1f1a3d14b6e84fc9d6a75cfab45decb2b6870162))
    - Refactor ([`510244e`](https://github.com/GitoxideLabs/gitoxide/commit/510244e1a86fea9ce16c2bf648d8169e5d53221a))
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
</details>

## 0.34.0 (2025-12-22)

### Bug Fixes

 - <csr-id-1afa7a5de63c9398ff7ddb25a33c979e3b504bdc/> prevent dots from being URL-encoded in userinfo and add tests

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 20 calendar days.
 - 29 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2271 from GitoxideLabs/improve-gix-url ([`169c17c`](https://github.com/GitoxideLabs/gitoxide/commit/169c17c099eb366ee11ba99c3d957f2403ae1bf5))
    - Refactor ([`e09b3d6`](https://github.com/GitoxideLabs/gitoxide/commit/e09b3d6c12cda43b99d109406972404a3c5dbc31))
    - Allow baseline failures on Windows until there is time to debug this by hand. ([`5d16214`](https://github.com/GitoxideLabs/gitoxide/commit/5d16214ef4f0809bd32fe7a4bbd47139796ce0e3))
    - Add more tests for better explicit coverage of important features ([`9a9d1b7`](https://github.com/GitoxideLabs/gitoxide/commit/9a9d1b7de161b7a57f06374c01ea5a4b3fe633de))
    - Make all baseline tests pass ([`5bfd150`](https://github.com/GitoxideLabs/gitoxide/commit/5bfd150fbe6b410628dffceab457accf714e1823))
    - Merge pull request #2280 from GitoxideLabs/copilot/add-tests-for-user-password-encoding ([`e871c55`](https://github.com/GitoxideLabs/gitoxide/commit/e871c55882abb5fc34380171031ef4447b86c198))
    - Refactor ([`87048a3`](https://github.com/GitoxideLabs/gitoxide/commit/87048a3d05ad62b0376e414765b2fe0ee6b41630))
    - Prevent dots from being URL-encoded in userinfo and add tests ([`1afa7a5`](https://github.com/GitoxideLabs/gitoxide/commit/1afa7a5de63c9398ff7ddb25a33c979e3b504bdc))
</details>

## 0.33.2 (2025-11-22)

### New Features

 - <csr-id-402bcc7b0b8a95a7dbfe6fcdc9e4b182dd303433/> Add `Scheme::default_port()`
 - <csr-id-b34d1d2704d58ec69c7722572fa5f66b268e1231/> Replace url dependency with minimal custom implementation

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
    - Merge pull request #2261 from GitoxideLabs/copilot/replace-url-dependency ([`896cb4d`](https://github.com/GitoxideLabs/gitoxide/commit/896cb4defcaad35ce54d83ac53ac6cbdfd00a6c3))
    - Add `Scheme::default_port()` ([`402bcc7`](https://github.com/GitoxideLabs/gitoxide/commit/402bcc7b0b8a95a7dbfe6fcdc9e4b182dd303433))
    - Refactor ([`f679c13`](https://github.com/GitoxideLabs/gitoxide/commit/f679c13cb419b530b0fa82363ad2aa6fa08f1fcf))
    - Replace url dependency with minimal custom implementation ([`b34d1d2`](https://github.com/GitoxideLabs/gitoxide/commit/b34d1d2704d58ec69c7722572fa5f66b268e1231))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
</details>

## 0.33.1 (2025-10-23)

<csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/>
<csr-id-c836c3dcdcfe7775ff10086d61cbfb2bdc115f57/>

### Other

 - <csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/> Remove `doc_auto_cfg` feature to fix docs.rs documentation.
   It is part of `doc_cfg` feature since https://github.com/rust-lang/rust/pull/138907
   
   This fixes the docs.rs build
 - <csr-id-c836c3dcdcfe7775ff10086d61cbfb2bdc115f57/> Add documentation explaining URL serialization and password handling

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.7, gix-actor v0.35.6, gix-trace v0.1.15, gix-features v0.44.1, gix-hash v0.20.1, gix-object v0.51.1, gix-glob v0.22.1, gix-attributes v0.28.1, gix-packetline-blocking v0.19.3, gix-commitgraph v0.30.1, gix-archive v0.23.1, gix-tempfile v19.0.1, gix-index v0.42.1, gix-config-value v0.15.3, gix-ignore v0.17.1, gix-worktree v0.43.1, gix-diff v0.54.1, gix-ref v0.54.1, gix-sec v0.12.2, gix-config v0.47.1, gix-url v0.33.1, gix-credentials v0.31.1, gix-mailmap v0.27.4, gix-revision v0.36.1, gix-pack v0.61.1, gix-odb v0.71.1, gix-packetline v0.19.3, gix-transport v0.49.1, gix-protocol v0.52.1, gix-status v0.21.1, gix v0.74.1 ([`bdcce5f`](https://github.com/GitoxideLabs/gitoxide/commit/bdcce5f2c6723ebe489dbe936a4656859ce1c2a5))
    - Remove `doc_auto_cfg` feature to fix docs.rs documentation. ([`6f469a6`](https://github.com/GitoxideLabs/gitoxide/commit/6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b))
    - Merge pull request #2223 from GitoxideLabs/copilot/update-gix-url-documentation ([`fb13346`](https://github.com/GitoxideLabs/gitoxide/commit/fb13346809b101e1cf24ad97ad5f5773ebd2a963))
    - Adjust documentation to be a bit less verbose. ([`78898a7`](https://github.com/GitoxideLabs/gitoxide/commit/78898a7987c181d1d04d917e308bba10eeac1e41))
    - Add documentation explaining URL serialization and password handling ([`c836c3d`](https://github.com/GitoxideLabs/gitoxide/commit/c836c3dcdcfe7775ff10086d61cbfb2bdc115f57))
    - Merge pull request #2224 from GitoxideLabs/report ([`3313233`](https://github.com/GitoxideLabs/gitoxide/commit/3313233aa4e7009aed0ddf644f4271fd2a98e8d4))
</details>

## 0.33.0 (2025-10-22)

### New Features

 - <csr-id-3a470d1e8e001594276546e75dab0552fff8629f/> Make all Url fields public
   This way it's easier to manipulate the URL at will. Since there is no validation, users of URLs should never take a parsed structure, but instead take the path or URL directly to parse it themselves.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 99 calendar days.
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
    - Make all Url fields public ([`3a470d1`](https://github.com/GitoxideLabs/gitoxide/commit/3a470d1e8e001594276546e75dab0552fff8629f))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.32.0 (2025-07-15)

### Bug Fixes

 - <csr-id-04bc4a81614146f56f341e15b459dfc1a880bd45/> username in scp-like url is no longer percent-encoded
   Since Git doesn't percent-decode characters in scp-like URL, we shouldn't encode
   username at all.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2056](https://github.com/GitoxideLabs/gitoxide/issues/2056)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2056](https://github.com/GitoxideLabs/gitoxide/issues/2056)**
    - Username in scp-like url is no longer percent-encoded ([`04bc4a8`](https://github.com/GitoxideLabs/gitoxide/commit/04bc4a81614146f56f341e15b459dfc1a880bd45))
 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2060 from yuja/push-urolxnurwtsn ([`68d761c`](https://github.com/GitoxideLabs/gitoxide/commit/68d761cdc87a46b72028153e096f1a22012239bc))
    - Add baseline tests for `_` and `@` in username ([`212b618`](https://github.com/GitoxideLabs/gitoxide/commit/212b618c5f99cc75ed612431669dcc2ec4c49a5e))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.31.0 (2025-04-26)

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

## 0.30.1 (2025-04-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1962](https://github.com/GitoxideLabs/gitoxide/issues/1962)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1962](https://github.com/GitoxideLabs/gitoxide/issues/1962)**
    - Allow `testing::TestUrlExtension` in release builds for consistency ([`633f0a8`](https://github.com/GitoxideLabs/gitoxide/commit/633f0a86b8941ffddebfe5f36f4604925b822ab3))
 * **Uncategorized**
    - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
    - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
    - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
    - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Merge pull request #1963 from joshtriplett/zlib-rs-default ([`9e075b9`](https://github.com/GitoxideLabs/gitoxide/commit/9e075b99ffc79173d4052d7550fd1d2826c5ec71))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.30.0 (2025-04-04)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-sec v0.10.12, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`ada5a94`](https://github.com/GitoxideLabs/gitoxide/commit/ada5a9447dc3c210afbd8866fe939c3f3a024226))
    - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
    - Update changelogs prior to release ([`38dff41`](https://github.com/GitoxideLabs/gitoxide/commit/38dff41d09b6841ff52435464e77cd012dce7645))
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Use raw literals for more strings with backslashes ([`01bd76d`](https://github.com/GitoxideLabs/gitoxide/commit/01bd76dcacb69d9c21f2fc6063e273a01aebf94f))
    - Merge pull request #1885 from stagnation/patch-1 ([`6d5f774`](https://github.com/GitoxideLabs/gitoxide/commit/6d5f77413e63cad45415d0f674c58d41091bffa1))
    - Fix typo ([`dbf3b02`](https://github.com/GitoxideLabs/gitoxide/commit/dbf3b02428c212de312fc8104524ac7bb6a79a79))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.29.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
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
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.28.2 (2024-12-22)

### Bug Fixes

 - <csr-id-530257f718d8abe0668eeeed072cc4fee9707cbb/> assure passwords aren't returned in percent-encoded form.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
    - Refactor `gix-url` tests ([`01f66eb`](https://github.com/GitoxideLabs/gitoxide/commit/01f66eba9c827599942d39bcbf72fc07c1bf7890))
    - Assure passwords aren't returned in percent-encoded form. ([`530257f`](https://github.com/GitoxideLabs/gitoxide/commit/530257f718d8abe0668eeeed072cc4fee9707cbb))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.28.1 (2024-11-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0 ([`4000197`](https://github.com/GitoxideLabs/gitoxide/commit/4000197ecc8cf1a5d79361620e4c114f86476703))
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1687 from EliahKagan/run-ci/32bit ([`aeaebec`](https://github.com/GitoxideLabs/gitoxide/commit/aeaebec7b1e07ce94429987c4f2466799c24cb67))
    - Fix gix-url baseline script instead of taking ownership ([`d07c32d`](https://github.com/GitoxideLabs/gitoxide/commit/d07c32d2795e964b6296a4d6dba9b8b197438bb6))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.28.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>
<csr-id-a794e9f4a5741a0e23cc380a4401c79e34dade9f/>
<csr-id-642be47ad1fbf688c2f9e58908c69d2d3a2920ab/>

### Bug Fixes

 - <csr-id-cdac4a9b04959b0fc71009b5c828cdcb10a38828/> remove workspace lints from Cargo manifests of fuzz-projects.
   They are not part of the parent workspace.
 - <csr-id-4e16a09059b999591123b2711d3a8b68665eafd4/> Require the most recent version of `url`
   This seems to be more compatible to nightly versions of the compiler.

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
 - <csr-id-a794e9f4a5741a0e23cc380a4401c79e34dade9f/> remove `home` from dependencies
 - <csr-id-642be47ad1fbf688c2f9e58908c69d2d3a2920ab/> use `gix_path::env::home_dir` instead of `home::home_dir`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 60 calendar days.
 - 61 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1593 from Byron/fix-fuzz ([`72daa46`](https://github.com/GitoxideLabs/gitoxide/commit/72daa46bad9d397ef2cc48a3cffda23f414ccd8a))
    - Remove workspace lints from Cargo manifests of fuzz-projects. ([`cdac4a9`](https://github.com/GitoxideLabs/gitoxide/commit/cdac4a9b04959b0fc71009b5c828cdcb10a38828))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1549 from ChanTsune/gix-url-wasi ([`db147b9`](https://github.com/GitoxideLabs/gitoxide/commit/db147b9176581e8981cf7f2f9e4f2e436f81816a))
    - Merge branch 'patch-2' ([`a64d94e`](https://github.com/GitoxideLabs/gitoxide/commit/a64d94e9b944f16b87d572e887bde5dec091d17d))
    - Remove `home` from dependencies ([`a794e9f`](https://github.com/GitoxideLabs/gitoxide/commit/a794e9f4a5741a0e23cc380a4401c79e34dade9f))
    - Use `gix_path::env::home_dir` instead of `home::home_dir` ([`642be47`](https://github.com/GitoxideLabs/gitoxide/commit/642be47ad1fbf688c2f9e58908c69d2d3a2920ab))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Require the most recent version of `url` ([`4e16a09`](https://github.com/GitoxideLabs/gitoxide/commit/4e16a09059b999591123b2711d3a8b68665eafd4))
</details>

## 0.27.5 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 20 calendar days.
 - 30 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-attributes v0.22.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`f2b522d`](https://github.com/GitoxideLabs/gitoxide/commit/f2b522df2ddad07f065f43c2dbad49aa726714dd))
    - Release gix-glob v0.16.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`a65a17f`](https://github.com/GitoxideLabs/gitoxide/commit/a65a17fc396ef49663b0a75cf7b5502d370db269))
    - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
    - Prepare changelogs prior to release ([`0f25841`](https://github.com/GitoxideLabs/gitoxide/commit/0f2584178ae88e425f1c629eb85b69f3b4310d9f))
    - Merge branch 'improvements' ([`12313f2`](https://github.com/GitoxideLabs/gitoxide/commit/12313f2720bb509cb8fa5d7033560823beafb91c))
    - Thanks clippy ([`ae2b733`](https://github.com/GitoxideLabs/gitoxide/commit/ae2b733c1e4de33ddf442cd8044dae364307085d))
</details>

## 0.27.4 (2024-07-23)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 101 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.31.5, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`6232824`](https://github.com/GitoxideLabs/gitoxide/commit/6232824301847a9786dea0b926796a3187493587))
    - Release gix-glob v0.16.4, gix-attributes v0.22.3, gix-command v0.3.8, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`a1b73a6`](https://github.com/GitoxideLabs/gitoxide/commit/a1b73a67c19d9102a2c5a7f574a7a53a86d0094c))
    - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
    - Prepare changelog prior to release ([`99c00cc`](https://github.com/GitoxideLabs/gitoxide/commit/99c00cc3ae9827555e2e1162328bc57038619d1f))
    - Release gix-path v0.10.9 ([`15f1cf7`](https://github.com/GitoxideLabs/gitoxide/commit/15f1cf76764221d14afa66d03a6528b19b9c30c9))
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'fix-url-parse' ([`34eb0c9`](https://github.com/GitoxideLabs/gitoxide/commit/34eb0c9af4ea0d0ccc2cfdcce1dcbfceaa024f7f))
    - Reproduce url parsing issue discovered by cluserfuzz. ([`594b488`](https://github.com/GitoxideLabs/gitoxide/commit/594b4886c4af04fd8b7654e4769f47b91acc90a9))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
</details>

## 0.27.3 (2024-04-13)

### New Features

<csr-id-db40382328c373258aa3bd5f9551511a42af6be5/>

 - <csr-id-545799882643532dd0b3e8ab436efd7722c74e3c/> Add `ArgumentSafety` and `Url::*_as_argument()` methods
   This adds three methods to `Url`:
   
   - `Url::user_as_argument`
1. There is no wrapped value (`Absent`).
2. The wrapped value is usable and presumably safe (`Usable`).
3. The wrapped value is dangerous and should not be passed as a
      command-line argument because it could be interpreted as an
      option due to starting with `-`. The value itself may still be
      useful and safe to include in error messages.

### Bug Fixes

 - <csr-id-f56ad390a5569d0129b7b16632991d18b9ddb4f7/> Prevent usernames with leading `-` from being passed to SSH
   This detects ambiguous usernames in dangerous cases where they
   would be passed to external commands to form SSH connections, if
   they would be misinterpreted as option arguments.
   
   This change is analogous to b06a0dd, hardening `gix-transport` and
   applications that use it against options smuggled in URLs, but for
   the non-mandatory username portion of a URL, rather than the host
   and path portions that were covered there.
   
   For example, commands like these no longer pass `-F...` options to
   `ssh`:
   
   gix clone 'ssh://-Fconfigfile@example.com/abc'
   gix clone -- '-Fconfigfile@example.com:abc/def'
   
   Instead, they refuse to run `ssh`, producing the error:
   
   Error: Username '-Fconfigfile' could be mistaken for a command-line argument

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
    - Merge branch 'strange-usernames' ([`1272542`](https://github.com/GitoxideLabs/gitoxide/commit/1272542e79c29302ada47324d6bb02101393563d))
    - Refactor `gix-url` ([`09311b0`](https://github.com/GitoxideLabs/gitoxide/commit/09311b0e0039c5a82c871047ff24336ab1741d47))
    - Give `ArgumentSafety` traits; test `*_as_argument` methods ([`1b0af07`](https://github.com/GitoxideLabs/gitoxide/commit/1b0af07ffa122a2199d444b84512300ef578abb5))
    - Add `ArgumentSafety` and `Url::*_as_argument()` methods ([`5457998`](https://github.com/GitoxideLabs/gitoxide/commit/545799882643532dd0b3e8ab436efd7722c74e3c))
    - Sketch *_as_argument methods and supporting enum ([`29941e6`](https://github.com/GitoxideLabs/gitoxide/commit/29941e66b367ff1eac448ec4024b1a17246ffaac))
    - Give `looks_like_argument` a more accurate name ([`beef8d2`](https://github.com/GitoxideLabs/gitoxide/commit/beef8d2ce20af8b97fcd85c185cf9373d4897cdb))
    - Prevent usernames with leading `-` from being passed to SSH ([`f56ad39`](https://github.com/GitoxideLabs/gitoxide/commit/f56ad390a5569d0129b7b16632991d18b9ddb4f7))
    - Clarify circumstances when *_argument_safe should be used ([`e8e2463`](https://github.com/GitoxideLabs/gitoxide/commit/e8e2463ad1e2c3b9f604b744d554cfa6684eca2f))
    - Expand and introduce tests of more combinations ([`169a698`](https://github.com/GitoxideLabs/gitoxide/commit/169a698ce82df559253c0389a6fa6921a136bc9e))
    - Add `Url::user_argument_safe()` ([`db40382`](https://github.com/GitoxideLabs/gitoxide/commit/db40382328c373258aa3bd5f9551511a42af6be5))
    - Prepare for adding Url::user_argument_safe ([`1bdfdd9`](https://github.com/GitoxideLabs/gitoxide/commit/1bdfdd98c9a82eee7ac40b38fe05d54f1f93237e))
    - Reorder tests and add username assertion placeholders ([`6cbe65d`](https://github.com/GitoxideLabs/gitoxide/commit/6cbe65d3f25c5c00fb9248a5558ac74f6e77ff06))
</details>

## 0.27.2 (2024-03-14)

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

## 0.27.1 (2024-02-25)

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
    - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
    - Adjust gitignore files with precious declarations ([`ae86a6a`](https://github.com/GitoxideLabs/gitoxide/commit/ae86a6a206074b85ff1eba32aea9c8b40c087b17))
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
</details>

## 0.27.0 (2024-01-20)

### New Features

 - <csr-id-ebca00abeb85f7f0a4c574cc65ade7e038dc94c2/> add `std::fmt::Display` to `Url` type.
   That way it's possible to have a more safe display implementation
   that tries to not spill obvious secrets, like the password of a
   URL.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 17 calendar days.
 - 20 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge branch 'url-fix' ([`0e579a0`](https://github.com/GitoxideLabs/gitoxide/commit/0e579a05c8daba09a967a9ea694d4c15ff003a58))
    - Enabled previously disabled test as it works with the latest version of `url` ([`3c060cb`](https://github.com/GitoxideLabs/gitoxide/commit/3c060cb7cab720a26019c53b255db0f66d8d40b4))
    - Add `std::fmt::Display` to `Url` type. ([`ebca00a`](https://github.com/GitoxideLabs/gitoxide/commit/ebca00abeb85f7f0a4c574cc65ade7e038dc94c2))
</details>

## 0.26.1 (2023-12-30)

<csr-id-3bd09ef120945a9669321ea856db4079a5dab930/>

### Chore

- <csr-id-3bd09ef120945a9669321ea856db4079a5dab930/> change `rust-version` manifest field back to 1.65.
  They didn't actually need to be higher to work, and changing them
  unecessarily can break downstream CI.

  Let's keep this value as low as possible, and only increase it when
  more recent features are actually used.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.3, gix-hash v0.14.1, gix-trace v0.1.6, gix-features v0.37.1, gix-actor v0.29.1, gix-validate v0.8.3, gix-object v0.40.1, gix-path v0.10.3, gix-glob v0.15.1, gix-quote v0.4.10, gix-attributes v0.21.1, gix-command v0.3.2, gix-packetline-blocking v0.17.2, gix-utils v0.1.8, gix-filter v0.8.1, gix-fs v0.9.1, gix-chunk v0.4.7, gix-commitgraph v0.23.1, gix-hashtable v0.5.1, gix-revwalk v0.11.1, gix-traverse v0.36.1, gix-worktree-stream v0.8.1, gix-archive v0.8.1, gix-config-value v0.14.3, gix-tempfile v12.0.1, gix-lock v12.0.1, gix-ref v0.40.1, gix-sec v0.10.3, gix-config v0.33.1, gix-prompt v0.8.2, gix-url v0.26.1, gix-credentials v0.23.1, gix-ignore v0.10.1, gix-bitmap v0.2.10, gix-index v0.28.1, gix-worktree v0.29.1, gix-diff v0.39.1, gix-discover v0.28.1, gix-macros v0.1.3, gix-mailmap v0.21.1, gix-negotiate v0.11.1, gix-pack v0.46.1, gix-odb v0.56.1, gix-pathspec v0.5.1, gix-packetline v0.17.2, gix-transport v0.40.1, gix-protocol v0.43.1, gix-revision v0.25.1, gix-refspec v0.21.1, gix-status v0.4.1, gix-submodule v0.7.1, gix-worktree-state v0.6.1, gix v0.57.1 ([`972241f`](https://github.com/GitoxideLabs/gitoxide/commit/972241f1904944e8b6e84c6aa1649a49be7a85c3))
    - Merge branch 'msrv' ([`8c492d7`](https://github.com/GitoxideLabs/gitoxide/commit/8c492d7b7e6e5d520b1e3ffeb489eeb88266aa75))
    - Virtually downgrade `home` to pass MRSV checks ([`e6706f2`](https://github.com/GitoxideLabs/gitoxide/commit/e6706f2b897ace40b0a1dcae1986bebb664f3b7e))
    - Change `rust-version` manifest field back to 1.65. ([`3bd09ef`](https://github.com/GitoxideLabs/gitoxide/commit/3bd09ef120945a9669321ea856db4079a5dab930))
</details>

## 0.26.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 19 calendar days.
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
    - Merge pull request #1209 from silvergasp/fuzz-gix-url ([`5e84453`](https://github.com/GitoxideLabs/gitoxide/commit/5e844533553352ecb4c345f24ff32da89c286b6f))
    - Add fuzzing assert to fuzz against CVE-2017-1000117 ([`8d4f9d7`](https://github.com/GitoxideLabs/gitoxide/commit/8d4f9d7359344f95d6631f7ac53fe54a40beb5f1))
    - Fuzz more of gix_url::Url ([`19e1b7b`](https://github.com/GitoxideLabs/gitoxide/commit/19e1b7b4320dd9da47705d3cbcdbb571c4339d30))
    - Merge branch 'gix-glob-fix' ([`c1e4c62`](https://github.com/GitoxideLabs/gitoxide/commit/c1e4c629c32f136d1bc6159d47c0a9cda1fffd19))
    - Make CI runs more robust by changing timings ([`dab926d`](https://github.com/GitoxideLabs/gitoxide/commit/dab926d39e3603d21ff4f9f57aa351e04d39304a))
    - Merge branch 'fuzz-gix-ref' ([`8ee1194`](https://github.com/GitoxideLabs/gitoxide/commit/8ee119442b2ff4b846a34b57a222a6abdf6c95d8))
    - Let's have the latest versions of fuzzers automatically ([`5828f37`](https://github.com/GitoxideLabs/gitoxide/commit/5828f3796031de8d9f50bd6536697754da0b3d8c))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Upgrade `url` to latest version. ([`58ed530`](https://github.com/GitoxideLabs/gitoxide/commit/58ed530da0eed05b6b5b97d4941a53e06b679918))
</details>

## 0.25.2 (2023-12-06)

### Bug Fixes

 - <csr-id-ee47fbab9ff995bc76a4c75185fe8e6e7c1d4ba3/> specify minimum required `url` version of v2.2.0.
   Note that this is also the minimal required version that is resolved
   with `cargo +nightly update -Z minimal-versions`, but it's nothing
   I could validate or reproduce myself just yet.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1082](https://github.com/GitoxideLabs/gitoxide/issues/1082), [#1119](https://github.com/GitoxideLabs/gitoxide/issues/1119)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1082](https://github.com/GitoxideLabs/gitoxide/issues/1082)**
    - Add new test-case to reproduce gix-url failure in debug mode only ([`d78f445`](https://github.com/GitoxideLabs/gitoxide/commit/d78f445449e534bbd858a8db1a16ad837417a78f))
 * **[#1119](https://github.com/GitoxideLabs/gitoxide/issues/1119)**
    - Specify minimum required `url` version of v2.2.0. ([`ee47fba`](https://github.com/GitoxideLabs/gitoxide/commit/ee47fbab9ff995bc76a4c75185fe8e6e7c1d4ba3))
 * **Uncategorized**
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Add another failing gix-url test-case found by the fuzzer. ([`9158ffc`](https://github.com/GitoxideLabs/gitoxide/commit/9158ffc24661e316327063705194c1fee3d9984c))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
    - Merge branch 'url-set-password' ([`650461c`](https://github.com/GitoxideLabs/gitoxide/commit/650461c1430e6e4b3697c5d1dcdaeaa77d5005d7))
    - Add test for password and user access ([`6b10245`](https://github.com/GitoxideLabs/gitoxide/commit/6b10245ace7eea1cf09f2bf4a1aed9733f38e2d5))
    - Allow changing URL password. ([`4ba43ac`](https://github.com/GitoxideLabs/gitoxide/commit/4ba43ac85355616ff110c243fb2dbfd34b45c289))
    - Merge branch 'fuzz' ([`c5a7e66`](https://github.com/GitoxideLabs/gitoxide/commit/c5a7e66d901868237ef5a4f86534b9878cc397ff))
    - Add a new failure case which doesn't reproduce ([`bc82f8f`](https://github.com/GitoxideLabs/gitoxide/commit/bc82f8fc4d6a06af94c5bfc0c4d19caa3f1eb805))
</details>

## 0.25.1 (2023-10-14)

### Bug Fixes

 - <csr-id-562b0c931db1d22b8e59903cb59afa9a36d5884c/> make file:// url parsing with full backslashed path more robust on windows.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1063](https://github.com/GitoxideLabs/gitoxide/issues/1063)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1063](https://github.com/GitoxideLabs/gitoxide/issues/1063)**
    - Make file:// url parsing with full backslashed path more robust on windows. ([`562b0c9`](https://github.com/GitoxideLabs/gitoxide/commit/562b0c931db1d22b8e59903cb59afa9a36d5884c))
 * **Uncategorized**
    - Release gix-url v0.25.1 ([`47a1241`](https://github.com/GitoxideLabs/gitoxide/commit/47a1241484fdb424184ca37f85a8b287d374d2a1))
    - Merge branch 'head-conversions' ([`c2cf20c`](https://github.com/GitoxideLabs/gitoxide/commit/c2cf20cd2d685c2c24527729fff35fd0a7903742))
    - Fix yet another fuzzed test-case related to url::parse DoS ([`21729ed`](https://github.com/GitoxideLabs/gitoxide/commit/21729edb0c12831eb9ea488cc3b66b5e79eacfae))
</details>

## 0.25.0 (2023-10-12)

This release contains a complete rewrite of the internal url parsing logic, the public interface stays mostly the same however. Gitoxide will now be
more correct, interpreting more urls the same way Git does. Improvements include the added support for ssh aliases (`github:byron/gitoxide` has previously
been parsed as local path), adjustments around the interpretation of colons in file names (previously we disallowed colons that were not followed up
with a slash character) and some smaller changes that bring the interpretation of file urls more in line with Git's implementation. Additionally, the
error types have been adjusted to print a more comprehensive message by default, making sure they stay helpful even when bubbled up through multiple abstraction
layers.

There are still many (edge) cases in Git's url parsing implementation which are not handled correctly by Gitoxide. If you notice any such deviation please
open a new issue to help us making Gitoxide even more correct.

### Bug Fixes

 - <csr-id-ea0ea88dfc232601b462fcf52fed4d34a17bc116/> another fuzz-issue that could cause long parse times of URLs
 - <csr-id-60126d7097280bf364fb83ef73588df414198855/> denial of service attack by passing a URL with a very long host.
   We now check for certain size limits and prevent passing long URLs to
   the `url` crate.

### New Features

 - <csr-id-4184a5e9019f2ac20213b5362d60c71e0bf295d3/> enable fuzzing for git url parsing

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Another fuzz-issue that could cause long parse times of URLs ([`ea0ea88`](https://github.com/GitoxideLabs/gitoxide/commit/ea0ea88dfc232601b462fcf52fed4d34a17bc116))
    - Add yet another bypass attack that runs into the `url` DoS issue ([`6aa63b1`](https://github.com/GitoxideLabs/gitoxide/commit/6aa63b19338d1b4d86906a1e16eeb7b48cbc83a4))
    - Merge branch 'improvements' ([`3939a45`](https://github.com/GitoxideLabs/gitoxide/commit/3939a455be2269280248cdfed4a5983f8d178141))
    - Assure we don't accidentally parse a valid-looking URL to `url` and cause long compute times. ([`7497553`](https://github.com/GitoxideLabs/gitoxide/commit/7497553e753dcff92cc7c78941d8bbef8b97bdb7))
    - Merge branch 'gix-url-fixture-improvements' ([`3d60c02`](https://github.com/GitoxideLabs/gitoxide/commit/3d60c0245ec4b787cfcb111319d730a6e5031ef4))
    - Fix panics not shown in gix-url baseline generation ([`2488ad9`](https://github.com/GitoxideLabs/gitoxide/commit/2488ad9f5ca2913a1a67f40b91291463602a3f21))
    - Improve output of the gix-url baseline test ([`a530037`](https://github.com/GitoxideLabs/gitoxide/commit/a5300378899d205ce96f53581b98d3149e1e8c66))
    - Denial of service attack by passing a URL with a very long host. ([`60126d7`](https://github.com/GitoxideLabs/gitoxide/commit/60126d7097280bf364fb83ef73588df414198855))
    - Merge branch 'gix-url-parse-rewrite' ([`a12e4a8`](https://github.com/GitoxideLabs/gitoxide/commit/a12e4a88d5f5636cd694c72ce45a8b75aa754d28))
    - Enable fuzzing for git url parsing ([`4184a5e`](https://github.com/GitoxideLabs/gitoxide/commit/4184a5e9019f2ac20213b5362d60c71e0bf295d3))
    - Assure we don't loose test coverage; possibly adjust expecations ([`30bb7dc`](https://github.com/GitoxideLabs/gitoxide/commit/30bb7dc27f4b5def6abb91adc81ff49a1b935cff))
    - Refactor ([`e318a4c`](https://github.com/GitoxideLabs/gitoxide/commit/e318a4c243f67807bee55ea4571dfb1c068f1d09))
    - Refactor baseline tests ([`4b4ac8a`](https://github.com/GitoxideLabs/gitoxide/commit/4b4ac8abee65a27f8677ef25f4f127bf6f6416b7))
    - Add platform specific baseline tests and run always run them. ([`e9aa690`](https://github.com/GitoxideLabs/gitoxide/commit/e9aa690d7c0692e5476c2065feb31b0c75e81128))
    - Update changelogs ([`4349353`](https://github.com/GitoxideLabs/gitoxide/commit/43493531bbf3049bee3d7b14b7a6dbe874e37ebc))
    - Align test with real behavior ([`a31af62`](https://github.com/GitoxideLabs/gitoxide/commit/a31af624f88f80dab0a3f2ccd36502bb494ab046))
    - Fix absolute windows file urls with extra slash ([`3bf12a3`](https://github.com/GitoxideLabs/gitoxide/commit/3bf12a3e9bda6ebe0f8942d9b3f1f7c3e357b435))
</details>

## 0.24.0 (2023-09-24)

### New Features

 - <csr-id-d80b5f69772a6e36b0131d3a538e896a8a6a29b1/> add `Url::host_argument_safe()` and `Url::path_argument_safe()`
   This will not provide values if they could be confused for an argument
   to to a commaneline application.

### Bug Fixes

 - <csr-id-b06a0dd781accad317fdec5f86f069df4c21875c/> prevent hosts or paths that look like arguments to be passed to invoked commands.
   See https://secure.phabricator.com/T12961 for more details.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 14 calendar days.
 - 16 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
    - Merge branch 'fix-exploit' ([`c53bbd2`](https://github.com/GitoxideLabs/gitoxide/commit/c53bbd265005c7eedc316205b217e137e2b9896e))
    - Prevent hosts or paths that look like arguments to be passed to invoked commands. ([`b06a0dd`](https://github.com/GitoxideLabs/gitoxide/commit/b06a0dd781accad317fdec5f86f069df4c21875c))
    - Add `Url::host_argument_safe()` and `Url::path_argument_safe()` ([`d80b5f6`](https://github.com/GitoxideLabs/gitoxide/commit/d80b5f69772a6e36b0131d3a538e896a8a6a29b1))
    - Parse absolute paths in file urls on Windows ([`637e1ae`](https://github.com/GitoxideLabs/gitoxide/commit/637e1ae996610b28812cf27efe13777666b44cc9))
    - Parse absolute paths as local urls on Windows ([`3af8834`](https://github.com/GitoxideLabs/gitoxide/commit/3af8834a4a6ee33473d1690fd98b40b22d41a55d))
    - Create test urls from raw parts directly ([`5ac705c`](https://github.com/GitoxideLabs/gitoxide/commit/5ac705cfaf8932469115f25b54a47f444c163bc0))
    - Fix file urls with dos driver letter on unix ([`99f4447`](https://github.com/GitoxideLabs/gitoxide/commit/99f444799d44bbe54ac05307b79da6add9181b8b))
    - Fix dropping of host during file url conversion ([`f3e1331`](https://github.com/GitoxideLabs/gitoxide/commit/f3e13312c2f0ea50b716189902c8b809a1a98cbc))
    - Use helper function for common url parsing steps ([`2344f91`](https://github.com/GitoxideLabs/gitoxide/commit/2344f913b0539942ae852e2b093558e04c79bb8a))
    - Thanks clippy ([`d077f56`](https://github.com/GitoxideLabs/gitoxide/commit/d077f56aa220d83f46e8d318c7bf48ec77072fdc))
    - Add missing slash to file url test ([`ffaead6`](https://github.com/GitoxideLabs/gitoxide/commit/ffaead685b49d6c6eebdf5c563d2dc3cdbfb9e13))
    - Remove wrong ssh alias url test ([`4da440d`](https://github.com/GitoxideLabs/gitoxide/commit/4da440d0f81a2fc462b2c04840a83beb7a738344))
</details>

## 0.23.0 (2023-09-08)

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 17 calendar days.
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
    - Handle missing host and path separator in file url ([`f05edae`](https://github.com/GitoxideLabs/gitoxide/commit/f05edaef5e8d6a2d90adc84c8dc5c4b5ab0017b6))
    - Use same logic for file urls and local file paths ([`679a7f4`](https://github.com/GitoxideLabs/gitoxide/commit/679a7f46e8121b5a0a5a396c2ce89f6706068507))
    - Return errors for empty paths and relative urls ([`8af79ec`](https://github.com/GitoxideLabs/gitoxide/commit/8af79ec98d82f3d2542fc43928103a20c2e4fc1a))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.22.0 (2023-08-22)

<csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/>
<csr-id-313a7b3ddc5786af1f120fa99dd659d725b6845a/>

### Chore

- <csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/> don't call crate 'WIP' in manifest anymore.

### Reverted (BREAKING)

 - <csr-id-fb0b8a769a8992e8079c66fea7d4708d9db0323d/> url parsing error NotALocalFile
   The `NotALocalFile` error variant was previously returned if the input
   url contained a colon but the slice after that character contained no
   slash character (`/` or `\`). This behavior is wrong.
   
   SCP like URLs may not need a slash character to specify the repositories
   location. Similarly, a local directory that contains a colon in its name
   is a valid repository path as well.
   
   The new implementation correctly parses such URLs.
 - <csr-id-dd1a1257d66d2f280be0adfce0a100a6a83e1c65/> url parsing error MissingResourceLocation
   The `MissingResourceLocation` error variant was previously returned if
   the input URL had either scheme ssh (no matter if URL or SCP format) or
   git and the URL contained no or an empty path section. In the future we
   will instead return the `MissingRepositoryPath` error in such a case.
   
   The only benefit of such a seperation is that it allows the caller to
   infer the kind of URL we tried to parse. A future commit will add a new
   field to the `MissingRepositoryPath` error variant which will clearly
   communicate this information and can therefore be used instead for this
   purpose.

### Chore (BREAKING)

- <csr-id-313a7b3ddc5786af1f120fa99dd659d725b6845a/> restructure gix-url parsing error variants
  All error variants now own a copy of the input which is used in their
  display implementation. I noticed that URL parsing errors are almost never
  acted upon but only wrapped and bubbled up to the user. It therefore
  makes sense to ensure the message for this error is informative enough
  by default.

  If an error can be thrown for different types of URLs it now also
  includes a kind field. With this field the caller can determine what
  kind of URL we tried to parse. Additionally, this information is used
  for the error message too.

  For testing the assert_matches crate was added which implements the
  unstable feature with the same name in stable rust. It makes testing the
  invalid variants much more convenient.

  BREAKING because public fields of some error variants are now no longer
  available

### Bug Fixes (BREAKING)

 - <csr-id-8857580cb270b737bf04437a6f5e65307df1c99b/> let `Url::canonicalize(d)()` take the current working dir as argument.
   That way it's free of side-effects and can be used properly from `gix::Repository`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release over the course of 17 calendar days.
 - 30 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0 ([`6c62e74`](https://github.com/GitoxideLabs/gitoxide/commit/6c62e748240ac0980fc23fdf30f8477dea8b9bc3))
    - Make `gix-url` publishable by adding baseline test ([`d3746df`](https://github.com/GitoxideLabs/gitoxide/commit/d3746df5dd402d8a461c2b07eaa0f8d8803fadf8))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Restructure gix-url parsing error variants ([`313a7b3`](https://github.com/GitoxideLabs/gitoxide/commit/313a7b3ddc5786af1f120fa99dd659d725b6845a))
    - Url parsing error NotALocalFile ([`fb0b8a7`](https://github.com/GitoxideLabs/gitoxide/commit/fb0b8a769a8992e8079c66fea7d4708d9db0323d))
    - Url parsing error MissingResourceLocation ([`dd1a125`](https://github.com/GitoxideLabs/gitoxide/commit/dd1a1257d66d2f280be0adfce0a100a6a83e1c65))
    - Rewrite gix-url parsing ([`14facd9`](https://github.com/GitoxideLabs/gitoxide/commit/14facd98632b44eb350c33a5aa425d2e68d25889))
    - Merge branch 'gix-url-fixture-tests' ([`1b957c5`](https://github.com/GitoxideLabs/gitoxide/commit/1b957c524cae288ac2063f6ba7e4d10d523eb8f3))
    - For now, ignore baseline tests as most of them fail. ([`c0be6ab`](https://github.com/GitoxideLabs/gitoxide/commit/c0be6aba49d734d166f279933220b02989c584de))
    - Don't call crate 'WIP' in manifest anymore. ([`229bd48`](https://github.com/GitoxideLabs/gitoxide/commit/229bd4899213f749a7cc124aa2b82a1368fba40f))
    - Merge branch 'submodule-active' ([`a3afaa4`](https://github.com/GitoxideLabs/gitoxide/commit/a3afaa42741616a0f1abeef9b54557e7c2b800cb))
    - Add support for "git+ssh" and "ssh+git" urls ([`3faac36`](https://github.com/GitoxideLabs/gitoxide/commit/3faac3648b4d39bff4a05602b481bf8e6d71b77e))
    - Let `Url::canonicalize(d)()` take the current working dir as argument. ([`8857580`](https://github.com/GitoxideLabs/gitoxide/commit/8857580cb270b737bf04437a6f5e65307df1c99b))
    - Fix wrong assertion in gix-url baseline test ([`f18bfd8`](https://github.com/GitoxideLabs/gitoxide/commit/f18bfd8d81a56e9a68d465c49ef2da192de6c279))
    - Improve error printing for gix-url baseline test ([`0e9a18e`](https://github.com/GitoxideLabs/gitoxide/commit/0e9a18ed6921116cca6e48345d6937ad7120e83e))
    - Fix prints of panics in gix-url baseline test ([`938baee`](https://github.com/GitoxideLabs/gitoxide/commit/938baee58383fd995d39c5528e2a104d33de8826))
    - Add assert output to failures in gix-url baseline test ([`163e139`](https://github.com/GitoxideLabs/gitoxide/commit/163e1391526dad528e0bca0cc876e50e48381e74))
    - Thanks clippy ([`fe5dbe1`](https://github.com/GitoxideLabs/gitoxide/commit/fe5dbe16b5a8cfefa31a561506218ede73f3ec7e))
    - Use libtest_mimic to run gix-url fixture tests ([`f128742`](https://github.com/GitoxideLabs/gitoxide/commit/f128742df039cf23529a5e1ee9268a64d904d9e0))
    - Refactor ([`cbaf339`](https://github.com/GitoxideLabs/gitoxide/commit/cbaf3395a702b8f43aa3eac61c1a2b782bf63e05))
    - Fix assertion in gix-url fixture test ([`ca421b7`](https://github.com/GitoxideLabs/gitoxide/commit/ca421b77aeebb8204da1a090fdd59ef5021e177d))
    - Thanks clippy ([`252dbb6`](https://github.com/GitoxideLabs/gitoxide/commit/252dbb65e33188b1b803fbe05ead27aaae6fe5d9))
    - Add parsing and asserting to gix-url fixture test ([`23e2982`](https://github.com/GitoxideLabs/gitoxide/commit/23e2982b731b30a6aa9c55fdde5b8f00e71afc56))
    - Fix bash styling in gix-url fixture ([`c77cd31`](https://github.com/GitoxideLabs/gitoxide/commit/c77cd31834a8ac1540ad54bea59157609466c9eb))
    - Adjust gix-url fixture loop for our test suite ([`f4214a6`](https://github.com/GitoxideLabs/gitoxide/commit/f4214a6163533811a42a3aa104d5ba32c05321f7))
    - Make gix-url fixture more readable ([`1ab3b07`](https://github.com/GitoxideLabs/gitoxide/commit/1ab3b07cdf6543515b9b9be5690e2bb27ec63b7b))
    - Add copy of gits t5500 test in preparation for fixture ([`d0de915`](https://github.com/GitoxideLabs/gitoxide/commit/d0de9153131cbd30cec694cfdfb54e4dde7e8d45))
</details>

## 0.21.1 (2023-07-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v7.0.2, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0 ([`107a64e`](https://github.com/GitoxideLabs/gitoxide/commit/107a64e734580ad9e2c4142db96394529d8072df))
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Prepare more changelogs ([`c4cc5f2`](https://github.com/GitoxideLabs/gitoxide/commit/c4cc5f261d29f712a101033a18293a97a9d4ae85))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.21.0 (2023-07-19)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
</details>

## 0.20.1 (2023-06-29)

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
    - Release gix-glob v0.9.1, gix-attributes v0.14.1, gix-config-value v0.12.3, gix-ref v0.32.1, gix-sec v0.8.3, gix-config v0.25.1, gix-url v0.20.1, gix-credentials v0.16.1, gix-discover v0.21.1, gix-ignore v0.4.1, gix-pack v0.39.1, gix-odb v0.49.1, gix-worktree v0.21.1, gix v0.48.0 ([`69c6a36`](https://github.com/GitoxideLabs/gitoxide/commit/69c6a36ba14cbef129deebda9fd8870005fefa17))
    - Prepare changelogs prior to release ([`c143cf4`](https://github.com/GitoxideLabs/gitoxide/commit/c143cf48ee1885467e3e9262a3f8823a1247bfe0))
    - Align usage of `gix-path` across all crates ([`73c1292`](https://github.com/GitoxideLabs/gitoxide/commit/73c1292be393986c4a1adde1400abf551e850da0))
</details>

## 0.20.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 10 calendar days.
 - 15 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
</details>

## 0.19.0 (2023-06-06)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 12 calendar days.
 - 40 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
</details>

## 0.18.0 (2023-04-27)

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

## 0.17.0 (2023-04-26)

### New Features (BREAKING)

 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#814](https://github.com/GitoxideLabs/gitoxide/issues/814)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
    - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
 * **Uncategorized**
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
</details>

## 0.16.0 (2023-03-10)

### New Features (BREAKING)

 - <csr-id-c6897d95febf40467c780c3827792135b70a7e25/> support passwords in urls.
   It's now possible to use urls like `https://user:pass@host/repo` without loosing
   the password portion of the URL.
   
   We also change the `from_parts()` method to take all parts needed to describe a URL,
   which is a breaking change.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v5.0.0, gix-lock v5.0.0, gix-ref v0.27.0, gix-config v0.19.0, gix-url v0.16.0, gix-credentials v0.12.0, gix-discover v0.16.0, gix-index v0.15.0, gix-pack v0.33.0, gix-odb v0.43.0, gix-transport v0.28.0, gix-protocol v0.29.0, gix-worktree v0.15.0, gix v0.41.0, safety bump 12 crates ([`29a0870`](https://github.com/GitoxideLabs/gitoxide/commit/29a087043d1feb2f127b065341c8028d0bd0301e))
    - Prepare changelogs prior to release ([`e06f5f5`](https://github.com/GitoxideLabs/gitoxide/commit/e06f5f523e83f4da390eddbebcb9a2d58674587b))
    - Merge branch 'password-in-urls' ([`85f8b28`](https://github.com/GitoxideLabs/gitoxide/commit/85f8b283a1671e2631cda437ca8da93f9a2a4ebd))
    - Support passwords in urls. ([`c6897d9`](https://github.com/GitoxideLabs/gitoxide/commit/c6897d95febf40467c780c3827792135b70a7e25))
</details>

## 0.15.0 (2023-03-04)

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

## 0.14.0 (2023-03-01)

### Bug Fixes

 - <csr-id-6d518496330ccaa00c1022fcdd2a9324cf218871/> improve error message spelling

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 2 calendar days.
 - 8 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
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
    - Improve error message spelling ([`6d51849`](https://github.com/GitoxideLabs/gitoxide/commit/6d518496330ccaa00c1022fcdd2a9324cf218871))
</details>

## 0.13.3 (2023-02-20)

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

## 0.13.2 (2023-02-17)

<csr-id-098f802e6dc9f55632791ddf8d046563f75cba7a/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>

### Bug Fixes (BREAKING)

 - <csr-id-2da6c862e184ac37d59147e9cf809017b65db966/> make Scheme work with serde, removing `Copy` in the process.  e#450)
   This wasn't supposed to happen but a requirement to get `serde` support
   back.
 - <csr-id-2bcfdee6a3af758a0b70e2af9c4b6f8cc09d8da0/> Prohibit invalid state by making parts the url's data private
   This fix is meant to improve serialization support which can now happen
   `to_bstring()` without possibility for error.
   
   Empty paths can still be set which won't be valid for all URLs.

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
 - <csr-id-96a265cc67ea787ed28adde2c5d0a07babf64c9e/> generalize extension schemes.
   Previously this was hard-coded to `radicle`, now it's just an extension
   scheme along with a statically known string. This means we have to
   explicitly support new formats which should be fine.

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`
 - <csr-id-653ebc52f97116e9c72e985eda0d76f566e8c74d/> Introduce `parse(&BStr)` (previously it took `&[u8]`)
   A `&BStr` better indicates that we are expecting human-readable input
   with ascii-compatible or UTF-8 endcoding.
 - <csr-id-f6506e0c463bdccbcfd9324bc312da9cc957d8e6/> Use `&BStr` as input instead of `[u8]`
   This signals that it's indeed intended to be human readable while
   allowing it to be a path as well without loss, at least theoretically.
   After all we currently don't have a way to parse invalid UTF-8.
 - <csr-id-79ab4aeb8206a5f32735891336d7745e046bbea1/> remove `impl std::fmt::Display for Url` as it's not lossless.
 - <csr-id-ffc4a85b9a914b685d7ab528b30f2a3eefb44094/> `From<&[u8]>` is now `From<&BStr>`
   This better represents the meaning of the input, and simplifies
   interactions with `git-config`.

### Other

- <csr-id-098f802e6dc9f55632791ddf8d046563f75cba7a/> try for leaner tests, but it does the opposite kind of :D

### New Features

 - <csr-id-61d89f586a0ad913fc2f502520282520a5e1fd15/> collect ssh-specific options to control how the ssh program is invoked.
   These are passed through when creating the ssh transport.
 - <csr-id-01f25744bba45a5f8a8615734a5beeacd29d1c4e/> add `Url::canonicalized()` and `Url::canonicalize()`.
   These methods allow to assure file urls are absolute, useful when
   cloning from any url.
 - <csr-id-22d3b37ea6239170a478b859361a7d1d7ba01a9a/> `Url::try_from(path: &std::path::Path)` for more convenient instantiation.
 - <csr-id-39ce98ba9a427b8cea1b843f333c2e7de300499c/> (mostly) lossless roundtripping of scp-like urls.
   Previously `git@host:path` would turn into `ssh://git@host/path`,
   which now remains exactly as is.
 - <csr-id-58a6000d669acd33bad91509eaa469f041f119e5/> lossless serialization of file urls.
   Previously a url like `/path/to/repo` would serialize to
   `file:///path/to/repo`, preventing round-trips.
   
   Now it serializes like it was parsed. This also means that
   `file://path` still serializes as `file://path`.
 - <csr-id-7484db5d36383de450de31b4c94c01bc4c237ce4/> `Url::port_or_default()` to fill in default numbers for ports if possible.
 - <csr-id-fbe75c9457708b95dd833e00afa2dcc1db677167/> `Url::path_is_root()` to determine if the path is `/`.
   This could also be considered an empty path depending on the context
   which is what makes it useful.
 - <csr-id-d40f6e1f34eb3f4664caec36727bf0aa3a396a33/> `Scheme::try_from(&str)`
 - <csr-id-7a1769009d68d14a134f368f93245abab0fb41dd/> `TryFrom<&OsStr>` to allow direct usage of `Url` in `clap`.
   In `clap_derive`, this needs
   `parse(try_from_os_str = std::convert::TryFrom::try_from)`.
 - <csr-id-b7a5f7a3b5cf058f503cc18d18fc75356ab98955/> `TryFrom<PathBuf>` which is useful if urls are obtained from the command-line.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-a67fc26b80e5d1183ddc5c6598396214f3e19945/> more conversions for `TryFrom`: `String` and `&str`
 - <csr-id-833899dce120d26a2bbe04d07fc4c71455eb3afe/> `Url::write_to(out)` to write itself more flexibly.
 - <csr-id-5f707c7e99c70ab9683d55c396e8dc11e1d3b0ea/> Add `Url::to_bstring()` for lossless but fallible bstring conversion.

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### Bug Fixes

 - <csr-id-372cea13ad8abe1c47ed7bc806b42292a8ebfae0/> correctly parse scp-like ssh urls with alias as such.
   Previously it would falsely consider it a file path, leading to
   the inability to use the URL at all.
   
   Now the initial scheme detection has been improved to not rely on
   the '.' in host names exclusively anymore.
 - <csr-id-1058330adcc3262c59d30a0b8854fade20ffc3d5/> properly set default SSH port to 22
 - <csr-id-f20f2728ee78d90510e27769a61ead405c4018c1/> scp-like URLs should preserve relative and home-relative paths
 - <csr-id-302a2d866692a541e01d112b6870aa22fcdbe32b/> reject empty paths where needed, add `Url::from_parts_as_alternative_form()`.
   The new constructor allows to create URLs that represent paths which otherwise couldn't
   be valid URLs.
 - <csr-id-3e3aff9f2f427d030a38fe147c5252d7bfd45109/> make sure that `file:..` isn't considered a valid file url.
 - <csr-id-d6f90beac37866f992a1714d38e5b320eea6f1bb/> handle `file:///C:/foo/bar` urls correctly on windows, as paths now are `C:\\foo\bar`.
   These paths are created when using the `url::Url::from_file_path()`
   family of methods, which adds an extra slash at the beginning of a
   windows path which makes it invalid there unless there is further
   processing.
   
   This is now applied by using `url` features, making this case work
   specifically. Note that all other attributes are still the same
   and `git-url` generally tries to keep paths in tact to be a hybrid
   of type that can handle any file system paths as well as actual urls.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 277 commits contributed to the release.
 - 34 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#333](https://github.com/GitoxideLabs/gitoxide/issues/333), [#364](https://github.com/GitoxideLabs/gitoxide/issues/364), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#524](https://github.com/GitoxideLabs/gitoxide/issues/524), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691), [#725](https://github.com/GitoxideLabs/gitoxide/issues/725), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 8 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Adjust all changelogs to fulfil requirements for publishing ([`04b9ca0`](https://github.com/GitoxideLabs/gitoxide/commit/04b9ca025a1667529b2221ab4280bd3c8dae01cf))
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Respect release-wide ignore list to allow removing entire conventional headlines ([`145103d`](https://github.com/GitoxideLabs/gitoxide/commit/145103d4aa715386da9d4953f7f85fadc49fff9a))
    - Only write headlines that we can parse back… ([`d44369a`](https://github.com/GitoxideLabs/gitoxide/commit/d44369ab5d849720dda9a9c0edc1ba1a3c1a78b5))
    - Rebuild all changelogs to assure properly ordered headlines ([`4a9a05f`](https://github.com/GitoxideLabs/gitoxide/commit/4a9a05f95930bad5938d4ce9c517ebf0e0b990f1))
    - Sort all commits by time, descending… ([`f536bad`](https://github.com/GitoxideLabs/gitoxide/commit/f536bad20ffbac4dc353dfeb1a917bb88becbb78))
    - Greatly reduce changelog size now that the traversal fix is applied ([`a0bc98c`](https://github.com/GitoxideLabs/gitoxide/commit/a0bc98c06c349de2fd6e0d4593606e68b98def72))
    - Fixup remaining changelogs… ([`2f75db2`](https://github.com/GitoxideLabs/gitoxide/commit/2f75db294fcf20c325555822f65629611be52971))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
    - Adapt to changes in git-path ([`cc2d810`](https://github.com/GitoxideLabs/gitoxide/commit/cc2d81012d107da7a61bf4de5b28342dea5083b7))
    - Use `git-path` crate instead of `git_features::path` ([`47e607d`](https://github.com/GitoxideLabs/gitoxide/commit/47e607dc256a43a3411406c645eb7ff04239dd3a))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - Document all features related to serde1 ([`72b97f2`](https://github.com/GitoxideLabs/gitoxide/commit/72b97f2ae4dc7642b160f183c6d5df4502dc186f))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Adapt to changes in git_features::path to deal with Result ([`bba4c68`](https://github.com/GitoxideLabs/gitoxide/commit/bba4c680c627a418efbd25f14bd168df19b8dedd))
 * **[#333](https://github.com/GitoxideLabs/gitoxide/issues/333)**
    - Use git_features::path everywhere where there is a path conversion ([`2e1437c`](https://github.com/GitoxideLabs/gitoxide/commit/2e1437cb0b5dc77f2317881767f71eaf9b009ebf))
 * **[#364](https://github.com/GitoxideLabs/gitoxide/issues/364)**
    - Update changelogs prior to release ([`746a676`](https://github.com/GitoxideLabs/gitoxide/commit/746a676056cd4907da7137a00798344b5bdb4419))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Add `Url::canonicalized()` and `Url::canonicalize()`. ([`01f2574`](https://github.com/GitoxideLabs/gitoxide/commit/01f25744bba45a5f8a8615734a5beeacd29d1c4e))
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
    - Refactor ([`02e5775`](https://github.com/GitoxideLabs/gitoxide/commit/02e5775f86ea414112ea9f66daf24ead8be31f73))
    - `Url::port_or_default()` to fill in default numbers for ports if possible. ([`7484db5`](https://github.com/GitoxideLabs/gitoxide/commit/7484db5d36383de450de31b4c94c01bc4c237ce4))
    - `Url::path_is_root()` to determine if the path is `/`. ([`fbe75c9`](https://github.com/GitoxideLabs/gitoxide/commit/fbe75c9457708b95dd833e00afa2dcc1db677167))
    - Make note about why we don't support passwords in URLs ([`970ec9b`](https://github.com/GitoxideLabs/gitoxide/commit/970ec9b2e51a32cae070e561a13ca3d76b9d22f9))
    - Url-preprocessing for scripts ([`c00cc35`](https://github.com/GitoxideLabs/gitoxide/commit/c00cc35493cec8f0b2673248caf1f0d83590dd54))
    - `Scheme::try_from(&str)` ([`d40f6e1`](https://github.com/GitoxideLabs/gitoxide/commit/d40f6e1f34eb3f4664caec36727bf0aa3a396a33))
    - Generalize extension schemes. ([`96a265c`](https://github.com/GitoxideLabs/gitoxide/commit/96a265cc67ea787ed28adde2c5d0a07babf64c9e))
    - `TryFrom<&OsStr>` to allow direct usage of `Url` in `clap`. ([`7a17690`](https://github.com/GitoxideLabs/gitoxide/commit/7a1769009d68d14a134f368f93245abab0fb41dd))
    - `TryFrom<PathBuf>` which is useful if urls are obtained from the command-line. ([`b7a5f7a`](https://github.com/GitoxideLabs/gitoxide/commit/b7a5f7a3b5cf058f503cc18d18fc75356ab98955))
    - A first sketch on how connections could be working ([`e55b43e`](https://github.com/GitoxideLabs/gitoxide/commit/e55b43ef72bb3f23655c7e0884b8efcf2496f944))
    - Use `&BStr` as input instead of `[u8]` ([`f6506e0`](https://github.com/GitoxideLabs/gitoxide/commit/f6506e0c463bdccbcfd9324bc312da9cc957d8e6))
    - Prohibit invalid state by making parts the url's data private ([`2bcfdee`](https://github.com/GitoxideLabs/gitoxide/commit/2bcfdee6a3af758a0b70e2af9c4b6f8cc09d8da0))
    - Remove invalid test as it looks like it parses hosts from paths and that is fine ([`224c605`](https://github.com/GitoxideLabs/gitoxide/commit/224c605d11a823bdaad6eb2bae1149bc671fb92d))
    - Switch to `thiserror` ([`cfd7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/cfd7c0a29f10010841b310e0eb8b000083381a58))
    - Prepare for better error handling around ssh urls ([`6d8d9b8`](https://github.com/GitoxideLabs/gitoxide/commit/6d8d9b87db3b41a45343c14ad1b50f742d084f11))
    - More conversions for `TryFrom`: `String` and `&str` ([`a67fc26`](https://github.com/GitoxideLabs/gitoxide/commit/a67fc26b80e5d1183ddc5c6598396214f3e19945))
    - Remove `impl std::fmt::Display for Url` as it's not lossless. ([`79ab4ae`](https://github.com/GitoxideLabs/gitoxide/commit/79ab4aeb8206a5f32735891336d7745e046bbea1))
    - `Url::write_to(out)` to write itself more flexibly. ([`833899d`](https://github.com/GitoxideLabs/gitoxide/commit/833899dce120d26a2bbe04d07fc4c71455eb3afe))
    - Add `Url::to_bstring()` for lossless but fallible bstring conversion. ([`5f707c7`](https://github.com/GitoxideLabs/gitoxide/commit/5f707c7e99c70ab9683d55c396e8dc11e1d3b0ea))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#524](https://github.com/GitoxideLabs/gitoxide/issues/524)**
    - Prepare changelogs prior to release ([`6446b39`](https://github.com/GitoxideLabs/gitoxide/commit/6446b395d5926565ef899b0c923f35468ccf1921))
    - Introduce `parse(&BStr)` (previously it took `&[u8]`) ([`653ebc5`](https://github.com/GitoxideLabs/gitoxide/commit/653ebc52f97116e9c72e985eda0d76f566e8c74d))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **[#725](https://github.com/GitoxideLabs/gitoxide/issues/725)**
    - Correctly parse scp-like ssh urls with alias as such. ([`372cea1`](https://github.com/GitoxideLabs/gitoxide/commit/372cea13ad8abe1c47ed7bc806b42292a8ebfae0))
 * **[#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)**
    - Prepare changelogs prior to release ([`8c0bca3`](https://github.com/GitoxideLabs/gitoxide/commit/8c0bca37ff9fbaadbe55561fb2b0d649980c95b1))
 * **Uncategorized**
    - Release gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`41d57b9`](https://github.com/GitoxideLabs/gitoxide/commit/41d57b98964094fc1528adb09f69ca824229bf25))
    - Release gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`e313112`](https://github.com/GitoxideLabs/gitoxide/commit/e31311257bd138b52042dea5fc40c3abab7f269b))
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
    - Adjust to renamining of `git-worktree` to `gix-worktree` ([`108bb1a`](https://github.com/GitoxideLabs/gitoxide/commit/108bb1a634f4828853fb590e9fc125f79441dd38))
    - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
    - Rename `git-url` to `gix-url` ([`e95d72e`](https://github.com/GitoxideLabs/gitoxide/commit/e95d72ed5b12b94a45f5ebfdea70a352b842cbec))
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
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
    - Prepare changelogs prior to release ([`93bef97`](https://github.com/GitoxideLabs/gitoxide/commit/93bef97b3c0c75d4bf7119fdd787516e1efc77bf))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`9e04685`](https://github.com/GitoxideLabs/gitoxide/commit/9e04685dd3f109bfb27663f9dc7c04102e660bf2))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Prepare changelogs prior to release ([`30d8ca1`](https://github.com/GitoxideLabs/gitoxide/commit/30d8ca19284049dcfbb0de2698cafae1d1a16b0c))
    - Make fmt ([`511ed00`](https://github.com/GitoxideLabs/gitoxide/commit/511ed0000397a5b268530c8f5362e7d25b7c1594))
    - Merge branch 'adjustments-for-cargo' ([`f8c562a`](https://github.com/GitoxideLabs/gitoxide/commit/f8c562a559e6dc3377583cc7200585dad7c3d481))
    - Collect ssh-specific options to control how the ssh program is invoked. ([`61d89f5`](https://github.com/GitoxideLabs/gitoxide/commit/61d89f586a0ad913fc2f502520282520a5e1fd15))
    - Release git-features v0.25.1, git-url v0.12.2, git-odb v0.38.1, git-transport v0.24.2, git-repository v0.30.2 ([`bb0a07b`](https://github.com/GitoxideLabs/gitoxide/commit/bb0a07b5edd5f980989d1a92e74df7f183febe87))
    - Merge branch 'fix/ssh-clone' ([`3678a6a`](https://github.com/GitoxideLabs/gitoxide/commit/3678a6abab6f59ff7008ccfe02bb8d61da47e166))
    - Properly set default SSH port to 22 ([`1058330`](https://github.com/GitoxideLabs/gitoxide/commit/1058330adcc3262c59d30a0b8854fade20ffc3d5))
    - Release git-url v0.12.1, git-transport v0.24.1, git-protocol v0.25.1, git-repository v0.30.1, git-commitgraph v0.12.0, gitoxide-core v0.22.0, gitoxide v0.20.0 ([`08ec3a9`](https://github.com/GitoxideLabs/gitoxide/commit/08ec3a93d77a1018439a5c41c23729ffed27c5a5))
    - Prepare changelogs prior to release ([`68ce15d`](https://github.com/GitoxideLabs/gitoxide/commit/68ce15d07b50cfacdac0d1e42fe7f5e6330ba523))
    - Merge branch 'fix/relative-scplike-urls' ([`b688592`](https://github.com/GitoxideLabs/gitoxide/commit/b68859254a02b93e7ea90f4881323357cfd080a4))
    - Don't sanitize SCP-like paths ([`d21f9eb`](https://github.com/GitoxideLabs/gitoxide/commit/d21f9eb3d6f295ed25da0b55541f9535f144b3b4))
    - Refactor ([`c70bf74`](https://github.com/GitoxideLabs/gitoxide/commit/c70bf74e6625179d4555a2468d3b2492179d86bf))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Scp-like URLs should preserve relative and home-relative paths ([`f20f272`](https://github.com/GitoxideLabs/gitoxide/commit/f20f2728ee78d90510e27769a61ead405c4018c1))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Merge branch 'main' into adjustments-for-cargo ([`bb60d3d`](https://github.com/GitoxideLabs/gitoxide/commit/bb60d3d5cb9dbd7abe61accded6d21e320c624db))
    - Reject empty paths where needed, add `Url::from_parts_as_alternative_form()`. ([`302a2d8`](https://github.com/GitoxideLabs/gitoxide/commit/302a2d866692a541e01d112b6870aa22fcdbe32b))
    - Make sure that `file:..` isn't considered a valid file url. ([`3e3aff9`](https://github.com/GitoxideLabs/gitoxide/commit/3e3aff9f2f427d030a38fe147c5252d7bfd45109))
    - Merge branch 'paulyoung/scheme-ext' ([`3e27550`](https://github.com/GitoxideLabs/gitoxide/commit/3e27550577ea942427a57c902570f0416f540753))
    - Realign test expectations ([`93e6d71`](https://github.com/GitoxideLabs/gitoxide/commit/93e6d7199408e492574c43fcfb81faccea2b6fd4))
    - Improve documentation ([`db7577f`](https://github.com/GitoxideLabs/gitoxide/commit/db7577ff348bbe9ffffcb1d5951c9dd579e111e3))
    - Cargo fmt ([`3b61a47`](https://github.com/GitoxideLabs/gitoxide/commit/3b61a47266abfb2145f64e8233eca12fa1d9cb65))
    - Allow parsing arbitrary URL schemes ([`4753e64`](https://github.com/GitoxideLabs/gitoxide/commit/4753e641eada72f4e944811ea85390481444b210))
    - Handle `file:///C:/foo/bar` urls correctly on windows, as paths now are `C:\\foo\bar`. ([`d6f90be`](https://github.com/GitoxideLabs/gitoxide/commit/d6f90beac37866f992a1714d38e5b320eea6f1bb))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Make a clearer note of the obvious deviation due to lack of storing passwords ([`d91bbcc`](https://github.com/GitoxideLabs/gitoxide/commit/d91bbcc14b34166c79bba6faafd4395d6a571477))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Merge branch 'clone' ([`507dc7e`](https://github.com/GitoxideLabs/gitoxide/commit/507dc7e706cb3c9d89d048b3aff5df239a9b6788))
    - `Url::try_from(path: &std::path::Path)` for more convenient instantiation. ([`22d3b37`](https://github.com/GitoxideLabs/gitoxide/commit/22d3b37ea6239170a478b859361a7d1d7ba01a9a))
    - More assurance we understand how relative paths in scp-like urls work ([`5926322`](https://github.com/GitoxideLabs/gitoxide/commit/5926322c7dc9ef45c0f8c7dc50551d0bf1800ada))
    - (mostly) lossless roundtripping of scp-like urls. ([`39ce98b`](https://github.com/GitoxideLabs/gitoxide/commit/39ce98ba9a427b8cea1b843f333c2e7de300499c))
    - Lossless serialization of file urls. ([`58a6000`](https://github.com/GitoxideLabs/gitoxide/commit/58a6000d669acd33bad91509eaa469f041f119e5))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0 ([`f5c36d8`](https://github.com/GitoxideLabs/gitoxide/commit/f5c36d85755d1f0f503b77d9a565fad6aecf6728))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
    - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
    - Make fmt ([`535e967`](https://github.com/GitoxideLabs/gitoxide/commit/535e967666c6da657ff1b7eff7c64ab27cafb182))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
    - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Release git-path v0.4.2, git-config-value v0.7.0 ([`c48fb31`](https://github.com/GitoxideLabs/gitoxide/commit/c48fb3107d29f9a06868b0c6de40567063a656d1))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-object v0.20.3, git-ref v0.15.4, git-config v0.7.1, git-diff v0.18.0, git-traverse v0.16.3, git-pack v0.22.0, git-odb v0.32.0, git-url v0.7.3, git-transport v0.19.3, git-protocol v0.19.1, git-refspec v0.1.1, git-repository v0.23.0, safety bump 6 crates ([`85a3bed`](https://github.com/GitoxideLabs/gitoxide/commit/85a3bedd68d2e5f36592a2f691c977dc55298279))
    - Release git-features v0.22.3, git-revision v0.4.4 ([`c2660e2`](https://github.com/GitoxideLabs/gitoxide/commit/c2660e2503323531ba02519eaa51124ee22fec51))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Thanks clippy ([`dc74fbd`](https://github.com/GitoxideLabs/gitoxide/commit/dc74fbd9a58e1d424713fc5f2442cedcc09c1200))
    - Make Scheme work with serde, removing `Copy` in the process.  e#450) ([`2da6c86`](https://github.com/GitoxideLabs/gitoxide/commit/2da6c862e184ac37d59147e9cf809017b65db966))
    - Release git-path v0.4.1 ([`5e82346`](https://github.com/GitoxideLabs/gitoxide/commit/5e823462b3deb904f5d6154a7bf114cef1988224))
    - Merge branch 'remote-ls-refs' ([`39d585d`](https://github.com/GitoxideLabs/gitoxide/commit/39d585d9f9ac6f3ecf51359c8e37f0a50e21ed45))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
    - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
    - Prepare changelogs prior to reelase ([`c06ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/c06ae1c606b6af9c2a12021103d99c2810750d60))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Release git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0 ([`aa639d8`](https://github.com/GitoxideLabs/gitoxide/commit/aa639d8c43f3098cc4a5b50614c5ae94a8156928))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Prepare changelog prior to release ([`3c50625`](https://github.com/GitoxideLabs/gitoxide/commit/3c50625fa51350ec885b0f38ec9e92f9444df0f9))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Assure document-features are available in all 'usable' and 'early' crates ([`238581c`](https://github.com/GitoxideLabs/gitoxide/commit/238581cc46c7288691eed37dc7de5069e3d86721))
    - `From<&[u8]>` is now `From<&BStr>` ([`ffc4a85`](https://github.com/GitoxideLabs/gitoxide/commit/ffc4a85b9a914b685d7ab528b30f2a3eefb44094))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
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
    - Release git-diff v0.14.0, git-bitmap v0.1.0, git-index v0.2.0, git-tempfile v2.0.1, git-lock v2.0.0, git-mailmap v0.1.0, git-traverse v0.13.0, git-pack v0.17.0, git-quote v0.2.0, git-odb v0.27.0, git-packetline v0.12.4, git-url v0.4.0, git-transport v0.16.0, git-protocol v0.15.0, git-ref v0.12.0, git-worktree v0.1.0, git-repository v0.15.0, cargo-smart-release v0.9.0, safety bump 5 crates ([`e58dc30`](https://github.com/GitoxideLabs/gitoxide/commit/e58dc3084cf17a9f618ae3a6554a7323e44428bf))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Merge branch 'svetli-n-path_value' ([`e8383ca`](https://github.com/GitoxideLabs/gitoxide/commit/e8383caf6db211beb57d70019fe4ad13ce9066ee))
    - Merge branch 'unify-path-encoding' ([`566ff8a`](https://github.com/GitoxideLabs/gitoxide/commit/566ff8a3597b889899d41ca15e5b9af7e05f1a4b))
    - Release git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`42ebb53`](https://github.com/GitoxideLabs/gitoxide/commit/42ebb536cd6086f096b8422291776c9720fa0948))
    - Release git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`1b76119`](https://github.com/GitoxideLabs/gitoxide/commit/1b76119259b8168aeb99cbbec233f7ddaa2d7d2c))
    - Release git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`8f57c29`](https://github.com/GitoxideLabs/gitoxide/commit/8f57c297d7d6ed68cf51415ea7ede4bf9263326e))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Prepare changelogs for release ([`674ec73`](https://github.com/GitoxideLabs/gitoxide/commit/674ec73b0816baa2c63b4ef1b40b7a41849c5e95))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`4ca9e07`](https://github.com/GitoxideLabs/gitoxide/commit/4ca9e07c7ac062d48d64ad7b516274e32dbc51c6))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Make fmt, but now it picked up some parts that usually don't get altered… ([`01f7b72`](https://github.com/GitoxideLabs/gitoxide/commit/01f7b729337bd2c99498321c479a9a13b1858e3e))
    - Update changelogs just for fun ([`21541b3`](https://github.com/GitoxideLabs/gitoxide/commit/21541b3301de1e053fc0e84373be60d2162fbaae))
    - Release git-url v0.3.3 ([`fdd5bdb`](https://github.com/GitoxideLabs/gitoxide/commit/fdd5bdb1bedc9a5d10ee69d315c11860d3f2468b))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - (cargo-release) version 0.3.2 ([`03de99e`](https://github.com/GitoxideLabs/gitoxide/commit/03de99e31fae18cabab19baafc78b2bef8b6a493))
    - (cargo-release) version 0.3.1 ([`4deef67`](https://github.com/GitoxideLabs/gitoxide/commit/4deef67a2259a0bf0e2cfa7d027e082240c67733))
    - Fix compile warnings ([`42fd77b`](https://github.com/GitoxideLabs/gitoxide/commit/42fd77b790eade874c559ed0bed14530ecda66d1))
    - (cargo-release) version 0.3.0 ([`d5c6643`](https://github.com/GitoxideLabs/gitoxide/commit/d5c6643a41d295eaf7aabb84eab435e42a11dd42))
    - Thanks clippy ([`e13adb2`](https://github.com/GitoxideLabs/gitoxide/commit/e13adb2b51e634ee5085038c3b1eaecfd6c43715))
    - [gitoxide-core] Use git-config for remote url parsing ([`c45feed`](https://github.com/GitoxideLabs/gitoxide/commit/c45feed6124601a8bbef609d5b47c5b8a9d5defa))
    - (cargo-release) version 0.2.0 ([`0c39373`](https://github.com/GitoxideLabs/gitoxide/commit/0c39373de5aba0acc4aaa330bf51b6abd4f50474))
    - Support for radicle urls ([`2c5b955`](https://github.com/GitoxideLabs/gitoxide/commit/2c5b955b07073c5ef0e7bbe3ab20f0047770440b))
    - (cargo-release) version 0.1.1 ([`e94fefa`](https://github.com/GitoxideLabs/gitoxide/commit/e94fefaf5e7a10605fa7ca46b2ce84a60b149aa0))
    - Finish git-url docs ([`4099508`](https://github.com/GitoxideLabs/gitoxide/commit/4099508ae32a4cce1a110d68c094d8c9002d8835))
    - Begin of documenting git-url crate ([`c891901`](https://github.com/GitoxideLabs/gitoxide/commit/c891901f1e7b2e0300bb7ae243c3579ced76c5e0))
    - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Finish removal of rust 2018 idioms ([`0d1699e`](https://github.com/GitoxideLabs/gitoxide/commit/0d1699e0e0bc9052be0a74b1b3f3d3eeeec39e3e))
    - Refactor ([`e07fbd6`](https://github.com/GitoxideLabs/gitoxide/commit/e07fbd63db297cd9f70f8b86b1f1f56b15e270a8))
    - [clone] encode message for git credentials helper ([`143549e`](https://github.com/GitoxideLabs/gitoxide/commit/143549e0757d4fa7a8347aa1b8b4734e9b62bf04))
    - [clone] make URL available in transport layer ([`6778447`](https://github.com/GitoxideLabs/gitoxide/commit/67784478b96f8afd142e52982e2161a1f05d2ec9))
    - [clone] Finish round-trip testing ([`df617fd`](https://github.com/GitoxideLabs/gitoxide/commit/df617fd8685e2efb9e897bc94a2dad163f0c9f2e))
    - Refactor ([`aea52fe`](https://github.com/GitoxideLabs/gitoxide/commit/aea52fe24168e20cd2949b7c4dd70abc88082429))
    - [clone] first sketch of roundtripping URLs ([`23678f8`](https://github.com/GitoxideLabs/gitoxide/commit/23678f8d91dd88cc4b797821cdc16af494044c0f))
    - [clone] first steps towards launching git-upload-pack while… ([`41f05f1`](https://github.com/GitoxideLabs/gitoxide/commit/41f05f13a1fac078b694e6f4a9c8f52eeaff4191))
    - [clone] Better error handling for generalized `connect(…)` ([`713808c`](https://github.com/GitoxideLabs/gitoxide/commit/713808cd8bd326b632c2b8f0cfbe7f147b1fa0aa))
    - [clone] expand-path should be server-side ([`8a38856`](https://github.com/GitoxideLabs/gitoxide/commit/8a38856a811078d1d453db9c0e0ad7b6baaaed3c))
    - Thanks clippy ([`0506fd9`](https://github.com/GitoxideLabs/gitoxide/commit/0506fd92aadec7c92747fb80c0aa6fe68908bc5c))
    - [url] more specific 'missing user home' error ([`ec5721a`](https://github.com/GitoxideLabs/gitoxide/commit/ec5721a7d153da1cc628de2bb20de8f723140a54))
    - Refactor ([`e54681a`](https://github.com/GitoxideLabs/gitoxide/commit/e54681aef693bfd4b0d5dfd385b6fb8cc150376b))
    - [url] Actually the is_relative() case should never be triggered ([`ac89d38`](https://github.com/GitoxideLabs/gitoxide/commit/ac89d38c6af96b2ae834df00451ad22a8947d43b))
    - [url] try again, maybe this works on windows… ([`f14fdd1`](https://github.com/GitoxideLabs/gitoxide/commit/f14fdd12fafec6b12feb2ae6ab965793f20ee2c5))
    - [url] Once more with feeling ([`2ea4a8c`](https://github.com/GitoxideLabs/gitoxide/commit/2ea4a8cb515c3cb8b8273648ebf367324cfec6ae))
    - [url] all debug output there is… ([`3df5b41`](https://github.com/GitoxideLabs/gitoxide/commit/3df5b41d33b54c87bdda663723253b66179148fe))
    - [url] yikes, more debugging for windows on CI ([`9a430e7`](https://github.com/GitoxideLabs/gitoxide/commit/9a430e77a428be5b5e499a7fc28ed88860cafe68))
    - [url] Another try to make this work on windows - tests probably ([`a51647f`](https://github.com/GitoxideLabs/gitoxide/commit/a51647fc8b2297e54ac2ac37f15a7c603ff92d1b))
    - [url] See if this fixes the windows tests ([`534c6a6`](https://github.com/GitoxideLabs/gitoxide/commit/534c6a67cd98944215e17a5f21490aa06a9f2113))
    - [url]  add standard conversions ([`27e3bdc`](https://github.com/GitoxideLabs/gitoxide/commit/27e3bdcfc1fe4ceabfe5aca2d55d68a005756cca))
    - Refactor ([`73e2b1b`](https://github.com/GitoxideLabs/gitoxide/commit/73e2b1b16ed5ba584a67488cb481ea13f54c0488))
    - [url] BString in public interface ([`745662d`](https://github.com/GitoxideLabs/gitoxide/commit/745662da413a0d5379d40a1e26b131477393d26f))
    - [url] Commit to 'bstr' ([`3d26ae1`](https://github.com/GitoxideLabs/gitoxide/commit/3d26ae1dfaac44054705a3ab3ae5e00ce98298dd))
    - [url] remove feature toggle, 'home' dependency is small enough ([`a5a6f0f`](https://github.com/GitoxideLabs/gitoxide/commit/a5a6f0fc7f193a3eed0992f90a6f37348fd47830))
    - [url] Add user expansion support (behind feature toggle) ([`a684cfe`](https://github.com/GitoxideLabs/gitoxide/commit/a684cfe05f6fa33e674ce7a521179e7f65f84705))
    - [url] first stab at expanding paths with user names ([`37459dc`](https://github.com/GitoxideLabs/gitoxide/commit/37459dcac513b6123157eefe4942a9610a1192ed))
    - Thanks clippy ([`50acab7`](https://github.com/GitoxideLabs/gitoxide/commit/50acab74e57911b1a10dda4a8c2823db5ae1fa2b))
    - [url] Support for git and http urls, as well as user expansion parsing ([`5ef201d`](https://github.com/GitoxideLabs/gitoxide/commit/5ef201db248d60656b949f59d10b539499459cff))
    - Refactor ([`6ab7cc6`](https://github.com/GitoxideLabs/gitoxide/commit/6ab7cc6c330b3d32cb85cfa3fba63c0e145104b7))
    - [url] first stab at implementing username expansion reasonably ([`86d17a3`](https://github.com/GitoxideLabs/gitoxide/commit/86d17a3da3330c495b7ec7e53aca50bf864723f7))
    - [url] fix serde ([`569014d`](https://github.com/GitoxideLabs/gitoxide/commit/569014d49514c744947b84e47be4dda46d2bcca3))
    - [url] Now with support for non-utf8 byte strings ([`81f01fd`](https://github.com/GitoxideLabs/gitoxide/commit/81f01fde78cb173d7bcdcfa8f22800e69e7981dd))
    - [url] more tests and additional limitations ([`3c2811f`](https://github.com/GitoxideLabs/gitoxide/commit/3c2811f8fedc9b0018e3bb01e81b365543d65505))
    - [url] handle trivial file protocol URLs better ([`18eb512`](https://github.com/GitoxideLabs/gitoxide/commit/18eb51286e12608f030cde10646d6502e0dbf427))
    - [url] Disable URL parsing for things that look like paths ([`03b0de9`](https://github.com/GitoxideLabs/gitoxide/commit/03b0de94c2d85484f474f6a780d564b28de98c8a))
    - [url] turns out that relative URLs and windows paths are killing it ([`0bee58e`](https://github.com/GitoxideLabs/gitoxide/commit/0bee58e66e24ce6002d4a7eeee86f92146bbee16))
    - [url] Switch to 'url' crate, as correctness certainly is more important than compile times ([`da6ad48`](https://github.com/GitoxideLabs/gitoxide/commit/da6ad48e48dbc619e1195d0ac10059c8a04e993e))
    - Thanks clippy ([`a37c7a3`](https://github.com/GitoxideLabs/gitoxide/commit/a37c7a37524b2a3a5ef853765832ac9a30ae8f2d))
    - [url] user and IPv4 parsing/simple validation ([`d1929ac`](https://github.com/GitoxideLabs/gitoxide/commit/d1929ac319767e7846f595dffb3a886abdafa87f))
    - [url] parse port number ([`bc8bd99`](https://github.com/GitoxideLabs/gitoxide/commit/bc8bd99335ba2502dd5ad7a1d54005c2093156cf))
    - Try for leaner tests, but it does the opposite kind of :D ([`098f802`](https://github.com/GitoxideLabs/gitoxide/commit/098f802e6dc9f55632791ddf8d046563f75cba7a))
    - Refactor ([`4499a08`](https://github.com/GitoxideLabs/gitoxide/commit/4499a08d1c54daaab643fce054141bc8fcc754be))
    - Refactor ([`42a1b51`](https://github.com/GitoxideLabs/gitoxide/commit/42a1b5150fee8d06e17f33fb04b56dac630f7b69))
    - [url] the first green tests ([`a501bc1`](https://github.com/GitoxideLabs/gitoxide/commit/a501bc19c0a2ad1b4ee576379841fea2af6db6cc))
    - Refactor ([`9c5fb91`](https://github.com/GitoxideLabs/gitoxide/commit/9c5fb91bde4f3562566c0528307cd74f056fe5ce))
    - [url] infrastructure for nom errors, taken from git-object ([`0ae38ed`](https://github.com/GitoxideLabs/gitoxide/commit/0ae38edcfd2c7b9c6793c0bc21e88e9d4d19a6b1))
    - [url] basic frame and first failing test ([`60aacf0`](https://github.com/GitoxideLabs/gitoxide/commit/60aacf0c279d277c4abf13e62697a51feeee26fd))
    - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
    - Add git-url crate ([`fd2e5ba`](https://github.com/GitoxideLabs/gitoxide/commit/fd2e5bab97f09666c983634fa89947a4bed1c92d))
</details>

## 0.13.1 (2023-01-10)

A maintenance release without user-facing changes.

## 0.13.0 (2022-12-30)

### New Features

 - <csr-id-61d89f586a0ad913fc2f502520282520a5e1fd15/> collect ssh-specific options to control how the ssh program is invoked.
   These are passed through when creating the ssh transport.

## 0.12.2 (2022-12-26)

### Bug Fixes

 - <csr-id-1058330adcc3262c59d30a0b8854fade20ffc3d5/> properly set default SSH port to 22

## 0.12.1 (2022-12-22)

A maintenance release without user-facing changes.

## 0.12.0 (2022-12-19)

### Bug Fixes

 - <csr-id-302a2d866692a541e01d112b6870aa22fcdbe32b/> reject empty paths where needed, add `Url::from_parts_as_alternative_form()`.
   The new constructor allows to create URLs that represent paths which otherwise couldn't
   be valid URLs.
 - <csr-id-3e3aff9f2f427d030a38fe147c5252d7bfd45109/> make sure that `file:..` isn't considered a valid file url.
 - <csr-id-d6f90beac37866f992a1714d38e5b320eea6f1bb/> handle `file:///C:/foo/bar` urls correctly on windows, as paths now are `C:\\foo\bar`.
   These paths are created when using the `url::Url::from_file_path()`
   family of methods, which adds an extra slash at the beginning of a
   windows path which makes it invalid there unless there is further
   processing.
   
   This is now applied by using `url` features, making this case work
   specifically. Note that all other attributes are still the same
   and `gix-url` generally tries to keep paths in tact to be a hybrid
   of type that can handle any file system paths as well as actual urls.
 - <csr-id-f20f2728ee78d90510e27769a61ead405c4018c1/> scp-like URLs should preserve relative and home-relative paths

## 0.11.0 (2022-11-21)

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

## 0.10.1 (2022-11-06)

### New Features

 - <csr-id-01f25744bba45a5f8a8615734a5beeacd29d1c4e/> add `Url::canonicalized()` and `Url::canonicalize()`.
   These methods allow to assure file urls are absolute, useful when
   cloning from any url.

## 0.10.0 (2022-10-10)

### New Features

 - <csr-id-22d3b37ea6239170a478b859361a7d1d7ba01a9a/> `Url::try_from(path: &std::path::Path)` for more convenient instantiation.
 - <csr-id-39ce98ba9a427b8cea1b843f333c2e7de300499c/> (mostly) lossless roundtripping of scp-like urls.
   Previously `git@host:path` would turn into `ssh://git@host/path`,
   which now remains exactly as is.
 - <csr-id-58a6000d669acd33bad91509eaa469f041f119e5/> lossless serialization of file urls.
   Previously a url like `/path/to/repo` would serialize to
   `file:///path/to/repo`, preventing round-trips.
   
   Now it serializes like it was parsed. This also means that
   `file://path` still serializes as `file://path`.

## 0.9.0 (2022-09-20)

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

## 0.8.0 (2022-09-04)

A major release to properly introduce the signature change that happened in 0.7.2, which effectively
broke compilation for users of `parse()` in 0.7.1.

### New Features

 - <csr-id-7484db5d36383de450de31b4c94c01bc4c237ce4/> `Url::port_or_default()` to fill in default numbers for ports if possible.
 - <csr-id-fbe75c9457708b95dd833e00afa2dcc1db677167/> `Url::path_is_root()` to determine if the path is `/`.
   This could also be considered an empty path depending on the context
   which is what makes it useful.

### Changed (BREAKING)

 - <csr-id-653ebc52f97116e9c72e985eda0d76f566e8c74d/> Introduce `parse(&BStr)` (previously it took `&[u8]`)
   A `&BStr` better indicates that we are expecting human-readable input
   with ascii-compatible or UTF-8 endcoding.

## 0.7.3 (2022-08-28)

Maintenance release without user-facing changes.

## 0.7.2 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### Bug Fixes (BREAKING)

 - <csr-id-2da6c862e184ac37d59147e9cf809017b65db966/> make Scheme work with serde, removing `Copy` in the process.  e#450)
   This wasn't supposed to happen but a requirement to get `serde` support
   back.

### New Features (BREAKING)

 - <csr-id-96a265cc67ea787ed28adde2c5d0a07babf64c9e/> generalize extension schemes.
   Previously this was hard-coded to `radicle`, now it's just an extension
   scheme along with a statically known string. This means we have to
   explicitly support new formats which should be fine.

### New Features

 - <csr-id-7a1769009d68d14a134f368f93245abab0fb41dd/> `TryFrom<&OsStr>` to allow direct usage of `Url` in `clap`.
   In `clap_derive`, this needs
   `parse(try_from_os_str = std::convert::TryFrom::try_from)`.
 - <csr-id-b7a5f7a3b5cf058f503cc18d18fc75356ab98955/> `TryFrom<PathBuf>` which is useful if urls are obtained from the command-line.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-d40f6e1f34eb3f4664caec36727bf0aa3a396a33/> `Scheme::try_from(&str)`

## 0.7.1 (2022-08-17)

A maintenance release without user facing changes.

### Bug Fixes (BREAKING)

 - <csr-id-2bcfdee6a3af758a0b70e2af9c4b6f8cc09d8da0/> Prohibit invalid state by making parts the url's data private
   This fix is meant to improve serialization support which can now happen
   `to_bstring()` without possibility for error.
   
   Empty paths can still be set which won't be valid for all URLs.

### Changed (BREAKING)

 - <csr-id-f6506e0c463bdccbcfd9324bc312da9cc957d8e6/> Use `&BStr` as input instead of `[u8]`
   This signals that it's indeed intended to be human readable while
   allowing it to be a path as well without loss, at least theoretically.
   After all we currently don't have a way to parse invalid UTF-8.
 - <csr-id-79ab4aeb8206a5f32735891336d7745e046bbea1/> remove `impl std::fmt::Display for Url` as it's not lossless.

### New Features

 - <csr-id-a67fc26b80e5d1183ddc5c6598396214f3e19945/> more conversions for `TryFrom`: `String` and `&str`
 - <csr-id-833899dce120d26a2bbe04d07fc4c71455eb3afe/> `Url::write_to(out)` to write itself more flexibly.
 - <csr-id-5f707c7e99c70ab9683d55c396e8dc11e1d3b0ea/> Add `Url::to_bstring()` for lossless but fallible bstring conversion.

## 0.7.0 (2022-07-22)

### Changed (BREAKING)

 - <csr-id-ffc4a85b9a914b685d7ab528b30f2a3eefb44094/> `From<&[u8]>` is now `From<&BStr>`
   This better represents the meaning of the input, and simplifies
   interactions with `gix-config`.

## 0.6.0 (2022-06-13)

A maintenance release without user-facing changes.

## 0.5.0 (2022-05-18)

A maintenance release without user-facing changes.

## 0.4.0 (2022-04-03)

A maintenance release without surfacing changes.

## 0.3.5 (2022-01-23)

A maintenance release with no relevant changes.

## v0.3.4 (2021-10-15)

This release contains no functional change, but a more useful changelog.

## v0.3.3 (2021-08-17)

## v0.3.2 (2021-08-10)

## v0.3.0 (2021-03-26)

## v0.2.0 (2021-01-14)

## v0.1.1 (2020-12-18)

## v0.1.0 (2020-09-12)

<csr-id-098f802e6dc9f55632791ddf8d046563f75cba7a/>

### Other

- <csr-id-098f802e6dc9f55632791ddf8d046563f75cba7a/> try for leaner tests, but it does the opposite kind of :D

## v0.0.0 (2020-08-13)

