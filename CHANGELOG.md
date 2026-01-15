# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.49.0 (2025-12-31)

## 0.48.0 (2025-12-22)

## 0.47.0 (2025-11-22)

<csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/>

### New Features

 - <csr-id-924cd62f7ef74f479114c80cee0db0ba5f522579/> allow credential fill with `gix credential fill` to run without a repo

### Other

 - <csr-id-6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b/> Remove `doc_auto_cfg` feature to fix docs.rs documentation.
   It is part of `doc_cfg` feature since https://github.com/rust-lang/rust/pull/138907
   
   This fixes the docs.rs build

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2198](https://github.com/GitoxideLabs/gitoxide/issues/2198)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2198](https://github.com/GitoxideLabs/gitoxide/issues/2198)**
    - Allow credential fill with `gix credential fill` to run without a repo ([`924cd62`](https://github.com/GitoxideLabs/gitoxide/commit/924cd62f7ef74f479114c80cee0db0ba5f522579))
 * **Uncategorized**
    - Merge pull request #2254 from ralphmodales/credential-fill-fix ([`dba337d`](https://github.com/GitoxideLabs/gitoxide/commit/dba337da2430190b2151a2814fcccc09fd9f1d50))
    - Merge pull request #2204 from cruessler/improve-blame-ranges ([`663b41e`](https://github.com/GitoxideLabs/gitoxide/commit/663b41eb65f0fffbc8397e91ce5107382b08b441))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
    - Remove `doc_auto_cfg` feature to fix docs.rs documentation. ([`6f469a6`](https://github.com/GitoxideLabs/gitoxide/commit/6f469a6fea59c88e6c69a5f94b0bc8a5977cb75b))
</details>

## 0.46.0 (2025-10-22)

### New Features

 - <csr-id-aacc326a198e4c78dd546018eebaff192ff5223d/> add a `gix commit sign` prototype
 - <csr-id-58b5060958e0090773bb15ffb3679d99704b70a0/> add first debug version of `gix branch list`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 60 calendar days.
 - 99 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2208 from cruessler/add-commit-signing ([`8699d0f`](https://github.com/GitoxideLabs/gitoxide/commit/8699d0f010458155f7f5e19c8018ea5e58079963))
    - Refactor ([`82f788e`](https://github.com/GitoxideLabs/gitoxide/commit/82f788e17aa3a4ad4bf21cc4293ec4f621f227ce))
    - Add a `gix commit sign` prototype ([`aacc326`](https://github.com/GitoxideLabs/gitoxide/commit/aacc326a198e4c78dd546018eebaff192ff5223d))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Adapt to change in `gix-blame` ([`a721e74`](https://github.com/GitoxideLabs/gitoxide/commit/a721e7428ba4fd145e14c4576d49b9ff5a1a9816))
    - Adapt to change in `gix-blame` ([`268dac4`](https://github.com/GitoxideLabs/gitoxide/commit/268dac4a18c8be8bacd89e1890d870253b0e1400))
    - Merge pull request #2142 from cruessler/add-branch-list ([`525873f`](https://github.com/GitoxideLabs/gitoxide/commit/525873fafacf339dd781df0d812df02942c220fd))
    - Fix copy-paste doc comment ([`04650a7`](https://github.com/GitoxideLabs/gitoxide/commit/04650a7786f5fb0ea4f4454d08ca16ab6cef3b74))
    - Move --all to 'branch list' ([`6651548`](https://github.com/GitoxideLabs/gitoxide/commit/6651548f6e426d79f834c5948e3d7877784dfc77))
    - Refactor ([`e537611`](https://github.com/GitoxideLabs/gitoxide/commit/e53761181b05f90f59b33906da033096b57a50d8))
    - Add first debug version of `gix branch list` ([`58b5060`](https://github.com/GitoxideLabs/gitoxide/commit/58b5060958e0090773bb15ffb3679d99704b70a0))
    - Merge pull request #2106 from cruessler/add-open-with-environment-overrides ([`c149116`](https://github.com/GitoxideLabs/gitoxide/commit/c1491167c2ea8f8a34bb2e30f9e632f790f0fcd8))
    - Fail `gix tag list` when JSON is requested ([`fb2766b`](https://github.com/GitoxideLabs/gitoxide/commit/fb2766bd006ba1a276dfd5554cf0cdc3b7043ee7))
</details>

## 0.45.0 (2025-07-15)

### New Features

 - <csr-id-37d3bf24ac1a79302f3e97b97372e4ad381c45e2/> add first debug version of `gix tag list`
 - <csr-id-ab52a49a555ab25e6cf632cb0b080eab72958a7d/> `gix revision list --long-hashes` for faster iteration.
   The performance of the short-hash generation was improved as well.
 - <csr-id-c5bc49f2a02e9b28c2466ea4c7ae711d091ffc96/> support for `commitgraph list from..to` to exercise the new 'hide' capability.
 - <csr-id-1df1ebb34dd3e2101d8a112dda66f6bac5261ea7/> Enable precious file parsing in `gix` CLI by default, allow overrides.
   That's pretty neat as one can now set `GIX_PARSE_PRECIOUS=0` in the environment
   to disable precious file parsing, good to see what difference it makes.
   
   It's also possible to do this wiht `gix -c gitoxide.parsePrecious=0`.
 - <csr-id-36a6ffeea7bbde7fb3689ddf2a107e09a50e602c/> add support for multiple blame ranges like `gix blame -L <range> -L <other-range> ...`
   Update the blame subcommand to handle multiple line ranges. This allows specifying multiple `-L` options similar to the usage of git.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 78 calendar days.
 - 79 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2073 from cruessler/add-tag-list ([`c7af04d`](https://github.com/GitoxideLabs/gitoxide/commit/c7af04db9b6bb1204e0f4c436d1db8f48a491e86))
    - Refactor ([`750ae9b`](https://github.com/GitoxideLabs/gitoxide/commit/750ae9bc3cf72c1d9a358307e423523324eb25fb))
    - Add first debug version of `gix tag list` ([`37d3bf2`](https://github.com/GitoxideLabs/gitoxide/commit/37d3bf24ac1a79302f3e97b97372e4ad381c45e2))
    - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
    - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
    - Merge pull request #2041 from cruessler/add-blame-extraction ([`dd5f0a4`](https://github.com/GitoxideLabs/gitoxide/commit/dd5f0a4811bc738051f7af164b8d2815aaa23220))
    - Merge pull request #2051 from GitoxideLabs/improvements ([`f933f80`](https://github.com/GitoxideLabs/gitoxide/commit/f933f8065c218ee1e9ae7158b15c8a0917140803))
    - `gix revision list --long-hashes` for faster iteration. ([`ab52a49`](https://github.com/GitoxideLabs/gitoxide/commit/ab52a49a555ab25e6cf632cb0b080eab72958a7d))
    - Adapt to changes in `gix-blame` ([`4afc51d`](https://github.com/GitoxideLabs/gitoxide/commit/4afc51d4ba669ad3c4b26f1c4222442d1dab1695))
    - Merge pull request #2022 from cruessler/add-rename-tracking-to-blame ([`76eddf8`](https://github.com/GitoxideLabs/gitoxide/commit/76eddf86b91afc3535f7eb0d9004652823ccda36))
    - Refactor ([`3e5365c`](https://github.com/GitoxideLabs/gitoxide/commit/3e5365cb066895c787a22422964a2b9459f37ec3))
    - Merge pull request #2037 from GitoxideLabs/hide ([`92febae`](https://github.com/GitoxideLabs/gitoxide/commit/92febae025165c55e596d58511b1634fb6580b9c))
    - Support for `commitgraph list from..to` to exercise the new 'hide' capability. ([`c5bc49f`](https://github.com/GitoxideLabs/gitoxide/commit/c5bc49f2a02e9b28c2466ea4c7ae711d091ffc96))
    - Merge pull request #2019 from GitoxideLabs/precious-opt-in ([`5f9de52`](https://github.com/GitoxideLabs/gitoxide/commit/5f9de52cf286163b503047b1ab3b51dfa093b4d4))
    - Enable precious file parsing in `gix` CLI by default, allow overrides. ([`1df1ebb`](https://github.com/GitoxideLabs/gitoxide/commit/1df1ebb34dd3e2101d8a112dda66f6bac5261ea7))
    - Merge pull request #1973 from holodorum/feature/blame-range-support ([`de13b16`](https://github.com/GitoxideLabs/gitoxide/commit/de13b16728f6d29452cb97b50281aa91d498eb49))
    - Refactor ([`d4461e7`](https://github.com/GitoxideLabs/gitoxide/commit/d4461e700657d049a8cbc1552f328e35b27c92c3))
    - Add support for multiple blame ranges like `gix blame -L <range> -L <other-range> ...` ([`36a6ffe`](https://github.com/GitoxideLabs/gitoxide/commit/36a6ffeea7bbde7fb3689ddf2a107e09a50e602c))
    - Adapt to changes in `gix-blame` ([`8143d69`](https://github.com/GitoxideLabs/gitoxide/commit/8143d692e51c8d1b3d8c2323e83676e1be122f13))
</details>

## 0.44.0 (2025-04-26)

## 0.43.0 (2025-04-25)

<csr-id-f952c101cc8686e685074e8604e0f332c06d6767/>

### Other

 - <csr-id-f952c101cc8686e685074e8604e0f332c06d6767/> Add missing command docs + small style fixups
   Changes that affect the help/usage text shown at runtime:
   
   - Add descriptions for a few `gix` commands that didn't have it:
     `gix is-clean`, `gix is-changed`, and `gix env`.
   
   - Add a description of the debug-only `ein panic` command.
   
   - Small spacing and capitalization improvements for consistency.
   
   - Add a line break in a `///` block for brevity of the top line.
   
   Changes to code style that do not affect what is shown at runtime:
   
   - Add `.` at the end of some `///` first lines. (This doesn't
     affect runtime behavior because `clap` normalizes this away.)
   
   - Put `///` above all `#[...]` in a few places where it wasn't.
   
   - Adjust comment wrapping in a couple of places for clarity.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Merge pull request #1937 from EliahKagan/document-commands ([`de2f97d`](https://github.com/GitoxideLabs/gitoxide/commit/de2f97d793d8675659020883a04f47fc9ff06471))
    - Add missing command docs + small style fixups ([`f952c10`](https://github.com/GitoxideLabs/gitoxide/commit/f952c101cc8686e685074e8604e0f332c06d6767))
</details>

## 0.42.0 (2025-04-04)

<csr-id-dd366a52d46078729a6e36468211b97c92c2921d/>

### New Features

 - <csr-id-dbb15328d7b26aaa51ceadcd5a8cb021155023b3/> add first 'debug' version of `gix diff file`

### Refactor

 - <csr-id-dd366a52d46078729a6e36468211b97c92c2921d/> use revspecs for revision and path

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #1880 from cruessler/add-gix-diff-file ([`b10cc6f`](https://github.com/GitoxideLabs/gitoxide/commit/b10cc6f0f2c3757677d9fdb3c01196d371ac6631))
    - Apply suggestions from code review ([`1039ae9`](https://github.com/GitoxideLabs/gitoxide/commit/1039ae95981a21340c77739891baa742b801a604))
    - Use revspecs for revision and path ([`dd366a5`](https://github.com/GitoxideLabs/gitoxide/commit/dd366a52d46078729a6e36468211b97c92c2921d))
    - Add first 'debug' version of `gix diff file` ([`dbb1532`](https://github.com/GitoxideLabs/gitoxide/commit/dbb15328d7b26aaa51ceadcd5a8cb021155023b3))
    - Merge pull request #1888 from cruessler/respect-diff-algorithm-in-blame ([`dce127e`](https://github.com/GitoxideLabs/gitoxide/commit/dce127e63f7788c5424e2da2cf4e3112f9c3b159))
    - Adapt to changes in `gix-blame` ([`8a31d88`](https://github.com/GitoxideLabs/gitoxide/commit/8a31d88dab3f2a79d4a0343f62d63a8d074ed94d))
    - Merge pull request #1859 from cruessler/add-failing-test-for-as-time ([`36a846f`](https://github.com/GitoxideLabs/gitoxide/commit/36a846f23ae0a3dfe95648605f0f618ccb55a881))
    - Merge pull request #1858 from cruessler/add-git-blame-since ([`7059609`](https://github.com/GitoxideLabs/gitoxide/commit/70596096e35ff8a910dacd6fefdc31d162282b81))
    - Add test for `--since` ([`330711d`](https://github.com/GitoxideLabs/gitoxide/commit/330711d69b343d3f4d59b3c9d9b87f75408fdc25))
    - Adapt to changes in `gix-blame` ([`e7084d8`](https://github.com/GitoxideLabs/gitoxide/commit/e7084d8db829bbc33aea958a997deb1db4b5a453))
    - Merge pull request #1861 from GitoxideLabs/revert-lazylock ([`6acca19`](https://github.com/GitoxideLabs/gitoxide/commit/6acca19163196f8402603cb031cb1d37f7178a43))
    - Don't use LazyLock just yet. ([`147eb41`](https://github.com/GitoxideLabs/gitoxide/commit/147eb4196acea76110b4adb26a295089e2365df5))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
</details>

## 0.41.0 (2025-01-18)

### New Features

 - <csr-id-4a783959c4862985cbffc4fe5cd2c1bed38383b1/> add `gix blame -L start,end`
 - <csr-id-75d689f6a195f301e4a3000e79f58e5ec1c20557/> add `gix env` to print paths relevant to the Git installation.
 - <csr-id-278125a845a22fef39b9bc0c69e22e21fb559104/> Document the remaining subcommands
 - <csr-id-4ffe6eb8f7921c6a03db0aa6d796cc2e3cc328e0/> Add support for statistics and additional performance information.
 - <csr-id-80e5804dea9c1090efdcddbfc97ed1d573c28091/> add `gix blame` to the CLI
   That way it's possible to see the `blame` result of any file in the
   repository.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 26 calendar days.
 - 27 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #1766 from cruessler/add-range-to-gix-blame ([`90fef01`](https://github.com/GitoxideLabs/gitoxide/commit/90fef0148376167763a3ebeff91a1cf9c236cf8a))
    - Add `gix blame -L start,end` ([`4a78395`](https://github.com/GitoxideLabs/gitoxide/commit/4a783959c4862985cbffc4fe5cd2c1bed38383b1))
    - Merge pull request #1758 from GitoxideLabs/git-shell ([`851a7c4`](https://github.com/GitoxideLabs/gitoxide/commit/851a7c4d0677b15e2158dbddcdd01d1b99711ea4))
    - Merge pull request #1757 from lu-zero/document-subcommands ([`31d83a4`](https://github.com/GitoxideLabs/gitoxide/commit/31d83a49c6a3f9b061b5df3728ab242e651c2ea7))
    - Add tailing `.` for consistency ([`ddddf02`](https://github.com/GitoxideLabs/gitoxide/commit/ddddf02a74d4fb749bb625b70434b3100843a59c))
    - Add `gix env` to print paths relevant to the Git installation. ([`75d689f`](https://github.com/GitoxideLabs/gitoxide/commit/75d689f6a195f301e4a3000e79f58e5ec1c20557))
    - Document the remaining subcommands ([`278125a`](https://github.com/GitoxideLabs/gitoxide/commit/278125a845a22fef39b9bc0c69e22e21fb559104))
    - Merge pull request #1453 from cruessler/gix-blame ([`6ed9976`](https://github.com/GitoxideLabs/gitoxide/commit/6ed9976abaa3915b50efa46c46b195f3a1fc4ff7))
    - Add support for statistics and additional performance information. ([`4ffe6eb`](https://github.com/GitoxideLabs/gitoxide/commit/4ffe6eb8f7921c6a03db0aa6d796cc2e3cc328e0))
    - Add `gix blame` to the CLI ([`80e5804`](https://github.com/GitoxideLabs/gitoxide/commit/80e5804dea9c1090efdcddbfc97ed1d573c28091))
</details>

## 0.40.0 (2024-12-22)

### New Features

 - <csr-id-c7e04e976989435ba752628522d53ac39348b49b/> add first 'debug' version of `gix log`
   It's primarily meant to better understand `gix blame`.
 - <csr-id-471e046a6002a68209e215db7ee904a4149f9d51/> add `--tree-favor` to `gix merge tree|commit`.
   With it one can decide which side to favor in case of
   irreconcilable tree-conflicts.

### Bug Fixes

 - <csr-id-0727b5679f9ddeb05a9a50c895b6d77ba61ed544/> `gix merge file` now uses `THEIRS` instead of `OURS` where needed

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 27 calendar days.
 - 28 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1703](https://github.com/GitoxideLabs/gitoxide/issues/1703)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1703](https://github.com/GitoxideLabs/gitoxide/issues/1703)**
    - `gix merge file` now uses `THEIRS` instead of `OURS` where needed ([`0727b56`](https://github.com/GitoxideLabs/gitoxide/commit/0727b5679f9ddeb05a9a50c895b6d77ba61ed544))
 * **Uncategorized**
    - Merge pull request #1643 from cruessler/add-gix-log ([`29cb775`](https://github.com/GitoxideLabs/gitoxide/commit/29cb775e6b85265a4907e882a45621aeb0432564))
    - Add first 'debug' version of `gix log` ([`c7e04e9`](https://github.com/GitoxideLabs/gitoxide/commit/c7e04e976989435ba752628522d53ac39348b49b))
    - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
    - Adapt to changes in `gix-diff` ([`960773e`](https://github.com/GitoxideLabs/gitoxide/commit/960773e5526d02e1f2294224859c821ed86a3463))
    - Add `--tree-favor` to `gix merge tree|commit`. ([`471e046`](https://github.com/GitoxideLabs/gitoxide/commit/471e046a6002a68209e215db7ee904a4149f9d51))
    - Merge pull request #1704 from GitoxideLabs/fix-1703 ([`e8ce25f`](https://github.com/GitoxideLabs/gitoxide/commit/e8ce25f8d752c383ad8aea45fc7710a6d4aeb2ef))
</details>

## 0.39.0 (2024-11-24)

### New Features

 - <csr-id-0a5993f4ed6e3815949b3cd398d38d69417d21c8/> add `gix merge commit --debug`
   This yields additional debugging information with details about detected conflicts.
 - <csr-id-4a5afc7524fc96213385454079ebf9baf302ad4b/> add `gix merge commits`
 - <csr-id-84707c2b7540f9a73cc3f0cde74dabd9822cd809/> add `gix merge tree` to merge trees similarly to `git merge-tree`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 21 calendar days.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #1661 from GitoxideLabs/merge ([`0b7abfb`](https://github.com/GitoxideLabs/gitoxide/commit/0b7abfbdebe8c5ab30b89499a70dd7727de41184))
    - Add `gix merge commit --debug` ([`0a5993f`](https://github.com/GitoxideLabs/gitoxide/commit/0a5993f4ed6e3815949b3cd398d38d69417d21c8))
    - Merge pull request #1658 from GitoxideLabs/merge ([`905e5b4`](https://github.com/GitoxideLabs/gitoxide/commit/905e5b42a6163f92edef8fab82d97aeb6f17cf06))
    - Add `gix merge commits` ([`4a5afc7`](https://github.com/GitoxideLabs/gitoxide/commit/4a5afc7524fc96213385454079ebf9baf302ad4b))
    - Merge pull request #1651 from GitoxideLabs/merge ([`a876533`](https://github.com/GitoxideLabs/gitoxide/commit/a8765330fc16997dee275866b18a128dec1c5d55))
    - Adapt to changes in `gix` ([`8d590f3`](https://github.com/GitoxideLabs/gitoxide/commit/8d590f33f49b556de1748818e0bbec610566842f))
    - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
    - Add `gix merge tree` to merge trees similarly to `git merge-tree`. ([`84707c2`](https://github.com/GitoxideLabs/gitoxide/commit/84707c2b7540f9a73cc3f0cde74dabd9822cd809))
    - Support for merge related options in config tree ([`80e006b`](https://github.com/GitoxideLabs/gitoxide/commit/80e006b759d130f4f07a346b75cfc0b39986210c))
</details>

## 0.38.0 (2024-10-22)

<csr-id-28ac6572722f7ea31795dc0417521c70bcb6ec8f/>
<csr-id-786bfec7c2cf25a68705b6ef09ba6390600742b8/>

### New Features

 - <csr-id-c7213bcf2cf6be4058a78dbd86bbab4c5ad4afa7/> support for listing worktrees with `gix worktree list`
 - <csr-id-0c554e096cf2bf0a2bb91397143f524219693a3b/> add first 'debug' version of `gix diff tree`.
   It's primarily meant to better understand `gix blame`.
 - <csr-id-9c8bc03de99e6494abd9755deef7e7be5577bce2/> add new `gix cat` command.
   It only prints things without fuzz.
   
   Inspired by https://youtu.be/JYH5ILv5g1g?si=bHLBPFJiZyRUTl6u&t=211.
 - <csr-id-3da2da9d7993adc16b19fc63e7524c768a6e2e7f/> add `gix merge-file` with similar features as `git merge-file`
 - <csr-id-7249291016253647c920852fb37eb9e29d615775/> `gix merge-base` for the CLI
   For now it only supports the standard merge-base, but more derivatives
   can be added easily on demand.

### Bug Fixes

 - <csr-id-6eb4ea6cf27b2feb3edb9dbf754393b0c7d272fd/> Adjust `gix clean` warning and help for worktree fix
 - <csr-id-c241422dc64f97e9dc781465ae69130a423ee501/> Clarify -r/--repositories and --skip-hidden-repositories
   This adds information to the help text for the `git clean` options
   `-r`/`--repositories` and `--skip-hidden-repositories` to make it
   clearer what their relationship is, avoid creating the false
   impression that repositories are never deleted in the absence of
   `-r`/`--repositories`, and note that `--skip-hidden-repositories`
   is sometimes needed to preserve not only separate repositories but
   the (probably rarer) case of hidden nested worktrees of the current
   repository.

### Other

 - <csr-id-28ac6572722f7ea31795dc0417521c70bcb6ec8f/> switch from `time` to `jiff`
   This swaps out `time` for `jiff`. It doesn't completely remove `time`
   from the dependency tree. The last remaining use of `time` is in
   `prodash`, outside of the gitoxide project.

### Other

 - <csr-id-786bfec7c2cf25a68705b6ef09ba6390600742b8/> Unify style in config support info
   This builds on b31d6b7 (#1531) by adjusting the capitalization and
   quoting style of string arguments in `progress.rs` that help
   document the status of what configuration options are and are not
   planned.
   
   - Since I believe these strings are not usually rendered as
     Markdown, and most places that code formatting in Markdown would
     be used were already written with single quotes, this applies
     that quoting style consistently.
   
   - This applies initial capitalization, which was sometimes but not
     always done. It does not apply a consistent style for when a
     string will end in a `.`, which seems already to differ
     deliberately for reasons of clarity. That can be adjusted later.
   
   - A small amount of minor rewording for clarity is also included.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release over the course of 89 calendar days.
 - 91 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1572](https://github.com/GitoxideLabs/gitoxide/issues/1572)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1572](https://github.com/GitoxideLabs/gitoxide/issues/1572)**
    - Make clear what `gix commit-graph` is ([`49bca2a`](https://github.com/GitoxideLabs/gitoxide/commit/49bca2abe4512180f080f678e73c0fa40885379a))
 * **Uncategorized**
    - Merge pull request #1639 from cruessler/respect-env-variables ([`48aa74b`](https://github.com/GitoxideLabs/gitoxide/commit/48aa74b911fb874986c244712b7fd5b5cc10070b))
    - Respect env variables when running gix ([`0bebe52`](https://github.com/GitoxideLabs/gitoxide/commit/0bebe524b75346edca219d13c10b52dee3273643))
    - Merge pull request #1626 from cruessler/add-gix-diff ([`f186c23`](https://github.com/GitoxideLabs/gitoxide/commit/f186c2381b91f350813076927bf988d253fe1ad0))
    - Some minor modifications ([`6777ecb`](https://github.com/GitoxideLabs/gitoxide/commit/6777ecb99306830a3353a0db24caaa69e348ca74))
    - Add first 'debug' version of `gix diff tree`. ([`0c554e0`](https://github.com/GitoxideLabs/gitoxide/commit/0c554e096cf2bf0a2bb91397143f524219693a3b))
    - Merge pull request #1616 from Byron/git-cat ([`31bdd2e`](https://github.com/GitoxideLabs/gitoxide/commit/31bdd2ecc6c800dc57faedc9250be6d5fbcc1133))
    - Add new `gix cat` command. ([`9c8bc03`](https://github.com/GitoxideLabs/gitoxide/commit/9c8bc03de99e6494abd9755deef7e7be5577bce2))
    - Merge pull request #1611 from Byron/merge ([`5ffccd2`](https://github.com/GitoxideLabs/gitoxide/commit/5ffccd2f08d70576347e3ae17a66ca5a60f1d81c))
    - Add `gix merge-file` with similar features as `git merge-file` ([`3da2da9`](https://github.com/GitoxideLabs/gitoxide/commit/3da2da9d7993adc16b19fc63e7524c768a6e2e7f))
    - Add all keys for merge-configuration ([`e0b09d2`](https://github.com/GitoxideLabs/gitoxide/commit/e0b09d2764fd02a2b69340d9b3aef9773ae899ce))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - `gix merge-base` for the CLI ([`7249291`](https://github.com/GitoxideLabs/gitoxide/commit/7249291016253647c920852fb37eb9e29d615775))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Merge pull request #1537 from EliahKagan/progress-style ([`472fe5f`](https://github.com/GitoxideLabs/gitoxide/commit/472fe5fa18d6d1b948e319919a48f965a1199929))
    - Unify style in config support info ([`786bfec`](https://github.com/GitoxideLabs/gitoxide/commit/786bfec7c2cf25a68705b6ef09ba6390600742b8))
    - Merge pull request #1531 from EliahKagan/progress-typos ([`25a3f1b`](https://github.com/GitoxideLabs/gitoxide/commit/25a3f1b0b07c01dd44df254f46caa6f78a4d3014))
    - Fix typos in config support info ([`b31d6b7`](https://github.com/GitoxideLabs/gitoxide/commit/b31d6b79fd9bc528deaf0e3027dcaae0229ff581))
    - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
    - Switch from `time` to `jiff` ([`28ac657`](https://github.com/GitoxideLabs/gitoxide/commit/28ac6572722f7ea31795dc0417521c70bcb6ec8f))
    - Merge pull request #1471 from EliahKagan/hidden ([`a9aac4f`](https://github.com/GitoxideLabs/gitoxide/commit/a9aac4f88d1d6245a3b03bd46e691697f661c114))
    - Adjust `gix clean` warning and help for worktree fix ([`6eb4ea6`](https://github.com/GitoxideLabs/gitoxide/commit/6eb4ea6cf27b2feb3edb9dbf754393b0c7d272fd))
    - Clarify -r/--repositories and --skip-hidden-repositories ([`c241422`](https://github.com/GitoxideLabs/gitoxide/commit/c241422dc64f97e9dc781465ae69130a423ee501))
    - Merge branch 'fix-clean' ([`33eacfb`](https://github.com/GitoxideLabs/gitoxide/commit/33eacfbaace2021043e2b5d72dcb9293af6c4020))
    - Support for listing worktrees with `gix worktree list` ([`c7213bc`](https://github.com/GitoxideLabs/gitoxide/commit/c7213bcf2cf6be4058a78dbd86bbab4c5ad4afa7))
</details>

## 0.37.0 (2024-07-23)

<csr-id-202f3e48a96b38d32b11d28449358c7d1f3546ff/>

### New Features

 - <csr-id-2a563870fc4d2f65e34c4e4a448b446fc05a43b2/> add `mailmap check` similar to `git check-mailmap`.
   That way it becomes a little easier to maybe try out other special cases
   in the wild.
 - <csr-id-9bf01e42b8d8964dfd1e099d645082c10bdabcdf/> `gix clone` with `--ref` support.
   `--ref` is similar to `--branch`, but was renamed as it also supports
   tags for example.

### Other

 - <csr-id-202f3e48a96b38d32b11d28449358c7d1f3546ff/> Make it easier to compile gitoxide as dynlib
   And also optimize compilation time: By making `plumbing` and `porcelain`
   as modules the `lib.rs`, they can be compiled after the rmeta for the
   dependencies are generated.
   
   For the `uni.rs` which uses both `plumbing` and `porcelain`, this would avoid
   compiling these two modules twice.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 62 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'fix-mailmap' ([`f107014`](https://github.com/GitoxideLabs/gitoxide/commit/f107014022a62271f02790a83336aed186ad38a3))
    - Add descriptive docs for `mailmap check` ([`3e08fa3`](https://github.com/GitoxideLabs/gitoxide/commit/3e08fa313c490f4ff299724a9363e0f3b370eea6))
    - Add `mailmap check` similar to `git check-mailmap`. ([`2a56387`](https://github.com/GitoxideLabs/gitoxide/commit/2a563870fc4d2f65e34c4e4a448b446fc05a43b2))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'feat/checkout-other-refs' ([`ecfde07`](https://github.com/GitoxideLabs/gitoxide/commit/ecfde07d0887322db34f5ea531891c92676e1ff4))
    - `gix clone` with `--ref` support. ([`9bf01e4`](https://github.com/GitoxideLabs/gitoxide/commit/9bf01e42b8d8964dfd1e099d645082c10bdabcdf))
    - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
    - Thanks clippy ([`acc1331`](https://github.com/GitoxideLabs/gitoxide/commit/acc13318731fabac8f65d604baf7e47814f92ad4))
    - Merge pull request #1384 from NobodyXu/feat/easier-to-compile-gix-as-dynlib ([`bb30e52`](https://github.com/GitoxideLabs/gitoxide/commit/bb30e52112a1b738afae557f6ba7bc23889cdcb7))
    - Fix clippy warning ([`bf5a111`](https://github.com/GitoxideLabs/gitoxide/commit/bf5a1112245b6d60ceaf5591acf15acd0c8c6363))
    - Fix compilation error in `src/plumbing/main.rs` ([`4596a39`](https://github.com/GitoxideLabs/gitoxide/commit/4596a3949cdeb7c6953c2a9b8ac1e51609e1b160))
    - Fix compilation errors ([`0b5dc74`](https://github.com/GitoxideLabs/gitoxide/commit/0b5dc744b951a2ff49ddc4d2c10aaeaf10da2c4b))
    - Make it easier to compile gitoxide as dynlib ([`202f3e4`](https://github.com/GitoxideLabs/gitoxide/commit/202f3e48a96b38d32b11d28449358c7d1f3546ff))
</details>

## 0.36.0 (2024-05-22)

### New Features

 - <csr-id-886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e/> checkout respects options for `core.protectHFS` and `core.protectNTFS`.
   This also adds `gitoxide.core.protectWindows` as a way to enforce
   additional restrictions that are usually only available on Windows.
   
   Note that `core.protectNFS` is always enabled by default, just like
   [it is in Git](https://github.com/git/git/commit/9102f958ee5254b10c0be72672aa3305bf4f4704).

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 10 calendar days.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'various-fixes' ([`d6cd449`](https://github.com/GitoxideLabs/gitoxide/commit/d6cd44930fb204b06e2b70fc6965e7705530c47a))
    - Merge pull request from GHSA-7w47-3wg8-547c ([`79dce79`](https://github.com/GitoxideLabs/gitoxide/commit/79dce79c62f6072aa2653780d590dc3993dfa401))
    - Checkout respects options for `core.protectHFS` and `core.protectNTFS`. ([`886d6b5`](https://github.com/GitoxideLabs/gitoxide/commit/886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e))
    - Mark safety-related core-flags as planned ([`f3d5a69`](https://github.com/GitoxideLabs/gitoxide/commit/f3d5a69bbe0ad14502ce617dc580cc2aa481bb0a))
    - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
    - More robustness in the face of a trampling-herd of threads loading a single index. ([`addf446`](https://github.com/GitoxideLabs/gitoxide/commit/addf446f052ff74edcdb083f2b2968b313daa940))
</details>

## 0.35.0 (2024-04-13)

### New Features

 - <csr-id-84c74ffa698d35f8395c63db6acd3d0e6700d07f/> add `gix status --ignored` support
 - <csr-id-66e87cd31c060c3f97ac685ee0541c408f600362/> add `gix status --index-worktree-renames`
   This enables rename-tracking between worktree and index, something
   that Git also doesn't do or doesn't do by default.
   It is, however, available in `git2`.
 - <csr-id-61c002bc4ca5b5345c411e561fdcb492e7ae1d97/> `gix status` with submodule and rewrite support.
   Submodule changes are now picked up as long as the submodule is
   in the index.
   Further, it's possible to enable rename-tracking between
   index and worktree separately.
 - <csr-id-98b368095ec99d1bc287da7f9294a9fce424deed/> add `gix is-clean|is-changed`
   It's a good way to compare the time it takes to run a full status
   compared to a quick is-dirty check.
 - <csr-id-afd20caadb40b6b793f2099b7232669f9a8f9086/> `gix submodules list --dirty-suffix` for dirty-information
   This is a submodule-centric and greatly simplified way of obtaining
   describe information with dirty-suffix.
   
   Note that `status` information is also possible, but it seems
   hard to display nicely, which this command isn't great at
   in the first place.
 - <csr-id-58231b418fa39ea122ef41bb7691289f5b0be855/> add `gix commit describe --dirty-suffix`
   That way a suffix will be added depending on the dirty-state of the repository.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #1341 from szepeviktor/typos ([`55f379b`](https://github.com/GitoxideLabs/gitoxide/commit/55f379bc47065822d078393d83d30c0835a89782))
    - Fix typos ([`f72ecce`](https://github.com/GitoxideLabs/gitoxide/commit/f72ecce45babcad2a0c9b73c79d01ff502907a57))
    - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
    - Add `gix status --format` to communicate the current format is very simple. ([`23bea36`](https://github.com/GitoxideLabs/gitoxide/commit/23bea36f046a6f652cd1e06885ae132c85bb4f05))
    - Add `gix status --ignored` support ([`84c74ff`](https://github.com/GitoxideLabs/gitoxide/commit/84c74ffa698d35f8395c63db6acd3d0e6700d07f))
    - Add `gix status --index-worktree-renames` ([`66e87cd`](https://github.com/GitoxideLabs/gitoxide/commit/66e87cd31c060c3f97ac685ee0541c408f600362))
    - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
    - `gix status` with submodule and rewrite support. ([`61c002b`](https://github.com/GitoxideLabs/gitoxide/commit/61c002bc4ca5b5345c411e561fdcb492e7ae1d97))
    - Add `gix is-clean|is-changed` ([`98b3680`](https://github.com/GitoxideLabs/gitoxide/commit/98b368095ec99d1bc287da7f9294a9fce424deed))
    - `gix submodules list --dirty-suffix` for dirty-information ([`afd20ca`](https://github.com/GitoxideLabs/gitoxide/commit/afd20caadb40b6b793f2099b7232669f9a8f9086))
    - Add `gix commit describe --dirty-suffix` ([`58231b4`](https://github.com/GitoxideLabs/gitoxide/commit/58231b418fa39ea122ef41bb7691289f5b0be855))
</details>

## 0.34.0 (2024-02-25)

### New Features

 - <csr-id-9863d75147445d3a598fe1339d88c353850a5984/> add `gix clean --patterns-for-entries|-m` to help with wildcards.
 - <csr-id-e8597f3559187fc8add294e72eb33403cdff0e09/> basic `gix clean`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 58 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'status' ([`d53504a`](https://github.com/GitoxideLabs/gitoxide/commit/d53504a1fad41cec7b6ca2a4abb7f185d8941e3f))
    - Allow the `-n` argument as no-op to make them more compatible. ([`8959b21`](https://github.com/GitoxideLabs/gitoxide/commit/8959b2153f32c2fba599e3dfa1720e155b462b94))
    - Add `gix clean --patterns-for-entries|-m` to help with wildcards. ([`9863d75`](https://github.com/GitoxideLabs/gitoxide/commit/9863d75147445d3a598fe1339d88c353850a5984))
    - Merge branch 'tabled' ([`ed79aa7`](https://github.com/GitoxideLabs/gitoxide/commit/ed79aa7b410b9e5a013c4f204b20abd51d270559))
    - Refactor ([`e25d7eb`](https://github.com/GitoxideLabs/gitoxide/commit/e25d7eb443d9e1fde8d16422251ee0d288ff4a51))
    - Merge branch 'ein-completions' ([`cebeedf`](https://github.com/GitoxideLabs/gitoxide/commit/cebeedf572d7086c9959754c873312a183d575e3))
    - Fix clippy lints ([`454e6b9`](https://github.com/GitoxideLabs/gitoxide/commit/454e6b9de86def27c811cc93fd80f4eb169dda52))
    - Add back line wrapping simplify Usage Variants ([`4ac597a`](https://github.com/GitoxideLabs/gitoxide/commit/4ac597a0e9ccb8ed1e1864afcffdc55abc37260c))
    - Tell clap that ein is named ein ([`4fd6ab8`](https://github.com/GitoxideLabs/gitoxide/commit/4fd6ab89753f9b947034806fecb60ad77380334c))
    - Copy over code from gix to generate completions for ein as well ([`abe0043`](https://github.com/GitoxideLabs/gitoxide/commit/abe004372c6a0027339563a88162ca1aca320a1d))
    - Fix indenting by removing formatting on the config str ([`c0c4d59`](https://github.com/GitoxideLabs/gitoxide/commit/c0c4d59bbae688074184b186e08c6bf8e23c5705))
    - Reserve 50 characters for the config name ([`cc8d008`](https://github.com/GitoxideLabs/gitoxide/commit/cc8d008843a4e4567301c14ce5f25855417cf898))
    - Bring back old implementation from https://github.com/Byron/gitoxide/commit/65e64964c7cd151e53e5a7d4b9ba8fabda1c0e16 this also removes terminal_size, although its not large ([`1ecc96d`](https://github.com/GitoxideLabs/gitoxide/commit/1ecc96d319bc60cb9dbabcf17211aa7057134fd6))
    - Remove tabled ([`fcd585b`](https://github.com/GitoxideLabs/gitoxide/commit/fcd585bb6dc27bbf5bf74b422b36614241b59eb4))
    - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
    - Basic `gix clean` ([`e8597f3`](https://github.com/GitoxideLabs/gitoxide/commit/e8597f3559187fc8add294e72eb33403cdff0e09))
    - Merge branch 'chunks_exact' ([`d4d478b`](https://github.com/GitoxideLabs/gitoxide/commit/d4d478ba6a70fce34717e2164f8b3aa7a1521a2d))
    - Use chunks_exact where possible ([`2482023`](https://github.com/GitoxideLabs/gitoxide/commit/24820232f07ae55c80d95470228cc9e874830487))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
</details>

## 0.33.0 (2023-12-29)

### New Features

 - <csr-id-193ffcd9945891867a14aa5b6f87e27d3880b280/> Add `rev parse --reference`.
   It's similar to `git rev-parse --symbolic-full-name`.

### Bug Fixes (BREAKING)

 - <csr-id-59b8104a5320d946abc9f5736fa76696cef1459d/> mark `gix::interrupt::init_handler()` as unsafe
   The passed `interrupt()` argument will be called from a signal
   handler, so that needs to be documented and the call sites need to
   state that they fulfill the contract.
   
   Thanks to @Manishearth for pointing this out.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 21 calendar days.
 - 22 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade tabled and setup wordwrapping ([`a78caba`](https://github.com/GitoxideLabs/gitoxide/commit/a78caba1e70adfae7beb2b4f6a18b8124efba46c))
    - Merge branch 'tracking-branch' ([`0fe20e8`](https://github.com/GitoxideLabs/gitoxide/commit/0fe20e80145419e1662f869657dabf689786395f))
    - Add `rev parse --reference`. ([`193ffcd`](https://github.com/GitoxideLabs/gitoxide/commit/193ffcd9945891867a14aa5b6f87e27d3880b280))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Merge branch 'push-yvzxzqrkkvry' ([`4917beb`](https://github.com/GitoxideLabs/gitoxide/commit/4917beb5760a9bafb75b59331b282f4d6dbb64f5))
    - Mark `gix::interrupt::init_handler()` as unsafe ([`59b8104`](https://github.com/GitoxideLabs/gitoxide/commit/59b8104a5320d946abc9f5736fa76696cef1459d))
</details>

## 0.32.0 (2023-12-06)

### New Features

<csr-id-4aea9b097fb08e504cdfc4a7c3b7511a308dc074/>
<csr-id-c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c/>
<csr-id-8f795e8abf706a24fe104500bf15efaa2bc07b15/>
<csr-id-8dfbb4ba531d86894a59e2b85ec29cd3133c8c4d/>
<csr-id-f9ae1bc6b514b6bbcda2c672f7b7ccbd999356a0/>
<csr-id-886289f98003f935d774a40f6db746ae70649936/>

 - <csr-id-cf51a4de2d06eb28435ef8e386131710003b6928/> `gix rev parse --format` to provide different versions of the same content.
   This only applies to blobs, but allows to obtain different versions of the same blob
   like:
   
   * what's stored in Git
* what would be checked out to the worktree
* what would be diffed

### Bug Fixes

 - <csr-id-b2300782ad09f17730fe2a5ba4938517d4e7ec16/> in `--trace` mode, greatly increase message-buffer size.
   That way, it's much less likely that messages will get lost
   due to being overwritten before they can be displayed every
   100ms or so.
 - <csr-id-eab22bd5d56bf91ee0edfc7156023c159c7f65f9/> set binary name for completions

### Bug Fixes (BREAKING)

 - <csr-id-2189cee47f99350b368390eaa2a01961bb77c250/> rename `GITOXIDE_*` environment variables to `GIX_#`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 27 commits contributed to the release.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'adjustments-for-cargo' ([`8156340`](https://github.com/GitoxideLabs/gitoxide/commit/8156340724b1b7cb15824f88c75f6ddd7302cff5))
    - Rename `GITOXIDE_*` environment variables to `GIX_#` ([`2189cee`](https://github.com/GitoxideLabs/gitoxide/commit/2189cee47f99350b368390eaa2a01961bb77c250))
    - Merge branch 'gix-status' ([`5fdc9df`](https://github.com/GitoxideLabs/gitoxide/commit/5fdc9df069f3d9a4bd88e4e0ca5d67916e2908c9))
    - `gix rev parse --format` to provide different versions of the same content. ([`cf51a4d`](https://github.com/GitoxideLabs/gitoxide/commit/cf51a4de2d06eb28435ef8e386131710003b6928))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
    - Adapt to changes in `gix-diff` ([`1706e23`](https://github.com/GitoxideLabs/gitoxide/commit/1706e2394380c35cd98d0e106eb0985ae1912da0))
    - Add the`diff::resource_cache()` low-level utility for rapid in-memory diffing of combinations of resources. ([`4aea9b0`](https://github.com/GitoxideLabs/gitoxide/commit/4aea9b097fb08e504cdfc4a7c3b7511a308dc074))
    - Merge branch 'support_ssl_verify' ([`5ce9784`](https://github.com/GitoxideLabs/gitoxide/commit/5ce978432231e257ef625fc401895b34f963bf6d))
    - In gix read http.sslVerify config value and pass it to gix-transport. ([`c6e83cf`](https://github.com/GitoxideLabs/gitoxide/commit/c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Remove dead code from non-existent "lean-cli" feature ([`be1fa2f`](https://github.com/GitoxideLabs/gitoxide/commit/be1fa2f6370501c6b8bbe9bb89f6ef5507b5f425))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'feat_basic_connectivity_check' ([`1f9aca5`](https://github.com/GitoxideLabs/gitoxide/commit/1f9aca5de45c1f7c25606cd1ddc6b93a915dcd77))
    - Flatten `fsck connectivity` into just `fsck` much like `git fsck` ([`7ab5c76`](https://github.com/GitoxideLabs/gitoxide/commit/7ab5c7625fcbfade21d4f754be7afd2f73a828ba))
    - Refactor ([`7a88b42`](https://github.com/GitoxideLabs/gitoxide/commit/7a88b420f0a43fb1ce163698723b30566add97a9))
    - Add basic connectivity check ([`8f795e8`](https://github.com/GitoxideLabs/gitoxide/commit/8f795e8abf706a24fe104500bf15efaa2bc07b15))
    - Merge branch 'fix-1096' ([`48ef17e`](https://github.com/GitoxideLabs/gitoxide/commit/48ef17e4d169f2b08d119c2dba51fcfd9c06fe61))
    - Allow to print a tree without prettification, using `--tree-style raw`. ([`8dfbb4b`](https://github.com/GitoxideLabs/gitoxide/commit/8dfbb4ba531d86894a59e2b85ec29cd3133c8c4d))
    - Merge branch 'trace-packetlines' ([`e7de4c7`](https://github.com/GitoxideLabs/gitoxide/commit/e7de4c702a223ad9eb19b407391028dcb08d80c4))
    - Automatically enforce strict mode if `-c` options are given on the command-line. ([`f9ae1bc`](https://github.com/GitoxideLabs/gitoxide/commit/f9ae1bc6b514b6bbcda2c672f7b7ccbd999356a0))
    - In `--trace` mode, greatly increase message-buffer size. ([`b230078`](https://github.com/GitoxideLabs/gitoxide/commit/b2300782ad09f17730fe2a5ba4938517d4e7ec16))
    - Merge branch 'discover-split-worktree' ([`16170d9`](https://github.com/GitoxideLabs/gitoxide/commit/16170d9c2e4de6a2e639ff99b75e65bbd0e782d7))
    - Add `gix free discover` to inform about repository discovery. ([`886289f`](https://github.com/GitoxideLabs/gitoxide/commit/886289f98003f935d774a40f6db746ae70649936))
    - Merge branch 'fix_completion-bin-name' ([`fb81093`](https://github.com/GitoxideLabs/gitoxide/commit/fb81093684a7ff3d9979ef21c2c8f771b6f99a50))
    - Refactor ([`7f2bc08`](https://github.com/GitoxideLabs/gitoxide/commit/7f2bc08e5713758c6188428221aac518cc6ecf2f))
    - Set binary name for completions ([`eab22bd`](https://github.com/GitoxideLabs/gitoxide/commit/eab22bd5d56bf91ee0edfc7156023c159c7f65f9))
</details>

## 0.31.1 (2023-10-13)

A re-release to deal with breakage in the dependency tree (self-caused).

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Improve documentation slightly ([`b7c9275`](https://github.com/GitoxideLabs/gitoxide/commit/b7c92753f52e71f118b6e21ab672ae56961306c5))
</details>

## 0.31.0 (2023-10-12)

This release includes an overhaul of the url parsing implementation, allowing more urls to be parsed correctly and refining the error messages for
invalid ones. Improvements include the added support for ssh aliases (`github:byron/gitoxide` has previously been parsed as local path), adjustments
around the interpretation of colons in file names (previously we disallowed colons that were not followed up with a slash character) and some smaller
changes that bring the interpretation of file urls more in line with Git's implementation.

There are still many (edge) cases in Git's url parsing implementation which are not handled correctly by Gitoxide. If you notice any such deviation please
open a new issue to help us making Gitoxide even more correct.

### New Features

 - <csr-id-70923a0fad3d409069bd9d0b492e6a9b7b113261/> add shell completions for bash
 - <csr-id-46e591914d548bacae2656ffe14a0ea7ca2eb7ae/> `gix status` auto-writes changed indices.
   This prevents expensive operations to re-occour.
 - <csr-id-7ba2fa1c7781230913b0a04ad8684fa7d0143c18/> `gix status -s/--statistics` to obtain additional information on what happened.
   This is useful for understanding performance characteristics in detail.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 7 calendar days.
 - 18 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'feat_completions' ([`681c607`](https://github.com/GitoxideLabs/gitoxide/commit/681c60774704a3c1d3ed8de916078833cf0398a7))
    - Refactor ([`fdd2df8`](https://github.com/GitoxideLabs/gitoxide/commit/fdd2df8e42f2a14a90e9a575e19811f25ab62237))
    - User decides completion output ([`215c3ac`](https://github.com/GitoxideLabs/gitoxide/commit/215c3acfd25554b9c2259d069f3916a9d6d911f5))
    - Add shell completions for bash ([`70923a0`](https://github.com/GitoxideLabs/gitoxide/commit/70923a0fad3d409069bd9d0b492e6a9b7b113261))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Trust Ctime again ([`f929d42`](https://github.com/GitoxideLabs/gitoxide/commit/f929d420cb768f2df1d7886564ca03b3c3254a82))
    - `gix status` auto-writes changed indices. ([`46e5919`](https://github.com/GitoxideLabs/gitoxide/commit/46e591914d548bacae2656ffe14a0ea7ca2eb7ae))
    - `gix status -s/--statistics` to obtain additional information on what happened. ([`7ba2fa1`](https://github.com/GitoxideLabs/gitoxide/commit/7ba2fa1c7781230913b0a04ad8684fa7d0143c18))
</details>

## 0.30.0 (2023-09-24)

<csr-id-d3ac691446c9d029eb4f04d111887fa06720939d/>
<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

This release contains a security fix which assure URLs like `'ssh://-oProxyCommand=open$IFS-aCalculator/foo'` will not execute attacker-controlled
strings. The above can, on MacOS, launch the calculator app when using it with `gix clone` for example, up to v0.29.0.

### New Features

 - <csr-id-f094f71dc1a50955552509d108556c01517c6ed6/> `gix status` with basic index-worktree comparison
 - <csr-id-3ff5ac0cda9e3e089dc79fdfbff5ff619ee60b1f/> `gix free index from-list` and `gix index from-tree` gain `--skip-hash`.
   This flag can be derived from options, but thus far we have no higher-level
   writing of the index so this has to do to see the difference in performance.

### Refactor

 - <csr-id-d3ac691446c9d029eb4f04d111887fa06720939d/> both `ein` and `gix` now share some code via the `gitoxide` library.
   This can slightly improve compile times as well, even though it wasn't measured.

### Chore (BREAKING)

 - <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
   It makes proper usage of `Progress` types easier and allows them to be used
   as `dyn` traits as well.

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 27 calendar days.
 - 33 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#987](https://github.com/GitoxideLabs/gitoxide/issues/987), [#992](https://github.com/GitoxideLabs/gitoxide/issues/992)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#987](https://github.com/GitoxideLabs/gitoxide/issues/987)**
    - Use a multi-call binary ('uni') to have only one build step ([`4ef9a32`](https://github.com/GitoxideLabs/gitoxide/commit/4ef9a32307a5e91868161a5f683b6b97ad9ebbdd))
 * **[#992](https://github.com/GitoxideLabs/gitoxide/issues/992)**
    - Both `ein` and `gix` now share some code via the `gitoxide` library. ([`d3ac691`](https://github.com/GitoxideLabs/gitoxide/commit/d3ac691446c9d029eb4f04d111887fa06720939d))
 * **Uncategorized**
    - Merge branch 'reset' ([`54a8495`](https://github.com/GitoxideLabs/gitoxide/commit/54a849545140f7f1c0c7564c418071c0a76a34e7))
    - `gix status` with basic index-worktree comparison ([`f094f71`](https://github.com/GitoxideLabs/gitoxide/commit/f094f71dc1a50955552509d108556c01517c6ed6))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
    - Merge branch 'fixes' ([`4bfd1cc`](https://github.com/GitoxideLabs/gitoxide/commit/4bfd1cc8f7922a8c4de6b9d078d54b93e78f51ff))
    - Thanks clippy ([`0d6d4ec`](https://github.com/GitoxideLabs/gitoxide/commit/0d6d4ec8030d2e8f4c7a9d6f421d54776c4b67fb))
    - Adapt to changes in `gix-index` and pass skip-hash through for performance.. ([`713cd59`](https://github.com/GitoxideLabs/gitoxide/commit/713cd59f0b1eff6397b80f1e1fceec278532fd59))
    - `gix free index from-list` and `gix index from-tree` gain `--skip-hash`. ([`3ff5ac0`](https://github.com/GitoxideLabs/gitoxide/commit/3ff5ac0cda9e3e089dc79fdfbff5ff619ee60b1f))
    - Add more configuration variables prior to potentially using them; remove `index.skipHash` ([`773b6e3`](https://github.com/GitoxideLabs/gitoxide/commit/773b6e3d5b69f819805fc77e963ec2a059bf40be))
</details>

## 0.29.0 (2023-08-22)

### New Features

 - <csr-id-1ccbe16117d58b68b25a8e3e66676a61a07dd49c/> `gix submodule` subcommand for simple submodule listing and information retrieval
 - <csr-id-6bc69e37deb3ebd88af8b0e8ffb9661562dcc679/> `gix index entries --recurse-subomdules` to also list submodules.
 - <csr-id-c30ac0c9f89fb4480c8f8c8c0c06fa046dcd4314/> `gix index entries` with styles and pathspecs.
   This adds support for more simple git style, which is faster and thus
   allows for more direct comparisons to `git ls-files`.
 - <csr-id-f194cfc920cc8622215bab5aa6b7b87e4fb9d7b2/> use real pathspecs where it was supported before.
 - <csr-id-cd6cfe4049bacd8d8a691668b5501a673867f149/> add `gix commit verify-signature` to do something akin to `git ... --show-signature`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 15 calendar days.
 - 30 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
    - `gix submodule` subcommand for simple submodule listing and information retrieval ([`1ccbe16`](https://github.com/GitoxideLabs/gitoxide/commit/1ccbe16117d58b68b25a8e3e66676a61a07dd49c))
    - Just fmt ([`0d258f4`](https://github.com/GitoxideLabs/gitoxide/commit/0d258f40afcd848509e2b0c7c264e9f346ed1726))
    - Merge branch 'submodule-in-gix' ([`36f7b78`](https://github.com/GitoxideLabs/gitoxide/commit/36f7b783c67b8a087076a130f5ee9b90b23bc3cc))
    - `gix index entries --recurse-subomdules` to also list submodules. ([`6bc69e3`](https://github.com/GitoxideLabs/gitoxide/commit/6bc69e37deb3ebd88af8b0e8ffb9661562dcc679))
    - Merge branch 'submodule-active' ([`a3afaa4`](https://github.com/GitoxideLabs/gitoxide/commit/a3afaa42741616a0f1abeef9b54557e7c2b800cb))
    - Adapt to changes in `gix` ([`ca972a2`](https://github.com/GitoxideLabs/gitoxide/commit/ca972a244c9b074a2dc89ff3bbf83df0c3853904))
    - Use `Defaults::from_environment()` when parsing pathspecs. ([`2a99034`](https://github.com/GitoxideLabs/gitoxide/commit/2a99034bf451b1a801650b80705c96ce4c9e61c1))
    - Optimize startup times of `gix index entries` ([`22477bd`](https://github.com/GitoxideLabs/gitoxide/commit/22477bdcf37419627dbb7afe0df166935825ad2b))
    - Merge branch 'pathspec-matching' ([`9f4dfe0`](https://github.com/GitoxideLabs/gitoxide/commit/9f4dfe0f0b948280692916b596923959ea2fd9da))
    - `gix index entries` with styles and pathspecs. ([`c30ac0c`](https://github.com/GitoxideLabs/gitoxide/commit/c30ac0c9f89fb4480c8f8c8c0c06fa046dcd4314))
    - Use real pathspecs where it was supported before. ([`f194cfc`](https://github.com/GitoxideLabs/gitoxide/commit/f194cfc920cc8622215bab5aa6b7b87e4fb9d7b2))
    - Merge branch 'handlers-mt' ([`f584d76`](https://github.com/GitoxideLabs/gitoxide/commit/f584d7698d93836daef2000fd369034de46037f0))
    - Adapt to changes in `gix` ([`a201f0d`](https://github.com/GitoxideLabs/gitoxide/commit/a201f0d45936826ef26ac815acb9d2efbb3d5c68))
    - Merge branch 'extract-signatures' ([`b37affe`](https://github.com/GitoxideLabs/gitoxide/commit/b37affefecfb30a94431cd21dae6659004ca6244))
    - Add `gix commit verify-signature` to do something akin to `git ... --show-signature`. ([`cd6cfe4`](https://github.com/GitoxideLabs/gitoxide/commit/cd6cfe4049bacd8d8a691668b5501a673867f149))
    - Merge branch 'archive-gz' ([`c7d9129`](https://github.com/GitoxideLabs/gitoxide/commit/c7d912917a2dad5c076d0bd645cfda092c66ff79))
    - Adapt to changes in `gix-archive` ([`feba76d`](https://github.com/GitoxideLabs/gitoxide/commit/feba76d636e3d177c41f120639980f8b46edfcaa))
    - Merge branch 'submodules' ([`b629f8a`](https://github.com/GitoxideLabs/gitoxide/commit/b629f8a774931d58c0a9b124fa75f85807c6c5d1))
    - `.gitmodule` file abstraction ([`6a2e6a4`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e6a436f76c8bbf2487f9967413a51356667a0))
</details>

## 0.28.0 (2023-07-22)

### New Features

 - <csr-id-32bbb8b7b1f195adf7e5f06fd2ddc19153516a2f/> add simple CLI for `gix archive`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 29 calendar days.
 - 30 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'gix-archive' ([`1dda48b`](https://github.com/GitoxideLabs/gitoxide/commit/1dda48ba2fccb93ebac00fe3460e923af43c86ce))
    - Allow to create additional entries via the command-line ([`4a9d0f1`](https://github.com/GitoxideLabs/gitoxide/commit/4a9d0f1df6f7a20f7f1769fd9c7dae2e53f7e83f))
    - Add compression support to `gix archive`, which is where it should shine. ([`567b1a4`](https://github.com/GitoxideLabs/gitoxide/commit/567b1a4488c43c1f7099435d10cdddbc3a98a5cc))
    - Add simple CLI for `gix archive` ([`32bbb8b`](https://github.com/GitoxideLabs/gitoxide/commit/32bbb8b7b1f195adf7e5f06fd2ddc19153516a2f))
    - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
    - Add keys required to deal with worktree conversions and filters. ([`3fbd7b0`](https://github.com/GitoxideLabs/gitoxide/commit/3fbd7b0c864cf2f1a38ae24e85d47b0b26b271a7))
    - Merge branch 'basic-filtering' ([`3fd5e16`](https://github.com/GitoxideLabs/gitoxide/commit/3fd5e16e205db18edc21341fb4c2a75d0726f5a5))
    - Add some configuration variables as planned due to relevancy with filters ([`fa04378`](https://github.com/GitoxideLabs/gitoxide/commit/fa0437899292e9e23db932041220bbeda732c246))
</details>

## 0.27.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

This release fixes a possibility for V1 fetches to deadlock if negotiation would span multiple rounds 
(see issue [#882](https://github.com/Byron/gitoxide/issues/882)) for details.

### New Features (BREAKING)

 - <csr-id-b82edc8b965344a866039f6c99295db4f8e776e2/> list commit-graph entries by graph traversal, move commit-graph up to `gix` level.
   This is merely a debug tool to learn about generation numbers.
   All commit-graph commands now operate on a repository.

### New Features

 - <csr-id-c494cfd6f80cf273107c626d0f2cbe054db378ee/> `gix --trace` to also print tree-like instrumentation
 - <csr-id-452ed6b79ed0e12ae214af3c8cd49a87dba91c73/> `gix fetch --open-negotiation-graph[=limit]`
   Open the negotiation graph as SVG, after optionally specifying a limit
   as rendering/layouting can be very slow.
   
   It's useful to see how the negotiation algorithm is reasoning about each commit.
 - <csr-id-096838ffa529148b2da3a2382a2d1fdbfe5bee5b/> `gix fetch --negotiation-info` to provide additional information about the negotiation phase.
 - <csr-id-bd32e393fedd01e27a4f6984281bcc3182c63b67/> `bit revision list --svg` to create a visual graph of commits.
   It's mainly a test of how well `layout-rs` performs.

### Chore

 - <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 27 commits contributed to the release over the course of 10 calendar days.
 - 15 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Make `gitoxide` publishable again ([`c16b1df`](https://github.com/GitoxideLabs/gitoxide/commit/c16b1dfda63f08c40ebab22b0a07e4617765acf5))
    - Merge branch 'gix-corpus' ([`5861afb`](https://github.com/GitoxideLabs/gitoxide/commit/5861afb45f32c16eefcd8e7b7480309bf44b6edc))
    - Add `corpus --dry-run` and `--task-sql-suffix` and `--repo-sql-suffix` ([`4cef57d`](https://github.com/GitoxideLabs/gitoxide/commit/4cef57db735c20da3fa1c56d9c1744e4f653bce0))
    - Gix-corpus now respects the --trace flag ([`0f973ac`](https://github.com/GitoxideLabs/gitoxide/commit/0f973ac53df631ad2abdf85dbe2453e528c7e6c3))
    - Adapt to changes in `gix-trace` ([`bd6e3d7`](https://github.com/GitoxideLabs/gitoxide/commit/bd6e3d7682d122997f8d3543eb87a62121ef0669))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Thanks clippy ([`361cf96`](https://github.com/GitoxideLabs/gitoxide/commit/361cf964f523fff5d92a8641bc3805b7e1082b18))
    - `gix --trace` to also print tree-like instrumentation ([`c494cfd`](https://github.com/GitoxideLabs/gitoxide/commit/c494cfd6f80cf273107c626d0f2cbe054db378ee))
    - Run tasks in parallel ([`cfd8e88`](https://github.com/GitoxideLabs/gitoxide/commit/cfd8e88e428075701a162cf8da6986700fd8c5af))
    - Refresh a corpus repositories by updating all of them. ([`a0b4385`](https://github.com/GitoxideLabs/gitoxide/commit/a0b4385d33112fc3f950ac40d874883e7ad075c0))
    - Obtain a repository-list with classifiers ([`a4300c8`](https://github.com/GitoxideLabs/gitoxide/commit/a4300c8318ded98dc465405d39ed014e0534de66))
    - Provide all the meta-data that is needed to make a run (and associate it with that) ([`fcbda1d`](https://github.com/GitoxideLabs/gitoxide/commit/fcbda1d2fee735c8bf3e7357a8691af2f7637922))
    - A build script to set a `gitoxide` version according to what's in git ([`83f6466`](https://github.com/GitoxideLabs/gitoxide/commit/83f646674118ff6227a605dc17b0be7325481069))
    - A basic command to perform a corpus run ([`d9e74ff`](https://github.com/GitoxideLabs/gitoxide/commit/d9e74ffdedf585a68d09d04f493c98eda729ff3b))
    - Merge branch 'gix-revision-graph' ([`036e60a`](https://github.com/GitoxideLabs/gitoxide/commit/036e60a3ad39ba9b018c0b56454f12fad455c7bb))
    - `gix fetch --open-negotiation-graph[=limit]` ([`452ed6b`](https://github.com/GitoxideLabs/gitoxide/commit/452ed6b79ed0e12ae214af3c8cd49a87dba91c73))
    - `gix fetch --negotiation-info` to provide additional information about the negotiation phase. ([`096838f`](https://github.com/GitoxideLabs/gitoxide/commit/096838ffa529148b2da3a2382a2d1fdbfe5bee5b))
    - Make top-level `gix` docs available when using `cargo doc` ([`518159d`](https://github.com/GitoxideLabs/gitoxide/commit/518159da99cf07384e9cacdc1d62a5dc1e02f09c))
    - `bit revision list --svg` to create a visual graph of commits. ([`bd32e39`](https://github.com/GitoxideLabs/gitoxide/commit/bd32e393fedd01e27a4f6984281bcc3182c63b67))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Merge branch 'fix-commitgraph' ([`2213321`](https://github.com/GitoxideLabs/gitoxide/commit/22133210f2d073eb472c51c83f29b18cf8bdd7ca))
    - Bring back the no-repo commitgraph for stress-tests to work ([`ff8d42a`](https://github.com/GitoxideLabs/gitoxide/commit/ff8d42aef988b99153950a24efda80acebde5cac))
    - Move commit-graph implementation into its correct place ([`750b07a`](https://github.com/GitoxideLabs/gitoxide/commit/750b07a69c659cedd46f33396468d4faa7e73f10))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
    - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
    - List commit-graph entries by graph traversal, move commit-graph up to `gix` level. ([`b82edc8`](https://github.com/GitoxideLabs/gitoxide/commit/b82edc8b965344a866039f6c99295db4f8e776e2))
    - Add options related to commitgraph to `gix config tree` ([`7aaaebf`](https://github.com/GitoxideLabs/gitoxide/commit/7aaaebfea9c1935f0a87ab81cef4261e00e4c159))
</details>

## 0.26.0 (2023-06-07)

The main attraction of this release is support for multi-round negotiations thanks to support for the standard negotiations algorithms.
Further, `gix index entries` now lists attributes as well, and there is the new `gix attribute query` sub-command in the vain of `git check-attrs`.

### New Features

 - <csr-id-e1fcc7f69e7b95aeac8cbbde3719a1e6b9dafeba/> `gix attributes validate` to validate attributes and ignore against `git` as baseline.
   Use this command to test the entire index for ignored paths and their attributes and use
   `git check-attr` and `git check-ignore` to validate that `git` agrees. Collect all
   mismatches and print them.
 - <csr-id-450b2325909459c7061f3d02f461c3f0f20f1e86/> `--statistics` for `gix excludes query`
   With it one gets some insights into the work done to fulfil the query.
 - <csr-id-bfdcb147cdc3d0338a4cd2bc915b0e0eeced1e99/> `gix attribute query` as something similar to `git check-attrs`.
 - <csr-id-b8db2072bb6a5625f37debe9e58d08461ece67dd/> `no-repo index from-list` to create an index with empty files from the given list.
   The list could be created with `find . -type f` for example.
 - <csr-id-13a0f1e568cf9ee462ecf01d416cfcd4949fd9c6/> add pro-forma library to allow feature documentation of `gitoxide` on docs.rs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 34 calendar days.
 - 40 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - `just fmt` ([`ffc1276`](https://github.com/GitoxideLabs/gitoxide/commit/ffc1276e0c991ac33ce842f5dca0b45ac69680c0))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Adapt to changes in `gix` ([`7983f6f`](https://github.com/GitoxideLabs/gitoxide/commit/7983f6fd4ef242d494962fe35b8b8f91a6135f32))
    - Keep track of core.alternateRefsCommand ([`d925896`](https://github.com/GitoxideLabs/gitoxide/commit/d9258969047a53bcb437b4ebd9467e6c95d715d7))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Include custom clippy settings ([`b057500`](https://github.com/GitoxideLabs/gitoxide/commit/b057500dd3e6b75be3ebcd258cda0b946bedd9e1))
    - Merge branch 'consecutive-negotiation' ([`4507f94`](https://github.com/GitoxideLabs/gitoxide/commit/4507f94984c811ea098e43472e5f54ec4dbb90c1))
    - Support reading the fetch algorithm from configuration ([`33b7770`](https://github.com/GitoxideLabs/gitoxide/commit/33b777074db21db8cd060ecf8cfdac0409a7e10c))
    - Merge branch 'gix-attributes-validate' ([`a849da8`](https://github.com/GitoxideLabs/gitoxide/commit/a849da8e35ca14fef9a2431fe1bb1c05b249680e))
    - `gix attributes validate` to validate attributes and ignore against `git` as baseline. ([`e1fcc7f`](https://github.com/GitoxideLabs/gitoxide/commit/e1fcc7f69e7b95aeac8cbbde3719a1e6b9dafeba))
    - Merge branch 'gix-attribute-query' ([`1d70213`](https://github.com/GitoxideLabs/gitoxide/commit/1d70213b5c41611c9a6172901b78844a76dece0e))
    - `--statistics` for `gix excludes query` ([`450b232`](https://github.com/GitoxideLabs/gitoxide/commit/450b2325909459c7061f3d02f461c3f0f20f1e86))
    - `gix attribute query` as something similar to `git check-attrs`. ([`bfdcb14`](https://github.com/GitoxideLabs/gitoxide/commit/bfdcb147cdc3d0338a4cd2bc915b0e0eeced1e99))
    - `no-repo index from-list` to create an index with empty files from the given list. ([`b8db207`](https://github.com/GitoxideLabs/gitoxide/commit/b8db2072bb6a5625f37debe9e58d08461ece67dd))
    - Merge branch 'fix-841' ([`edeb4de`](https://github.com/GitoxideLabs/gitoxide/commit/edeb4de2fa0f4e8ecab0c7f4af169d4443cf0eaf))
    - Add pro-forma libaray to allow feature documentation of `gitoxide` on docs.rs ([`13a0f1e`](https://github.com/GitoxideLabs/gitoxide/commit/13a0f1e568cf9ee462ecf01d416cfcd4949fd9c6))
    - Use unlimited parallelism for `jwalk` to have a better comparison. ([`d67551d`](https://github.com/GitoxideLabs/gitoxide/commit/d67551d4e43359990b4391ea2902eaf13631dd99))
</details>

## 0.25.0 (2023-04-27)

A maintenance release without user-facing changes, just to fix installation

## 0.24.0 (2023-04-27)

### New Features

 - <csr-id-08e8fc2152794652ba1c986df493c2ac915af9e7/> `gix index entries` also prints attributes.
 - <csr-id-9723e1addf52cc336d59322de039ea0537cdca36/> `gix clone` and `gix fetch` with controls for shallow repositories.
 - <csr-id-f93e964b1f7ba95861966ce58ebef477013dd1b4/> `ein t trace-path` know accepts the `trace-file` alias.

### Bug Fixes

 - <csr-id-dd14a80a78fea3f092b1d5300637c80cfd9e59a7/> printing to stdout for commands that don't show progress is greatly improved.
   Previously it would have to lock `stdout` on each write, now this is done only once.
 - <csr-id-7bd8823ab4241d6d0401f03aec8c0d34f68c347c/> opening repositories without 'strict' mode also ignores IO errors.
   These will instead be logged, but won't make it impossible to open an
   otherwise fine repository.

### New Features (BREAKING)

 - <csr-id-ca8ebdfb9647ff15b0293823bb3bb3c6779c8dd2/> turn `gix free index entries` into `gix index entries`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#790](https://github.com/GitoxideLabs/gitoxide/issues/790)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update crate status ([`ed63ec0`](https://github.com/GitoxideLabs/gitoxide/commit/ed63ec08ac651d642123d5abd1d4589e67049ff2))
 * **[#790](https://github.com/GitoxideLabs/gitoxide/issues/790)**
    - Opening repositories without 'strict' mode also ignores IO errors. ([`7bd8823`](https://github.com/GitoxideLabs/gitoxide/commit/7bd8823ab4241d6d0401f03aec8c0d34f68c347c))
 * **Uncategorized**
    - Merge branch 'index-entries-attrs' ([`f37a930`](https://github.com/GitoxideLabs/gitoxide/commit/f37a930aefa27e67f0b693ba9669cc26d49044fa))
    - `gix index entries` also prints attributes. ([`08e8fc2`](https://github.com/GitoxideLabs/gitoxide/commit/08e8fc2152794652ba1c986df493c2ac915af9e7))
    - Printing to stdout for commands that don't show progress is greatly improved. ([`dd14a80`](https://github.com/GitoxideLabs/gitoxide/commit/dd14a80a78fea3f092b1d5300637c80cfd9e59a7))
    - Turn `gix free index entries` into `gix index entries`. ([`ca8ebdf`](https://github.com/GitoxideLabs/gitoxide/commit/ca8ebdfb9647ff15b0293823bb3bb3c6779c8dd2))
    - Merge branch 'attributes-cache' ([`3456c84`](https://github.com/GitoxideLabs/gitoxide/commit/3456c845dfeedd2fa96b4313b1a84c8cbe9433c5))
    - Adjust to changes in `gix-worktree` ([`13a070f`](https://github.com/GitoxideLabs/gitoxide/commit/13a070f405230d52e4377e18f6bdc5c673b718a0))
    - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
    - Add information about configuration flags related to fsync usage ([`8d5762c`](https://github.com/GitoxideLabs/gitoxide/commit/8d5762c2031cc24bcea7eaba2911b907ba443b72))
    - Merge branch 'main' into dev ([`23ee47f`](https://github.com/GitoxideLabs/gitoxide/commit/23ee47fb24c197f8437bd426544b2aa74e005bdc))
    - Merge branch 'worktree-stack' ([`3d47919`](https://github.com/GitoxideLabs/gitoxide/commit/3d47919c1a2f83fc7c1fd7ae590d098057a22626))
    - Assure we load all gitattributes when needed. ([`9237121`](https://github.com/GitoxideLabs/gitoxide/commit/923712175fffaa6d0b71e109f243bdd6bea3ff36))
    - Merge branch 'fix-790' ([`ee36e5b`](https://github.com/GitoxideLabs/gitoxide/commit/ee36e5bb985e9ad90bc382cdd051a8b5295ca18c))
    - Add group headings to all shallow-related arguments. ([`bbb4cb0`](https://github.com/GitoxideLabs/gitoxide/commit/bbb4cb03ee2bcd0c5dd0635da542a942179cf6fd))
    - Keep track of `index.skipHash` ([`f29f35c`](https://github.com/GitoxideLabs/gitoxide/commit/f29f35cfc43f435f15aebc4d678f01d7f7ab7fbf))
    - Make fmt ([`3836cc0`](https://github.com/GitoxideLabs/gitoxide/commit/3836cc0c9c3e1158b56142b924483c8a77217d53))
    - Merge branch 'various-fixes' ([`cc0f506`](https://github.com/GitoxideLabs/gitoxide/commit/cc0f5061fba27d57022dc616c941034b98fd4875))
    - Fix clap derive declaration of conflicts ([`c502fa8`](https://github.com/GitoxideLabs/gitoxide/commit/c502fa8362cb449dc1837a0b8febfad03d9f66ef))
    - Merge branch 'shallow-protocol' ([`531dd19`](https://github.com/GitoxideLabs/gitoxide/commit/531dd19502b8b635fb1a60f747eb381fd12e00ca))
    - `gix clone` and `gix fetch` with controls for shallow repositories. ([`9723e1a`](https://github.com/GitoxideLabs/gitoxide/commit/9723e1addf52cc336d59322de039ea0537cdca36))
    - Improve error handling and reporting of `ein t query` ([`d5616b6`](https://github.com/GitoxideLabs/gitoxide/commit/d5616b66052cd20cbfabbecc1f5f500f754e89a2))
    - Make fmt ([`8ef1cb2`](https://github.com/GitoxideLabs/gitoxide/commit/8ef1cb293434c7b9e1fda4a6963368e0435920a9))
    - Merge branch 'rename-tracking' ([`3827ac6`](https://github.com/GitoxideLabs/gitoxide/commit/3827ac6c6b002219a7fb0877ec01d8afc909ca4c))
    - `ein t trace-path` know accepts the `trace-file` alias. ([`f93e964`](https://github.com/GitoxideLabs/gitoxide/commit/f93e964b1f7ba95861966ce58ebef477013dd1b4))
</details>

## 0.23.0 (2023-02-24)

This release adds the new `ein tool query` analytics engine, which maintains a database of a git repository as acceleration data structure to run useful queries on.
It's nothing more than an MVP, and was inspired by a program to accelerate [Git-Heat-Map](https://github.com/jmforsythe/Git-Heat-Map/pull/6).

### New Features

 - <csr-id-f8cc62376bd6e940be4c41e84ccbfce111e64e39/> `ein tool query` - a git analytics engine.
   A tool to build and efficiently maintain a database of information contained
   in a git repository, preferably the kind of information that is expensive to obtain,
   in order to facilitate queries that would be prohibitive without an accelerating
   data structure.
 - <csr-id-49520d1f03ee1d011f6e9f2eb07084d327fe95ae/> `gix tree entries` with rev-spec support.
   Previously it wanted a tree-id, now it can derive it itself.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 7 calendar days.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'rename-tracking' ([`550144a`](https://github.com/GitoxideLabs/gitoxide/commit/550144a5fd37d501d86f4b1c4db2948d951d1c93))
    - `ein tool query` - a git analytics engine. ([`f8cc623`](https://github.com/GitoxideLabs/gitoxide/commit/f8cc62376bd6e940be4c41e84ccbfce111e64e39))
    - `gix tree entries` with rev-spec support. ([`49520d1`](https://github.com/GitoxideLabs/gitoxide/commit/49520d1f03ee1d011f6e9f2eb07084d327fe95ae))
    - Fix journey tests; improve panic handling when --progress is used. ([`571121c`](https://github.com/GitoxideLabs/gitoxide/commit/571121ccceaab25323bc6cd1ed2e2a1560289a57))
</details>

## 0.22.1 (2023-02-17)

### Bug Fixes

 - <csr-id-73372d0bcdca8cf201f057ebee8d67a8e4752625/> re-enable local-time support for all binaries
   The `time` improved the way one can opt-in to potential unsoundness
   in their local time support, forcing the application to make that
   decision at runtime.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Re-enable local-time support for all binaries ([`73372d0`](https://github.com/GitoxideLabs/gitoxide/commit/73372d0bcdca8cf201f057ebee8d67a8e4752625))
</details>

## 0.22.0 (2023-02-17)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 7 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Adjust to renaming for `git-protocol` to `gix-protocol` ([`823795a`](https://github.com/GitoxideLabs/gitoxide/commit/823795addea3810243cab7936cd8ec0137cbc224))
    - Adjust to renaming of `git-features` to `gix-features` ([`e2dd68a`](https://github.com/GitoxideLabs/gitoxide/commit/e2dd68a417aad229e194ff20dbbfd77668096ec6))
    - Merge branch 'rename-crates' ([`6461c3d`](https://github.com/GitoxideLabs/gitoxide/commit/6461c3da4d6daee857606d94294c3f87fc36965a))
    - Rename `git-repository` to `gix` ([`7bed2a9`](https://github.com/GitoxideLabs/gitoxide/commit/7bed2a96604397fa990f427b1a970ddeb6f09f1c))
    - Merge branch 'rename-tracking' ([`9e7d792`](https://github.com/GitoxideLabs/gitoxide/commit/9e7d79273487abfcb99ed2d439c475a659cd25e6))
    - Keep track of all keys relevant renaming when merging and obtaining status ([`0f2ed59`](https://github.com/GitoxideLabs/gitoxide/commit/0f2ed59b928ef3f4d6cd5a0cd0380504891d92f2))
</details>

## 0.21.0 (2023-02-09)

<csr-id-1d9a5e925890883e9e712db14ac82411ad2fdfc3/>

### Chore

 - <csr-id-1d9a5e925890883e9e712db14ac82411ad2fdfc3/> upgrade to clap 4.1

### New Features

 - <csr-id-b81f650b476354c5bd030aa03b9a723d9d7a0970/> `gix clone <url>` is now permitted without specifying a destination directory.
   Note that the implementation doesn't take into account potential redirects and renames
   as it's implemented only with the first URL it sees (not the redirected ones).
 - <csr-id-38ae61a805bd8cca5df8d1c1dcf3a8a0f9c85f5a/> make more HTTP options available
   - `http.schannelCheckRevoke`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 47 calendar days.
 - 49 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#683](https://github.com/GitoxideLabs/gitoxide/issues/683)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#683](https://github.com/GitoxideLabs/gitoxide/issues/683)**
    - `gix clone <url>` is now permitted without specifying a destination directory. ([`b81f650`](https://github.com/GitoxideLabs/gitoxide/commit/b81f650b476354c5bd030aa03b9a723d9d7a0970))
 * **Uncategorized**
    - Make fmt ([`e22080e`](https://github.com/GitoxideLabs/gitoxide/commit/e22080e4a29d0bad15a99d565a5e3e304a8743ec))
    - Merge branch 'adjustments-for-cargo' ([`7bba270`](https://github.com/GitoxideLabs/gitoxide/commit/7bba2709488b7eb999b8136dbab03af977241678))
    - `gix progress` now usees the configuration tree as basis for data. ([`ea76bf5`](https://github.com/GitoxideLabs/gitoxide/commit/ea76bf5ba1b06bcf94bcadf5129ce265c90144d3))
    - Thanks clippy ([`f7b8794`](https://github.com/GitoxideLabs/gitoxide/commit/f7b879448258a058b8607916da0ddf13b83896d9))
    - Upgrade to clap 4.1 ([`1d9a5e9`](https://github.com/GitoxideLabs/gitoxide/commit/1d9a5e925890883e9e712db14ac82411ad2fdfc3))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`9e04685`](https://github.com/GitoxideLabs/gitoxide/commit/9e04685dd3f109bfb27663f9dc7c04102e660bf2))
    - Upgrade `tabled` ([`7aed326`](https://github.com/GitoxideLabs/gitoxide/commit/7aed326e706b13ae88f98880de055201b2a869cc))
    - Merge branch 'gix-clone-improvements' ([`76c99f3`](https://github.com/GitoxideLabs/gitoxide/commit/76c99f3005f1b0031921b536f5d268715e41f3c8))
    - Merge branch 'adjustments-for-cargo' ([`f8c562a`](https://github.com/GitoxideLabs/gitoxide/commit/f8c562a559e6dc3377583cc7200585dad7c3d481))
    - Adapt to changes in git-repository and git-transport ([`d336368`](https://github.com/GitoxideLabs/gitoxide/commit/d336368ea2b6e5918b709220522cf1509ef30be2))
    - Update planned usage of `ssh` related configuration variables. ([`3f91088`](https://github.com/GitoxideLabs/gitoxide/commit/3f910880fdf691b59460eea272772c7aff284716))
    - Merge branch 'adjustments-for-cargo' ([`d821fc5`](https://github.com/GitoxideLabs/gitoxide/commit/d821fc5b4ef4ba606f2b6bb68b66f7260a0205dc))
    - Make more HTTP options available ([`38ae61a`](https://github.com/GitoxideLabs/gitoxide/commit/38ae61a805bd8cca5df8d1c1dcf3a8a0f9c85f5a))
</details>

## 0.20.0 (2022-12-22)

This release also fixes compatibility issues that formerly prevented to fetch or clone form `https://googlesource.com`.

### Changed

 - <csr-id-a4ac9cf3e667a3059e33aac8188150529578622d/> represent `GIT_(COMMITTER|AUTHOR)_(NAME|EMAIL|DATE)` with git configuration.
   That way it becomes more obvious where values are coming from.

### New Features

 - <csr-id-98143699bb9481b010e21647f64dcb8a74bd80ad/> auto-enabled verbosity for `gix fetch/clone` and add `--no-verbose`.
   I found myself always adding (and having to remember to add) the `-v` flag
   for long-running operations so these should be able to default to a
   higher verbosity level.
   
   To counter that, there is a new `--no-verbose` flag to turn that off.
 - <csr-id-aeb4a1d5cb76316058c7d687e26f5c7db351c09c/> add `--strict` option to enforce strict checking of configuration.
 - <csr-id-8c9c243fc574dc17b92a8e2025fbd8efdf2833da/> `gix odb stats` to calculate statistics on the object database.
   This includes the amount of objects along with a listing of where they are
   located.
 - <csr-id-e2b8c5dce7185b5fa194b90f32e642e5c9d1227f/> `gix clone --no-tags` support.
   This is the same as `git clone --no-tags`.

### Changed (BREAKING)

 - <csr-id-49f39d6bb487c0254176a5082f2c7851b83952a1/> `open::ReplacementObjects` is removed in favor of two custom git-configuration flags.
   Now it's possible to map the environment variables `GIT_REPLACE_REF_BASE` and `GIT_NO_REPLACE_OBJECTS`
   to custom git configuration keys which can also be set, namely `gitoxide.odb.replaceObjectsRefBase`
   and `gitoxide.odb.noReplaceObjects`.
   
   Along with the possibility of disabling the usage of `GIT_` prefixed environment variables one
   reaches the previous level of control without making object replacement a special case.

### New Features (BREAKING)

 - <csr-id-becbd8d896a1663f1607be4e86e632773e926f1f/> represent object cache configuration like `GITOXIDE_PACK_CACHE_MEMORY` in git-configuration.
   That way there is a unified system for how to set values, which may be overridable by configuration
   variables or not.
   
   With this changes, the explicit application of environment variables for setting the cache
   isn't required anymore as everything happens using git-configuration, and automatically,
   while providing full control like before.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release over the course of 30 calendar days.
 - 30 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix label for `ein t estimate-hours` command ([`4c93de1`](https://github.com/GitoxideLabs/gitoxide/commit/4c93de113898c6ff6ae1da3ea38d646626c86205))
    - Merge branch 'read-header' ([`3d01252`](https://github.com/GitoxideLabs/gitoxide/commit/3d0125271ec7bd606734bd74757a7e31a18c7ce5))
    - `gix odb stats` to calculate statistics on the object database. ([`8c9c243`](https://github.com/GitoxideLabs/gitoxide/commit/8c9c243fc574dc17b92a8e2025fbd8efdf2833da))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - `gix clone --no-tags` support. ([`e2b8c5d`](https://github.com/GitoxideLabs/gitoxide/commit/e2b8c5dce7185b5fa194b90f32e642e5c9d1227f))
    - Update `gix progress` to inform about `remote.<name>.tagOpt` ([`3ebb340`](https://github.com/GitoxideLabs/gitoxide/commit/3ebb3405f265fbd78a89c01bd30c3ad34036ef84))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Upgrade to prodash v22 for API improvements ([`77ab98d`](https://github.com/GitoxideLabs/gitoxide/commit/77ab98dd41c3849b674d8b3794ef29219ca1447d))
    - Auto-enabled verbosity for `gix fetch/clone` and add `--no-verbose`. ([`9814369`](https://github.com/GitoxideLabs/gitoxide/commit/98143699bb9481b010e21647f64dcb8a74bd80ad))
    - Switch from `atty` to `is-terminal` ([`7304bc1`](https://github.com/GitoxideLabs/gitoxide/commit/7304bc1c0efaad64a39520962072343ef02f6c25))
    - Adapt to changes in `git-repository` ([`c4f68bf`](https://github.com/GitoxideLabs/gitoxide/commit/c4f68bf775b854625d901fe0bfcbdd38f656d408))
    - Represent object cache configuration like `GITOXIDE_PACK_CACHE_MEMORY` in git-configuration. ([`becbd8d`](https://github.com/GitoxideLabs/gitoxide/commit/becbd8d896a1663f1607be4e86e632773e926f1f))
    - Represent `GIT_(COMMITTER|AUTHOR)_(NAME|EMAIL|DATE)` with git configuration. ([`a4ac9cf`](https://github.com/GitoxideLabs/gitoxide/commit/a4ac9cf3e667a3059e33aac8188150529578622d))
    - `open::ReplacementObjects` is removed in favor of two custom git-configuration flags. ([`49f39d6`](https://github.com/GitoxideLabs/gitoxide/commit/49f39d6bb487c0254176a5082f2c7851b83952a1))
    - Apply related environment variables as config overrides ([`9441c26`](https://github.com/GitoxideLabs/gitoxide/commit/9441c261bcae61d1d1e674b5e783f38b0471be29))
    - Adapt to changes in `git-repository` ([`f1a4c8b`](https://github.com/GitoxideLabs/gitoxide/commit/f1a4c8b42ed8c94e7fe3a61eb222cf6b0886f4ee))
    - Update progress of http.proxyAuthMethod ([`872dc1a`](https://github.com/GitoxideLabs/gitoxide/commit/872dc1ab43ce626b4166dae3dc8bddf8e85c9409))
    - Add `--strict` option to enforce strict checking of configuration. ([`aeb4a1d`](https://github.com/GitoxideLabs/gitoxide/commit/aeb4a1d5cb76316058c7d687e26f5c7db351c09c))
    - Don't lock stdout/stderr as it will deadlock on dbg-printing ([`62cae0e`](https://github.com/GitoxideLabs/gitoxide/commit/62cae0e6bfe8113c0225152a896338017c8de474))
    - Adapt to changes in `git-config` ([`1c2e755`](https://github.com/GitoxideLabs/gitoxide/commit/1c2e755e517b0f9fe8671187f5c30076ce43a3c9))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
</details>

## 0.19.0 (2022-11-21)

### New Features

 - <csr-id-3ddbd2de369b521fa3f21935f10fe9c248840893/> Make `reqwest` TLS backend configuration easy.
   We provide the choice of `native-tls` or `rust-tls`. If none is
   provided, the user can configure on their on similar to how it's done
   in `gix`.
   
   Please note that a choice now has to be made or HTTPS will not be
   available, so use one of…
   
   * blocking-http-transport-reqwest-rust-tls
* blocking-http-transport-reqwest-native-tls

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Make `reqwest` TLS backend configuration easy. ([`3ddbd2d`](https://github.com/GitoxideLabs/gitoxide/commit/3ddbd2de369b521fa3f21935f10fe9c248840893))
</details>

## 0.18.0 (2022-11-17)

This releases fixes `gix fetch` so that it is able to clone or fetch `pytorch` or other repositories on case-insensitive file systems. 

It's also an attempt to trigger CI to build binary releases.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 9 calendar days.
 - 10 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'http-config' ([`665b53e`](https://github.com/GitoxideLabs/gitoxide/commit/665b53e1c2e1de65fafa28b669f58977868bbc81))
    - Document that histogram is now the default diff algorithm ([`c76572b`](https://github.com/GitoxideLabs/gitoxide/commit/c76572b3662776b524a7e4a1fd96d2eaa22a560f))
    - Introduce new `gitoxide.http.connectTimeout` for more control for git clients ([`2ab80e4`](https://github.com/GitoxideLabs/gitoxide/commit/2ab80e4c95a7bf3c7e56bb5a95ac78ac930fc9ee))
    - Keep track of `no_proxy` environment variable support ([`f0625de`](https://github.com/GitoxideLabs/gitoxide/commit/f0625de13073de4767881ed0398d0cd2791b0ad2))
    - Update progress ([`3d9fb6c`](https://github.com/GitoxideLabs/gitoxide/commit/3d9fb6c095a272d5ddf6c5b6ce96820bc9d59cbb))
    - Don't forget to update 'progress' ([`0ec5220`](https://github.com/GitoxideLabs/gitoxide/commit/0ec5220fea50f06eb61bafff525111ce2435c994))
    - Update progress with gitoxide.userAgent ([`1c012f4`](https://github.com/GitoxideLabs/gitoxide/commit/1c012f4c2e05e1f565fc51fffee2f7d278e5a7de))
    - Plan for user agent string configuration ([`f5499a5`](https://github.com/GitoxideLabs/gitoxide/commit/f5499a55ed0230e2852b41b54648003e3d6cb859))
    - Keep track of http related configuration keys. ([`1afaebd`](https://github.com/GitoxideLabs/gitoxide/commit/1afaebdcae977af8a9a0f0788ec754746d6d05bb))
</details>

## 0.17.0 (2022-11-06)

### New Features

 - <csr-id-e973dfeaf17ca63385496202e9fdcdd912e20f42/> `gix remote ref-map --show-unmapped-remote-refs`.
   That way it's more obvious to see what was filtered out by ref-specs.
   
   It's also great to validate that server-side filtering via ref-prefix
   will not send refs that are referred to by symbolic refs that are
   not filtered out. That should be fine as it's all about objects,
   it's just something to deal with as we may have to deal with symbolic
   refs that aren't in the set of refs the server sent to us.
 - <csr-id-b1edb9e3537df86669714f03666f4a88e0ac3709/> diff algorithm is controlled by git configuration `diff.algorithm`
 - <csr-id-20259da4ddf9fabfb2d2bd4e2274c0ed42bdb0e5/> `ein t hours` allows to specify the amount of worker threads.

### Bug Fixes

 - <csr-id-3a053284cfefe27873dcc5b4f74d63a560bb5d41/> collect `stderr` and print it afterwards to avoid intersection with line progress.
   Previously it would happen that stderr would be printed directly and mix
   with the line progress (as in `-v`) which also prints to stderr.
   
   Now errors are collected and output at the end once the line renderer
   was already shutdown.

### Changed (BREAKING)

 - <csr-id-3a0fb1b45c757add49677450836c0aaf6179a2b5/> remote `lock_mode` from all methods dealing with reference edits.
   It is now read from `core.filesRefLockTimeout` accordingly.

### New Features (BREAKING)

 - <csr-id-92bbe335688e4c8e96061663e71a599022f7b96f/> remove `gix remote --url` in favor of determining the intention similar to `git fetch`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 65 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#536](https://github.com/GitoxideLabs/gitoxide/issues/536)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Keep track of several branch related keys ([`443a75a`](https://github.com/GitoxideLabs/gitoxide/commit/443a75a6098968995bed644c84ed8ce9d8ed0c34))
    - `gix remote ref-map --show-unmapped-remote-refs`. ([`e973dfe`](https://github.com/GitoxideLabs/gitoxide/commit/e973dfeaf17ca63385496202e9fdcdd912e20f42))
    - Update progress as we now respect `diff.algorithm` ([`30d32e7`](https://github.com/GitoxideLabs/gitoxide/commit/30d32e7255986e6e7ec92d55c86b747d7486e183))
    - Apply configuration overrides to newborn repo during clone ([`c8ef759`](https://github.com/GitoxideLabs/gitoxide/commit/c8ef759923a3c980b5a23c868f38804eccbc0fbc))
    - Collect `stderr` and print it afterwards to avoid intersection with line progress. ([`3a05328`](https://github.com/GitoxideLabs/gitoxide/commit/3a053284cfefe27873dcc5b4f74d63a560bb5d41))
    - Reduce verbosity of `clone` and print once entire clone is done ([`9a476df`](https://github.com/GitoxideLabs/gitoxide/commit/9a476df519bccfc6bcda8bd02aa4c1573a2691e7))
    - First rough sketch of `gix clone` ([`23a5e8b`](https://github.com/GitoxideLabs/gitoxide/commit/23a5e8b658c5642c3f3060e013fd0eab06cbf027))
    - Progress totals (planned, devitaion) ([`63947ae`](https://github.com/GitoxideLabs/gitoxide/commit/63947ae77ecb5952d7b5da5c6695f002bb7a7c4c))
    - Prepare attribute-group setup as far as possible. ([`f5e2eeb`](https://github.com/GitoxideLabs/gitoxide/commit/f5e2eebe9560f664f044b82ffa0cd19fd0df311f))
    - Update usage of `core.checkState` ([`8b2aba1`](https://github.com/GitoxideLabs/gitoxide/commit/8b2aba1d2baf42a3578ee8c9ce89dbe29d4d3f4c))
    - Update progress based on filesystem config usage ([`de13c66`](https://github.com/GitoxideLabs/gitoxide/commit/de13c66ade29635ff585241fec2783cf972dd5c3))
    - Inform about even more keys in `gix progress` ([`0e9bd41`](https://github.com/GitoxideLabs/gitoxide/commit/0e9bd4133fe33382d6d8c42ca1280601582a1a17))
    - Plan more core filesystem attributes ([`d10a82e`](https://github.com/GitoxideLabs/gitoxide/commit/d10a82e86a461e72b9e5339a180f8adb34721e3e))
    - Update `init.defaultBranch` - now used by custom initialization. ([`4d6d7bb`](https://github.com/GitoxideLabs/gitoxide/commit/4d6d7bbf5c3d8159535e5756da082ca493e9a66a))
    - Support for handshake information in `gix fetch` ([`c47dcc6`](https://github.com/GitoxideLabs/gitoxide/commit/c47dcc69e54c635650b540c590131dbe7d32d05b))
    - Fix build ([`d034882`](https://github.com/GitoxideLabs/gitoxide/commit/d03488211ec5bd186f6d274c55cd96cfd9d119d5))
    - Remove `gix remote --url` in favor of determining the intention similar to `git fetch` ([`92bbe33`](https://github.com/GitoxideLabs/gitoxide/commit/92bbe335688e4c8e96061663e71a599022f7b96f))
    - Support for `--url` for arbitrary urls when fetching ([`8c7351c`](https://github.com/GitoxideLabs/gitoxide/commit/8c7351c4c50517b3ccc3479f2a7a020bc607bf24))
    - Frame for `gix fetch` ([`5b72d27`](https://github.com/GitoxideLabs/gitoxide/commit/5b72d2708889dc388facd9cbc61e5bfa5403e003))
    - Update progress information to include packedRefsTimeout ([`fd18320`](https://github.com/GitoxideLabs/gitoxide/commit/fd18320561e05431796aa4044c0a2b0605c9ca9d))
    - Remote `lock_mode` from all methods dealing with reference edits. ([`3a0fb1b`](https://github.com/GitoxideLabs/gitoxide/commit/3a0fb1b45c757add49677450836c0aaf6179a2b5))
    - Add information about planned lock timeout support (from configuration) ([`7076891`](https://github.com/GitoxideLabs/gitoxide/commit/7076891fb1b44cd442928f7e56f53f4b085e7a11))
    - Add remotes.<group> as planned feature for remotes ([`3c188b2`](https://github.com/GitoxideLabs/gitoxide/commit/3c188b2253bfb6d47394718425eef2d1a0547949))
    - Slightly nicer styling of config keys ([`eade88f`](https://github.com/GitoxideLabs/gitoxide/commit/eade88f8ebc638f504881e8bbbd60d42a5a3d9be))
    - Complete listing of records based on current usage, probably ([`6abd5a4`](https://github.com/GitoxideLabs/gitoxide/commit/6abd5a4e1daaf91fd109acb714057a82f67fa076))
    - Add more records ([`5c0d0ab`](https://github.com/GitoxideLabs/gitoxide/commit/5c0d0ab66d46a8d093ca0b5451996099a27ef1dd))
    - Add tabled for nicer printing ([`65e6496`](https://github.com/GitoxideLabs/gitoxide/commit/65e64964c7cd151e53e5a7d4b9ba8fabda1c0e16))
    - Refactor ([`b42b08a`](https://github.com/GitoxideLabs/gitoxide/commit/b42b08afafd904ff2adb1f00688437357532193a))
    - Add support for more types of configurations ([`317e02a`](https://github.com/GitoxideLabs/gitoxide/commit/317e02a5900189ec7f9f3c2bb27d5696178d7869))
    - A very first version of `gix progress show` ([`92e082a`](https://github.com/GitoxideLabs/gitoxide/commit/92e082a288cf5e48caa205a4c7bd1ced025fea46))
 * **[#536](https://github.com/GitoxideLabs/gitoxide/issues/536)**
    - `ein t hours` allows to specify the amount of worker threads. ([`20259da`](https://github.com/GitoxideLabs/gitoxide/commit/20259da4ddf9fabfb2d2bd4e2274c0ed42bdb0e5))
 * **Uncategorized**
    - Merge branch 'write-sparse-index' ([`ba17db0`](https://github.com/GitoxideLabs/gitoxide/commit/ba17db03e4e832db724ab3e08e3df05eb61dd401))
    - Plan `index.version` for when we can write V4 indices. ([`da96d34`](https://github.com/GitoxideLabs/gitoxide/commit/da96d34bd609776f3c8590171ddbf894f2205ae5))
    - Notes about the split-index extension. ([`ad44982`](https://github.com/GitoxideLabs/gitoxide/commit/ad449822c952a7ed5e6124216e7d7668f0f5873f))
    - Take note of additional options for promisor packs and partial clone filters ([`1ec27f8`](https://github.com/GitoxideLabs/gitoxide/commit/1ec27f86113aa45a1540441a21152988b3911015))
    - Make note of `extension.worktreeConfig` ([`fe1e646`](https://github.com/GitoxideLabs/gitoxide/commit/fe1e6467296ff93c23fe191ed5a265348ce930dc))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Add `init.templateDir` to `gix progress` ([`9fab050`](https://github.com/GitoxideLabs/gitoxide/commit/9fab0501f5017e43132138579238a46726a7348f))
    - Make fmt ([`ea2136b`](https://github.com/GitoxideLabs/gitoxide/commit/ea2136b065979cecb3a1fdbf7b20ed7514128d9a))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Diff algorithm is controlled by git configuration `diff.algorithm` ([`b1edb9e`](https://github.com/GitoxideLabs/gitoxide/commit/b1edb9e3537df86669714f03666f4a88e0ac3709))
    - Merge branch 'main' into gix-clone ([`fa27570`](https://github.com/GitoxideLabs/gitoxide/commit/fa27570f491388cce6137af44330d76870d07202))
    - Merge branch 'imra-diff' ([`f53f942`](https://github.com/GitoxideLabs/gitoxide/commit/f53f9426f206686b30abd73a201a92b1405e782d))
    - Adapt to changes in `git-diff` for a 2x speedup when calculating line changes ([`296f3b6`](https://github.com/GitoxideLabs/gitoxide/commit/296f3b6ee29d5e628a19d56db80ba8736223e226))
    - Update `gix progress` records ([`b05a2e7`](https://github.com/GitoxideLabs/gitoxide/commit/b05a2e7939df0c25da8186395e7eceda9e1baa9b))
    - Merge branch 'main' into gix-clone ([`de4fe06`](https://github.com/GitoxideLabs/gitoxide/commit/de4fe06202906ea5c62e667826b42cf7b57b1ff0))
    - Mark the upcoming usage of init.defaultBranch. ([`6225f35`](https://github.com/GitoxideLabs/gitoxide/commit/6225f35398bc494ad74da342c4ebbe0487b106f8))
    - Merge branch 'fix-gix-index-from-tree' ([`da5f63c`](https://github.com/GitoxideLabs/gitoxide/commit/da5f63cbc7506990f46d310f8064678decb86928))
    - Write index without output path to memory only. ([`c8d0345`](https://github.com/GitoxideLabs/gitoxide/commit/c8d03454f1b2c18876cc8935e0afcea611cb8647))
    - Merge branch 'gix-index-from-tree' ([`8c24386`](https://github.com/GitoxideLabs/gitoxide/commit/8c24386f1874cd94f78fefbe434963f772878b1f))
    - Refactor ([`67f2247`](https://github.com/GitoxideLabs/gitoxide/commit/67f224785193a5269cf65963fd21b21b723d485e))
    - Refactor ([`01ab5ca`](https://github.com/GitoxideLabs/gitoxide/commit/01ab5cac23427a9c3b7f153201627eb8c8898e96))
    - Update with various configuration variables relevant to checking out worktrees ([`09d767a`](https://github.com/GitoxideLabs/gitoxide/commit/09d767abd371a268a9fa475956be09ddda6b42ea))
    - Merge branch 'clone' ([`507dc7e`](https://github.com/GitoxideLabs/gitoxide/commit/507dc7e706cb3c9d89d048b3aff5df239a9b6788))
    - Update usage of clone related configuration ([`1a1e862`](https://github.com/GitoxideLabs/gitoxide/commit/1a1e862b2e8cd88f5f6fbb9d86f618761bb71ef1))
    - Update progress with intended uses of `clone.` variables ([`8b804a3`](https://github.com/GitoxideLabs/gitoxide/commit/8b804a31cb20a5264311f0b6ba02f857bea933ad))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Make fmt ([`53acf25`](https://github.com/GitoxideLabs/gitoxide/commit/53acf2565743eff7cead7a42011107b2fc8d7e0e))
    - Merge branch 'fetch-pack' ([`f47c891`](https://github.com/GitoxideLabs/gitoxide/commit/f47c89129732bcb06fe76a4696fe38ab1151fb0c))
    - Merge branch 'fetch-pack' ([`3c49400`](https://github.com/GitoxideLabs/gitoxide/commit/3c49400809c7c2120f4ce704c19a0421545b5acd))
    - Update docs ([`c5c0ac5`](https://github.com/GitoxideLabs/gitoxide/commit/c5c0ac50616957b5b0dcaf530d294abe63e3b4c7))
    - Input id can now be a commit or tree as prefix or full object id ([`8ef3fcb`](https://github.com/GitoxideLabs/gitoxide/commit/8ef3fcbc416b2a53ead7d6ba36991a3d035f5f22))
    - Thanks clippy ([`8dadd70`](https://github.com/GitoxideLabs/gitoxide/commit/8dadd70f8b7db1794652805c6238763886a8570d))
    - Merge branch 'fix-odb-race' ([`b862fc5`](https://github.com/GitoxideLabs/gitoxide/commit/b862fc52dd2409e912c892c7f428a571f565b64a))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
</details>

## 0.16.0 (2022-09-20)

### Changed

 - <csr-id-3c7c9a735f5771ef787cbc86b46cbafc9226f4d6/> `ein tool hours -s` was split into `-f|--file-stats` and `-l|line-stats`.
   That way more information is generated at increasingly high costs.

### New Features

 - <csr-id-28c4cae70aab2bd5b479961fcc6ee91ff80f651b/> `ein tool hours --stat` to collect additional statistics per author.
   Note that these are expensive and unconditionally use threads to speed
   up these computations.
 - <csr-id-5d0332f51c63c5456a28c8f3f466ad805b2e0626/> `ein tool hours -b` ignores bots.
   For now it only considers bots with names containing `[bot]`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 20 calendar days.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Refactor ([`11851f3`](https://github.com/GitoxideLabs/gitoxide/commit/11851f334f642e7bd69bcbfc7ad4f1990fc326ba))
    - Option to print server information about the connection ([`4720666`](https://github.com/GitoxideLabs/gitoxide/commit/4720666c8bfdaa3acc5c832b44755d4b4f86e16e))
    - Show fixes as well ([`2237495`](https://github.com/GitoxideLabs/gitoxide/commit/2237495d82624b39bf75c6430549424a5e36b8bb))
    - Correct printing of tag information (even though it doesn't look great) ([`f4d8198`](https://github.com/GitoxideLabs/gitoxide/commit/f4d8198992b4c45f64d81e20f40a1cad69883162))
    - Wire up the `ref-map` sub-command. ([`94c2b78`](https://github.com/GitoxideLabs/gitoxide/commit/94c2b785f892f85503b8927c7fa98ae99d677be7))
    - Select `gix` commands will now load the git installation configuration ([`23d2dec`](https://github.com/GitoxideLabs/gitoxide/commit/23d2dec375305c39d472c4f8ff764274dd033f6b))
    - Refactor ([`7abc0a3`](https://github.com/GitoxideLabs/gitoxide/commit/7abc0a39205b9f374c90c4750fe6cc9b3749d7b9))
    - Add sketch of `gix credential` ([`642e21f`](https://github.com/GitoxideLabs/gitoxide/commit/642e21fc58d8d4b68cba3067c88d44c019ec4ace))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - `ein tool hours -s` was split into `-f|--file-stats` and `-l|line-stats`. ([`3c7c9a7`](https://github.com/GitoxideLabs/gitoxide/commit/3c7c9a735f5771ef787cbc86b46cbafc9226f4d6))
    - Upgrade to prodash 20.1 for `Progress::counter()` feature ([`0ac4a2c`](https://github.com/GitoxideLabs/gitoxide/commit/0ac4a2c514aeb94d8e90ce28ae7a0e0350c21ab2))
    - `ein tool hours --stat` to collect additional statistics per author. ([`28c4cae`](https://github.com/GitoxideLabs/gitoxide/commit/28c4cae70aab2bd5b479961fcc6ee91ff80f651b))
 * **Uncategorized**
    - Merge branch 'hours-upgrade' ([`26489d1`](https://github.com/GitoxideLabs/gitoxide/commit/26489d14472b840b36696435c22d9077f7ab323d))
    - Use rev-specs instead of ref-names ([`cf7182e`](https://github.com/GitoxideLabs/gitoxide/commit/cf7182e3390c03df97c10cd101440f7aa8874904))
    - `ein tool hours -b` ignores bots. ([`5d0332f`](https://github.com/GitoxideLabs/gitoxide/commit/5d0332f51c63c5456a28c8f3f466ad805b2e0626))
    - Merge branch 'filter-refs' ([`3773b92`](https://github.com/GitoxideLabs/gitoxide/commit/3773b92b8372c9a40a74d281149ca65b057a7da9))
    - Implement `gix index from-tree` ([`2fbd3df`](https://github.com/GitoxideLabs/gitoxide/commit/2fbd3df89373eea5d6268fa47e046c8ebad8bba9))
    - Make fmt ([`535e967`](https://github.com/GitoxideLabs/gitoxide/commit/535e967666c6da657ff1b7eff7c64ab27cafb182))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into filter-refs-by-spec ([`56ba481`](https://github.com/GitoxideLabs/gitoxide/commit/56ba481f4c48f74f10397feb1b6dc3d7dd3704fb))
    - A basic implementation of rev-list without anything fancy ([`791dd66`](https://github.com/GitoxideLabs/gitoxide/commit/791dd666430fe0586c7db75b352487a72d3789e7))
</details>

## 0.15.0 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

 - <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-45a30f0f31a99cda5cf105408e9c3905f43071f2/> Support for `-c/--config` in `gix`
 - <csr-id-5d6d5ca305615568dfedbcc10ea86294c0a0472d/> `gix remote refs` to list all remote references of a suitable remote.
   This takes into account either a named remote, or the remote associated
   with the current branch, or the default remote it could deduce or obtain
   from the configuration.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs

### Changed (BREAKING)

 - <csr-id-dda995790c260131048484a11e66185b9c841311/> remove `gix free remote ref-list` in favor of `gix remote refs`
   The functionality is the same, but the latter is built on top of a
   repository which is slightly less flexible, but preferable over
   maintaining a non-repo version.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 5 calendar days.
 - 6 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#450](https://github.com/GitoxideLabs/gitoxide/issues/450)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Support for -c CLI config overrides in `gix config`. ([`19c1746`](https://github.com/GitoxideLabs/gitoxide/commit/19c1746cefca9bc76537637ec99634eb4cf66a92))
    - Remove `gix free remote ref-list` in favor of `gix remote refs` ([`dda9957`](https://github.com/GitoxideLabs/gitoxide/commit/dda995790c260131048484a11e66185b9c841311))
    - Support for `-c/--config` in `gix` ([`45a30f0`](https://github.com/GitoxideLabs/gitoxide/commit/45a30f0f31a99cda5cf105408e9c3905f43071f2))
    - Refactor ([`e0be6e9`](https://github.com/GitoxideLabs/gitoxide/commit/e0be6e9558add3255de63f3785306daace2707a6))
    - Add support for passing urls directly to bypass all remote repository logic. ([`df3cf18`](https://github.com/GitoxideLabs/gitoxide/commit/df3cf18a6ac1e4f35f6d11d62184a43722397bbe))
    - `gix remote refs` to list all remote references of a suitable remote. ([`5d6d5ca`](https://github.com/GitoxideLabs/gitoxide/commit/5d6d5ca305615568dfedbcc10ea86294c0a0472d))
    - Try to use maybe async for the simplest of possibly blocking remote interactions ([`db4df25`](https://github.com/GitoxideLabs/gitoxide/commit/db4df250d7e58518015bed0b9a1e3391b209cb29))
    - Basic parsing of `gix remote refs` without any implementation. ([`f8f1249`](https://github.com/GitoxideLabs/gitoxide/commit/f8f124943f73bacf816c6d0055f0b66659fd3906))
 * **Uncategorized**
    - Merge branch 'remote-ls-refs' ([`39d585d`](https://github.com/GitoxideLabs/gitoxide/commit/39d585d9f9ac6f3ecf51359c8e37f0a50e21ed45))
    - Thanks clippy ([`bb6813a`](https://github.com/GitoxideLabs/gitoxide/commit/bb6813abf365728d9851ee205b2c25b925a0f06a))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
</details>

## 0.14.0 (2022-08-17)

### Changed

 - <csr-id-0235111a4fcc40c7b57d973bfce27a66eddea901/> Invert behaviour to `open::Options::strict_config()`, with lenient being the default.
   This means API users will get libgit2 behaviour but commands like `gix` can
   change options to emulate `git` behaviour.

### New Features

 - <csr-id-b83f6bdf7f8d8f5cef2a57fa3932b6a0e0988db1/> `--cat-file` flag for `gix rev parse` to cat instead of resolving.
 - <csr-id-e972aad020d3653a53b20fa6e535d5759e239a45/> `gix rev previous-branches` subcommand

### Changed (BREAKING)

 - <csr-id-edf73dd4db5b0f5d9309c95bf366e11ea6723885/> `ein tools` to `ein tool` for as it's more intuitive

### New Features (BREAKING)

 - <csr-id-c5846e05aa54f0601ac7b8e2e59bcf1ffaa305f1/> `gix rev resolve --explain`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 26 calendar days.
 - 26 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#427](https://github.com/GitoxideLabs/gitoxide/issues/427)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - `--cat-file` flag for `gix rev parse` to cat instead of resolving. ([`b83f6bd`](https://github.com/GitoxideLabs/gitoxide/commit/b83f6bdf7f8d8f5cef2a57fa3932b6a0e0988db1))
    - `gix rev resolve --explain` ([`c5846e0`](https://github.com/GitoxideLabs/gitoxide/commit/c5846e05aa54f0601ac7b8e2e59bcf1ffaa305f1))
    - `gix rev previous-branches` subcommand ([`e972aad`](https://github.com/GitoxideLabs/gitoxide/commit/e972aad020d3653a53b20fa6e535d5759e239a45))
    - Support for parsing multiple specs in one invocation ([`84b5448`](https://github.com/GitoxideLabs/gitoxide/commit/84b5448deb7b87f67a1b7461f00793e7ae33ef31))
    - Support overriding cache settings with environment variables in `gix` ([`b838202`](https://github.com/GitoxideLabs/gitoxide/commit/b8382026cb5b979a5c563ea40d1d8e483c1ca23a))
 * **Uncategorized**
    - Merge branch 'core-abbrev-handling' ([`dbaff13`](https://github.com/GitoxideLabs/gitoxide/commit/dbaff13eaf7ba5f9c0ee2c90ac9f17e6078cad9e))
    - Control which command is lenient or not. That way `gix-config` can be lenient. ([`6a9c58f`](https://github.com/GitoxideLabs/gitoxide/commit/6a9c58fde7ca4a52fa1c3225974a2019e7d93168))
    - Invert behaviour to `open::Options::strict_config()`, with lenient being the default. ([`0235111`](https://github.com/GitoxideLabs/gitoxide/commit/0235111a4fcc40c7b57d973bfce27a66eddea901))
    - Merge branch 'main' into remote-ls-refs ([`c4bf958`](https://github.com/GitoxideLabs/gitoxide/commit/c4bf9585d815bc342e5fb383336cc654280dd34f))
    - Merge branch 'index-write-refactor' ([`805f432`](https://github.com/GitoxideLabs/gitoxide/commit/805f432bf8e9d2dd9ede56caf959de386d5d80c7))
    - `ein tools` to `ein tool` for as it's more intuitive ([`edf73dd`](https://github.com/GitoxideLabs/gitoxide/commit/edf73dd4db5b0f5d9309c95bf366e11ea6723885))
    - Merge branch 'main' into write-index-v2 ([`a938986`](https://github.com/GitoxideLabs/gitoxide/commit/a938986877302c197d1aed087594c5605416fe5f))
    - Add aliases to make revision sub-commands more accessible ([`a6d79e3`](https://github.com/GitoxideLabs/gitoxide/commit/a6d79e38cb0dd7e87d00a098030bbcaa614f259d))
    - Merge branch 'rev-parse-delegate' ([`2f506c7`](https://github.com/GitoxideLabs/gitoxide/commit/2f506c7c2988477b0f97d272a9ac9ed47b236457))
    - Merge pull request #2 from SidneyDouw/main ([`ce885ad`](https://github.com/GitoxideLabs/gitoxide/commit/ce885ad4c3324c09c83751c32e014f246c748766))
    - Merge branch 'Byron:main' into main ([`9b9ea02`](https://github.com/GitoxideLabs/gitoxide/commit/9b9ea0275f8ff5862f24cf5a4ca53bb1cd610709))
    - Merge branch 'main' into rev-parse-delegate ([`6da8250`](https://github.com/GitoxideLabs/gitoxide/commit/6da82507588d3bc849217c11d9a1d398b67f2ed6))
    - Merge branch 'main' into pathspec ([`7b61506`](https://github.com/GitoxideLabs/gitoxide/commit/7b615060712565f515515e35a3e8346278ad770c))
    - Make fmt ([`47724c0`](https://github.com/GitoxideLabs/gitoxide/commit/47724c0edb382c036a3fc99884becfd2b0740d4b))
    - Merge branch 'kianmeng-fix-typos' ([`4e7b343`](https://github.com/GitoxideLabs/gitoxide/commit/4e7b34349c0a01ad8686bbb4eb987e9338259d9c))
    - Fix typos ([`e9fcb70`](https://github.com/GitoxideLabs/gitoxide/commit/e9fcb70e429edb2974afa3f58d181f3ef14c3da3))
</details>

## 0.13.0 (2022-07-22)

### New Features

 - <csr-id-eda39ec7d736d49af1ad9e2ad775e4aa12b264b7/> `gix config` with section and sub-section filtering.
 - <csr-id-d99453ebeb970ed493be236def299d1e82b01f83/> `gix config` lists all entries of all configuration files git considers.
   Filters allow to narrow down the output.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 47 commits contributed to the release over the course of 101 calendar days.
 - 108 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#331](https://github.com/GitoxideLabs/gitoxide/issues/331), [#427](https://github.com/GitoxideLabs/gitoxide/issues/427)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 3 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Allow reading patterns from stdin ([`0c597fe`](https://github.com/GitoxideLabs/gitoxide/commit/0c597fe78acdd5672b4535a7d82620c5f7f93649))
    - Add `--show-ignore-patterns` to `gix repo exclude query` ([`09f904b`](https://github.com/GitoxideLabs/gitoxide/commit/09f904b1f393f03176882d491d7fffcad4058b49))
    - Basic prefix support as well the first working version of `exclude query` ([`9cb8385`](https://github.com/GitoxideLabs/gitoxide/commit/9cb83859f9bb76f38ab5bbd0ae6d6f20a691e9e1))
    - Support for overrides on the command-line ([`7d98b21`](https://github.com/GitoxideLabs/gitoxide/commit/7d98b2196c130263ace4a948418affdd950302ed))
    - Fix build ([`cb56f12`](https://github.com/GitoxideLabs/gitoxide/commit/cb56f12ad83cf2932a068ef4fa0ca5ce4aa73e84))
    - Refactor ([`3ff991d`](https://github.com/GitoxideLabs/gitoxide/commit/3ff991d0ca0d63632fc5710680351840f51c14c3))
    - Frame for `gix repo exclude query` ([`a331314`](https://github.com/GitoxideLabs/gitoxide/commit/a331314758629a93ba036245a5dd03cf4109dc52))
    - Make fmt ([`50ff7aa`](https://github.com/GitoxideLabs/gitoxide/commit/50ff7aa7fa86e5e2a94fb15aab86470532ac3f51))
 * **[#331](https://github.com/GitoxideLabs/gitoxide/issues/331)**
    - Fix journey tests after `gix` restructuring ([`59b95c9`](https://github.com/GitoxideLabs/gitoxide/commit/59b95c94aacac174e374048b7d11d2c0984a19e0))
    - `gix config` with section and sub-section filtering. ([`eda39ec`](https://github.com/GitoxideLabs/gitoxide/commit/eda39ec7d736d49af1ad9e2ad775e4aa12b264b7))
    - `gix config` lists all entries of all configuration files git considers. ([`d99453e`](https://github.com/GitoxideLabs/gitoxide/commit/d99453ebeb970ed493be236def299d1e82b01f83))
    - Refactor ([`a437abe`](https://github.com/GitoxideLabs/gitoxide/commit/a437abe8e77ad07bf25a16f19ca046ebdaef42d6))
    - Move 'exclude' up one level and dissolve 'repo' subcommand ([`8e5b796`](https://github.com/GitoxideLabs/gitoxide/commit/8e5b796ea3fd760839f3c29a4f65bb42b1f3e893))
    - Move 'mailmap' up one level ([`5cf08ce`](https://github.com/GitoxideLabs/gitoxide/commit/5cf08ce3d04d635bbfee169cb77ce259efbf6bc3))
    - Move 'odb' up one level ([`0ed65da`](https://github.com/GitoxideLabs/gitoxide/commit/0ed65da9b66d4cc3c85d3b70fa4bc383c7a0d1a3))
    - Move 'tree' up one level ([`38a8350`](https://github.com/GitoxideLabs/gitoxide/commit/38a8350d75720a8455e9c55d12f7cdf4b1742e56))
    - Move 'commit' up one level ([`72876f1`](https://github.com/GitoxideLabs/gitoxide/commit/72876f1fd65efc816b704db6880ab881c89cff01))
    - Move 'verify' up one level ([`ac7d99a`](https://github.com/GitoxideLabs/gitoxide/commit/ac7d99ac42ff8561e81f476856d0bbe86b5fa627))
    - Move 'revision' one level up ([`c9c78e8`](https://github.com/GitoxideLabs/gitoxide/commit/c9c78e86c387c09838404c90de420892f41f4356))
    - Move 'remote' to 'free' ([`8967fcd`](https://github.com/GitoxideLabs/gitoxide/commit/8967fcd009260c2d32881866244ba673894775f2))
    - Move commitgraph to 'free' ([`f99c3b2`](https://github.com/GitoxideLabs/gitoxide/commit/f99c3b29cea30f1cbbea7e5855abfec3de6ca630))
    - Move index to 'free' ([`83585bd`](https://github.com/GitoxideLabs/gitoxide/commit/83585bdfccdc42b5307255b2d56d8cb12d4136cb))
    - Move 'pack' to 'free' ([`1cdecbc`](https://github.com/GitoxideLabs/gitoxide/commit/1cdecbc583ae412e7f25cade73b46e00a182125f))
    - Migrate mailmap to the new 'free' section ([`141c5f1`](https://github.com/GitoxideLabs/gitoxide/commit/141c5f1145f9d3864e2d879089c66c62f38a2b5d))
    - First step towards moving all repository-commands one level up. ([`f4e1810`](https://github.com/GitoxideLabs/gitoxide/commit/f4e1810fb711d57778be79c88f49aa583821abab))
    - Make obvious what plumbing and porcelain really are ([`faaf791`](https://github.com/GitoxideLabs/gitoxide/commit/faaf791cc960c37b180ddef9792dfabc7d106138))
    - Adjustments due to breaking changes in `git_path` ([`4420ae9`](https://github.com/GitoxideLabs/gitoxide/commit/4420ae932d5b20a9662a6d36353a27111b5cd672))
 * **[#427](https://github.com/GitoxideLabs/gitoxide/issues/427)**
    - Add rough but working version of `rev-parse` ([`f3f176d`](https://github.com/GitoxideLabs/gitoxide/commit/f3f176db42cef4036cc7c0ced1ee68f247424896))
    - Basic infrastructure for delegate implementation ([`d3c0bc6`](https://github.com/GitoxideLabs/gitoxide/commit/d3c0bc6e8d7764728f4e10500bb895152ccd0b0b))
    - Hookup explain command ([`1049b00`](https://github.com/GitoxideLabs/gitoxide/commit/1049b00eaa261a67f060eaca4eb50dcda831eafd))
    - Frame for `gix repo rev explain` ([`12e6277`](https://github.com/GitoxideLabs/gitoxide/commit/12e6277a65a6572a0e43e8324d2d1dfb23d0bb40))
 * **Uncategorized**
    - Merge branch 'gix-repo-config' ([`afecb63`](https://github.com/GitoxideLabs/gitoxide/commit/afecb6337dcf0fc51d5c74747c3c60fa06ae6346))
    - Thanks clippy ([`48b3f4a`](https://github.com/GitoxideLabs/gitoxide/commit/48b3f4a5077ba66d47482a80e505feb69e9ac9fc))
    - Merge pull request #1 from Byron/main ([`085e76b`](https://github.com/GitoxideLabs/gitoxide/commit/085e76b121291ed9bd324139105d2bd4117bedf8))
    - Make fmt ([`0700b09`](https://github.com/GitoxideLabs/gitoxide/commit/0700b09d6828849fa2470df89af1f75a67bfb27d))
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`0e9df36`](https://github.com/GitoxideLabs/gitoxide/commit/0e9df364c4cddf006b1de18b8d167319b7cc1186))
    - Use git_path::realpath in all places that allow it right now ([`229dc91`](https://github.com/GitoxideLabs/gitoxide/commit/229dc917fc7d9241b85e5818260a6fbdd3a5daaa))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Merge branch 'main' into git_includeif ([`05eb340`](https://github.com/GitoxideLabs/gitoxide/commit/05eb34023933918c51c03cf2afd774db89cc5a33))
    - Merge branch 'main' into msrv-for-windows ([`7cb1972`](https://github.com/GitoxideLabs/gitoxide/commit/7cb19729133325bdfacedf44cdc0500cbcf36684))
    - Make fmt ([`251b6df`](https://github.com/GitoxideLabs/gitoxide/commit/251b6df5dbdda24b7bdc452085f808f3acef69d8))
    - Merge branch 'worktree-stack' ([`98da8ba`](https://github.com/GitoxideLabs/gitoxide/commit/98da8ba52cef8ec27f705fcbc84773e5bacc4e10))
    - Thanks clippy ([`056e8d2`](https://github.com/GitoxideLabs/gitoxide/commit/056e8d26dc511fe7939ec87c62ef16aafd34fa9c))
    - Thanks clippy ([`fdec111`](https://github.com/GitoxideLabs/gitoxide/commit/fdec11135692b3503087b0a3245c12cc87554d67))
    - Merge branch 'main' into repo-status ([`4086335`](https://github.com/GitoxideLabs/gitoxide/commit/40863353a739ec971b49410fbc2ba048b2762732))
    - Merge branch 'worktree-stack' ([`e90d3fd`](https://github.com/GitoxideLabs/gitoxide/commit/e90d3fd0a9764511e6280596f21d3a0494ed7021))
</details>

## 0.12.0 (2022-04-05)

### New Features

 - <csr-id-7e99e6aeee9bf200a561d215c586301f5e4a8cbc/> Add `gix repo commit describe`
   It supports typical but basic flags mostly similar to the ones in git.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#298](https://github.com/GitoxideLabs/gitoxide/issues/298)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Use all tags by default, instead of requiring annotated tags ([`00c42ca`](https://github.com/GitoxideLabs/gitoxide/commit/00c42ca36e93a22f233fc1d3f9a1afc241fd4464))
    - Support for the --max-candidates flag ([`b9e6754`](https://github.com/GitoxideLabs/gitoxide/commit/b9e67540801f2630be8aa1acbfddfec4202360ac))
    - Reduce amount of max candidates, add --debug flag ([`c8c13e3`](https://github.com/GitoxideLabs/gitoxide/commit/c8c13e398671a21e96282547fc0e3bd445627e2f))
    - Add `gix repo commit describe` ([`7e99e6a`](https://github.com/GitoxideLabs/gitoxide/commit/7e99e6aeee9bf200a561d215c586301f5e4a8cbc))
    - A first sketch of the `gix repo describe` plumbing command ([`2d6ccef`](https://github.com/GitoxideLabs/gitoxide/commit/2d6ccefd5506d84ba14e3ff11c2af4cb107a386d))
</details>

## 0.11.0 (2022-04-03)

<csr-id-4d2d433e7e98ac42db858688edac06e68ee4d10d/>

Adapt to changes in `gix-features` which change `Send + Sync` to `Send + Clone`. This happens to allow non-sync implementations (i.e. thread-local), along with `Sync` ones
which usually are `Clone` too as they are passed by immutable reference (which is `Clone + Copy`).

### Refactor (BREAKING)

 - <csr-id-4d2d433e7e98ac42db858688edac06e68ee4d10d/> Remove light* features, add 'lean-async' in its place; remove termion support

### Changed (BREAKING)

 - <csr-id-bf04644ab75ed1969507f957dc8d4868790d462d/> remove `Option<impl Progress>` in favor of `impl Progress`
 - <csr-id-d851bede97801096d188ff6af06c98a79fe276db/> remove unnecessary `Arc` around `should_interrupt` flag
 - <csr-id-c2679a03358b9c19d63ed1af1cd57324c6381447/> remove Sha1 mentions in `index::verify::Mode::*` variants
   The hash is repository defined and not hard-coded
 - <csr-id-51bf03feaa94bebb26690dff92262b2134070a44/> Remove lean plumbing CLI

### Bug Fixes

 - <csr-id-57ca0456cf02073099bfd403f9155290af756ecd/> Collect all stdout messages in line renderer as well
   Otherwise the threaded line renderer will interfere with genuine
   program output.

### New Features

 - <csr-id-384ed665c7423feca1b1ee1f81db10867fa813a8/> `gix mailmap verify` command
 - <csr-id-70109bee679d33a5c5fb3a78a708b479684b03b1/> `ein find --debug` to learn why it is slow
 - <csr-id-00909619ff04e247aabc9ffe3c025f0064c3092d/> --counting-threads flag to configure amount of threads when counting
   The efficiency of multi-threaded counting is low per core, and despite
   some speedups might be desirable, one might not want to commit all cores
   to this amount of waste.
 - <csr-id-25da30f3652bd72c157e84439dd6e3957471fa08/> in-manifest and in-bin documentation of feature toggles
   Unfortunately, these don't show up on docs.rs due to it being a binary
   only crate. One could consider throwing in a lib just for good measure.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 63 commits contributed to the release over the course of 126 calendar days.
 - 165 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#215](https://github.com/GitoxideLabs/gitoxide/issues/215), [#263](https://github.com/GitoxideLabs/gitoxide/issues/263), [#266](https://github.com/GitoxideLabs/gitoxide/issues/266), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#287](https://github.com/GitoxideLabs/gitoxide/issues/287), [#289](https://github.com/GitoxideLabs/gitoxide/issues/289), [#293](https://github.com/GitoxideLabs/gitoxide/issues/293), [#298](https://github.com/GitoxideLabs/gitoxide/issues/298), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#366](https://github.com/GitoxideLabs/gitoxide/issues/366), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#215](https://github.com/GitoxideLabs/gitoxide/issues/215)**
    - Collect all stdout messages in line renderer as well ([`57ca045`](https://github.com/GitoxideLabs/gitoxide/commit/57ca0456cf02073099bfd403f9155290af756ecd))
    - Fix compile warning ([`e4514a8`](https://github.com/GitoxideLabs/gitoxide/commit/e4514a85d406aaa0aa959a18e0e32d46f1994cc8))
    - Remove reference of pretty-cli in code tree ([`4bd2f29`](https://github.com/GitoxideLabs/gitoxide/commit/4bd2f29da7e37c5d6e920c97df82c7860dd9f22c))
    - Remove lean plumbing CLI ([`51bf03f`](https://github.com/GitoxideLabs/gitoxide/commit/51bf03feaa94bebb26690dff92262b2134070a44))
    - Remove light* features, add 'lean-async' in its place; remove termion support ([`4d2d433`](https://github.com/GitoxideLabs/gitoxide/commit/4d2d433e7e98ac42db858688edac06e68ee4d10d))
 * **[#263](https://github.com/GitoxideLabs/gitoxide/issues/263)**
    - Fmt ([`fbeddeb`](https://github.com/GitoxideLabs/gitoxide/commit/fbeddebcab999f4898f768a3184906091f8ce0b8))
    - A mad attempt to use thread-local everywhere and avoid Sync… ([`0af5077`](https://github.com/GitoxideLabs/gitoxide/commit/0af5077e1f028c1c69bbdc098bb567e486282c37))
 * **[#266](https://github.com/GitoxideLabs/gitoxide/issues/266)**
    - Provide handle with a snapshot of the store's state ([`6e0cd6d`](https://github.com/GitoxideLabs/gitoxide/commit/6e0cd6d38c5df874990ace6c2c3c0b39342c4d05))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Fast-path multi-pack index verification in the CLI ([`bcde935`](https://github.com/GitoxideLabs/gitoxide/commit/bcde935e7102ba5cd50c057a8323353247d3dd85))
    - Basic multi-pack index creation ([`89428b2`](https://github.com/GitoxideLabs/gitoxide/commit/89428b2936fb0169606a543cf531bddaacb8187c))
    - 'index' with its own sub-commands ([`c4c5678`](https://github.com/GitoxideLabs/gitoxide/commit/c4c56787b1f9165984a8bddf35cfee530554fa2f))
    - Even nicer printing ([`d2bea27`](https://github.com/GitoxideLabs/gitoxide/commit/d2bea270787597d6aef48ffe023ff49969c33bd9))
    - Remove `Option<impl Progress>` in favor of `impl Progress` ([`bf04644`](https://github.com/GitoxideLabs/gitoxide/commit/bf04644ab75ed1969507f957dc8d4868790d462d))
    - Remove unnecessary `Arc` around `should_interrupt` flag ([`d851bed`](https://github.com/GitoxideLabs/gitoxide/commit/d851bede97801096d188ff6af06c98a79fe276db))
    - Remove Sha1 mentions in `index::verify::Mode::*` variants ([`c2679a0`](https://github.com/GitoxideLabs/gitoxide/commit/c2679a03358b9c19d63ed1af1cd57324c6381447))
 * **[#287](https://github.com/GitoxideLabs/gitoxide/issues/287)**
    - Share and pass cli arguments for pack verification ([`db43e47`](https://github.com/GitoxideLabs/gitoxide/commit/db43e47fc0a43ef45824ac1c9426c1889bdb13a3))
    - Very rough version of repository verification ([`80a4a7a`](https://github.com/GitoxideLabs/gitoxide/commit/80a4a7add688d16376b9bf2ed7f1c7f655b7c912))
    - Adjustments to deal with changes to git-pack/git-odb ([`fcf8fde`](https://github.com/GitoxideLabs/gitoxide/commit/fcf8fde7272974a70df808bd7ac03e925b7e39a8))
 * **[#289](https://github.com/GitoxideLabs/gitoxide/issues/289)**
    - 'pack' with its own sub-commands ([`fb64af4`](https://github.com/GitoxideLabs/gitoxide/commit/fb64af4d747960bfa40ec23051ecb03ea8ec5d83))
    - 'remote' with its own sub-commands ([`8677f7e`](https://github.com/GitoxideLabs/gitoxide/commit/8677f7edd516ea54ec652a4a59cb220422036b90))
    - 'commitgraph' with its own sub-commands ([`db0251e`](https://github.com/GitoxideLabs/gitoxide/commit/db0251e277ee9035bd3b44bf5ec9152fb64ac8c8))
 * **[#293](https://github.com/GitoxideLabs/gitoxide/issues/293)**
    - Faster writing to stdout/stderr for plumbing commands ([`d04dc01`](https://github.com/GitoxideLabs/gitoxide/commit/d04dc01115efa6688e71a2a0ef4ffce45d3d0db6))
    - Add 'index verify' subcommand to 'gix' ([`1ac2c21`](https://github.com/GitoxideLabs/gitoxide/commit/1ac2c210c311c4b2ef835e04e2d7c477981b850f))
    - Flag to hide extension details ([`34ea001`](https://github.com/GitoxideLabs/gitoxide/commit/34ea001fafa93b6453513cf458fe24327a13ff28))
    - Print basic index information, including the tree extension ([`9277cf8`](https://github.com/GitoxideLabs/gitoxide/commit/9277cf877e1f2276dcad1efdeb97e0e3d96ed3f0))
    - Basic entry information ([`239e7b2`](https://github.com/GitoxideLabs/gitoxide/commit/239e7b291297d6d49ebdf3d4986fb9fb86480e9a))
    - Refactor ([`8bf585d`](https://github.com/GitoxideLabs/gitoxide/commit/8bf585d67cd67b168d819ba05858cef7d9b90894))
    - JSON output for index entries ([`3fc1622`](https://github.com/GitoxideLabs/gitoxide/commit/3fc1622488054c6ab655eb9d2f941b68cc3ccf18))
 * **[#298](https://github.com/GitoxideLabs/gitoxide/issues/298)**
    - Simplify command-line options declaration ([`f790a55`](https://github.com/GitoxideLabs/gitoxide/commit/f790a55ff4263bea9b9476137bac3824912044ac))
    - Frame for printing index information ([`9ea98fd`](https://github.com/GitoxideLabs/gitoxide/commit/9ea98fda75fbef339647a0ca03776060356d1206))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Greatly simplify render-line logic ([`a8fa53a`](https://github.com/GitoxideLabs/gitoxide/commit/a8fa53a007780ec89f4768745b1549e8e73a8478))
    - Pass thread-limit along to checkout ([`07e9081`](https://github.com/GitoxideLabs/gitoxide/commit/07e9081fb5628e4ddc8f87e2d4ba0c7b3247bb35))
    - Add thread-count and chunk-size computation; interrupt capability ([`8cbe85d`](https://github.com/GitoxideLabs/gitoxide/commit/8cbe85d135898826a91939726465a9e295c1e24b))
    - A first sketch of access odb information using a sub-command ([`89b628a`](https://github.com/GitoxideLabs/gitoxide/commit/89b628ab5b833a34f0b426b3a399bb182e63f3f4))
    - Sub-command to print multi-index entries ([`6c10e09`](https://github.com/GitoxideLabs/gitoxide/commit/6c10e097a432d81b930008abc00c6821ed7ac9be))
    - Pack multi-index info subcommand ([`21c2dd5`](https://github.com/GitoxideLabs/gitoxide/commit/21c2dd5da20a9e3cbae618b6311b6c9de12cf43c))
    - Refactor ([`e6a3d43`](https://github.com/GitoxideLabs/gitoxide/commit/e6a3d437e1a97c56fba18d80ac54928d953cb507))
    - Detailed report about issues after checkout ([`613483b`](https://github.com/GitoxideLabs/gitoxide/commit/613483b297b8a7e9a91cac3ef8205f2103ea946b))
    - Keep-going support on the command-line ([`73a7393`](https://github.com/GitoxideLabs/gitoxide/commit/73a73932f430fe991f26222ba2735332c03c0e77))
    - Add tree-info subcommand to more easily test actual tree-traversal performance ([`29fb0c8`](https://github.com/GitoxideLabs/gitoxide/commit/29fb0c8ff628716d33c9c41d3910e617791dcc77))
    - Frame for traversing tree entries ([`0e55fbb`](https://github.com/GitoxideLabs/gitoxide/commit/0e55fbb2fb0cec6f402b7a3aed7ee55078d233a1))
    - Properly use 'max-performance' feature toggle to get pack caches :D ([`a39d476`](https://github.com/GitoxideLabs/gitoxide/commit/a39d4768e36f27aababefd5bd519e51f33ffa7b6))
    - Allow writing empty files during checkout but also query the odb ([`5388d80`](https://github.com/GitoxideLabs/gitoxide/commit/5388d8091ef02cf927478a1492847ae1666040d4))
    - Support for repo to write actual objects ([`5494fb3`](https://github.com/GitoxideLabs/gitoxide/commit/5494fb3e1de1234dde8c47336597283dbd8bcb29))
    - Basic version of index checkout via command-line ([`f23b8d2`](https://github.com/GitoxideLabs/gitoxide/commit/f23b8d2f1c4b767d337ec51888afaa8b3719798c))
    - Support for unicode-precomposition for gix apps ([`e90c123`](https://github.com/GitoxideLabs/gitoxide/commit/e90c123675a98ab62fc6bb22019f889cee8b7301))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - In-manifest and in-bin documentation of feature toggles ([`25da30f`](https://github.com/GitoxideLabs/gitoxide/commit/25da30f3652bd72c157e84439dd6e3957471fa08))
 * **[#366](https://github.com/GitoxideLabs/gitoxide/issues/366)**
    - Frame for printing mailmap entries using git-repository ([`2a01f47`](https://github.com/GitoxideLabs/gitoxide/commit/2a01f4728ae858b47280b587501d343fdb86655d))
    - Gix mailmap verify can now detect collisions ([`f89fe2f`](https://github.com/GitoxideLabs/gitoxide/commit/f89fe2f867fa792db5d9e003ce342a337a6ac973))
    - `gix mailmap verify` command ([`384ed66`](https://github.com/GitoxideLabs/gitoxide/commit/384ed665c7423feca1b1ee1f81db10867fa813a8))
 * **[#67](https://github.com/GitoxideLabs/gitoxide/issues/67)**
    - --counting-threads flag to configure amount of threads when counting ([`0090961`](https://github.com/GitoxideLabs/gitoxide/commit/00909619ff04e247aabc9ffe3c025f0064c3092d))
 * **Uncategorized**
    - Make fmt ([`7cf3545`](https://github.com/GitoxideLabs/gitoxide/commit/7cf354509b545f7e7c99e159b5989ddfbe86273d))
    - Small build now uses the line renderer as well ([`652a0ac`](https://github.com/GitoxideLabs/gitoxide/commit/652a0acdf9f06e35e65c1b66d264d5e8734ccc65))
    - Upgrade to prodash 19 ([`90c6c5a`](https://github.com/GitoxideLabs/gitoxide/commit/90c6c5aec4015ff969d6e2514fa4d49873ee80f5))
    - `ein find --debug` to learn why it is slow ([`70109be`](https://github.com/GitoxideLabs/gitoxide/commit/70109bee679d33a5c5fb3a78a708b479684b03b1))
    - Fix clap warnings ([`aa51e05`](https://github.com/GitoxideLabs/gitoxide/commit/aa51e05923695e20aecc16317331c7e26d49a2e7))
    - Merge branch 'index-verification' ([`ad3c803`](https://github.com/GitoxideLabs/gitoxide/commit/ad3c8032cee02052ef3940d1d7c950270a0a299a))
    - Merge branch 'index-information' ([`025f157`](https://github.com/GitoxideLabs/gitoxide/commit/025f157de10a509a4b36a9aed41de80487e8c15c))
    - Improve CLI docs ([`866530a`](https://github.com/GitoxideLabs/gitoxide/commit/866530a154c3ef9383fae30c694991e31e97528c))
    - Rename 'gix commitgraph' back to 'gix commit-graph' ([`d6a72e7`](https://github.com/GitoxideLabs/gitoxide/commit/d6a72e70c9b4ee9b10a1172cce64ade5664599eb))
    - Merge branch 'use-midx-in-store' ([`338521b`](https://github.com/GitoxideLabs/gitoxide/commit/338521b0443b9dc1007581de42ef6a950f6e0bbf))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`b0f7328`](https://github.com/GitoxideLabs/gitoxide/commit/b0f73280c0233e05b68a22b0b01f40d574786a03))
</details>

## v0.10.0 (2021-10-20)

This release pins beta versions of `clap` to avoid it to automatically fetch the latest one
during installation.

This is made possible due to `clap` itself pinning its dependency
to the `clap-derive` crate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 1 calendar day.
 - 4 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#222](https://github.com/GitoxideLabs/gitoxide/issues/222)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#222](https://github.com/GitoxideLabs/gitoxide/issues/222)**
    - Upgrade to clap 3 beta 5 ([`2ddc4ed`](https://github.com/GitoxideLabs/gitoxide/commit/2ddc4eddda23c77b5891a11a3e7215702c63882b))
</details>

## v0.9.0 (2021-10-15)

A first usable version of `gix` to make using `gitoxide` from your applications so much easier. It serves as a one-stop shop for application developers without sacrificing performance by default while making common use-cases more convenient.

### Feature list

* `gix` as hub crate for application development with focus on usability without sacrificing any knob to tune performance.
* opt-in `async` for `gix-packetline`, `gix-transport` and `gix-protocol` for fully async git clients, along with the `light-async` feature toggle to build a `gix pack-receive` with an async client instead of a blocking one.
* Statistics for `gix pack-create` with the `-s/--statistics` flag to have data indicating the cost of the operation. Currently it's doing a lot of work that has to be avoided in order to be useable in production and the numbers underline that. Future iterations will cause key metrics to go down.
* Packs are now reproducible by default, which means that the same tip will always generate a pack with the same hash. This may be a desirable property for some kinds of packs, but not for others which is why it can be turned off for a considerable speed boost.
* `gix-tempfile` crate
* `gix-lock` crate
* `gix-ref` crate with complete loose-ref, packed-ref and transaction support.


### Performance

* On M1, thanks to [a new release](https://github.com/RustCrypto/hashes/pull/289#event-5035369215), Sha1 is now computed much faster which unlocks a massive performance boost. In my test, verifying/decoding the entire linux kernel pack now happens in 17s, as compared to 37s for canonical `git`.
* `gix-object` parsing is a few percent faster thanks a reworked error handling for objects. By default, error collection is disabled entirely making the error case zero-sized. If needed, verbose and stacked errors can be turned on using a feature toggle for applications who expect repositories with malformed objects and need detailed diagnostics.

### New Features

 - <csr-id-60c9fad8002b4e3f6b9607bba6361871752f4d3d/> control pack and object cache size in megabytes in some sub-commands

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 26 calendar days.
 - 35 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#200](https://github.com/GitoxideLabs/gitoxide/issues/200), [#67](https://github.com/GitoxideLabs/gitoxide/issues/67)

## v0.8.4 (2021-09-10)

This is a maintenance release.

## v0.8.3 (2021-09-07)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 8 calendar days.
 - 20 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

## v0.8.2 (2021-08-17)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

## v0.8.1 (2021-08-15)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release over the course of 95 calendar days.
 - 98 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#83](https://github.com/GitoxideLabs/gitoxide/issues/83)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

## v0.7.0 (2021-05-09)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 32 commits contributed to the release over the course of 128 calendar days.
 - 143 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 3 times to make code idiomatic. 

## v0.6.0 (2020-12-16)

Maintenance release without any new features.

These are created to account for breaking changes within the dependency graph of
`gitoxide` crates. Due to some blunders in the past the version on crates.io
could not be installed anymore.
This was eventually fixed with new minor releases across the ecosystem.

Finally, yet another breaking change due to the introduction of the `gix-hash`
crate to break a dependency cycle between `gix-object` and `gix-features` caused
yet another maintenance release.

## v0.5.0 (2020-12-15)

Maintenance release without any new features.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 78 calendar days.
 - 84 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge branch 'commit-graph' into main ([`9cb09b2`](https://github.com/GitoxideLabs/gitoxide/commit/9cb09b248796f0ff5c9d3f3e857de4731324cfd5))
    - Add lean-plumbing docs for path of commit-graph-verify ([`5c7b52d`](https://github.com/GitoxideLabs/gitoxide/commit/5c7b52d658d5b86bf4cf05c724202e824016c0e2))
    - [commitgraph] Implement basic commit-graph file verification. ([`2571113`](https://github.com/GitoxideLabs/gitoxide/commit/2571113fea516737acedac08d66632ead499b474))
    - [commitgraph] Stub out commit-graph-verify plumbing command. ([`aacf0f0`](https://github.com/GitoxideLabs/gitoxide/commit/aacf0f05a909e5b7d9ffd5623ef9833e0465be93))
    - Merge branch 'main' into commit-graph ([`ca5b801`](https://github.com/GitoxideLabs/gitoxide/commit/ca5b80174b73cc9ac162b3f33b5d3721ef936cb1))
</details>

## v0.4.3 (2020-09-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

## v0.4.1 (2020-09-18)

* fix installation via `cargo install`

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
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Finish removal of rust 2018 idioms ([`0d1699e`](https://github.com/GitoxideLabs/gitoxide/commit/0d1699e0e0bc9052be0a74b1b3f3d3eeeec39e3e))
    - Provide terminal dimensions to better use horizontal space ([`11f6b84`](https://github.com/GitoxideLabs/gitoxide/commit/11f6b8497a5089377e605f4cbe1cd317ef677d59))
</details>

## v0.4.0 (2020-09-12)

* add `remote-ref-list` and `pack-receive` subcommands to **gix**

### CLI Breaking

 * rename plumbing sub-command from `index-from-pack` to `pack-index-from-data`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 29 calendar days.
 - 30 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - [clone] refs can now be written into a specified directory ([`fb1f048`](https://github.com/GitoxideLabs/gitoxide/commit/fb1f04837be994fa5bcb9aa24f25b5f4f72e4ce0))
    - [clone] First version of writing references, but… ([`445be27`](https://github.com/GitoxideLabs/gitoxide/commit/445be27cf81663ba4fe941c00262448444efbac2))
    - [clone] first journey test for pack-receive ([`46a3511`](https://github.com/GitoxideLabs/gitoxide/commit/46a3511aead043bc45256ce603285ff4d0fff60a))
    - [clone] This actually works: first MVP of retrieving packs via clone ([`c06d819`](https://github.com/GitoxideLabs/gitoxide/commit/c06d8194173f9ec468ddd0faf72dd6d8dbf7d35d))
    - [ref-ls] add pretty version for ls-refs ([`487d06d`](https://github.com/GitoxideLabs/gitoxide/commit/487d06d53b9cc201b5a009977e835b51f4b9f690))
    - [ref-ls] Fix progress display ([`2fcb557`](https://github.com/GitoxideLabs/gitoxide/commit/2fcb557dce941eb94ca60f46ecee86b94e029db7))
    - Refactor ([`b38290e`](https://github.com/GitoxideLabs/gitoxide/commit/b38290e4a8fcabd758f26a15407710ab2abcdc07))
    - [ref-ls] refactor ([`35e26fc`](https://github.com/GitoxideLabs/gitoxide/commit/35e26fc32978232aebda3468c9f172fb7b08b815))
    - Refactor ([`f90b92f`](https://github.com/GitoxideLabs/gitoxide/commit/f90b92ffc2994f594352abaf4bacd9767cbc2e6c))
    - [ref-ls] Frame for remote-ref-ls command in gitoxide-core ([`161e7df`](https://github.com/GitoxideLabs/gitoxide/commit/161e7df34a53db40551879c6d2319ee775dfd551))
    - [clone] link up lean plumbing command with gitoxide-core: pack-receive ([`5ea49c8`](https://github.com/GitoxideLabs/gitoxide/commit/5ea49c8aa0d449bed98ce0147ad222ff25c27c32))
    - Refactor ([`40a6412`](https://github.com/GitoxideLabs/gitoxide/commit/40a64125dc5556630576ec2164b68838c76ccd79))
    - Less ambiguous name for 'index-from-pack': 'pack-index-from-data' ([`386673c`](https://github.com/GitoxideLabs/gitoxide/commit/386673ccc99d18d023c7df3fcd40e86d71960b25))
    - Refactor ([`b4a6e16`](https://github.com/GitoxideLabs/gitoxide/commit/b4a6e16364822c0dccb56f98dbfb0ca4c8007069))
</details>

## v0.3.0 (2020-08-12)

* add `pack-explode` and `pack-index-from-data` sub-commands
* massive speed improvements for `pack-verify`

Many small and possibly breaking changes are not mentioned here.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 46 commits contributed to the release over the course of 30 calendar days.
 - 31 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Make obvious that interrupt request was received ([`34b2373`](https://github.com/GitoxideLabs/gitoxide/commit/34b23737f560fe52d4f98fb886eba754652f9a5e))
    - Make interrupt handler work reliably ([`e71da0f`](https://github.com/GitoxideLabs/gitoxide/commit/e71da0fce6d6eab68f7b81b13cdc78ce8e9b7ee3))
    - Unify used ranges for line renderer amond pretty and lean interface ([`f59f66e`](https://github.com/GitoxideLabs/gitoxide/commit/f59f66e189732f567414f68c7463364e510f41c4))
    - Add percentage and throughput to tasks that matter ([`763d7ca`](https://github.com/GitoxideLabs/gitoxide/commit/763d7caa4c70111b7cb3ef5733d2c3c697758c28))
    - Upgrade to latest iteration of prodash ([`3a4faec`](https://github.com/GitoxideLabs/gitoxide/commit/3a4faecab56e37670c553e6563f11a46d740c333))
    - Support for JSON format output ([`1931575`](https://github.com/GitoxideLabs/gitoxide/commit/19315750f4f409e3f105c3c4054c4afbef91daad))
    - First pieces of the index-from-pack journey tests ([`181d69c`](https://github.com/GitoxideLabs/gitoxide/commit/181d69c1da46a931c513cbd7d8bca7b2fa53351c))
    - Add versions back to main command, remove from sub-commands ([`e509373`](https://github.com/GitoxideLabs/gitoxide/commit/e509373b26c9a7b120057fc6e75970568f328fc4))
    - Ditch structopt in favor of clap 3.0 beta1 ([`d7591e2`](https://github.com/GitoxideLabs/gitoxide/commit/d7591e24a5178732713286a5e28cbc90f5fe9ed9))
    - Move common flags to common plac ([`c0352c2`](https://github.com/GitoxideLabs/gitoxide/commit/c0352c2643e2badde79778d4a22d2e392a44f0a3))
    - Write about user interfaces and the use/non-use of async ([`91ba045`](https://github.com/GitoxideLabs/gitoxide/commit/91ba0457745f860b7a68cb38b13e69754747e8d9))
    - Interrupt support for pretty plumbing ([`bca7ce2`](https://github.com/GitoxideLabs/gitoxide/commit/bca7ce2e668a4be2600d2d04d00f46b21c82eee2))
    - Revert "Less memory for look up mode, faster start" - too slow ([`584350a`](https://github.com/GitoxideLabs/gitoxide/commit/584350af91f533db4cf980327d530445384c6b5a))
    - Less memory for look up mode, faster start ([`395c7e7`](https://github.com/GitoxideLabs/gitoxide/commit/395c7e78ef344ee56cf3d4ef49828942a09094bc))
    - Remove memory mode entirely (and some complexity with it) ([`8812e91`](https://github.com/GitoxideLabs/gitoxide/commit/8812e916a21983868a37c4aade10f79a1dc9b926))
    - Turns out you never want to keep deltas in memory ([`657aa2c`](https://github.com/GitoxideLabs/gitoxide/commit/657aa2c38673cf10174f42bcb97039ac37b2926e))
    - Remove support for keeping compressed memory to reduce the index size ([`1e2ec7e`](https://github.com/GitoxideLabs/gitoxide/commit/1e2ec7e9d0ef2f2a4908860672080e411e945bff))
    - …but there seem to be issues with the kernel pack… ([`cc147bc`](https://github.com/GitoxideLabs/gitoxide/commit/cc147bc60066c4ef31353a499958edadc960a9c4))
    - Minor fixes after first local tests - it's up to twice as fast!! ([`43c7fd1`](https://github.com/GitoxideLabs/gitoxide/commit/43c7fd1f81b9b4c938f99c0bf1deabdf121226b9))
    - Quick and dirty impl of lean command-line for index-from-pack ([`9660bbf`](https://github.com/GitoxideLabs/gitoxide/commit/9660bbffd8ace621178b067e22d227ef8c50ba84))
    - Upgrade dependencies ([`44b8221`](https://github.com/GitoxideLabs/gitoxide/commit/44b8221800454f9b651778a422186bd5061877f4))
    - Remove invalid clap configuration ([`665696f`](https://github.com/GitoxideLabs/gitoxide/commit/665696f636e152ad9969ea0ca004cb83f1641ae6))
    - Prepare full 'verify' implementation ([`ee45c7f`](https://github.com/GitoxideLabs/gitoxide/commit/ee45c7f47b95fc406cc5922a322c8fd6c0f52775))
    - Refactor ([`0a33b24`](https://github.com/GitoxideLabs/gitoxide/commit/0a33b24f5b61ccdf1358f1e9adcf0f6fd4099c1c))
    - Allow sink-compress configuration; choose best algorithm ([`29b9c23`](https://github.com/GitoxideLabs/gitoxide/commit/29b9c230e35ba9b4334797b63ab9fa88c2fe59d0))
    - Nice error message on failure ([`adbc82c`](https://github.com/GitoxideLabs/gitoxide/commit/adbc82c31450681fcb38233eeb8095efc5e52a18))
    - The first 'explode' implementation… ([`0d31ad1`](https://github.com/GitoxideLabs/gitoxide/commit/0d31ad1b61997fa0d0692c5919fb8032ffaaa35b))
    - Get all pieces ready for action ([`1805d64`](https://github.com/GitoxideLabs/gitoxide/commit/1805d64b9222d6a05a8718f04b29b789c1f42fea))
    - Pass option for safety checks down to explode(…) ([`0bcb790`](https://github.com/GitoxideLabs/gitoxide/commit/0bcb790dc8c35097916876afbb68bbfcc894c369))
    - Refactor ([`f66b116`](https://github.com/GitoxideLabs/gitoxide/commit/f66b116ddfbee62c3e20a4c5e7cd878fbf064195))
    - Basic tests and CLI args for explode pack ([`f932256`](https://github.com/GitoxideLabs/gitoxide/commit/f932256a62d6fc5d5558446de079fb666ddc27da))
    - Rename verify-pack to pack-verify (keeping it more formal) ([`ec8c48a`](https://github.com/GitoxideLabs/gitoxide/commit/ec8c48a8fcbcd748c9c764734d881b5f0217e1e4))
    - Refactor ([`d3c00c8`](https://github.com/GitoxideLabs/gitoxide/commit/d3c00c841ee1aeda6bb0534fe365db13c31f8d3c))
    - Change bin names from 'gio' to 'gix' and 'gixp' ([`5e23137`](https://github.com/GitoxideLabs/gitoxide/commit/5e231371432ad02c67b095448564b2aa6af76799))
    - Revert "Invert --statitics switch to become --no-statistics" ([`93a9b30`](https://github.com/GitoxideLabs/gitoxide/commit/93a9b30069d9abc5742546ade90913026ac5774b))
    - Invert --statitics switch to become --no-statistics ([`aeb8778`](https://github.com/GitoxideLabs/gitoxide/commit/aeb87789ecc5cf3fd0ac69d67c7d0785e4eb329c))
    - Use faster algorithm by default ([`bb45c3d`](https://github.com/GitoxideLabs/gitoxide/commit/bb45c3d8a2aabf87231981000240f0444abf6fc4))
    - Fix clippy ([`ec40e09`](https://github.com/GitoxideLabs/gitoxide/commit/ec40e093d72f93d86168f39ebaca5b122ca0bec3))
    - Change course and do pack streaming first ([`bcb275e`](https://github.com/GitoxideLabs/gitoxide/commit/bcb275e91cfd6f0a71b3cb59a2b706b60608a594))
    - Get rid of annoying warnings - there is no better and easier way ([`41f38c4`](https://github.com/GitoxideLabs/gitoxide/commit/41f38c442e086b1f3fb48eea25839ef6207f0cbc))
    - Fully implement --encode and --re-encode flags ([`a7cfac8`](https://github.com/GitoxideLabs/gitoxide/commit/a7cfac83ddd859d9c2c25e457c0d7043738792dc))
    - Prepare for re-encoding each pack object ([`afae684`](https://github.com/GitoxideLabs/gitoxide/commit/afae684c72e5dc4b718976056dd5d34ed61de72a))
    - Fix naming change, which was introduced accidentally ([`fbb9f98`](https://github.com/GitoxideLabs/gitoxide/commit/fbb9f98508ec722e192466e28ded47aef2fb78b3))
    - Refactor ([`2888f1b`](https://github.com/GitoxideLabs/gitoxide/commit/2888f1b10a2baf40155544e667ddd461f3ddc938))
    - Pass threadlimit down from CLIs ([`f98c5b1`](https://github.com/GitoxideLabs/gitoxide/commit/f98c5b160db80a7cac530e18b9256562c25be47f))
    - Add new Context argument to support more configuration options ([`7c5d8b8`](https://github.com/GitoxideLabs/gitoxide/commit/7c5d8b8bb318e59a59ad74ad767a1525e2833632))
</details>

## v0.1.0 (2020-07-12)

* Initial release with `pack-verify`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 54 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bring color back to 'max' versions ([`c68d9ab`](https://github.com/GitoxideLabs/gitoxide/commit/c68d9ab6e9cccab3610a77a2c6839a26fb42de2d))
    - Support for disabling the cursor in the pretty line renderer ([`48c4bbd`](https://github.com/GitoxideLabs/gitoxide/commit/48c4bbd35f678de801fa2243f7a8705e825bdbf9))
    - Allow TUI to terminate action properly ([`1f1b725`](https://github.com/GitoxideLabs/gitoxide/commit/1f1b7257895a219b5623b1ba3beaee1282ff2b63))
    - Refactor ([`cce71aa`](https://github.com/GitoxideLabs/gitoxide/commit/cce71aadb53528cb2f1d173b81df939ad1df8083))
    - Refactor ([`f276a05`](https://github.com/GitoxideLabs/gitoxide/commit/f276a0561fcec78f802b166f09110becf22ea7ee))
    - Preliminary support for line renderer in max version ([`4aa8022`](https://github.com/GitoxideLabs/gitoxide/commit/4aa8022c248fc26292ea9f1c61e2e8dbadc56e7c))
    - Unify frame rate across plumbing (and later potentially porcelain) ([`e2a7bdd`](https://github.com/GitoxideLabs/gitoxide/commit/e2a7bdd9a549400f0e8d31b590f469660e458c89))
    - Refactor ([`41e01a5`](https://github.com/GitoxideLabs/gitoxide/commit/41e01a53a7f48dd3341ee287a243a33190bccea0))
    - Proper implementation of line renderer into 'lean' CLI ([`e98e7c2`](https://github.com/GitoxideLabs/gitoxide/commit/e98e7c280d73e9d9ebd13202afb93a56cb2f7c9c))
    - Mild improvements to look of verbose log ([`5fff552`](https://github.com/GitoxideLabs/gitoxide/commit/5fff5524c4443d9c9ae26307c19745c722334d0c))
    - First very basic version of line renderer progress - works… ([`0cc1bf2`](https://github.com/GitoxideLabs/gitoxide/commit/0cc1bf25c69611f9512fec415ae8e09b608706fc))
    - Prepare for optional addition of line renderer for lean version ([`aac0d34`](https://github.com/GitoxideLabs/gitoxide/commit/aac0d341eb02f0dccdf740f7ef15e8f585907544))
    - Upgrade to prodash version 7 ([`af02b46`](https://github.com/GitoxideLabs/gitoxide/commit/af02b46cc1eff5ba1da7da20d3f524a79fad686f))
    - Make --version flags work as expected. ([`a4d978c`](https://github.com/GitoxideLabs/gitoxide/commit/a4d978ccc11e73fd752055c9a28b3b23dea145ea))
    - Merge branch 'release' ([`a1a0b13`](https://github.com/GitoxideLabs/gitoxide/commit/a1a0b135c991edfe5cddb71c5fbfbed25b47e3b3))
    - Rename 'pretty' target into 'max', a better fit for what it is ([`5acecc5`](https://github.com/GitoxideLabs/gitoxide/commit/5acecc59d2d39141f2e98b6f8556c6d457ab0965))
    - Make gio commands less cumbersome, self-document their build type (pretty, lean) ([`1f9bc03`](https://github.com/GitoxideLabs/gitoxide/commit/1f9bc03dd773d90960a6f6d4ee59af3f938ad80b))
    - Allow to limit the logging depth for less cluttered output ([`fce7035`](https://github.com/GitoxideLabs/gitoxide/commit/fce703531d7006f7d961d6ffa66f51f6c9bc0efc))
    - Support for json in pretty-plumbing and gitoxide (on demand) ([`b3780f8`](https://github.com/GitoxideLabs/gitoxide/commit/b3780f87438d34b372c48b7385199f7ea22b3965))
    - Simplify the 'keep open' logic of TUI progress window ([`13cd8ce`](https://github.com/GitoxideLabs/gitoxide/commit/13cd8ce372800eb0016190960834c759c9744b9c))
    - Attempt to implement progress with a mode enum ([`ac490c2`](https://github.com/GitoxideLabs/gitoxide/commit/ac490c21b8f369c45ee0d7688ddb381ce6f4af94))
    - Allow for more screen space when formatting ([`6794300`](https://github.com/GitoxideLabs/gitoxide/commit/67943002e7f4215b5383bd0538786ce2857f011e))
    - Assure pretty progress doesn't occlude the output ([`122d69f`](https://github.com/GitoxideLabs/gitoxide/commit/122d69fee217eb264a335f0a056d03eba066332e))
    - Fix pretty build ([`6adf615`](https://github.com/GitoxideLabs/gitoxide/commit/6adf615ed7d6c488c25589940fc0a55bf0fb3d5c))
    - Pass average stats through to the top level ([`5b4979c`](https://github.com/GitoxideLabs/gitoxide/commit/5b4979c1dfeb9a29974dd4e6529ae5da074d0b1a))
    - Refactor ([`7add82c`](https://github.com/GitoxideLabs/gitoxide/commit/7add82c39169e3c2fff76c48cdd318fe6040d7bc))
    - Now ACTUALLY stop TUI when there is no progress anymore :D ([`3bf3321`](https://github.com/GitoxideLabs/gitoxide/commit/3bf33210a96e1e3bc2a81782b339b5c67344ac34))
    - Automatically close the TUI when there is no progress anymore. ([`c416152`](https://github.com/GitoxideLabs/gitoxide/commit/c416152b04051958de7bd161a8a2ee42ca163275))
    - Assure we wait for GUI thread to finish ([`60eaea0`](https://github.com/GitoxideLabs/gitoxide/commit/60eaea0ee01214202ab9f23514dc45a9909d7888))
    - Pretty progress in a generalized form ([`caa883b`](https://github.com/GitoxideLabs/gitoxide/commit/caa883b96827deb63b5c8787ed820d22f2c85249))
    - Neater progress log messages: don't show the module it originates from ([`026a0dd`](https://github.com/GitoxideLabs/gitoxide/commit/026a0dd1faf28c4668f58bd1790c168a0134559f))
    - Refactor ([`30925e6`](https://github.com/GitoxideLabs/gitoxide/commit/30925e654144a05365908f7d2aa90deb7b2952d3))
    - Support for logging in pretty binaries ([`67026e4`](https://github.com/GitoxideLabs/gitoxide/commit/67026e479f0aa3e47ff3fd230c8741a7a5dbe99c))
    - --verbose flag for lean plumbing binary ([`aaf4825`](https://github.com/GitoxideLabs/gitoxide/commit/aaf482584d1ee080c0a6c091c4675736c4c8d6a7))
    - First very basic progress implementation ([`b820717`](https://github.com/GitoxideLabs/gitoxide/commit/b8207177daee8a9ffa23c7c052cf9ca651b15804))
    - Pass progress everywhere, for now just to discard it ([`da3ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/da3ae1c82cd726b8fae9b8d26069719930e9ba99))
    - Split plumbing into separate binary ([`b1e51d6`](https://github.com/GitoxideLabs/gitoxide/commit/b1e51d6a83ca7a00923b39209d0a2bfb3b78de0d))
    - Refactor ([`0fbba9f`](https://github.com/GitoxideLabs/gitoxide/commit/0fbba9fe7597af03912f956c251c88472b48c3eb))
    - Refactor ([`ba6a8ef`](https://github.com/GitoxideLabs/gitoxide/commit/ba6a8ef064a9884066414c82f4f7d1bb72ab524f))
    - Add initial version of 'lean-cli' feature toggle, but… ([`f01c298`](https://github.com/GitoxideLabs/gitoxide/commit/f01c2985732ac05b24a7fcbc3752ef52dd1bc438))
    - Support for verifying pack files and index files ([`b09b4e1`](https://github.com/GitoxideLabs/gitoxide/commit/b09b4e1f35c3802dfd3418bda42b96828acd9ec8))
    - Reorganize crates to make 'gitoxide' the CLI, and 'gitoxide-core' the library ([`0ac9c5a`](https://github.com/GitoxideLabs/gitoxide/commit/0ac9c5af0cbb562d3cb48a661736afd98dd1a940))
    - Add simple pack verification to gio ([`8c0e0b5`](https://github.com/GitoxideLabs/gitoxide/commit/8c0e0b5bb79c8c337eed03d37cbf818d8bb9c924))
    - Goodbye git-core, hello git-repository ([`7cec2b6`](https://github.com/GitoxideLabs/gitoxide/commit/7cec2b648f86fc665b4fc5bfe269e9ca16679a55))
    - Document existing use of unsafe, deny everywhere else ([`41f4bce`](https://github.com/GitoxideLabs/gitoxide/commit/41f4bce9d9a492f8e20a6eb5b3eaf5adc6d78329))
    - Cargo clippy ([`1179ac1`](https://github.com/GitoxideLabs/gitoxide/commit/1179ac16ea2bb84816f9b615d1191f8a2d4e775b))
    - Move parsing tests close to actual parsing ([`3ca2c59`](https://github.com/GitoxideLabs/gitoxide/commit/3ca2c592d91c9aa8fab8ed749871d6d96f2ef4e2))
    - Color for all grit commands/subcommands ([`aa8efdd`](https://github.com/GitoxideLabs/gitoxide/commit/aa8efdd922d45bdab668dc71e8b30adf79930667))
    - Use structopt instead of clap ([`eb7388c`](https://github.com/GitoxideLabs/gitoxide/commit/eb7388c5d51e4ef3ea928d5f8f9e5b218cdbbd57))
    - Remove failure from grit binary, too ([`417c34b`](https://github.com/GitoxideLabs/gitoxide/commit/417c34b82469bcc3391706646dd39c7f6d1ad69c))
    - Refactor ([`87c8a2e`](https://github.com/GitoxideLabs/gitoxide/commit/87c8a2e288140b04e163fe85266d040d039ec69c))
    - Cargo fmt ([`2aa0857`](https://github.com/GitoxideLabs/gitoxide/commit/2aa085752aa3e99b51034a3dec882aea8c27ad94))
    - Implement git-init ([`57737c2`](https://github.com/GitoxideLabs/gitoxide/commit/57737c2c48ff898a327ba57712fea21b5d83188e))
    - Initial commit - based on standard project template ([`c3d319f`](https://github.com/GitoxideLabs/gitoxide/commit/c3d319f2b3076a0bb169bcd8a7b6a011f6aba9a5))
</details>

