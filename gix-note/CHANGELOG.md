# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 12 calendar days.
 - 13 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
    - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
    - Merge pull request #3022 from GitoxideLabs/release-testtools ([`f819565`](https://github.com/GitoxideLabs/gitoxide/commit/f819565c2c4c56619c4888acef6cf3b8144cbccb))
    - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
</details>

## 0.2.0 (2026-09-25)

### New Features

 - <csr-id-dfa5173be89d9d78a9a6691c9cc36b6fdbd9173e/> batch staged note edits into a single tree write
   <!-- agent -->
   Creating many notes with repeated `State::replace()` calls serializes all
   materialized mappings after every edit. Add `State::edit()` to update
   retained state and `State::write()` to serialize once. Lookups observe
   pending edits, while immediate replacement and removal continue to flush.
   Failed writes recover the last saved root.
   
   Regression coverage checks zero writes during staging, cached reads,
   persisted replacements and removals, non-note preservation, no-op flushes,
   and recovery after failed writes.

### Bug Fixes

 - <csr-id-190593fb1127f48eb12019580608653690f59467/> edit notes trees lazily
   <!-- agent -->
   `replace()` and `remove()` loaded every fanout subtree into a `HashMap`, then
   reconstructed every logical note path. This made one edit O(total notes).
   
   Represent unopened fanout directories as radix-tree leaves carrying their
   existing tree IDs. Materialize only the matching edit path and subtrees that the
   fanout heuristic must collapse. Preserve opaque subtrees whenever the current
   fanout allows it, matching Git's trade-off: fanout after an edit depends on what
   the lazy trie has materialized instead of a global recount.
   
   Retain each opened subtree's original path components so non-note entries keep
   their spelling, including uppercase fanout-like directories. A regression with
   32 root fanout entries proves replacement reads and writes only the root and
   target subtree, while the Git-produced boundary fixture verifies exact tree IDs.
   The now-unused `gix-hashtable` dependency is removed.
   
   Add an in-memory Criterion benchmark for `get()` and `replace()` on 65,536-note
   trees. It covers worst-case one-to-two-level fanout expansion and common
   steady-state replacement, excludes fixture construction and filesystem I/O, and
   reports notes per second for GET and PUT. Run it with `cargo bench -p gix-note
   --bench read-write`.
   
   Git baseline: git.git 1630431f326e15fcde608827b5ff38422528eb59,
   especially `note_tree_search()`, `load_subtree()`, `determine_fanout()`,
   `for_each_note_helper()`, and `write_notes_tree()` in `notes.c`.

### Changed (BREAKING)

 - <csr-id-4b42e0ce80ae934cae4f102f44c392581758608f/> raise MSRV to Rust 1.88
   <!-- agent -->
   The newly published `dua-core` 3.3 release used by linked-worktree removal
   requires Rust 1.88, so raise every workspace crate and the advertised badge
   together.
   
   Keep the MSRV checks buildable by selecting the latest `sysinfo` and `rusqlite`
   release lines that support Rust 1.88.

### New Features (BREAKING)

 - <csr-id-0418bf8c0046ce767b717fe66504fa524a880db5/> require caller-owned notes-tree state
   Git keeps `struct notes_tree` alive across lookups and edits, preserving radix
   nodes and lazily materialized subtree entries. Recreating that structure for
   every operation repeats tree parsing and entry allocation.
   
   Add caller-owned `State` and require it in `get()`, `replace()`, and `remove()`.
   The state advances after each successful edit. `gix::note::Platform` retains
   states by root-tree ID so references and aliases sharing a tree also share
   parsed entries, and clears cached states after plumbing errors.
   
   A counting in-memory ODB regression proves repeated operations do not reread
   materialized trees. Measurements on 65,536-note fixtures were:
   
   | Scenario | New state | Reused state |
   | --- | ---: | ---: |
   | Common two-level GET | ~16,700 GET/s | ~111,000,000 GET/s |
   | Common two-level PUT | ~5,470 PUT/s | ~8,000 PUT/s |
   | One-level fanout GET | ~20,100 GET/s | ~108,000,000 GET/s |
   | One-level fanout PUT | ~3,310 PUT/s | ~3,980 PUT/s |
   
   The reused one-level PUT is primed by one untimed expanding write and therefore
   measures blob-only replacements after expansion.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 31 calendar days.
 - 32 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
    - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
    - Use borrowed error inspection throughout the workspace ([`daf73b5`](https://github.com/GitoxideLabs/gitoxide/commit/daf73b5fe5a21e3ddcc58f0882c2d880f48b7860))
    - Merge pull request #3004 from GitoxideLabs/gix-notes-example ([`a5e8c4d`](https://github.com/GitoxideLabs/gitoxide/commit/a5e8c4d62b8b84cc6228e1da6b94fdccad58fdb1))
    - Batch staged note edits into a single tree write ([`dfa5173`](https://github.com/GitoxideLabs/gitoxide/commit/dfa5173be89d9d78a9a6691c9cc36b6fdbd9173e))
    - Merge pull request #2963 from GitoxideLabs/gix-notes-perf ([`4a870be`](https://github.com/GitoxideLabs/gitoxide/commit/4a870be7db38a3fde68db3774fa7d7f2000c3d68))
    - Require caller-owned notes-tree state ([`0418bf8`](https://github.com/GitoxideLabs/gitoxide/commit/0418bf8c0046ce767b717fe66504fa524a880db5))
    - Edit notes trees lazily ([`190593f`](https://github.com/GitoxideLabs/gitoxide/commit/190593fb1127f48eb12019580608653690f59467))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
    - Merge pull request #2955 from GitoxideLabs/transport-url-encoding ([`7e35849`](https://github.com/GitoxideLabs/gitoxide/commit/7e35849b36646cff9722f6906a4527d64818a374))
    - Release gix-path v0.12.6, gix-error v0.3.2, gix-command v0.10.1, gix-transport v0.59.2 ([`888677a`](https://github.com/GitoxideLabs/gitoxide/commit/888677ad2d63a2e3930a02add2de0b4b667a5581))
    - Merge pull request #2940 from GitoxideLabs/vendor-bisync ([`dda600d`](https://github.com/GitoxideLabs/gitoxide/commit/dda600d7ee29a6bda4cf1047d0d1782e76b16f98))
</details>

## 0.1.1 (2026-08-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-packetline v0.22.2, gix-worktree-stream v0.36.1, gix-archive v0.36.1, gix-diff v0.67.1, gix-blame v0.17.1, gix-dir v0.29.1, gix-mailmap v0.34.1, gix-revision v0.49.1, gix-merge v0.20.1, gix-negotiate v0.35.1, gix-note v0.1.1, gix-pack v0.74.2, gix-macros v0.1.6, gix-refspec v0.45.1, gix-transport v0.59.1, gix-protocol v0.65.1, gix-status v0.34.1, gix-worktree-state v0.34.1, gix v0.87.1, gix-fsck v0.25.1, gitoxide-core v0.61.1, gix-tix v0.3.0, gitoxide v0.58.0, safety bump gitoxide v0.58.0 ([`3ebca8b`](https://github.com/GitoxideLabs/gitoxide/commit/3ebca8b66017ab2dd02a38f75f78f485bee1ded8))
    - Merge pull request #2932 from GitoxideLabs/fundamental-types-comp ([`6704303`](https://github.com/GitoxideLabs/gitoxide/commit/6704303ed5ef3403b129e2b6cc4a9214432ffd03))
    - Release gix-error v0.3.1, gix-hash v0.26.2, gix-object v0.64.1, gix-ref v0.67.1, gix-packetline v0.22.1, gix-pack v0.74.1, gix-testtools v0.20.0 ([`e52fe9d`](https://github.com/GitoxideLabs/gitoxide/commit/e52fe9d03e82437a25bdfb1098e7046ec7e1b558))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
</details>

## 0.1.0 (2026-08-22)

### Chore

 - <csr-id-3e05ca352597ef5966fa4dc4f52456c2424cddad/> add package.include directives to control which files are packaged.
 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.
 - <csr-id-3bd09ef120945a9669321ea856db4079a5dab930/> change `rust-version` manifest field back to 1.65.
   They didn't actually need to be higher to work, and changing them
   unecessarily can break downstream CI.
   
   Let's keep this value as low as possible, and only increase it when
   more recent features are actually used.
 - <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
   Our MSRV follows the one of `helix`, which in turn follows Firefox.

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

 - <csr-id-d21b52afaaae0cb5312aa3d474f476166546b760/> support note mutation in gix-note
   <!-- agent -->
   Add plumbing operations to insert, replace, and remove note mappings while
   returning the rewritten root tree and the previous note id.
   
   Rebuild notes using the same dynamic fanout rule as Git so growing and shrinking
   collections remain compatible with existing notes trees. Preserve unrelated
   entries during rewriting, reject mixed object-hash kinds, report duplicate
   mappings, and leave the tree unchanged when removing a missing note.
   
   Keep object persistence delegated through Find and Write traits, allowing
   callers to decide how commits and refs are updated.
 - <csr-id-048bfd3058cb122c5155f59136b24d522681d62b/> support lazy note lookup in gix-note
   <!-- agent -->
   Establish the minimal plumbing API needed to query notes efficiently for many
   objects, such as every commit visible in a history view.
   
   Resolve note blobs through Git notes progressive two-hex-digit fanout trees
   and use Git tree ordering for direct entry lookup. Load only trees along the
   requested object path and retain decoded trees in a caller-owned cache that can
   be reused across lookups and notes roots.
   
   Ignore entries that do not form valid notes instead of mistaking arbitrary tree
   contents for mappings. Use gix-error for contextual failures.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 23 commits contributed to the release.
 - 1101 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Make gix-note compile on docs.rs ([`94aa28e`](https://github.com/GitoxideLabs/gitoxide/commit/94aa28ee57d73e32a43edc9ebf6dbb8dd460ed45))
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2930 from GitoxideLabs/gix-notes ([`7424676`](https://github.com/GitoxideLabs/gitoxide/commit/7424676f86cd3f5a67c53f8db6baf0803e937d4a))
    - Support note mutation in gix-note ([`d21b52a`](https://github.com/GitoxideLabs/gitoxide/commit/d21b52afaaae0cb5312aa3d474f476166546b760))
    - Support lazy note lookup in gix-note ([`048bfd3`](https://github.com/GitoxideLabs/gitoxide/commit/048bfd3058cb122c5155f59136b24d522681d62b))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Remove rust_2018_idioms lint declarations ([`e10d5f6`](https://github.com/GitoxideLabs/gitoxide/commit/e10d5f662df2ee05f973a3167ad215a330ee74e1))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
    - Add package.include directives to control which files are packaged. ([`3e05ca3`](https://github.com/GitoxideLabs/gitoxide/commit/3e05ca352597ef5966fa4dc4f52456c2424cddad))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge branch 'msrv' ([`8c492d7`](https://github.com/GitoxideLabs/gitoxide/commit/8c492d7b7e6e5d520b1e3ffeb489eeb88266aa75))
    - Change `rust-version` manifest field back to 1.65. ([`3bd09ef`](https://github.com/GitoxideLabs/gitoxide/commit/3bd09ef120945a9669321ea856db4079a5dab930))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
</details>

## 0.0.0 (2023-08-17)

An empty crate without any content to reserve the name for the gitoxide project.

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

### Chore

 - <csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/> don't call crate 'WIP' in manifest anymore.
 - <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
 - <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#691](https://github.com/GitoxideLabs/gitoxide/issues/691)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **Uncategorized**
    - Release gix-tix v0.0.0, gix-note v0.0.0, gix-lfs v0.0.0, gix-fetchhead v0.0.0, gix-sequencer v0.0.0, gix-rebase v0.0.0 ([`0199927`](https://github.com/GitoxideLabs/gitoxide/commit/019992765d3cfc2627cf57e82771c006726c8fbc))
    - Don't call crate 'WIP' in manifest anymore. ([`229bd48`](https://github.com/GitoxideLabs/gitoxide/commit/229bd4899213f749a7cc124aa2b82a1368fba40f))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Adjust to renaming of `git-note` to `gix-note` ([`9cc9ae2`](https://github.com/GitoxideLabs/gitoxide/commit/9cc9ae28102bca9c5eac4c7699be7029e7f7697f))
    - Rename `git-note` to `gix-note` ([`794b485`](https://github.com/GitoxideLabs/gitoxide/commit/794b485e40728aba3a364455210a2035e7ba9fbd))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
    - Release git-note v0.0.0 ([`2dcb3f3`](https://github.com/GitoxideLabs/gitoxide/commit/2dcb3f3e030bc87914c7c58ce2916d81649aa0d9))
    - Empty crate for git-note ([`2fb8b46`](https://github.com/GitoxideLabs/gitoxide/commit/2fb8b46abd3905bdf654977e77ec2f36f09e754f))
</details>

