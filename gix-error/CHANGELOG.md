# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.2 (2026-09-01)

### New Features

 - <csr-id-870c191a6d3e87c8806205e64b4446266c381844/> add ergonomic test errors and string comparisons
   <!-- agent -->
   Test functions only require Debug for their error type, so TestError can accept
   both standard errors and Exn without relying on a boxed error as the public
   result type. Direct, asymmetric comparisons with string slices and owned strings
   keep assertions concise while retaining Display semantics.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 8 calendar days.
 - 9 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2944 from GitoxideLabs/error-conversion-review ([`e3a6fa1`](https://github.com/GitoxideLabs/gitoxide/commit/e3a6fa1516481ec69ab00cddcca081ecdc52b4ca))
    - Add ergonomic test errors and string comparisons ([`870c191`](https://github.com/GitoxideLabs/gitoxide/commit/870c191a6d3e87c8806205e64b4446266c381844))
    - Merge pull request #2932 from GitoxideLabs/fundamental-types-comp ([`6704303`](https://github.com/GitoxideLabs/gitoxide/commit/6704303ed5ef3403b129e2b6cc4a9214432ffd03))
</details>

## 0.3.1 (2026-08-23)

### New Features

 - <csr-id-6b8edbba8ad382d218359baa6641256eee8c4809/> add BoxedResultExt for boxed errors

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
    - Release gix-error v0.3.1, gix-hash v0.26.2, gix-object v0.64.1, gix-ref v0.67.1, gix-packetline v0.22.1, gix-pack v0.74.1, gix-testtools v0.20.0 ([`e52fe9d`](https://github.com/GitoxideLabs/gitoxide/commit/e52fe9d03e82437a25bdfb1098e7046ec7e1b558))
    - Add BoxedResultExt for boxed errors ([`6b8edbb`](https://github.com/GitoxideLabs/gitoxide/commit/6b8edbba8ad382d218359baa6641256eee8c4809))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
</details>

## 0.3.0 (2026-08-22)

### New Features (BREAKING)

 - <csr-id-1af3e724c305862975bab35a6c9c263abb5d894b/> preserve and classify typed error sources
   <!-- agent -->
   Error classification and probable-cause selection need to inspect the complete
   error graph without turning native sources into strings or behaving differently
   across tree and auto-chain modes.
   
   Add CorruptionError, NotFoundError, and RetryableError together with Error
   classification helpers and support for boxed standard errors. Classification
   follows native source chains and nested gix errors while retaining concrete
   types for downcasting.
   
   Keep native sources owned by their original errors and traverse them lazily
   alongside explicit frames. Add breadth-first error iteration with optional
   captured locations, direct stored-error access, and downcasting across the
   complete graph.
   
   Preserve probable-cause identity and logical parent relationships while
   flattening exception trees into ChainedError. This lets auto-chain mode
   reconstruct the same traversal order without duplicating nested compatibility
   source chains.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 38 calendar days.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2930 from GitoxideLabs/gix-notes ([`7424676`](https://github.com/GitoxideLabs/gitoxide/commit/7424676f86cd3f5a67c53f8db6baf0803e937d4a))
    - Preserve and classify typed error sources ([`1af3e72`](https://github.com/GitoxideLabs/gitoxide/commit/1af3e724c305862975bab35a6c9c263abb5d894b))
    - Merge pull request #2714 from GitoxideLabs/fix-credentials-parsing ([`cf3053a`](https://github.com/GitoxideLabs/gitoxide/commit/cf3053a3c18e2de788cdaa9f41b5bd343bdc0091))
</details>

## 0.2.5 (2026-07-15)

### Bug Fixes

 - <csr-id-ccd5bd412eeaf3fa9537a021d647f818d4a27833/> keep erased errors visible to source-iteration and downcasting
   Since 499402c941, erasing an Exn wraps its error in the Untyped marker so that
   the typed accessors of Exn<Untyped> keep working. However, the marker also hid
   the original error from everything that walks the error afterwards: frames now
   yielded the marker, which cannot be downcast to the original type, and whose
   empty Error implementation cut off the source chain below it.
   
   This broke gix diff file, whose fallback for treating a revspec as a path on
   disk downcasts the sources() of a failed rev-parse to find the ref-not-found
   error - it would now fail with "couldn't parse revision" instead.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 50 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2694](https://github.com/GitoxideLabs/gitoxide/issues/2694)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2694](https://github.com/GitoxideLabs/gitoxide/issues/2694)**
    - Keep erased errors visible to source-iteration and downcasting ([`ccd5bd4`](https://github.com/GitoxideLabs/gitoxide/commit/ccd5bd412eeaf3fa9537a021d647f818d4a27833))
 * **Uncategorized**
    - Release gix-path v0.12.2, gix-error v0.2.5, gix-utils v0.3.4, gix-date v0.15.6, gix-url v0.36.2, gix-credentials v0.38.2 ([`27aec47`](https://github.com/GitoxideLabs/gitoxide/commit/27aec474c113cc885d44631b329454dc1ad0fed2))
    - Merge pull request #2702 from ameyypawar/fix/2694-exn-source-chain ([`e9c973d`](https://github.com/GitoxideLabs/gitoxide/commit/e9c973d9476bef293bec89cd683cb60b02a85e52))
    - Review ([`dc1fdc3`](https://github.com/GitoxideLabs/gitoxide/commit/dc1fdc3de8a9fb1266c2b5b01a1456bd177e7646))
    - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))
</details>

## 0.2.4 (2026-05-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 28 calendar days.
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
    - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))
</details>

## 0.2.3 (2026-04-28)

### Bug Fixes

 - <csr-id-76a03ebec19ec0a0d45d5ecf67ad49203df26adf/> improve error message around "Signature name or email must not contain..."
   This might make it easier to understand where the error is coming from.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 2 calendar days.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2491](https://github.com/GitoxideLabs/gitoxide/issues/2491)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2491](https://github.com/GitoxideLabs/gitoxide/issues/2491)**
    - Improve error message around "Signature name or email must not contain..." ([`76a03eb`](https://github.com/GitoxideLabs/gitoxide/commit/76a03ebec19ec0a0d45d5ecf67ad49203df26adf))
 * **Uncategorized**
    - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
    - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))
    - Merge pull request #2529 from GitoxideLabs/reflog-newline-handling ([`2c3a08e`](https://github.com/GitoxideLabs/gitoxide/commit/2c3a08e7d255df7d939af3d59c42aa0d6a21b76a))
</details>

## 0.2.2 (2026-04-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 32 calendar days.
 - 33 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
    - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
    - Make `package.include` patterns more specific so they don't match ignored files ([`c2c917f`](https://github.com/GitoxideLabs/gitoxide/commit/c2c917fce56c40a9af0d06bd603b7d1d2e51474f))
    - Merge pull request #2483 from GitoxideLabs/improvements ([`5f5a836`](https://github.com/GitoxideLabs/gitoxide/commit/5f5a836f666bf346050af21a75f22ecd649cc698))
    - Make `just nextest` work reliably ([`789b57f`](https://github.com/GitoxideLabs/gitoxide/commit/789b57f95eaedf9ff58bcd587d99940f22038f25))
    - Merge pull request #2480 from GitoxideLabs/report ([`98bae84`](https://github.com/GitoxideLabs/gitoxide/commit/98bae84fe534879899489c6f2c5e8cfcc863116d))
</details>

## 0.2.1 (2026-03-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 28 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.1, gix-date v0.15.1, gix-path v0.11.2, gix-features v0.46.2, gix-hash v0.23.0, gix-hashtable v0.13.0, gix-object v0.58.0, gix-packetline v0.21.2, gix-filter v0.28.0, gix-fs v0.19.2, gix-commitgraph v0.35.0, gix-revwalk v0.29.0, gix-traverse v0.55.0, gix-worktree-stream v0.30.0, gix-archive v0.30.0, gix-tempfile v21.0.2, gix-lock v21.0.2, gix-index v0.49.0, gix-pathspec v0.16.1, gix-ignore v0.19.1, gix-worktree v0.50.0, gix-diff v0.61.0, gix-blame v0.11.0, gix-ref v0.61.0, gix-sec v0.13.2, gix-config v0.54.0, gix-prompt v0.14.1, gix-credentials v0.37.1, gix-discover v0.49.0, gix-dir v0.23.0, gix-revision v0.43.0, gix-merge v0.14.0, gix-negotiate v0.29.0, gix-pack v0.68.0, gix-odb v0.78.0, gix-refspec v0.39.0, gix-shallow v0.10.0, gix-transport v0.55.1, gix-protocol v0.59.0, gix-status v0.28.0, gix-submodule v0.28.0, gix-worktree-state v0.28.0, gix v0.81.0, gix-fsck v0.19.0, gitoxide-core v0.55.0, gitoxide v0.52.0, safety bump 31 crates ([`c389a2c`](https://github.com/GitoxideLabs/gitoxide/commit/c389a2ccb32b36c1178a1352a2bb3229aef3b016))
    - Merge pull request #2454 from GitoxideLabs/dependabot/cargo/cargo-da044b9bb0 ([`6183fd0`](https://github.com/GitoxideLabs/gitoxide/commit/6183fd092d7acd43763fe15be400ce81e7172775))
    - Bump the cargo group with 68 updates ([`6bdb331`](https://github.com/GitoxideLabs/gitoxide/commit/6bdb33145e8aa81ba0dae5caafc675c591569715))
    - Merge pull request #2442 from GitoxideLabs/report ([`f7277f3`](https://github.com/GitoxideLabs/gitoxide/commit/f7277f3c9e3e5130edb714ff5bd3db06b7f589b3))
</details>

## 0.2.0 (2026-02-22)

### Documentation

 - <csr-id-fdf321b9b9c7ca1e762ed3b7ddbe149e55e2e4bb/> add `From<Message>` for `ValidationError` guide.
   This allows to more conveniently create validation errors.

### New Features (BREAKING)

 - <csr-id-502eaa0f750130bbd01112c8486be1f5e576a753/> `gix-error` instead of `thiserror` in `gix-quote`
   Replace the thiserror-derived `ansi_c::undo::Error` enum with
   `gix_error::Exn<gix_error::ValidationError>`, converting the `Error::new()`
   factory and variant constructors to `message!()` calls.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 10 calendar days.
 - 12 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.2.0, gix-date v0.15.0, gix-actor v0.40.0, gix-object v0.57.0, gix-quote v0.7.0, gix-attributes v0.31.0, gix-command v0.8.0, gix-filter v0.27.0, gix-chunk v0.7.0, gix-commitgraph v0.34.0, gix-revwalk v0.28.0, gix-traverse v0.54.0, gix-worktree-stream v0.29.0, gix-archive v0.29.0, gix-bitmap v0.3.0, gix-index v0.48.0, gix-pathspec v0.16.0, gix-worktree v0.49.0, gix-diff v0.60.0, gix-blame v0.10.0, gix-ref v0.60.0, gix-config v0.53.0, gix-prompt v0.14.0, gix-url v0.35.2, gix-credentials v0.37.0, gix-discover v0.48.0, gix-dir v0.22.0, gix-mailmap v0.32.0, gix-revision v0.42.0, gix-merge v0.13.0, gix-negotiate v0.28.0, gix-pack v0.67.0, gix-odb v0.77.0, gix-refspec v0.38.0, gix-shallow v0.9.0, gix-transport v0.55.0, gix-protocol v0.58.0, gix-status v0.27.0, gix-submodule v0.27.0, gix-worktree-state v0.27.0, gix v0.80.0, gix-fsck v0.18.0, gitoxide-core v0.54.0, gitoxide v0.51.0, safety bump 42 crates ([`ecf90fc`](https://github.com/GitoxideLabs/gitoxide/commit/ecf90fccb9d43bff320c17f46fdc3f5832533a52))
    - Merge pull request #2423 from GitoxideLabs/gix-error ([`000d58a`](https://github.com/GitoxideLabs/gitoxide/commit/000d58a9e3ec680b89186793bd8e09b9704835f5))
    - `gix-error` instead of `thiserror` in `gix-quote` ([`502eaa0`](https://github.com/GitoxideLabs/gitoxide/commit/502eaa0f750130bbd01112c8486be1f5e576a753))
    - Add `From<Message>` for `ValidationError` guide. ([`fdf321b`](https://github.com/GitoxideLabs/gitoxide/commit/fdf321b9b9c7ca1e762ed3b7ddbe149e55e2e4bb))
    - Merge branch 'release' ([`9327b73`](https://github.com/GitoxideLabs/gitoxide/commit/9327b73785227f1322a327cb48fbb0800e1286ae))
</details>

## 0.1.0 (2026-02-10)

### Documentation

 - <csr-id-9007e1b6b8b4b444c1159a2dc9a01242da6ee818/> improve documentation to be more vibe-friendly

### Bug Fixes (BREAKING)

 - <csr-id-b2c516a1689b62e61c9a517f726e5c782cd506b9/> turn `ParseError` into `ValidationError`
   The latter is more general and makes sense both for parsing,
   and for validation.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 19 calendar days.
 - 19 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.1.0, gix-date v0.14.0, gix-actor v0.39.0, gix-trace v0.1.18, gix-path v0.11.1, gix-features v0.46.1, gix-hash v0.22.1, gix-object v0.56.0, gix-quote v0.6.2, gix-attributes v0.30.1, gix-command v0.7.1, gix-packetline v0.21.1, gix-filter v0.26.0, gix-fs v0.19.1, gix-chunk v0.6.0, gix-commitgraph v0.33.0, gix-revwalk v0.27.0, gix-traverse v0.53.0, gix-worktree-stream v0.28.0, gix-archive v0.28.0, gix-bitmap v0.2.16, gix-tempfile v21.0.1, gix-lock v21.0.1, gix-index v0.47.0, gix-config-value v0.17.1, gix-pathspec v0.15.1, gix-worktree v0.48.0, gix-diff v0.59.0, gix-blame v0.9.0, gix-ref v0.59.0, gix-sec v0.13.1, gix-config v0.52.0, gix-prompt v0.13.1, gix-url v0.35.1, gix-credentials v0.36.0, gix-discover v0.47.0, gix-dir v0.21.0, gix-mailmap v0.31.0, gix-revision v0.41.0, gix-merge v0.12.0, gix-negotiate v0.27.0, gix-pack v0.66.0, gix-odb v0.76.0, gix-refspec v0.37.0, gix-shallow v0.8.1, gix-transport v0.54.0, gix-protocol v0.57.0, gix-status v0.26.0, gix-submodule v0.26.0, gix-worktree-state v0.26.0, gix v0.79.0, safety bump 35 crates ([`d66ac10`](https://github.com/GitoxideLabs/gitoxide/commit/d66ac1057a5b7bfb608d4e6be585c69fb692bfee))
    - Merge pull request #2400 from GitoxideLabs/gix-error ([`e4f016b`](https://github.com/GitoxideLabs/gitoxide/commit/e4f016bd386deae6466bf703ba0b7959e6460ac8))
    - Refactor2 ([`f860c0b`](https://github.com/GitoxideLabs/gitoxide/commit/f860c0b5f5fe316464baaf6e6487e8cb394b78e8))
    - Address Copilot review ([`0b0e9f8`](https://github.com/GitoxideLabs/gitoxide/commit/0b0e9f8df95e60626cfec2f8665af072b4ddc77c))
    - Improve documentation to be more vibe-friendly ([`9007e1b`](https://github.com/GitoxideLabs/gitoxide/commit/9007e1b6b8b4b444c1159a2dc9a01242da6ee818))
    - Merge pull request #2407 from GitoxideLabs/dependabot/cargo/cargo-fb4135702f ([`8bceefb`](https://github.com/GitoxideLabs/gitoxide/commit/8bceefbfc5f897517bfdd24744695a82cfa0d5be))
    - Bump the cargo group with 59 updates ([`7ce3c55`](https://github.com/GitoxideLabs/gitoxide/commit/7ce3c5587aec1ca813039c047783b9cb2a106826))
    - Merge pull request #2396 from GitoxideLabs/gix-error ([`e8612b5`](https://github.com/GitoxideLabs/gitoxide/commit/e8612b5bd16eb19a04ddf7e37d94bef013127f88))
    - Turn `ParseError` into `ValidationError` ([`b2c516a`](https://github.com/GitoxideLabs/gitoxide/commit/b2c516a1689b62e61c9a517f726e5c782cd506b9))
    - Merge pull request #2393 from GitoxideLabs/report ([`f7d0975`](https://github.com/GitoxideLabs/gitoxide/commit/f7d09758d245aaa89409e39bb6ba1ed6b7118ea5))
</details>

## 0.0.0 (2026-01-22)

### New Features

 - <csr-id-461c87667c75a9db0a74c43ef68d71b88a7dd754/> Add an `auto-chain-error` feature to let `gix-error::Error` produce error chains suitable for `anyhow`.
 - <csr-id-28f4211afadd91c5b5d2d2a0698f37e660cc0c66/> make it possible to produce errors that work well with `anyhow` source-chain display.
 - <csr-id-053c3ee2217480eead3aa7c71fa4b65455444921/> anyhow support for `gix-error::Exn`
   This is mainly useful for `gitoxide-core`, which may call plumbing.
 - <csr-id-3301eb8b2906861952726061629d74babfd24f73/> Add `Exn::downcast_any_ref()`

### New Features (BREAKING)

 - <csr-id-5ab19a7a3344c58ad1185a23a789848ed5e02241/> Use `gix-error` in `gix-date`
   This will make for easier introspection for users of these errors.

### Refactor (BREAKING)

 - <csr-id-829393ac596bf2684bd8a837ae931773b24ee033/> ErrorExt::raise_iter to raise_all + remove Frame::downcast
   Be more compatible to `exn`.
 - <csr-id-f8517bedcbb9b3328f435aa37f4c63bd30b19fc0/> catch up Exn designs with the upstream
   refactor!: rename `Exn::from_iter` to `raise_all`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 12 calendar days.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#2384](https://github.com/GitoxideLabs/gitoxide/issues/2384), [#2385](https://github.com/GitoxideLabs/gitoxide/issues/2385)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2384](https://github.com/GitoxideLabs/gitoxide/issues/2384)**
    - Catch up Exn designs with the upstream ([`f8517be`](https://github.com/GitoxideLabs/gitoxide/commit/f8517bedcbb9b3328f435aa37f4c63bd30b19fc0))
 * **[#2385](https://github.com/GitoxideLabs/gitoxide/issues/2385)**
    - ErrorExt::raise_iter to raise_all + remove Frame::downcast ([`829393a`](https://github.com/GitoxideLabs/gitoxide/commit/829393ac596bf2684bd8a837ae931773b24ee033))
 * **Uncategorized**
    - Fixes to make a release work. ([`fa302a1`](https://github.com/GitoxideLabs/gitoxide/commit/fa302a115918289ca2c4b33f5aa576f478e46092))
    - Merge pull request #2383 from GitoxideLabs/gix-error ([`9d39656`](https://github.com/GitoxideLabs/gitoxide/commit/9d39656710c297f9a22e4a7e6facc3a1f35f89e0))
    - Address Copilot review ([`16327ef`](https://github.com/GitoxideLabs/gitoxide/commit/16327efe24e2321e2a4efe5321e9f0483484b10a))
    - Add an `auto-chain-error` feature to let `gix-error::Error` produce error chains suitable for `anyhow`. ([`461c876`](https://github.com/GitoxideLabs/gitoxide/commit/461c87667c75a9db0a74c43ef68d71b88a7dd754))
    - Make it possible to produce errors that work well with `anyhow` source-chain display. ([`28f4211`](https://github.com/GitoxideLabs/gitoxide/commit/28f4211afadd91c5b5d2d2a0698f37e660cc0c66))
    - Anyhow support for `gix-error::Exn` ([`053c3ee`](https://github.com/GitoxideLabs/gitoxide/commit/053c3ee2217480eead3aa7c71fa4b65455444921))
    - Merge pull request #2378 from GitoxideLabs/gix-error ([`6cff657`](https://github.com/GitoxideLabs/gitoxide/commit/6cff65786b5213194fffd2c77b7c2dc44dcb4b52))
    - Address Copilot review ([`e112cac`](https://github.com/GitoxideLabs/gitoxide/commit/e112cacc42a192d5159b299e49739f3af2589e3e))
    - Change `ErrorExt::erased()` to `ErrorExt::raise_erased()`. ([`373fced`](https://github.com/GitoxideLabs/gitoxide/commit/373fceddcc1a0ef79f306b519a2ca3682b3110ef))
    - Make `Exn` work properly after the type was erased. ([`499402c`](https://github.com/GitoxideLabs/gitoxide/commit/499402c941e85e6cff5c3ffef8a09afac842c7ac))
    - Add `Exn::downcast_any_ref()` ([`3301eb8`](https://github.com/GitoxideLabs/gitoxide/commit/3301eb8b2906861952726061629d74babfd24f73))
    - Merge pull request #2374 from GitoxideLabs/gix-error ([`25233ce`](https://github.com/GitoxideLabs/gitoxide/commit/25233ced7f17e14842aa400cf007a0feb6127d89))
    - Turn `Exn::into_box()` to `Exn::into_inner()`. ([`939b8fc`](https://github.com/GitoxideLabs/gitoxide/commit/939b8fcbb2115eba77aca1be8527ad0d7f644c56))
    - Merge pull request #2373 from GitoxideLabs/gix-error ([`4c6a7a7`](https://github.com/GitoxideLabs/gitoxide/commit/4c6a7a76c214c94910f141542d677dc2a7500ddd))
    - Adapt to changes in `gix-chunk` ([`e6e90ff`](https://github.com/GitoxideLabs/gitoxide/commit/e6e90ff82b1f839a6d78170685f2a69566766675))
    - Add conversion from Message to `ParseError` for less noisy invocations. ([`08f9ed4`](https://github.com/GitoxideLabs/gitoxide/commit/08f9ed48c896ea92d8d8da9b15bc44f7709b013e))
    - More docs to better explain `gix-error` ([`f46ca99`](https://github.com/GitoxideLabs/gitoxide/commit/f46ca9925930e4b2a660d2896ccbfb8edd3aa4e9))
    - Merge pull request #2352 from GitoxideLabs/gix-error ([`ae23762`](https://github.com/GitoxideLabs/gitoxide/commit/ae23762932ea0d78e91463185a304d778746a167))
    - Make it possible to traverse frames using an iterator. ([`3bac149`](https://github.com/GitoxideLabs/gitoxide/commit/3bac149385a6f64ab0ee1989ad562132574dc021))
    - Actually introduce `gix-error` into `gix-revision`. ([`4819ea8`](https://github.com/GitoxideLabs/gitoxide/commit/4819ea8d81645b8b79dc2a3fcba7b27d773a9fce))
    - Adadpt `exn` to most pressing needs of `gitoxide` ([`abedade`](https://github.com/GitoxideLabs/gitoxide/commit/abedadec5463b57e78aa53e62d8c511b989ae9ca))
    - Vendor `exn` from https://github.com/fast/exn@bb4d8ea4e4df335c46d4fa3f4f260121f9f84305 ([`0eaab70`](https://github.com/GitoxideLabs/gitoxide/commit/0eaab70ee6e897635d7fb41402ec87387b8ecd4b))
    - Use `gix-error` in `gix-date` ([`5ab19a7`](https://github.com/GitoxideLabs/gitoxide/commit/5ab19a7a3344c58ad1185a23a789848ed5e02241))
    - Create a basic `gix-error` crate to forward `exn` ([`35cf1ff`](https://github.com/GitoxideLabs/gitoxide/commit/35cf1ff837ea30a1366b20bde0d59baf9ab699be))
</details>

