# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.10.18 (2025-05-10)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 14 calendar days.
 - 14 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Prepare changelogs prior to release of `gix-index` ([`bfc4880`](https://github.com/GitoxideLabs/gitoxide/commit/bfc48801bf3ed39cdf7ec02e01aa3cfb6181705f))
    - Merge pull request #1975 from GitoxideLabs/improvements ([`28935a5`](https://github.com/GitoxideLabs/gitoxide/commit/28935a56ff91f1fc2c17a7d23b057cf7119144e9))
    - Thanks clippy ([`dbf65c9`](https://github.com/GitoxideLabs/gitoxide/commit/dbf65c95644e6a134e7f9b75e7871479720b4deb))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.10.17 (2025-04-26)

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

## 0.10.16 (2025-04-25)

### New Features

 - <csr-id-9f8a468cfb8730fcb0f88cbf62e559cb369fdb42/> Add `&gix_path::RelativePath`.
   It's a utility to assure functions get the right input, i.e. a type-safe
   version of what previously was `&BStr`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge pull request #1921 from cruessler/introduce-repository-path ([`fdc06b1`](https://github.com/GitoxideLabs/gitoxide/commit/fdc06b139a331bd2b345d34f09482317388fcba8))
    - Refactor ([`294902e`](https://github.com/GitoxideLabs/gitoxide/commit/294902e0dbc350a33a0e54164eed626720c1a1d7))
    - Add `&gix_path::RelativePath`. ([`9f8a468`](https://github.com/GitoxideLabs/gitoxide/commit/9f8a468cfb8730fcb0f88cbf62e559cb369fdb42))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Adapt `gix-path` tests to changes in `windows` ([`2fc48a1`](https://github.com/GitoxideLabs/gitoxide/commit/2fc48a11df504e135546f6c23c7d696c04c5006a))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.10.15 (2025-04-04)

<csr-id-028635165ddd98322d8b902fe0714fe2d0699a3e/>

### Bug Fixes

<csr-id-10af2d005fbe92a289be01492206c6e8a38ab0bd/>

 - <csr-id-1f269b0d5aa958f25423db1f83d144781bf22024/> Check prefix and prefer shim in `gix_path::env::shell()`
   This makes a few changes to make `shell()` more robust:
   
   1. Check the last two components of the path `git --exec-path`
   gave, to make sure they are `libexec/git-core`.
2. Check the grandparent component (that `../..` would go to) of
      the path `git --exec-path` gave, to make sure it is recognized
      name of a platform-specific `usr`-like directory that has been
      used in MSYS2.
   
      This is to avoid traversing up out of less common directory
      trees that have some different and shallower structure than
      found in a typical Git for Windows or MSYS2 installation.
3. Instead of using only the `(git root)/usr/bin/sh.exe` non-shim,
      prefer the `(git root)/bin/sh.exe` shim. If that is not found,
      fall back to the `(git root)/usr/bin/sh.exe` non-shim, mainly to
      support the Git for Windows SDK, which doesn't have the shim.
   
      The reason to prefer the shim is that it sets environment
      variables, including prepending `bin` directories that provide
      tools one would expect to have when using it. Without this,
      common POSIX commands may be unavailable, or different and
      incompatible implementations of them may be found.
   
      In particular, if they are found in a different MSYS2
      installation whose `msys-2.0.dll` is of a different version or
      otherwise a different build, then calling them directly may
      produce strange behavior. See:
   
      - https://cygwin.com/faq.html#faq.using.multiple-copies
      - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   
      This makes things more robust overall than either preferring the
      non-shim or just doing a path search for `sh` as was done before
      that. But it exacerbates #1868 (as described there), so if the
      Git for Windows `sh.exe` shim continues to work as it currently
      does, then further improvements may be called for here.
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
   - https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
- https://github.com/GitoxideLabs/gitoxide/pull/1862#issuecomment-2692158831
1. Adding components with `/` separators. While in principle a `\`
      should work, the path of the shell itself is used in shell
      scripts (script files and `sh -c` operands) that may not account
      for the presence of backslashes, and it is also harder to read
      paths with `\` in contexts where it appears escaped, which may
      include various messages from Rust code and shell scripts.
   
      The path before what we add will already use `/` and never `\`,
      unless `GIT_EXEC_PATH` has been set to a strange value, because
      it is based on `git --exec-path`, which by default gives a path
      with `/` separators. Thus, ensuring that the part we add uses `/`
      should be sufficient to get a path without `\` in all cases when
      it is clearly reasonable to do so. This therefore also usually
      increases stylistic consistency of the path, which is another
      factor that makes it more user-friendly in messages.
   
      This is needed to get tests to pass since changing `gix-command`
      to use `gix_path::env::shell()` on Windows, where a path is
      formatted in away that sometimes quotes `\` characters. Their
      expectations could be adjusted, but it seems likely that various
      other software, much of which may otherwise be working, has
      similar expectations. Using `/` instead of `\` works whether `\`
      is expected to be displayed quoted or not.
2. Check that the path to the shell plausibly has a shell there,
      only using it if it a file or a non-broken file symlink. When
      this is not the case, the fallback short name is used instead.
3. The fallback short name is changed from `sh` to `sh.exe`, since
      the `.exe` suffix is appended in other short names on Windows,
      such as `git.exe`, as well as being part of the filename
      component of the path we build for the shell when using the
      implementation provided as part of Git for Windows.

### Other

 - <csr-id-028635165ddd98322d8b902fe0714fe2d0699a3e/> Fix `to_windows_separators` docstring, revise others
   The `gix_path::convert::to_windows_separators` documentation
   comment had inadvertently described the opposite of what this
   function does, saying it changed backslashed to slashes and that
   the effect was like a Unix path. This fixes the docstring so it
   saye `to_windows_separators` changes slashes to backslashes, and
   moves the text that likens it to paths on Unix to the
   `to_unix_separators` docstring, where it applies.
   
   This also makes a number of other more minor revisions for
   clarity and stylistic consistency to documentation comments on
   other functions in the `gix_path::convert` module.

### New Features

 - <csr-id-9f8a468cfb8730fcb0f88cbf62e559cb369fdb42/> Add `&gix_path::RelativePath`.
   It's a utility to assure functions get the right input, i.e. a type-safe
   version of what previously was `&BStr`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Use raw literals for more strings with backslashes ([`01bd76d`](https://github.com/GitoxideLabs/gitoxide/commit/01bd76dcacb69d9c21f2fc6063e273a01aebf94f))
    - Merge pull request #1862 from EliahKagan/run-ci/consistent-sh ([`0ba3147`](https://github.com/GitoxideLabs/gitoxide/commit/0ba31474968ddbe7f2b2d54a756eeeb8a28fbabf))
    - Compile the new tests on all platforms ([`b70cdb1`](https://github.com/GitoxideLabs/gitoxide/commit/b70cdb19fbb194a97099afeb2ab208bd1355ee75))
    - Expand `git_for_windows_root()` comments ([`c824b92`](https://github.com/GitoxideLabs/gitoxide/commit/c824b9297824d52103d6ea40f49356d5d8b489e6))
    - Add some tests for finding associated executables ([`fb67059`](https://github.com/GitoxideLabs/gitoxide/commit/fb6705913727640a595792403ff35928d4d29d8d))
    - Revise `find_git_associated_windows_executable` future directions ([`56dc3cc`](https://github.com/GitoxideLabs/gitoxide/commit/56dc3cc008eb9eb9094450b1da327a34bfc9c850))
    - Generalize the `gix_path::env::auxiliary` helpers ([`17b5c31`](https://github.com/GitoxideLabs/gitoxide/commit/17b5c31978a7868b2e2f36cd1c5f57c7a23eab1a))
    - Simplify code and comments ([`0a6a056`](https://github.com/GitoxideLabs/gitoxide/commit/0a6a0568ce91534f551e87270ac076b79bf5c204))
    - Remove a potentially misleading docstring paragraph ([`bd26745`](https://github.com/GitoxideLabs/gitoxide/commit/bd267454e4261ea48e6a35b25ea5febffc0187b3))
    - Divide helpers more logically, expand doc comments ([`9d9ec58`](https://github.com/GitoxideLabs/gitoxide/commit/9d9ec58d5ee77873399b51faa9d5e8ddd1e82f30))
    - Move `shell()` helpers to a helper module ([`0b75b23`](https://github.com/GitoxideLabs/gitoxide/commit/0b75b231c48ef2509a3cef0a8a719418b4ff9d45))
    - Don't match `libexec/git-core` in `usr`; refactor ([`83574e1`](https://github.com/GitoxideLabs/gitoxide/commit/83574e1636e3a25b48c3c6198e8c17e6e81d04e9))
    - Check prefix and prefer shim in `gix_path::env::shell()` ([`1f269b0`](https://github.com/GitoxideLabs/gitoxide/commit/1f269b0d5aa958f25423db1f83d144781bf22024))
    - Revise `gix_path::env` docstrings for clarity ([`da7d70e`](https://github.com/GitoxideLabs/gitoxide/commit/da7d70ede6fe9fe10a0500c284d8f27610f613df))
    - Use `/` in `gix_path::env::shell()` and check existence ([`10af2d0`](https://github.com/GitoxideLabs/gitoxide/commit/10af2d005fbe92a289be01492206c6e8a38ab0bd))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1841 from EliahKagan/no-esc-todo ([`ad7a94e`](https://github.com/GitoxideLabs/gitoxide/commit/ad7a94ed6385e5c35c0fdaef420425d8e170334f))
    - Merge pull request #1843 from EliahKagan/convert-doc ([`bb64ee1`](https://github.com/GitoxideLabs/gitoxide/commit/bb64ee170fd308d83b7925e19627ed297bcdbb38))
    - Clarify `to_unix_separators{,_on_windows}` relationship ([`42875c9`](https://github.com/GitoxideLabs/gitoxide/commit/42875c9e4314bc2c92478393bc8dda4400fc397b))
    - Fix `to_windows_separators` docstring, revise others ([`0286351`](https://github.com/GitoxideLabs/gitoxide/commit/028635165ddd98322d8b902fe0714fe2d0699a3e))
    - Remove TODOs about using `path-slash` to handle escapes ([`a810d1f`](https://github.com/GitoxideLabs/gitoxide/commit/a810d1faa29803fb9d42a7d645a1ab41fe7a45de))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

<csr-unknown>
This makes things more robust overall than either preferring thenon-shim or just doing a path search for sh as was done beforethat. But it exacerbates #1868 (as described there), so if theGit for Windows sh.exe shim continues to work as it currentlydoes, then further improvements may be called for here.<csr-unknown/>

## 0.10.14 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### New Features

 - <csr-id-73ee27afb1871cea7117982f41a999efa3167186/> add `env::core_dir()`
 - <csr-id-840c71dfb5cda32fcaa6f744e23751c53a8281b1/> add `env::git_shell()` to obtain the shell Git would be using.
   This is particularly useful to execute Git hooks.

### Bug Fixes

 - <csr-id-51bbb8646287f0a6dfa9ff0fd3852c25e001aaf0/> rename `env::login_shell()` to `shell()` and explain what it is.
   This assures we don't suggest the usage of actual login shells to ever
   be used to execute hooks.
   
   Note that this is not marked as breaking change as no release was made
   with the old name yet.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 55 calendar days.
 - 55 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1758 from GitoxideLabs/git-shell ([`851a7c4`](https://github.com/GitoxideLabs/gitoxide/commit/851a7c4d0677b15e2158dbddcdd01d1b99711ea4))
    - Apply suggestions from code review ([`5400320`](https://github.com/GitoxideLabs/gitoxide/commit/5400320cb8bfa94181abb94fe08321ae188f8555))
    - More robust localization of `sh.exe` on Windows ([`0737c41`](https://github.com/GitoxideLabs/gitoxide/commit/0737c41ab921a8b3228252b5809cb0a0f064bfa6))
    - Add `env::core_dir()` ([`73ee27a`](https://github.com/GitoxideLabs/gitoxide/commit/73ee27afb1871cea7117982f41a999efa3167186))
    - Rename `env::login_shell()` to `shell()` and explain what it is. ([`51bbb86`](https://github.com/GitoxideLabs/gitoxide/commit/51bbb8646287f0a6dfa9ff0fd3852c25e001aaf0))
    - Merge pull request #1752 from GitoxideLabs/git-shell ([`1ca480a`](https://github.com/GitoxideLabs/gitoxide/commit/1ca480aa4093328a7e047e770fdffdb8cc6d8e8d))
    - Add `env::git_shell()` to obtain the shell Git would be using. ([`840c71d`](https://github.com/GitoxideLabs/gitoxide/commit/840c71dfb5cda32fcaa6f744e23751c53a8281b1))
    - Refactor git-path tests ([`4ef3a8d`](https://github.com/GitoxideLabs/gitoxide/commit/4ef3a8d34512b97039e8335a34f4e4256a4ff7d0))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.10.13 (2024-11-24)

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

## 0.10.12 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>

### Bug Fixes

 - <csr-id-eb72d31a64b5041170e5903c328b246dab522067/> Don't read "installation" config from `GIT_CONFIG`
   When `gix-path` runs `git config -l ...` to obtain the path of a
   configuration file to regard as being associated with the `git`
   installation itself, it now no longer allows `GIT_CONFIG` to affect
   this path.
   
   Previously, setting `GIT_CONFIG` would treat it as the path of a
   `GitInstallation` configuration file. Although it is possible that
   this may have been used intentionally, it was never documented, did
   not work reliably on all platforms, and carried significant
   disadvantages. This is most likely a non-breaking change.
   
   The disadvantages of treating a path in `GIT_CONFIG` as the path
   to a configuration file associated with the `git` installation
   were:
   
   - For `git`, the `GIT_CONFIG` environment variable *only* affects
   `git config` commands, and not other commands that use
   configuration variables (which are most `git` commands). But when
   `gix-path` would obtain a path from `git config -l ...` that came
   from `GIT_CONFIG`, that configuration file would be used
   anywhere, and not only `gix config` commands.

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

 - 11 commits contributed to the release over the course of 46 calendar days.
 - 46 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1594 from EliahKagan/comments ([`ab8880f`](https://github.com/GitoxideLabs/gitoxide/commit/ab8880fbdf0f7af7b483f4e1f9adbb8e374183ee))
    - Expand and clarify "unkown" scope comment in `git_cmd` ([`1526b01`](https://github.com/GitoxideLabs/gitoxide/commit/1526b018c45a4392f20d01e1df014395824bc3bf))
    - Merge pull request #1583 from EliahKagan/config-origin-env ([`130db3b`](https://github.com/GitoxideLabs/gitoxide/commit/130db3b1b4c969d64d734feba3a4decef79b10e2))
    - Don't read "installation" config from `GIT_CONFIG` ([`eb72d31`](https://github.com/GitoxideLabs/gitoxide/commit/eb72d31a64b5041170e5903c328b246dab522067))
    - Fix the `never_from_git_config_env_var` test ([`f8e38d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8e38d0575e7e898133aa471539ad79b7508c88e))
    - Start testing that installation config is not from `GIT_CONFIG` ([`340ff37`](https://github.com/GitoxideLabs/gitoxide/commit/340ff371d181686afd80582f7d5c8f340aabd24c))
    - Unset other `GIT_DIR` related env vars in `git config -l` ([`01a412f`](https://github.com/GitoxideLabs/gitoxide/commit/01a412feb5abf201a7eb98901d81f73a83a820e6))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
</details>

## 0.10.11 (2024-09-06)

### Bug Fixes

<csr-id-7280a2d2f8b55a594ae134dd9a0a7a1668b7b56c/>
<csr-id-650a1b5cf25e086197cc55a68525a411e1c28031/>

 - <csr-id-f70b904bc520b2962dd2a77c035b6d9c47bb1cb8/> Don't require usable temp dir to get installation config
   When running `git config -l ...` to find the configuration file
   path associated with the `git` installation itself, the current
   working directory for the subprocess was set to the current
   directory prior to #1523, and to `/tmp` or a `/tmp`-like directory
   since #1523 (which improved performance and security).
   
   This builds on #1523, as well as on subsequent changes to run `git`
   in a way that its behavior depends less on its CWD, by making an
   even more robust choice of CWD for the subprocess, so that the CWD
   is less likely to be deeply nested or on network storage; more
   likely to exist; and, on Unix-like systems, less likely to contain
   a `.git` entry (though a `git` with security updates should refuse
   to take any configuration from such a repository unless it is owned
   by the user).
   
   Due to a combination of other measures that harden against
   malicious or unusual contents (especially setting `GIT_DIR`), the
   most significant benefit of this change is to fix the problem that
   a nonexistent temp dir would prevent the command from succeeding.
   
   The main way that could happen is if `TMPDIR` on Unix-like systems,
   or `TMP` or `TEMP` on Windows, is set to an incorrect value.
   Because these variables are sometimes reasonable to customize for
   specific purposes, it is plausible for them to be set to incorrect
   values by accident.
   
   Except on Windows, this always uses `/` as the CWD for the
   subprocess.
   
   On Windows, we use the Windows directory (usually `C:\Windows`)
   rather than the root of the system drive (usually `C:\`), because:
   
   - We are currently obtaining this information from environment
   variables, and it is possible for our own parent process to pass
   down an overly sanitized environment.
* https://go-review.googlesource.com/c/go/+/174318
   * https://github.com/git-for-windows/git/security/advisories/GHSA-vw2c-22j4-2fh2
   * https://github.com/Byron/gitoxide/security/advisories/GHSA-mgvv-9p9g-3jv4
 * https://github.com/Byron/gitoxide/security/advisories/GHSA-mgvv-9p9g-3jv4
- The user profile directory may be more deeply nested.
- The user profile directory may sometimes be on slow network
     storage when the discovered Windows directory is not.
- In some situations, the user profile directory does not actually
     exist, or does not exist yet.
- Overly sanitized environments are more likely to lack the
     `USERPROFILE` vairable than the `SystemRoot` variable.
- Users may occasionally choose to have their entire user profile
     directory be a Git repository.
- It's no easier to avoid the problem of using `C:\.git` in a user
     profile directory than in `C:\Windows`: they're usually both under
     `C:\`, and are both not the same as `C:\`. (If the user profile
     directory is a repository, then that will avoid that problem, yet
     be its own problem, if not for other measures that prevent both.)
- If the `git` command is an old and unpatched vulnerable version
     in which `safe.directory` is not yet implemented, or in which
     https://github.com/git/git/security/advisories/GHSA-j342-m5hw-rr3v
     or other vulnerabilities where `git` would perform operations on
     untrusted local repositories owned by other users are unpatched,
     then a `.git` subdirectory of a shared `/tmp` or `/tmp`-like
     directory could be created by another account, and its local
     configuration would still have been used. (This is not a bug in
     gitoxide per se; having vulnerable software installed that other
     software may use is inherently insecure. But it is nice to offer
     a small amount of protection against this when readily feasible.)
- If the `/tmp`-like location is a Git repository owned by the
     current user, then its local configuration would have been used.
- https://github.com/dotnet/docs/issues/41193
- https://github.com/python/cpython/pull/95486#issuecomment-1881469554
- https://github.com/python/cpython/pull/95486#issuecomment-1882134234
- Parsing is more reliable for paths containing unusual characters,
     because `-z`/`--null` causes all paths to be output literally.
   
     Previously, `"` characters were trimmed from the ends, but this
     would not always extract a correct path, because when a path
     contains characters that cause `git` to enclose it in double
     quotes, those characters are usually represented in a symbolic
     form, usually with `\` escapes.
   
     In some scenarios, such as usually on Windows when the escaped
     character is itself a `\` and not in the leading position, the
     mangled path would be usable, but more often it would not.
- The volume of output is less, because `--name-only` casues values
     not to be included in the output.
- The combination of `-z`/`--null` and `--name-only` makes the
     output format simpler, and the parsing logic is accordingly
     simpler.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 57 commits contributed to the release.
 - 14 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Prepare changelogs prior to release. ([`c759819`](https://github.com/GitoxideLabs/gitoxide/commit/c759819666fdad1ba743eed8e9458517f5cdf63c))
    - Merge pull request #1569 from EliahKagan/config-origin-naming ([`3cf9694`](https://github.com/GitoxideLabs/gitoxide/commit/3cf969487e07c4bf5d5b89b4e372d70b95ebbe36))
    - Rename to `GIT_HIGHEST_SCOPE_CONFIG_PATH` ([`0672576`](https://github.com/GitoxideLabs/gitoxide/commit/067257684918f2802f56eb3933f5e324bf35f914))
    - Merge pull request #1568 from EliahKagan/config-origin-next ([`adbaa2a`](https://github.com/GitoxideLabs/gitoxide/commit/adbaa2ab6840fd247c14701c39dcda3d280d86c0))
    - Rename `EXE_INFO` to something that probably captures its contents better. ([`dd2d666`](https://github.com/GitoxideLabs/gitoxide/commit/dd2d6665981425f5878318d5469216f27c3ac278))
    - Run `cargo fmt` ([`b11f7db`](https://github.com/GitoxideLabs/gitoxide/commit/b11f7db7d16c06c040619cf0fb5feab85734bc06))
    - Make `EXE_NAME` a `const` too ([`fb0b6d8`](https://github.com/GitoxideLabs/gitoxide/commit/fb0b6d8823354b50f3bd2eb3a8eb6ea3da56b5b6))
    - Make `NULL_DEVICE` a `const`, rather than a `static` item ([`9917d47`](https://github.com/GitoxideLabs/gitoxide/commit/9917d4719ed32fd34628594aaea6d4ca816448e3))
    - Put `first_file_from_config_with_origin` test with related ones ([`57e9a6f`](https://github.com/GitoxideLabs/gitoxide/commit/57e9a6fa1d354eb9a29e6e6fd2a1d09190292804))
    - Fix indentation nit ([`7cd20bb`](https://github.com/GitoxideLabs/gitoxide/commit/7cd20bba2ccb8224b25eeb6a579c6130d0c9ce2e))
    - Merge pull request #1567 from EliahKagan/config-origin ([`dd65e7b`](https://github.com/GitoxideLabs/gitoxide/commit/dd65e7bd0adf7afeec377f47ba67970ae5126fa3))
    - Improve structure of `exe_info` tests ([`5ac5f74`](https://github.com/GitoxideLabs/gitoxide/commit/5ac5f741f8fdae5b7425f89cfc73d7ebecd5b92b))
    - Clarify comment about where we run `git` from ([`5200184`](https://github.com/GitoxideLabs/gitoxide/commit/5200184ecbe888d5c18617f7b3b9c4c6fdef2c5f))
    - Test no local scope with empty system config ([`6160a83`](https://github.com/GitoxideLabs/gitoxide/commit/6160a8308038cee1e604b6fb86c9bfb779b3b835))
    - Don't set/change ceiling directories ([`2bce0d2`](https://github.com/GitoxideLabs/gitoxide/commit/2bce0d20c876f076e899e9e0ea03e571487309d9))
    - Explore also setting a ceiling directory ([`073e277`](https://github.com/GitoxideLabs/gitoxide/commit/073e27783c752901211fb18acb6b4a917ad91d0e))
    - Fix misstatement of Windows directory rationale ([`4e936bc`](https://github.com/GitoxideLabs/gitoxide/commit/4e936bc99c781c9829b6529de32e2bb0bb1debbd))
    - Unset a couple env vars just in case ([`8f6d39d`](https://github.com/GitoxideLabs/gitoxide/commit/8f6d39d2a6988e2e6c69a4f15f5d8aa6c917c6a6))
    - Simplify the new comments ([`b827813`](https://github.com/GitoxideLabs/gitoxide/commit/b8278138271835ace4e520ccdc1e5c7e4556992a))
    - Explain why we run `git` from a different directory ([`7fa5e35`](https://github.com/GitoxideLabs/gitoxide/commit/7fa5e3530620e2540d615dd3195c645d0c9446e9))
    - Small clarity tweaks ([`598c487`](https://github.com/GitoxideLabs/gitoxide/commit/598c487e25c5463a90130c8e77884f272b3ef1c3))
    - Fix `os::windows` error on non-Windows ([`1305114`](https://github.com/GitoxideLabs/gitoxide/commit/130511452bc5605d5af48c10c53ba2f6e9ddce3d))
    - Refactor for readability; clarify comments ([`ab0dcc1`](https://github.com/GitoxideLabs/gitoxide/commit/ab0dcc168a8ecd2caedffc63f9e00e468e2628c2))
    - Fix unused import on non-Windows systems ([`8472447`](https://github.com/GitoxideLabs/gitoxide/commit/847244728e9464cf2924509bd09b8b698e83fc09))
    - Don't require usable temp dir to get installation config ([`f70b904`](https://github.com/GitoxideLabs/gitoxide/commit/f70b904bc520b2962dd2a77c035b6d9c47bb1cb8))
    - Explain why we don't just use `--system` ([`29c6cca`](https://github.com/GitoxideLabs/gitoxide/commit/29c6ccabf99c3cd694d5f503fb45fceac986ebb6))
    - Explain why we don't just use `--show-scope` ([`f35e44c`](https://github.com/GitoxideLabs/gitoxide/commit/f35e44c41cc2ae4216c8c76075cdeedd5c4076c7))
    - Fix a test name for consistency ([`15e7b67`](https://github.com/GitoxideLabs/gitoxide/commit/15e7b67ba7447a7a630550d14543d721ee05b696))
    - Add another broken temp test ([`c80d562`](https://github.com/GitoxideLabs/gitoxide/commit/c80d562fc0b0dd6298c01e8ba48f6fde211a732c))
    - Extract nonexistent directory logic to a test helper struct ([`e60540f`](https://github.com/GitoxideLabs/gitoxide/commit/e60540fe4592b26b9b5858750312b9b5cbe92f6a))
    - Maybe slightly decrease risk of test precondition check failure ([`56dab13`](https://github.com/GitoxideLabs/gitoxide/commit/56dab133353c5605121bfd9ab362bf05f8848945))
    - Adjust some test code for clarity ([`5c1b4c0`](https://github.com/GitoxideLabs/gitoxide/commit/5c1b4c0a912847551803c3f29da21778e736b23c))
    - Check `env::temp_dir()` in both tests that set temp vars ([`79af259`](https://github.com/GitoxideLabs/gitoxide/commit/79af25969e148efd456dacb05100fb2a90ebc054))
    - Clarify assert and expect messages ([`703f882`](https://github.com/GitoxideLabs/gitoxide/commit/703f882f1887b3c69553c79a8d6e4d3214be904f))
    - Test EXE_INFO no local config even if temp dir doesn't exist ([`60465a5`](https://github.com/GitoxideLabs/gitoxide/commit/60465a5b5900916d3e7ab80e025e01f237c21e7b))
    - Slightly improve quality of test failure messages ([`9641660`](https://github.com/GitoxideLabs/gitoxide/commit/9641660ebf0f330b5f02db8d7bda8dd3266e890b))
    - Set GIT_WORK_TREE along with GIT_DIR, to avoid confusion ([`5723077`](https://github.com/GitoxideLabs/gitoxide/commit/572307708e52f57624b5ef5dabfbb4b4eb68982d))
    - More robustly ensure "installation" config is not local ([`7280a2d`](https://github.com/GitoxideLabs/gitoxide/commit/7280a2d2f8b55a594ae134dd9a0a7a1668b7b56c))
    - Check that the test affects `env::temp_dir()` as desired ([`15cec4e`](https://github.com/GitoxideLabs/gitoxide/commit/15cec4ed114b2cd8acc333bb847ff18a9b59589c))
    - Fix bug in new test where temp dir should be a repo ([`744bb38`](https://github.com/GitoxideLabs/gitoxide/commit/744bb38726309cd2d97003ca721fc7850866eefa))
    - Test EXE_INFO no local config even if temp dir is a repo ([`287f267`](https://github.com/GitoxideLabs/gitoxide/commit/287f2676b7c4603d3eb412f9c2478db9f7ec2647))
    - Code formatting ([`65d5151`](https://github.com/GitoxideLabs/gitoxide/commit/65d51518e7fcb7fd69f5121e76d2abd2a01446d4))
    - Fix EXE_INFO no local scope test for macOS ([`49e0715`](https://github.com/GitoxideLabs/gitoxide/commit/49e0715f67d6fa6b8d009dbf684f7ce013b4104c))
    - Add generated archive for local_config.sh ([`fd065ac`](https://github.com/GitoxideLabs/gitoxide/commit/fd065ac281f64c4bee18e0ce68930b1c8444a4c7))
    - Test that EXE_INFO never has local scope config ([`5a300e6`](https://github.com/GitoxideLabs/gitoxide/commit/5a300e6c827b62a0883aeb328f828f9e898ed76b))
    - Make EXE_INFO testable and add a basic test for it ([`1ee98bf`](https://github.com/GitoxideLabs/gitoxide/commit/1ee98bfa9383888e73219b47234f369930d51332))
    - Reorder gix_path::env::git tests to match order in code ([`ccd0401`](https://github.com/GitoxideLabs/gitoxide/commit/ccd04018905bd0c00857f4ef10ce6f1156b6e835))
    - Extract git_cmd helper for EXE_INFO ([`de2f35f`](https://github.com/GitoxideLabs/gitoxide/commit/de2f35f5c355cacb0c076674126e8243ce24ec5c))
    - Parse installation config path more robustly ([`650a1b5`](https://github.com/GitoxideLabs/gitoxide/commit/650a1b5cf25e086197cc55a68525a411e1c28031))
    - Comment Git version compatibility for EXE_INFO ([`9df57aa`](https://github.com/GitoxideLabs/gitoxide/commit/9df57aaec4dc5df3da4d4ee2e16db648cd735432))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
</details>

<csr-unknown>
https://github.com/Byron/gitoxide/security/advisories/GHSA-mgvv-9p9g-3jv4<csr-unknown/>

## 0.10.10 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 35 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
    - Prepare changelogs prior to release ([`0f25841`](https://github.com/GitoxideLabs/gitoxide/commit/0f2584178ae88e425f1c629eb85b69f3b4310d9f))
    - Merge pull request #1523 from martinvonz/push-xmsuurxprnnw ([`83c9de0`](https://github.com/GitoxideLabs/gitoxide/commit/83c9de0db43761b7692a122f1a337964a7cfeb7a))
    - Remove `--system` from `git config` call as it fails on MacOS ([`6b1c243`](https://github.com/GitoxideLabs/gitoxide/commit/6b1c2432f096b3de770a77694a23b93b8d29dc8d))
    - Run `git config -l` in temp dir when looking up system config ([`20ef4e9`](https://github.com/GitoxideLabs/gitoxide/commit/20ef4e99149ef8a1255d02d8e60ff30c90fd9bff))
    - Merge branch 'push-ysnqkzlzwuwq' ([`e2c747d`](https://github.com/GitoxideLabs/gitoxide/commit/e2c747d9049bc4a4975de2fd780e8b1fb862fc09))
    - Don't show console on Windows ([`087594c`](https://github.com/GitoxideLabs/gitoxide/commit/087594c226d0e38163e18eee76123f8e332130f2))
</details>

## 0.10.9 (2024-07-18)

### Bug Fixes

 - <csr-id-15235bf7968042da0493d431bbc955d6f9f54188/> Don't assume program files folder locations
   This checks where *program files* directories are located on a
   Windows system, which are used for a fallback check after `git`
   has not been found in a `PATH` search (to invoke `git` to find out
   information such as the location of its system config file).
   
   Previously, two hard-coded paths were used. These were correct for
   the vast majority of 64-bit Windows systems, but were in practice
   never correct on 32-bit Windows systems. Checking programmatically
   for the locations should thus enable detection to succeed on more
   systems and under more circumstances, and avoid other problems.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 36 commits contributed to the release.
 - 23 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.10.9 ([`15f1cf7`](https://github.com/GitoxideLabs/gitoxide/commit/15f1cf76764221d14afa66d03a6528b19b9c30c9))
    - Merge branch 'program-files' ([`ffe6b60`](https://github.com/GitoxideLabs/gitoxide/commit/ffe6b60be3c0361d38fd6634a7fbc299665a6a0c))
    - Refactor ([`73ed340`](https://github.com/GitoxideLabs/gitoxide/commit/73ed3402861c88092b9a2cb2502e0f20d8bbcd36))
    - Move `test::loc`'s `use` statements up ([`dd53408`](https://github.com/GitoxideLabs/gitoxide/commit/dd534087fd7829eee1a9b426ee8bcdaca9c352e1))
    - Update/refine comments and do some minor cleanup ([`dea1746`](https://github.com/GitoxideLabs/gitoxide/commit/dea1746b06483796398441de391567f9ac115d21))
    - Omit excess punctuation in `expect` messages ([`76e3b28`](https://github.com/GitoxideLabs/gitoxide/commit/76e3b28b8824c00a545faa9079751c8d445c4574))
    - Thanks clippy ([`464e0a2`](https://github.com/GitoxideLabs/gitoxide/commit/464e0a266aa7f3de29750f1c4f77cc4dacf37d91))
    - Add second `super::` in non-Windows `alternative_locations` test ([`8ae54e8`](https://github.com/GitoxideLabs/gitoxide/commit/8ae54e850463da417dfc87a68d8ca0025f997b7a))
    - Don't assume program files folder locations ([`15235bf`](https://github.com/GitoxideLabs/gitoxide/commit/15235bf7968042da0493d431bbc955d6f9f54188))
    - Fix test bug generating expected value on 32-bit system ([`98db88b`](https://github.com/GitoxideLabs/gitoxide/commit/98db88bb81f8b47fe83011aa9ecf1d709b9680c9))
    - Put the new tests in their own nested mod ([`d254e62`](https://github.com/GitoxideLabs/gitoxide/commit/d254e62819b66e5385b633f0670f8456ad2d0b6e))
    - Rename helper, write its tests (and their helpers), fix a bug ([`e990bcd`](https://github.com/GitoxideLabs/gitoxide/commit/e990bcd48212db0d5b39c1a2c5b3cf1aa22faa6d))
    - Call fewer different things "suffixes" ([`de7c49f`](https://github.com/GitoxideLabs/gitoxide/commit/de7c49fbd61ae518db0eb0ad5ec862569e6dae61))
    - Reorder statements in alternative_locations test for clarity ([`4d98535`](https://github.com/GitoxideLabs/gitoxide/commit/4d9853549fa4750de35f8a5b4306ccb105dfff0e))
    - Simplify path construction and config; improve comments ([`c8b2eb3`](https://github.com/GitoxideLabs/gitoxide/commit/c8b2eb35f880d1257318327bc6980f753a45e3b6))
    - Add the missing `Git` component in the suffixes ([`1f0c3bf`](https://github.com/GitoxideLabs/gitoxide/commit/1f0c3bf87905ead954646c045c846422cffcf67c))
    - Remove a fixme that should no longer be done ([`00127a7`](https://github.com/GitoxideLabs/gitoxide/commit/00127a7195cd9552ec9ee7377872b5ae32a87408))
    - Rename helper; condense huge comment and move into position ([`df175bc`](https://github.com/GitoxideLabs/gitoxide/commit/df175bc1a8d11a72dd059464a6553f675570dddd))
    - Fix the helper to join the `mingw*/bin` subdirs ([`c486a7d`](https://github.com/GitoxideLabs/gitoxide/commit/c486a7d8b2ad669e9d6daaee53a151b0dea4920b))
    - Start implementing a helper for ALTERNATIVE_LOCATIONS ([`167dc14`](https://github.com/GitoxideLabs/gitoxide/commit/167dc14f917c23104e01ab591bb00ccd26ba1c11))
    - Narrow assert_architectures fixme to just what's left ([`e9eabeb`](https://github.com/GitoxideLabs/gitoxide/commit/e9eabeb58c33ebed05d2ae72842da5898badcc3b))
    - Adjust GitBinSuffixes so uses in the test method are clearer ([`671c476`](https://github.com/GitoxideLabs/gitoxide/commit/671c47609f9726d629ab34d93630dcb882ceea94))
    - Assert that the corresponding suffixes are correct ([`5258f7a`](https://github.com/GitoxideLabs/gitoxide/commit/5258f7ab1ec02a1f64e2fa4e3e1bf646012a4dea))
    - Assert `ALTERNATIVE_LOCATIONS` count and prefixes ([`dd1e5c8`](https://github.com/GitoxideLabs/gitoxide/commit/dd1e5c8d30fb1326a96172e886fb03c3533453c8))
    - Rename PathsByRole to ProgramFilesPaths ([`3dd1d1f`](https://github.com/GitoxideLabs/gitoxide/commit/3dd1d1fd4e9f98d842ad70ca181160a51b87e844))
    - Clarify what pf_current represents ([`95708dd`](https://github.com/GitoxideLabs/gitoxide/commit/95708dd754482db9afe4ec417f6d5ebdd35bc539))
    - Make PathsByRole::maybe_pf_64bit an Option<PathBuf> ([`5701145`](https://github.com/GitoxideLabs/gitoxide/commit/5701145bafae708f9c43e2c6947c11037f1349e0))
    - Remove extra comment left over from before ([`518fd27`](https://github.com/GitoxideLabs/gitoxide/commit/518fd2709ea3606da07a7fa9fb634d19df3425d5))
    - Improve expect and assert messages in PathsByRole::validate() ([`5df0cf5`](https://github.com/GitoxideLabs/gitoxide/commit/5df0cf581e7bdcacaf6e630a3653ac449e01e30b))
    - Revise obtain_envlessly() doc comment for clarity ([`a5a5342`](https://github.com/GitoxideLabs/gitoxide/commit/a5a5342c7f94e3e53e2a4969beee81fadc8592ef))
    - Do a minor simplification now that it's PathBuf ([`99a8eb3`](https://github.com/GitoxideLabs/gitoxide/commit/99a8eb3748db9f50f11491ce3c9ebb197e70a942))
    - Make ALTERNATIVE_LOCATIONS items PathBuf ([`5b206bc`](https://github.com/GitoxideLabs/gitoxide/commit/5b206bce8149349f9b4390ca04c1304def225dff))
    - Start on unit test for ALTERNATIVE_LOCATIONS ([`edc1351`](https://github.com/GitoxideLabs/gitoxide/commit/edc1351d66f9ae1482193228d00a2b555a1ee979))
    - Add dev dependencies for soon to be added unit test ([`6af59ca`](https://github.com/GitoxideLabs/gitoxide/commit/6af59cab05183472a1843e867bce894b8c07557c))
    - Restyle just-changed and related code for clarity ([`98b3d90`](https://github.com/GitoxideLabs/gitoxide/commit/98b3d901bd770cc8cf178fce52695306ca48f403))
    - Make ALTERNATIVE_LOCATIONS dynamic and OsString-based ([`9eaa0d9`](https://github.com/GitoxideLabs/gitoxide/commit/9eaa0d90470192104def649d0e317b0b55f0d7a6))
</details>

## 0.10.8 (2024-06-24)

### New Features

 - <csr-id-fee775ca083189cd8909e8af143a69071cb71ecd/> provide `env::executable_invocation()` to know how to invoke Git.
   That way we can make it easier to rely on Git even if finding it is a bit
   more involved.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 3 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Merge branch 'git-executable' ([`f0a4431`](https://github.com/GitoxideLabs/gitoxide/commit/f0a44319e546670180197ba32a848d608d9ca7e9))
    - Use `gix_path::env::executable_invocation()` where possible. ([`5bf7f89`](https://github.com/GitoxideLabs/gitoxide/commit/5bf7f898b5e92207812dd950bdd37f856cdd9d9e))
    - Provide `env::executable_invocation()` to know how to invoke Git. ([`fee775c`](https://github.com/GitoxideLabs/gitoxide/commit/fee775ca083189cd8909e8af143a69071cb71ecd))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
</details>

## 0.10.7 (2024-03-14)

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

## 0.10.6 (2024-02-25)

### New Features

 - <csr-id-9ba8bcab07f4b3d0048b5a125b632bd70fe788d9/> add `relativize_with_prefix()`.
   With it, a path 'a' with prefix 'b' will be '../a'.
 - <csr-id-c8ccbe58cbafd70936b4143db01687e5bf9f7cf4/> add `try_os_str_into_bstr()`, with `Cow<OsStr>` as input.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 16 calendar days.
 - 30 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'status' ([`bb48c4c`](https://github.com/GitoxideLabs/gitoxide/commit/bb48c4ce22650b8c76af3b147e252ebe7cedb205))
    - Add `relativize_with_prefix()`. ([`9ba8bca`](https://github.com/GitoxideLabs/gitoxide/commit/9ba8bcab07f4b3d0048b5a125b632bd70fe788d9))
    - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
    - Add `try_os_str_into_bstr()`, with `Cow<OsStr>` as input. ([`c8ccbe5`](https://github.com/GitoxideLabs/gitoxide/commit/c8ccbe58cbafd70936b4143db01687e5bf9f7cf4))
</details>

## 0.10.5 (2024-01-26)

### Bug Fixes

 - <csr-id-8ba04e0d3525fc290d9ef24836c5ba88c022e8b7/> always try `HOME` environment variable first when obtaining the home directory.
   This will fix issues like the one described here:

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 1 calendar day.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
    - Merge branch 'fix-home' ([`e40352c`](https://github.com/GitoxideLabs/gitoxide/commit/e40352cd2f7b74da84fc8095c8cad917d07910a1))
    - Always try `HOME` environment variable first when obtaining the home directory. ([`8ba04e0`](https://github.com/GitoxideLabs/gitoxide/commit/8ba04e0d3525fc290d9ef24836c5ba88c022e8b7))
    - Assure time-based test doesn't spuriously fail on slow windows machines (CI) ([`74704c7`](https://github.com/GitoxideLabs/gitoxide/commit/74704c78276bd8bc9bdf3950e05a79bbc7d41309))
</details>

## 0.10.4 (2024-01-20)

### Bug Fixes

 - <csr-id-8d4bf403aecbe16ad2f4083f40c504c6fc4d7eab/> prevent very long path from using unbounded time in `realpath()`.
   It's possible to inject such paths using urls which can then end up
   being canonicalized, causing very long runtimes with excessively long
   paths due to `is_symlink` calls which will be slow.
   
   Now the amount of components is limited to 4096/2, which should be
   a worst-case path at the border of realistic.
   
   If this limitation becomes too arbitrary, one could consider making
   this cut-off value configurable.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 4 calendar days.
 - 20 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Merge branch 'url-fuzz' ([`db86fba`](https://github.com/GitoxideLabs/gitoxide/commit/db86fba550b8378caa737bccf80852d1309f0684))
    - Prevent very long path from using unbounded time in `realpath()`. ([`8d4bf40`](https://github.com/GitoxideLabs/gitoxide/commit/8d4bf403aecbe16ad2f4083f40c504c6fc4d7eab))
    - Merge branch 'dirwalk' ([`5d176fc`](https://github.com/GitoxideLabs/gitoxide/commit/5d176fc5ab82bfc7c194b4d929e73da9659ae8b8))
    - Use `gix_fs::current_dir(precompose_unicode)`. ([`7d8d167`](https://github.com/GitoxideLabs/gitoxide/commit/7d8d1678c9dd185a369e996551a6ed89fbb391f5))
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
</details>

## 0.10.3 (2023-12-30)

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

## 0.10.2 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
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
</details>

## 0.10.1 (2023-12-06)

### New Features

 - <csr-id-bc44497606656cddc4f18a0acb272c34b8df4ba8/> trace credential helper invocations.
   This should make it easier to understand what's going on in case
   something isn't working as expected.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1103](https://github.com/GitoxideLabs/gitoxide/issues/1103)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1103](https://github.com/GitoxideLabs/gitoxide/issues/1103)**
    - Trace credential helper invocations. ([`bc44497`](https://github.com/GitoxideLabs/gitoxide/commit/bc44497606656cddc4f18a0acb272c34b8df4ba8))
 * **Uncategorized**
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - Merge branch 'error' ([`c372321`](https://github.com/GitoxideLabs/gitoxide/commit/c372321dd6ea66a41c135d28c7319ab05a6d0942))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
</details>

## 0.10.0 (2023-09-08)

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

## 0.9.0 (2023-08-22)

<csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/>
<csr-id-5b5983a9686e9fe61a29e9e1b9e905cd4dbd296a/>

### Chore

- <csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/> don't call crate 'WIP' in manifest anymore.

### Other

- <csr-id-5b5983a9686e9fe61a29e9e1b9e905cd4dbd296a/> make clear that `normalize()` does not touch duplicate path separators nor single `.`.

### New Features (BREAKING)

 - <csr-id-df83d746ff44ae192b7c69356624ff8b4cc61dcd/> remove `Spec` type in favor of `gix-pathspec::Pattern`.
   The latter isn't included to keep concerns and crates separate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 12 calendar days.
 - 30 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Don't call crate 'WIP' in manifest anymore. ([`229bd48`](https://github.com/GitoxideLabs/gitoxide/commit/229bd4899213f749a7cc124aa2b82a1368fba40f))
    - Merge branch 'pathspec-matching' ([`9f4dfe0`](https://github.com/GitoxideLabs/gitoxide/commit/9f4dfe0f0b948280692916b596923959ea2fd9da))
    - Remove `Spec` type in favor of `gix-pathspec::Pattern`. ([`df83d74`](https://github.com/GitoxideLabs/gitoxide/commit/df83d746ff44ae192b7c69356624ff8b4cc61dcd))
    - Make clear that `normalize()` does not touch duplicate path separators nor single `.`. ([`5b5983a`](https://github.com/GitoxideLabs/gitoxide/commit/5b5983a9686e9fe61a29e9e1b9e905cd4dbd296a))
</details>

## 0.8.4 (2023-07-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 1 calendar day.
 - 23 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.8.3 (2023-06-29)

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

## 0.8.2 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### New Features

 - <csr-id-3cffa268460eb2d41bd6a30d45778b88db4ec602/> provide basic `tracing` spans for common operations.
   This is just the beginning and more crates will integrate with it over time.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 10 calendar days.
 - 15 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Release gix-trace v0.1.0 ([`2ab69c6`](https://github.com/GitoxideLabs/gitoxide/commit/2ab69c6fd142fcbbf6df3f981b1550b0c8167a04))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Provide basic `tracing` spans for common operations. ([`3cffa26`](https://github.com/GitoxideLabs/gitoxide/commit/3cffa268460eb2d41bd6a30d45778b88db4ec602))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
</details>

## 0.8.1 (2023-06-06)

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
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
</details>

## 0.8.0 (2023-04-27)

A maintenance release without user-facing changes.

### Changed (BREAKING)

 - <csr-id-94564df0b5edcb41f1088c6db04add851c66b2af/> Moved `home_dir()` to `env::home_dir()` and `env_var()` to `env::var()`.
   Please note that this change was previously and erroneously not declared as breaking.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.8.0, gix-glob v0.7.0, gix-attributes v0.12.0, gix-config-value v0.12.0, gix-ref v0.29.0, gix-sec v0.8.0, gix-config v0.22.0, gix-prompt v0.5.0, gix-url v0.18.0, gix-credentials v0.14.0, gix-discover v0.18.0, gix-ignore v0.2.0, gix-pack v0.35.0, gix-odb v0.45.0, gix-transport v0.31.0, gix-protocol v0.32.0, gix-refspec v0.10.1, gix-worktree v0.17.0, gix v0.44.1 ([`7ebc9f7`](https://github.com/GitoxideLabs/gitoxide/commit/7ebc9f734ec4371dd27daa568c0244185bb49eb5))
    - Prepare changelogs prior to release ([`0135158`](https://github.com/GitoxideLabs/gitoxide/commit/013515897215400539bfd53c25548bd054186ba6))
    - Bump gix-path v0.8.0, safety bump 20 crates (gix set to 0.44.1 manually) ([`43ebaf2`](https://github.com/GitoxideLabs/gitoxide/commit/43ebaf267557218865862538ffc7bdf00558492f))
    - Moved `home_dir()` to `env::home_dir()` and `env_var()` to `env::var()`. ([`94564df`](https://github.com/GitoxideLabs/gitoxide/commit/94564df0b5edcb41f1088c6db04add851c66b2af))
</details>

## 0.7.4 (2023-04-26)

**YANKED** due to accidental breaking changes.

### New Features

 - <csr-id-1e73f3c319caec0298cdd8c34f29ff78ff4060ab/> add `join_bstr_unix_pathsep(base, component)`.
   It's useful to have to avoid certain conversions to happen otherwise.
 - <csr-id-0d340f4fdeff1576460d43ca2210b11f0641c5dd/> add `xdg_config_home()`, installation_config` and `installation_config_prefix()` functions.

### Bug Fixes

 - <csr-id-bd1ae0db32191fb96d6b2a55b9fcb635d1652c1f/> `join_bstr_unix_pathsep()` works more suitably if base path is empty.
 - <csr-id-13edfe96696636c30040ec81ebc4e235ac689429/> use `home` in `env::home_dir()`
 - <csr-id-ec049fec7f789f6b55ca18a691388c44778603d4/> use `home` in `env::home_dir()`.
   This way it should work better on windows as it now uses the home_dir implementation
   of a crate used by `cargo`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 13 calendar days.
 - 27 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - Merge branch 'index-entries-attrs' ([`f37a930`](https://github.com/GitoxideLabs/gitoxide/commit/f37a930aefa27e67f0b693ba9669cc26d49044fa))
    - `join_bstr_unix_pathsep()` works more suitably if base path is empty. ([`bd1ae0d`](https://github.com/GitoxideLabs/gitoxide/commit/bd1ae0db32191fb96d6b2a55b9fcb635d1652c1f))
    - Merge branch 'attributes-cache' ([`3456c84`](https://github.com/GitoxideLabs/gitoxide/commit/3456c845dfeedd2fa96b4313b1a84c8cbe9433c5))
    - Add `join_bstr_unix_pathsep(base, component)`. ([`1e73f3c`](https://github.com/GitoxideLabs/gitoxide/commit/1e73f3c319caec0298cdd8c34f29ff78ff4060ab))
    - Merge branch 'utkarshgupta137/main' ([`74cb5ee`](https://github.com/GitoxideLabs/gitoxide/commit/74cb5ee03d7a5fbba312c0a5c782489a6fc039a7))
    - Use `home` in `env::home_dir()` ([`13edfe9`](https://github.com/GitoxideLabs/gitoxide/commit/13edfe96696636c30040ec81ebc4e235ac689429))
    - Revert "fix: use `home` in `env::home_dir()`." ([`222ece2`](https://github.com/GitoxideLabs/gitoxide/commit/222ece2a2fb639cbcdec639b06fa1bc2cc36c972))
    - Merge branch 'utkarshgupta137/main' ([`5092c59`](https://github.com/GitoxideLabs/gitoxide/commit/5092c59084250af45e5c81d1ec5219428249974d))
    - Use `home` in `env::home_dir()`. ([`ec049fe`](https://github.com/GitoxideLabs/gitoxide/commit/ec049fec7f789f6b55ca18a691388c44778603d4))
    - Make fmt ([`5d2b5d0`](https://github.com/GitoxideLabs/gitoxide/commit/5d2b5d02c3869e07dc2507a8f2519ee1df633df7))
    - Merge branch 'main' into dev ([`23ee47f`](https://github.com/GitoxideLabs/gitoxide/commit/23ee47fb24c197f8437bd426544b2aa74e005bdc))
    - Merge branch 'worktree-stack' ([`3d47919`](https://github.com/GitoxideLabs/gitoxide/commit/3d47919c1a2f83fc7c1fd7ae590d098057a22626))
    - Add `xdg_config_home()`, installation_config` and `installation_config_prefix()` functions. ([`0d340f4`](https://github.com/GitoxideLabs/gitoxide/commit/0d340f4fdeff1576460d43ca2210b11f0641c5dd))
</details>

## 0.7.3 (2023-03-30)

### Bug Fixes

 - <csr-id-d1bd513f27e17787eb223f7b0521f954c518153e/> $HOME detection on windows

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.7.3, gix-config-value v0.10.2, gix-config v0.20.1, gix-discover v0.16.2, gix-index v0.15.1, gix-odb v0.43.1, gix-packetline v0.15.1, gix-protocol v0.30.2, gix-worktree v0.15.2, gix v0.43.1 ([`38eed1d`](https://github.com/GitoxideLabs/gitoxide/commit/38eed1d06e7cbb8fbcd54b2cad3163ca45e0baf1))
    - Merge branch 'pascalkuthe/main' ([`d47cebe`](https://github.com/GitoxideLabs/gitoxide/commit/d47cebe3b23080c45829cb307b867220e3af20db))
    - Refactor ([`d1e5e12`](https://github.com/GitoxideLabs/gitoxide/commit/d1e5e12d54f79c030325860838c1cfadac1a7ac5))
    - $HOME detection on windows ([`d1bd513`](https://github.com/GitoxideLabs/gitoxide/commit/d1bd513f27e17787eb223f7b0521f954c518153e))
</details>

## 0.7.2 (2023-02-20)

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

## 0.7.1 (2023-02-17)

<csr-id-37cab07f283a368f323604372c84475d73d6c258/>
<csr-id-54801592488416ef2bb0f34c5061b62189c35c5e/>
<csr-id-8ab47bbdac44c0fa738215d3cc457eb3b6f30504/>
<csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/>
<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>

### Refactor (BREAKING)

- <csr-id-37cab07f283a368f323604372c84475d73d6c258/> Rename absolutize() to normalize()
  The name absolutize implies strongly that the returned path will be
  absolute, but the function only converts relative paths to absolute under a
  few specific circumstances.

  The new name, normalize(), is inspired by Python's os.path.normpath(),
  Java's java.nio.file.Path.normalize(), Node's Path.normalize(), and maybe
  some others which have similar semantics to this function.
- <csr-id-54801592488416ef2bb0f34c5061b62189c35c5e/> various name changes for more convenient API

### Bug Fixes (BREAKING)

 - <csr-id-7dbab1c62c49822983c59be0443478f7b4fecbca/> `absolutize()` now takes a mandatory `current_dir()` parameter and returns `Option<path>`
   Previously the function was willing to return an empty path despite it
   being invalid. With the `current_dir` being required, this won't be the
   case anymore and will yield logically consistent results in all cases.
   
   This forces the caller to deal with the relative path being invalid
   or crafted to produce some other path, maybe to bypass sanity checks.
 - <csr-id-c9933c0b0f51d21dc8244b2acc33d7dc8a33f6ce/> Remove `git-config` test utilities from `git-path`.

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
 - <csr-id-266d4379e9132fd7dd21e6c8fccb36e125069d6e/> Make `realpath()` easier to use by introducing `realpath_opt()`.
   That way there is consistency about how many symlinks to follow.

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`
 - <csr-id-dfa1e05d3c983f1e8b1cb3b80d03608341187883/> `realpath()` handles `cwd` internally
   This makes for more convenient usage in the common case.

### Refactor

- <csr-id-8ab47bbdac44c0fa738215d3cc457eb3b6f30504/> rename tests/convert/normalize.rs
  This renames the test module to match the new function name.

### Other

- <csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/> :discover()` now returns the shortest path.
  If and only if it canonicalized the source path. That way, users will
  still get a familiar path. This is due to `parent()` not operating
  in the file system, which otherwise would be equivalent to `..`,
  but that's not how we work.

  Maybe we should overhaul the way this works to use `../` instead
  and just 'absoluteize' the path later (std::path::absolute()) is
  on the way for that.

### Bug Fixes

 - <csr-id-745d92636f8a3436ded0c9da21beb92182341998/> `.` substitution is only done if the input was relative.
   Previously it was possible to have `/a/b/../b` and a CWD of `/a/b`
   replaced with `.` even though that clearly isn't what the user provided.
   
   Now the `.` resubstitution only happens when it's in the interest
   of the caller.
 - <csr-id-92d5d133e17c6b79400ec57b55ccd5337f3796b7/> `normalize()` would fail to interpret `../` correctly and end up in an invalid path.
   This is now fixed and should never happen again thanks to the addition
   of a missing test.
 - <csr-id-9171adb796b38b08cae9bdd375b16a59a8166a1c/> Handle `.` specifically in `absolutize()`.
   Previously, absolutizing `./../../` would lead to one path component
   of the `../` to be ignored as `.` was popped successfully, not realizing
   that it is a no-op.
   
   This could lead to problems with repository discovery if `.` was passed.

### New Features

 - <csr-id-25e795f4fe858d646ae7a3c4706e14a3837c3e66/> Add `os_string_into_bstring()` as sibling of `os_str_into_bstr()`.
 - <csr-id-523418f69030faa0add6472b14333e9aafc69f56/> add support for `wasi`
   This allows path conversions there to be just as efficient as on unix.
   
   This was adopted from [a PR in the
   hexlix-editor](https://github.com/helix-editor/helix/pull/3890/files#diff-504515b66023120e75a921cd56a932aed76c0ff62593fbb69d92e0ef65089501R47).
 - <csr-id-f58a043273b8e15afd01aac71f33652783baf462/> add `is_absolute()` for git-style absolute checks
   This essentially means that starting slashes are always absolute, even
   on windows.
 - <csr-id-35f146a8573dcc9a1de3230373c0cf0794c6b897/> Add `absolutize_components()`
   It helps to cleanup paths a little which comes in handy when dealing
   with `commondir` appended paths.

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 189 commits contributed to the release.
 - 20 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#422](https://github.com/GitoxideLabs/gitoxide/issues/422), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 5 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
    - `absolutize_*(dir)` is now `absolutize(dir, Option<cwd>)` ([`de87657`](https://github.com/GitoxideLabs/gitoxide/commit/de87657194ad976cc73ebcc13c231537b35b4195))
    - More robust absolutize-paths implementation ([`4800ebe`](https://github.com/GitoxideLabs/gitoxide/commit/4800ebec42f9bb6298cb5b2efdab71d6baf3b1ba))
    - Add `absolutize_components()` ([`35f146a`](https://github.com/GitoxideLabs/gitoxide/commit/35f146a8573dcc9a1de3230373c0cf0794c6b897))
    - Allow reading patterns from stdin ([`0c597fe`](https://github.com/GitoxideLabs/gitoxide/commit/0c597fe78acdd5672b4535a7d82620c5f7f93649))
    - :discover()` now returns the shortest path. ([`e4f4c4b`](https://github.com/GitoxideLabs/gitoxide/commit/e4f4c4b2c75a63a40a174e3a006ea64ef8d78809))
    - Basic prefix support as well the first working version of `exclude query` ([`9cb8385`](https://github.com/GitoxideLabs/gitoxide/commit/9cb83859f9bb76f38ab5bbd0ae6d6f20a691e9e1))
    - Frame for `gix repo exclude query` ([`a331314`](https://github.com/GitoxideLabs/gitoxide/commit/a331314758629a93ba036245a5dd03cf4109dc52))
    - Refactor ([`21d4076`](https://github.com/GitoxideLabs/gitoxide/commit/21d407638285b728d0c64fabf2abe0e1948e9bec))
    - The first indication that directory-based excludes work ([`e868acc`](https://github.com/GitoxideLabs/gitoxide/commit/e868acce2e7c3e2501497bf630e3a54f349ad38e))
    - Various name changes for more convenient API ([`5480159`](https://github.com/GitoxideLabs/gitoxide/commit/54801592488416ef2bb0f34c5061b62189c35c5e))
    - Use bstr intead of [u8] ([`9380e99`](https://github.com/GitoxideLabs/gitoxide/commit/9380e9990065897e318b040f49b3c9a6de8bebb1))
    - Use `git-path` crate instead of `git_features::path` ([`47e607d`](https://github.com/GitoxideLabs/gitoxide/commit/47e607dc256a43a3411406c645eb7ff04239dd3a))
    - Copy all existing functions from git-features::path to git-path:: ([`725e198`](https://github.com/GitoxideLabs/gitoxide/commit/725e1985dc521d01ff9e1e89b6468ef62fc09656))
    - Add empty git-path crate ([`8d13f81`](https://github.com/GitoxideLabs/gitoxide/commit/8d13f81068b4663d322002a9617d39b307b63469))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - `realpath()` handles `cwd` internally ([`dfa1e05`](https://github.com/GitoxideLabs/gitoxide/commit/dfa1e05d3c983f1e8b1cb3b80d03608341187883))
 * **[#422](https://github.com/GitoxideLabs/gitoxide/issues/422)**
    - Prepare changelog ([`de2d587`](https://github.com/GitoxideLabs/gitoxide/commit/de2d5874b8d75c53165a9fc3ed35e2b37142bf52))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
    - Add `is_absolute()` for git-style absolute checks ([`f58a043`](https://github.com/GitoxideLabs/gitoxide/commit/f58a043273b8e15afd01aac71f33652783baf462))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
    - Add support for `wasi` ([`523418f`](https://github.com/GitoxideLabs/gitoxide/commit/523418f69030faa0add6472b14333e9aafc69f56))
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
    - Adjust to renamining of `git-worktree` to `gix-worktree` ([`108bb1a`](https://github.com/GitoxideLabs/gitoxide/commit/108bb1a634f4828853fb590e9fc125f79441dd38))
    - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
    - Adjust to renaming of `git-date` to `gix-date` ([`9a79ff2`](https://github.com/GitoxideLabs/gitoxide/commit/9a79ff2d5cc74c1efad9f41e21095ae498cce00b))
    - Adjust to renaming of `git-pathspec` to `gix-pathspec` ([`37f7c6b`](https://github.com/GitoxideLabs/gitoxide/commit/37f7c6b9070e118604aa3fc0b38530699dcfec6e))
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
    - Rename `git-path` to `gix-path` ([`9fe8e83`](https://github.com/GitoxideLabs/gitoxide/commit/9fe8e8389c0ba677f31356c26a375e694e4d1f64))
    - Adjust to rename of `git-config-value` to `gix-config-value` ([`622b3e1`](https://github.com/GitoxideLabs/gitoxide/commit/622b3e1d0bffa0f8db73697960f9712024fac430))
    - Merge branch 'git-pack-wasm' ([`4bc19d1`](https://github.com/GitoxideLabs/gitoxide/commit/4bc19d104233a3e3d3d2768c0e9b9ad027cc34c0))
    - CI validates WASM support ([`0d4b804`](https://github.com/GitoxideLabs/gitoxide/commit/0d4b804171acd307bdac6eecd3b49bd8b2fb2968))
    - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
    - Prepare changelogs prior to release ([`7c846d2`](https://github.com/GitoxideLabs/gitoxide/commit/7c846d2102dc767366771925212712ef8cc9bf07))
    - Merge branch 'Lioness100/main' ([`1e544e8`](https://github.com/GitoxideLabs/gitoxide/commit/1e544e82455bf9ecb5e3c2146280eaf7ecd81f16))
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - Thanks clippy ([`f1160fb`](https://github.com/GitoxideLabs/gitoxide/commit/f1160fb42acf59b37cbeda546a7079af3c9bc050))
    - Make fmt ([`747008d`](https://github.com/GitoxideLabs/gitoxide/commit/747008d9d370844574dda94e5bec1648c4deb57e))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Merge branch 'discovery-fix' ([`689752e`](https://github.com/GitoxideLabs/gitoxide/commit/689752e67e8895b3d40d335e5778d8a90ec89c4c))
    - `.` substitution is only done if the input was relative. ([`745d926`](https://github.com/GitoxideLabs/gitoxide/commit/745d92636f8a3436ded0c9da21beb92182341998))
    - `normalize()` would fail to interpret `../` correctly and end up in an invalid path. ([`92d5d13`](https://github.com/GitoxideLabs/gitoxide/commit/92d5d133e17c6b79400ec57b55ccd5337f3796b7))
    - Merge branch 'path-normalize' ([`805329a`](https://github.com/GitoxideLabs/gitoxide/commit/805329a0a5f6543bbc1d5885977b47bf7baa7f08))
    - Rename tests/convert/normalize.rs ([`8ab47bb`](https://github.com/GitoxideLabs/gitoxide/commit/8ab47bbdac44c0fa738215d3cc457eb3b6f30504))
    - Rename absolutize() to normalize() ([`37cab07`](https://github.com/GitoxideLabs/gitoxide/commit/37cab07f283a368f323604372c84475d73d6c258))
    - Add `os_string_into_bstring()` as sibling of `os_str_into_bstr()`. ([`25e795f`](https://github.com/GitoxideLabs/gitoxide/commit/25e795f4fe858d646ae7a3c4706e14a3837c3e66))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'discover-rel-path' ([`5f908fb`](https://github.com/GitoxideLabs/gitoxide/commit/5f908fb86857d565715b9b0b8b453b29273fb022))
    - Improve documentation to clarify intent ([`b8f73aa`](https://github.com/GitoxideLabs/gitoxide/commit/b8f73aa5afe3f7aefa5627d7708e4c7e7da950a2))
    - Merge branch 'cwd-consistency' ([`ea7c6a3`](https://github.com/GitoxideLabs/gitoxide/commit/ea7c6a3b069c9e13905b51b87538c57ba9182dca))
    - Adapt to changes in `git-discover` and `git-path` and `git-odb` ([`98c2501`](https://github.com/GitoxideLabs/gitoxide/commit/98c250175a39598b9d37613c43dda2299da8eff3))
    - `absolutize()` now takes a mandatory `current_dir()` parameter and returns `Option<path>` ([`7dbab1c`](https://github.com/GitoxideLabs/gitoxide/commit/7dbab1c62c49822983c59be0443478f7b4fecbca))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Release git-path v0.4.2, git-config-value v0.7.0 ([`c48fb31`](https://github.com/GitoxideLabs/gitoxide/commit/c48fb3107d29f9a06868b0c6de40567063a656d1))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-path v0.4.1 ([`5e82346`](https://github.com/GitoxideLabs/gitoxide/commit/5e823462b3deb904f5d6154a7bf114cef1988224))
    - Handle `.` specifically in `absolutize()`. ([`9171adb`](https://github.com/GitoxideLabs/gitoxide/commit/9171adb796b38b08cae9bdd375b16a59a8166a1c))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Prepare changelog prior to release ([`3c50625`](https://github.com/GitoxideLabs/gitoxide/commit/3c50625fa51350ec885b0f38ec9e92f9444df0f9))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Make fmt ([`0700b09`](https://github.com/GitoxideLabs/gitoxide/commit/0700b09d6828849fa2470df89af1f75a67bfb27d))
    - Fix docs ([`4f8e3b1`](https://github.com/GitoxideLabs/gitoxide/commit/4f8e3b169e57d599439c7abc861c82c08bcd92e3))
    - Thanks clippy ([`7a2a31e`](https://github.com/GitoxideLabs/gitoxide/commit/7a2a31e5758a2be8434f22cd9401ac00539f2bd9))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`0e9df36`](https://github.com/GitoxideLabs/gitoxide/commit/0e9df364c4cddf006b1de18b8d167319b7cc1186))
    - Generally avoid using `target_os = "windows"` in favor of `cfg(windows)` and negations ([`91d5402`](https://github.com/GitoxideLabs/gitoxide/commit/91d54026a61c2aae5e3e1341d271acf16478cd83))
    - Use git_path::realpath in all places that allow it right now ([`229dc91`](https://github.com/GitoxideLabs/gitoxide/commit/229dc917fc7d9241b85e5818260a6fbdd3a5daaa))
    - Avoid unwraps in tests as they are now stable ([`efa1423`](https://github.com/GitoxideLabs/gitoxide/commit/efa14234c352b6b8417f0a42fc946e88f2eb52d3))
    - Remove canonicalized-path abstraction ([`9496e55`](https://github.com/GitoxideLabs/gitoxide/commit/9496e5512975825efebe0db86335d0d2dc8c9095))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
    - Fix git-paths tests; improve error handling. ([`9c00504`](https://github.com/GitoxideLabs/gitoxide/commit/9c0050451f634a54e610c86199b5d7d393378878))
    - Docs for git-path ([`a520092`](https://github.com/GitoxideLabs/gitoxide/commit/a52009244c9b1059ebb3d5dd472c25f9c49691f3))
    - Remove `git-config` test utilities from `git-path`. ([`c9933c0`](https://github.com/GitoxideLabs/gitoxide/commit/c9933c0b0f51d21dc8244b2acc33d7dc8a33f6ce))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Update changelogs prior to release ([`bb424f5`](https://github.com/GitoxideLabs/gitoxide/commit/bb424f51068b8a8e762696890a55ab48900ab980))
    - Make fmt ([`c665aef`](https://github.com/GitoxideLabs/gitoxide/commit/c665aef4270c5ee54da89ee015cc0affd6337608))
    - Merge branch 'main' into svetli-n-cont_include_if ([`315c87e`](https://github.com/GitoxideLabs/gitoxide/commit/315c87e18c6cac0fafa7b4e59fdd3c076a58a45a))
    - Make `realpath()` easier to use by introducing `realpath_opt()`. ([`266d437`](https://github.com/GitoxideLabs/gitoxide/commit/266d4379e9132fd7dd21e6c8fccb36e125069d6e))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Release git-path v0.1.3, git-discover v0.1.2, git-repository v0.18.1, cargo-smart-release v0.10.1 ([`b7399cc`](https://github.com/GitoxideLabs/gitoxide/commit/b7399cc44ee419355a649a7b0ba7b352cd48b400))
    - Prepare for smart-release release ([`2f74cb0`](https://github.com/GitoxideLabs/gitoxide/commit/2f74cb05e9b2399355af07517fe3c14e4e8724c5))
    - Adjust git-path size limits ([`5ac8a3b`](https://github.com/GitoxideLabs/gitoxide/commit/5ac8a3b58e0f61d4801a6f4dbd011f757208dbac))
    - Release git-path v0.1.2, git-sec v0.1.1, git-config v0.4.0, git-discover v0.1.1, git-pack v0.19.1, git-repository v0.18.0, cargo-smart-release v0.10.0, safety bump 2 crates ([`ceb6dff`](https://github.com/GitoxideLabs/gitoxide/commit/ceb6dff13362a2b4318a551893217c1d11643b9f))
    - Merge branch 'svetli-n-git_includeif' ([`cf24fbe`](https://github.com/GitoxideLabs/gitoxide/commit/cf24fbe4b62d67b06138243d470dcc1805ebd55b))
    - Remove forbid missing_docs ([`23acebb`](https://github.com/GitoxideLabs/gitoxide/commit/23acebb8e9e53d89e7f629ab690253610358b0bb))
    - Merge branch 'main' into git_includeif ([`229d938`](https://github.com/GitoxideLabs/gitoxide/commit/229d9383bef8844111d2bf3c406a2ea570109c8b))
    - Declare `git-path` usable ([`496594d`](https://github.com/GitoxideLabs/gitoxide/commit/496594d2d8b4216b51cfbd97805834c71c030c75))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Make fmt ([`e043807`](https://github.com/GitoxideLabs/gitoxide/commit/e043807abf364ca46d00760e2f281528efe20c75))
    - Fix create_symlink ([`714db70`](https://github.com/GitoxideLabs/gitoxide/commit/714db70f02134c7f53dc7ba0461f43a0d6b659e9))
    - Add includeIf test with symlink. ([`5d74404`](https://github.com/GitoxideLabs/gitoxide/commit/5d744049286632f3141ec07fa3f128093480d1c0))
    - Fix realpath tests. ([`0426f4d`](https://github.com/GitoxideLabs/gitoxide/commit/0426f4deb5d73fd88529530f9a6d01ba55eeadc4))
    - Refactor real_path tests. ([`b696849`](https://github.com/GitoxideLabs/gitoxide/commit/b696849e5fd210da397b0e7a3b26a63314d87607))
    - Refactor real_path tests. ([`8ade69f`](https://github.com/GitoxideLabs/gitoxide/commit/8ade69fbddfa5d0be3bbe761210e49be647c3356))
    - Fix windows (probably) ([`c980014`](https://github.com/GitoxideLabs/gitoxide/commit/c980014206ff071bc4f351416bb14995ac739e1b))
    - Thanks clippy ([`da13aff`](https://github.com/GitoxideLabs/gitoxide/commit/da13affabe34c3d691b18a70ce61eb00319668c5))
    - Refactor ([`6bba054`](https://github.com/GitoxideLabs/gitoxide/commit/6bba054a9a87219a7f94c155058fda5a3e6dffa6))
    - Turn recursion into loop ([`9b83c2c`](https://github.com/GitoxideLabs/gitoxide/commit/9b83c2c233d41034796694d000bed10d45f40c92))
    - Refactor ([`1ca0540`](https://github.com/GitoxideLabs/gitoxide/commit/1ca0540d170dcb8066a9141ce97631fcb9f2d5ae))
    - Refactor ([`1f6ecd2`](https://github.com/GitoxideLabs/gitoxide/commit/1f6ecd2ba91a34171d708ab7cb9414e853face95))
    - Refactor ([`5efb972`](https://github.com/GitoxideLabs/gitoxide/commit/5efb97251a9bf9e342d28bcbde27b0e69b0b7849))
    - Refactor ([`353c245`](https://github.com/GitoxideLabs/gitoxide/commit/353c2455dc01cf342b1186f0be263a87952b70be))
    - Put `realpath` into its own module ([`d142e01`](https://github.com/GitoxideLabs/gitoxide/commit/d142e01445ef545bd8284d3899d7e68f578943e9))
    - Refactor ([`50583f0`](https://github.com/GitoxideLabs/gitoxide/commit/50583f083be7ba890f7727a6491cbacf8b87ebe4))
    - Rename `real_path()` to `realpath()` ([`478ff6c`](https://github.com/GitoxideLabs/gitoxide/commit/478ff6caa630970847094fc11af10a6b69d72c34))
    - Refactor ([`8f1daf5`](https://github.com/GitoxideLabs/gitoxide/commit/8f1daf55c0027ec124bc6672ec545275065af9a7))
    - Fix linux test ([`8a36810`](https://github.com/GitoxideLabs/gitoxide/commit/8a368102c89161006cad343839105d3a5ff284e2))
    - Fix windows test. ([`1afb2da`](https://github.com/GitoxideLabs/gitoxide/commit/1afb2daa6704cc0c2efd9437dff5518ea3e64429))
    - Temp ignore real_path_tests. ([`c2f5db9`](https://github.com/GitoxideLabs/gitoxide/commit/c2f5db9a3fcc7bdcdd84cdda30d970bdcedaff2a))
    - Windows absolute path. ([`8dc33cc`](https://github.com/GitoxideLabs/gitoxide/commit/8dc33ccd1f5886b5e3f23eac5d6381473c386c2f))
    - Windows absolute path. ([`070f8c7`](https://github.com/GitoxideLabs/gitoxide/commit/070f8c79a54141d3b3064622ac7b528a24875d4f))
    - Windows absolute path. ([`cefc8fb`](https://github.com/GitoxideLabs/gitoxide/commit/cefc8fbfbb591fe714ffd87f39d0a7ca00e4a754))
    - Windows absolute path. ([`31a71f3`](https://github.com/GitoxideLabs/gitoxide/commit/31a71f37d596a3a7a7279d4b6e2508c32383b2b6))
    - Fix merge. ([`f2b46df`](https://github.com/GitoxideLabs/gitoxide/commit/f2b46dfbf73387d4501a7bf5039cb80ac4cb8d9c))
    - Merge branch 'main' into git_includeif ([`b1bfc8f`](https://github.com/GitoxideLabs/gitoxide/commit/b1bfc8fe8efb6d8941f54dddd0fcad99aa13ed6c))
    - Add custom tempdir in. ([`8bfd52a`](https://github.com/GitoxideLabs/gitoxide/commit/8bfd52a65fcecb33ae69917a67c48027f8fb3dff))
    - Merge branch 'basic-worktree-support' ([`e058bda`](https://github.com/GitoxideLabs/gitoxide/commit/e058bdabf8449b6a6fdff851e3929137d9b71568))
    - Thanks clippy ([`a084951`](https://github.com/GitoxideLabs/gitoxide/commit/a084951c72818d7cb2061053078793213890c899))
    - Temp ignore real_path_tests. ([`27f4bfc`](https://github.com/GitoxideLabs/gitoxide/commit/27f4bfcb2fba45bd02d1977094acb31b7b989cac))
    - Windows fix. ([`ce0b408`](https://github.com/GitoxideLabs/gitoxide/commit/ce0b408fcdeae80d6c9263955f70a00ead3841e1))
    - Windows fix. ([`25dd319`](https://github.com/GitoxideLabs/gitoxide/commit/25dd319a2b46327fb553f824619311484726c742))
    - Windows fix. ([`61bc0e7`](https://github.com/GitoxideLabs/gitoxide/commit/61bc0e776b9b02fdd36df6c0f54aecae63bf5895))
    - Merge branch 'main' into git_includeif ([`05eb340`](https://github.com/GitoxideLabs/gitoxide/commit/05eb34023933918c51c03cf2afd774db89cc5a33))
    - Handle windows path prefix. ([`1723236`](https://github.com/GitoxideLabs/gitoxide/commit/1723236377db483b09f123a5c24c949afa285b8a))
    - Max symlinks exceeded test. ([`cfff300`](https://github.com/GitoxideLabs/gitoxide/commit/cfff30075d87045bf9def697c417a3eb46b4b215))
    - Use thiserror in `real_path()` ([`2bd7a44`](https://github.com/GitoxideLabs/gitoxide/commit/2bd7a441beb7e0a86169ec89ca56a8ba448fbf2b))
    - Input_path is Iterator. ([`c993d78`](https://github.com/GitoxideLabs/gitoxide/commit/c993d7826fcf76ddaddffca619b4d35555b6636c))
    - Real_path wip ([`3890a61`](https://github.com/GitoxideLabs/gitoxide/commit/3890a6149683663b16dccdc3b50e2aab7eb4e048))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Make fmt ([`251b6df`](https://github.com/GitoxideLabs/gitoxide/commit/251b6df5dbdda24b7bdc452085f808f3acef69d8))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Release git-path v0.1.0 ([`ca019fc`](https://github.com/GitoxideLabs/gitoxide/commit/ca019fca03c4ea0d70fabbf09808732925b58077))
</details>

## 0.7.0 (2022-12-19)

<csr-id-8ab47bbdac44c0fa738215d3cc457eb3b6f30504/>
<csr-id-37cab07f283a368f323604372c84475d73d6c258/>

### New Features

 - <csr-id-25e795f4fe858d646ae7a3c4706e14a3837c3e66/> Add `os_string_into_bstring()` as sibling of `os_str_into_bstr()`.

### Bug Fixes

 - <csr-id-745d92636f8a3436ded0c9da21beb92182341998/> `.` substitution is only done if the input was relative.
   Previously it was possible to have `/a/b/../b` and a CWD of `/a/b`
   replaced with `.` even though that clearly isn't what the user provided.
   
   Now the `.` resubstitution only happens when it's in the interest
   of the caller.
 - <csr-id-92d5d133e17c6b79400ec57b55ccd5337f3796b7/> `normalize()` would fail to interpret `../` correctly and end up in an invalid path.
   This is now fixed and should never happen again thanks to the addition
   of a missing test.

### Refactor

- <csr-id-8ab47bbdac44c0fa738215d3cc457eb3b6f30504/> rename tests/convert/normalize.rs
  This renames the test module to match the new function name.

### Refactor (BREAKING)

- <csr-id-37cab07f283a368f323604372c84475d73d6c258/> Rename absolutize() to normalize()
  The name absolutize implies strongly that the returned path will be
  absolute, but the function only converts relative paths to absolute under a
  few specific circumstances.

  The new name, normalize(), is inspired by Python's os.path.normpath(),
  Java's java.nio.file.Path.normalize(), Node's Path.normalize(), and maybe
  some others which have similar semantics to this function.

## 0.6.0 (2022-11-21)

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

 - <csr-id-7dbab1c62c49822983c59be0443478f7b4fecbca/> `absolutize()` now takes a mandatory `current_dir()` parameter and returns `Option<path>`
   Previously the function was willing to return an empty path despite it
   being invalid. With the `current_dir` being required, this won't be the
   case anymore and will yield logically consistent results in all cases.
   
   This forces the caller to deal with the relative path being invalid
   or crafted to produce some other path, maybe to bypass sanity checks.

## 0.5.0 (2022-09-20)

### New Features

 - <csr-id-523418f69030faa0add6472b14333e9aafc69f56/> add support for `wasi`
   This allows path conversions there to be just as efficient as on unix.
   
   This was adopted from [a PR in the
   hexlix-editor](https://github.com/helix-editor/helix/pull/3890/files#diff-504515b66023120e75a921cd56a932aed76c0ff62593fbb69d92e0ef65089501R47).

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

## 0.4.2 (2022-08-29)

### New Features

 - <csr-id-f58a043273b8e15afd01aac71f33652783baf462/> add `is_absolute()` for git-style absolute checks
   This essentially means that starting slashes are always absolute, even
   on windows.

## 0.4.1 (2022-08-23)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
- <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### Bug Fixes

 - <csr-id-9171adb796b38b08cae9bdd375b16a59a8166a1c/> Handle `.` specifically in `absolutize()`.
   Previously, absolutizing `./../../` would lead to one path component
   of the `../` to be ignored as `.` was popped successfully, not realizing
   that it is a no-op.
   
   This could lead to problems with repository discovery if `.` was passed.

## 0.4.0 (2022-07-22)

### Changed (BREAKING)

 - <csr-id-dfa1e05d3c983f1e8b1cb3b80d03608341187883/> `realpath()` handles `cwd` internally
   This makes for more convenient usage in the common case.

## 0.3.0 (2022-06-19)

### Bug Fixes (BREAKING)

 - <csr-id-c9933c0b0f51d21dc8244b2acc33d7dc8a33f6ce/> Remove `gix-config` test utilities from `gix-path`.

## 0.2.0 (2022-06-13)

### New Features (BREAKING)

 - <csr-id-266d4379e9132fd7dd21e6c8fccb36e125069d6e/> Make `realpath()` easier to use by introducing `realpath_opt()`.
   That way there is consistency about how many symlinks to follow.

## 0.1.3 (2022-05-23)

A maintenance release without user-facing changes.

## 0.1.2 (2022-05-21)

A maintenance release without user-facing changes.

## 0.1.1 (2022-05-18)

<csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/>

### New Features

 - <csr-id-35f146a8573dcc9a1de3230373c0cf0794c6b897/> Add `absolutize_components()`
   It helps to cleanup paths a little which comes in handy when dealing
   with `commondir` appended paths.

### Other

- <csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/> :discover()` now returns the shortest path.
  If and only if it canonicalized the source path. That way, users will
  still get a familiar path. This is due to `parent()` not operating
  in the file system, which otherwise would be equivalent to `..`,
  but that's not how we work.

  Maybe we should overhaul the way this works to use `../` instead
  and just 'absolutize' the path later (std::path::absolute()) is
  on the way for that.

## 0.1.0 (2022-04-28)

<csr-id-54801592488416ef2bb0f34c5061b62189c35c5e/>

### Refactor (BREAKING)

- <csr-id-54801592488416ef2bb0f34c5061b62189c35c5e/> various name changes for more convenient API

## 0.0.0 (2022-03-31)

An empty crate without any content to reserve the name for the gitoxide project.

