# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.21.0 (2026-09-25)

### Chore

 - <csr-id-6de11757e69b932d78aa437e5e47d8043d43b78e/> make docs-rs work, and publishing

### New Features

 - <csr-id-faf85101e1b2c67ca8bbaa60bc6ae360034694c1/> redact unstable values in diagnostic snapshots
   <!-- agent -->
   Complete error snapshots expose useful context and causes, but temporary
   paths, dynamically assigned ports, object IDs, and platform-specific I/O
   messages make their output unstable.
   
   Add `redact_debug_snapshot()` to apply explicit replacements, reuse object-ID
   normalization, and render OS errors by their portable `ErrorKind`. Handle
   plain and debug-escaped Windows paths while retaining meaningful suffixes
   and unrelated backslashes. Return an owned `Debug` value so `insta` displays
   the diagnostic without additional quoting or newline escaping.
   
   Cover redaction, object identity, and OS error formatting, and convert the
   environment, SBOM argument, and Rust fixture error assertions to complete
   inline snapshots. The helper is available before downstream diagnostic
   snapshot migrations need it.

### Bug Fixes

 - <csr-id-4e0f8ff9b920e75d9554eb58c76e396e50502346/> Keep rust workspace tests inside disposable repositories and isolated environments
   <!-- agent -->
   Direct Git launches inherited repository selectors and user configuration even
   when tests supplied a fixture working directory. Tests of default-environment
   APIs and local Git transports also shared the runner's environment. A few
   journey tests wrote beneath source directories or used the source checkout as
   the repository under test.
   
   Use the shared `gix-testtools` Git command builder for subprocess setup, isolated
   repository options for fixtures, and isolated child processes where the real
   environment-reading API must be exercised. Scope CWD changes, copy the fixture
   used by an object-write test, and run shell journeys through `jtt run`. Keep
   journey worktrees and example output within their disposable sandboxes and
   replace the attributes checkout test with a representative fixture repository.
   Prompt examples also run in isolated children and must build successfully; the
   old tests could ignore build failures and execute stale cached binaries.
   
   The affected Rust crate suites, internal test-tool build, and `max-pure` journey
   suite pass from a source copy without Git metadata. Signing and Git-daemon
   checks use only disposable keys, repositories, and local sockets.
 - <csr-id-42c46f4b70c3671c8455bf36fc2e05d28ddb6e72/> isolate fixtures from inherited Git templates
   <!-- agent -->
   Fixture commands already ignore external Git configuration, but inherited
   `GIT_TEMPLATE_DIR` could still inject personal template files into repositories
   created by `git init`, making fixtures depend on the caller's environment.
   
   Clear that variable in the shared command setup used by fixture scripts and
   isolated Git helpers. Keep Git's installed templates available because existing
   fixtures rely on their standard files and directory layout.
   
   Document the isolation behavior and add a regression that supplies an external
   template containing a marker file, then verifies that `git init` does not copy
   it into the fixture repository.
 - <csr-id-13d784dce6f30780a6b0d20011161bedaf3a3c07/> keep fixture scripts isolated when they set GIT_CONFIG_COUNT themselves
   The isolation configuration (no signing, no auto-maintenance, `init.defaultBranch=main`)
   reached scripts only as `GIT_CONFIG_COUNT` variables, so a script exporting its own
   `GIT_CONFIG_COUNT` replaced all of it. With maintenance back on, each commit in such a
   script spawned a detached `git maintenance run --auto` that could outlive the script and
   mutate the fixture while it was being copied, failing tests with a bare ENOENT.
   
   - the isolation now lives in a global configuration file under
     `tests/fixtures/generated-do-not-edit/`, written once per process; a script's own
     `GIT_CONFIG_COUNT` layers on top of it
   - ambient `GIT_CONFIG_COUNT` is removed from the script environment, as it used to be
     overwritten
   - a script or `git()` call that writes global configuration fails with a clear message
     and the file is restored, so the mistake stays contained to the offender

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

 - 22 commits contributed to the release over the course of 32 calendar days.
 - 33 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Make docs-rs work, and publishing ([`6de1175`](https://github.com/GitoxideLabs/gitoxide/commit/6de11757e69b932d78aa437e5e47d8043d43b78e))
    - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
    - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
    - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
    - Redact unstable values in diagnostic snapshots ([`faf8510`](https://github.com/GitoxideLabs/gitoxide/commit/faf85101e1b2c67ca8bbaa60bc6ae360034694c1))
    - Merge pull request #2999 from GitoxideLabs/sbom ([`77c8cd9`](https://github.com/GitoxideLabs/gitoxide/commit/77c8cd956c08a2757318d3a0e6ef30d7fa71286e))
    - Add configurable CycloneDX and SPDX SBOM generation with `jtt` ([`40b8e5e`](https://github.com/GitoxideLabs/gitoxide/commit/40b8e5e28120ed5af9a3899c3555069c3bce670d))
    - Merge pull request #2989 from GitoxideLabs/error-conversion-review ([`4b9ff51`](https://github.com/GitoxideLabs/gitoxide/commit/4b9ff511a49f7963e97a669ca82c6f6e833d8ea2))
    - Merge pull request #2990 from GitoxideLabs/various-improvements ([`c609062`](https://github.com/GitoxideLabs/gitoxide/commit/c609062db5e7030e922a7554143a6bfc52ba317c))
    - Keep rust workspace tests inside disposable repositories and isolated environments ([`4e0f8ff`](https://github.com/GitoxideLabs/gitoxide/commit/4e0f8ff9b920e75d9554eb58c76e396e50502346))
    - Isolate all Git subprocess helpers used by tests ([`a1834d6`](https://github.com/GitoxideLabs/gitoxide/commit/a1834d674d25c9bd52ac3fa6b4a5dc0ee31db033))
    - Merge pull request #2984 from justonemorenight/fix/config-path-tilde-parity ([`92b6508`](https://github.com/GitoxideLabs/gitoxide/commit/92b65081dee8d9f744485e2bbb267f222745c29f))
    - Isolate fixtures from inherited Git templates ([`42c46f4`](https://github.com/GitoxideLabs/gitoxide/commit/42c46f4b70c3671c8455bf36fc2e05d28ddb6e72))
    - Merge pull request #2982 from mtsgrd/testtools-config-file-isolation ([`0cd9319`](https://github.com/GitoxideLabs/gitoxide/commit/0cd9319555ff2b30d8eca95eb3befba018b15de5))
    - Review ([`bc9185d`](https://github.com/GitoxideLabs/gitoxide/commit/bc9185d33bc24e85165b53b546d87aa6f7c0de95))
    - Keep fixture scripts isolated when they set GIT_CONFIG_COUNT themselves ([`13d784d`](https://github.com/GitoxideLabs/gitoxide/commit/13d784dce6f30780a6b0d20011161bedaf3a3c07))
    - Merge pull request #2958 from GitoxideLabs/sign-on-windows ([`c16300c`](https://github.com/GitoxideLabs/gitoxide/commit/c16300cf781df9b580132bfa397d8c9d033ea7ad))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
    - Merge pull request #2955 from GitoxideLabs/transport-url-encoding ([`7e35849`](https://github.com/GitoxideLabs/gitoxide/commit/7e35849b36646cff9722f6906a4527d64818a374))
    - Release gix-path v0.12.6, gix-error v0.3.2, gix-command v0.10.1, gix-transport v0.59.2 ([`888677a`](https://github.com/GitoxideLabs/gitoxide/commit/888677ad2d63a2e3930a02add2de0b4b667a5581))
    - Merge pull request #2932 from GitoxideLabs/fundamental-types-comp ([`6704303`](https://github.com/GitoxideLabs/gitoxide/commit/6704303ed5ef3403b129e2b6cc4a9214432ffd03))
</details>

## 0.20.0 (2026-08-23)

### New Features

 - <csr-id-6a30419657d1c3dd8f3b1b26b209f135006dc7e6/> allow requiring archived scripted fixtures
   <!-- agent -->
   
   Add version-aware fixture loaders:
   
   - `scripted_fixture_read_only_with_git_version()`
   - `scripted_fixture_read_only_with_args_with_git_version()`
   
   Both use normal fixture generation when the installed Git version satisfies
   the caller-provided predicate. For incompatible versions, they require an
   identity-matching archive, ignore `GIX_TEST_IGNORE_ARCHIVES`, and return `None`
   instead of running the fixture script when no usable archive exists.

### Bug Fixes

 - <csr-id-4314d09dc44f2d474d03743f5e6841989084a479/> avoid calling `git --version` each time a metadata directory is created.

### New Features (BREAKING)

 - <csr-id-677eb8398d3e19c290e224d2581a89ac56b7feec/> expand gix-testtools fixture support
   Note that I did only skim through `git.rs`,
   considering the details of the implementation secondary to the outcome.
   
   `gix.rs` was refactored quite a bit, helping to provide more utilities
   in other plumbing crates to help with this 'gix' avoidance.
   
   <!-- agent -->
   Add deterministic repository snapshots covering HEAD, references, reachable
   commits, every index stage, conflict-free index trees, and exact worktree
   contents. Provide a portable form which omits platform-specific filesystem modes
   for stable cross-platform assertions.
   
   Add writable Git-version-aware fixture APIs for copy and execute creation modes.
   Require archived fixtures when the selected Git is incompatible, without running
   unsupported scripts or post-processing.
   
   Use the Git executable selected by gix-path consistently for version checks,
   helper commands, and fixture scripts, including PATH selection for scripts.
   Harden signing fixtures with command-compatible paths, isolated homes, and
   complete SSH key pairs.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 89 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.3.1, gix-hash v0.26.2, gix-object v0.64.1, gix-ref v0.67.1, gix-packetline v0.22.1, gix-pack v0.74.1, gix-testtools v0.20.0 ([`e52fe9d`](https://github.com/GitoxideLabs/gitoxide/commit/e52fe9d03e82437a25bdfb1098e7046ec7e1b558))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2905 from GitoxideLabs/various-improvements ([`f3bbfad`](https://github.com/GitoxideLabs/gitoxide/commit/f3bbfadd4b4f1d72c85c62eb3d7ae337c922f945))
    - Expand gix-testtools fixture support ([`677eb83`](https://github.com/GitoxideLabs/gitoxide/commit/677eb8398d3e19c290e224d2581a89ac56b7feec))
    - Add reusable signature test fixtures ([`24dabe2`](https://github.com/GitoxideLabs/gitoxide/commit/24dabe207e3acb19b1e20ad5fc557089f3de17f7))
    - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))
    - Release gix-actor v0.41.2, gix-features v0.49.0, gix-hash v0.26.0, gix-hashtable v0.16.0, gix-object v0.63.0, gix-glob v0.27.0, gix-attributes v0.34.0, gix-packetline v0.22.0, gix-filter v0.33.0, gix-fs v0.22.0, gix-chunk v0.7.3, gix-commitgraph v0.38.0, gix-revwalk v0.34.0, gix-traverse v0.60.0, gix-worktree-stream v0.35.0, gix-archive v0.35.0, gix-bitmap v0.3.3, gix-tempfile v24.0.0, gix-lock v24.0.0, gix-index v0.54.0, gix-pathspec v0.19.0, gix-ignore v0.22.0, gix-worktree v0.55.0, gix-imara-diff v0.2.4, gix-diff v0.66.0, gix-blame v0.16.0, gix-ref v0.66.0, gix-config v0.59.0, gix-discover v0.54.0, gix-dir v0.28.0, gix-mailmap v0.33.2, gix-revision v0.48.0, gix-merge v0.19.0, gix-negotiate v0.34.0, gix-zlib v0.1.0, gix-pack v0.73.0, gix-odb v0.83.0, gix-refspec v0.44.0, gix-shallow v0.13.0, gix-transport v0.58.0, gix-protocol v0.64.0, gix-status v0.33.0, gix-submodule v0.33.0, gix-worktree-state v0.33.0, gix v0.86.0, gix-fsck v0.24.0, gitoxide-core v0.60.0, gix-tix v0.1.0, gitoxide v0.56.0, safety bump 40 crates ([`842bc44`](https://github.com/GitoxideLabs/gitoxide/commit/842bc447e3aeacf5d9d36f7f8a01068eda4b7999))
    - Merge pull request #2732 from GitoxideLabs/testools-use-archive ([`a3e9a77`](https://github.com/GitoxideLabs/gitoxide/commit/a3e9a7742a3926baeed64f90f6906df6a9942647))
    - Avoid calling `git --version` each time a metadata directory is created. ([`4314d09`](https://github.com/GitoxideLabs/gitoxide/commit/4314d09dc44f2d474d03743f5e6841989084a479))
    - Allow requiring archived scripted fixtures ([`6a30419`](https://github.com/GitoxideLabs/gitoxide/commit/6a30419657d1c3dd8f3b1b26b209f135006dc7e6))
    - Merge pull request #2721 from GitoxideLabs/remove-kstring ([`e70732a`](https://github.com/GitoxideLabs/gitoxide/commit/e70732a7cad4b5dca4890d394908c858ab406906))
    - Adapt to changes in `gix-attributes` ([`e11d7a2`](https://github.com/GitoxideLabs/gitoxide/commit/e11d7a2c734882e4ddb74c82ce5fe50b46265467))
    - Merge pull request #2646 from GitoxideLabs/report ([`1b1541e`](https://github.com/GitoxideLabs/gitoxide/commit/1b1541ed7a457afd48385c1ee39113949a9f5263))
    - Release gix-date v0.15.5, gix-hashtable v0.15.2, gix-object v0.62.0, gix-attributes v0.33.2, gix-filter v0.32.0, gix-revwalk v0.33.0, gix-traverse v0.59.0, gix-worktree-stream v0.34.0, gix-archive v0.34.0, gix-tempfile v23.0.2, gix-index v0.53.0, gix-worktree v0.54.0, gix-imara-diff v0.2.3, gix-diff v0.65.0, gix-blame v0.15.0, gix-ref v0.65.0, gix-config v0.58.0, gix-discover v0.53.0, gix-dir v0.27.0, gix-revision v0.47.0, gix-merge v0.18.0, gix-negotiate v0.33.0, gix-pack v0.72.0, gix-odb v0.82.0, gix-refspec v0.43.0, gix-transport v0.57.2, gix-protocol v0.63.0, gix-status v0.32.0, gix-submodule v0.32.0, gix-worktree-state v0.32.0, gix v0.85.0, gix-fsck v0.23.0, gitoxide-core v0.59.0, gitoxide v0.55.0, safety bump 28 crates ([`6428edc`](https://github.com/GitoxideLabs/gitoxide/commit/6428edc82fc8a16d5ef34ca2d49aa6fdff3645fe))
    - Merge pull request #2628 from GitoxideLabs/dependabot/cargo/tar-0.4.46 ([`5aadd6e`](https://github.com/GitoxideLabs/gitoxide/commit/5aadd6ed92c97ac364a743d23da0151960b50e3b))
    - Bump tar from 0.4.45 to 0.4.46 ([`07d16ea`](https://github.com/GitoxideLabs/gitoxide/commit/07d16ea626fb16e6cbee48afab232b60f54d43c0))
    - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))
</details>

## 0.19.0 (2026-05-26)

### New Features

 - <csr-id-36d84982d42f5955da3e6288a4da7e152089476a/> allow fixtures to require checked-in archives`
   Add `scripted_fixture_read_only_needs_archive()` for fixtures whose generated
   output can vary in byte order across platforms or filesystems. The helper
   still validates archive identity, but bypasses GIX_TEST_IGNORE_ARCHIVES and
   re-extracts the checked-in archive so tests can opt into frozen fixture bytes
   when rerunning the producer is non-deterministic,
   for instance due to filesystem order.
 - <csr-id-fa02742da12e6f2fe18da2b16674a25f3a581648/> add `git` utility to call `git` easily like `git(cwd, "log --oneline")`
 - <csr-id-33c0a8bbe7be5ba232b884334202f09b70bab622/> add `invoke_bash` test helper()
 - <csr-id-2a5e787ffaa10e850d9131928715df177dee5c77/> Add `hash_kind_from_env()` to obtain the value of `GIX_TEST_FIXTURE_HASH`
   This is useful when tests want to inspect the hash under test.
 - <csr-id-7f50c3003ebfdcff044d142591ece1dcdab0b0ac/> use `GIX_TEST_FIXTURE_HASH` environment variable.
   For now, there is no direct way to set it, but that may change
   once we have tests that need it.
 - <csr-id-61c504dfdd6505eeeda22a6c4254c7342b78d6e3/> Add variants of `scripted_fixture_read_only` that allow for Rust post modification.
   That way, we can have the benefit of scripts *and* Rust for final tweaks.
 - <csr-id-501da557fcccabf79961e441d0bb9a3e8841b906/> add rust_fixture_read_only and rust_fixture_writable functions to gix-testtools

### Bug Fixes

 - <csr-id-cad26e94a6f711105a08e39998f0a64d0a3903ba/> disable automatic Git maintenance in gix-testtools, add `apply_git_config_by_environment` utility
   Disable maintenance.auto and gc.auto through command-scope Git config for
   gix-testtools process execution, including fixture scripts, helper-launched Git,
   cargo example builds, git-daemon wrappers, and shell helpers.
 - <csr-id-83ab103077a76683b97755b3acd59c67b83a2def/> also override XDG_CONFIG_HOME for test-scripts and commands runs.

### New Features (BREAKING)

 - <csr-id-c8253522f181385afbe0279d6c24f5eab9b73f0f/> Make `gix-testtools` independent of `gix` crates
   This means that a trait implementation has to always be passed
   for what boils down to ignore handling.
   
   The `standalone` portion of the API is also removed.
 - <csr-id-85961b590c86038b125de5546debb844414f51ca/> add `object_hash()` function and rename `hash_kind_from_env()` to `object_hash_from_env()`
   For consistency and convenience.

### Bug Fixes (BREAKING)

 - <csr-id-91c854e7b9f41738d0fde825cd474b8c00c1a49b/> remove `winnow` and replace it with hand-implemented parsers everywhere.
   This will allow for simplified maintenance and editing (both human and machine)
   down the road, and enable additional performance optimisations.
   
   Parser compbinators to me ultimately were a failed experiment as I couldn't maintain
   them anyway, with it being too difficult for me to grasp and express everything
   in its very own kind of language, with a lot of different things to consider.
   
   Note that this also removes detailed errors from all parsers that previously
   used `winnow`, with the option to re-add those if there is demand.
 - <csr-id-f4bb1063ce0b09a6d076d9c45d7f505cf31a1c85/> rename `Creation::ExecuteScipt` to `Creation::Execute`
   This is more fitting since the arrival of Rust-based fixtures.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 70 commits contributed to the release.
 - 123 days passed between releases.
 - 13 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-testtools v0.19.0 ([`8eefc31`](https://github.com/GitoxideLabs/gitoxide/commit/8eefc3153366671deaf1b70b43346b9587b89dd1))
    - Release gix-error v0.2.4, gix-date v0.15.4, gix-actor v0.41.1, gix-trace v0.1.20, gix-validate v0.11.2, gix-path v0.12.1, gix-utils v0.3.3, gix-features v0.48.1, gix-hash v0.25.1, gix-hashtable v0.15.1, gix-object v0.61.0, gix-glob v0.26.1, gix-quote v0.7.2, gix-attributes v0.33.1, gix-command v0.9.1, gix-packetline v0.21.4, gix-filter v0.31.0, gix-fs v0.21.2, gix-chunk v0.7.2, gix-commitgraph v0.37.1, gix-revwalk v0.32.0, gix-traverse v0.58.0, gix-worktree-stream v0.33.0, gix-archive v0.33.0, gix-bitmap v0.3.2, gix-tempfile v23.0.1, gix-lock v23.0.1, gix-index v0.52.0, gix-config-value v0.18.1, gix-pathspec v0.18.1, gix-ignore v0.21.1, gix-worktree v0.53.0, gix-imara-diff v0.2.2, gix-diff v0.64.0, gix-blame v0.14.0, gix-ref v0.64.0, gix-sec v0.14.1, gix-config v0.57.0, gix-prompt v0.15.1, gix-url v0.36.1, gix-credentials v0.38.1, gix-discover v0.52.0, gix-dir v0.26.0, gix-mailmap v0.33.1, gix-revision v0.46.0, gix-merge v0.17.0, gix-negotiate v0.32.0, gix-pack v0.71.0, gix-odb v0.81.0, gix-refspec v0.42.0, gix-shallow v0.12.1, gix-transport v0.57.1, gix-protocol v0.62.0, gix-status v0.31.0, gix-submodule v0.31.0, gix-worktree-state v0.31.0, gix v0.84.0, gix-fsck v0.22.0, gitoxide-core v0.58.0, gitoxide v0.54.0, safety bump 27 crates ([`10c58bb`](https://github.com/GitoxideLabs/gitoxide/commit/10c58bb56597d9335611da121aac21f9b09b6e5b))
    - Merge pull request #2612 from GitoxideLabs/improvements ([`4377485`](https://github.com/GitoxideLabs/gitoxide/commit/43774856ea44f6ec2176802aca0bc5facd7c7ad7))
    - `spawn_git_daemon` now spawns the git daemon on a free port automatically ([`123cdaf`](https://github.com/GitoxideLabs/gitoxide/commit/123cdaf01e0ba7eea82d5acd1f2362cb2e236a85))
    - Disable automatic Git maintenance in gix-testtools, add `apply_git_config_by_environment` utility ([`cad26e9`](https://github.com/GitoxideLabs/gitoxide/commit/cad26e94a6f711105a08e39998f0a64d0a3903ba))
    - Merge pull request #2591 from AaronMoat/untracked-extension-reading ([`85c6087`](https://github.com/GitoxideLabs/gitoxide/commit/85c608725f11c47d5236b1d825e31c9b575493e5))
    - Also override XDG_CONFIG_HOME for test-scripts and commands runs. ([`83ab103`](https://github.com/GitoxideLabs/gitoxide/commit/83ab103077a76683b97755b3acd59c67b83a2def))
    - Allow fixtures to require checked-in archives` ([`36d8498`](https://github.com/GitoxideLabs/gitoxide/commit/36d84982d42f5955da3e6288a4da7e152089476a))
    - Merge pull request #2578 from cruessler/run-gix-tests-with-sha-256 ([`2d4a6f2`](https://github.com/GitoxideLabs/gitoxide/commit/2d4a6f22bb7bf8a8f83b39aa3ccf5d1c2c30f13e))
    - Add `git` utility to call `git` easily like `git(cwd, "log --oneline")` ([`fa02742`](https://github.com/GitoxideLabs/gitoxide/commit/fa02742da12e6f2fe18da2b16674a25f3a581648))
    - Merge pull request #2590 from GitoxideLabs/independent-testtools ([`575113d`](https://github.com/GitoxideLabs/gitoxide/commit/575113dfb10b3ba12eb57f57a81b241e773968bd))
    - Adapt to changes in `gix-testtools` ([`ce9e6bd`](https://github.com/GitoxideLabs/gitoxide/commit/ce9e6bded2cb47cc9b995f8882d98c09ba5c2c8b))
    - Make `gix-testtools` independent of `gix` crates ([`c825352`](https://github.com/GitoxideLabs/gitoxide/commit/c8253522f181385afbe0279d6c24f5eab9b73f0f))
    - Merge pull request #2589 from GitoxideLabs/fix-status-in-unborn-repo ([`ba7d9a4`](https://github.com/GitoxideLabs/gitoxide/commit/ba7d9a47287e7d6184eedbe345e066716b95fd01))
    - Stabilize filter-driver example use in parallel tests to remove flake ([`db16a05`](https://github.com/GitoxideLabs/gitoxide/commit/db16a0508aa22278ceb8fa9dfa631276954a3304))
    - Merge pull request #2573 from cruessler/run-gix-traverse-tests-with-sha-256 ([`278d7ec`](https://github.com/GitoxideLabs/gitoxide/commit/278d7ec395124b3ce00db6f3e029265bfec8ccd1))
    - Review ([`1cd36be`](https://github.com/GitoxideLabs/gitoxide/commit/1cd36be4bcd9ffaf13b0ddb3305777bbf9bb8034))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Merge pull request #2559 from GitoxideLabs/fix/symlink-prefix-reuse-worktree-escape-ghsa-f89h-2fjh-2r9q ([`3af9b4a`](https://github.com/GitoxideLabs/gitoxide/commit/3af9b4adcbca28e7072b96d266b716efb483893c))
    - Release gix-fs v0.21.1 ([`d3e4c17`](https://github.com/GitoxideLabs/gitoxide/commit/d3e4c176f52a685f0405427ad7070014a6baf92b))
    - Merge pull request #2543 from cruessler/run-gix-worktree-stream-tests-with-sha-256 ([`23af41a`](https://github.com/GitoxideLabs/gitoxide/commit/23af41ab13d4fa894c2c2774a096fef5438bef7d))
    - Adapt to changes in `gix-testtools`and rename `hash_kind` -> `object_hash` ([`d9648e8`](https://github.com/GitoxideLabs/gitoxide/commit/d9648e85c53616fe2fb79e19ee0a42967127cc2f))
    - Add `object_hash()` function and rename `hash_kind_from_env()` to `object_hash_from_env()` ([`85961b5`](https://github.com/GitoxideLabs/gitoxide/commit/85961b590c86038b125de5546debb844414f51ca))
    - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))
    - Cargo fmt to fix CI ([`d1237b7`](https://github.com/GitoxideLabs/gitoxide/commit/d1237b7aaacf54a25e7a6e6d3151a6cc6715ea88))
    - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
    - Fix CI - and probably prevent `can't connect to localhost` in journey tests ([`09687eb`](https://github.com/GitoxideLabs/gitoxide/commit/09687eba6bb3e0f398e0c0bef1e3f19f5f8b8cc0))
    - Remove `winnow` and replace it with hand-implemented parsers everywhere. ([`91c854e`](https://github.com/GitoxideLabs/gitoxide/commit/91c854e7b9f41738d0fde825cd474b8c00c1a49b))
    - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))
    - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
    - Merge pull request #2511 from GitoxideLabs/improvements ([`1fc129d`](https://github.com/GitoxideLabs/gitoxide/commit/1fc129d45062703b62d4bb281c71b2223359264d))
    - Add `invoke_bash` test helper() ([`33c0a8b`](https://github.com/GitoxideLabs/gitoxide/commit/33c0a8bbe7be5ba232b884334202f09b70bab622))
    - Merge pull request #2499 from GitoxideLabs/dependabot/cargo/cargo-17cc682121 ([`44020e0`](https://github.com/GitoxideLabs/gitoxide/commit/44020e01d2f037d94a3625fbc323d203718dcc66))
    - Bump the cargo group with 26 updates ([`9d04035`](https://github.com/GitoxideLabs/gitoxide/commit/9d040352ca432b23e666eb0163267170d0926401))
    - Merge pull request #2496 from GitoxideLabs/improvements ([`2aedd14`](https://github.com/GitoxideLabs/gitoxide/commit/2aedd14c6d967022e6c4e9cc347ab86e58785216))
    - Remove unnecessary message when using fixtures ([`e68a42a`](https://github.com/GitoxideLabs/gitoxide/commit/e68a42a50e005c8be666a0d994bcf54a052bbf1c))
    - Merge pull request #2448 from cruessler/add-sha-256-to-gix-refspec ([`9a78ae2`](https://github.com/GitoxideLabs/gitoxide/commit/9a78ae2c4f0a1a290d067413787ecfdf486fa616))
    - Add `hash_kind_from_env()` to obtain the value of `GIX_TEST_FIXTURE_HASH` ([`2a5e787`](https://github.com/GitoxideLabs/gitoxide/commit/2a5e787ffaa10e850d9131928715df177dee5c77))
    - Make `hash_kind_from_env` public ([`726e1b9`](https://github.com/GitoxideLabs/gitoxide/commit/726e1b90161fec68e8f207d3c821367e5a9fecd8))
    - Merge pull request #2480 from GitoxideLabs/report ([`98bae84`](https://github.com/GitoxideLabs/gitoxide/commit/98bae84fe534879899489c6f2c5e8cfcc863116d))
    - Release gix-error v0.2.1, gix-date v0.15.1, gix-path v0.11.2, gix-features v0.46.2, gix-hash v0.23.0, gix-hashtable v0.13.0, gix-object v0.58.0, gix-packetline v0.21.2, gix-filter v0.28.0, gix-fs v0.19.2, gix-commitgraph v0.35.0, gix-revwalk v0.29.0, gix-traverse v0.55.0, gix-worktree-stream v0.30.0, gix-archive v0.30.0, gix-tempfile v21.0.2, gix-lock v21.0.2, gix-index v0.49.0, gix-pathspec v0.16.1, gix-ignore v0.19.1, gix-worktree v0.50.0, gix-diff v0.61.0, gix-blame v0.11.0, gix-ref v0.61.0, gix-sec v0.13.2, gix-config v0.54.0, gix-prompt v0.14.1, gix-credentials v0.37.1, gix-discover v0.49.0, gix-dir v0.23.0, gix-revision v0.43.0, gix-merge v0.14.0, gix-negotiate v0.29.0, gix-pack v0.68.0, gix-odb v0.78.0, gix-refspec v0.39.0, gix-shallow v0.10.0, gix-transport v0.55.1, gix-protocol v0.59.0, gix-status v0.28.0, gix-submodule v0.28.0, gix-worktree-state v0.28.0, gix v0.81.0, gix-fsck v0.19.0, gitoxide-core v0.55.0, gitoxide v0.52.0, safety bump 31 crates ([`c389a2c`](https://github.com/GitoxideLabs/gitoxide/commit/c389a2ccb32b36c1178a1352a2bb3229aef3b016))
    - Merge pull request #2454 from GitoxideLabs/dependabot/cargo/cargo-da044b9bb0 ([`6183fd0`](https://github.com/GitoxideLabs/gitoxide/commit/6183fd092d7acd43763fe15be400ce81e7172775))
    - Bump the cargo group with 68 updates ([`6bdb331`](https://github.com/GitoxideLabs/gitoxide/commit/6bdb33145e8aa81ba0dae5caafc675c591569715))
    - Merge pull request #2441 from cruessler/remove-sha-1-from-default-features ([`e8bf096`](https://github.com/GitoxideLabs/gitoxide/commit/e8bf096c07205a41089a697a9726f075d3515643))
    - Adapt to sha1 not being default feature of `gix-hash` ([`e71c703`](https://github.com/GitoxideLabs/gitoxide/commit/e71c703f0b8ca209f8aa912cbaf5aa26551496ef))
    - Merge pull request #2442 from GitoxideLabs/report ([`f7277f3`](https://github.com/GitoxideLabs/gitoxide/commit/f7277f3c9e3e5130edb714ff5bd3db06b7f589b3))
    - Release gix-error v0.2.0, gix-date v0.15.0, gix-actor v0.40.0, gix-object v0.57.0, gix-quote v0.7.0, gix-attributes v0.31.0, gix-command v0.8.0, gix-filter v0.27.0, gix-chunk v0.7.0, gix-commitgraph v0.34.0, gix-revwalk v0.28.0, gix-traverse v0.54.0, gix-worktree-stream v0.29.0, gix-archive v0.29.0, gix-bitmap v0.3.0, gix-index v0.48.0, gix-pathspec v0.16.0, gix-worktree v0.49.0, gix-diff v0.60.0, gix-blame v0.10.0, gix-ref v0.60.0, gix-config v0.53.0, gix-prompt v0.14.0, gix-url v0.35.2, gix-credentials v0.37.0, gix-discover v0.48.0, gix-dir v0.22.0, gix-mailmap v0.32.0, gix-revision v0.42.0, gix-merge v0.13.0, gix-negotiate v0.28.0, gix-pack v0.67.0, gix-odb v0.77.0, gix-refspec v0.38.0, gix-shallow v0.9.0, gix-transport v0.55.0, gix-protocol v0.58.0, gix-status v0.27.0, gix-submodule v0.27.0, gix-worktree-state v0.27.0, gix v0.80.0, gix-fsck v0.18.0, gitoxide-core v0.54.0, gitoxide v0.51.0, safety bump 42 crates ([`ecf90fc`](https://github.com/GitoxideLabs/gitoxide/commit/ecf90fccb9d43bff320c17f46fdc3f5832533a52))
    - Merge pull request #2377 from cruessler/add-sha-256-to-gix-commitgraph ([`228caf7`](https://github.com/GitoxideLabs/gitoxide/commit/228caf7191eed5fd9e2095cc5a60179b374156b5))
    - Refactor ([`8ccd760`](https://github.com/GitoxideLabs/gitoxide/commit/8ccd760621a54d14ca9386b2123d7fedb02931ac))
    - Use `GIX_TEST_FIXTURE_HASH` environment variable. ([`7f50c30`](https://github.com/GitoxideLabs/gitoxide/commit/7f50c3003ebfdcff044d142591ece1dcdab0b0ac))
    - Merge branch 'release' ([`9327b73`](https://github.com/GitoxideLabs/gitoxide/commit/9327b73785227f1322a327cb48fbb0800e1286ae))
    - Release gix-error v0.1.0, gix-date v0.14.0, gix-actor v0.39.0, gix-trace v0.1.18, gix-path v0.11.1, gix-features v0.46.1, gix-hash v0.22.1, gix-object v0.56.0, gix-quote v0.6.2, gix-attributes v0.30.1, gix-command v0.7.1, gix-packetline v0.21.1, gix-filter v0.26.0, gix-fs v0.19.1, gix-chunk v0.6.0, gix-commitgraph v0.33.0, gix-revwalk v0.27.0, gix-traverse v0.53.0, gix-worktree-stream v0.28.0, gix-archive v0.28.0, gix-bitmap v0.2.16, gix-tempfile v21.0.1, gix-lock v21.0.1, gix-index v0.47.0, gix-config-value v0.17.1, gix-pathspec v0.15.1, gix-worktree v0.48.0, gix-diff v0.59.0, gix-blame v0.9.0, gix-ref v0.59.0, gix-sec v0.13.1, gix-config v0.52.0, gix-prompt v0.13.1, gix-url v0.35.1, gix-credentials v0.36.0, gix-discover v0.47.0, gix-dir v0.21.0, gix-mailmap v0.31.0, gix-revision v0.41.0, gix-merge v0.12.0, gix-negotiate v0.27.0, gix-pack v0.66.0, gix-odb v0.76.0, gix-refspec v0.37.0, gix-shallow v0.8.1, gix-transport v0.54.0, gix-protocol v0.57.0, gix-status v0.26.0, gix-submodule v0.26.0, gix-worktree-state v0.26.0, gix v0.79.0, safety bump 35 crates ([`d66ac10`](https://github.com/GitoxideLabs/gitoxide/commit/d66ac1057a5b7bfb608d4e6be585c69fb692bfee))
    - Merge pull request #2415 from GitoxideLabs/improvements ([`5c9e6ae`](https://github.com/GitoxideLabs/gitoxide/commit/5c9e6ae8e7d0a2a0a67a05f2d222fb10f0216d60))
    - Rename `Creation::ExecuteScipt` to `Creation::Execute` ([`f4bb106`](https://github.com/GitoxideLabs/gitoxide/commit/f4bb1063ce0b09a6d076d9c45d7f505cf31a1c85))
    - Merge pull request #2414 from GitoxideLabs/improvements ([`a3e7b6e`](https://github.com/GitoxideLabs/gitoxide/commit/a3e7b6eb7330e2d83df492b301ac4975d17c1fd2))
    - Make sure rust-scripts are always executed in the final destination. ([`8c97151`](https://github.com/GitoxideLabs/gitoxide/commit/8c9715103950c80bcb96489d6c509dc3555b984e))
    - Merge pull request #2413 from GitoxideLabs/improvements ([`70a250c`](https://github.com/GitoxideLabs/gitoxide/commit/70a250c4ad08954db1364f9dcb7d7b743e4cba1d))
    - Change the signature of `post` Rust closures to allow returning values. ([`594fe4c`](https://github.com/GitoxideLabs/gitoxide/commit/594fe4c56c9b02641261329b4677f994f7da9469))
    - Add variants of `scripted_fixture_read_only` that allow for Rust post modification. ([`61c504d`](https://github.com/GitoxideLabs/gitoxide/commit/61c504dfdd6505eeeda22a6c4254c7342b78d6e3))
    - Better docs for Rust fixture method in test tools ([`fd0328f`](https://github.com/GitoxideLabs/gitoxide/commit/fd0328ff752104bcb4bd3137bd3392cf082919eb))
    - Merge pull request #2412 from GitoxideLabs/copilot/add-execute-rust-closures-again ([`675ce68`](https://github.com/GitoxideLabs/gitoxide/commit/675ce68914406b06d69415e938835015aa04a616))
    - Address Copilot review ([`b63bf6f`](https://github.com/GitoxideLabs/gitoxide/commit/b63bf6f59bc0f80b334feb2d21a4e0fc4891a923))
    - Deduplicate testtools marker implementation ([`fff844b`](https://github.com/GitoxideLabs/gitoxide/commit/fff844bb4d3dfcd20f9c033815e9de7e35a8270a))
    - Refactor ([`ab33ceb`](https://github.com/GitoxideLabs/gitoxide/commit/ab33ceb5d897b699abc29c6cc33b314327b915e5))
    - Add rust_fixture_read_only and rust_fixture_writable functions to gix-testtools ([`501da55`](https://github.com/GitoxideLabs/gitoxide/commit/501da557fcccabf79961e441d0bb9a3e8841b906))
    - Merge pull request #2407 from GitoxideLabs/dependabot/cargo/cargo-fb4135702f ([`8bceefb`](https://github.com/GitoxideLabs/gitoxide/commit/8bceefbfc5f897517bfdd24744695a82cfa0d5be))
    - Bump the cargo group with 59 updates ([`7ce3c55`](https://github.com/GitoxideLabs/gitoxide/commit/7ce3c5587aec1ca813039c047783b9cb2a106826))
    - Merge pull request #2398 from GitoxideLabs/release ([`3b6650a`](https://github.com/GitoxideLabs/gitoxide/commit/3b6650a66e957d124964c7f41cc9895d4292598b))
</details>

## 0.18.0 (2026-01-23)

### Bug Fixes (BREAKING)

 - <csr-id-d6b9d97dac91db3714069928c89b16f515b3f226/> let `writable` fixture functions take `AsRef<Path>` just like the read functions.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 1 calendar day.
 - 13 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-testtools v0.18.0 ([`73d45f6`](https://github.com/GitoxideLabs/gitoxide/commit/73d45f6c71eec3d9468cd1fdf77f05ad21373a15))
    - Merge pull request #2395 from GitoxideLabs/improvements ([`766ab10`](https://github.com/GitoxideLabs/gitoxide/commit/766ab10104c5f3a2edfe5550791a8d5e78479b3b))
    - Let `writable` fixture functions take `AsRef<Path>` just like the read functions. ([`d6b9d97`](https://github.com/GitoxideLabs/gitoxide/commit/d6b9d97dac91db3714069928c89b16f515b3f226))
    - Merge pull request #2393 from GitoxideLabs/report ([`f7d0975`](https://github.com/GitoxideLabs/gitoxide/commit/f7d09758d245aaa89409e39bb6ba1ed6b7118ea5))
    - Release gix-error v0.0.0, gix-date v0.13.0, gix-actor v0.38.0, gix-validate v0.11.0, gix-path v0.11.0, gix-features v0.46.0, gix-hash v0.22.0, gix-hashtable v0.12.0, gix-object v0.55.0, gix-glob v0.24.0, gix-attributes v0.30.0, gix-command v0.7.0, gix-packetline v0.21.0, gix-filter v0.25.0, gix-fs v0.19.0, gix-chunk v0.5.0, gix-commitgraph v0.32.0, gix-revwalk v0.26.0, gix-traverse v0.52.0, gix-worktree-stream v0.27.0, gix-archive v0.27.0, gix-tempfile v21.0.0, gix-lock v21.0.0, gix-index v0.46.0, gix-config-value v0.17.0, gix-pathspec v0.15.0, gix-ignore v0.19.0, gix-worktree v0.47.0, gix-diff v0.58.0, gix-blame v0.8.0, gix-ref v0.58.0, gix-sec v0.13.0, gix-config v0.51.0, gix-prompt v0.13.0, gix-url v0.35.0, gix-credentials v0.35.0, gix-discover v0.46.0, gix-dir v0.20.0, gix-mailmap v0.30.0, gix-revision v0.40.0, gix-merge v0.11.0, gix-negotiate v0.26.0, gix-pack v0.65.0, gix-odb v0.75.0, gix-refspec v0.36.0, gix-shallow v0.8.0, gix-transport v0.53.0, gix-protocol v0.56.0, gix-status v0.25.0, gix-submodule v0.25.0, gix-worktree-state v0.25.0, gix v0.78.0, gix-fsck v0.17.0, gitoxide-core v0.53.0, gitoxide v0.50.0, safety bump 50 crates ([`562e684`](https://github.com/GitoxideLabs/gitoxide/commit/562e684319fa649db6a96c0a22d64bbe3c11e9e6))
</details>

## 0.17.0 (2026-01-10)

### Documentation

 - <csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/> Remove `doc_auto_cfg` feature to fix docs.rs documentation.
   It is part of `doc_cfg` feature since https://github.com/rust-lang/rust/pull/138907
   
   This fixes the docs.rs build

### Bug Fixes

 - <csr-id-b24783accba4bdd39c0821564060a3b4f3745903/> Use `nul` instead of `NUL` on Windows
   `NULL_DEVICE` is `/dev/null` except on Windows, where there are
   several possible choices. Previously we were using `NUL` because
   the more modern full path `\\.\NUL` is not supported by `git`.
   
   However, `git` also rejects `NUL`, when capitalized, on some
   Windows systems. This can be observed on Windows 11 ARM64 builds.
   In contrast, the lower-case `nul`, which Windows itself treats the
   same as `NUL`, is always accepted by Git for Windows.
   
   Although it's not readily apparent why Git for Windows accepts
   `NUL` on some Windows operating systems and/or platforms and not
   others, the preferential treatment of `nul` (not extending to
   `NUL`) can be seen in a few places in the Git for Windows source
   code, including in `mingw_access` (`strcmp` is case-sensitive):

### New Features (BREAKING)

 - <csr-id-828e9035a40796f79650cf5e3becb8d8e5e29883/> Pattern parser in is now stateful to allow options for how to parse ignore patterns.
   That way it can support settings and other state that affect parsing.
   This affects various crates which are all marked as breaking now.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 38 commits contributed to the release.
 - 258 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2341 from GitoxideLabs/dependabot/cargo/cargo-cf4a2135ae ([`d914d95`](https://github.com/GitoxideLabs/gitoxide/commit/d914d9533ed2243658d51ba05e68dd444b75a748))
    - Bump the cargo group across 1 directory with 51 updates ([`4edc5dd`](https://github.com/GitoxideLabs/gitoxide/commit/4edc5dda7ca39cc8249cb98dc39ca46c7d00eb44))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
    - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2275 from GitoxideLabs/dependabot/cargo/cargo-92eaa62a2e ([`93dd630`](https://github.com/GitoxideLabs/gitoxide/commit/93dd630ca6a2a4622ca74d7eaff42ece2750b6c5))
    - Bump the cargo group across 1 directory with 14 updates ([`703644c`](https://github.com/GitoxideLabs/gitoxide/commit/703644c8821aae161592d19495d3b3162133324f))
    - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
    - Release gix-date v0.10.7, gix-actor v0.35.6, gix-trace v0.1.15, gix-features v0.44.1, gix-hash v0.20.1, gix-object v0.51.1, gix-glob v0.22.1, gix-attributes v0.28.1, gix-packetline-blocking v0.19.3, gix-commitgraph v0.30.1, gix-archive v0.23.1, gix-tempfile v19.0.1, gix-index v0.42.1, gix-config-value v0.15.3, gix-ignore v0.17.1, gix-worktree v0.43.1, gix-diff v0.54.1, gix-ref v0.54.1, gix-sec v0.12.2, gix-config v0.47.1, gix-url v0.33.1, gix-credentials v0.31.1, gix-mailmap v0.27.4, gix-revision v0.36.1, gix-pack v0.61.1, gix-odb v0.71.1, gix-packetline v0.19.3, gix-transport v0.49.1, gix-protocol v0.52.1, gix-status v0.21.1, gix v0.74.1 ([`bdcce5f`](https://github.com/GitoxideLabs/gitoxide/commit/bdcce5f2c6723ebe489dbe936a4656859ce1c2a5))
    - Remove `doc_auto_cfg` feature to fix docs.rs documentation. ([`6f469a6`](https://github.com/GitoxideLabs/gitoxide/commit/6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b))
    - Merge pull request #2224 from GitoxideLabs/report ([`3313233`](https://github.com/GitoxideLabs/gitoxide/commit/3313233aa4e7009aed0ddf644f4271fd2a98e8d4))
    - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
    - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
    - Merge pull request #2115 from EliahKagan/run-ci/arm-windows ([`c2c8c2f`](https://github.com/GitoxideLabs/gitoxide/commit/c2c8c2fb13749b2f10d882cdbadafcb2e4c4dba3))
    - Use `nul` instead of `NUL` on Windows ([`b24783a`](https://github.com/GitoxideLabs/gitoxide/commit/b24783accba4bdd39c0821564060a3b4f3745903))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
    - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2019 from GitoxideLabs/precious-opt-in ([`5f9de52`](https://github.com/GitoxideLabs/gitoxide/commit/5f9de52cf286163b503047b1ab3b51dfa093b4d4))
    - Pattern parser in is now stateful to allow options for how to parse ignore patterns. ([`828e903`](https://github.com/GitoxideLabs/gitoxide/commit/828e9035a40796f79650cf5e3becb8d8e5e29883))
    - Merge pull request #1992 from EliahKagan/run-ci/ouroboros ([`6acdc04`](https://github.com/GitoxideLabs/gitoxide/commit/6acdc040d880ebdef90f1cc4f1471b648de2afbe))
    - Let `gix-testtools` use `gix-*` workspace crates ([`7e057f2`](https://github.com/GitoxideLabs/gitoxide/commit/7e057f2c73a70a2cd56bbc5484e6141feef2f014))
    - Merge pull request #1975 from GitoxideLabs/improvements ([`28935a5`](https://github.com/GitoxideLabs/gitoxide/commit/28935a56ff91f1fc2c17a7d23b057cf7119144e9))
    - Thanks clippy ([`dbf65c9`](https://github.com/GitoxideLabs/gitoxide/commit/dbf65c95644e6a134e7f9b75e7871479720b4deb))
    - Merge pull request #1972 from GitoxideLabs/gix-testtools ([`d2f2333`](https://github.com/GitoxideLabs/gitoxide/commit/d2f2333271c4c7eac538203f43c41cb45df337df))
</details>

## 0.16.1 (2025-04-27)

### Bug Fixes

 - <csr-id-9b12d5007ca3ec98d061b6d2b94c7cdda4fcd3e4/> unify the dependency graph by choosing the right versions, upgrading to `gix-features 0.42`
   This is what should silence audit failures.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-testtools v0.16.1 ([`48079a5`](https://github.com/GitoxideLabs/gitoxide/commit/48079a5f64703881d0939b5c5f3066596794ba8d))
    - Unify the dependency graph by choosing the right versions, upgrading to `gix-features 0.42` ([`9b12d50`](https://github.com/GitoxideLabs/gitoxide/commit/9b12d5007ca3ec98d061b6d2b94c7cdda4fcd3e4))
</details>

## 0.16.0 (2025-04-27)

<csr-id-3b173054c76f5113f36beca3ba5a3a44642e1915/>

### Changed

 - <csr-id-f7f24e537246a47498d41f06816c14f17ea4ee48/> In gix-testtools use `ignore` and `index` via `gix_worktree`
   This removes the `gix-ignore` and `gix-index` direct dependencies
   of `gix-testtools`, by having `gix_testtools` use them through
   `gix-worktree`, accessing `gix_worktree::ignore` for `gix_ignore`
   and `gix_worktree::index` for `gix_index`.
   
   The rationale is that various other gix-* dependencies were used
   this way already, and also that this specific change appears to
   have been planned, based on the TODO comment in ddaacda (#1413).

### New Features

 - <csr-id-07022eee3d2e4a9257a07976a9871ae756493c96/> Show unexpected stderr in `umask` panic message
   Because `gix_testtools::umask()` is only suitable for use in tests,
   where signaling an error with a panic is typically acceptable, it
   panics rather than returning an `Error` to indicate errors. To
   avoid leading to the use of a potentially inaccurate umask value,
   it treats as an error any departure from the typical output of
   the `umask` command: in addition to treating a nonzero exit status
   as an error, it also treats anything it cannot strictly parse on
   stdout as an error, as well as anything at all written to stderr as
   an error. The latter is to avoid a situation where a warning is
   printed that is could be significant to some umask use cases.
   
   Warnings from `umask` are rare, as well as from the shell that is
   used as an intermediary for running the command (since no external
   `umask` command need exist and, often, does not) when it is used
   just to run `umask`. When they do occur, they are sometimes from
   the dynamic linker, such as a warning about a shared library listed
   in the `LD_PRELOAD` environment variable that cannot be used by
   the shell program. To understand and distinguish such errors, it is
   useful to show the text that was sent to stderr, since tests are
   sometimes run in environments that are nontrivial to reproduce
   otherwise. For example, running tests with `cross` produces an
   environment that is not in all respects the same as what one gets
   with `docker exec -it <container>`, even if `<container>` is the
   same still-running container being used to run the tests.
   
   This modifies `gix_testtools::umask()` so that when it panics due
   anything being written to stderr, it shows what was written.
 - <csr-id-720a23f873508fc574c0f0e9b212dbd0dd76b8bb/> Add `jtt bash-program` (`jtt bp`) to show `bash_program()`
   This adds a `bash-program` subcommand, abbreviated `bp`, to the
   `gix-testools` program (`jtt`) to check what the `bash_program()`
   library function gives.
   
   This is intended for diagnostic use and should probably not be used
   in scripting. Currently it shows the quoted debug repreesentation
   of the path.
 - <csr-id-47234b66c5de20b6ea701c3f215d832c86d770c1/> Document `gix_testtools::bash_program()` and make it public
   To make it easier for users of `gix-testtools` to diagnose problems
   or verify that the fallback for running fixutre scripts without
   usable shebangs (which is effectively how any fixture shell script
   is run on Windows), the formerly private `bash_program()` is now
   public.
   
   However, it is not recommend to rely on this specific value or on
   observed behavior of how it is computed. The details may change at
   any time.
   
   The purpose of `bash_program()` and how it is used internally by
   `gix-testtools` is also now documented explicitly. Broad details of
   how it searches or guesses what path to use are likewise documented,
   with a caveat that changes to them are not considered breaking.
 - <csr-id-0b45bafe2026a2404183f50460d36ec4d40e8e14/> Look for bash in `(git root)/bin`, then `(git root)/usr/bin`
   This changes `bash_program()` so that it will find the `bash.exe`
   provided by Git for Windows that is most reliable for our use in
   runinng test fixture scripts, of those that are available. First
   it uses the shim, but falls back to the non-shim if the shim is
   not available. If neither is found, then the fallback of using the
   simple command `bash.exe` (which triggers a path search when run)
   continues to be used.
 - <csr-id-ba76201f619d32a3606609d14e3bb0f1dfb100b7/> Prefer `/` over `\` in `bash.exe` path (for fixtures)
   Starting in #1712, `gix-testtools` looks for `bash.exe` on Windows
   in one of its common locations as provided by Git for Windows, by
   taking the path given by `git --exec-path`, going up by three
   components, and going down to `bin/bash.exe` under that. But the
   `bin` and `bash.exe` components were appended in a way that used
   `\` directory separators.
   
   Ordinarily, that would be ideal, since `\` is the primary directory
   separator on Windows. However, in this case, appending with `\`
   produces a path that is harder to read (especially in diagostic
   messages), and that may cause problems if it is processed by a
   shell or in a way that is intended to operate similarly to shell
   processing of `\`.
   
   A path that does not explicitly prefer `/` but instead uses
   `PathBuf::push` will have `\` in front of the new components, but
   will still usually have `/` in front of the old components. This is
   because, aside from the unusual case that the `GIT_EXEC_PATH`
   environment vairable is set explicitly and its value uses all `\`
   separators, the output of `git --exec-path`, which we use to find
   where `git` installed on Windows, uses `/` separators.
   
   The effect of this change seems to be fairly minor, with existing
   test fixtures having worked before it was done. This is partly
   because, on Windows, the value of `argv[0]` that the shell
   actually sees -- and that populates `$0` when no script name is
   available, as happens in `bash -c '...'` with no subsequent
   arguments -- is translated by an MSYS DLL such as `msys-2.0.dll`
   (or, for Cygwin, `cygwin1.dll`) into a Unix-style path meaningful
   to the shell.
   
   This also refactors for clarity, and adds new tests related to the
   change.
 - <csr-id-8e7fb99111f2dc9710ae4533f47341138bd58ade/> Add `gix_testtools::umask`, safe but only meant for tests
   This implements a function for tests to safely read the current
   process umask without the usual race condition of doing so, at the
   expense of using subprocesses to do it. This just calls a shell and
   runs `umask` from it (which is expected to be a builtin and, on
   many systems, is available as a builtin but not an executable).
   
   Even though this is safe, including thread-safe, it is unlikely to
   be suitable for use outside of tests, because of its use of
   `expect` and assertions when there are errors, combined with the
   possibly slow speed of using subprocesses.
   
   Given that this is effecitvely running a tiny shell script to do
   the work, why is it not instead a fixture script that is named in
   a `.gitignore` file so that it is not tracked? The reason is that
   the outcomes of running such fixture scripts are still saved across
   separate test runs, but it is useful to be able to run the tests
   with differnt umasks, e.g. `(umask 077; cargo nextest run ...)`.
   
   The immediate purpose is in forthcoming tests that, when checkout
   sets +x on an existing file, it doesn't set excessive permissions.
   The fix to pass such a test is not currently planned to use the
   umask explicitly. But the tests will use it, at least to detect
   when they cannot really verify the code under test on the grounds
   that they are running with an excessively permissive umask that
   doesn't allow behavior that only occurs with a generally reasonable
   umask to be observed.
 - <csr-id-77c3c59d1f3be76f228ada15304d5af1f3f03a14/> Add `size_ok` for asserting size is not too big
   This compares using `==` on 64-bit targets and `<=` on 32-bit
   targets.
   
   As noted in the documentation comment, when assertions about data
   stuructures' sizes are being done to safeguard against them growing
   too big, then it may be acceptable to use `<=` if the structure is
   smaller on 32-bit targets, but it is still valuable to be able to
   use `==` on 64-bit targets in the same assertions, since this
   guards against a data structure becoming smaller, other changes
   causing the smaller size to be important for memory usage or speed,
   but then the data structure growing again, up to its original size.
   An unconditional `<=` will not catch this, while `size_ok` usually
   will.
   
   A related reason to do a `==` on 64-bit systems is so that the
   expected value being compared to remains tied to the code. It can
   otherwise become unclear what the expected value's significance is
   and whether it ought to be updated.
 - <csr-id-03d5a6873e1b6d44227f709e91ac447e29abfac3/> Recognize `GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI`
   When `gix-testtools` runs fixtures, it now recognizes a new
   environment variable, `GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI`,
   specifying that archives should be generated even when on CI.
   
   By default, they are still not generated when on CI. It may make
   sense to enable this:
   
   - If automatically testing archive creation, or

### Bug Fixes

<csr-id-39323c34ec232ea686f8cfb227f87e23336467cb/>
<csr-id-9d4dd121498907e820f82051d840deefa719ab26/>
<csr-id-8dc5d7aa736059aa45a17dfdc76d9d4c9993f996/>
<csr-id-a879d2214ae40be7692fa00360c8151bb8e2e88e/>
<csr-id-3cf9fc12cb8ebb9bf04e4f5bd2aee884c18d672f/>
<csr-id-581957ea3d810da7529b818604067d16fc025631/>
<csr-id-8a0fedb22bad576ea11017777f476947f366e5f5/>

 - <csr-id-93cb5ba03d364efcbb4110c2bd207f3d8de9b292/> fix check to detect `git-lfs` managed files that weren't checked out.
   Previously it would detect them incorrectly due to a find-and-replace
   error.
 - <csr-id-8b694a68cf60f5dd296733761d10fff612b4fc5e/> create a more local lock when creating writable fixtures.
   Previously, the lock location would block all writers from executing
   a fixture even though they wouldn't step on each others feet.
   
   Now, a script destination is used to assure locks are created close
   to the destination when creating writable fixtures, typically removing
   the need for multiple writers to wait on each other unnecessarily.
 - <csr-id-fe3f2d128a1478af97999025b46c7b146e778524/> Run test fixture scripts on Windows with Git Bash
   Rather than hard-coding `bash` on all systems as the fallback
   interpreter when a fixture script cannot be run directly, this
   falls back in an operating system specific manner:
   
   - Except on Windows, always fall back to `bash`, as before.
1. This only modifies how test fixture scripts are run. It only
      affects the behavior of `gix-testtools`, and not of any other
      gitoxide crates such as `gix-command`. This is because:
   
      - While gitoxide uses information from `git` to find out where
        it is installed, mainly so we know where to find installation
        level configuration, we cannot in assume that `git` is present
        at all. Unlike GitPython, gitoxide is usable without `git`.
   
      - We know our test fixture scripts are all (at least currently)
        `bash` scripts, and this seems likely for other software that
        currently uses this functionality of `gix-testtools`. But
        scripts that are run as hooks, or as custom commands, or
        filters, etc., are often written in other languages, such as
        Perl. (The fallback here does not examine leading `#!` lines.)
   
      - Although a `bash.exe` located at the usual place relative to
        (but outside of) the `git-core` directory is usually suitable,
        there may be scenarios where running an executable found this
        way is not safe. Limiting it to `gix-testtools` pending
        further research may help mitigate this risk.
   - We know our test fixture scripts are all (at least currently)
        `bash` scripts, and this seems likely for other software that
        currently uses this functionality of `gix-testtools`. But
        scripts that are run as hooks, or as custom commands, or
        filters, etc., are often written in other languages, such as
        Perl. (The fallback here does not examine leading `#!` lines.)
   - Although a `bash.exe` located at the usual place relative to
        (but outside of) the `git-core` directory is usually suitable,
        there may be scenarios where running an executable found this
        way is not safe. Limiting it to `gix-testtools` pending
        further research may help mitigate this risk.
- We know our test fixture scripts are all (at least currently)
        `bash` scripts, and this seems likely for other software that
        currently uses this functionality of `gix-testtools`. But
        scripts that are run as hooks, or as custom commands, or
        filters, etc., are often written in other languages, such as
        Perl. (The fallback here does not examine leading `#!` lines.)
- Although a `bash.exe` located at the usual place relative to
        (but outside of) the `git-core` directory is usually suitable,
        there may be scenarios where running an executable found this
        way is not safe. Limiting it to `gix-testtools` pending
        further research may help mitigate this risk.
- It would add `gix-path` as a dependency of `gix-testtools`.
- Finding `git` in a `std::process::Command` path search is an
        established (though not promised) approach in `gix-testtools`,
        including to run `git --exec-path` (to find `git-daemon`).
- It is not immediately obvious that `exe_invocation` behavior
        is semantically correct for `gix-testtools`, though it most
        likely is reasonable.
   
        The main issue is that, in many cases where `git` itself runs
        scripts, it prepends the path to the `git-core` directory to
        the `PATH` environment variable for the script. This directory
        has a `git` (or `git.exe`) executable in it, so scripts run
        an equivalent `git` associated with the same installation.
   
        In contrast, when we run test fixture scripts with a
        `bash.exe` associated with a Git for Windows installation, we
        do not customize its path. Since top-level scripts written to
        use `git` but not to be used *by* `git` are usually written
        without the expectation of such an environment, prepending
        this will not necessarily be an improvement.
- Some variables may be set on the test machine without envisioning
     this usage, but should still be kept, such as those that cause
     more or less traversal than usual to be done. For example, if
     `GIT_CEILING_DIRECTORIES` or even `GIT_DISCOVERY_ACROSS_FILESYSTEM`
     are set, it may be for a good reason.
- Some variables will have no default unless other variables that
     are being modified here are changed again after the changes here.
     In particular, `GIT_CONFIG_SYSTEM` only has an effect if
     `GIT_CONFIG_NOSYSTEM` is not set. We set `GIT_CONFIG_NOSYSTEM` to
     `1`, so if it is unset then a fixture script has unset it, in
     which case it is presumably intended that `GIT_CONFIG_SYSTEM`
     have some effect (if the fixture script doesn't change/unset it).
- Some variables are useful for extra debugging and make sense to
     set when running the test fixtures under foreseeable conditions.
     For example, the effects of all `GIT_TRACE*` variables are
     intentionally preserved.
- For a few variables, such as `GIT_DEFAULT_HASH`, it is unlikely
     that they would be wanted in the test environment, but even more
     unlikely that they would be set in that environment without the
     intention of experimenting with their effect on fixtures.
- That is a single `git` invocation for a specific command, so the
     environment variables that ought to affect it must be kept, and
     others can be removed. But here, arbitrary fixtures need to work,
     and they provide almost all of their own environment as needed.
- Setting an unusual value of `GIT_DIR` there that `git` cannot
     take to be a usable repository also prevents the variables
     that override `GIT_DIR` for specific files from being used. (But
     maybe those should be unset there anyway, for clarity?)
- https://git-scm.com/docs/git#Documentation/git.txt-codeGITCONFIGGLOBALcode

### New Features (BREAKING)

 - <csr-id-0899c2ee36a714573b223ae85114fb7284fc661e/> on Windows, also instruct msys to create real symlinks
   This will only reliably work on with developer setups, but that
   seems fair to assume.
   If this causes problems, it's fine to make it opt-in as well.

### Bug Fixes (BREAKING)

 - <csr-id-692caeba599110d61da66cbbff545f2bc16748d5/> don't panic, instead provide an error when fixture script fails.
   This makes introspection easier, even though we still have to print to
   script output to stderr in order to make it legible.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 105 commits contributed to the release over the course of 296 calendar days.
 - 308 days passed between releases.
 - 22 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1440](https://github.com/GitoxideLabs/gitoxide/issues/1440), [#1443](https://github.com/GitoxideLabs/gitoxide/issues/1443)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 6 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1440](https://github.com/GitoxideLabs/gitoxide/issues/1440)**
    - Assure archives are unique if their generator-scripts are called with arguments. ([`8a0fedb`](https://github.com/GitoxideLabs/gitoxide/commit/8a0fedb22bad576ea11017777f476947f366e5f5))
 * **[#1443](https://github.com/GitoxideLabs/gitoxide/issues/1443)**
    - On Windows, also instruct msys to create real symlinks ([`0899c2e`](https://github.com/GitoxideLabs/gitoxide/commit/0899c2ee36a714573b223ae85114fb7284fc661e))
 * **Uncategorized**
    - Release gix-testtools v0.16.0 ([`c5c726e`](https://github.com/GitoxideLabs/gitoxide/commit/c5c726e64f01b3a927ec5dc0e9917a4470d9cc5c))
    - Prepare new testtools release ([`070f5f6`](https://github.com/GitoxideLabs/gitoxide/commit/070f5f68976217c4fec84cae8516c3f5b716e513))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Thanks clippy ([`6f009d7`](https://github.com/GitoxideLabs/gitoxide/commit/6f009d781da9e931d44b113a925a80e77e8788af))
    - Merge pull request #1968 from GitoxideLabs/dependabot/cargo/cargo-bd18780e40 ([`46227e6`](https://github.com/GitoxideLabs/gitoxide/commit/46227e6d1ddc0879662730e5bb21a8597716b1ca))
    - Bump the cargo group with 40 updates ([`06bf1e1`](https://github.com/GitoxideLabs/gitoxide/commit/06bf1e1552de65ce692911bdc4c501d487bbc3d7))
    - Merge pull request #1825 from DianaNites/diananites-reftable ([`edb449c`](https://github.com/GitoxideLabs/gitoxide/commit/edb449c9dd60f74562dc78a33e41cfcb5d7be81e))
    - Don't panic, instead provide an error when fixture script fails. ([`692caeb`](https://github.com/GitoxideLabs/gitoxide/commit/692caeba599110d61da66cbbff545f2bc16748d5))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1895 from EliahKagan/run-ci/s390x ([`705b86d`](https://github.com/GitoxideLabs/gitoxide/commit/705b86d59d6f18e76dcc5278f54b0e4838437d9d))
    - Move `env` subcommand to `internal-tools` ([`809fb2f`](https://github.com/GitoxideLabs/gitoxide/commit/809fb2f3fa2ceaef7fbcad5222ec6b892a086a9b))
    - Test `gix_testtools::umask()` on Android targets ([`c19bf1d`](https://github.com/GitoxideLabs/gitoxide/commit/c19bf1de76a3d6afc7740bdbc35181276bfcb58d))
    - Add `env` testtools subcommand to show the environment ([`8d596b4`](https://github.com/GitoxideLabs/gitoxide/commit/8d596b4179f37fbecf09291f29c15103851b63f2))
    - Show unexpected stderr in `umask` panic message ([`07022ee`](https://github.com/GitoxideLabs/gitoxide/commit/07022eee3d2e4a9257a07976a9871ae756493c96))
    - Merge pull request #1864 from EliahKagan/run-ci/bash-program ([`97d50a3`](https://github.com/GitoxideLabs/gitoxide/commit/97d50a3c2218a777919a7b06ac19e87100382b71))
    - Add `jtt bash-program` (`jtt bp`) to show `bash_program()` ([`720a23f`](https://github.com/GitoxideLabs/gitoxide/commit/720a23f873508fc574c0f0e9b212dbd0dd76b8bb))
    - Document `gix_testtools::bash_program()` and make it public ([`47234b6`](https://github.com/GitoxideLabs/gitoxide/commit/47234b66c5de20b6ea701c3f215d832c86d770c1))
    - Look for bash in `(git root)/bin`, then `(git root)/usr/bin` ([`0b45baf`](https://github.com/GitoxideLabs/gitoxide/commit/0b45bafe2026a2404183f50460d36ec4d40e8e14))
    - Look for bash in `(git root)/usr/bin`, not `(git root)/bin` ([`73d30d7`](https://github.com/GitoxideLabs/gitoxide/commit/73d30d7a1e61388a415b27552aa2596abb0238b5))
    - Prefer `/` over `\` in `bash.exe` path (for fixtures) ([`ba76201`](https://github.com/GitoxideLabs/gitoxide/commit/ba76201f619d32a3606609d14e3bb0f1dfb100b7))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1822 from epage/w7 ([`11ac79c`](https://github.com/GitoxideLabs/gitoxide/commit/11ac79c068181d4ed9f6a404e4875ad7c206520c))
    - Upgrade to Winnow 0.7 ([`fdc57e7`](https://github.com/GitoxideLabs/gitoxide/commit/fdc57e79af6f7922d91ad8d7796943821f637124))
    - Upgrade to Winnow 0.6.26 ([`783c4e6`](https://github.com/GitoxideLabs/gitoxide/commit/783c4e698234b8afaf8fbd25057aca11c5c66e75))
    - Merge pull request #1815 from EliahKagan/quadratic ([`ffb73b5`](https://github.com/GitoxideLabs/gitoxide/commit/ffb73b5f69dbe86ff88f1c473af65f368a6bcbe5))
    - Comment forthcoming dependency changes for gix-testtools ([`6990f76`](https://github.com/GitoxideLabs/gitoxide/commit/6990f76a88b0a629a1bd578979218f5e791a862e))
    - Merge pull request #1764 from EliahKagan/finalize-entry ([`12f672f`](https://github.com/GitoxideLabs/gitoxide/commit/12f672f20f622a8488356a12df2d773851a683d4))
    - Refactor ([`4d5e656`](https://github.com/GitoxideLabs/gitoxide/commit/4d5e656fc0103e11ac2ed64305dd2430c6ed4648))
    - Fix an MSRV incompatibility ([`61174e5`](https://github.com/GitoxideLabs/gitoxide/commit/61174e585304ff34536c7ec5f325b734e3822161))
    - Add `gix_testtools::umask`, safe but only meant for tests ([`8e7fb99`](https://github.com/GitoxideLabs/gitoxide/commit/8e7fb99111f2dc9710ae4533f47341138bd58ade))
    - Merge pull request #1752 from GitoxideLabs/git-shell ([`1ca480a`](https://github.com/GitoxideLabs/gitoxide/commit/1ca480aa4093328a7e047e770fdffdb8cc6d8e8d))
    - Make a note to use `env::git_login_shell()` in testtools when available. ([`3aff1e5`](https://github.com/GitoxideLabs/gitoxide/commit/3aff1e57ba25b39774db4b1dd051bfe9c110911e))
    - Thanks clippy ([`9193b05`](https://github.com/GitoxideLabs/gitoxide/commit/9193b05b2528f62d829447ccc50314bd4cffc415))
    - Merge pull request #1733 from GitoxideLabs/fix-testools ([`df5cead`](https://github.com/GitoxideLabs/gitoxide/commit/df5cead220c193a9ceb8b78c8d6225368293416d))
    - Fix check to detect `git-lfs` managed files that weren't checked out. ([`93cb5ba`](https://github.com/GitoxideLabs/gitoxide/commit/93cb5ba03d364efcbb4110c2bd207f3d8de9b292))
    - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
    - Create a more local lock when creating writable fixtures. ([`8b694a6`](https://github.com/GitoxideLabs/gitoxide/commit/8b694a68cf60f5dd296733761d10fff612b4fc5e))
    - Merge pull request #1712 from EliahKagan/run-ci/git-bash ([`fadf106`](https://github.com/GitoxideLabs/gitoxide/commit/fadf106c735837c627f072ee37a9f7587f987bf2))
    - Fix an ambiguous `expect` message ([`1f6a866`](https://github.com/GitoxideLabs/gitoxide/commit/1f6a8669a64b15fbe7021c6906f88f5b7c7c142e))
    - Run test fixture scripts on Windows with Git Bash ([`fe3f2d1`](https://github.com/GitoxideLabs/gitoxide/commit/fe3f2d128a1478af97999025b46c7b146e778524))
    - Refine `EXEC_PATH` validation in `spawn_git_daemon` ([`479c06b`](https://github.com/GitoxideLabs/gitoxide/commit/479c06b372224a33d29e8b12fd59d96ab29fc60f))
    - Run `cargo fmt` ([`da03932`](https://github.com/GitoxideLabs/gitoxide/commit/da0393213dd0b08958da847e856b96028d038b46))
    - Use consistent `git` command name in gix-testtools ([`e30c070`](https://github.com/GitoxideLabs/gitoxide/commit/e30c0700c3e64b87c20b4695ccfb41f05f961129))
    - Rename `parse_gix_version` to `parse_git_version` ([`01737ad`](https://github.com/GitoxideLabs/gitoxide/commit/01737ad7b76f16d84c2d8a43d70e1c92f8514867))
    - Merge pull request #1687 from EliahKagan/run-ci/32bit ([`aeaebec`](https://github.com/GitoxideLabs/gitoxide/commit/aeaebec7b1e07ce94429987c4f2466799c24cb67))
    - Add `size_ok` for asserting size is not too big ([`77c3c59`](https://github.com/GitoxideLabs/gitoxide/commit/77c3c59d1f3be76f228ada15304d5af1f3f03a14))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Merge pull request #1606 from EliahKagan/fixture-timeout ([`8c9e827`](https://github.com/GitoxideLabs/gitoxide/commit/8c9e827bc7727a8b0bcf36d2195120a2fc509d51))
    - Double the fixture lock timeout ([`39323c3`](https://github.com/GitoxideLabs/gitoxide/commit/39323c34ec232ea686f8cfb227f87e23336467cb))
    - Merge pull request #1594 from EliahKagan/comments ([`ab8880f`](https://github.com/GitoxideLabs/gitoxide/commit/ab8880fbdf0f7af7b483f4e1f9adbb8e374183ee))
    - Copyedit `configure_command` comment ([`56dbcd5`](https://github.com/GitoxideLabs/gitoxide/commit/56dbcd5632d8a4adf7189196a4bb3942a9fe1f4d))
    - Merge pull request #1592 from EliahKagan/tools-cfgcmd ([`5e783de`](https://github.com/GitoxideLabs/gitoxide/commit/5e783de0620d7c15992fcaa28c97f0ec04020b18))
    - Note that how we set `MSYS` ignores `env`/`env_remove` calls ([`4a25ef5`](https://github.com/GitoxideLabs/gitoxide/commit/4a25ef5030a868164eaf12ffe9603fbcc1a89d01))
    - Use more compact notation ([`0e1e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/0e1e6a9bba4390906326c0a074f082e93b8345fe))
    - Broaden `args` param of `configure_command` ([`c202084`](https://github.com/GitoxideLabs/gitoxide/commit/c2020848f1be737c06669aeb977383559077b446))
    - Merge pull request #1590 from EliahKagan/run-ci/check-clean ([`4f92140`](https://github.com/GitoxideLabs/gitoxide/commit/4f92140febf4e9a13d7490b36c04fbf3fc63a5ad))
    - Merge pull request #1591 from EliahKagan/run-ci/deps-via-worktree ([`c485a2b`](https://github.com/GitoxideLabs/gitoxide/commit/c485a2bcdf3a07fd690fda4e02366bb52c2638ee))
    - Thanks clippy ([`94c6d70`](https://github.com/GitoxideLabs/gitoxide/commit/94c6d704ae216b12f7132c17876e0526097e86e6))
    - Recognize `GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI` ([`03d5a68`](https://github.com/GitoxideLabs/gitoxide/commit/03d5a6873e1b6d44227f709e91ac447e29abfac3))
    - Rename create_archive_if_{not_on_ci -> we_should} ([`8b51b3a`](https://github.com/GitoxideLabs/gitoxide/commit/8b51b3a0cb1edafc7b041f1037b3dfab4cdc3640))
    - Copyedit `create_archive_if_not_on_ci` comments ([`6963b37`](https://github.com/GitoxideLabs/gitoxide/commit/6963b376bbb0cff3e9331aaf924e96c1428d3892))
    - In gix-testtools use `ignore` and `index` via `gix_worktree` ([`f7f24e5`](https://github.com/GitoxideLabs/gitoxide/commit/f7f24e537246a47498d41f06816c14f17ea4ee48))
    - Merge pull request #1581 from EliahKagan/fixture-env ([`4044ffb`](https://github.com/GitoxideLabs/gitoxide/commit/4044ffb956a8a3842eaa6c5479be00a2bf2ae7e3))
    - Merge pull request #1580 from EliahKagan/msys ([`d00235a`](https://github.com/GitoxideLabs/gitoxide/commit/d00235a906e9155691f5fc3126b868dda515cd69))
    - Unset other env vars related to `GIT_DIR` for fixtures ([`9d4dd12`](https://github.com/GitoxideLabs/gitoxide/commit/9d4dd121498907e820f82051d840deefa719ab26))
    - Remove `configure_command_msys*` tests at least for now ([`c38d9b9`](https://github.com/GitoxideLabs/gitoxide/commit/c38d9b9f666cfd0858e46c0f600f4bc17259bb85))
    - Append to preexisting `MSYS` env var even if ill-formed ([`8dc5d7a`](https://github.com/GitoxideLabs/gitoxide/commit/8dc5d7aa736059aa45a17dfdc76d9d4c9993f996))
    - Start testing how the MSYS env var is customized ([`fbd4908`](https://github.com/GitoxideLabs/gitoxide/commit/fbd4908d8506254e3901bdce66e474ca08ff230b))
    - Merge pull request #1571 from EliahKagan/fixture-config ([`0e2f831`](https://github.com/GitoxideLabs/gitoxide/commit/0e2f831836ca13d7bc62d416c32e413b9823fe60))
    - Run `cargo fmt` ([`91e065c`](https://github.com/GitoxideLabs/gitoxide/commit/91e065cbbaad4454f9116d43e5a43a0d20bfd866))
    - Omit other high-scoped config in fixtures ([`a879d22`](https://github.com/GitoxideLabs/gitoxide/commit/a879d2214ae40be7692fa00360c8151bb8e2e88e))
    - Test that env for fixture scripts has only command-scope config ([`d576b32`](https://github.com/GitoxideLabs/gitoxide/commit/d576b321008d7e19180cfcd6a8d132352600bc91))
    - Merge pull request #1570 from EliahKagan/tools-nulldev ([`6f128dd`](https://github.com/GitoxideLabs/gitoxide/commit/6f128dd6adf9148e859a0cd027ff1c0ba0b619c0))
    - Minor refactors ([`50fcd7e`](https://github.com/GitoxideLabs/gitoxide/commit/50fcd7eb2dee0a8f57b1ffcf01868379571c1afb))
    - Don't assert that `tempfile::TempDir` cleans up ([`15bb2e3`](https://github.com/GitoxideLabs/gitoxide/commit/15bb2e36b17b57ae08d263b8e550e4655aad74c8))
    - Avoid `File::create_new` for compatibility with project MSRV ([`40ac226`](https://github.com/GitoxideLabs/gitoxide/commit/40ac226e6d44a05c8731e4eaf647bb5d6a9dda79))
    - Thanks clippy ([`2d7abaf`](https://github.com/GitoxideLabs/gitoxide/commit/2d7abaf8a816ecc3ec8a006223d4b636eab7a1b6))
    - Omit system/global config in fixtures regardless of contents ([`3cf9fc1`](https://github.com/GitoxideLabs/gitoxide/commit/3cf9fc12cb8ebb9bf04e4f5bd2aee884c18d672f))
    - Verify that we really write the strangely named test files ([`d7dca27`](https://github.com/GitoxideLabs/gitoxide/commit/d7dca27b81e4890cd0c597af275890df3d76b048))
    - Test on Windows with an actual file called `NUL` ([`f71d596`](https://github.com/GitoxideLabs/gitoxide/commit/f71d5966c204a2f3d1b48d59bad5b880caab717a))
    - Refactor the test for readability ([`7186eed`](https://github.com/GitoxideLabs/gitoxide/commit/7186eed39c40df82de2dd61530b6232c34b91ca9))
    - Fix assertion messages and expected exit status ([`a2dc5d8`](https://github.com/GitoxideLabs/gitoxide/commit/a2dc5d85a2ea61da4b4baddcd183b170763ed610))
    - Test that the system/global scopes are really cleared ([`d5b61df`](https://github.com/GitoxideLabs/gitoxide/commit/d5b61df18c5ab184e242f4e9d5d08730f7cd4fe7))
    - Start testing that we clear system/global scopes for fixtures ([`85c5e2f`](https://github.com/GitoxideLabs/gitoxide/commit/85c5e2fea1dbba8b8603bde7209bc8604042c55e))
    - Merge pull request #1560 from EliahKagan/run-ci/env-lifo ([`2972ea8`](https://github.com/GitoxideLabs/gitoxide/commit/2972ea8c3a03b8d6be2abdbd371c3f06dbdd67a4))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Move new tests up into a `tests` subdirectory ([`555164f`](https://github.com/GitoxideLabs/gitoxide/commit/555164f2387f77348ab876d05a77c142a69cacfa))
    - Let `gix_testtools::Env` undo multiple changes to the same var ([`581957e`](https://github.com/GitoxideLabs/gitoxide/commit/581957ea3d810da7529b818604067d16fc025631))
    - Add tests for `gix_testtools::Env` ([`505151c`](https://github.com/GitoxideLabs/gitoxide/commit/505151c9a9f61d9706a803d1cd7b25eaa1a99417))
    - Fix description of `gix_testtools::Env::unset` ([`3b17305`](https://github.com/GitoxideLabs/gitoxide/commit/3b173054c76f5113f36beca3ba5a3a44642e1915))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
    - Merge branch 'fix-windows-tests' ([`c2753b8`](https://github.com/GitoxideLabs/gitoxide/commit/c2753b8425c285c6b53f46eba9bc3584aa85eb01))
    - Fix gix-archive tests for when symlinks are allowed ([`93e088a`](https://github.com/GitoxideLabs/gitoxide/commit/93e088a619db0d4b81e444922f375de65c94a317))
    - Merge branch 'fix-1440' ([`f87322e`](https://github.com/GitoxideLabs/gitoxide/commit/f87322e185704d9d4368ae88e95892635a976e4a))
</details>

## 0.15.0 (2024-06-23)

Now by default, `tar` files will be written which works better when checking them into
Git. Those who need the previous behaviour, can use the `xz` feature instead.

### New Features (BREAKING)

 - <csr-id-55382c0aa6f04a3bb689299c613df2a39f261289/> make `xz2` optional to write uncompressed tar files by default.
   Previously, compression was beneficial due to storage in `git-lfs`.
   Now, storing (mostly) non-binary files is actually better moving forward.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-testtools v0.15.0 ([`78b8e41`](https://github.com/GitoxideLabs/gitoxide/commit/78b8e4110abd303511bad3fab469e7458f31f75a))
    - Update changelog prior to `gix-testtools` release ([`88eb841`](https://github.com/GitoxideLabs/gitoxide/commit/88eb841b7e3991b6f228390433f58333c6d4b85f))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Make `xz2` optional to write uncompressed tar files by default. ([`55382c0`](https://github.com/GitoxideLabs/gitoxide/commit/55382c0aa6f04a3bb689299c613df2a39f261289))
</details>

## 0.14.0 (2024-06-22)

A maintenance release with updated dependencies, and possibly minor improvements.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release.
 - 176 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-testtools v0.14.0 ([`f6eaba3`](https://github.com/GitoxideLabs/gitoxide/commit/f6eaba3a465720819f2ef7844648699195dd8341))
    - Prepare changelog prior to release of `gix-testtools` ([`ae012ba`](https://github.com/GitoxideLabs/gitoxide/commit/ae012ba8627a8ea2f0145c53c42972db1d875662))
    - Merge branch 'gix-testtools-license-file' ([`c50af65`](https://github.com/GitoxideLabs/gitoxide/commit/c50af65bf84963cb430538915f8eca9bd4481012))
    - Update `gix-testtools` dependencies to the latest version. ([`ddaacda`](https://github.com/GitoxideLabs/gitoxide/commit/ddaacda6667092ccea100180f70d2ccb9bb79dbc))
    - Add include directive to test-tools, to avoid CHANGELOG.md ([`c668bdd`](https://github.com/GitoxideLabs/gitoxide/commit/c668bdd0b29f513df75c96f09c84abac612b8e6a))
    - Fix missing license files in the released gix-testtools crate ([`ef73ed4`](https://github.com/GitoxideLabs/gitoxide/commit/ef73ed4644855ba53cc2b73674ee8df51ae7c053))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Release gix-fs v0.11.1, gix-glob v0.16.3 ([`2cefe77`](https://github.com/GitoxideLabs/gitoxide/commit/2cefe77203131878d0d8f5346f20f0e25b76cbea))
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Merge pull request #1365 from EliahKagan/no-fixture-lfs ([`c4e0a82`](https://github.com/GitoxideLabs/gitoxide/commit/c4e0a82ec9948ed0d9876a50b156d1ba30e2ad77))
    - Remove LFS CI step and modify docs/comments ([`3283445`](https://github.com/GitoxideLabs/gitoxide/commit/3283445424566579e2c2b31731f82d3f948a5c48))
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
    - Merge pull request #1290 from epage/winnow ([`a663e9f`](https://github.com/GitoxideLabs/gitoxide/commit/a663e9fcdb5a3aedc9200da77ebae17d5c3e7135))
    - Update winnow to 0.6 ([`e175b20`](https://github.com/GitoxideLabs/gitoxide/commit/e175b20d431faa6859fbcc52f78400e50f91cad1))
    - Use winnow BStr ([`47d0374`](https://github.com/GitoxideLabs/gitoxide/commit/47d0374e86cab4d498d955ac73bd7468cd5fcda9))
    - Update winnow to 0.5.40 ([`516e105`](https://github.com/GitoxideLabs/gitoxide/commit/516e105db5f22e1483b4b8a886cc4f3929ad7f6a))
    - Merge pull request #1267 from epage/winnow ([`69cb78b`](https://github.com/GitoxideLabs/gitoxide/commit/69cb78bd865a372c580b386766d7b61e5ca9303a))
    - Update from winnow 0.5.31 to 0.5.36 ([`9470554`](https://github.com/GitoxideLabs/gitoxide/commit/94705546cf0e4c8e38bcc96999cfa79cd8ee1acd))
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Release gix-date v0.8.3, gix-hash v0.14.1, gix-trace v0.1.6, gix-features v0.37.1, gix-actor v0.29.1, gix-validate v0.8.3, gix-object v0.40.1, gix-path v0.10.3, gix-glob v0.15.1, gix-quote v0.4.10, gix-attributes v0.21.1, gix-command v0.3.2, gix-packetline-blocking v0.17.2, gix-utils v0.1.8, gix-filter v0.8.1, gix-fs v0.9.1, gix-chunk v0.4.7, gix-commitgraph v0.23.1, gix-hashtable v0.5.1, gix-revwalk v0.11.1, gix-traverse v0.36.1, gix-worktree-stream v0.8.1, gix-archive v0.8.1, gix-config-value v0.14.3, gix-tempfile v12.0.1, gix-lock v12.0.1, gix-ref v0.40.1, gix-sec v0.10.3, gix-config v0.33.1, gix-prompt v0.8.2, gix-url v0.26.1, gix-credentials v0.23.1, gix-ignore v0.10.1, gix-bitmap v0.2.10, gix-index v0.28.1, gix-worktree v0.29.1, gix-diff v0.39.1, gix-discover v0.28.1, gix-macros v0.1.3, gix-mailmap v0.21.1, gix-negotiate v0.11.1, gix-pack v0.46.1, gix-odb v0.56.1, gix-pathspec v0.5.1, gix-packetline v0.17.2, gix-transport v0.40.1, gix-protocol v0.43.1, gix-revision v0.25.1, gix-refspec v0.21.1, gix-status v0.4.1, gix-submodule v0.7.1, gix-worktree-state v0.6.1, gix v0.57.1 ([`972241f`](https://github.com/GitoxideLabs/gitoxide/commit/972241f1904944e8b6e84c6aa1649a49be7a85c3))
    - Fixup `gix-testtools` manifest to allow releasing all other crates. ([`9587972`](https://github.com/GitoxideLabs/gitoxide/commit/95879729c411337cb5f6f5fd699b8a6d61e83a78))
</details>

## 0.13.0 (2023-12-29)

<csr-id-ef54aab9e5521add4154ee8d902d62612a9d8d4a/>
<csr-id-7f7db9794c23b87c8ea50b7bcf38955c9d977624/>
<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### New Features

 - <csr-id-06d4682c9fed696fee09234223814016a6453a6d/> write informative message if archives are ignored due to script change.

### Bug Fixes

 - <csr-id-11cb4317c75864bf310f4964edba7cf487a604f9/> better debug output when fixture script script fails
 - <csr-id-ed1407c85525a524bcfa0a4a021a22de339e6149/> don't let scripts run on (potentially) partially extracted archives and don't create archives on windows
   This could cause unrelated failures which didn't help debugging at all.
   Also improve error messages when trying to delete a stale/incorrect script
   output directory fails.
   This can easily happen on windows when symlinks are involved.
   
   We also stop creating archives on windows as the archive-metadata can't be deleted for some
   reason, which means it stays in the created directory and may cause script failures.
   This seems acceptable as windows users simply won't be able to create their own archives for accelration,
   but could probably use WSL for it.

### Bug Fixes (BREAKING)

 - <csr-id-2189cee47f99350b368390eaa2a01961bb77c250/> rename `GITOXIDE_*` environment variables to `GIX_#`
 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 50 commits contributed to the release.
 - 244 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#960](https://github.com/GitoxideLabs/gitoxide/issues/960)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#960](https://github.com/GitoxideLabs/gitoxide/issues/960)**
    - Try to prevent 'git' reading user and system configuration ([`33be0e0`](https://github.com/GitoxideLabs/gitoxide/commit/33be0e032e4802b0d8e261cccf19ecb9244ffb36))
 * **Uncategorized**
    - Release gix-testtools v0.13.0 ([`edb12ff`](https://github.com/GitoxideLabs/gitoxide/commit/edb12ff7216fdc9b0cf9960949b5794e6fa3cd08))
    - Set `gix-testtools` package versions so that it can be released ([`793c2af`](https://github.com/GitoxideLabs/gitoxide/commit/793c2afbb5e4a5c817b06c15103310781769abfd))
    - Release gix-testtools v0.13.0 ([`7fe620d`](https://github.com/GitoxideLabs/gitoxide/commit/7fe620d6135e7619c5484aeca7ca8823bee585e1))
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade testtools dependencies to latest feasible version ([`29c5904`](https://github.com/GitoxideLabs/gitoxide/commit/29c59045425dcfc23392e0b766d6d6ca399b00a4))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Write informative message if archives are ignored due to script change. ([`06d4682`](https://github.com/GitoxideLabs/gitoxide/commit/06d4682c9fed696fee09234223814016a6453a6d))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Merge branch 'adjustments-for-cargo' ([`8156340`](https://github.com/GitoxideLabs/gitoxide/commit/8156340724b1b7cb15824f88c75f6ddd7302cff5))
    - Rename `GITOXIDE_*` environment variables to `GIX_#` ([`2189cee`](https://github.com/GitoxideLabs/gitoxide/commit/2189cee47f99350b368390eaa2a01961bb77c250))
    - Upgrade to `winnow` 0.5.24 ([`abcfb65`](https://github.com/GitoxideLabs/gitoxide/commit/abcfb659786425ec09eff6b644cd2ad36b7d6bc4))
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Switch `nom` to `winnow` in remaining uses in `gix-object`, `gix-ref`, and `gix-actor` for ~20% more performance. ([`ef54aab`](https://github.com/GitoxideLabs/gitoxide/commit/ef54aab9e5521add4154ee8d902d62612a9d8d4a))
    - Refactor and fixes ([`02587fc`](https://github.com/GitoxideLabs/gitoxide/commit/02587fc879c54b2b3e62ffbe1ab4c29591ea0d80))
    - Upgrade `winnow` to latest patch release ([`8c41848`](https://github.com/GitoxideLabs/gitoxide/commit/8c4184817e4e4364c34badc8ff0a71c6ae952efd))
    - Switch errors to StrContext ([`df226dd`](https://github.com/GitoxideLabs/gitoxide/commit/df226dd31df2c591c6470ed70098202112e13dae))
    - Show more error details in parse tests failures ([`266864f`](https://github.com/GitoxideLabs/gitoxide/commit/266864f35dc9ee96b81d22281c8f267fd7c059a4))
    - Minor cleanup possible with 0.5 ([`a07590c`](https://github.com/GitoxideLabs/gitoxide/commit/a07590cb46423cb0422c18b9fc04b153c0fd53b1))
    - Upgrade to Winnow 0.5 ([`3f8c91f`](https://github.com/GitoxideLabs/gitoxide/commit/3f8c91fa463fbb53d54b2bf359e0dee7387afa00))
    - Upgrade to Winnow 0.4 ([`86ea47f`](https://github.com/GitoxideLabs/gitoxide/commit/86ea47f28079c51f874b0d662867040b92f88d14))
    - Resolve remaining winnow 0.3 deprecations ([`fee441d`](https://github.com/GitoxideLabs/gitoxide/commit/fee441da875d52b1a0cb557d2fa58cee9c29e16a))
    - Switch gix to winnow 0.3 ([`ee75de1`](https://github.com/GitoxideLabs/gitoxide/commit/ee75de1e6035305fc23bdef2522ae5081272ac82))
    - Merge branch 'limit-git' ([`68d9e80`](https://github.com/GitoxideLabs/gitoxide/commit/68d9e809d4e746fd7beaddeabd3313d59a4cbdfd))
    - Merge branch 'dev-on-linux' ([`6b4a303`](https://github.com/GitoxideLabs/gitoxide/commit/6b4a30330fe49fc97daa73f55bf56580cc0597aa))
    - Better debug output when fixture script script fails ([`11cb431`](https://github.com/GitoxideLabs/gitoxide/commit/11cb4317c75864bf310f4964edba7cf487a604f9))
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
    - Don't let scripts run on (potentially) partially extracted archives and don't create archives on windows ([`ed1407c`](https://github.com/GitoxideLabs/gitoxide/commit/ed1407c85525a524bcfa0a4a021a22de339e6149))
    - Curtail `bstr` features to exactly what's needed. ([`7f7db97`](https://github.com/GitoxideLabs/gitoxide/commit/7f7db9794c23b87c8ea50b7bcf38955c9d977624))
    - Upgrade memmap2 and fastrand dependencies ([`6fc7497`](https://github.com/GitoxideLabs/gitoxide/commit/6fc74971ac6838cbfd9c869ba3746713001d7a38))
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - Allow gix-testtools to refer to the local crates that don't cause cycles. ([`082a6fc`](https://github.com/GitoxideLabs/gitoxide/commit/082a6fc65ae08ea0fda11a3340941d58ead4036a))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Auto-fix clippy to remove explicit iter looping ([`3eff567`](https://github.com/GitoxideLabs/gitoxide/commit/3eff567c683b5c650c14792b68968cbdbc90ec5c))
</details>

## 0.12.0 (2023-04-29)

<csr-id-b973f19274bb2d8218e5ff63ce0a81f34985f54c/>

### Documentation

 - <csr-id-cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5/> fix minor typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 68 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-discover v0.18.1, gix-worktree v0.17.1, gix-testtools v0.12.0 ([`f7b6c6f`](https://github.com/GitoxideLabs/gitoxide/commit/f7b6c6f27c090cbc584fbd3f5403da5ac1a9ff02))
    - Prepare changelogs prior to release of gix-testtools ([`fc45f1b`](https://github.com/GitoxideLabs/gitoxide/commit/fc45f1b417bf545d4a0a105c40b37f92c24decad))
    - Upgrade dependencies ([`b973f19`](https://github.com/GitoxideLabs/gitoxide/commit/b973f19274bb2d8218e5ff63ce0a81f34985f54c))
    - Accept paths in scripted_fixture_writable ([`efcbf0d`](https://github.com/GitoxideLabs/gitoxide/commit/efcbf0d1cb1c9d77eaf04fbcf6e86dc101c886d2))
    - Add note about shortcomings of `Creation::CopyFromReadOnly` mode. ([`b2e3223`](https://github.com/GitoxideLabs/gitoxide/commit/b2e322332ac017824e90d260d7041504c38ab57f))
    - Fix minor typos ([`cc48c35`](https://github.com/GitoxideLabs/gitoxide/commit/cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5))
    - Prepare for git-tempfile release ([`56c005b`](https://github.com/GitoxideLabs/gitoxide/commit/56c005b13c44376f71e61781e73c0bf93416d0e4))
</details>

## 0.11.0 (2023-02-20)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>
<csr-id-29bf8ca8399b6d4941aa242b9f08c74e59a179bb/>
<csr-id-1d5ab44145ccbc2064ee8cc7acebb62db82c45aa/>

### New Features

 - <csr-id-fb68bffcb26582d508db946f72234bfd847a3a11/> `set_current_dir()` to change the CWD and reset it to previous version on drop.
 - <csr-id-15ecd841cfe7c77bbdfdfa232dd51a44c4940bbc/> allow execution of scripts without 'bash'.
   This works by trying to execute the file directly, and on failure, use 'bash'
   as interpreter.
   
   That way we are finally able to support a wider variety of fixture generators
   and make the crate more useful to a wieder audience.
 - <csr-id-221f1374aa004a76693cfb1529daab930a5a9dd7/> `spawn_git_daemon()` to spawn a git daemon hosting a working directoy…
   …with support for multiple at a time thanks to port selection (allowing
   tests to run in parallel) as well as auto-kill on drop.
 - <csr-id-67777a81f8d9d0335475e4fe4cbf770c328bd24f/> increase the waiting time on MacOS for file base locks
   It appears that these workers run into timeouts more and more,
   they got slower or maybe there are just more tests.
 - <csr-id-09da4c5eeff5c6657beb9c53c168f90e74d6f758/> add `Env::unset()` for convenience
 - <csr-id-231785644194cd3be0b0dab06224a39ecf0ed714/> Provide `GIT_VERSION` information along with a way to skip a test based on it.
 - <csr-id-654b521323a5822cbb86e57bee159d90576fa5ff/> expose `on_ci` in the top-level.
 - <csr-id-449b6c1555fc2832c712ba51cd41ab9ed79e0b15/> Allow to re-execute scripts into temp directories.
   This is important in cases where the files created by the script
   contain absolute mentions of locations. That way, when copying
   files over, the test might accidentally return to the original
   read-only location, and write into it making future test runs fail.
 - <csr-id-f1635c3ee36678cff9f26135946c281bf4a75331/> publicly accessible `Result` type

### Bug Fixes

 - <csr-id-00286c9cf63b5eba9534ef7639805545ec40eb03/> `gix-testtools` use the latest dependencies
   These should compile properly.
 - <csr-id-e14dc7d475373d2c266e84ff8f1826c68a34ab92/> note that crates have been renamed from `git-*` to `gix-*`.
   This also means that the `git-*` prefixed crates of the `gitoxide` project
   are effectively unmaintained.
   Use the crates with the `gix-*` prefix instead.
   
   If you were using `git-repository`, then `gix` is its substitute.
 - <csr-id-761b7d71977a5aa4876010faa61ab88f0dba6eab/> don't overwrite unexpanded `git-lfs` pointer files.
   It's possible for those with incomplete `git-lfs` installations
   (and many more situations) to end up in a spot where pointer files
   aren't expanded. If we overwrite the with archives, files look
   changed which can be confusing and lead to even bigger messes
   to happen.
   
   Now we don't overwrite those files anyomre.
 - <csr-id-1ce3190000f6211ce31468c7603d491bb5b90293/> Disable tag.gpgSign in test scripts
   This is done for the same reason that commit.gpgsign is disabled for test
   scripts. It prevents test failures if the user has tag.gpgsign enabled in
   their global git config when invoking tests.
 - <csr-id-cba9edeb403aae4d77087de4167cbabe72525d92/> Allow multiple scripts to run at the same time, if they are not the same.
   Previously, per integration test and thus per crate, one would
   effectively only be able to run a single script at a time because of the
   global identity lock. This was required previously before the additional
   file based lock, per script name, was introduced.
   
   This is now fixed by dropping the lock after the script identity was
   obtained.
 - <csr-id-004dab17deab4c360adb5ac428f6b4951c974fe3/> `_with_args(…)` functions now allow non-static strings

### Changed (BREAKING)

 - <csr-id-dbf6c8c87cdca8169ac01aa89aefe56a33215142/> rename `scripted_fixture_*` to not contain 'repo' in the name.
   Further make clear in the documentation that `bash` is used to execute
   the fixture scripts, previously it wasn't even implied and got
   lost in history.
 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

### New Features (BREAKING)

 - <csr-id-21bd6075ca36ca49b3c85d0431ec11f68e6e9f9c/> remove `hex_to_id()` and add various `*_standalone()` versions of existing methods.
   With these it's possible to handle fully standalone (as in with their own Cargo.toml) integration
   tests which are needed to resolve cyclic dependencies.
 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `git-features` and `git-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 247 commits contributed to the release.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#364](https://github.com/GitoxideLabs/gitoxide/issues/364), [#366](https://github.com/GitoxideLabs/gitoxide/issues/366), [#382](https://github.com/GitoxideLabs/gitoxide/issues/382), [#384](https://github.com/GitoxideLabs/gitoxide/issues/384), [#391](https://github.com/GitoxideLabs/gitoxide/issues/391), [#393](https://github.com/GitoxideLabs/gitoxide/issues/393), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#488](https://github.com/GitoxideLabs/gitoxide/issues/488), [#509](https://github.com/GitoxideLabs/gitoxide/issues/509), [#607](https://github.com/GitoxideLabs/gitoxide/issues/607), [#650](https://github.com/GitoxideLabs/gitoxide/issues/650), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 11 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Fix windows tests by transforming line endings ([`e276d77`](https://github.com/GitoxideLabs/gitoxide/commit/e276d777eb7a88dc424badbf88a929b5f567e5de))
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - A failing test to show the handle-stability doesn't quite work yet ([`5562e88`](https://github.com/GitoxideLabs/gitoxide/commit/5562e8888cd8ac8fc3d89a41f8e8cc5cec7b8ca6))
    - Refactor ([`c499843`](https://github.com/GitoxideLabs/gitoxide/commit/c499843485a8af102cb4d3594c4e6014976c5aa0))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - REUC reading works ([`29c1af9`](https://github.com/GitoxideLabs/gitoxide/commit/29c1af9b2d7b9879a806fc84cfc89ed6c0d7f083))
    - Use parking_lot mutex to avoid poison errors ([`d8ca74f`](https://github.com/GitoxideLabs/gitoxide/commit/d8ca74f358e802916353f545b90127f9a7bb5137))
    - Base setup for index testing ([`aa60fdf`](https://github.com/GitoxideLabs/gitoxide/commit/aa60fdf3d86e08877c88f9e4973f546642ed1370))
 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Upgrade dependencies ([`b039d39`](https://github.com/GitoxideLabs/gitoxide/commit/b039d39613bb14d49670c4d8b586f76ffb420d03))
    - Upgrade parking_lot and cargo_toml ([`f95c1a0`](https://github.com/GitoxideLabs/gitoxide/commit/f95c1a0d9c19bcc6feb9b8739a09d86f9970a0e0))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Allow to re-execute scripts into temp directories. ([`449b6c1`](https://github.com/GitoxideLabs/gitoxide/commit/449b6c1555fc2832c712ba51cd41ab9ed79e0b15))
    - Don't print archive message if archive is excluded ([`c6bd30e`](https://github.com/GitoxideLabs/gitoxide/commit/c6bd30e81997931d1f65a62924d20fe5e74b8521))
    - Support unique directories for different platforms ([`0b385b3`](https://github.com/GitoxideLabs/gitoxide/commit/0b385b31cf95d500f9ec2d05be0894956e40e4a1))
    - Use git exclude information to determine if archives should be generated ([`4a3dccc`](https://github.com/GitoxideLabs/gitoxide/commit/4a3dccc7fc7a5e190d88af8c7eb0713edbada55f))
    - Add TODO ([`778fd77`](https://github.com/GitoxideLabs/gitoxide/commit/778fd7703920e6a2693beb59aad611f3c9fab106))
    - Publicly accessible `Result` type ([`f1635c3`](https://github.com/GitoxideLabs/gitoxide/commit/f1635c3ee36678cff9f26135946c281bf4a75331))
    - Refactor ([`9ea1e44`](https://github.com/GitoxideLabs/gitoxide/commit/9ea1e4474a3ce803da7a56e1fc1748f65c11a876))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Expose `on_ci` in the top-level. ([`654b521`](https://github.com/GitoxideLabs/gitoxide/commit/654b521323a5822cbb86e57bee159d90576fa5ff))
    - Move `Env` test utility into `git-testtools` ([`bd3f4d0`](https://github.com/GitoxideLabs/gitoxide/commit/bd3f4d014dd7df7a1e25defa8eea7253eec1560a))
 * **[#364](https://github.com/GitoxideLabs/gitoxide/issues/364)**
    - Add test-tools changelog prior to release ([`1ebc16a`](https://github.com/GitoxideLabs/gitoxide/commit/1ebc16a6ac9ef188c188a52737820773aa949cee))
 * **[#366](https://github.com/GitoxideLabs/gitoxide/issues/366)**
    - Quickfix for unintentionally using 'unicode' feature of bytecode ([`fb5593a`](https://github.com/GitoxideLabs/gitoxide/commit/fb5593a7272498ae042b6c8c7605faa3d253fa10))
 * **[#382](https://github.com/GitoxideLabs/gitoxide/issues/382)**
    - Simplify state tests ([`fc61c0d`](https://github.com/GitoxideLabs/gitoxide/commit/fc61c0d4f7cb3cd9073418e4d8edc55cd14f5fb3))
 * **[#384](https://github.com/GitoxideLabs/gitoxide/issues/384)**
    - Enforce signal handler setup to cleanup tempfiles on abort ([`1caf3ae`](https://github.com/GitoxideLabs/gitoxide/commit/1caf3ae2cabee776dc45a687f00ce386c27ab87d))
    - No need to isolate archives by crate name ([`19d46f3`](https://github.com/GitoxideLabs/gitoxide/commit/19d46f35440419b9911b6e2bca2cfc975865dce9))
    - Provide some more information when using archives; debug windows more ([`4f5b1fd`](https://github.com/GitoxideLabs/gitoxide/commit/4f5b1fd5e6440208c460388a9d69d664d6d8d0d7))
    - Protect test generation from multi-process races ([`1aec924`](https://github.com/GitoxideLabs/gitoxide/commit/1aec924f009fd16b953cd1313b9408558b1c7aeb))
    - Definitely don't follow symlnks ([`1343448`](https://github.com/GitoxideLabs/gitoxide/commit/13434481c44efbc170cb74dd9057807c3ee58e01))
    - Make sure existing files aren't written into ([`9b5a8a2`](https://github.com/GitoxideLabs/gitoxide/commit/9b5a8a243d49b6567d1db31050d3bf3123dd54d3))
    - Extraction of tar archives with identity check ([`07c1f07`](https://github.com/GitoxideLabs/gitoxide/commit/07c1f0752fefbd3e49ef414bced2ca6bbc844448))
    - Assure there are no archive file-name clashes across crates ([`c30bebf`](https://github.com/GitoxideLabs/gitoxide/commit/c30bebf4f0272fe728e18b1932e419128f63ed44))
    - Actual compression of archives ([`5dd3d82`](https://github.com/GitoxideLabs/gitoxide/commit/5dd3d82aa68c9024cd1742043a3c56cd6b0665fd))
    - Simple creation of test-archives ([`f1e107a`](https://github.com/GitoxideLabs/gitoxide/commit/f1e107aa864107e02309b15b41da8d8f962e19a6))
    - Make sure archives are handled by git-lfs ([`f744a6c`](https://github.com/GitoxideLabs/gitoxide/commit/f744a6cc8b453ea349664540af4be0566e376528))
    - Frame for extracting and generating archives ([`92c7044`](https://github.com/GitoxideLabs/gitoxide/commit/92c7044cfbc3054b237ea7c79da981bb91908812))
    - Further partition generated test directories by script name ([`e141ddb`](https://github.com/GitoxideLabs/gitoxide/commit/e141ddbdd2e0677e921856b30096733530fde569))
    - Auto-set commit.gpgsign=false when executing git ([`c23feb6`](https://github.com/GitoxideLabs/gitoxide/commit/c23feb64ad157180cfba8a11c882b829733ea8f6))
 * **[#391](https://github.com/GitoxideLabs/gitoxide/issues/391)**
    - Also write a failure marker if archive creation failed ([`7f88c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7f88c7f9d908df39ad4e710402783dca35eb758f))
    - Auto-clean test fixtures on re-run if they failed previously ([`3617ff4`](https://github.com/GitoxideLabs/gitoxide/commit/3617ff411224a691057eb1c39c4144b932b33f51))
 * **[#393](https://github.com/GitoxideLabs/gitoxide/issues/393)**
    - Add support for disabling archive usage ([`624ad2e`](https://github.com/GitoxideLabs/gitoxide/commit/624ad2ef42172556efe942129f6f46dd627250d5))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - Make fmt ([`4b320e7`](https://github.com/GitoxideLabs/gitoxide/commit/4b320e773368ac5e8c38dd8a779ef3d6d2d024ec))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Add `Env::unset()` for convenience ([`09da4c5`](https://github.com/GitoxideLabs/gitoxide/commit/09da4c5eeff5c6657beb9c53c168f90e74d6f758))
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
    - Allow multiple scripts to run at the same time, if they are not the same. ([`cba9ede`](https://github.com/GitoxideLabs/gitoxide/commit/cba9edeb403aae4d77087de4167cbabe72525d92))
    - Make tests more robust; fix windows tests ([`1983fbc`](https://github.com/GitoxideLabs/gitoxide/commit/1983fbc39be3da5598cf3af6fb97f6ea0bc3ec6b))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#488](https://github.com/GitoxideLabs/gitoxide/issues/488)**
    - Provide `GIT_VERSION` information along with a way to skip a test based on it. ([`2317856`](https://github.com/GitoxideLabs/gitoxide/commit/231785644194cd3be0b0dab06224a39ecf0ed714))
 * **[#509](https://github.com/GitoxideLabs/gitoxide/issues/509)**
    - Some unit tests for the time when something truly unparseable shows up ([`94fc0d6`](https://github.com/GitoxideLabs/gitoxide/commit/94fc0d60d21c22a0d36f5de986cd9443755141bf))
    - Be more verbose when git version parsing fails ([`9c2f1b5`](https://github.com/GitoxideLabs/gitoxide/commit/9c2f1b5d03fbcf5dd08c2469ea17da426ea6670c))
 * **[#607](https://github.com/GitoxideLabs/gitoxide/issues/607)**
    - Don't overwrite unexpanded `git-lfs` pointer files. ([`761b7d7`](https://github.com/GitoxideLabs/gitoxide/commit/761b7d71977a5aa4876010faa61ab88f0dba6eab))
    - Improve documentation to inform about the need for `git-lfs`. ([`519db50`](https://github.com/GitoxideLabs/gitoxide/commit/519db50eac6576906f266a6f0b980f88098e3f9f))
 * **[#650](https://github.com/GitoxideLabs/gitoxide/issues/650)**
    - Allow execution of scripts without 'bash'. ([`15ecd84`](https://github.com/GitoxideLabs/gitoxide/commit/15ecd841cfe7c77bbdfdfa232dd51a44c4940bbc))
    - Rename `scripted_fixture_*` to not contain 'repo' in the name. ([`dbf6c8c`](https://github.com/GitoxideLabs/gitoxide/commit/dbf6c8c87cdca8169ac01aa89aefe56a33215142))
 * **[#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)**
    - `_with_args(…)` functions now allow non-static strings ([`004dab1`](https://github.com/GitoxideLabs/gitoxide/commit/004dab17deab4c360adb5ac428f6b4951c974fe3))
 * **Uncategorized**
    - Release gix-testtools v0.11.0 ([`dfe2402`](https://github.com/GitoxideLabs/gitoxide/commit/dfe24026f9b1d85b8ab01e69dfec6a4188091850))
    - `gix-testtools` use the latest dependencies ([`00286c9`](https://github.com/GitoxideLabs/gitoxide/commit/00286c9cf63b5eba9534ef7639805545ec40eb03))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Note that crates have been renamed from `git-*` to `gix-*`. ([`e14dc7d`](https://github.com/GitoxideLabs/gitoxide/commit/e14dc7d475373d2c266e84ff8f1826c68a34ab92))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
    - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
    - Adjust to renaming of `git-index` to `gix-index` ([`86db5e0`](https://github.com/GitoxideLabs/gitoxide/commit/86db5e09fc58ce66b252dc13b8d7e2c48e4d5062))
    - Adjust to renaming of `git-diff` to `gix-diff` ([`49a163e`](https://github.com/GitoxideLabs/gitoxide/commit/49a163ec8b18f0e5fcd05a315de16d5d8be7650e))
    - Adjust to renaming of `git-commitgraph` to `gix-commitgraph` ([`f1dd0a3`](https://github.com/GitoxideLabs/gitoxide/commit/f1dd0a3366e31259af029da73228e8af2f414244))
    - Adjust to renaming of `git-mailmap` to `gix-mailmap` ([`2e28c56`](https://github.com/GitoxideLabs/gitoxide/commit/2e28c56bb9f70de6f97439818118d3a25859698f))
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
    - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
    - Adjust to renaming of `git-date` to `gix-date` ([`9a79ff2`](https://github.com/GitoxideLabs/gitoxide/commit/9a79ff2d5cc74c1efad9f41e21095ae498cce00b))
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
    - Merge branch 'unc-paths' ([`ff0387e`](https://github.com/GitoxideLabs/gitoxide/commit/ff0387e9975e61a2d796b86f4d857c3b8528c94b))
    - `set_current_dir()` to change the CWD and reset it to previous version on drop. ([`fb68bff`](https://github.com/GitoxideLabs/gitoxide/commit/fb68bffcb26582d508db946f72234bfd847a3a11))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Remove `hex_to_id()` and add various `*_standalone()` versions of existing methods. ([`21bd607`](https://github.com/GitoxideLabs/gitoxide/commit/21bd6075ca36ca49b3c85d0431ec11f68e6e9f9c))
    - Break cyclical dev dependencies ([`1fea18f`](https://github.com/GitoxideLabs/gitoxide/commit/1fea18f5f8b4189a23dc4fa3f041a672f6fbcfb3))
    - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`b34c9fe`](https://github.com/GitoxideLabs/gitoxide/commit/b34c9fe58223862712eacc1cb7353e497a4b9778))
    - Release git-date v0.4.0, git-actor v0.17.0, git-object v0.26.0, git-traverse v0.22.0, git-index v0.12.0, safety bump 15 crates ([`0e3d0a5`](https://github.com/GitoxideLabs/gitoxide/commit/0e3d0a56d7e6a60c6578138f2690b4fa54a2072d))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Adapt to changes in `git-worktree` ([`5a97bb5`](https://github.com/GitoxideLabs/gitoxide/commit/5a97bb5365573895500f0adeb73c482b797051c4))
    - Merge branch 'adjustments-for-cargo' ([`f8c562a`](https://github.com/GitoxideLabs/gitoxide/commit/f8c562a559e6dc3377583cc7200585dad7c3d481))
    - Release git-testtools v0.10.0 ([`926ba5b`](https://github.com/GitoxideLabs/gitoxide/commit/926ba5bf1a5f1b665e0791d12496b8a88bf60be5))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - Thanks clippy ([`f1160fb`](https://github.com/GitoxideLabs/gitoxide/commit/f1160fb42acf59b37cbeda546a7079af3c9bc050))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Merge branch 'git-lfs-improvements' ([`4c1685b`](https://github.com/GitoxideLabs/gitoxide/commit/4c1685b971bb18117897a2c958ac2434bcb4f9e8))
    - Merge branch 'jpgrayson/main' ([`b242853`](https://github.com/GitoxideLabs/gitoxide/commit/b242853abd790e5234b2f18b4aaeddb8f6f4d36f))
    - Disable tag.gpgSign in test scripts ([`1ce3190`](https://github.com/GitoxideLabs/gitoxide/commit/1ce3190000f6211ce31468c7603d491bb5b90293))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Release git-glob v0.4.2, git-config-value v0.8.2, git-lock v2.2.0, git-ref v0.19.0, git-config v0.11.0, git-discover v0.8.0, git-index v0.8.0, git-transport v0.22.0, git-protocol v0.23.0, git-worktree v0.8.0, git-repository v0.28.0, gitoxide-core v0.20.0, gitoxide v0.18.0, safety bump 9 crates ([`0c253b1`](https://github.com/GitoxideLabs/gitoxide/commit/0c253b15143dcedfe4c66d64ab1ea6e097030651))
    - Merge branch 'main' into http-config ([`f4ff821`](https://github.com/GitoxideLabs/gitoxide/commit/f4ff821fd4233dd1dc1a449af4d4600becf3b4ac))
    - Merge branch 'async-fetch' ([`0c9c48b`](https://github.com/GitoxideLabs/gitoxide/commit/0c9c48b3b91a1396eb1796f288a2cb10380d1f14))
    - Let's be very conservative regarding maximum lock times ([`ba83945`](https://github.com/GitoxideLabs/gitoxide/commit/ba83945bf885fd14b841323655991554af8b33d1))
    - This should work on windows (when launching the git-daemon) ([`52f4095`](https://github.com/GitoxideLabs/gitoxide/commit/52f4095812f311abeb0184bfb70b133de64a6b62))
    - Make sure we can shut-down the daemon by starting it directly ([`4924b33`](https://github.com/GitoxideLabs/gitoxide/commit/4924b33b40fa874ec3cc22476680ffce3eb30c84))
    - `spawn_git_daemon()` to spawn a git daemon hosting a working directoy… ([`221f137`](https://github.com/GitoxideLabs/gitoxide/commit/221f1374aa004a76693cfb1529daab930a5a9dd7))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Merge branch 'main' into write-sparse-index (upgrade to Rust 1.65) ([`5406630`](https://github.com/GitoxideLabs/gitoxide/commit/5406630466145990b5adbdadb59151036993060d))
    - Thanks clippy ([`04cfa63`](https://github.com/GitoxideLabs/gitoxide/commit/04cfa635a65ae34ad6d22391f2febd2ca7eabca9))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Assure the 'file' protocol is always allowed ([`7086101`](https://github.com/GitoxideLabs/gitoxide/commit/7086101d3950b3e5ecb143b78185f2988cfb8fe8))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Merge branch 'fetch-pack' ([`3c49400`](https://github.com/GitoxideLabs/gitoxide/commit/3c49400809c7c2120f4ce704c19a0421545b5acd))
    - Merge branch 'main' into fetch-pack ([`93917cb`](https://github.com/GitoxideLabs/gitoxide/commit/93917cb6ecbb30daf3d20bb5a7c65e12211f084f))
    - Increase the waiting time on MacOS for file base locks ([`67777a8`](https://github.com/GitoxideLabs/gitoxide/commit/67777a81f8d9d0335475e4fe4cbf770c328bd24f))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
    - Make fmt ([`535e967`](https://github.com/GitoxideLabs/gitoxide/commit/535e967666c6da657ff1b7eff7c64ab27cafb182))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into filter-refs-by-spec ([`1f6e5ab`](https://github.com/GitoxideLabs/gitoxide/commit/1f6e5ab15f5fd8d23719b13e6aea59cd231ac0fe))
    - Merge branch 'fix-522' ([`5869e9f`](https://github.com/GitoxideLabs/gitoxide/commit/5869e9ff2508d5a93c07635277af8764fcb57713))
    - Release git-hash v0.9.9 ([`da0716f`](https://github.com/GitoxideLabs/gitoxide/commit/da0716f8c27b4f29cfff0e5ce7fcb3d7240f4aeb))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into filter-refs-by-spec ([`51dc828`](https://github.com/GitoxideLabs/gitoxide/commit/51dc8282fb77b519ff7d2c94c6bd73af306cfe8b))
    - Release git-diff v0.18.1, git-discover v0.4.2, git-traverse v0.16.4, git-repository v0.23.1 ([`2571831`](https://github.com/GitoxideLabs/gitoxide/commit/2571831e5939bf4ea6f19537b0c1ccd71dc99088))
    - Merge branch 'main' into filter-refs-by-spec ([`56ba481`](https://github.com/GitoxideLabs/gitoxide/commit/56ba481f4c48f74f10397feb1b6dc3d7dd3704fb))
    - Merge branch 'joelparkerhenderson/main' ([`239cb8a`](https://github.com/GitoxideLabs/gitoxide/commit/239cb8a7c25f89ad087f201982585ab4c904c77b))
    - Fix format ([`1b00ab1`](https://github.com/GitoxideLabs/gitoxide/commit/1b00ab1d2a38e0ee33570714760a21cc8ca3785e))
    - Fix git_version_from_bytes to handle trailing newline ([`14e4e66`](https://github.com/GitoxideLabs/gitoxide/commit/14e4e66fe064114f3d9f1dc07ce34497abf8374e))
    - Merge branch 'main' into filter-refs-by-spec ([`a36c05d`](https://github.com/GitoxideLabs/gitoxide/commit/a36c05d281269f3f8b297e7adc463bfb3c306663))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-worktree v0.4.3, git-testtools v0.8.0 ([`b2e4bf2`](https://github.com/GitoxideLabs/gitoxide/commit/b2e4bf2c11ff2c3c32efcb91837fb5677714bdf9))
    - Release git-attributes v0.3.3, git-ref v0.15.3, git-index v0.4.3, git-worktree v0.4.3, git-testtools v0.8.0 ([`baad4ce`](https://github.com/GitoxideLabs/gitoxide/commit/baad4ce51fe0e8c0c1de1b08148d8303878ca37b))
    - Prepare changelogs prior to release of git-testtools ([`7668e38`](https://github.com/GitoxideLabs/gitoxide/commit/7668e38fab8891ed7e73fae3a6f5a8772e0f0d0b))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Thanks clippy ([`9aa8277`](https://github.com/GitoxideLabs/gitoxide/commit/9aa827785c25e63dd1b351a7cc553f140fb93c2e))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
    - Merge branch 'main' into remote-ls-refs ([`c4bf958`](https://github.com/GitoxideLabs/gitoxide/commit/c4bf9585d815bc342e5fb383336cc654280dd34f))
    - Fix CI for good ([`e0c0b8c`](https://github.com/GitoxideLabs/gitoxide/commit/e0c0b8c7c1898b2bc11a915e8e4fb8426295ccbb))
    - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Add docs related to archives. ([`f409a2a`](https://github.com/GitoxideLabs/gitoxide/commit/f409a2ae88f2b0d80c7d160563c07935993203a6))
    - Add documentation to test-tools. ([`074b283`](https://github.com/GitoxideLabs/gitoxide/commit/074b2833d15c8483bd89e4bde4486c0c7df14637))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Make fmt ([`47724c0`](https://github.com/GitoxideLabs/gitoxide/commit/47724c0edb382c036a3fc99884becfd2b0740d4b))
    - Release git-hash v0.9.6, git-features v0.22.0, git-date v0.0.2, git-actor v0.11.0, git-glob v0.3.1, git-path v0.4.0, git-attributes v0.3.0, git-tempfile v2.0.2, git-object v0.20.0, git-ref v0.15.0, git-sec v0.3.0, git-config v0.6.0, git-credentials v0.3.0, git-diff v0.17.0, git-discover v0.3.0, git-index v0.4.0, git-mailmap v0.3.0, git-traverse v0.16.0, git-pack v0.21.0, git-odb v0.31.0, git-url v0.7.0, git-transport v0.19.0, git-protocol v0.18.0, git-revision v0.3.0, git-worktree v0.4.0, git-repository v0.20.0, git-commitgraph v0.8.0, gitoxide-core v0.15.0, gitoxide v0.13.0, safety bump 22 crates ([`4737b1e`](https://github.com/GitoxideLabs/gitoxide/commit/4737b1eea1d4c9a8d5a69fb63ecac5aa5d378ae5))
    - Merge branch 'config-cascade' ([`f144eaf`](https://github.com/GitoxideLabs/gitoxide/commit/f144eaf5863ae5cac63103f0db51c35fcf03a948))
    - Thanks clippy ([`49f5a54`](https://github.com/GitoxideLabs/gitoxide/commit/49f5a5415c119267ea37e20fb198df80f621cbde))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Release git-sec v0.1.2, git-discover v0.1.3, cargo-smart-release v0.10.2 ([`6cd365e`](https://github.com/GitoxideLabs/gitoxide/commit/6cd365e2cf6851f5cdecc22f3b1667440ad011b0))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Release git-path v0.1.3, git-discover v0.1.2, git-repository v0.18.1, cargo-smart-release v0.10.1 ([`b7399cc`](https://github.com/GitoxideLabs/gitoxide/commit/b7399cc44ee419355a649a7b0ba7b352cd48b400))
    - Release git-path v0.1.2, git-sec v0.1.1, git-config v0.4.0, git-discover v0.1.1, git-pack v0.19.1, git-repository v0.18.0, cargo-smart-release v0.10.0, safety bump 2 crates ([`ceb6dff`](https://github.com/GitoxideLabs/gitoxide/commit/ceb6dff13362a2b4318a551893217c1d11643b9f))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Make fmt ([`e043807`](https://github.com/GitoxideLabs/gitoxide/commit/e043807abf364ca46d00760e2f281528efe20c75))
    - Merge branch 'refs-and-worktrees' ([`8131227`](https://github.com/GitoxideLabs/gitoxide/commit/8131227ddff6f36919b6a0f7b33792ebde0f8ae9))
    - Thanks clippy ([`60cf67c`](https://github.com/GitoxideLabs/gitoxide/commit/60cf67cb081b91932d9943b9c525cac2c0cf0782))
    - Merge branch 'main' into git_includeif ([`b1bfc8f`](https://github.com/GitoxideLabs/gitoxide/commit/b1bfc8fe8efb6d8941f54dddd0fcad99aa13ed6c))
    - Merge branch 'basic-worktree-support' ([`e058bda`](https://github.com/GitoxideLabs/gitoxide/commit/e058bdabf8449b6a6fdff851e3929137d9b71568))
    - Merge branch 'main' into git_includeif ([`05eb340`](https://github.com/GitoxideLabs/gitoxide/commit/05eb34023933918c51c03cf2afd774db89cc5a33))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Make fmt ([`251b6df`](https://github.com/GitoxideLabs/gitoxide/commit/251b6df5dbdda24b7bdc452085f808f3acef69d8))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Set the time to wait for lock to longest expected runtime of fixture scripts ([`eea3988`](https://github.com/GitoxideLabs/gitoxide/commit/eea3988462a61e8a64d646a15d062d13fdbfb615))
    - More robust archive creation on windows ([`e7b2d8f`](https://github.com/GitoxideLabs/gitoxide/commit/e7b2d8f446b41b26b518abf7d1b048605ef2bbe8))
    - Merge branch 'main' into repo-status ([`0eb2372`](https://github.com/GitoxideLabs/gitoxide/commit/0eb23721dca78f6e6bf864c5c3a3e44df8b419f0))
    - Merge branch 'test-archive-support' ([`350df01`](https://github.com/GitoxideLabs/gitoxide/commit/350df01042d6ca8b93f8737fa101e69b50535a0f))
    - Thanks clippy ([`658862e`](https://github.com/GitoxideLabs/gitoxide/commit/658862eeb042073632f5a3f203e264a47151d454))
    - Thanks clippy ([`c8d218c`](https://github.com/GitoxideLabs/gitoxide/commit/c8d218c6399f52fb1a57eca22005196d1c686774))
    - Release git-testtools v0.6.0 ([`45386a0`](https://github.com/GitoxideLabs/gitoxide/commit/45386a0b135656681dbdf8c47ad888b50e68f151))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Thanks clippy ([`1038dab`](https://github.com/GitoxideLabs/gitoxide/commit/1038dab842b32ec1359a53236b241a91427ccb65))
    - Add `fixture_bytes` to test tools ([`85e3820`](https://github.com/GitoxideLabs/gitoxide/commit/85e3820caa106a32c3406fd1e9e4c67fb0033bc5))
    - Commit to using 'unicode' feature of bstr as git-object wants it too ([`471fa62`](https://github.com/GitoxideLabs/gitoxide/commit/471fa62b142ba744541d7472464d62826f5c6b93))
    - Release git-hash v0.9.2, git-object v0.17.1, git-pack v0.16.1 ([`0db19b8`](https://github.com/GitoxideLabs/gitoxide/commit/0db19b8deaf11a4d4cbc03fa3ae40eea104bc302))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Ensure tests use 'merge.ff false' and recreate fixtures on each run ([`1d5ab44`](https://github.com/GitoxideLabs/gitoxide/commit/1d5ab44145ccbc2064ee8cc7acebb62db82c45aa))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Merge branch 'changelog-generation' ([`bf0106e`](https://github.com/GitoxideLabs/gitoxide/commit/bf0106ea21734d4e59d190b424c22743c22da966))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - Merge branch 'Byron:main' into main ([`dc58eca`](https://github.com/GitoxideLabs/gitoxide/commit/dc58eca510e5a067acdeaad4b595a34b4598a0cd))
    - Release git-testtools v0.5.0 ([`86e0a92`](https://github.com/GitoxideLabs/gitoxide/commit/86e0a92c7dc3b69a766aeac1b675b148d61a7ec5))
    - Upgrade to nom-7 ([`f0aa3e1`](https://github.com/GitoxideLabs/gitoxide/commit/f0aa3e1b5b407b2afd187c9cb622676fcddaf706))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - (cargo-release) version 0.4.0 ([`70ef344`](https://github.com/GitoxideLabs/gitoxide/commit/70ef3442775b54ba9e4ee9ebfffb37af9804cc5b))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - [pack] refactor ([`9ee1e22`](https://github.com/GitoxideLabs/gitoxide/commit/9ee1e22fa5c5d97ff626f0dfc44706272433bfef))
    - [ref] packed refs header line parsing ([`fde5543`](https://github.com/GitoxideLabs/gitoxide/commit/fde5543ad22395e27266db02a5442a33d16e68c5))
    - [tools] fix create writable fixture ([`bf7783d`](https://github.com/GitoxideLabs/gitoxide/commit/bf7783dd9ccc9ac433b978b9dded0d38f7351494))
    - [ref] on the way towards realistic transactions… ([`c808cb1`](https://github.com/GitoxideLabs/gitoxide/commit/c808cb17b2fea12e018fabb789862e9b7703e49b))
    - [ref] on the way to setup the first transaction test ([`29c0b51`](https://github.com/GitoxideLabs/gitoxide/commit/29c0b51625e2c7e3a8d60075bb925126a024dc83))
    - Bump once_cell from 1.7.2 to 1.8.0 ([`bd323d9`](https://github.com/GitoxideLabs/gitoxide/commit/bd323d911b6becf8b379343c6ef56ec46e28fa28))
    - (cargo-release) version 0.3.0 ([`6b33678`](https://github.com/GitoxideLabs/gitoxide/commit/6b33678f83e6d261ca15c4a7634ff5b4e66d81dd))
    - Merge branch 'dependabot/cargo/crc-2.0.0' ([`683c44d`](https://github.com/GitoxideLabs/gitoxide/commit/683c44db682d8dbef401286963e84cdca145abc8))
    - (cargo-release) version 0.2.0 ([`3286e42`](https://github.com/GitoxideLabs/gitoxide/commit/3286e42547b59df6365087cbae9ce1c9c959faad))
    - Manually fix crc in tooling ([`48fa9bc`](https://github.com/GitoxideLabs/gitoxide/commit/48fa9bc80876a0186f43add6c6d3477385241f5e))
    - Bump crc from 1.8.1 to 2.0.0 ([`07f08ac`](https://github.com/GitoxideLabs/gitoxide/commit/07f08ac1ea04ec278993ad1a5fc1d4f243bf8eb7))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - [git-ref] the first failing test ([`7e802a0`](https://github.com/GitoxideLabs/gitoxide/commit/7e802a0576230dfc666c253d484ea255f265f92f))
    - Prepare test utilities for release… ([`d35e654`](https://github.com/GitoxideLabs/gitoxide/commit/d35e654747f96cec93bdecd1314ce325129cbc44))
    - [tree-diff] Beginning of more nested test-suite… ([`b8a90e7`](https://github.com/GitoxideLabs/gitoxide/commit/b8a90e7c9347b0eefdbef6f4c724cc0561cd79c9))
    - Fix debug assert, thanks gitpython ([`fe954b9`](https://github.com/GitoxideLabs/gitoxide/commit/fe954b9f6d26bd8629f24a01bd2a06f9800deed0))
    - Revert "FAIL: try to disable GPG signing with environment variables…" ([`e326352`](https://github.com/GitoxideLabs/gitoxide/commit/e326352eec7bd1aae13f770328979e5730ffc32b))
    - Try to disable GPG signing with environment variables… ([`29bf8ca`](https://github.com/GitoxideLabs/gitoxide/commit/29bf8ca8399b6d4941aa242b9f08c74e59a179bb))
    - Thanks, cargo audit ([`4f293f5`](https://github.com/GitoxideLabs/gitoxide/commit/4f293f5036c44a69ccacf102d35202adad83bbe0))
    - Thanks clippy ([`002792a`](https://github.com/GitoxideLabs/gitoxide/commit/002792a8bc2512c92c16fd28662c26c9b3a12572))
    - Set environment in testtools to freeze repositories generation scripts ([`eaad3ab`](https://github.com/GitoxideLabs/gitoxide/commit/eaad3ab69338115439a553ba1062160dc3a08082))
    - Faster repeated tests if fixtures don't change ([`792277f`](https://github.com/GitoxideLabs/gitoxide/commit/792277f241446086dd6c9b78f688363d4e66e5a7))
    - Allow the use of shared test utilities across crates ([`b117626`](https://github.com/GitoxideLabs/gitoxide/commit/b117626df6da714c24d2b7914301678e89d2d0cb))
    - The first test with the new and nice and cheap journey test tool ([`d3c99e1`](https://github.com/GitoxideLabs/gitoxide/commit/d3c99e1cf3125ab107e12718b39ac9b7c9a9165c))
</details>

## 0.10.0 (2022-12-28)

### New Features

 - <csr-id-15ecd841cfe7c77bbdfdfa232dd51a44c4940bbc/> allow execution of scripts without 'bash'.
   This works by trying to execute the file directly, and on failure, use 'bash'
   as interpreter.
   
   That way we are finally able to support a wider variety of fixture generators
   and make the crate more useful to a wieder audience.
 - <csr-id-221f1374aa004a76693cfb1529daab930a5a9dd7/> `spawn_git_daemon()` to spawn a git daemon hosting a working directoy…
   …with support for multiple at a time thanks to port selection (allowing
   tests to run in parallel) as well as auto-kill on drop.
 - <csr-id-67777a81f8d9d0335475e4fe4cbf770c328bd24f/> increase the waiting time on MacOS for file base locks
   It appears that these workers run into timeouts more and more,
   they got slower or maybe there are just more tests.
 - <csr-id-09da4c5eeff5c6657beb9c53c168f90e74d6f758/> add `Env::unset()` for convenience

### Bug Fixes

 - <csr-id-761b7d71977a5aa4876010faa61ab88f0dba6eab/> don't overwrite unexpanded `gix-lfs` pointer files.
   It's possible for those with incomplete `gix-lfs` installations
   (and many more situations) to end up in a spot where pointer files
   aren't expanded. If we overwrite the with archives, files look
   changed which can be confusing and lead to even bigger messes
   to happen.
   
   Now we don't overwrite those files anyomre.
 - <csr-id-1ce3190000f6211ce31468c7603d491bb5b90293/> Disable tag.gpgSign in test scripts
   This is done for the same reason that commit.gpgsign is disabled for test
   scripts. It prevents test failures if the user has tag.gpgsign enabled in
   their global git config when invoking tests.

### Changed (BREAKING)

 - <csr-id-dbf6c8c87cdca8169ac01aa89aefe56a33215142/> rename `scripted_fixture_*` to not contain 'repo' in the name.
   Further make clear in the documentation that `bash` is used to execute
   the fixture scripts, previously it wasn't even implied and got
   lost in history.

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

## 0.9.0 (2022-09-20)

### Bug Fixes

 - <csr-id-cba9edeb403aae4d77087de4167cbabe72525d92/> Allow multiple scripts to run at the same time, if they are not the same.
   Previously, per integration test and thus per crate, one would
   effectively only be able to run a single script at a time because of the
   global identity lock. This was required previously before the additional
   file based lock, per script name, was introduced.
   
   This is now fixed by dropping the lock after the script identity was
   obtained.

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

## 0.8.0 (2022-08-27)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>
<csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/>

### Chore

 - <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
 - <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### New Features

 - <csr-id-231785644194cd3be0b0dab06224a39ecf0ed714/> Provide `GIT_VERSION` information along with a way to skip a test based on it.
 - <csr-id-654b521323a5822cbb86e57bee159d90576fa5ff/> expose `on_ci` in the top-level.
 - <csr-id-449b6c1555fc2832c712ba51cd41ab9ed79e0b15/> Allow to re-execute scripts into temp directories.
   This is important in cases where the files created by the script
   contain absolute mentions of locations. That way, when copying
   files over, the test might accidentally return to the original
   read-only location, and write into it making future test runs fail.
 - <csr-id-f1635c3ee36678cff9f26135946c281bf4a75331/> publicly accessible `Result` type

### Bug Fixes

 - <csr-id-004dab17deab4c360adb5ac428f6b4951c974fe3/> `_with_args(…)` functions now allow non-static strings

## v0.6.0 (2022-04-04)

<csr-id-1d5ab44145ccbc2064ee8cc7acebb62db82c45aa/>

### Test

 - <csr-id-1d5ab44145ccbc2064ee8cc7acebb62db82c45aa/> ensure tests use 'merge.ff false' and recreate fixtures on each run

## v0.5.0 (2021-08-24)

## v0.4.0 (2021-08-11)

## v0.3.0 (2021-06-07)

## v0.1.0 (2021-04-30)

<csr-id-29bf8ca8399b6d4941aa242b9f08c74e59a179bb/>

### Other

 - <csr-id-29bf8ca8399b6d4941aa242b9f08c74e59a179bb/> try to disable GPG signing with environment variables…
   …but it's not picked up at all even though it's definitely present.

