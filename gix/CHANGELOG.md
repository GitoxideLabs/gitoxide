# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Bug Fixes

 - <csr-id-4773fd171ec12db75761052d8a5de4dc513f71ff/> Correctly use `$COMMON_DIR/info/exclude` to make excludes work in worktrees.
   It turns out there is no per-worktree excludes file either.

### Chore (BREAKING)

 - <csr-id-2358b1d250d3d2348210fb61dcb95ebe7aa6314b/> Upgrade `prodash` and `crosstermion` to the latest version.
   This will fix the `cargo deny` issue as it brings in a newer `lru` crate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2354 from canac/worktree-info-exclude ([`10bdf7e`](https://github.com/GitoxideLabs/gitoxide/commit/10bdf7e90679ec5bf5f4796ffe7300085f1e6523))
    - Refactor ([`3f0e26e`](https://github.com/GitoxideLabs/gitoxide/commit/3f0e26e0ddfc556ea57ac1fa5d7827edee9677dc))
    - Correctly use `$COMMON_DIR/info/exclude` to make excludes work in worktrees. ([`4773fd1`](https://github.com/GitoxideLabs/gitoxide/commit/4773fd171ec12db75761052d8a5de4dc513f71ff))
    - Merge pull request #2353 from GitoxideLabs/improvements ([`ee0a1f6`](https://github.com/GitoxideLabs/gitoxide/commit/ee0a1f6cc99831d7c5f96b657df6ebca5ee5638c))
    - Upgrade `prodash` and `crosstermion` to the latest version. ([`2358b1d`](https://github.com/GitoxideLabs/gitoxide/commit/2358b1d250d3d2348210fb61dcb95ebe7aa6314b))
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2341 from GitoxideLabs/dependabot/cargo/cargo-cf4a2135ae ([`d914d95`](https://github.com/GitoxideLabs/gitoxide/commit/d914d9533ed2243658d51ba05e68dd444b75a748))
    - Bump the cargo group across 1 directory with 51 updates ([`4edc5dd`](https://github.com/GitoxideLabs/gitoxide/commit/4edc5dda7ca39cc8249cb98dc39ca46c7d00eb44))
    - Merge pull request #2333 from GitoxideLabs/improvements ([`9e1ba5e`](https://github.com/GitoxideLabs/gitoxide/commit/9e1ba5ee6b7e0e032f682097d48664a68891762d))
    - Release gix-command v0.6.4, gix-credentials v0.34.1, gix-transport v0.52.1 ([`3bc0c47`](https://github.com/GitoxideLabs/gitoxide/commit/3bc0c472a8395e9634dd602839e208501f841c11))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.77.0 (2025-12-31)

### Documentation

 - <csr-id-e29e95210b6962f1dba57d20d3391c3226f37a9b/> using PrepareFetch::with_ref_name with non-branch refs causes panics

### Bug Fixes

 - <csr-id-2ffee22e32cda4da0f483956f79205c5d99b2cdc/> obtain correct worktree directories for submodule status when behind symlinks.
   Related to https://github.com/gitui-org/gitui/issues/2820

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2320](https://github.com/GitoxideLabs/gitoxide/issues/2320)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2320](https://github.com/GitoxideLabs/gitoxide/issues/2320)**
    - Create configuration file before possibly probing the filesystem. ([`b1d6c67`](https://github.com/GitoxideLabs/gitoxide/commit/b1d6c6743f09de1c16e967fc213d2994abdda0d3))
 * **Uncategorized**
    - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
    - Merge pull request #2314 from GitoxideLabs/copilot/fix-shallow-cloning-issue ([`30d8d5c`](https://github.com/GitoxideLabs/gitoxide/commit/30d8d5c3992e70a0361301e659d6e25de1fbd4b4))
    - Refactor ([`3529428`](https://github.com/GitoxideLabs/gitoxide/commit/35294280d88ab6dbcfa1fa84d78e54480b4da826))
    - Merge pull request #2312 from GitoxideLabs/copilot/adapt-script-for-gix-test-suite ([`0b29bd3`](https://github.com/GitoxideLabs/gitoxide/commit/0b29bd3d68f8b3a4ae0dbe16866e31d2fb66dfce))
    - Obtain correct worktree directories for submodule status when behind symlinks. ([`2ffee22`](https://github.com/GitoxideLabs/gitoxide/commit/2ffee22e32cda4da0f483956f79205c5d99b2cdc))
    - Fix shallow clone to respect remote HEAD instead of local defaults ([`f087953`](https://github.com/GitoxideLabs/gitoxide/commit/f0879534b6910c9c60763f4bbd8a3508c11f8fdf))
    - Merge pull request #2321 from GitoxideLabs/improvements ([`ac0af1d`](https://github.com/GitoxideLabs/gitoxide/commit/ac0af1d93b82b4c3ec68d02031c87de6ceb0e4d5))
    - Merge pull request #2315 from decathorpe/main ([`7c59cf4`](https://github.com/GitoxideLabs/gitoxide/commit/7c59cf4377502cbc9d115afea5763743c39f8e60))
    - More standard panic heading. ([`7d900a6`](https://github.com/GitoxideLabs/gitoxide/commit/7d900a6acf39ea4fc9f31eb863f6ced235fe7c3e))
    - Using PrepareFetch::with_ref_name with non-branch refs causes panics ([`e29e952`](https://github.com/GitoxideLabs/gitoxide/commit/e29e95210b6962f1dba57d20d3391c3226f37a9b))
    - Merge pull request #2307 from cruessler/add-feature-flag-to-binary ([`d602d7e`](https://github.com/GitoxideLabs/gitoxide/commit/d602d7e836071f1e9e3af44c33f01522dde5c417))
    - Refactor ([`d532bbf`](https://github.com/GitoxideLabs/gitoxide/commit/d532bbffab880ef253207a5e68a2ad39e7d1c9ac))
    - Add feature flag `blame-experimental` to `gix` ([`826effb`](https://github.com/GitoxideLabs/gitoxide/commit/826effb563e369f57b22a8418be2e9ea58fc4844))
    - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))
</details>

## 0.76.0 (2025-12-22)

### New Features

 - <csr-id-580bee9f9cb30beaa31ac5a7c164a91f21e2632d/> respect the `:(optional)` prefix when interpolating paths via `ConfigSnapshot::trusted_path()`.
   Optional, but non-existing paths are now returned as `None`.

### Bug Fixes (BREAKING)

 - <csr-id-1955b5b1a436518c036cfc9bf642170f9c7b6540/> expose raw commit/tag actor headers for round-tripping.
   Note that this means you have to call `CommitRef::commiter|author()?` and `TagRef::tagger()?` instead
   of assuming pre-parsed fields.
   
   This PR makes signature handling truly lossless for "creative" emails and other info.
   We now stash the raw name <email> slice on IdentityRef/SignatureRef and fall back to it when rewriting,
   so even commits with embedded angle brackets round-trip cleanly (might want to expand to other malformed characters before merging?
   Parsing and serialization honor that flag but still keep strict validation for normal input.
   I also added regression coverage for these scenarios.

### Refactor (BREAKING)

 - <csr-id-ab3cce472efc4f6469d99a0d3a425d58ac2140b9/> split async and blocking implementations for Handshake::fetch_or_extract_refmap()
 - <csr-id-c26efb9e733cb33a38ef885eded915e17e61bd9c/> split Handshake::fetch_or_extract_refmap()

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 29 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2291 from j-walther/main ([`e4775d2`](https://github.com/GitoxideLabs/gitoxide/commit/e4775d277bcabc3f407b08d7ec2278e6d1559820))
    - Also call  for the initial connection in shallow clones ([`201ec78`](https://github.com/GitoxideLabs/gitoxide/commit/201ec78be3aec0bd77243e74b725b0fa56dd7b81))
    - Merge pull request #2274 from djc/simplify-ls-refs ([`eab774c`](https://github.com/GitoxideLabs/gitoxide/commit/eab774c3bbc5fc0b4b81877892f12d4491f17565))
    - Refactor ([`9d936fb`](https://github.com/GitoxideLabs/gitoxide/commit/9d936fbb357877c13c0737bd84715e189a3d9dea))
    - Merge pull request #2283 from GitoxideLabs/copilot/add-optional-prefix-support ([`e343ed9`](https://github.com/GitoxideLabs/gitoxide/commit/e343ed9eb1527751e8b76fd83b310b8f2eafbdad))
    - Respect the `:(optional)` prefix when interpolating paths via `ConfigSnapshot::trusted_path()`. ([`580bee9`](https://github.com/GitoxideLabs/gitoxide/commit/580bee9f9cb30beaa31ac5a7c164a91f21e2632d))
    - Split async and blocking implementations for Handshake::fetch_or_extract_refmap() ([`ab3cce4`](https://github.com/GitoxideLabs/gitoxide/commit/ab3cce472efc4f6469d99a0d3a425d58ac2140b9))
    - Split Handshake::fetch_or_extract_refmap() ([`c26efb9`](https://github.com/GitoxideLabs/gitoxide/commit/c26efb9e733cb33a38ef885eded915e17e61bd9c))
    - Merge pull request #2253 from Pingasmaster/raw-email-attempt-fix ([`f471ac5`](https://github.com/GitoxideLabs/gitoxide/commit/f471ac5fece0baf3766d80482400f6b37db8edd0))
    - Refactor ([`6f7b23a`](https://github.com/GitoxideLabs/gitoxide/commit/6f7b23a4b85eb7b8c922753aa86849893d2a089d))
    - Expose raw commit/tag actor headers for round-tripping. ([`1955b5b`](https://github.com/GitoxideLabs/gitoxide/commit/1955b5b1a436518c036cfc9bf642170f9c7b6540))
    - Merge pull request #2275 from GitoxideLabs/dependabot/cargo/cargo-92eaa62a2e ([`93dd630`](https://github.com/GitoxideLabs/gitoxide/commit/93dd630ca6a2a4622ca74d7eaff42ece2750b6c5))
    - Bump the cargo group across 1 directory with 14 updates ([`703644c`](https://github.com/GitoxideLabs/gitoxide/commit/703644c8821aae161592d19495d3b3162133324f))
</details>

## 0.75.0 (2025-11-22)

### New Features

 - <csr-id-a004329195ff9dcbe11e6d52195ff30536fe7a16/> add `Repository::worktree_proxy_by_id(id)`.
   That way it's more straightforward to obtain information about worktrees
   which are known by ID.
 - <csr-id-aeee98234002f0cf79bd3629ada5d3ec3d7b86cd/> add `Handshake::ref_map()` to produce a ref-map from a V1 or V2 handshake.
 - <csr-id-0ed01645e2284f86e1afaf1486f08cad3243ed76/> add `Repository::set_workdir()`.
   Force this repository instance to use the given worktree directory.

### Bug Fixes

 - <csr-id-f7700e452c79ff5582626bb3377d0429a921d869/> refspec for shallow clones uses a single-branch
   When doing shallow clones (depth != NoChange), it now uses a single-branch
   refspec instead of fetching all branches. This matches Git's behavior
   and significantly reduces the repository size for shallow clones.
   
   For shallow clones:
   - If ref_name is specified: uses that branch
   - Otherwise: attempts to detect from Protocol V1 handshake or falls
     back to init.defaultBranch config or "main"
   
   This addresses issue #2227 where `gix clone --depth 1` was creating
   repositories ~130MB vs Git's ~70MB due to fetching all branches.

### Bug Fixes (BREAKING)

 - <csr-id-bb2cec09db45c10e58527b56682e602692a5628d/> respect `diff.algorithm` in `Repository::blame_file`
 - <csr-id-c2050d0ec74e2c2d1d70b68e57bbb79e196796b9/> make gix-transport I/O mode features additive

### Refactor (BREAKING)

 - <csr-id-36c2be80e61a7a0ef637a5385e1960de13261984/> simplify gix-protocol handshake API by avoiding duplicates that remove the `server` parameter.
 - <csr-id-19a005135b91641a19a3a70fd714985ce1203333/> hoist handshake ref handling out of transport code
 - <csr-id-50120de92dab3d8347672df7a43df02291162c9a/> store all_refspecs in Options
 - <csr-id-37ab03687a940c89131141b882e5ef1a7fef85be/> store fetch refspecs in Options
 - <csr-id-f05dfab881481de5de1d23e3266cad9c069f59c5/> flatten RefMap::new() arguments

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 36 commits contributed to the release.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2227](https://github.com/GitoxideLabs/gitoxide/issues/2227)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2227](https://github.com/GitoxideLabs/gitoxide/issues/2227)**
    - Refspec for shallow clones uses a single-branch ([`f7700e4`](https://github.com/GitoxideLabs/gitoxide/commit/f7700e452c79ff5582626bb3377d0429a921d869))
 * **Uncategorized**
    - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
    - Merge branch 'copilot/fix-issue-2258' ([`b20d9e9`](https://github.com/GitoxideLabs/gitoxide/commit/b20d9e94ab000edccf8802f67b8097c9a30db9cb))
    - Fix dead code warnings in gix and gitoxide-core ([`b170451`](https://github.com/GitoxideLabs/gitoxide/commit/b170451e6404d4837ab214605375b067479a1542))
    - Merge pull request #2250 from djc/rm-handshake-layers ([`3c2b422`](https://github.com/GitoxideLabs/gitoxide/commit/3c2b422db647b91b584913b8109b1a6c225d1dd7))
    - Bring the `service` parameter back to not hardcode the handshake for fetches. ([`c9a97db`](https://github.com/GitoxideLabs/gitoxide/commit/c9a97dbe35d1447319fd953062c465cd1c45d9b6))
    - Simplify gix-protocol handshake API by avoiding duplicates that remove the `server` parameter. ([`36c2be8`](https://github.com/GitoxideLabs/gitoxide/commit/36c2be80e61a7a0ef637a5385e1960de13261984))
    - Merge pull request #2252 from GitoxideLabs/improvements ([`2f14246`](https://github.com/GitoxideLabs/gitoxide/commit/2f14246f01029ceee4c39fd9aedffe1fb938ba42))
    - Add `Repository::worktree_proxy_by_id(id)`. ([`a004329`](https://github.com/GitoxideLabs/gitoxide/commit/a004329195ff9dcbe11e6d52195ff30536fe7a16))
    - Merge pull request #2249 from iczero/fetch-performance-fix ([`c3beb20`](https://github.com/GitoxideLabs/gitoxide/commit/c3beb202fbb49686f32d94fe8e59eb46850461f0))
    - Move worktree_branches call out of loop ([`37e7e02`](https://github.com/GitoxideLabs/gitoxide/commit/37e7e022f6c008882d44c16353958bad768230ff))
    - Merge pull request #2242 from djc/ls-refs ([`0ac3080`](https://github.com/GitoxideLabs/gitoxide/commit/0ac3080d00d753429845e994c19b6baa9bccae21))
    - Add `Handshake::ref_map()` to produce a ref-map from a V1 or V2 handshake. ([`aeee982`](https://github.com/GitoxideLabs/gitoxide/commit/aeee98234002f0cf79bd3629ada5d3ec3d7b86cd))
    - Adapt to changes in `gix-protocol`: Outcome -> Handshake ([`a61b2ab`](https://github.com/GitoxideLabs/gitoxide/commit/a61b2abd74e112450788bfa6355dfa8239f360b6))
    - Refactor ([`48fdf5d`](https://github.com/GitoxideLabs/gitoxide/commit/48fdf5d9d32f45811eba9600f2a3f154c69ce5a9))
    - Hoist handshake ref handling out of transport code ([`19a0051`](https://github.com/GitoxideLabs/gitoxide/commit/19a005135b91641a19a3a70fd714985ce1203333))
    - Store all_refspecs in Options ([`50120de`](https://github.com/GitoxideLabs/gitoxide/commit/50120de92dab3d8347672df7a43df02291162c9a))
    - Store fetch refspecs in Options ([`37ab036`](https://github.com/GitoxideLabs/gitoxide/commit/37ab03687a940c89131141b882e5ef1a7fef85be))
    - Flatten RefMap::new() arguments ([`f05dfab`](https://github.com/GitoxideLabs/gitoxide/commit/f05dfab881481de5de1d23e3266cad9c069f59c5))
    - Merge pull request #2238 from GitoxideLabs/copilot/update-refspec-parsing-logic ([`c2c1a61`](https://github.com/GitoxideLabs/gitoxide/commit/c2c1a6177f6197d895cdc11feda9c5d2895ddc7b))
    - Refactor ([`ba2301f`](https://github.com/GitoxideLabs/gitoxide/commit/ba2301f16261e9e1c49b51b94d6ac43548ec2114))
    - Merge pull request #2248 from GitoxideLabs/improvements ([`c400dd3`](https://github.com/GitoxideLabs/gitoxide/commit/c400dd34e29ae3be922e55a282645e9767d36a22))
    - Adapt to changes in `gix-date` ([`2bcac0c`](https://github.com/GitoxideLabs/gitoxide/commit/2bcac0c1115e77c5e20692e7cf0dcb1bbb87149c))
    - Merge pull request #2247 from GitoxideLabs/improvements ([`3087b76`](https://github.com/GitoxideLabs/gitoxide/commit/3087b76b2f2f5a4df2d16a9d2cf9721b470b3fba))
    - Adapt to changes in `gix-date` ([`d5e194d`](https://github.com/GitoxideLabs/gitoxide/commit/d5e194d0ab0e58b02d159b4f48c4e2edd946f355))
    - Merge pull request #2229 from GitoxideLabs/copilot/fix-refspec-for-shallow-clone ([`bc62b1e`](https://github.com/GitoxideLabs/gitoxide/commit/bc62b1ec3c1cb7e75755abfe1a2b3e8615bdfca4))
    - Refactor ([`c331afc`](https://github.com/GitoxideLabs/gitoxide/commit/c331afc01388a37b66eaeb39c302742b3187de27))
    - Merge pull request #2197 from cruessler/add-tests-for-slider-problem ([`ab44f45`](https://github.com/GitoxideLabs/gitoxide/commit/ab44f45f1c80120b2203b72827f7bb45e185d559))
    - Merge pull request #2240 from cruessler/add-gix-repository-blame-options ([`385ab16`](https://github.com/GitoxideLabs/gitoxide/commit/385ab16b591b847b144c1a94de6b46a8ec8fa08a))
    - Refactor ([`2a187ca`](https://github.com/GitoxideLabs/gitoxide/commit/2a187cacbc66dd6c78c0549d9e07f65a0337f0a6))
    - Respect `diff.algorithm` in `Repository::blame_file` ([`bb2cec0`](https://github.com/GitoxideLabs/gitoxide/commit/bb2cec09db45c10e58527b56682e602692a5628d))
    - Merge pull request #2204 from cruessler/improve-blame-ranges ([`663b41e`](https://github.com/GitoxideLabs/gitoxide/commit/663b41eb65f0fffbc8397e91ce5107382b08b441))
    - Add `Repository::set_workdir()`. ([`0ed0164`](https://github.com/GitoxideLabs/gitoxide/commit/0ed01645e2284f86e1afaf1486f08cad3243ed76))
    - Merge pull request #2236 from djc/additive-transport ([`6e89afa`](https://github.com/GitoxideLabs/gitoxide/commit/6e89afacbf1fc8392ce52d5b87f7e85c9bab17a1))
    - Make gix-transport I/O mode features additive ([`c2050d0`](https://github.com/GitoxideLabs/gitoxide/commit/c2050d0ec74e2c2d1d70b68e57bbb79e196796b9))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
</details>

## 0.74.1 (2025-10-23)

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

## 0.74.0 (2025-10-22)

### New Features

 - <csr-id-03804965f5723de001349cf3fe7399ac4815c9ef/> replace `Reference::peel_to_id_in_place_packed`
   Also, update documentation where it was still referring to deprecated
   `in_place` methods to refer to the new methods instead.
 - <csr-id-1d4a7f534d076d2793a8f17a36b11212ddc84d6e/> ability to change the fetch url of a remote
   The remote has a couple of "builder" methods to change
   is fields, e.g. `push_url` for setting the push url.
   
   A builder method for changing the fetch url of a remote
   was missing. This makes it impossible to fully replicate 
   the functionality of `git remote set-url`.
 - <csr-id-b6fbf053925d6530ca97e75bf1d99402eb98f202/> replace `Head::(try_)peel_to_x_in_place` with `Head::peel_to_x`.
   The `_in_place()` suffixed methods are now deprecated.
 - <csr-id-3c0b7374f113017e47d384ccc799a8ada368855c/> Add `Repository::new_commit` and `Repository::new_commit_as()` methods.
 - <csr-id-cdb1100d7ec85868298c5ebe30d2bc5711c4b7eb/> add `Repository::blame_file`
 - <csr-id-fa1287f2e218a0959700cb858b41465f315db9d9/> Add `gix::discover_with_environment_overrides()`
 - <csr-id-6693ab9bfc123eb0cb8fb7a35654e01fe5853318/> support `diff.ignoreSubmodules` overrides in status

### Bug Fixes

 - <csr-id-620d275beacf5f47c2ec56c9d077e7121b9570ca/> deprecate `Remote::push_url*()` in favor of `Remote::with_push*()`.
 - <csr-id-e0879603475051c65cc2fe39f768fc71b53181b6/> remove special handling for empty blob hash to match Git behaviour
   This feature was recently introduced, but was never released.
 - <csr-id-2fc9dbe7e5ee7a0d122ee3ed9c94f7e9f8ee3afc/> empty blob hashes are now automatically considered present.
 - <csr-id-ba563b0e39ebc3ce635f399accd8a4be5210f816/> extend lifetime of iterators
   Previously the iterators would return references with lifetimes that
   were shorter than the actual lifetimes of the `gix::Reference`
   themselves. This was dues to a footgun with `'_` (eliding the lifetime).
   
   When a function returns an elided lifetime, this lifetime usually is the
   lifetime of the `&self` parameter, but sometimes it is the lifetime of
   the type itself (e.g. `Iter<'_>`).
   
   I made the lifetimes explicit to ensure we were using the correct ones.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 65 commits contributed to the release over the course of 99 calendar days.
 - 99 days passed between releases.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Adapt to change in `gix-blame` ([`268dac4`](https://github.com/GitoxideLabs/gitoxide/commit/268dac4a18c8be8bacd89e1890d870253b0e1400))
    - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
    - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
    - Merge pull request #2174 from cruessler/deprecate-in-place-methods ([`442f800`](https://github.com/GitoxideLabs/gitoxide/commit/442f800026d1549eb8352e52178f6b578fc9e3ac))
    - Replace `Reference::peel_to_id_in_place_packed` ([`0380496`](https://github.com/GitoxideLabs/gitoxide/commit/03804965f5723de001349cf3fe7399ac4815c9ef))
    - Merge pull request #2173 from metlos/remote-with-url ([`51f998f`](https://github.com/GitoxideLabs/gitoxide/commit/51f998f60a00db33b3ea78daec747f9da0d0c052))
    - Deprecate `Remote::push_url*()` in favor of `Remote::with_push*()`. ([`620d275`](https://github.com/GitoxideLabs/gitoxide/commit/620d275beacf5f47c2ec56c9d077e7121b9570ca))
    - Ability to change the fetch url of a remote ([`1d4a7f5`](https://github.com/GitoxideLabs/gitoxide/commit/1d4a7f534d076d2793a8f17a36b11212ddc84d6e))
    - Merge pull request #2171 from cruessler/deprecate-in-place-methods-on-head ([`81c0c16`](https://github.com/GitoxideLabs/gitoxide/commit/81c0c1612ddc280edd6e3ceb7f0d7e239516d963))
    - Merge pull request #2170 from GitoxideLabs/copilot/fix-7a3e1d32-c145-43e2-8e87-319b255dbc2f ([`d302fa2`](https://github.com/GitoxideLabs/gitoxide/commit/d302fa20dfbb014de5b989506839889349a7455e))
    - Replace `Head::(try_)peel_to_x_in_place` with `Head::peel_to_x`. ([`b6fbf05`](https://github.com/GitoxideLabs/gitoxide/commit/b6fbf053925d6530ca97e75bf1d99402eb98f202))
    - Refactor ([`3499050`](https://github.com/GitoxideLabs/gitoxide/commit/3499050a972efeba3524748af463fc0f29a1b509))
    - Implement WriteTo trait for gix::Blob with comprehensive tests ([`ca99882`](https://github.com/GitoxideLabs/gitoxide/commit/ca9988284c48e96210ebdc6417b75cc6e8e3b396))
    - Merge pull request #2169 from GitoxideLabs/copilot/fix-54bce74a-dc5e-4361-b53b-326c16b34046 ([`e60e253`](https://github.com/GitoxideLabs/gitoxide/commit/e60e25383103636c1ca711f5ff860ec142e696e2))
    - Refactor ([`b9c7a7e`](https://github.com/GitoxideLabs/gitoxide/commit/b9c7a7e0766e61c47c8abc3a95b65b0a888294ba))
    - Remove special handling for empty blob hash to match Git behaviour ([`e087960`](https://github.com/GitoxideLabs/gitoxide/commit/e0879603475051c65cc2fe39f768fc71b53181b6))
    - Merge pull request #2168 from GitoxideLabs/copilot/fix-01a02b99-91ef-4e27-b90f-19af7d0d252c ([`f891c37`](https://github.com/GitoxideLabs/gitoxide/commit/f891c37edb46a43d3e26d1dc3188f0935b9ab918))
    - Refactor ([`d4c2542`](https://github.com/GitoxideLabs/gitoxide/commit/d4c2542c2a042372b809564d5a62a2f38050fe50))
    - Add `Repository::new_commit` and `Repository::new_commit_as()` methods. ([`3c0b737`](https://github.com/GitoxideLabs/gitoxide/commit/3c0b7374f113017e47d384ccc799a8ada368855c))
    - Merge pull request #2167 from GitoxideLabs/copilot/fix-3952f55e-8faf-4737-886f-09e74cab4ca8 ([`1a4c84d`](https://github.com/GitoxideLabs/gitoxide/commit/1a4c84dd96718526e78e4ba11ead785e724462ed))
    - Refactor ([`689d839`](https://github.com/GitoxideLabs/gitoxide/commit/689d839230cff2cf4407c59d443db841ab846b98))
    - Empty blob hashes are now automatically considered present. ([`2fc9dbe`](https://github.com/GitoxideLabs/gitoxide/commit/2fc9dbe7e5ee7a0d122ee3ed9c94f7e9f8ee3afc))
    - Merge pull request #2163 from cruessler/deprecate-in-place-methods ([`42f8db5`](https://github.com/GitoxideLabs/gitoxide/commit/42f8db5bc9096cdedba497c850c86285ecbacc69))
    - Adapt to changes in `gix-ref` ([`44922d0`](https://github.com/GitoxideLabs/gitoxide/commit/44922d0a71b6cd3595dec8187fa3842097eacd5b))
    - Merge pull request #2153 from cruessler/add-blame-file-on-repository ([`bd47fb5`](https://github.com/GitoxideLabs/gitoxide/commit/bd47fb5b5dc5587550b377dc2dda003c30f5ba48))
    - Add `blame` to extras, and asssure it's tested separately on CI ([`3c54e5b`](https://github.com/GitoxideLabs/gitoxide/commit/3c54e5b8d3d0a770c8dc00155b1a5b4c06446c9d))
    - Merge pull request #2155 from folkertdev/skip-flate2 ([`752d6dc`](https://github.com/GitoxideLabs/gitoxide/commit/752d6dc830f98980c96e9a1dc6ec8bf4432fb7f1))
    - Refactor ([`0e7aa81`](https://github.com/GitoxideLabs/gitoxide/commit/0e7aa815dbe10e9d19334a3f0b881c6d608a3b8e))
    - In `gix-features`, use `libz-rs-sys` directly, skipping `flate2` ([`5a2361b`](https://github.com/GitoxideLabs/gitoxide/commit/5a2361b42b01d77a98c7f16c23783f69a59740f7))
    - Merge pull request #2154 from folkertdev/fix-gix-blame-performance-regression ([`768164a`](https://github.com/GitoxideLabs/gitoxide/commit/768164a877bf1f73cfa72eae9d73ff1949178900))
    - Add `Repository::blame_file` ([`cdb1100`](https://github.com/GitoxideLabs/gitoxide/commit/cdb1100d7ec85868298c5ebe30d2bc5711c4b7eb))
    - Fix `gix-blame` performance regresion ([`8dc5e98`](https://github.com/GitoxideLabs/gitoxide/commit/8dc5e98d9dfe1fdbbd9db15ea149f253c9aaad0e))
    - Merge pull request #2143 from GitoxideLabs/improvements ([`eee5199`](https://github.com/GitoxideLabs/gitoxide/commit/eee5199799357b8ae94cc8a2b75218a284f63183))
    - Fix one failing test on MacOS ([`888b763`](https://github.com/GitoxideLabs/gitoxide/commit/888b76301a804f4a2ae7c9917d3198a6fcc2a5e5))
    - Merge pull request #2137 from tmm1/typo-fix ([`ad1e2ed`](https://github.com/GitoxideLabs/gitoxide/commit/ad1e2ed9ce32d1e21acda2d89d26b11895401877))
    - Fix ([`8e451cf`](https://github.com/GitoxideLabs/gitoxide/commit/8e451cfff1e07bcb65dbda5596c77ffbd54474c9))
    - Merge pull request #2106 from cruessler/add-open-with-environment-overrides ([`c149116`](https://github.com/GitoxideLabs/gitoxide/commit/c1491167c2ea8f8a34bb2e30f9e632f790f0fcd8))
    - Fail `gix tag list` when JSON is requested ([`fb2766b`](https://github.com/GitoxideLabs/gitoxide/commit/fb2766bd006ba1a276dfd5554cf0cdc3b7043ee7))
    - Add `gix::discover_with_environment_overrides()` ([`fa1287f`](https://github.com/GitoxideLabs/gitoxide/commit/fa1287f2e218a0959700cb858b41465f315db9d9))
    - Merge pull request #2132 from EliahKagan/run-ci/overlap ([`a4b0d2c`](https://github.com/GitoxideLabs/gitoxide/commit/a4b0d2cbdd418c8a6c810f3cec7baedda203510c))
    - Add regenerated `make_remote_config_repos` archive ([`c8c84e4`](https://github.com/GitoxideLabs/gitoxide/commit/c8c84e4539273c87f10ece6afdfece4bee8c051f))
    - Use `git config` to make overlapping remotes in a test repo ([`4de96fc`](https://github.com/GitoxideLabs/gitoxide/commit/4de96fcacde66c768abff330352e99d6caae21a6))
    - Make `git remote` failure clearer in `make_remote_config_repos` ([`8cabd16`](https://github.com/GitoxideLabs/gitoxide/commit/8cabd1694441ba78558cd3e6517a7d577e2441a0))
    - Merge pull request #2119 from GitoxideLabs/improvements ([`f3be6e3`](https://github.com/GitoxideLabs/gitoxide/commit/f3be6e380450d6b1e178d2fda0446674429bdfe6))
    - Adapt to changes in `gix-status` ([`5da38e5`](https://github.com/GitoxideLabs/gitoxide/commit/5da38e5a473979965b622869c4676f0f21bed58b))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2110 from jpgrayson/fix/gix-date-parse-raw ([`651f9fa`](https://github.com/GitoxideLabs/gitoxide/commit/651f9fa560d5df7260a45068b8440f72820a6ffd))
    - Release gix-date v0.10.5 ([`4289ae6`](https://github.com/GitoxideLabs/gitoxide/commit/4289ae635d94d713d247eaf6f87d0ba91a1a3826))
    - Merge pull request #2107 from davidkna/feat-diff-ignore-sm ([`d4dd783`](https://github.com/GitoxideLabs/gitoxide/commit/d4dd783def0521e77efb37222b74d44cafafeb02))
    - Refactor ([`453bb2c`](https://github.com/GitoxideLabs/gitoxide/commit/453bb2cfb5f11080065f336c3d16ba701db64991))
    - Support `diff.ignoreSubmodules` overrides in status ([`6693ab9`](https://github.com/GitoxideLabs/gitoxide/commit/6693ab9bfc123eb0cb8fb7a35654e01fe5853318))
    - Merge pull request #2105 from jalil-salame/fix-2103 ([`04a18f3`](https://github.com/GitoxideLabs/gitoxide/commit/04a18f3a4520dd6f49b5f87fe3782dd1cd1547f2))
    - Refactor ([`d4130c3`](https://github.com/GitoxideLabs/gitoxide/commit/d4130c326640f4731ab711be908b2a08b585070a))
    - Extend lifetime of iterators ([`ba563b0`](https://github.com/GitoxideLabs/gitoxide/commit/ba563b0e39ebc3ce635f399accd8a4be5210f816))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2097 from GitoxideLabs/fix-gix-date ([`589d63e`](https://github.com/GitoxideLabs/gitoxide/commit/589d63ed21e5f2cd53ad2cac96fc387df3ea26e9))
    - Release gix-date v0.10.4 ([`007e3f6`](https://github.com/GitoxideLabs/gitoxide/commit/007e3f66246aaafc2374b85cbf77f89ec0b09512))
    - Remove a hack which makes '1979-02-26 18:30:00' special. ([`91b3220`](https://github.com/GitoxideLabs/gitoxide/commit/91b32208dda387916b87fc1d02809a73415a58c0))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.73.0 (2025-07-15)

<csr-id-b7c1f2c25c7485095022fec290492aa4b7c5c5a2/>

### New Features

 - <csr-id-2affbab7491d6b4667572d4d17db864c5b703c7a/> add `repo.references().pseudo()` for traversing refs like `HEAD` and `FETCH_HEAD`.
 - <csr-id-d7db360d5b42ec9d2b4d9977f7b7bee0f6cc4d58/> add `Repository::committer_or_set_generic_fallback()`.
   That way one can always obtain a committer, even though it might
   not represent the entity actually committing.
 - <csr-id-a9befb284dc17d3656cf83859836bc221a42d67e/> add `revision::walk::Platform::hide()`.
   This finally makes safe traversals possible and is what most people would want to use
   instead of `boundary()`.
 - <csr-id-85a24b3a07f08bc83a3ef34c3f07ed00cdbd9fe2/> add `gitoxide.parsePrecious` configuration key to opt-in to precious file parsing.
 - <csr-id-b985766c9c9c5eb09ea4c4b17be9e380bfdad9b4/> add `Repository::is_empty()` to emulate the similar `git2` API
 - <csr-id-f687cb16676dcae37db517c5d6905be08cd9395a/> add `Repository::merge_bases_many()` for simplified retrieval of multiple mergebases.
 - <csr-id-3a5068eb3f9e112cf21c4c6a8bd17aa3081c5edf/> add `tree::EntryRef::to_owned()`.
   That way it's in a more reasonable spot as sibling to `Entry` and it's clearer how to convert noe into the other.
 - <csr-id-3ef6b5595f6d71d27a00b178fbe356257fe4b8a5/> add `EntryRef::kind()` as shortcut for `EntryRef::mode().kind()`.

### Bug Fixes

 - <csr-id-427274bdf64d30e3bcd330e849ea067e359588fe/> don't panic if `remote::Connection::ref_map()` doesn't finish the handshake
 - <csr-id-a75b4a2bc0cc602da336421ebcfda11dd36545b7/> `Repository::branch_remote_ref_name()` won't fail on short names anymore.
   Instead, these partial names are turned into branch names, which seems more
   in line with what Git can do.
 - <csr-id-3f85bf5e97cee359264051bb64357361c7a0f33e/> `strict_config` in conjunction with `GIT_WORK_TREE` no longer triggers an error.

### Other

 - <csr-id-b7c1f2c25c7485095022fec290492aa4b7c5c5a2/> Fixed no_locations options for diffing

### Bug Fixes (BREAKING)

 - <csr-id-26ae766b182218151ae4c3f30306b6d41bab358a/> allow querying `Repository::submodules()` in an unborn repository.
   It's a breaking change merely because the error type changed.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 53 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 13 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1985](https://github.com/GitoxideLabs/gitoxide/issues/1985), [#2055](https://github.com/GitoxideLabs/gitoxide/issues/2055)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1985](https://github.com/GitoxideLabs/gitoxide/issues/1985)**
    - `strict_config` in conjunction with `GIT_WORK_TREE` no longer triggers an error. ([`3f85bf5`](https://github.com/GitoxideLabs/gitoxide/commit/3f85bf5e97cee359264051bb64357361c7a0f33e))
 * **[#2055](https://github.com/GitoxideLabs/gitoxide/issues/2055)**
    - Don't panic if `remote::Connection::ref_map()` doesn't finish the handshake ([`427274b`](https://github.com/GitoxideLabs/gitoxide/commit/427274bdf64d30e3bcd330e849ea067e359588fe))
 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2061 from orthros/pseudo-refs ([`60c29a5`](https://github.com/GitoxideLabs/gitoxide/commit/60c29a59302bfc9d0be7aab5dd3ef05e4ee8e3fa))
    - Refactor ([`43f92b5`](https://github.com/GitoxideLabs/gitoxide/commit/43f92b5285af6696cd21f0e94f3bec568aef8468))
    - Add `repo.references().pseudo()` for traversing refs like `HEAD` and `FETCH_HEAD`. ([`2affbab`](https://github.com/GitoxideLabs/gitoxide/commit/2affbab7491d6b4667572d4d17db864c5b703c7a))
    - Merge pull request #2071 from cruessler/add-accessors-to-change-ref ([`5335c84`](https://github.com/GitoxideLabs/gitoxide/commit/5335c84a68739adc5a7db31220037c83b7be2429))
    - Adapt to changes in `gix-diff` ([`a0cef8b`](https://github.com/GitoxideLabs/gitoxide/commit/a0cef8bd5351acd334459b115c139a9c75e41f55))
    - Merge pull request #2070 from GitoxideLabs/dependabot/cargo/cargo-827bceb7eb ([`dab97f7`](https://github.com/GitoxideLabs/gitoxide/commit/dab97f7618f160421b6e31de8f3e2f3d11dc2ef2))
    - Bump the cargo group across 1 directory with 68 updates ([`a9a8ea1`](https://github.com/GitoxideLabs/gitoxide/commit/a9a8ea1472532dde03bce4e0afdfa82924af1f96))
    - Merge pull request #2065 from cruessler/add-asset-dir-to-blame-copy-royal ([`3f2be40`](https://github.com/GitoxideLabs/gitoxide/commit/3f2be402e20f7642f89721a6a7b9ce7e833dfce7))
    - Fix CI by not using `-t bad` ([`73a30f8`](https://github.com/GitoxideLabs/gitoxide/commit/73a30f8a91fcf5db1244a9a5388e05f4349b0c2e))
    - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
    - Merge pull request #2057 from GitoxideLabs/improvements ([`e8b7a4e`](https://github.com/GitoxideLabs/gitoxide/commit/e8b7a4e9a0d94236af58e693aab2d1b981166704))
    - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
    - Thanks clippy ([`554ce13`](https://github.com/GitoxideLabs/gitoxide/commit/554ce134bc4b514b52a935f17f57f76ebf23ab97))
    - `Repository::branch_remote_ref_name()` won't fail on short names anymore. ([`a75b4a2`](https://github.com/GitoxideLabs/gitoxide/commit/a75b4a2bc0cc602da336421ebcfda11dd36545b7))
    - Merge pull request #2048 from ralphmodales/fetch-without-commiter-config ([`5cf6d05`](https://github.com/GitoxideLabs/gitoxide/commit/5cf6d05e41bf0bf9077be80e158fabc2126d7c7b))
    - Add `Repository::committer_or_set_generic_fallback()`. ([`d7db360`](https://github.com/GitoxideLabs/gitoxide/commit/d7db360d5b42ec9d2b4d9977f7b7bee0f6cc4d58))
    - Add committer fallback for fetch ([`62e4bab`](https://github.com/GitoxideLabs/gitoxide/commit/62e4bab024ee1cdefe4026e35098da8fff18fb0d))
    - Merge pull request #2045 from uberroot4/main ([`298f22e`](https://github.com/GitoxideLabs/gitoxide/commit/298f22ee0086df86e1cae45bcb76cc8b9cad9102))
    - Fixed no_locations options for diffing ([`b7c1f2c`](https://github.com/GitoxideLabs/gitoxide/commit/b7c1f2c25c7485095022fec290492aa4b7c5c5a2))
    - Merge pull request #2037 from GitoxideLabs/hide ([`92febae`](https://github.com/GitoxideLabs/gitoxide/commit/92febae025165c55e596d58511b1634fb6580b9c))
    - Add `revision::walk::Platform::hide()`. ([`a9befb2`](https://github.com/GitoxideLabs/gitoxide/commit/a9befb284dc17d3656cf83859836bc221a42d67e))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2029 from GitoxideLabs/submodule-all ([`b199c6e`](https://github.com/GitoxideLabs/gitoxide/commit/b199c6eacedad0a0617cfae83541b2e7dfd1cefd))
    - Add a test to assure `subomdule.<name>.ignore = all` is handled correctly. ([`657dec4`](https://github.com/GitoxideLabs/gitoxide/commit/657dec4f10bc6babbfa71a4506b1ff1439c06eaf))
    - Merge pull request #2026 from EliahKagan/run-ci/check-msrv-next ([`40f5a56`](https://github.com/GitoxideLabs/gitoxide/commit/40f5a56937ecdd9ecebd5e2d1f28c31d9f6b1b70))
    - Use `gix` manifest `rust-version` in all MSRV checks ([`654a8fa`](https://github.com/GitoxideLabs/gitoxide/commit/654a8fa1a84ac0b9b872aa09b4cbd3cf94157d6f))
    - Merge pull request #2019 from GitoxideLabs/precious-opt-in ([`5f9de52`](https://github.com/GitoxideLabs/gitoxide/commit/5f9de52cf286163b503047b1ab3b51dfa093b4d4))
    - Adapt to changes in `gix-ignore` and `gix-glob`, and more. ([`4ef7806`](https://github.com/GitoxideLabs/gitoxide/commit/4ef7806e62954d069861bddb06cb8c0baf47bb69))
    - Add `gitoxide.parsePrecious` configuration key to opt-in to precious file parsing. ([`85a24b3`](https://github.com/GitoxideLabs/gitoxide/commit/85a24b3a07f08bc83a3ef34c3f07ed00cdbd9fe2))
    - Merge pull request #2016 from GitoxideLabs/improvements ([`7ae3797`](https://github.com/GitoxideLabs/gitoxide/commit/7ae3797f19cf2dd3bc3e02a6437643e5f50ed338))
    - Add `Repository::is_empty()` to emulate the similar `git2` API ([`b985766`](https://github.com/GitoxideLabs/gitoxide/commit/b985766c9c9c5eb09ea4c4b17be9e380bfdad9b4))
    - Allow querying `Repository::submodules()` in an unborn repository. ([`26ae766`](https://github.com/GitoxideLabs/gitoxide/commit/26ae766b182218151ae4c3f30306b6d41bab358a))
    - Merge pull request #2014 from GitoxideLabs/zip ([`648022b`](https://github.com/GitoxideLabs/gitoxide/commit/648022b44e12f597cae55cc45830d0a19b87eb4c))
    - Release gix-glob v0.20.1, gix-attributes v0.26.1, gix-command v0.6.1, gix-filter v0.19.2, gix-worktree-stream v0.21.2, gix-archive v0.21.2 ([`f0ed2cc`](https://github.com/GitoxideLabs/gitoxide/commit/f0ed2cc0046f866e67944bff9aef0579c12d5852))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #2000 from GitoxideLabs/improvements ([`fdfb239`](https://github.com/GitoxideLabs/gitoxide/commit/fdfb2398d09fa496b1daa8e7318acfc40a3bd3f7))
    - Add `Repository::merge_bases_many()` for simplified retrieval of multiple mergebases. ([`f687cb1`](https://github.com/GitoxideLabs/gitoxide/commit/f687cb16676dcae37db517c5d6905be08cd9395a))
    - Merge pull request #1993 from EliahKagan/run-ci/baseline ([`7a33e2a`](https://github.com/GitoxideLabs/gitoxide/commit/7a33e2a0496e3456fcda09428f37c20907a015bb))
    - Completely remove `:/` baseline skip ([`b623bf1`](https://github.com/GitoxideLabs/gitoxide/commit/b623bf1802474d92dbd0b63856c0b3b1f664e8d7))
    - Flip `:/` baseline skip from CI to local and extend ([`2400158`](https://github.com/GitoxideLabs/gitoxide/commit/2400158d6ce2ff28d428402f2d4030c04cd5f470))
    - Merge pull request #1987 from GitoxideLabs/fix-1985 ([`189d1a0`](https://github.com/GitoxideLabs/gitoxide/commit/189d1a0a8674e52e9ad2393fc296f3231e85e689))
    - Merge pull request #1975 from GitoxideLabs/improvements ([`28935a5`](https://github.com/GitoxideLabs/gitoxide/commit/28935a56ff91f1fc2c17a7d23b057cf7119144e9))
    - Add `tree::EntryRef::to_owned()`. ([`3a5068e`](https://github.com/GitoxideLabs/gitoxide/commit/3a5068eb3f9e112cf21c4c6a8bd17aa3081c5edf))
    - Merge pull request #1977 from GitoxideLabs/dependabot/cargo/cargo-811d7b929d ([`800738a`](https://github.com/GitoxideLabs/gitoxide/commit/800738a37f3d33926a427edfa294423bbe3f2b66))
    - Bump the cargo group with 12 updates ([`4408166`](https://github.com/GitoxideLabs/gitoxide/commit/4408166bf56197a67419277a4ef8feeba9060fee))
    - Add `EntryRef::kind()` as shortcut for `EntryRef::mode().kind()`. ([`3ef6b55`](https://github.com/GitoxideLabs/gitoxide/commit/3ef6b5595f6d71d27a00b178fbe356257fe4b8a5))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.72.1 (2025-04-26)

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

## 0.72.0 (2025-04-25)

### Bug Fixes

 - <csr-id-b07f907ba2e01849744c72df35dac57b624f2f85/> Adapt to changes in gix-actor
   Use the committer date and author date that are now backed by bytes and
   interpret these bytes into a `gix_date::Time` on demand.
 - <csr-id-ca165174e3502aa75e74ef90728d192f517f4406/> correctly handle safe.directory for worktrees
 - <csr-id-24d235d7a6bdb01c5c1cff79b5be14f1d5afa11a/> `safe.directory` now applies to configuration as well
   This means that repo-local configuration that is considered safe, ideally with
   `safe.directory=safe/dir/*` notation, will be usable for sensitive operations.
 - <csr-id-38b63c2fc9d407b3c634d8b0c72d4d0c104aa5ad/> make `fs::walkdir_sorted_new()` sort entries by paths literally
   This follows up 7b1b5bf864e74706aefeb1213e8bdb0545d5464a. Since packed-refs
   appears to be sorted by full ref name, loose-refs should also be emitted in
   that order.
   
   The comparison function is copied from gix::diff::object::tree::EntryRef.
   Non-utf8 file names are simply mapped to "" on Windows. We could add some
   fallback, but callers can't handle such file names anyway.

### New Features (BREAKING)

 - <csr-id-a6d7d701517f7eafe1e2ed15d564114aa5c1d07c/> use `RelativePath` for prefixed ref iteration.
   Its type captures the requirements better.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 41 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1788](https://github.com/GitoxideLabs/gitoxide/issues/1788), [#1912](https://github.com/GitoxideLabs/gitoxide/issues/1912), [#1928](https://github.com/GitoxideLabs/gitoxide/issues/1928)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1788](https://github.com/GitoxideLabs/gitoxide/issues/1788)**
    - Add test to assure dynamic allocation of slots works ([`5396b2b`](https://github.com/GitoxideLabs/gitoxide/commit/5396b2b6318d2066b07b23a58286d71344d804d3))
 * **[#1912](https://github.com/GitoxideLabs/gitoxide/issues/1912)**
    - Correctly handle safe.directory for worktrees ([`ca16517`](https://github.com/GitoxideLabs/gitoxide/commit/ca165174e3502aa75e74ef90728d192f517f4406))
    - `safe.directory` now applies to configuration as well ([`24d235d`](https://github.com/GitoxideLabs/gitoxide/commit/24d235d7a6bdb01c5c1cff79b5be14f1d5afa11a))
 * **[#1928](https://github.com/GitoxideLabs/gitoxide/issues/1928)**
    - Make `fs::walkdir_sorted_new()` sort entries by paths literally ([`38b63c2`](https://github.com/GitoxideLabs/gitoxide/commit/38b63c2fc9d407b3c634d8b0c72d4d0c104aa5ad))
 * **Uncategorized**
    - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
    - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
    - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
    - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
    - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
    - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
    - Thanks clippy ([`6f009d7`](https://github.com/GitoxideLabs/gitoxide/commit/6f009d781da9e931d44b113a925a80e77e8788af))
    - Adapt to changes in `gix-date` and `gix-actor` ([`afdf1a5`](https://github.com/GitoxideLabs/gitoxide/commit/afdf1a5d5c9fb2645f481c17f580ad59d14d6095))
    - Apply feedback from discussion ([`70097c0`](https://github.com/GitoxideLabs/gitoxide/commit/70097c0feb481541ed96358842de96d6b1af24a9))
    - Adapt to changes in gix-actor ([`b07f907`](https://github.com/GitoxideLabs/gitoxide/commit/b07f907ba2e01849744c72df35dac57b624f2f85))
    - Merge pull request #1965 from GitoxideLabs/report ([`737bb49`](https://github.com/GitoxideLabs/gitoxide/commit/737bb49639db74c154a4380c97271f4b8d17661c))
    - Merge pull request #1964 from GitoxideLabs/fix-1912 ([`359914c`](https://github.com/GitoxideLabs/gitoxide/commit/359914ce567d90d2db52b605bc126ad23db7f039))
    - Merge pull request #1963 from joshtriplett/zlib-rs-default ([`9e075b9`](https://github.com/GitoxideLabs/gitoxide/commit/9e075b99ffc79173d4052d7550fd1d2826c5ec71))
    - Switch to zlib-rs by default and drop other zlib backends ([`96164c5`](https://github.com/GitoxideLabs/gitoxide/commit/96164c5936032b4edb973828178cc55793dd57cc))
    - Merge pull request #1921 from cruessler/introduce-repository-path ([`fdc06b1`](https://github.com/GitoxideLabs/gitoxide/commit/fdc06b139a331bd2b345d34f09482317388fcba8))
    - Refactor ([`294902e`](https://github.com/GitoxideLabs/gitoxide/commit/294902e0dbc350a33a0e54164eed626720c1a1d7))
    - Use `RelativePath` for prefixed ref iteration. ([`a6d7d70`](https://github.com/GitoxideLabs/gitoxide/commit/a6d7d701517f7eafe1e2ed15d564114aa5c1d07c))
    - Merge pull request #1853 from GitoxideLabs/odb-issue ([`cd1a777`](https://github.com/GitoxideLabs/gitoxide/commit/cd1a77777f8a5b162b14ec4d0d3f476a0387d7f1))
    - Merge pull request #1825 from DianaNites/diananites-reftable ([`edb449c`](https://github.com/GitoxideLabs/gitoxide/commit/edb449c9dd60f74562dc78a33e41cfcb5d7be81e))
    - Show that ref-tables aren't currently supported. ([`3c16e53`](https://github.com/GitoxideLabs/gitoxide/commit/3c16e534a5db85a73f9dfbc6b4846ff6f89d57b2))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1954 from GitoxideLabs/fix-recursive-list-refs-prefix ([`71275d1`](https://github.com/GitoxideLabs/gitoxide/commit/71275d16b6a3a22b5e6e33f441d50fc6d44ff20e))
    - Adapt to changes in `gix-ref`. ([`52142b4`](https://github.com/GitoxideLabs/gitoxide/commit/52142b472918844f5d4f752aa343ee9e4f9f7c30))
    - Fix ci failures ([`57c9014`](https://github.com/GitoxideLabs/gitoxide/commit/57c9014d4f17f00ceb7fd2e3ca6b80f081af3356))
    - Merge pull request #1953 from GitoxideLabs/dependabot/cargo/cargo-4a3cda0de8 ([`3aec7fb`](https://github.com/GitoxideLabs/gitoxide/commit/3aec7fbac52377bdeebc49759d4e0420b18b4e81))
    - Bump the cargo group with 3 updates ([`9f1fbc7`](https://github.com/GitoxideLabs/gitoxide/commit/9f1fbc741e1b6c718c7787f2858e07f3bd5473e9))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Use Into<Cow<>> ([`507d682`](https://github.com/GitoxideLabs/gitoxide/commit/507d682c08dcda29f044068c13ce87678c1b2a5e))
    - Handle trailing slash in ref list prefix filtering ([`3ca6811`](https://github.com/GitoxideLabs/gitoxide/commit/3ca6811418275c4ebede0993b9741e804e81094b))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1931 from yuja/push-klrqpplwxrkx ([`7502b4a`](https://github.com/GitoxideLabs/gitoxide/commit/7502b4abde6196b982cf66344c0df992e99493cb))
    - Merge pull request #1917 from pierrechevalier83/issue_1887 ([`6307f57`](https://github.com/GitoxideLabs/gitoxide/commit/6307f571f969eb7ff2490e4c68dc7994fb2fecac))
    - Adapt to changes in gix-object ([`8969245`](https://github.com/GitoxideLabs/gitoxide/commit/8969245a6b5874d8524f508569ae1266e48d100e))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.71.0 (2025-04-04)

<csr-id-866affde8ef17f201884b8a4b36cc4c7f449d6fe/>

### Changed

 - <csr-id-9800e9c25c2a61dbe5d502270a962f20bf305f47/> read config losslessly even without `debug_assertions`
   This should hopefully not be a breaking change, as the same code
   could produce the same behaviour if compiled with different flags,
   and the semantic meaning of the resulting configuration should be
   the same. But Hyrum’s law is always lurking…

### Documentation

 - <csr-id-687322b45787a2552fb16a8d1fed8630318f9148/> specify ThreadSafeRepository is not Send/Sync without "parallel"

### New Features

 - <csr-id-5054780967ef7d5651d358337aa78e09b9007c38/> add `Repository::checkout_options()`.
   It's a low-level set of options to drive (quite unsafe) checkouts.
   They are unsafe as they may be configured to overwrite, and are in no
   way similar to `git checkout`.
 - <csr-id-02878c9c3418c904c61c8e0ef02f9448b11fd14c/> add `Repository::head_tree_id_or_empty()` for convenience.
 - <csr-id-776f9bec5bb0ad04ccc6e082d10eb421de1483ee/> add `Repository::workdir_path()` to easily obtain a `Path` for worktree items.
 - <csr-id-518fbbc7f4e66c908f33cc3b3dcf5f0a7d6f5d1b/> add `Repository::workdir()` as replacement for `Repository::work_dir()`.
   Keep the latter as deprecated though.
 - <csr-id-27e62d7e741b56418e174db0495a9ddc32f11b47/> `filter::Pipeline::worktree_file_to_object()` now can add `Commit` type objects.
 - <csr-id-70ebd5f4128e7dcf83175ca05a70741434b71379/> add `filter::Pipeline::worktree_file_to_object()`.
   That way it's easier to correctly add whole files into the object
   database.
 - <csr-id-23d2bed707e0c0cb164f3f279849536688ffa4c1/> make internal `repo` fields public for ease of use.
   That way, functions or methods taking such a type as argument
   have access to the underlying repository so it doesn't need
   to be passed as separate argument.
 - <csr-id-37582b089357ba9e06547374ea651c51d3608890/> add `blob::platform::Resource::intern_source_strip_newline_separators()`
   That way it will be easier to have typical Git-style patches diffs around
   files that don't end with a newline.
 - <csr-id-f3257f3c0fd1b91aecdb8f1d947e9648080d162f/> add `Repository::big_file_threshold()` to easily learn what Git considers a big file.

### Bug Fixes

 - <csr-id-aa8daf89bcc3c26baeb7d850c19bb9a5d403f555/> Don't panic when rev-parsing `^^^` and similar
 - <csr-id-dcdb8eae4b5c7897bef752a66bc17a8ee662da97/> `filter::Pipeline::convert_to_git()` now also works on Windows under all circumstances.
 - <csr-id-9bec947deb4313a9d1eb701555d96ea829132976/> assure `Repository::commit_as()` also uses the committer for reflogs
   Previously it would retrieve the configured committer, or trigger an error
   if there was none despite the commiter being provided to `commit_as()`.
   
   This als adds `Repository::edit_references_as(committer)` to allow passing
   a given committer.

### Other

 - <csr-id-866affde8ef17f201884b8a4b36cc4c7f449d6fe/> `Repository::commit()` now explains how to create a commit without ref updates.

### Changed (BREAKING)

 - <csr-id-fd12ef89af29bf0684fc1df3e7b76ff367dee994/> drop obsolete SHA‐1 features
   The hashing API has moved to `gix_hash::hasher`, and we now use
   `sha1-checked` unconditionally.

### Bug Fixes (BREAKING)

 - <csr-id-b78e7dd99230077b694434e0ed7d236f97aba046/> make clear what `with_pruned()` is doing by renaming it to `with_boundary()`.
   This is how it acts, and it's not at all the same as `hide()` in `git2`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 58 commits contributed to the release.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1829](https://github.com/GitoxideLabs/gitoxide/issues/1829), [#1914](https://github.com/GitoxideLabs/gitoxide/issues/1914)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1829](https://github.com/GitoxideLabs/gitoxide/issues/1829)**
    - Assure `Repository::commit_as()` also uses the committer for reflogs ([`9bec947`](https://github.com/GitoxideLabs/gitoxide/commit/9bec947deb4313a9d1eb701555d96ea829132976))
 * **[#1914](https://github.com/GitoxideLabs/gitoxide/issues/1914)**
    - Don't panic when rev-parsing `^^^` and similar ([`aa8daf8`](https://github.com/GitoxideLabs/gitoxide/commit/aa8daf89bcc3c26baeb7d850c19bb9a5d403f555))
 * **Uncategorized**
    - Release gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`d248e3d`](https://github.com/GitoxideLabs/gitoxide/commit/d248e3d87d45ca3983cb9fd7c6143dacbd8301cc))
    - Release gix-sec v0.10.12, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`ada5a94`](https://github.com/GitoxideLabs/gitoxide/commit/ada5a9447dc3c210afbd8866fe939c3f3a024226))
    - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
    - Update changelogs prior to release ([`38dff41`](https://github.com/GitoxideLabs/gitoxide/commit/38dff41d09b6841ff52435464e77cd012dce7645))
    - Merge pull request #1915 from emilazy/push-qvyqmopsoltr ([`4660f7a`](https://github.com/GitoxideLabs/gitoxide/commit/4660f7a6f71873311f68f170b0f1f6659a02829d))
    - Migrate `gix_object::{try_ =>}compute_hash` users ([`3d7e379`](https://github.com/GitoxideLabs/gitoxide/commit/3d7e379f26cbe53ddb430427b8e88ce0966be456))
    - Migrate hashing API users to fallible versions ([`fbf6cc8`](https://github.com/GitoxideLabs/gitoxide/commit/fbf6cc897cfeff5ed2a2d5946c060e0cebbd1afd))
    - Drop obsolete SHA‐1 features ([`fd12ef8`](https://github.com/GitoxideLabs/gitoxide/commit/fd12ef89af29bf0684fc1df3e7b76ff367dee994))
    - Merge pull request #1851 from GitoxideLabs/fix-1850 ([`cd96b64`](https://github.com/GitoxideLabs/gitoxide/commit/cd96b6439d119c5189a8e7349d2e7e2533db41b5))
    - Adapt to changes in `gix-features` ([`5f8bff8`](https://github.com/GitoxideLabs/gitoxide/commit/5f8bff844420a2ea1fb1f949650451d235251185))
    - Merge pull request #1916 from GitoxideLabs/fix-1914 ([`32b54b3`](https://github.com/GitoxideLabs/gitoxide/commit/32b54b3ab7f101c6b9cd7c3349153c2fc71e496d))
    - Merge pull request #1909 from cruessler/take-to-components-in-fs-stack ([`5cb5337`](https://github.com/GitoxideLabs/gitoxide/commit/5cb5337efd7679d8a2ab4bd5e6a5da8c366f7f1a))
    - Use `gix_fs::stack::ToNormalPathComponents` everywhere. ([`1f98edb`](https://github.com/GitoxideLabs/gitoxide/commit/1f98edbaa51caaf152eda289b769388676259a06))
    - Update MSRV to 1.75 for access to `impl` returns in traits. ([`569c186`](https://github.com/GitoxideLabs/gitoxide/commit/569c18685e714f9d89946ec69be4116d02f74a2a))
    - Merge pull request #1911 from GitoxideLabs/improvements ([`bfa3253`](https://github.com/GitoxideLabs/gitoxide/commit/bfa32530c99ce7c7f7360b41a0d49183ac88cec4))
    - `filter::Pipeline::convert_to_git()` now also works on Windows under all circumstances. ([`dcdb8ea`](https://github.com/GitoxideLabs/gitoxide/commit/dcdb8eae4b5c7897bef752a66bc17a8ee662da97))
    - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
    - Drop trailing `,` just before `)` on same line in function calls ([`66a5ae1`](https://github.com/GitoxideLabs/gitoxide/commit/66a5ae1b586d583066402c801213a55141e2aad6))
    - Use raw literals for more strings with backslashes ([`01bd76d`](https://github.com/GitoxideLabs/gitoxide/commit/01bd76dcacb69d9c21f2fc6063e273a01aebf94f))
    - Merge pull request #1898 from GitoxideLabs/improvements ([`7255a5f`](https://github.com/GitoxideLabs/gitoxide/commit/7255a5fc0aa790b54e3176e8ecf066457acd9eef))
    - Improve documentation of a field that one can easily get wrong otherwise. ([`5a1b3d6`](https://github.com/GitoxideLabs/gitoxide/commit/5a1b3d66b161a00c47f35cb5ad92f1c40554e538))
    - Merge pull request #1873 from NobodyXu/zlib-rs ([`316f113`](https://github.com/GitoxideLabs/gitoxide/commit/316f11322f156760a0e344a3bda33e11ca4e8862))
    - Review adjustments for zlib-rs support. ([`5e618b6`](https://github.com/GitoxideLabs/gitoxide/commit/5e618b6e7632a037326d759678bef452b32a3b30))
    - Add new feature zlib-rs ([`8b1b55c`](https://github.com/GitoxideLabs/gitoxide/commit/8b1b55c337e65071156856771daee3cbcead1e24))
    - Revert "Instrument make_remote_repos.sh to view `config` corruption" ([`9061fc4`](https://github.com/GitoxideLabs/gitoxide/commit/9061fc4260fe0d7b2c1ba345ae7923f2d3e37ad4))
    - Instrument make_remote_repos.sh to view `config` corruption ([`d290ad9`](https://github.com/GitoxideLabs/gitoxide/commit/d290ad962fe88e2aa28d23d412117f59ee5664c0))
    - Merge pull request #1884 from GitoxideLabs/improvements ([`0bf1d5b`](https://github.com/GitoxideLabs/gitoxide/commit/0bf1d5b9f0b0971b9f25a8e44b7818e37c78d68e))
    - Merge pull request #1876 from joshtriplett/fix-tests-in-environments-with-env-variables-set ([`dc8bd63`](https://github.com/GitoxideLabs/gitoxide/commit/dc8bd63f608d6704d76c2fd68d2a3c9d425ce1c8))
    - Fix tests when `GIT_AUTHOR_NAME` or `GIT_COMMITTER_NAME` are set ([`94dda22`](https://github.com/GitoxideLabs/gitoxide/commit/94dda22aa9e920de6ff3c1f076d5d1f5e6e5c4a6))
    - Add `Repository::checkout_options()`. ([`5054780`](https://github.com/GitoxideLabs/gitoxide/commit/5054780967ef7d5651d358337aa78e09b9007c38))
    - Add `Repository::head_tree_id_or_empty()` for convenience. ([`02878c9`](https://github.com/GitoxideLabs/gitoxide/commit/02878c9c3418c904c61c8e0ef02f9448b11fd14c))
    - Add `Repository::workdir_path()` to easily obtain a `Path` for worktree items. ([`776f9be`](https://github.com/GitoxideLabs/gitoxide/commit/776f9bec5bb0ad04ccc6e082d10eb421de1483ee))
    - Add `Repository::workdir()` as replacement for `Repository::work_dir()`. ([`518fbbc`](https://github.com/GitoxideLabs/gitoxide/commit/518fbbc7f4e66c908f33cc3b3dcf5f0a7d6f5d1b))
    - Merge pull request #1882 from emilazy/push-ylwwuwymlmwt ([`10e41ee`](https://github.com/GitoxideLabs/gitoxide/commit/10e41ee6d1d3607c3d26a66b488d7d1eabc45c6e))
    - Fix cargo-deny using a prodash-update and ignore directive ([`cf7f34d`](https://github.com/GitoxideLabs/gitoxide/commit/cf7f34dcd653ddafaaecb149d4b98efa97d5b871))
    - Read config losslessly even without `debug_assertions` ([`9800e9c`](https://github.com/GitoxideLabs/gitoxide/commit/9800e9c25c2a61dbe5d502270a962f20bf305f47))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1837 from GitoxideLabs/improvements ([`b4fe425`](https://github.com/GitoxideLabs/gitoxide/commit/b4fe425a1a7823790fab592c84aa8494d295640d))
    - `Repository::commit()` now explains how to create a commit without ref updates. ([`866affd`](https://github.com/GitoxideLabs/gitoxide/commit/866affde8ef17f201884b8a4b36cc4c7f449d6fe))
    - Merge pull request #1835 from GitoxideLabs/fixes ([`503098d`](https://github.com/GitoxideLabs/gitoxide/commit/503098d1f93853502083fc4bf51675784879be12))
    - Merge pull request #1834 from GitoxideLabs/improvements ([`5c327bb`](https://github.com/GitoxideLabs/gitoxide/commit/5c327bbfeb7c685a93962e087f72d1083a768b2d))
    - `filter::Pipeline::worktree_file_to_object()` now can add `Commit` type objects. ([`27e62d7`](https://github.com/GitoxideLabs/gitoxide/commit/27e62d7e741b56418e174db0495a9ddc32f11b47))
    - Merge pull request #1833 from GitoxideLabs/improvements ([`c042813`](https://github.com/GitoxideLabs/gitoxide/commit/c042813b13fadd414134dd6fc93b13d4da49577d))
    - Add `filter::Pipeline::worktree_file_to_object()`. ([`70ebd5f`](https://github.com/GitoxideLabs/gitoxide/commit/70ebd5f4128e7dcf83175ca05a70741434b71379))
    - Make internal `repo` fields public for ease of use. ([`23d2bed`](https://github.com/GitoxideLabs/gitoxide/commit/23d2bed707e0c0cb164f3f279849536688ffa4c1))
    - Merge pull request #1821 from GitoxideLabs/improvements ([`914bf28`](https://github.com/GitoxideLabs/gitoxide/commit/914bf28409e8c319c25967f2bdb6aa71f2255879))
    - Add `blob::platform::Resource::intern_source_strip_newline_separators()` ([`37582b0`](https://github.com/GitoxideLabs/gitoxide/commit/37582b089357ba9e06547374ea651c51d3608890))
    - Merge pull request #1820 from GitoxideLabs/improvements ([`daa6d4a`](https://github.com/GitoxideLabs/gitoxide/commit/daa6d4a62489d16a6520c644f2b2ca180c9f5733))
    - Make clear what `with_pruned()` is doing by renaming it to `with_boundary()`. ([`b78e7dd`](https://github.com/GitoxideLabs/gitoxide/commit/b78e7dd99230077b694434e0ed7d236f97aba046))
    - Merge pull request #1807 from bryceberger/bryce/push-xqrmpyoxlosq ([`79cb655`](https://github.com/GitoxideLabs/gitoxide/commit/79cb65588aa9107b35706f0d6e16772fd43dc96e))
    - Refactor ([`d7ddbb7`](https://github.com/GitoxideLabs/gitoxide/commit/d7ddbb73da487ba5dbc5d3c64eb764a9eb50b554))
    - Specify ThreadSafeRepository is not Send/Sync without "parallel" ([`687322b`](https://github.com/GitoxideLabs/gitoxide/commit/687322b45787a2552fb16a8d1fed8630318f9148))
    - Merge pull request #1785 from GitoxideLabs/improvements ([`1a69c40`](https://github.com/GitoxideLabs/gitoxide/commit/1a69c4080bc38ef9151bc8ebfb9d5f87f19b5755))
    - Add `Repository::big_file_threshold()` to easily learn what Git considers a big file. ([`f3257f3`](https://github.com/GitoxideLabs/gitoxide/commit/f3257f3c0fd1b91aecdb8f1d947e9648080d162f))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.70.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>
<csr-id-9db21601b61601c01cd2419543e2c461a7dd568d/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### New Features

 - <csr-id-da0e1c7a442e67a73a080ed2ffe80c65ed7851ed/> add `Repository::upstream_branch_and_remote_name_for_tracking_branch()`
   It's a way to learn about the Remote and upstream branch which would
   match the given local tracking branch.
 - <csr-id-5b6e5c8b0cfa714d0de0dc89d0a8fa7794e102a0/> more often check for interrupts in status iterator
 - <csr-id-3b53982db092e6c57a4ab9c979f7c104a7ced207/> add `tree::Editor|editor::Cursor::get()` to see if an entry is loaded at path.
   This can be useful to get a feeling for how far the tree was already made available,
   even though it won't reveal if an entry was edited.
 - <csr-id-8ae9e5729bd9e7d6308bd226f510b3415381de89/> `Repository::is_dirty()` now also checks for tree/index changes.
   This copmpletes the `is_dirty()` implementation.
 - <csr-id-83f3d93eaa1d7a96e0fa60840502f211c20edc3b/> `Repository::tree_index_status()` to see the changes between a tree and an index.
   It also respects `status.rename` and `status.renameLimit` to configure rename tracking.
 - <csr-id-592e250f8f01788d37f9fb7b1938b67446042bf3/> add `Tree::depthfirst()` with a delegate.
   This allows a depth-first traversal with a delegate.
 - <csr-id-25efbfb72e5a043ce8f7d196c1f7104ef93394df/> Add `blame` plumbing crate to the top-level.
   For now, it doesn't come with a simplified `gix` API though.

### Bug Fixes

 - <csr-id-cd8fabf583e75f59feda7a78b8710f26a8200cbb/> `Repository::status()` detects files added to the index in an unborn repository.
   Previously it wouldn't show them.
 - <csr-id-84019cb42cff153c305cb718307493df42a134a4/> `Respository::status()` iterator won't fail in unborn directories.
 - <csr-id-bc022845ace1962a2a85f9272cdbc0cf24745c62/> worktrees of submodules now know their correct worktree
   Previously they would use a very incorrect worktree which would cause
   the status to be calculated very wrongly.
 - <csr-id-3bbd1f7b60b09f9862ee88293c316a359d79e3d8/> status-iterator won't swallow legitimate modification during 'racy-git'.
   When a modification is marked as being racy, then previously the iterator would have
   kept the whole modification even though it should just have tracked the single change.
   
   This made the legitimate modification disappear.
 - <csr-id-a03bde58176e68850fa2d3299f9901a9b36b892f/> `write_blob_stream()` does not need `Seek` trait anymore.
   Internally, it has to turn it into a buffer so it's not needed anymore.
   It also counteracts the idea of using a stream with arbitrarily big files.
 - <csr-id-a987e682aefa352ceaffa05d56545c9bc9c14934/> `Submodule::status()` now konws about tree-index changes as well.
   This completes the status implementation.
 - <csr-id-51a430114493de392ce0c60f462d6e3ff36475a4/> remove unused fetch-error variants
   Note that it's a breaking change, but it's on top of a previous breaking change
   so folks would already have to update explicitly.

### Other

 - <csr-id-9db21601b61601c01cd2419543e2c461a7dd568d/> make really clear that `Repository::worktrees()` lists linked worktrees.
   Excluding the main worktree which isn't always present.

### New Features (BREAKING)

 - <csr-id-801689b4aa860e1054dd9362a59d76077f31f248/> add `status::Platform::into_iter()` for obtaining a complete status.
   Note that it is still possible to disable the head-index status.
   
   Types moved around, effectivey removing the `iter::` module for most
   more general types, i.e. those that are quite genericlally useful in
   a status.

### Bug Fixes (BREAKING)

 - <csr-id-a6f397f529953ac4177962059c9e6d9bcee2b657/> all `config::Snapshot` access now uses the new `Key` trait.
   That way one can officially use "section.name" strings or `&Section::NAME`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 41 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1770](https://github.com/GitoxideLabs/gitoxide/issues/1770)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1770](https://github.com/GitoxideLabs/gitoxide/issues/1770)**
    - `Repository::status()` detects files added to the index in an unborn repository. ([`cd8fabf`](https://github.com/GitoxideLabs/gitoxide/commit/cd8fabf583e75f59feda7a78b8710f26a8200cbb))
 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1774 from EliahKagan/complex-graph-no-baseline-next ([`90e08f1`](https://github.com/GitoxideLabs/gitoxide/commit/90e08f18d9cd2630f245d3a190e7bc5585bd4bc7))
    - Use parse_spec_no_baseline with :/ for all 2.47.* on CI ([`fe33fa7`](https://github.com/GitoxideLabs/gitoxide/commit/fe33fa7ab639ee0005167fd7a16712446fa522bb))
    - Merge pull request #1772 from GitoxideLabs/improvements ([`4c8200f`](https://github.com/GitoxideLabs/gitoxide/commit/4c8200f374b146456df7568fe5a7e9c3d10b8502))
    - Merge pull request #1769 from GitoxideLabs/improvements ([`47e44c5`](https://github.com/GitoxideLabs/gitoxide/commit/47e44c51f6dfb8f70b12633dd95698d481077bea))
    - `Respository::status()` iterator won't fail in unborn directories. ([`84019cb`](https://github.com/GitoxideLabs/gitoxide/commit/84019cb42cff153c305cb718307493df42a134a4))
    - Merge pull request #1768 from GitoxideLabs/improvements ([`34fa6bb`](https://github.com/GitoxideLabs/gitoxide/commit/34fa6bbcdaafa9a690dd7504c42d286e4dce0fd1))
    - Adapt to changes in `gix-status` ([`25d480c`](https://github.com/GitoxideLabs/gitoxide/commit/25d480c1e48ffc89431cbdddb1e028c8d399e6d9))
    - Merge pull request #1750 from GitoxideLabs/odb-issue ([`e4fb21e`](https://github.com/GitoxideLabs/gitoxide/commit/e4fb21eb73e7cdb43e30df49eb72512e2836dad1))
    - Reproduce issue with 'too many packs' for slotmap ([`dbf079f`](https://github.com/GitoxideLabs/gitoxide/commit/dbf079f4b70db01ae4f850796ae6006d45d3f99c))
    - Merge pull request #1763 from GitoxideLabs/better-refspec-primitives ([`af8f201`](https://github.com/GitoxideLabs/gitoxide/commit/af8f2019723dd9ee3ac46a935910946fcc15e8bb))
    - Add `Repository::upstream_branch_and_remote_name_for_tracking_branch()` ([`da0e1c7`](https://github.com/GitoxideLabs/gitoxide/commit/da0e1c7a442e67a73a080ed2ffe80c65ed7851ed))
    - Adapt to changes in `gix-refspec` ([`6d7dd9b`](https://github.com/GitoxideLabs/gitoxide/commit/6d7dd9bced4a1a0e8175e047be838746a95aa596))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Make really clear that `Repository::worktrees()` lists linked worktrees. ([`9db2160`](https://github.com/GitoxideLabs/gitoxide/commit/9db21601b61601c01cd2419543e2c461a7dd568d))
    - Worktrees of submodules now know their correct worktree ([`bc02284`](https://github.com/GitoxideLabs/gitoxide/commit/bc022845ace1962a2a85f9272cdbc0cf24745c62))
    - Merge pull request #1752 from GitoxideLabs/git-shell ([`1ca480a`](https://github.com/GitoxideLabs/gitoxide/commit/1ca480aa4093328a7e047e770fdffdb8cc6d8e8d))
    - Thanks clippy ([`9193b05`](https://github.com/GitoxideLabs/gitoxide/commit/9193b05b2528f62d829447ccc50314bd4cffc415))
    - Merge pull request #1749 from GitoxideLabs/status ([`8d84818`](https://github.com/GitoxideLabs/gitoxide/commit/8d84818240d44e1f5fe78a231b5d9bffd0283918))
    - More often check for interrupts in status iterator ([`5b6e5c8`](https://github.com/GitoxideLabs/gitoxide/commit/5b6e5c8b0cfa714d0de0dc89d0a8fa7794e102a0))
    - Merge pull request #1746 from GitoxideLabs/status ([`af704f5`](https://github.com/GitoxideLabs/gitoxide/commit/af704f57bb9480c47cdd393465264d586f1d4562))
    - Add `tree::Editor|editor::Cursor::get()` to see if an entry is loaded at path. ([`3b53982`](https://github.com/GitoxideLabs/gitoxide/commit/3b53982db092e6c57a4ab9c979f7c104a7ced207))
    - Status-iterator won't swallow legitimate modification during 'racy-git'. ([`3bbd1f7`](https://github.com/GitoxideLabs/gitoxide/commit/3bbd1f7b60b09f9862ee88293c316a359d79e3d8))
    - `write_blob_stream()` does not need `Seek` trait anymore. ([`a03bde5`](https://github.com/GitoxideLabs/gitoxide/commit/a03bde58176e68850fa2d3299f9901a9b36b892f))
    - Merge pull request #1410 from GitoxideLabs/status ([`0ab4f64`](https://github.com/GitoxideLabs/gitoxide/commit/0ab4f64407b7fa0924830f7b7bd2f5b0ba1cc16e))
    - `Submodule::status()` now konws about tree-index changes as well. ([`a987e68`](https://github.com/GitoxideLabs/gitoxide/commit/a987e682aefa352ceaffa05d56545c9bc9c14934))
    - Add `status::Platform::into_iter()` for obtaining a complete status. ([`801689b`](https://github.com/GitoxideLabs/gitoxide/commit/801689b4aa860e1054dd9362a59d76077f31f248))
    - All `config::Snapshot` access now uses the new `Key` trait. ([`a6f397f`](https://github.com/GitoxideLabs/gitoxide/commit/a6f397f529953ac4177962059c9e6d9bcee2b657))
    - `Repository::is_dirty()` now also checks for tree/index changes. ([`8ae9e57`](https://github.com/GitoxideLabs/gitoxide/commit/8ae9e5729bd9e7d6308bd226f510b3415381de89))
    - `Repository::tree_index_status()` to see the changes between a tree and an index. ([`83f3d93`](https://github.com/GitoxideLabs/gitoxide/commit/83f3d93eaa1d7a96e0fa60840502f211c20edc3b))
    - Add `Tree::depthfirst()` with a delegate. ([`592e250`](https://github.com/GitoxideLabs/gitoxide/commit/592e250f8f01788d37f9fb7b1938b67446042bf3))
    - Adapt to changes in `gix-traverse` ([`1de4e70`](https://github.com/GitoxideLabs/gitoxide/commit/1de4e70569cd7c3bfcc9094b7591699b5b419608))
    - Merge pull request #1453 from cruessler/gix-blame ([`6ed9976`](https://github.com/GitoxideLabs/gitoxide/commit/6ed9976abaa3915b50efa46c46b195f3a1fc4ff7))
    - Add `blame` plumbing crate to the top-level. ([`25efbfb`](https://github.com/GitoxideLabs/gitoxide/commit/25efbfb72e5a043ce8f7d196c1f7104ef93394df))
    - Release gix v0.69.1 ([`7659a65`](https://github.com/GitoxideLabs/gitoxide/commit/7659a651205c08ea4ec0cbf0b441a3bd17ec49dd))
    - Merge pull request #1740 from GitoxideLabs/cargo-improvements ([`3fb0c18`](https://github.com/GitoxideLabs/gitoxide/commit/3fb0c188cb42c624ebbf5add4140bf8518e05bb2))
    - Remove unused fetch-error variants ([`51a4301`](https://github.com/GitoxideLabs/gitoxide/commit/51a430114493de392ce0c60f462d6e3ff36475a4))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.69.1 (2024-12-22)

### Bug Fixes

 - <csr-id-51a430114493de392ce0c60f462d6e3ff36475a4/> remove unused fetch-error variants
   Note that it's a breaking change, but it's on top of a previous breaking change
   so folks would already have to update explicitly.

## 0.69.0 (2024-12-22)

### Changed

 - <csr-id-c0f4da5ef4791370c98f34f0eb3bb5773edbba32/> Adjust gix::dirwalk::Options::{X,set_X} parameter names
   This adjusts the names of parameters to `X` and `set_X` methods of
   `gix::dirwalk::Options` (where `X` is an option name) to use a
   systematic naming convention:
   
   - For the same option `X`, the `X` and `set_X` methods now always
   have the same name of the parameter that specifies a value for an
   option.

### New Features

 - <csr-id-d0df20a5458e3f660bacaf86995518bc5bb8c10b/> handle RefLogLookup::Date
 - <csr-id-e17b3a9c93bd9fc5847c37b1f8e336bc4b1b1e39/> add `merge::tree::TreeFavor` similar to `*::FileFavor`.
   That way it's possible to control how tree-conflicts should be auto-resolved.

### Bug Fixes

 - <csr-id-14c3744a0bb5bf9e78055e0b86103a37a1f0c299/> assure date-tests won't fail over time.
   Need to use absolute timestamps as it's impossible to control the system time.
 - <csr-id-efc71fd0a36c7f4befe54d46a9d804853ee5a583/> public access to the contained repository in wrapped types, like `Id`.
   This makes these types easier to use as it's enough to pass a wrapped type
   to perform more actions on the underlying repository.

### New Features (BREAKING)

 - <csr-id-e59fc09f12a1c6b27d878525ff6074ac846aa87e/> move all possible code from `gix` to `gix-protocol`.
   For now, just move the code down and immediately re-integrate in `gix`
   to be able to use its tests to validate it.
   
   This is a breaking change as some types move and change the layout.
 - <csr-id-6367c7d0a796aff8ee8778916c1a1ddae68b654d/> Add `gix-shallow` crate and use it from `gix` and `gix-protocol`
   That way it's easier to reuse shallow-handling code from plumbing crates.
   
   Note that this is a breaking change as `gix-protocol` now uses the `gix-shallow::Update`
   type, which doesn't implement a formerly public `from_line()` method anymore.
   Now it is available as `fetch::response::shallow_update_from_line()`.

### Bug Fixes (BREAKING)

<csr-id-bd905a6f151ac7f3153f5208ef7c2d686f372c30/>

 - <csr-id-ea8b95f4df8a6b06aa393acd907f6400785661ff/> symlinks_to_directories_are_ignored_like_directories by value
   The methods of `gix::dirwalk::Options` are paired, where for each
   option `X` of `Options`, a method named like `X` takes and returns
   `self` by value, and a method `set_X` takes and returns `self` by
   mutable reference.
   
   But in `symlinks_to_directories_are_ignored_like_directories`, both
   took `self` by mutable reference. This fixes that. The effect of
   this fix is to allow building `Options` with a call to that method
   as the last factory method call (and using it where `Options` is
   accepted but `&mut Options` is not).
   
   Most code that consumes the crate should be unaffected, but:
   
   - Code where calls were ordered unnaturally to avoid putting such
   a call last should be able to be improved.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 30 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0 ([`beb0ea8`](https://github.com/GitoxideLabs/gitoxide/commit/beb0ea8c4ff94c64b7773772a9d388ccb403f3c1))
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1733 from GitoxideLabs/fix-testools ([`df5cead`](https://github.com/GitoxideLabs/gitoxide/commit/df5cead220c193a9ceb8b78c8d6225368293416d))
    - Assure date-tests won't fail over time. ([`14c3744`](https://github.com/GitoxideLabs/gitoxide/commit/14c3744a0bb5bf9e78055e0b86103a37a1f0c299))
    - Merge pull request #1645 from dvtkrlbs/refloglookup-date ([`cbdbb8a`](https://github.com/GitoxideLabs/gitoxide/commit/cbdbb8aaaa2f944b7ebd0c6af32b6a0a86c80277))
    - Refactor reflog support ([`9662bc1`](https://github.com/GitoxideLabs/gitoxide/commit/9662bc17e8342170a9cf0d3f3b4480ed8408ced2))
    - Handle RefLogLookup::Date ([`d0df20a`](https://github.com/GitoxideLabs/gitoxide/commit/d0df20a5458e3f660bacaf86995518bc5bb8c10b))
    - Merge pull request #1731 from GitoxideLabs/fix-pack-receive ([`ca54b8c`](https://github.com/GitoxideLabs/gitoxide/commit/ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc))
    - Adapt to changes in `gix-protocol` ([`41b6571`](https://github.com/GitoxideLabs/gitoxide/commit/41b6571b9f331c018672fcd0bb7d5ce0f8885178))
    - Merge pull request #1726 from GitoxideLabs/radicle-tuning ([`a542775`](https://github.com/GitoxideLabs/gitoxide/commit/a54277561a62cd560a9a072c6052eaf182ad4ace))
    - Adapt to changes in `gix-protocol` ([`25b8480`](https://github.com/GitoxideLabs/gitoxide/commit/25b848080c7df2da0fa662c580451aec0deb29c4))
    - Merge pull request #1634 from GitoxideLabs/remove-delegates ([`ddeb97f`](https://github.com/GitoxideLabs/gitoxide/commit/ddeb97f550bb95835648841b476d7647dd7c1dc0))
    - Adapt to changes in `gix` and `gix-protocol` ([`fcb21a4`](https://github.com/GitoxideLabs/gitoxide/commit/fcb21a4b4ad25eb3bb1a2116fa6e709e62e77c84))
    - Move all possible code from `gix` to `gix-protocol`. ([`e59fc09`](https://github.com/GitoxideLabs/gitoxide/commit/e59fc09f12a1c6b27d878525ff6074ac846aa87e))
    - Add `gix-shallow` crate and use it from `gix` and `gix-protocol` ([`6367c7d`](https://github.com/GitoxideLabs/gitoxide/commit/6367c7d0a796aff8ee8778916c1a1ddae68b654d))
    - Merge pull request #1721 from EliahKagan/run-ci/dirwalk-options ([`cd9060a`](https://github.com/GitoxideLabs/gitoxide/commit/cd9060aa3cb5b5e02673b55c2b33bef5674b148c))
    - Adjust gix::dirwalk::Options::{X,set_X} parameter names ([`c0f4da5`](https://github.com/GitoxideLabs/gitoxide/commit/c0f4da5ef4791370c98f34f0eb3bb5773edbba32))
    - Symlinks_to_directories_are_ignored_like_directories by value ([`ea8b95f`](https://github.com/GitoxideLabs/gitoxide/commit/ea8b95f4df8a6b06aa393acd907f6400785661ff))
    - Merge pull request #1719 from EliahKagan/run-ci/complex-graph-no-baseline ([`f8ba4b9`](https://github.com/GitoxideLabs/gitoxide/commit/f8ba4b9668f233c8d9cb0854bc829755de70f3d4))
    - Refine complex_graph `regex_matches` partial suppressions ([`f4b4bf0`](https://github.com/GitoxideLabs/gitoxide/commit/f4b4bf091489446f3bb9c14cdd366fcac3fcc011))
    - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
    - Adapt to changes in `gix-diff` ([`960773e`](https://github.com/GitoxideLabs/gitoxide/commit/960773e5526d02e1f2294224859c821ed86a3463))
    - Adapt to changes in `gix-merge` ([`aaeb427`](https://github.com/GitoxideLabs/gitoxide/commit/aaeb4273de936e293030a895e9bb147ce614c58a))
    - Assure that rename tracking can be turned off. ([`bd905a6`](https://github.com/GitoxideLabs/gitoxide/commit/bd905a6f151ac7f3153f5208ef7c2d686f372c30))
    - Public access to the contained repository in wrapped types, like `Id`. ([`efc71fd`](https://github.com/GitoxideLabs/gitoxide/commit/efc71fd0a36c7f4befe54d46a9d804853ee5a583))
    - Improve merge related API in `gix` ([`b2b8181`](https://github.com/GitoxideLabs/gitoxide/commit/b2b8181748ae1a3727fbbd01bfb85758f7ba3805))
    - Add `merge::tree::TreeFavor` similar to `*::FileFavor`. ([`e17b3a9`](https://github.com/GitoxideLabs/gitoxide/commit/e17b3a9c93bd9fc5847c37b1f8e336bc4b1b1e39))
    - Adapt to changes in `gix-merge` ([`3228de6`](https://github.com/GitoxideLabs/gitoxide/commit/3228de627fd059db8abbad7f465023fa559b9b0e))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.68.0 (2024-11-24)

### New Features

 - <csr-id-71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0/> Add support for `index` application in merge results via `merge::tree::Outcome::index_changed_after_applying_conflicts()`
 - <csr-id-65ae68eac6b77d12ca804927090da5bb80551eae/> add `Repository::merge_base_octopus()`
 - <csr-id-7aee32abef13f5bf331249fed38e1e6b3e12876a/> add `Repository::virtual_merge_base()` and `Repository::virtual_merge_base_with_graph()`.
 - <csr-id-5f3f63a05d1e594d92dbff75ce7293f9f163b397/> add `Repository::merge_commits()`
   It's often more convenient to work with commits when merging, especially
   when merge-bases are dealt with automatically.
 - <csr-id-27b663e4116c5aaa7b898283379d4d7d49a30865/> add `objects::tree::Editor::detach()` to get the underlying editor back.
   This can be useful to have more control over what gets written, or how.
 - <csr-id-1f9556a7e4b89f00c4de091b50909a8d2081f0d4/> add `Repository::index_or_load_from_head_or_empty()`.
   It's useful to get a reasonable index in any case, even on unborn repositories.
   It's for cases where the `HEAD` isn't setup at all, despite content being available,
   and to avoid unnecessary restrictions on what works.
 - <csr-id-2fce14ff4724ddc2f1a13c8686a7f221f8d2024e/> add `Object::peel_to_commit()` to assure an object turns into a commit.
 - <csr-id-d1ac584d9098988e66091217db4264d69fb77282/> add `Repository::merge_trees()`
 - <csr-id-07746f3ccf3e5c71f12319cfd2fb069b5f380624/> `gix::Repository` implements all traits for object reading and writing.
   That way it becomes usable when merging trees, which benefits from automatic
   checking of hashes before writing loose objects.
 - <csr-id-4b1764ca9148e08ae9f11bca68e0689b12bc8c80/> add `tree()` and `commit()` merge support, en par with `merge-ORT` as far as tests go.
   Note that this judgement of quality is based on a limited amount of partially complex
   test, but it's likely that in practice there will be deviations of sorts.
   
   Also, given the complexity of the implementation it is definitely under-tested,
   but with that it's mostly en par with Git, unfortunatly.
   
   On the bright side, some of the tests are very taxing and I'd hope this
   means something for real-world quality.

### Bug Fixes

 - <csr-id-b57be7189df216110adb52aa51c50a0f38692f2a/> `Repository::tree_merge_options()` now comes with rewrite tracking.
   This is the way Git acts, as it's either configured, or defaults to the value
   coming from the `diff.renames` configuration.
 - <csr-id-88d9d4387287b7540a0f42b26c6a4adb4cd769a9/> respect `core.bare=true` in conjunction with the main worktree
 - <csr-id-dc3d8bf79e90733172a2c3796995cdfbed438355/> propagate errors that are triggered when writing objects
   Previously it was assumed that writing objects could never fail unless
   there isn't enough memory to do so. However, it turns out that
   some last-minute validation can always be triggered and prevent an object
   to be written.
   
   Now that error is propagated instead.
 - <csr-id-4079519e7e292ee193248e3acea6587788c6b884/> assure submodules are skipped everywhere
   This also adds `Repository::head_tree()` for convenience.

### New Features (BREAKING)

 - <csr-id-a43e56314189c91f67b69de4001c6c6fad8184b6/> `Repository::merge_trees()` now takes portable version of `Options`.
 - <csr-id-1d2262f2ca416e3c22f9601e7eab11f3372b2128/> `Repository::merge_trees()` now has a fully-wrapped outcome.
   That way, more attached types are used for greater convenience.

### Bug Fixes (BREAKING)

 - <csr-id-254793581a135553e555f0bcc815154bb0951324/> rename `blob-merge` feature to `tree-merge`.
   By now, `blob-merge` is the lowest-level of features which is required
   for both tree-merges and commit based merges. Hence it's better
   to just call it `merge`.
 - <csr-id-de1cfb6a9caf5ac086c6411824835c75e888e2d7/> Adjust blob-merge baseline to also test the reverse of each operation
   This also fixes an issue with blob merge computations.
   
   It's breaking because the marker-size was reduced to `u8`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 47 commits contributed to the release.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1678](https://github.com/GitoxideLabs/gitoxide/issues/1678), [#1683](https://github.com/GitoxideLabs/gitoxide/issues/1683)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1678](https://github.com/GitoxideLabs/gitoxide/issues/1678)**
    - Propagate errors that are triggered when writing objects ([`dc3d8bf`](https://github.com/GitoxideLabs/gitoxide/commit/dc3d8bf79e90733172a2c3796995cdfbed438355))
    - Reproduce a commit-write panic if the author is invalid ([`d15a493`](https://github.com/GitoxideLabs/gitoxide/commit/d15a49303f65346dd8841249f7bf8716e61f842c))
 * **[#1683](https://github.com/GitoxideLabs/gitoxide/issues/1683)**
    - Respect `core.bare=true` in conjunction with the main worktree ([`88d9d43`](https://github.com/GitoxideLabs/gitoxide/commit/88d9d4387287b7540a0f42b26c6a4adb4cd769a9))
 * **Uncategorized**
    - Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0 ([`4000197`](https://github.com/GitoxideLabs/gitoxide/commit/4000197ecc8cf1a5d79361620e4c114f86476703))
    - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
    - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
    - Merge pull request #1661 from GitoxideLabs/merge ([`0b7abfb`](https://github.com/GitoxideLabs/gitoxide/commit/0b7abfbdebe8c5ab30b89499a70dd7727de41184))
    - Add support for `index` application in merge results via `merge::tree::Outcome::index_changed_after_applying_conflicts()` ([`71b0cea`](https://github.com/GitoxideLabs/gitoxide/commit/71b0ceaf02e022e83e6c24cfd0bdc26299dc95a0))
    - `Repository::tree_merge_options()` now comes with rewrite tracking. ([`b57be71`](https://github.com/GitoxideLabs/gitoxide/commit/b57be7189df216110adb52aa51c50a0f38692f2a))
    - Adapt to changes in `gix-merge`. ([`47110d6`](https://github.com/GitoxideLabs/gitoxide/commit/47110d600c5b02f82b4aad69ec838865e1887c0a))
    - Merge pull request #1687 from EliahKagan/run-ci/32bit ([`aeaebec`](https://github.com/GitoxideLabs/gitoxide/commit/aeaebec7b1e07ce94429987c4f2466799c24cb67))
    - Add 32-bit expectations for remaining `==` size assertions ([`daf9990`](https://github.com/GitoxideLabs/gitoxide/commit/daf999043779e7e8d9cc6a602a3a0f24024d38fa))
    - Use `<=` on 32-bit for some size assertions ([`fc13fc3`](https://github.com/GitoxideLabs/gitoxide/commit/fc13fc3eb950ebaff0c22c4d093a4d2300914f72))
    - Merge pull request #1684 from GitoxideLabs/fixes ([`9ab86a2`](https://github.com/GitoxideLabs/gitoxide/commit/9ab86a23d45941c4f0a3239e0cb57d4161dd279c))
    - Merge pull request #1679 from GitoxideLabs/fix-1678 ([`275a0c5`](https://github.com/GitoxideLabs/gitoxide/commit/275a0c55ac074e5a1004c188b87f8fc8aa9adc5b))
    - Turn single-mod directories into a file ([`49ba115`](https://github.com/GitoxideLabs/gitoxide/commit/49ba115fbebb609d7ab8728ab283cdaeab160e1b))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1659 from GitoxideLabs/merge ([`cf0c7ee`](https://github.com/GitoxideLabs/gitoxide/commit/cf0c7ee4b3bbe83a6d894d960412b0274f9dc0e5))
    - Add `Repository::merge_base_octopus()` ([`65ae68e`](https://github.com/GitoxideLabs/gitoxide/commit/65ae68eac6b77d12ca804927090da5bb80551eae))
    - Merge pull request #1658 from GitoxideLabs/merge ([`905e5b4`](https://github.com/GitoxideLabs/gitoxide/commit/905e5b42a6163f92edef8fab82d97aeb6f17cf06))
    - Add `Repository::virtual_merge_base()` and `Repository::virtual_merge_base_with_graph()`. ([`7aee32a`](https://github.com/GitoxideLabs/gitoxide/commit/7aee32abef13f5bf331249fed38e1e6b3e12876a))
    - Add `Repository::merge_commits()` ([`5f3f63a`](https://github.com/GitoxideLabs/gitoxide/commit/5f3f63a05d1e594d92dbff75ce7293f9f163b397))
    - Merge pull request #1656 from GitoxideLabs/hasconfig ([`c5955fc`](https://github.com/GitoxideLabs/gitoxide/commit/c5955fc4ad1064c7e4b4c57de32a661e693fbe49))
    - Add test to assure `hasconfig` is working on `gix` level as well. ([`d51aec9`](https://github.com/GitoxideLabs/gitoxide/commit/d51aec95588fee219dee62438d26e4574d38a497))
    - Merge pull request #1653 from GitoxideLabs/merge ([`697a632`](https://github.com/GitoxideLabs/gitoxide/commit/697a6320c7664845590e3e8251015085b6cc5d81))
    - Assure submodules are skipped everywhere ([`4079519`](https://github.com/GitoxideLabs/gitoxide/commit/4079519e7e292ee193248e3acea6587788c6b884))
    - Merge pull request #1651 from GitoxideLabs/merge ([`a876533`](https://github.com/GitoxideLabs/gitoxide/commit/a8765330fc16997dee275866b18a128dec1c5d55))
    - `Repository::merge_trees()` now takes portable version of `Options`. ([`a43e563`](https://github.com/GitoxideLabs/gitoxide/commit/a43e56314189c91f67b69de4001c6c6fad8184b6))
    - Rename `blob-merge` feature to `tree-merge`. ([`2547935`](https://github.com/GitoxideLabs/gitoxide/commit/254793581a135553e555f0bcc815154bb0951324))
    - `Repository::merge_trees()` now has a fully-wrapped outcome. ([`1d2262f`](https://github.com/GitoxideLabs/gitoxide/commit/1d2262f2ca416e3c22f9601e7eab11f3372b2128))
    - Add `objects::tree::Editor::detach()` to get the underlying editor back. ([`27b663e`](https://github.com/GitoxideLabs/gitoxide/commit/27b663e4116c5aaa7b898283379d4d7d49a30865))
    - Add `Repository::index_or_load_from_head_or_empty()`. ([`1f9556a`](https://github.com/GitoxideLabs/gitoxide/commit/1f9556a7e4b89f00c4de091b50909a8d2081f0d4))
    - Add `Object::peel_to_commit()` to assure an object turns into a commit. ([`2fce14f`](https://github.com/GitoxideLabs/gitoxide/commit/2fce14ff4724ddc2f1a13c8686a7f221f8d2024e))
    - Remove a TODO that turned out to be unnecessary. ([`5b428a9`](https://github.com/GitoxideLabs/gitoxide/commit/5b428a9e931b19622ae76c25bfb1fe882744cd1f))
    - Merge pull request #1652 from EliahKagan/run-ci/chmod ([`8e99eba`](https://github.com/GitoxideLabs/gitoxide/commit/8e99eba2a284b35b5e9bcb97e47bfbbafc3df5d1))
    - Fix `chmod` in make_rev_spec_parse_repos; regenerate archive on macOS ([`d74e919`](https://github.com/GitoxideLabs/gitoxide/commit/d74e919193e7d40018d7a602cb525f8fe8d33e84))
    - Regenerate gix make_rev_spec_parse_repos fixture archive ([`72cd7f3`](https://github.com/GitoxideLabs/gitoxide/commit/72cd7f337355c04782f3b804280005234416b8a1))
    - Avoid unneeded +x in make_rev_spec_parse_repos ([`8720acb`](https://github.com/GitoxideLabs/gitoxide/commit/8720acbe8c4a9f4d9de0cf0d4f1d41c3b738e3ad))
    - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
    - Add `Repository::merge_trees()` ([`d1ac584`](https://github.com/GitoxideLabs/gitoxide/commit/d1ac584d9098988e66091217db4264d69fb77282))
    - `gix::Repository` implements all traits for object reading and writing. ([`07746f3`](https://github.com/GitoxideLabs/gitoxide/commit/07746f3ccf3e5c71f12319cfd2fb069b5f380624))
    - Support for merge related options in config tree ([`80e006b`](https://github.com/GitoxideLabs/gitoxide/commit/80e006b759d130f4f07a346b75cfc0b39986210c))
    - Add `tree()` and `commit()` merge support, en par with `merge-ORT` as far as tests go. ([`4b1764c`](https://github.com/GitoxideLabs/gitoxide/commit/4b1764ca9148e08ae9f11bca68e0689b12bc8c80))
    - Adapt to changes in `gix-object` and `gix-odb` ([`96488f7`](https://github.com/GitoxideLabs/gitoxide/commit/96488f745506698e4183e8e544c4e0ecd1ef868b))
    - Adjust blob-merge baseline to also test the reverse of each operation ([`de1cfb6`](https://github.com/GitoxideLabs/gitoxide/commit/de1cfb6a9caf5ac086c6411824835c75e888e2d7))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.67.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>
<csr-id-743695fc345b59e30e75fb6b91357ab7e994bda2/>
<csr-id-45b71554f6437fbfe3ead020ff182f77cd57e47f/>

### New Features

<csr-id-b279957beaf581c16293343dbdb2121bd1d4dd1c/>
<csr-id-7249291016253647c920852fb37eb9e29d615775/>
<csr-id-3abf0432dad4d47d0fd70ae8a9fadea0ef82dba3/>

 - <csr-id-2b81e6c8bd30cc95e91cc92a89f0a0e6047eec6b/> add `Repository::diff_tree_to_tree()` for greater similarity to `git2`
 - <csr-id-3da2da9d7993adc16b19fc63e7524c768a6e2e7f/> add `gix merge-file` with similar features as `git merge-file`
 - <csr-id-c02adc736c9c150d2eb71307a13adfa5f35e9d47/> add `Repository::blob_merge_options()` to obtain options for merging blobs and `Repository::diff_algorithm()`
 - <csr-id-19374800ebfa19b0ddc5c2f30d1c42324732a34e/> `Repository::merge_resource_cache()` to obtain the foundation for merging files directly.
 - <csr-id-0cac69077e738cb22914e77a9a9dd3fd712d5670/> make implicit free-list more controllable
   This is done by three new `Repository` methods:
   
   * `empty_reusable_buffer()` - hook into the free-list yourself.
* `set_freelist()` - enable or initialize the free-list.
* `without_freelist()` - a builder to disable the freelist from the start.

### Bug Fixes

 - <csr-id-e9b3db8021ad1f8bf7b2ee6ffecd5b1b1c8a38b9/> make `GIT_WORK_TREE` variable work as expected.
   Now it's picked up durign initialization.
 - <csr-id-53fa8abda6cf96e2afd8082db0d7a9f686d82752/> improve directory matching
   Previously the sorting wasn't accounted for, so an assumption about
   the order of changes weren't actually true.
 - <csr-id-f8952e4cbfaf9ab7ddc12a028a1cdb821ac9a3b1/> don't be too generous when extrapolating worktree directories.
   Previously it was possible that a non-bare repository that didn't have
   worktree directory incorrectly claimed it had one.
 - <csr-id-877f4d2091a24d691f2c88a5841a6e4eb357aca3/> don't unconditionally stuff fetch-specs if these are already present.
   Previously, we'd always add 'default' refspecs, even though ref-specs
   might already have been present.
   Now we only do this if there were no refspecs prior, and that might
   still be more than Git does. I wonder where this requirement came from,
   except that it might help with tests.
 - <csr-id-977b81bba9d6cf78104ced90531079346b39843f/> Don't bail out if there are extra-refspecs.
 - <csr-id-c515edd97370f2dff2f63a44fccbd72a0f70c00f/> make `rev_walk` available even without the `revision` feature.
   Technically it doesn't depend on it.

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

### New Features (BREAKING)

 - <csr-id-7be142d087a736339af54f2cb894edc7c36cdc90/> optional rename tracking for directories.
   Depending on the source of the rename-information, items that are children
   of renamed parents may be provided to enable rename tracking based on these
   containers, instead of always resorting to tracking leaf nodes (i.e. blobs).
 - <csr-id-14d6bb92b42cb882b0fd6d0ae3c38fa634eacdaa/> Support for 'fast-tracking' reaching the beginning of the commit-graph during traversals.
   It's implemented by sorting commits oldest first when choosing the next one to traverse,
   which can greatly reduce the time it takes to reach the first commit of a graph.
 - <csr-id-dfbc732effc0fd1c90aba9c736a670d244c042e0/> optionally store objects new objects in memory only.
   The default object database changed to a version that allows to
   keep objects in memory. This needs a mutable `Repository` instance
   to setup.
 - <csr-id-d63ec06500386450f6f7b9e3b9d23ad759371832/> Do not let `revision::walk::Platform` rely on plumbing crate types.
   This is a step towards a more stable API, but also, will allow using
   different implementations.
   
   Notably, this replaces `gix_traverse::commit::simple::Sorting` with
   `gix::revision::walk::Sorting`.

### Bug Fixes (BREAKING)

 - <csr-id-206f5d70fa74c23c56c6cbecc5625234fde930fc/> improve error messages when failing to find references.
   It's breaking due to changes in the error type.
 - <csr-id-9e79ba37cf5dc7c0c295218b2de67b4b2eeff443/> unify location of error type of `Repository::diff_resource_cache()`.
 - <csr-id-c545d71ebfd4ac7d960fc75aae5558b341d8ecf2/> `Tree::lookup_entry()` looses its `buf` argument.
   The buffer will now be previded from the free-list of the repository.
 - <csr-id-17573779688e755a786546d5e42ab533088cd726/> remove all workspace dependencies
   The problem is that with them, we don't notice anymore if the crate changes,
   because a dependency changes. That also means that older versions of the dependency
   may stay even though some other crates might pick up a newer version.
   
   Ultimately, this will lead to drift and subtle incompatibilities.
   
   We declare this breaking to enforce a proper re-release.

### Refactor (BREAKING)

 - <csr-id-743695fc345b59e30e75fb6b91357ab7e994bda2/> always trackt he full path when producing diffs, but allow to disable it.
 - <csr-id-45b71554f6437fbfe3ead020ff182f77cd57e47f/> Use the new `tree_with_rewrites` plumbing implementation.
   This merges `object::tree::diff::change::Event` into `object::tree::diff::Change`
   as well.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 78 commits contributed to the release over the course of 60 calendar days.
 - 60 days passed between releases.
 - 25 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1562](https://github.com/GitoxideLabs/gitoxide/issues/1562)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1562](https://github.com/GitoxideLabs/gitoxide/issues/1562)**
    - Don't bail out if there are extra-refspecs. ([`977b81b`](https://github.com/GitoxideLabs/gitoxide/commit/977b81bba9d6cf78104ced90531079346b39843f))
 * **Uncategorized**
    - Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0 ([`f1364dc`](https://github.com/GitoxideLabs/gitoxide/commit/f1364dcb8aa66e3d8730e38445b045c5b63c56e6))
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1639 from cruessler/respect-env-variables ([`48aa74b`](https://github.com/GitoxideLabs/gitoxide/commit/48aa74b911fb874986c244712b7fd5b5cc10070b))
    - Make `GIT_WORK_TREE` variable work as expected. ([`e9b3db8`](https://github.com/GitoxideLabs/gitoxide/commit/e9b3db8021ad1f8bf7b2ee6ffecd5b1b1c8a38b9))
    - Merge pull request #1637 from GitoxideLabs/improve-error-message ([`b36d7ef`](https://github.com/GitoxideLabs/gitoxide/commit/b36d7efb9743766338ac7bb7fb2399a06fae5e60))
    - Improve error messages when failing to find references. ([`206f5d7`](https://github.com/GitoxideLabs/gitoxide/commit/206f5d70fa74c23c56c6cbecc5625234fde930fc))
    - Merge pull request #1635 from GitoxideLabs/fix-ci ([`2622936`](https://github.com/GitoxideLabs/gitoxide/commit/2622936e77d938d6cb441b4e7001dd55374328cd))
    - Improve wording of comment in test ([`e51fcd0`](https://github.com/GitoxideLabs/gitoxide/commit/e51fcd0772e087b5692f632ff8785b43d299a3f6))
    - Fix CI for now by excluding the failing assertion from running. ([`c949030`](https://github.com/GitoxideLabs/gitoxide/commit/c9490300b116cf468cca82d87c65c9190e9a6696))
    - Merge pull request #1630 from GitoxideLabs/diff-fix ([`155b5e1`](https://github.com/GitoxideLabs/gitoxide/commit/155b5e1c3691852b08dc81241423597dc34fa2dc))
    - Improve directory matching ([`53fa8ab`](https://github.com/GitoxideLabs/gitoxide/commit/53fa8abda6cf96e2afd8082db0d7a9f686d82752))
    - Reproduce ordering issue when finding parents ([`ddc99b5`](https://github.com/GitoxideLabs/gitoxide/commit/ddc99b5ae53dee19b53b67168ee45274c43b2d95))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1620 from Byron/fix-discovery ([`6487269`](https://github.com/GitoxideLabs/gitoxide/commit/64872690e60efdd9267d517f4d9971eecd3b875c))
    - Silently re-add `diff::tree::ChangeDetached` ([`c18ebbe`](https://github.com/GitoxideLabs/gitoxide/commit/c18ebbeabb3e4bd775cf59bd90e6672749ce9549))
    - Don't be too generous when extrapolating worktree directories. ([`f8952e4`](https://github.com/GitoxideLabs/gitoxide/commit/f8952e4cbfaf9ab7ddc12a028a1cdb821ac9a3b1))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Refactor integration tests for a more modern look ([`3745212`](https://github.com/GitoxideLabs/gitoxide/commit/3745212abf0353f15fec41556c55ee1d30d69f0a))
    - Add `Repository::diff_tree_to_tree()` for greater similarity to `git2` ([`2b81e6c`](https://github.com/GitoxideLabs/gitoxide/commit/2b81e6c8bd30cc95e91cc92a89f0a0e6047eec6b))
    - Always trackt he full path when producing diffs, but allow to disable it. ([`743695f`](https://github.com/GitoxideLabs/gitoxide/commit/743695fc345b59e30e75fb6b91357ab7e994bda2))
    - Use the new `tree_with_rewrites` plumbing implementation. ([`45b7155`](https://github.com/GitoxideLabs/gitoxide/commit/45b71554f6437fbfe3ead020ff182f77cd57e47f))
    - Adapt to changes in `gix-diff` ([`3fd9fab`](https://github.com/GitoxideLabs/gitoxide/commit/3fd9fabe0c0e74d54b5153c4f572eff76293f334))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Adapt to changes in `gix-diff` ([`2bf1e5f`](https://github.com/GitoxideLabs/gitoxide/commit/2bf1e5f15a60ef57c7c15279124d3eb227e585e0))
    - Optional rename tracking for directories. ([`7be142d`](https://github.com/GitoxideLabs/gitoxide/commit/7be142d087a736339af54f2cb894edc7c36cdc90))
    - Adapt to chagnes in `gix-diff` ([`5c1f010`](https://github.com/GitoxideLabs/gitoxide/commit/5c1f010851e40bf5b284ec82e6cd27cd41ab70bf))
    - Merge pull request #1611 from Byron/merge ([`5ffccd2`](https://github.com/GitoxideLabs/gitoxide/commit/5ffccd2f08d70576347e3ae17a66ca5a60f1d81c))
    - Add `gix merge-file` with similar features as `git merge-file` ([`3da2da9`](https://github.com/GitoxideLabs/gitoxide/commit/3da2da9d7993adc16b19fc63e7524c768a6e2e7f))
    - Add `Repository::blob_merge_options()` to obtain options for merging blobs and `Repository::diff_algorithm()` ([`c02adc7`](https://github.com/GitoxideLabs/gitoxide/commit/c02adc736c9c150d2eb71307a13adfa5f35e9d47))
    - Unify location of error type of `Repository::diff_resource_cache()`. ([`9e79ba3`](https://github.com/GitoxideLabs/gitoxide/commit/9e79ba37cf5dc7c0c295218b2de67b4b2eeff443))
    - `Repository::merge_resource_cache()` to obtain the foundation for merging files directly. ([`1937480`](https://github.com/GitoxideLabs/gitoxide/commit/19374800ebfa19b0ddc5c2f30d1c42324732a34e))
    - Add all keys for merge-configuration ([`e0b09d2`](https://github.com/GitoxideLabs/gitoxide/commit/e0b09d2764fd02a2b69340d9b3aef9773ae899ce))
    - Merge pull request #1585 from Byron/merge ([`2261de4`](https://github.com/GitoxideLabs/gitoxide/commit/2261de470aeb77be080f9e423e1513bde85d9cc0))
    - Use new `WorktreeRoot` API provided by `gix-diff` ([`25c6806`](https://github.com/GitoxideLabs/gitoxide/commit/25c68067d87ddcb476f973af6d7e29f9533166a8))
    - Merge pull request #1610 from nrdxp/traverse/oldest-first ([`20f9b3f`](https://github.com/GitoxideLabs/gitoxide/commit/20f9b3f361b46226be102a065cbb0fbaa83ae2db))
    - Support for 'fast-tracking' reaching the beginning of the commit-graph during traversals. ([`14d6bb9`](https://github.com/GitoxideLabs/gitoxide/commit/14d6bb92b42cb882b0fd6d0ae3c38fa634eacdaa))
    - Merge pull request #1604 from Byron/protocol-shallow-v1 ([`612896d`](https://github.com/GitoxideLabs/gitoxide/commit/612896d7ec15c153d3d48012c75aaf85d10a5abe))
    - Adapt to changes in `gix-protocol` ([`0d3b480`](https://github.com/GitoxideLabs/gitoxide/commit/0d3b480e5e7d27c308fb5f76f36831dfc7af3e8f))
    - Merge pull request #1603 from Byron/freelist ([`73a7d15`](https://github.com/GitoxideLabs/gitoxide/commit/73a7d15fb9030081a64803aacfd8d2ac7babf904))
    - Make implicit free-list more controllable ([`0cac690`](https://github.com/GitoxideLabs/gitoxide/commit/0cac69077e738cb22914e77a9a9dd3fd712d5670))
    - Merge pull request #1589 from EliahKagan/maintenance ([`7c2af44`](https://github.com/GitoxideLabs/gitoxide/commit/7c2af442748f7245734ec1f987b6d839f2a795bd))
    - Add missing executable bits ([`694ebad`](https://github.com/GitoxideLabs/gitoxide/commit/694ebadb2d11d25c5b1285c61cef5df03685701a))
    - Merge pull request #1587 from jayvdb/typos ([`c2bdda4`](https://github.com/GitoxideLabs/gitoxide/commit/c2bdda4f1ad85ee3705b464d1a951f3c9ec50147))
    - Fix typos ([`b12c7c9`](https://github.com/GitoxideLabs/gitoxide/commit/b12c7c931672203380413a2faa5c21dc311e987e))
    - Merge pull request #1586 from Byron/fix-ci ([`22fbe70`](https://github.com/GitoxideLabs/gitoxide/commit/22fbe705968689acdc08e7472a1345cf690d1b19))
    - Update crate-status to inform about tree-editing capabilities ([`fe1eb97`](https://github.com/GitoxideLabs/gitoxide/commit/fe1eb9740c66ccb49d1c43a269f2970a721b1a34))
    - Merge pull request #1584 from EliahKagan/jj-realistic-ignore ([`5242aad`](https://github.com/GitoxideLabs/gitoxide/commit/5242aad86ce30fe8e99d53959a255058ea72cd54))
    - Don't test `jj_realistic_needs_to_be_more_clever` on Windows ([`3adcfc5`](https://github.com/GitoxideLabs/gitoxide/commit/3adcfc53b17a1daa1299949ae6ed3424d5357f30))
    - Merge pull request #1582 from Byron/gix-path-release ([`93e86f1`](https://github.com/GitoxideLabs/gitoxide/commit/93e86f12a8d0ab59ad5d885ce552d0dec9a6fba6))
    - Release gix-trace v0.1.10, gix-path v0.10.11 ([`012a754`](https://github.com/GitoxideLabs/gitoxide/commit/012a75455edebc857ff13c97c1e7603ea5ea6cdc))
    - Merge pull request #1566 from Byron/merge ([`d69c617`](https://github.com/GitoxideLabs/gitoxide/commit/d69c6175574f34d6df92b4488ed2c9a85df12c89))
    - Optionally store objects new objects in memory only. ([`dfbc732`](https://github.com/GitoxideLabs/gitoxide/commit/dfbc732effc0fd1c90aba9c736a670d244c042e0))
    - Add tree-editing capabilities to `Tree` and `Repository`. ([`b279957`](https://github.com/GitoxideLabs/gitoxide/commit/b279957beaf581c16293343dbdb2121bd1d4dd1c))
    - `Tree::lookup_entry()` looses its `buf` argument. ([`c545d71`](https://github.com/GitoxideLabs/gitoxide/commit/c545d71ebfd4ac7d960fc75aae5558b341d8ecf2))
    - Merge pull request #1564 from Byron/improvements ([`1cfe577`](https://github.com/GitoxideLabs/gitoxide/commit/1cfe577d461293879e91538dbc4bbfe01722e1e8))
    - When using the cache, allow using the entire graph. ([`0fe5133`](https://github.com/GitoxideLabs/gitoxide/commit/0fe5133598c6f843fb3172a4e0c4f58932405647))
    - Adapt to changes in `gix-revwalk` ([`ea403a4`](https://github.com/GitoxideLabs/gitoxide/commit/ea403a42cab97ee803b700cdd47bbfd5ce3fbcd6))
    - Merge pull request #1563 from Byron/fixes ([`750e268`](https://github.com/GitoxideLabs/gitoxide/commit/750e26876fd4cb94fa84b48c0ad21f862a64cb65))
    - Don't unconditionally stuff fetch-specs if these are already present. ([`877f4d2`](https://github.com/GitoxideLabs/gitoxide/commit/877f4d2091a24d691f2c88a5841a6e4eb357aca3))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - `gix merge-base` for the CLI ([`7249291`](https://github.com/GitoxideLabs/gitoxide/commit/7249291016253647c920852fb37eb9e29d615775))
    - Add `Repository::merge_base()` ([`3abf043`](https://github.com/GitoxideLabs/gitoxide/commit/3abf0432dad4d47d0fd70ae8a9fadea0ef82dba3))
    - Adapt to changes in `gix-revision` ([`ce5a320`](https://github.com/GitoxideLabs/gitoxide/commit/ce5a3204c9ed6a48364bf17d7898178555724e81))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - A few more missing semicolons ([`fc45c93`](https://github.com/GitoxideLabs/gitoxide/commit/fc45c931c132ac8b6ea4f2e4c3d5f0d19727f46f))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1547 from nyurik/cast-lossless ([`c3a7dcf`](https://github.com/GitoxideLabs/gitoxide/commit/c3a7dcf859a8022468ea8289db837374d07d734f))
    - Fix clippy::cast_lossless ([`29ad2df`](https://github.com/GitoxideLabs/gitoxide/commit/29ad2df419c6d03f9f0160ca17cc94acdb30bcb7))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Merge branch 'improvements' ([`e82f795`](https://github.com/GitoxideLabs/gitoxide/commit/e82f795a56c645088b59d2b9faa5984ea067ab5c))
    - Do not let `revision::walk::Platform` rely on plumbing crate types. ([`d63ec06`](https://github.com/GitoxideLabs/gitoxide/commit/d63ec06500386450f6f7b9e3b9d23ad759371832))
    - Make `rev_walk` available even without the `revision` feature. ([`c515edd`](https://github.com/GitoxideLabs/gitoxide/commit/c515edd97370f2dff2f63a44fccbd72a0f70c00f))
    - Merge branch 'fixes' ([`46cd1ae`](https://github.com/GitoxideLabs/gitoxide/commit/46cd1aed7815d27cdc818edb87641b20b82ba048))
    - Remove all workspace dependencies ([`1757377`](https://github.com/GitoxideLabs/gitoxide/commit/17573779688e755a786546d5e42ab533088cd726))
</details>

## 0.66.0 (2024-08-22)

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

## 0.65.0 (2024-08-22)

<csr-id-26748ddbedc281b7b6b1defc51201d97e58f13e4/>

### New Features

 - <csr-id-e079250089c1d2717d62f6ab7f7b6a902b24831e/> add `objects::tree::diff::Platform::stats()` to quickly obtain diff-stats.
   This function is inspired by `git2` which also makes it very simple to obtain.
 - <csr-id-b291de0a81584430d5b5f2ae05ee220bbdadd6e8/> allow threaded-handling of tree-diff changes.
   This works by providing `Change::detach()` and `ChangeDetached::attach()`.
 - <csr-id-d986b2b271cd29d013c7874e4f021fe776115004/> add `Reference::follow_to_object()`
   It's an equivalent to `git2::Reference::resolve()`.`
 - <csr-id-6c6f9467d5d22c2f9a923a6ccda52e7cde6a6a8f/> add `Repository::diff_resource_cache_for_tree_diff()`
 - <csr-id-cdaba846526085736501a23d39ee5fbe3fe488d5/> add `Reference::peel_to_kind()`
   Make it easy to follow a ref and peel it to a given object type.
   Additional `peel_to_<kind>()` shortcuts are also provided, with
   the same name as in `git2`.
 - <csr-id-98bcb144e8fc948d95facc8a4f2a1a6ba0770106/> add `Repository::find_*()` methods for every object type.
 - <csr-id-63c7a03bb7090b5ecc7e8ec5a6b9dffdb49c3315/> add `Repository::compute_object_cache_size_for_tree_diffs()`.
   With it it's easier to obtain reasonable object cache sizes as optimized
   for tree-diffs.
 - <csr-id-8a27454a9738f077309104fde02d589d052e9f11/> `remote::Name::to_owned()` to get a static version of it.
   Also, add optional `serde` support.
 - <csr-id-6ac28673550ccd0b85b7fd2b191fa6049bbe481e/> `Reference::remote_name()` now also provides valid remote names for local tracking branches.
 - <csr-id-c612440bbfa2d5a2c033f28805eda0661009469d/> add `Reference::peel_to_id_in_place_packed()` to allow passing a packed-buffer snapshot.
   This is useful for speeding up reference lookups as otherwise, it will have to validate the packed-buffer
   snapshot didn't change internally each time a ref is peeled.
 - <csr-id-7c8f409bfa2066562930981b2d072245e5dca88e/> add `remote::Names` as shortcut to the value returned for all remote names.

### Bug Fixes

 - <csr-id-6990afd269c3461ae22e2821164c03a0dedc82f4/> similarity detection
   Previously it would incorrectly count only the last batch of removed bytes, and now it will count all of them. This leads to realistic results with complex diffs, even though it's probably still not en-par with Git which uses more complex heuristics.
 - <csr-id-e74095e0a035b090284135c88133106ba6a19fc3/> prevent panic in `Repository::rev_parse_single()` when `HEAD` was invalid.
   When using a refspec like `HEAD:file`.
 - <csr-id-6f2eb910f88ab8a61961efffd6d946d95927d8b8/> do not automatically use a parallel directory walk.
   This reduces dependencies and can speed-up typical ref-walks as these
   don't benefit from this many threads - the overhead here usually outweighs
   the benefit.
   
   This can be turned back on based on the expected workload.

### Other

 - <csr-id-26748ddbedc281b7b6b1defc51201d97e58f13e4/> make tree-diff more easily discoverable when coming from `git2`

### Bug Fixes (BREAKING)

 - <csr-id-ba72ee0f069b7df96f7543fe2d97612c98544733/> better peeling performance for reference traversal.
   This is done by keeping a packed-buffer around and reusing it, instead
   of re-checking it every time.
   
   For this to work, the `peeled()` function on the `reference::Iter` can now fail
   as it has to open a packed-refs snapshot.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release over the course of 28 calendar days.
 - 29 days passed between releases.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1508](https://github.com/GitoxideLabs/gitoxide/issues/1508), [#1524](https://github.com/GitoxideLabs/gitoxide/issues/1524)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1508](https://github.com/GitoxideLabs/gitoxide/issues/1508)**
    - Reproduce panic when parsing refspecs ([`17bd32a`](https://github.com/GitoxideLabs/gitoxide/commit/17bd32aa9311af5b10800402f4ca79a29290985a))
 * **[#1524](https://github.com/GitoxideLabs/gitoxide/issues/1524)**
    - Add a real-world test to reproduce an issue discovered in `jj` ([`7ef1e88`](https://github.com/GitoxideLabs/gitoxide/commit/7ef1e886e67d7754ecd0e6ecabcee945c28c1c3f))
 * **Uncategorized**
    - Release gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`f218578`](https://github.com/GitoxideLabs/gitoxide/commit/f21857844332f73a6c090f349aa6693684d443bb))
    - Release gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`4fe330e`](https://github.com/GitoxideLabs/gitoxide/commit/4fe330e68d10d51b0a7116a7ef8b9ea3b48a224c))
    - Release gix-attributes v0.22.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`f2b522d`](https://github.com/GitoxideLabs/gitoxide/commit/f2b522df2ddad07f065f43c2dbad49aa726714dd))
    - Release gix-glob v0.16.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-config-value v0.14.8, gix-tempfile v14.0.2, gix-ref v0.46.0, gix-sec v0.10.8, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-pathspec v0.7.7, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0 ([`a65a17f`](https://github.com/GitoxideLabs/gitoxide/commit/a65a17fc396ef49663b0a75cf7b5502d370db269))
    - Release gix-date v0.9.0, gix-actor v0.31.6, gix-validate v0.9.0, gix-object v0.43.0, gix-path v0.10.10, gix-attributes v0.22.4, gix-command v0.3.9, gix-packetline-blocking v0.17.5, gix-filter v0.12.0, gix-fs v0.11.3, gix-revwalk v0.14.0, gix-traverse v0.40.0, gix-worktree-stream v0.14.0, gix-archive v0.14.0, gix-ref v0.46.0, gix-config v0.39.0, gix-prompt v0.8.7, gix-url v0.27.5, gix-credentials v0.24.5, gix-ignore v0.11.4, gix-index v0.34.0, gix-worktree v0.35.0, gix-diff v0.45.0, gix-discover v0.34.0, gix-dir v0.7.0, gix-mailmap v0.23.6, gix-negotiate v0.14.0, gix-pack v0.52.0, gix-odb v0.62.0, gix-packetline v0.17.6, gix-transport v0.42.3, gix-protocol v0.45.3, gix-revision v0.28.0, gix-refspec v0.24.0, gix-status v0.12.0, gix-submodule v0.13.0, gix-worktree-state v0.12.0, gix v0.65.0, gix-fsck v0.5.0, gitoxide-core v0.40.0, gitoxide v0.38.0, safety bump 25 crates ([`d19af16`](https://github.com/GitoxideLabs/gitoxide/commit/d19af16e1d2031d4f0100e76b6cd410a5d252af1))
    - Prepare changelogs prior to release ([`0f25841`](https://github.com/GitoxideLabs/gitoxide/commit/0f2584178ae88e425f1c629eb85b69f3b4310d9f))
    - Merge branch 'improvements' ([`9ed2b24`](https://github.com/GitoxideLabs/gitoxide/commit/9ed2b24e5d275e09ef29f341de8184874d5c23fe))
    - Similarity detection ([`6990afd`](https://github.com/GitoxideLabs/gitoxide/commit/6990afd269c3461ae22e2821164c03a0dedc82f4))
    - Fix similarity detection ([`f8c5d9c`](https://github.com/GitoxideLabs/gitoxide/commit/f8c5d9ce87a3b6b7d0e051b1cb7de3707209c432))
    - Better peeling performance for reference traversal. ([`ba72ee0`](https://github.com/GitoxideLabs/gitoxide/commit/ba72ee0f069b7df96f7543fe2d97612c98544733))
    - Merge branch 'improvements' ([`242fedc`](https://github.com/GitoxideLabs/gitoxide/commit/242fedc973c56b6c1b6f150af99dda972a67f547))
    - Use improved gix-diff API for better buffer handling ([`f944e49`](https://github.com/GitoxideLabs/gitoxide/commit/f944e49ec97b6e9dcffab8606ca0b46e343d7e55))
    - Add `objects::tree::diff::Platform::stats()` to quickly obtain diff-stats. ([`e079250`](https://github.com/GitoxideLabs/gitoxide/commit/e079250089c1d2717d62f6ab7f7b6a902b24831e))
    - Allow threaded-handling of tree-diff changes. ([`b291de0`](https://github.com/GitoxideLabs/gitoxide/commit/b291de0a81584430d5b5f2ae05ee220bbdadd6e8))
    - Add `Reference::follow_to_object()` ([`d986b2b`](https://github.com/GitoxideLabs/gitoxide/commit/d986b2b271cd29d013c7874e4f021fe776115004))
    - Add `Repository::diff_resource_cache_for_tree_diff()` ([`6c6f946`](https://github.com/GitoxideLabs/gitoxide/commit/6c6f9467d5d22c2f9a923a6ccda52e7cde6a6a8f))
    - Add `Reference::peel_to_kind()` ([`cdaba84`](https://github.com/GitoxideLabs/gitoxide/commit/cdaba846526085736501a23d39ee5fbe3fe488d5))
    - Adapt to changes in `gix-ref` ([`d296ee8`](https://github.com/GitoxideLabs/gitoxide/commit/d296ee87e1b46e603b98a5f5ebf9e6f0695f8a52))
    - Adapt to changes in `gix-ref` ([`5464bfb`](https://github.com/GitoxideLabs/gitoxide/commit/5464bfb2df3fa8b5c582718349997170c3f2745e))
    - Add `Repository::find_*()` methods for every object type. ([`98bcb14`](https://github.com/GitoxideLabs/gitoxide/commit/98bcb144e8fc948d95facc8a4f2a1a6ba0770106))
    - Add `Repository::compute_object_cache_size_for_tree_diffs()`. ([`63c7a03`](https://github.com/GitoxideLabs/gitoxide/commit/63c7a03bb7090b5ecc7e8ec5a6b9dffdb49c3315))
    - Make tree-diff more easily discoverable when coming from `git2` ([`26748dd`](https://github.com/GitoxideLabs/gitoxide/commit/26748ddbedc281b7b6b1defc51201d97e58f13e4))
    - Merge pull request #1529 from Byron/better-copy-detection ([`7b7902e`](https://github.com/GitoxideLabs/gitoxide/commit/7b7902eb8478607d611030d59fa87b390ca7ee76))
    - Remove `#[momo]` directive as it seems to prevent auto-completion in IDEs. ([`3a339da`](https://github.com/GitoxideLabs/gitoxide/commit/3a339da03ce6498b1c8a0f367a19d7eda773668f))
    - Merge pull request #1521 from mvlabat/fix-dir-filename-tracking ([`12251eb`](https://github.com/GitoxideLabs/gitoxide/commit/12251eb052df30105538fa831e641eea557f13d8))
    - Fix dir name tracking in the FileName mode ([`63936e5`](https://github.com/GitoxideLabs/gitoxide/commit/63936e5407d137d32d86a9557c3dfe378755cea0))
    - Merge branch 'fix-panic' ([`0b28297`](https://github.com/GitoxideLabs/gitoxide/commit/0b2829712be53846c830dbb175da2beaf00e7c76))
    - Prevent panic in `Repository::rev_parse_single()` when `HEAD` was invalid. ([`e74095e`](https://github.com/GitoxideLabs/gitoxide/commit/e74095e0a035b090284135c88133106ba6a19fc3))
    - Merge branch 'improvements' ([`7dff447`](https://github.com/GitoxideLabs/gitoxide/commit/7dff44754e0fdc369f92221468fb953bad9be60a))
    - `remote::Name::to_owned()` to get a static version of it. ([`8a27454`](https://github.com/GitoxideLabs/gitoxide/commit/8a27454a9738f077309104fde02d589d052e9f11))
    - Merge branch 'improvements' ([`29898e3`](https://github.com/GitoxideLabs/gitoxide/commit/29898e3010bd3332418c683f2ac96aff5c8e72fa))
    - `Reference::remote_name()` now also provides valid remote names for local tracking branches. ([`6ac2867`](https://github.com/GitoxideLabs/gitoxide/commit/6ac28673550ccd0b85b7fd2b191fa6049bbe481e))
    - Add `Reference::peel_to_id_in_place_packed()` to allow passing a packed-buffer snapshot. ([`c612440`](https://github.com/GitoxideLabs/gitoxide/commit/c612440bbfa2d5a2c033f28805eda0661009469d))
    - Do not automatically use a parallel directory walk. ([`6f2eb91`](https://github.com/GitoxideLabs/gitoxide/commit/6f2eb910f88ab8a61961efffd6d946d95927d8b8))
    - Add `remote::Names` as shortcut to the value returned for all remote names. ([`7c8f409`](https://github.com/GitoxideLabs/gitoxide/commit/7c8f409bfa2066562930981b2d072245e5dca88e))
    - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
    - Assure the next release is breaking ([`9fd1090`](https://github.com/GitoxideLabs/gitoxide/commit/9fd10905449a41cdda5eb2764e4d45d314de9c04))
    - Release gix-credentials v0.24.4 ([`f6a4eb9`](https://github.com/GitoxideLabs/gitoxide/commit/f6a4eb98740fe4151e293d53a578233119307a58))
    - Merge branch 'fix-clean' ([`33eacfb`](https://github.com/GitoxideLabs/gitoxide/commit/33eacfbaace2021043e2b5d72dcb9293af6c4020))
    - Adapt to changes in `gix-dir` ([`37c2852`](https://github.com/GitoxideLabs/gitoxide/commit/37c2852a1fdbc13ff9e76db6f60224b4cb5f75ab))
</details>

## 0.64.0 (2024-07-23)

<csr-id-d9a813fdd2cac522999dccb2dbff84c6a50735a2/>

### New Features

 - <csr-id-41e018dfaef7c2743894134dbd39f0a226be3532/> enable tracing with the new `tracing` feature in the Cargo manifest.
   That way, it's easier to directly enable tracing, instead of having to
   resort to `gix-features`.
 - <csr-id-5c7e744c604b3baaab97d3a5a79cc3e2e4dba783/> add `config::section::filter()` for the default section filter.
   Provide the default implementation to determine which configuration
   sections should be trusted.
 - <csr-id-478bbc149951b0f81c4a76f0a8d534c170589ebf/> provide `config::credential_helpers()` function to configure an invocation.
   That way it's possible to use git credential helpers even without a Repository instance.
 - <csr-id-afc6e258ce0445c804707f7cee2daccde1df937d/> export `gix_validate` as `validate`.
   The crate appears in the public API of `gix_index::State::from_tree`
   but it's types weren't readily available.
 - <csr-id-acbfa6fb5f749e84e6c9f34c3c97b02f97db5f68/> add `PrepareFetch::with_ref_name()` to control which ref is checked out.

### Bug Fixes

 - <csr-id-40d18816a85c41eb2b9075752b092ae68f4d979c/> make sure that `refs/heads/HEAD` isn't considered conflicting
 - <csr-id-c8c56aebaac95f0f73220055dc33e6e0ebdb5ced/> re-export `gix_validate` as it's now part of the public API.
   This allows calling `State::from_tree()`.
 - <csr-id-a146d140da6c848a39d9f14b40f3fd46b749cc11/> don't attempt negotiation without any refspec-mappings
   Previously, when a shallow operation was specified, it was possible
   to skip past the no-change test and attempt to negotiate even though
   there was nothing to want.
   
   This is now 'fixed' by simply aborting early if there is no refspec
   mapping at all.
   
   Further, it aborts as early as possible with a nicer error message,
   after all, there is no use in having no mapping.
   
   Note that it's explicitly implemented such that obtaining such an empty
   refmap is fine, but trying to receive it is not. That way, the user can
   obtain information about the server without anything being an error.

### Other

 - <csr-id-d9a813fdd2cac522999dccb2dbff84c6a50735a2/> gate parking_lot behind interrupt feature

### New Features (BREAKING)

 - <csr-id-0ec2389e4e3c457f87cff2cbdd394a94f7d0d54a/> `gix-config` convenience initiative
   A new `gix_config::AsKey` trait allows all `gix_config::File` methods
   to receive an implementation of `AsKey` instead of the tuple of
   `(section, subsection, value_name)`. This is the default, and all
   methods that take a tuple have been renamed to `<name>_by()`, note the `_by` suffix.
   
   The terminology was reworked so that `key` is now only used to identify a value.
   
   This change also affects the public API of `gix`, which provides
   ways to set values on configuration snapshots which now by default
   will only take a `AsKey` implementation.
   
   Note that `gix::config::tree::*` keys now also implement `AsKey`,
   which allows them to be used more conveniently when setting values,
   too.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 53 commits contributed to the release over the course of 58 calendar days.
 - 62 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1405](https://github.com/GitoxideLabs/gitoxide/issues/1405), [#1428](https://github.com/GitoxideLabs/gitoxide/issues/1428)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1405](https://github.com/GitoxideLabs/gitoxide/issues/1405)**
    - Don't attempt negotiation without any refspec-mappings ([`a146d14`](https://github.com/GitoxideLabs/gitoxide/commit/a146d140da6c848a39d9f14b40f3fd46b749cc11))
 * **[#1428](https://github.com/GitoxideLabs/gitoxide/issues/1428)**
    - Make sure that `refs/heads/HEAD` isn't considered conflicting ([`40d1881`](https://github.com/GitoxideLabs/gitoxide/commit/40d18816a85c41eb2b9075752b092ae68f4d979c))
 * **Uncategorized**
    - Release gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`0e8508a`](https://github.com/GitoxideLabs/gitoxide/commit/0e8508a629c74068a1e667ea4222c3e69ae0b861))
    - Release gix-actor v0.31.5, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`6232824`](https://github.com/GitoxideLabs/gitoxide/commit/6232824301847a9786dea0b926796a3187493587))
    - Release gix-glob v0.16.4, gix-attributes v0.22.3, gix-command v0.3.8, gix-filter v0.11.3, gix-fs v0.11.2, gix-commitgraph v0.24.3, gix-revwalk v0.13.2, gix-traverse v0.39.2, gix-worktree-stream v0.13.1, gix-archive v0.13.2, gix-config-value v0.14.7, gix-tempfile v14.0.1, gix-ref v0.45.0, gix-sec v0.10.7, gix-config v0.38.0, gix-prompt v0.8.6, gix-url v0.27.4, gix-credentials v0.24.3, gix-ignore v0.11.3, gix-index v0.33.1, gix-worktree v0.34.1, gix-diff v0.44.1, gix-discover v0.33.0, gix-pathspec v0.7.6, gix-dir v0.6.0, gix-mailmap v0.23.5, gix-negotiate v0.13.2, gix-pack v0.51.1, gix-odb v0.61.1, gix-transport v0.42.2, gix-protocol v0.45.2, gix-revision v0.27.2, gix-refspec v0.23.1, gix-status v0.11.0, gix-submodule v0.12.0, gix-worktree-state v0.11.1, gix v0.64.0, gix-fsck v0.4.1, gitoxide-core v0.39.0, gitoxide v0.37.0 ([`a1b73a6`](https://github.com/GitoxideLabs/gitoxide/commit/a1b73a67c19d9102a2c5a7f574a7a53a86d0094c))
    - Update manifests (by cargo-smart-release) ([`0470df3`](https://github.com/GitoxideLabs/gitoxide/commit/0470df3b8ebb136b219f0057f1e9a7031975cce5))
    - Prepare changelog prior to release ([`99c00cc`](https://github.com/GitoxideLabs/gitoxide/commit/99c00cc3ae9827555e2e1162328bc57038619d1f))
    - Merge branch 'fixes' ([`b4dba1c`](https://github.com/GitoxideLabs/gitoxide/commit/b4dba1c187baba44ee927daa538783f7f424b2f2))
    - Add more tests for remote name validation ([`1267712`](https://github.com/GitoxideLabs/gitoxide/commit/126771270e7a2d08c5d702a6903e8b9b326b2f39))
    - Thanks clippy ([`113cbcc`](https://github.com/GitoxideLabs/gitoxide/commit/113cbcc3028e6c6ed6d15980e11d2bf67d033066))
    - Release gix-path v0.10.9 ([`15f1cf7`](https://github.com/GitoxideLabs/gitoxide/commit/15f1cf76764221d14afa66d03a6528b19b9c30c9))
    - Merge branch 'fix-1428' ([`caae926`](https://github.com/GitoxideLabs/gitoxide/commit/caae9260ef3d66998d6826c493631f3d7296c73f))
    - Merge branch 'fix-1440' ([`f87322e`](https://github.com/GitoxideLabs/gitoxide/commit/f87322e185704d9d4368ae88e95892635a976e4a))
    - Adapt to changes in `gix-testtools` ([`f5a9884`](https://github.com/GitoxideLabs/gitoxide/commit/f5a9884006b0ea8d22cc51a119ae87ce10cd3484))
    - Release gix-actor v0.31.4, gix-object v0.42.3 ([`bf3d82a`](https://github.com/GitoxideLabs/gitoxide/commit/bf3d82abc7c875109f9a5d6b6713ce68153b6456))
    - Merge branch 'heredocs' ([`7330844`](https://github.com/GitoxideLabs/gitoxide/commit/73308446e5ffee053af35b108e3d49c71db31e99))
    - Regenerate archives ([`a4bb7b9`](https://github.com/GitoxideLabs/gitoxide/commit/a4bb7b9b7f15992644171bb06865637e18e1141f))
    - Use `<<` rather than `<<-` heredoc operator ([`2641f8b`](https://github.com/GitoxideLabs/gitoxide/commit/2641f8b36008ade04d59d76bd6d546005ad76a21))
    - Release gix-actor v0.31.3, gix-mailmap v0.23.4 ([`1e79c5c`](https://github.com/GitoxideLabs/gitoxide/commit/1e79c5cdf20fc0440e9a497c9d01b0c0ca3ce424))
    - Allow `use_http_path` to be set in `config::credential_helpers()` ([`55cbc1b`](https://github.com/GitoxideLabs/gitoxide/commit/55cbc1b9d6f210298a86502a7f20f9736c7e963e))
    - Enable tracing with the new `tracing` feature in the Cargo manifest. ([`41e018d`](https://github.com/GitoxideLabs/gitoxide/commit/41e018dfaef7c2743894134dbd39f0a226be3532))
    - Merge branch 'config-globals' ([`929744a`](https://github.com/GitoxideLabs/gitoxide/commit/929744ab628c8a32ce8e357c1000df20175a5b41))
    - Merge pull request #1430 from klensy/deps ([`ab02aa9`](https://github.com/GitoxideLabs/gitoxide/commit/ab02aa99842c17d68b8ee37e05e2f35720291e42))
    - Gate parking_lot behind interrupt feature ([`d9a813f`](https://github.com/GitoxideLabs/gitoxide/commit/d9a813fdd2cac522999dccb2dbff84c6a50735a2))
    - Add `config::section::filter()` for the default section filter. ([`5c7e744`](https://github.com/GitoxideLabs/gitoxide/commit/5c7e744c604b3baaab97d3a5a79cc3e2e4dba783))
    - Provide `config::credential_helpers()` function to configure an invocation. ([`478bbc1`](https://github.com/GitoxideLabs/gitoxide/commit/478bbc149951b0f81c4a76f0a8d534c170589ebf))
    - Release gix-mailmap v0.23.3 ([`0c5d1ff`](https://github.com/GitoxideLabs/gitoxide/commit/0c5d1ff3f48aab43119f86501b14974f92c2017d))
    - Release gix-path v0.10.8 ([`8d89b86`](https://github.com/GitoxideLabs/gitoxide/commit/8d89b865c84d1fb153d93343d1ce4e1d64e53541))
    - Merge branch 'various-fixes' ([`f71b7a0`](https://github.com/GitoxideLabs/gitoxide/commit/f71b7a0ea1f70b2596ced9179c41e82fec7a7fae))
    - Re-export `gix_validate` as it's now part of the public API. ([`c8c56ae`](https://github.com/GitoxideLabs/gitoxide/commit/c8c56aebaac95f0f73220055dc33e6e0ebdb5ced))
    - Export `gix_validate` as `validate`. ([`afc6e25`](https://github.com/GitoxideLabs/gitoxide/commit/afc6e258ce0445c804707f7cee2daccde1df937d))
    - Release gix-date v0.8.7, gix-mailmap v0.23.2 ([`c1d7c02`](https://github.com/GitoxideLabs/gitoxide/commit/c1d7c023d595eb04891b65295f001d85c9ba8074))
    - Merge branch 'tar-only' ([`1dfa90d`](https://github.com/GitoxideLabs/gitoxide/commit/1dfa90d641306b4099a6ecd52e2056b231467807))
    - Remove binary files in favor of `tar` files ([`dcab79a`](https://github.com/GitoxideLabs/gitoxide/commit/dcab79a6958cbf5cd69184c24497dc27c6f94961))
    - Merge branch 'config-key' ([`5663a2c`](https://github.com/GitoxideLabs/gitoxide/commit/5663a2c9f3b23c189af7f0a30664639df4acd411))
    - `gix-config` convenience initiative ([`0ec2389`](https://github.com/GitoxideLabs/gitoxide/commit/0ec2389e4e3c457f87cff2cbdd394a94f7d0d54a))
    - Addditional fixes on top of the merge commit ([`dbe1f22`](https://github.com/GitoxideLabs/gitoxide/commit/dbe1f22373a8e60d5b124e10fd131d3921134aa5))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
    - Merge branch 'feat/checkout-other-refs' ([`ecfde07`](https://github.com/GitoxideLabs/gitoxide/commit/ecfde07d0887322db34f5ea531891c92676e1ff4))
    - Thanks clippy ([`f36b9bd`](https://github.com/GitoxideLabs/gitoxide/commit/f36b9bd28052131401d048b5aa55c5ae1f9248db))
    - Improve documentation of `PrepareCheckout` and make it easier to use ([`39180b4`](https://github.com/GitoxideLabs/gitoxide/commit/39180b4602745678f9204fe9e11c0facbdd23f40))
    - Add `PrepareFetch::with_ref_name()` to control which ref is checked out. ([`acbfa6f`](https://github.com/GitoxideLabs/gitoxide/commit/acbfa6fb5f749e84e6c9f34c3c97b02f97db5f68))
    - Squash 11 commits that get started with allowing to checkout a particular branch ([`0912a46`](https://github.com/GitoxideLabs/gitoxide/commit/0912a4649134c33251fee18e2c030c68a10c19bd))
    - Merge branch 'status' ([`2f9f0ac`](https://github.com/GitoxideLabs/gitoxide/commit/2f9f0ac36eb37b1736e21ee09e5a91833b80fc65))
    - Merge pull request #1407 from jsimonrichard/06-19-add_rela_path_to_Item ([`2856434`](https://github.com/GitoxideLabs/gitoxide/commit/285643458c358c667d4c2a99d69ecc1bc0a7c383))
    - Add rela_path to crate::status::index_worktree::iter::Item ([`1dc4568`](https://github.com/GitoxideLabs/gitoxide/commit/1dc456813fd17effe8a2e3c4f11c1906379390af))
    - Merge pull request #1361 from EliahKagan/freebsd ([`9c65d98`](https://github.com/GitoxideLabs/gitoxide/commit/9c65d9886328f53129b966aecdc91644297c54be))
    - Regenerate archives for changed scripts ([`ea12fc2`](https://github.com/GitoxideLabs/gitoxide/commit/ea12fc234e898eb15013da40d2a82f69c2d20482))
    - Make bash script shebangs more portable ([`68cbea8`](https://github.com/GitoxideLabs/gitoxide/commit/68cbea815aa979acb0b86943db83ab77bbc728c4))
    - Release gix-fs v0.11.1, gix-glob v0.16.3 ([`2cefe77`](https://github.com/GitoxideLabs/gitoxide/commit/2cefe77203131878d0d8f5346f20f0e25b76cbea))
    - Merge pull request #1385 from Byron/fix-gix-ref ([`8da55a3`](https://github.com/GitoxideLabs/gitoxide/commit/8da55a3488a3389ec02c56cb79d0f93d600905e7))
    - Release gix-ref v0.44.1 ([`2d0a352`](https://github.com/GitoxideLabs/gitoxide/commit/2d0a3520e1df80f8f6edece0884a672cbc18839d))
    - Release gix-archive v0.13.1 ([`bd32c7a`](https://github.com/GitoxideLabs/gitoxide/commit/bd32c7a40f53f4cff57e600bc350f8ca7ed624cc))
</details>

## 0.63.0 (2024-05-22)

### New Features

 - <csr-id-886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e/> checkout respects options for `core.protectHFS` and `core.protectNTFS`.
   This also adds `gitoxide.core.protectWindows` as a way to enforce
   additional restrictions that are usually only available on Windows.
   
   Note that `core.protectNFS` is always enabled by default, just like
   [it is in Git](https://github.com/git/git/commit/9102f958ee5254b10c0be72672aa3305bf4f4704).

### Bug Fixes

 - <csr-id-3c7b7b3a7b981040cd51417202d7022597179114/> empty paths as configured will not be an error with lenient configuration enabled.
   When using `gix::open_opts(path, options.strict_config(false))`, empty `core.excludesFile` values
   will not cause an error anymore.
   
   Note that in strict mode, the behaviour is unchanged so invalid configuration can rather be fixed
   than ignored.
 - <csr-id-88a6a4e6d882fc7a3a0b4017d772a3fe38e57598/> don't unwrap when reading possibly left-over bytes from pack-stream

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 24 commits contributed to the release.
 - 38 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1352](https://github.com/GitoxideLabs/gitoxide/issues/1352), [#1370](https://github.com/GitoxideLabs/gitoxide/issues/1370)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1352](https://github.com/GitoxideLabs/gitoxide/issues/1352)**
    - Don't unwrap when reading possibly left-over bytes from pack-stream ([`88a6a4e`](https://github.com/GitoxideLabs/gitoxide/commit/88a6a4e6d882fc7a3a0b4017d772a3fe38e57598))
 * **[#1370](https://github.com/GitoxideLabs/gitoxide/issues/1370)**
    - Empty paths as configured will not be an error with lenient configuration enabled. ([`3c7b7b3`](https://github.com/GitoxideLabs/gitoxide/commit/3c7b7b3a7b981040cd51417202d7022597179114))
 * **Uncategorized**
    - Release gix-features v0.38.2, gix-actor v0.31.2, gix-validate v0.8.5, gix-object v0.42.2, gix-command v0.3.7, gix-filter v0.11.2, gix-fs v0.11.0, gix-revwalk v0.13.1, gix-traverse v0.39.1, gix-worktree-stream v0.13.0, gix-archive v0.13.0, gix-tempfile v14.0.0, gix-lock v14.0.0, gix-ref v0.44.0, gix-config v0.37.0, gix-prompt v0.8.5, gix-index v0.33.0, gix-worktree v0.34.0, gix-diff v0.44.0, gix-discover v0.32.0, gix-pathspec v0.7.5, gix-dir v0.5.0, gix-macros v0.1.5, gix-mailmap v0.23.1, gix-negotiate v0.13.1, gix-pack v0.51.0, gix-odb v0.61.0, gix-transport v0.42.1, gix-protocol v0.45.1, gix-revision v0.27.1, gix-status v0.10.0, gix-submodule v0.11.0, gix-worktree-state v0.11.0, gix v0.63.0, gitoxide-core v0.38.0, gitoxide v0.36.0, safety bump 19 crates ([`4f98e94`](https://github.com/GitoxideLabs/gitoxide/commit/4f98e94e0e8b79ed2899b35bef40f3c30b3025b0))
    - Adjust changelogs prior to release ([`9511416`](https://github.com/GitoxideLabs/gitoxide/commit/9511416a6cd0c571233f958c165329c8705c2498))
    - Merge branch 'various-fixes' ([`d6cd449`](https://github.com/GitoxideLabs/gitoxide/commit/d6cd44930fb204b06e2b70fc6965e7705530c47a))
    - Update dependencies ([`cd4de83`](https://github.com/GitoxideLabs/gitoxide/commit/cd4de8327fc195eb862ab6e138f2315a87374f85))
    - Fix-CI ([`6f55f2a`](https://github.com/GitoxideLabs/gitoxide/commit/6f55f2abd13078f94e8c4e10922806f195ae0d8b))
    - Merge pull request from GHSA-7w47-3wg8-547c ([`79dce79`](https://github.com/GitoxideLabs/gitoxide/commit/79dce79c62f6072aa2653780d590dc3993dfa401))
    - Adapt to changes in `gix-ref` ([`d2ae9d5`](https://github.com/GitoxideLabs/gitoxide/commit/d2ae9d5f11be9f2561f6799d88804d0d8eae33ef))
    - Adapt to changes in `gix-index` ([`5f86e6b`](https://github.com/GitoxideLabs/gitoxide/commit/5f86e6b11bb73921b458ffee9091bc028a7d6204))
    - Fix compile warnings ([`f961687`](https://github.com/GitoxideLabs/gitoxide/commit/f9616871e83502e720edad621bc6a9cbcfc53de3))
    - Address review comments ([`fcc3b69`](https://github.com/GitoxideLabs/gitoxide/commit/fcc3b69867db1628f6a44d0e0dad8f7417f566bc))
    - Apply suggestions from code review ([`bad9a79`](https://github.com/GitoxideLabs/gitoxide/commit/bad9a797b99880ce9d1c20e11c801bd0e741db64))
    - Checkout respects options for `core.protectHFS` and `core.protectNTFS`. ([`886d6b5`](https://github.com/GitoxideLabs/gitoxide/commit/886d6b58e4612ac21cc660ea4ddf1dd0b49d1c6e))
    - Adapt to changes in `gix-worktree` ([`1ca6a3c`](https://github.com/GitoxideLabs/gitoxide/commit/1ca6a3ce22887c7eb42ec3e0a19f6e1202715745))
    - Merge pull request #1371 from Byron/fix-empty-excludes-file ([`3c21741`](https://github.com/GitoxideLabs/gitoxide/commit/3c2174101ed35dcb9bdb4585b3245507b15efe59))
    - Release gix-date v0.8.6 ([`d3588ca`](https://github.com/GitoxideLabs/gitoxide/commit/d3588ca4fe0364c88e42cdac24ceae548355d99d))
    - Merge branch 'status' ([`04ef31e`](https://github.com/GitoxideLabs/gitoxide/commit/04ef31e9d6f5332d49037a5a4c248ebbb5aaf92b))
    - Improve docs to be more approachable from `git2` ([`5197b5a`](https://github.com/GitoxideLabs/gitoxide/commit/5197b5abd988002ffbb40f34bbe000ce5dcaffcf))
    - Merge branch 'status' ([`e791bc5`](https://github.com/GitoxideLabs/gitoxide/commit/e791bc5da52a1237fb7cac230af583199162825d))
    - Merge branch 'cargo-fixes' ([`977346e`](https://github.com/GitoxideLabs/gitoxide/commit/977346ee61de6207c66f3de003db6e8c722fb81c))
    - Release gix-index v0.32.1, gix-pathspec v0.7.4, gix-worktree v0.33.1, gix-dir v0.4.1 ([`54ac559`](https://github.com/GitoxideLabs/gitoxide/commit/54ac55946bb04635cd74582a1ce2e4bee70f2e60))
    - Merge pull request #1345 from EliahKagan/shell-scripts ([`fe24c89`](https://github.com/GitoxideLabs/gitoxide/commit/fe24c89e326670deaa3aaa643276d612d866072e))
    - Add missing +x bit on scripts that are run and not sourced ([`41bf65a`](https://github.com/GitoxideLabs/gitoxide/commit/41bf65adef6f7d2cdd28fede262173ec7ba10822))
</details>

## 0.62.0 (2024-04-13)

Please note that this release contains a security fix originally implemented in `gix-transport` via [this PR](https://github.com/Byron/gitoxide/pull/1342)
which prevents `ssh` options to be smuggled into the `ssh` command-line invocation with a username provided to a clone or fetch URL.

Details can be found [in the advisory](https://github.com/Byron/gitoxide/security/advisories/GHSA-98p4-xjmm-8mfh).

### Bug Fixes

 - <csr-id-18b2921aaa28df536faf74098d5f1f13d34148f9/> `into_index_worktree_iter()` now takes an iterator, instead of a Vec.
   This makes the API more consistent, and one can pass `None`
   as well.
 - <csr-id-719ced8a7949ba1f30fef13801e3466a7d1da590/> show submodules in status independently of their active state.
   Even inactive submodules are shown in the status by `git status`,
   so `gix` should do the same.
   
   First observed in https://github.com/helix-editor/helix/pull/5645#issuecomment-2016798212
 - <csr-id-98cfbec51276bbd6caa48fd6d8942247df091c94/> forward `curl` rustls feature from `gix-transport` to avoid `curl` in `gix`.
   This removes the `curl` dependency just for configuring it, and removes
   a hazard which became evident with reqwest.

### Bug Fixes (BREAKING)

 - <csr-id-2a9c178326b7f13ba6bc1f89fc2b9d9facbecf48/> Make `topo` more similar to `Ancestors`, but also rename `Ancestors` to `Simple`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1328](https://github.com/GitoxideLabs/gitoxide/issues/1328)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1328](https://github.com/GitoxideLabs/gitoxide/issues/1328)**
    - Forward `curl` rustls feature from `gix-transport` to avoid `curl` in `gix`. ([`98cfbec`](https://github.com/GitoxideLabs/gitoxide/commit/98cfbec51276bbd6caa48fd6d8942247df091c94))
 * **Uncategorized**
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
    - Merge pull request #1341 from szepeviktor/typos ([`55f379b`](https://github.com/GitoxideLabs/gitoxide/commit/55f379bc47065822d078393d83d30c0835a89782))
    - Fix typos ([`f72ecce`](https://github.com/GitoxideLabs/gitoxide/commit/f72ecce45babcad2a0c9b73c79d01ff502907a57))
    - Merge branch 'add-topo-walk' ([`b590a9d`](https://github.com/GitoxideLabs/gitoxide/commit/b590a9d2b6a273f76f0320d2b9fe1f679c08f549))
    - Adapt to changes in `gix-traverse` ([`1cfeb11`](https://github.com/GitoxideLabs/gitoxide/commit/1cfeb11f1fe9ad9c7b9084840ed7f5c5877f2f9a))
    - Make `topo` more similar to `Ancestors`, but also rename `Ancestors` to `Simple` ([`2a9c178`](https://github.com/GitoxideLabs/gitoxide/commit/2a9c178326b7f13ba6bc1f89fc2b9d9facbecf48))
    - Adapt to changes in `gix-traverse` ([`6154bf3`](https://github.com/GitoxideLabs/gitoxide/commit/6154bf3a346d69f9749271d50e4f3aacdcbad4d0))
    - Thanks clippy ([`7f6bee5`](https://github.com/GitoxideLabs/gitoxide/commit/7f6bee5452ee01638f89a0cec2d4ee2a6f0d0136))
    - Merge branch 'status' ([`45edd2e`](https://github.com/GitoxideLabs/gitoxide/commit/45edd2ea66035adf526cb2f617873dcba60a2a9a))
    - `into_index_worktree_iter()` now takes an iterator, instead of a Vec. ([`18b2921`](https://github.com/GitoxideLabs/gitoxide/commit/18b2921aaa28df536faf74098d5f1f13d34148f9))
    - Show submodules in status independently of their active state. ([`719ced8`](https://github.com/GitoxideLabs/gitoxide/commit/719ced8a7949ba1f30fef13801e3466a7d1da590))
    - Make it easier to discover `is_path_excluded()` in documentation ([`c136329`](https://github.com/GitoxideLabs/gitoxide/commit/c13632959e287f31a00c1ba8fc6e97470f0cd734))
    - Adapt to changes in `gix-index` ([`1e1fce1`](https://github.com/GitoxideLabs/gitoxide/commit/1e1fce11a968ebbcede1135ccbd0b03e749a1267))
    - Merge branch 'patch-1' ([`9e9c653`](https://github.com/GitoxideLabs/gitoxide/commit/9e9c653a83df58f8cdfe3a7adb2d824c8a368e72))
    - Remove dep reqwest from gix ([`e3eedd8`](https://github.com/GitoxideLabs/gitoxide/commit/e3eedd8b5326b8de2e6fe8941e1851bdbad673ab))
</details>

## 0.61.1 (2024-03-22)

This release also updates `reqwest` to v0.12, bringing hyper 1.0 and a more recent `rustls` version.

### Bug Fixes

 - <csr-id-e1fec3c3a46a358036255a2487c2a48cc7176b4e/> missing closing backtick in gix lib documentation

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-packetline v0.17.5, gix-transport v0.41.3, gix v0.61.1 ([`57579f1`](https://github.com/GitoxideLabs/gitoxide/commit/57579f1ee4ef12c214db36325a2a0b2e8b2b14fd))
    - Prepare changelogs prior to release ([`7018a92`](https://github.com/GitoxideLabs/gitoxide/commit/7018a928a405ba0534442f0b538d58f520145376))
    - Merge branch 'patch-1' ([`8fde62b`](https://github.com/GitoxideLabs/gitoxide/commit/8fde62b2617985f835e2e2fa07c735a5158789cf))
    - Turn`curl` into a workspace package ([`adee500`](https://github.com/GitoxideLabs/gitoxide/commit/adee50016007619495c93580e845ae757377c4f0))
    - Make reqwest a workspace package ([`369cf1b`](https://github.com/GitoxideLabs/gitoxide/commit/369cf1b03735617debe1527b3f23247685181e7d))
    - Merge pull request #1325 from kdelorey/fix/simple-docs-formatting ([`3b34699`](https://github.com/GitoxideLabs/gitoxide/commit/3b34699d127a2fccbf4345ddc74070e56e26dd6e))
    - Fixed opening of backtick in documentation. ([`f1bc4cd`](https://github.com/GitoxideLabs/gitoxide/commit/f1bc4cd11aad91fc026c20979a02f3e9d8814d3d))
    - Missing closing backtick in gix lib documentation ([`e1fec3c`](https://github.com/GitoxideLabs/gitoxide/commit/e1fec3c3a46a358036255a2487c2a48cc7176b4e))
</details>

## 0.61.0 (2024-03-18)

### Documentation

 - <csr-id-e51b6b624994714c7e25d00e1204cefbf1b4ca12/> fix typo

### New Features (BREAKING)

 - <csr-id-ba3f2db0b65582a917466127dc16e4945104b01b/> provide `Repository::dirwalk_iter()`.
   That way, more copying happens but the usability increases tremendously as well.
   It's breaking as public types moved from `repository::dirwalk` to `dirwalk`,
   dissolving `repository::dirwalk` entirely.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 3 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.31.1, gix-object v0.42.1, gix-index v0.31.1, gix-pathspec v0.7.2, gix-dir v0.3.0, gix-status v0.8.0, gix v0.61.0, safety bump 2 crates ([`155cc45`](https://github.com/GitoxideLabs/gitoxide/commit/155cc45730b259e662d7c4be42a469a3af3750e1))
    - Prepare changelog prior to release ([`129ba3d`](https://github.com/GitoxideLabs/gitoxide/commit/129ba3deccc9ada0dc571466458845939502763d))
    - Merge branch 'improvements-for-cargo' ([`41cd53e`](https://github.com/GitoxideLabs/gitoxide/commit/41cd53e2af76e35e047aac4eca6324774df4cb50))
    - Provide `Repository::dirwalk_iter()`. ([`ba3f2db`](https://github.com/GitoxideLabs/gitoxide/commit/ba3f2db0b65582a917466127dc16e4945104b01b))
    - Adapt to changes in `gix-dir` ([`b90ab3d`](https://github.com/GitoxideLabs/gitoxide/commit/b90ab3dd5e8986e28624f3e1cf54f8a9171ce9f0))
    - Merge pull request #1318 from wtlin1228/main ([`4ccf39b`](https://github.com/GitoxideLabs/gitoxide/commit/4ccf39b9f52fd318a2eba4c63dd13e96269a4c99))
    - Refine typo-fix ([`c18734b`](https://github.com/GitoxideLabs/gitoxide/commit/c18734b05fcbc3d6f57bcb2c6525ec10015a7192))
    - Fix typo ([`e51b6b6`](https://github.com/GitoxideLabs/gitoxide/commit/e51b6b624994714c7e25d00e1204cefbf1b4ca12))
</details>

## 0.60.0 (2024-03-14)

### New Features

 - <csr-id-66e87cd31c060c3f97ac685ee0541c408f600362/> add `gix status --index-worktree-renames`
   This enables rename-tracking between worktree and index, something
   that Git also doesn't do or doesn't do by default.
   It is, however, available in `git2`.
 - <csr-id-c7ddd30fc9fde6cac55153fa8e7fd783c83b336f/> describing commits can now be done with conditional dirty-suffix using `commit::describe::Resolution::format_with_dirty_suffix()`
 - <csr-id-c20ad287128132cda995a47abac1dd18f415f02d/> add `Repository::is_dirty()`
   The simplest way to learn if the repository is dirty or not.
 - <csr-id-a29fa00d0727baffcba10c8f2f09115a362a2baf/> Add `Submodule::status()` method.
   That way it's possible to obtain submodule status information,
   with enough information to implement `git status`-like commands.
 - <csr-id-0330ad77edab88e14812c57f812c96c5e4561045/> add `Status` iterator.
   We also move the `IndexPersistedOrInMemory` type to the `worktree` module
   as its more widely useful.

### New Features (BREAKING)

 - <csr-id-57cf83b57b0de01bd69f63ec3637859ccd757272/> `diff::resource_cache()` now takes the attribute stack directly.
   That way, the constructor becaomes more versatile as the user can chose
   to pass attribute stacks that have more functionality, and thus can be
   used in more places.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 12 calendar days.
 - 18 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.5, gix-hash v0.14.2, gix-trace v0.1.8, gix-utils v0.1.11, gix-features v0.38.1, gix-actor v0.31.0, gix-validate v0.8.4, gix-object v0.42.0, gix-path v0.10.7, gix-glob v0.16.2, gix-quote v0.4.12, gix-attributes v0.22.2, gix-command v0.3.6, gix-filter v0.11.0, gix-fs v0.10.1, gix-chunk v0.4.8, gix-commitgraph v0.24.2, gix-hashtable v0.5.2, gix-revwalk v0.13.0, gix-traverse v0.38.0, gix-worktree-stream v0.11.0, gix-archive v0.11.0, gix-config-value v0.14.6, gix-tempfile v13.1.1, gix-lock v13.1.1, gix-ref v0.43.0, gix-sec v0.10.6, gix-config v0.36.0, gix-prompt v0.8.4, gix-url v0.27.2, gix-credentials v0.24.2, gix-ignore v0.11.2, gix-bitmap v0.2.11, gix-index v0.31.0, gix-worktree v0.32.0, gix-diff v0.42.0, gix-discover v0.31.0, gix-pathspec v0.7.1, gix-dir v0.2.0, gix-macros v0.1.4, gix-mailmap v0.23.0, gix-negotiate v0.13.0, gix-pack v0.49.0, gix-odb v0.59.0, gix-packetline v0.17.4, gix-transport v0.41.2, gix-protocol v0.44.2, gix-revision v0.27.0, gix-refspec v0.23.0, gix-status v0.7.0, gix-submodule v0.10.0, gix-worktree-state v0.9.0, gix v0.60.0, safety bump 26 crates ([`b050327`](https://github.com/GitoxideLabs/gitoxide/commit/b050327e76f234b19be921b78b7b28e034319fdb))
    - Prepare changelogs prior to release ([`52c3bbd`](https://github.com/GitoxideLabs/gitoxide/commit/52c3bbd36b9e94a0f3a78b4ada84d0c08eba27f6))
    - Merge branch 'status' ([`3e5c974`](https://github.com/GitoxideLabs/gitoxide/commit/3e5c974dd62ac134711c6c2f5a5490187a6ea55e))
    - Assure submodule status doesn't operate if there is no worktree checkout ([`3753592`](https://github.com/GitoxideLabs/gitoxide/commit/3753592ef2e33f138544f761d8e77742b80680d2))
    - Make `summary` available for `Item`. ([`da45d92`](https://github.com/GitoxideLabs/gitoxide/commit/da45d92f844d670dd23712a031584a4c3352708b))
    - Add `gix status --index-worktree-renames` ([`66e87cd`](https://github.com/GitoxideLabs/gitoxide/commit/66e87cd31c060c3f97ac685ee0541c408f600362))
    - Add `status.showUntrackedFiles` to config-tree and use it in `status()` ([`22abf60`](https://github.com/GitoxideLabs/gitoxide/commit/22abf605858404fcd38a5f4b8713358a526819ac))
    - Fix lints for nightly, and clippy ([`f8ce3d0`](https://github.com/GitoxideLabs/gitoxide/commit/f8ce3d0721b6a53713a9392f2451874f520bc44c))
    - Allow configuration of interrupts in status iter ([`f1ba7bd`](https://github.com/GitoxideLabs/gitoxide/commit/f1ba7bd459390080052024920992054f1d11cd3e))
    - Provide a non-parallel version of the status iteration ([`17bef30`](https://github.com/GitoxideLabs/gitoxide/commit/17bef301f2be29c8d0545b35d1581e57037e69df))
    - Describing commits can now be done with conditional dirty-suffix using `commit::describe::Resolution::format_with_dirty_suffix()` ([`c7ddd30`](https://github.com/GitoxideLabs/gitoxide/commit/c7ddd30fc9fde6cac55153fa8e7fd783c83b336f))
    - Add `Repository::is_dirty()` ([`c20ad28`](https://github.com/GitoxideLabs/gitoxide/commit/c20ad287128132cda995a47abac1dd18f415f02d))
    - Add submodule support for status iterator ([`4a4989d`](https://github.com/GitoxideLabs/gitoxide/commit/4a4989d5170173269dcdc19890827911d13e7a89))
    - Add `Submodule::status()` method. ([`a29fa00`](https://github.com/GitoxideLabs/gitoxide/commit/a29fa00d0727baffcba10c8f2f09115a362a2baf))
    - Add `Status` iterator. ([`0330ad7`](https://github.com/GitoxideLabs/gitoxide/commit/0330ad77edab88e14812c57f812c96c5e4561045))
    - `diff::resource_cache()` now takes the attribute stack directly. ([`57cf83b`](https://github.com/GitoxideLabs/gitoxide/commit/57cf83b57b0de01bd69f63ec3637859ccd757272))
    - Cargo fmt ([`b3556b2`](https://github.com/GitoxideLabs/gitoxide/commit/b3556b2237f4fd8298999fd6ba08a411c3a9471c))
    - Update gix-config setters. ([`ba3bf65`](https://github.com/GitoxideLabs/gitoxide/commit/ba3bf65808fbde44254e55955110ad43c9baedc5))
    - Gix-config now uses a Key trait rather than Into<&BStr> ([`6281e1a`](https://github.com/GitoxideLabs/gitoxide/commit/6281e1ac140c939b046ac88d536f16e076a3206c))
</details>

## 0.59.0 (2024-02-25)

### New Features

 - <csr-id-6914d1a7195d869ea776f30bbf29edb300f460be/> add `Repository::dirwalk_with_delegate()`.
   That way it's possible to perform arbitrary directory walks,
   useful for status, clean, and add.
 - <csr-id-d8bd45eb4dba4aca2ef009b1594f244c669625b8/> add `open::Options::current_dir()`.
   That way it's possible to obtain the current working directory
   with which the repository was opened.

### New Features (BREAKING)

 - <csr-id-0b1b44fa79a60ed40a9da154f7487408e6436941/> empty pathspecs with prefix now are optionally matching the prefix.
   Otherwise it's not possible to have the 'no pattern matches everything' case
   which is important in conjunction with prefixes and the requirement to
   still see everything outside of the prefix.

### Bug Fixes (BREAKING)

 - <csr-id-1e853961b0254893e277a0e14ee89099bac097f3/> leave more control to the user when creating pathspecs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 30 calendar days.
 - 36 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'status' ([`bb48c4c`](https://github.com/GitoxideLabs/gitoxide/commit/bb48c4ce22650b8c76af3b147e252ebe7cedb205))
    - Empty pathspecs with prefix now are optionally matching the prefix. ([`0b1b44f`](https://github.com/GitoxideLabs/gitoxide/commit/0b1b44fa79a60ed40a9da154f7487408e6436941))
    - Leave more control to the user when creating pathspecs ([`1e85396`](https://github.com/GitoxideLabs/gitoxide/commit/1e853961b0254893e277a0e14ee89099bac097f3))
    - Adapt to changes in `gix-dir` ([`ab0f63a`](https://github.com/GitoxideLabs/gitoxide/commit/ab0f63aa5ab90c3a18a62e72d486a889b540d804))
    - Merge pull request #1300 from DianaNites/patch-1 ([`e186199`](https://github.com/GitoxideLabs/gitoxide/commit/e186199cca96fccb82ac0b759fe5149ef1a9acf6))
    - Fix a typo in `gix::clone::PrepareFetch::new`, crate_opts -> create_opts ([`adbf8e8`](https://github.com/GitoxideLabs/gitoxide/commit/adbf8e8dd8a112ece7fe6c1167892297f73ea325))
    - Adapt to changes in `gix-status` ([`366dfb3`](https://github.com/GitoxideLabs/gitoxide/commit/366dfb375d1c4844e4b0edb934fa8c7a5c10b9b3))
    - Adapt to changes in `gix-dir` ([`e91accc`](https://github.com/GitoxideLabs/gitoxide/commit/e91accc8a1e03b04f0ae50c161201e53273f6e03))
    - Merge branch 'panic-msg-fix' ([`a86a5c0`](https://github.com/GitoxideLabs/gitoxide/commit/a86a5c053745660f7e130bdfcd7ae1fca7a9f3b2))
    - Fix into_{blob,tag} panic messages ([`b81d8ae`](https://github.com/GitoxideLabs/gitoxide/commit/b81d8aead1bb0c3e1186d035c88982889178e3f1))
    - Merge branch 'dirwalk' ([`face359`](https://github.com/GitoxideLabs/gitoxide/commit/face359443ba33e8985ec1525d5ec38b743ea7a9))
    - Add `Repository::dirwalk_with_delegate()`. ([`6914d1a`](https://github.com/GitoxideLabs/gitoxide/commit/6914d1a7195d869ea776f30bbf29edb300f460be))
    - Add `open::Options::current_dir()`. ([`d8bd45e`](https://github.com/GitoxideLabs/gitoxide/commit/d8bd45eb4dba4aca2ef009b1594f244c669625b8))
    - Merge branch 'tempfile-permissions' ([`7b44c7f`](https://github.com/GitoxideLabs/gitoxide/commit/7b44c7ff1dc0b8875214d2673c7f52948cf04ff0))
    - Release gix-tempfile v13.1.0, gix-lock v13.1.0, safety bump 12 crates ([`8430442`](https://github.com/GitoxideLabs/gitoxide/commit/84304427dfe4d170c7732161b126961719f70059))
    - Release gix-command v0.3.4 ([`8a62fb5`](https://github.com/GitoxideLabs/gitoxide/commit/8a62fb57f7751d3d57273d9430517487e555f999))
    - Release gix-path v0.10.5 ([`b8cba96`](https://github.com/GitoxideLabs/gitoxide/commit/b8cba96ce57f8b6b0067d6a8cf3e37eaf280a238))
</details>

## 0.58.0 (2024-01-20)

### New Features

<csr-id-a7e606b4dffe5c524b60a89fa0dbc753e80ce599/>
<csr-id-1ba9488a7c3737a4b5a15a871108924095c061b2/>

 - <csr-id-8847676ddd1aefb3610d384245ec6d692d5258de/> add `max-control` feature for fine-grained performance control.
   This also adds the following performance features:
   
   - `zlib-ng`

### Bug Fixes

 - <csr-id-e3c5a0feaeef5ca1683da0adee25154c9e868b3e/> `object::tree::diff::Platform::for_each_to_obtain_tree(callback)` errors are more convenient to use.
   Due to a change in how the generic error type is declared it should now be possible to
   use `anyhow` with it as well.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 18 calendar days.
 - 20 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#670](https://github.com/GitoxideLabs/gitoxide/issues/670)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#670](https://github.com/GitoxideLabs/gitoxide/issues/670)**
    - `object::tree::diff::Platform::for_each_to_obtain_tree(callback)` errors are more convenient to use. ([`e3c5a0f`](https://github.com/GitoxideLabs/gitoxide/commit/e3c5a0feaeef5ca1683da0adee25154c9e868b3e))
 * **Uncategorized**
    - Release gix-utils v0.1.9, gix-features v0.38.0, gix-actor v0.30.0, gix-object v0.41.0, gix-path v0.10.4, gix-glob v0.16.0, gix-attributes v0.22.0, gix-command v0.3.3, gix-packetline-blocking v0.17.3, gix-filter v0.9.0, gix-fs v0.10.0, gix-commitgraph v0.24.0, gix-revwalk v0.12.0, gix-traverse v0.37.0, gix-worktree-stream v0.9.0, gix-archive v0.9.0, gix-config-value v0.14.4, gix-tempfile v13.0.0, gix-lock v13.0.0, gix-ref v0.41.0, gix-sec v0.10.4, gix-config v0.34.0, gix-url v0.27.0, gix-credentials v0.24.0, gix-ignore v0.11.0, gix-index v0.29.0, gix-worktree v0.30.0, gix-diff v0.40.0, gix-discover v0.29.0, gix-mailmap v0.22.0, gix-negotiate v0.12.0, gix-pack v0.47.0, gix-odb v0.57.0, gix-pathspec v0.6.0, gix-packetline v0.17.3, gix-transport v0.41.0, gix-protocol v0.44.0, gix-revision v0.26.0, gix-refspec v0.22.0, gix-status v0.5.0, gix-submodule v0.8.0, gix-worktree-state v0.7.0, gix v0.58.0, safety bump 39 crates ([`eb6aa8f`](https://github.com/GitoxideLabs/gitoxide/commit/eb6aa8f502314f886fc4ea3d52ab220763968208))
    - Prepare changelogs prior to release ([`6a2e0be`](https://github.com/GitoxideLabs/gitoxide/commit/6a2e0bebfdf012dc2ed0ff2604086081f2a0f96d))
    - Merge branch 'finegrained-features' ([`d8570d0`](https://github.com/GitoxideLabs/gitoxide/commit/d8570d08c72a977b22ff44d4c8b49af6c3017885))
    - Add `max-control` feature for fine-grained performance control. ([`8847676`](https://github.com/GitoxideLabs/gitoxide/commit/8847676ddd1aefb3610d384245ec6d692d5258de))
    - Merge branch 'dirwalk' ([`5d176fc`](https://github.com/GitoxideLabs/gitoxide/commit/5d176fc5ab82bfc7c194b4d929e73da9659ae8b8))
    - Use `gix_fs::current_dir(precompose_unicode)`. ([`7d8d167`](https://github.com/GitoxideLabs/gitoxide/commit/7d8d1678c9dd185a369e996551a6ed89fbb391f5))
    - Adapt to changes in `gix-features` ([`eacb5a4`](https://github.com/GitoxideLabs/gitoxide/commit/eacb5a4ae2fd94b095005cfbc0a8b2aa67539e52))
    - Add `env::args_os_opt()` which takes an argument to determine input unicode-decomposition ([`a7e606b`](https://github.com/GitoxideLabs/gitoxide/commit/a7e606b4dffe5c524b60a89fa0dbc753e80ce599))
    - Release gix-trace v0.1.7, gix-features v0.37.2, gix-commitgraph v0.23.2, gix-traverse v0.36.2, gix-index v0.28.2 ([`b6c04c8`](https://github.com/GitoxideLabs/gitoxide/commit/b6c04c87b426bf36a059df8dc52b56d384b27b79))
    - Merge pull request #1248 from joshtriplett/tyop ([`39f35da`](https://github.com/GitoxideLabs/gitoxide/commit/39f35da390bc46005d0374b9bf4e7106fc1bd0ec))
    - Typo fixes ([`3ef3bc2`](https://github.com/GitoxideLabs/gitoxide/commit/3ef3bc20a1b90799e5ac26858f898bc7a7c96901))
    - `max-performance-zlib-ng-compat` flag ([`1ba9488`](https://github.com/GitoxideLabs/gitoxide/commit/1ba9488a7c3737a4b5a15a871108924095c061b2))
    - Add a max-performance-zlib-ng-compat flag ([`cfb06ec`](https://github.com/GitoxideLabs/gitoxide/commit/cfb06ec695f1926778c78362cc6cd6a8f48f7e84))
</details>

## 0.57.1 (2023-12-30)

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

## 0.57.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

 - <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
   Our MSRV follows the one of `helix`, which in turn follows Firefox.

### New Features

 - <csr-id-3fba5b856b37af2db40c29f463efe09dcfc8d085/> `Repository::rev_parse*()` now supports `branch@{upstream|push|u|p}`.
   Previously it would be parsed, but always error as the implementation didn't exist.
   Now it will return the fetch and push tracking branches respectively.
 - <csr-id-270322e75a49f9a5c2d996cf0a20c6d622b40394/> Add `Reference::remote_tracking_ref_name()` and `*::remote_ref_name()`.
   These methods mirror their respective `Repository::branch_*` prefixed versions.
 - <csr-id-4aa4b05b9dc785d550386218915208c0c9fdb78b/> add `Repository::branch_remote_tracking_ref_name()`.
 - <csr-id-8ac2dccd62d88c0c196d2fc054a68a1b69121da0/> add `push.default` config key
 - <csr-id-3f842134d0e2bbc9c62ee77af3c8e5c7fd9f47d1/> add `config::Snapshot::trusted_program()`.
   That way it's possible to obtain an executable, program or script
   from a key in the configuration that is in a trusted section of the
   configuration.
   
   This goes along with a new `command` feature that brings in the `command`
   module at the top level to be able to execute such commands.
 - <csr-id-b5c36b805d0b269e6d87ca8b6c517b7fd7337622/> add `clone::PrepareFetch::with_in_memory_config_overrides()`.
   With it one can affect the repository configuration right before fetching.

### New Features (BREAKING)

 - <csr-id-5c07c760e392e9aecbe521f30ade5b693f977dc0/> `Repository::remote_names|remote_default_name()` now returns `Cow<'_, BStr>` instead of `Cow<'_, str>`.
   That way information won't degenerate due to enforcement of UTF-8.

### Bug Fixes (BREAKING)

 - <csr-id-404fde55d3cc85acce207ca259b0fa9b144dd694/> rename `Repository::branch_remote_ref()` to `Repository::branch_remote_ref_name()`, add `direction` argument (also to `Repository::branch_remote_name()` and `Repository::branch_remote()`).
   This better differentiates the return value from the corresponding ref objects,
   which would require the named ref to exist in the repository.
   
   The `direction` argument allows to get the reference to push to as well.
   Further, it now takes a full ref name to support deriving the name of branches
   to push to.
   
   Regarding `Repository::branch_remote()`,  previously, this functionality
   was only available from a `Reference`,
   but now it's more generally available with just a branch name.
   
   The method was also adjusted to permit looking up non-symbolic remote
   names, like remotes that are specified by their URL.
 - <csr-id-59b8104a5320d946abc9f5736fa76696cef1459d/> mark `gix::interrupt::init_handler()` as unsafe
   The passed `interrupt()` argument will be called from a signal
   handler, so that needs to be documented and the call sites need to
   state that they fulfill the contract.
   
   Thanks to @Manishearth for pointing this out.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 41 commits contributed to the release over the course of 22 calendar days.
 - 22 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1158](https://github.com/GitoxideLabs/gitoxide/issues/1158), [#1165](https://github.com/GitoxideLabs/gitoxide/issues/1165), [#1178](https://github.com/GitoxideLabs/gitoxide/issues/1178), [#1191](https://github.com/GitoxideLabs/gitoxide/issues/1191)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1158](https://github.com/GitoxideLabs/gitoxide/issues/1158)**
    - Remove extra-lines from changelog ([`11c9f66`](https://github.com/GitoxideLabs/gitoxide/commit/11c9f66523011064e5e82d3b32713a7f4643809d))
 * **[#1165](https://github.com/GitoxideLabs/gitoxide/issues/1165)**
    - Complete partial note ([`8ef0538`](https://github.com/GitoxideLabs/gitoxide/commit/8ef053882957819e44485c356204a2689376ba1c))
 * **[#1178](https://github.com/GitoxideLabs/gitoxide/issues/1178)**
    - Add `config::Snapshot::trusted_program()`. ([`3f84213`](https://github.com/GitoxideLabs/gitoxide/commit/3f842134d0e2bbc9c62ee77af3c8e5c7fd9f47d1))
 * **[#1191](https://github.com/GitoxideLabs/gitoxide/issues/1191)**
    - Add note to clarify what users might want to do ([`2e04403`](https://github.com/GitoxideLabs/gitoxide/commit/2e04403589356c320f44f1516c29770d91a3d20b))
 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Thanks clippy ([`d38d1cc`](https://github.com/GitoxideLabs/gitoxide/commit/d38d1cc1aa3402629a0f182324e3310e730ce3f2))
    - Merge branch 'tracking-branch' ([`0fe20e8`](https://github.com/GitoxideLabs/gitoxide/commit/0fe20e80145419e1662f869657dabf689786395f))
    - Refactor ([`530c15d`](https://github.com/GitoxideLabs/gitoxide/commit/530c15d45227ac9e86b3edd8c029a9c7da8e5842))
    - `Repository::rev_parse*()` now supports `branch@{upstream|push|u|p}`. ([`3fba5b8`](https://github.com/GitoxideLabs/gitoxide/commit/3fba5b856b37af2db40c29f463efe09dcfc8d085))
    - Add `Reference::remote_tracking_ref_name()` and `*::remote_ref_name()`. ([`270322e`](https://github.com/GitoxideLabs/gitoxide/commit/270322e75a49f9a5c2d996cf0a20c6d622b40394))
    - Add `Repository::branch_remote_tracking_ref_name()`. ([`4aa4b05`](https://github.com/GitoxideLabs/gitoxide/commit/4aa4b05b9dc785d550386218915208c0c9fdb78b))
    - Rename `Repository::branch_remote_ref()` to `Repository::branch_remote_ref_name()`, add `direction` argument (also to `Repository::branch_remote_name()` and `Repository::branch_remote()`). ([`404fde5`](https://github.com/GitoxideLabs/gitoxide/commit/404fde55d3cc85acce207ca259b0fa9b144dd694))
    - `Repository::remote_names|remote_default_name()` now returns `Cow<'_, BStr>` instead of `Cow<'_, str>`. ([`5c07c76`](https://github.com/GitoxideLabs/gitoxide/commit/5c07c760e392e9aecbe521f30ade5b693f977dc0))
    - Add `push.default` config key ([`8ac2dcc`](https://github.com/GitoxideLabs/gitoxide/commit/8ac2dccd62d88c0c196d2fc054a68a1b69121da0))
    - Merge branch 'match_ceiling_dir_or_error' ([`cda5b51`](https://github.com/GitoxideLabs/gitoxide/commit/cda5b51f3b39153fd8919209e2210da3051b928d))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Release gix-ref v0.39.1 ([`c1cfe6e`](https://github.com/GitoxideLabs/gitoxide/commit/c1cfe6e4ab0d97ca98e93e1c01d9afa3b2c9a351))
    - Merge branch 'patch-1' ([`20dce42`](https://github.com/GitoxideLabs/gitoxide/commit/20dce42c85dc7c55a3ee1db42a6fd03c017ffc8a))
    - Differentiate between `Executable` and `Program` ([`56d1d09`](https://github.com/GitoxideLabs/gitoxide/commit/56d1d09307e65f3d721bdb73b8740734b94dbd62))
    - Add `core.editor` key ([`ff71e07`](https://github.com/GitoxideLabs/gitoxide/commit/ff71e07a45e62a6775f9e3e739e89c6718153c74))
    - Merge branch 'archive-handling' ([`7549559`](https://github.com/GitoxideLabs/gitoxide/commit/7549559fcbf42249939f41fd7aa34b4449eb1fec))
    - Check all git-lfs managed files into the repository ([`35439de`](https://github.com/GitoxideLabs/gitoxide/commit/35439defd2d71779d4b3795b7652cde18ff11150))
    - Git-lfs might fail early; let's rely on these caches to be recreated, where possible ([`b6f2b81`](https://github.com/GitoxideLabs/gitoxide/commit/b6f2b818f34e85edbdb0777a1df0cbf7fc9a0c98))
    - Release gix-hash v0.13.3, gix-index v0.27.1 ([`98b08f4`](https://github.com/GitoxideLabs/gitoxide/commit/98b08f4d0d9237be0e0c2caa9bf5c13ae8bbf9d8))
    - Merge branch 'mailmap-config-section' ([`8dda069`](https://github.com/GitoxideLabs/gitoxide/commit/8dda069306fb140b1267b213079310a974b2d979))
    - Use new `mailmap` keys and make a few improvements. ([`7f65ffd`](https://github.com/GitoxideLabs/gitoxide/commit/7f65ffdb57488d5561b2f54a613259d74e26b250))
    - Assign more suitable types to `mailmap` keys ([`1bf3e88`](https://github.com/GitoxideLabs/gitoxide/commit/1bf3e888e5b7c37bedb738d7a8f30dd3dbf3d75d))
    - Add config section for mailmap.{blob,file}. ([`86c7fa1`](https://github.com/GitoxideLabs/gitoxide/commit/86c7fa198c03152eaa99129472d03531d941e0a3))
    - Merge branch 'configure-prepare-fetch' ([`281fda0`](https://github.com/GitoxideLabs/gitoxide/commit/281fda06a89b1d38cf6afeda23cf80a68486140b))
    - Add `clone::PrepareFetch::with_in_memory_config_overrides()`. ([`b5c36b8`](https://github.com/GitoxideLabs/gitoxide/commit/b5c36b805d0b269e6d87ca8b6c517b7fd7337622))
    - Allow overriding Git configuration when cloning. ([`9833b45`](https://github.com/GitoxideLabs/gitoxide/commit/9833b45c59ebb078b3f5c35fdb0a5b9bd3453fbc))
    - Merge branch 'push-yvzxzqrkkvry' ([`4917beb`](https://github.com/GitoxideLabs/gitoxide/commit/4917beb5760a9bafb75b59331b282f4d6dbb64f5))
    - Fixup new unsafe interrupt handler ([`c23bb87`](https://github.com/GitoxideLabs/gitoxide/commit/c23bb878812a54f589fd1626a9ae8c3e12ce5ec5))
    - Mark `gix::interrupt::init_handler()` as unsafe ([`59b8104`](https://github.com/GitoxideLabs/gitoxide/commit/59b8104a5320d946abc9f5736fa76696cef1459d))
    - Reduce size of unsafe block in signal handler ([`d77bc0e`](https://github.com/GitoxideLabs/gitoxide/commit/d77bc0e96787d555fe8c6ca34b7710cda3681c8e))
    - Release gix-config v0.32.1 ([`cd26fd8`](https://github.com/GitoxideLabs/gitoxide/commit/cd26fd8babb023286ed9f6a6c71a06575de8d246))
    - Merge branch 'adjustments-for-cargo' ([`56588a9`](https://github.com/GitoxideLabs/gitoxide/commit/56588a9b3e97665f1dd4c11dc74a692f35abba60))
    - Fix import/prevent warning ([`ec0211a`](https://github.com/GitoxideLabs/gitoxide/commit/ec0211afa6313d0f640dc4cbb4c0988ad27009df))
</details>

## 0.56.0 (2023-12-06)

### New Features

 - <csr-id-27627248a019d85a904ecd8a57e395f34c1b16a4/> add `gitoxide.core.externalCommandStderr` to allow enabling `stderr` to the enclosing terminal.
   Previously, this was enabled by default, now it can additionally be disabled by
   the caller.
 - <csr-id-6cf73a44cbcd8bdca6a353cfd02d6237b1883b8c/> use `gitoxide.credentials.helperStderr` key to control how stderr is handled with helpers.
   That way users can configure each repository instance according to their needs,
   with which includes disabling the `stderr` of credential helpers.
 - <csr-id-77686db3f91e16fa6657dbae2182ec72e88d3fd0/> `revision::Spec::path_and_mode()`
   Provide additional information about revspecs for use with
   worktree filters.
 - <csr-id-6f4bbc31411cd3528cc6dd3db54a333ff861ec95/> add key for `diff.external`.
   That way it's conceivable that applications correctly run either
   a configured external diff tool, or one that is configured on a
   per diff-driver basis, while being allowed to fall back to
   a built-in implementation as needed.
 - <csr-id-4aea9b097fb08e504cdfc4a7c3b7511a308dc074/> add the`diff::resource_cache()` low-level utility for rapid in-memory diffing of combinations of resources.
   We also add the `object::tree::diff::Platform::for_each_to_obtain_tree_with_cache()` to pass a resource-cache
   for re-use between multiple invocation for significant savings.
 - <csr-id-dd575cd0e3749ff7f59c1582cec0524ff231667d/> Add config value gitoxide.http.sslNoVerify
   This value can by overriden by GIT_SSL_NO_VERIFY env variable. We use
   the value to override http.sslVerify when specifying ssl_verify in
   transport Options.
 - <csr-id-c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c/> In gix read http.sslVerify config value and pass it to gix-transport.
 - <csr-id-8434aab5fb6ce32be2bf3b20e38c28c780bd5db9/> add `gitoxide.core.refsNamespace` key and respect the `GIT_NAMESPACE` environment variable.
   It's also provided as context value.
 - <csr-id-0ed0a8936eaf73407721fe0e06da5d345a54956b/> make `verbose-object-parsing-errors` available in `gix`.
   That way, it's easy to create programs that are geared towards
   debugging repositories and finding invalid objects with detailed
   errors.
 - <csr-id-e95bb9fa3a69bec039ebf932b672496de753fe97/> add the `gitoxide.credentials.terminalPrompt` key to represent the GIT_TERMINAL_PROMPT
   That way, it's easy to control the usage of terminals without using and environment.
 - <csr-id-f34f46a3895a157036b099d6663d8953567119e7/> Add `http-client-curl-rustls` (CLI) and `blocking-http-transport-curl-rustls` (lib) features to avoid openssl.
   That way, we should be able to avoid crashes on certain CI configurations.
 - <csr-id-117357e7bbfcb1bfe887f85173e88db9436814b1/> add `Head::try_into_peeled_object()` and `Head::peel_to_object_in_place()`
   This makes it easier to peel to a specific object type, after
   all tags have been followed, without having to assume an intermediate
   commit.

### Bug Fixes

<csr-id-20f962e5d6a7c19ca097ccd3f06434f6c9501262/>

 - <csr-id-0b3eb141bee59ffc17c973a8d126efaa52edb9b3/> assure the correct repository is used for checkouts after clone.
   If this is not the case, it's possible for filters to run in the context of
   potential parent repositories, which then can have all kinds of issues.
   
   In case of `git-lfs`, for instance, it would try to download objects
   from the wrong repository.
 - <csr-id-3ff1827a12557a601da22d138beb97e8647d5d6e/> Allow multiple packs to be received one after another.
   Previously it would be difficult to perform another fetch operation on the
   same connection as the final flush packet after a pack wouldn't be consumed.
   
   This has now been mitigated by consuming it in the one place where knoweldge
   about this specialty exists.
 - <csr-id-8d9296ff150a887cb887ee6b6a9c4a9cb550cae0/> don't use `trust-dns` by default when using request.
   It's reported to have issues under certain condition, please see
   https://github.com/seanmonstar/reqwest/pull/437 for more.
   
   The `blocking-http-transport-reqwest-rust-tls-trust-dns` feature was added
   to provide the same feature-set as before for those who want `trust-dns`.
 - <csr-id-6295dec2bdd6c3bb35e45db7a486651ebfe50369/> V1 negotiation won't hang anymore
   The logic previously tried to estimate when a pack can be expected,
   and when a NAK is the end of a block, or the beginning of a pack.
   
   This can be known because a pack (with our settings) needs two things:
   
   * the server thinks it's ready
* a `done` sent by the client

### New Features (BREAKING)

 - <csr-id-4743212269c6fab69f6306fba88ee38b669a7dc3/> `object::blob::diff::Platform` now performs all necessary conversions.
   Previously it would just offer the git-ODB version of a blob for diffing,
   while it will now make it possible to apply all necessary conversion steps
   for you.
   
   This also moves `Event::diff()` to `Change::diff()`, adds
   `Repository::diff_resource_cache()` and refactors nearly everything
   about the `objects::blob::diff::Platform`.
 - <csr-id-089c4dc8b7d323637e5f9a9f7446f2a8e9f51ce1/> generalize rename-tracking engine for later use with status.
   Previously the rename tracking engine was integrated with tree-diffs,
   but already operates in a stand-alone fashion.
   Now it's officially generalized which allows it to be tested separately
   and used when tracking renames for diffs between index and tree, index
   and index, and index and worktree.
 - <csr-id-c3edef1c0c49accbb037bdf086dade3ed0e5e507/> make it possible to trace incoming and outgoing packetlines.
   Due to the way this is (and has to be) setup, unfortunately one
   has to integrate that with two crates, instead of just one.
   
   This changes touches multiple crates, most of which receive a single
   boolean as last argument to indicate whether the tracing should
   happen in the first place.
 - <csr-id-4e6a4e6ef440c72f61513ba82b439b9dca298e73/> improve `head()` peeling API
   Previously it was partially untested and it was hard to obtain an object of choice.
   
   Further breaking changes:
   
   * rename `Head::peeled()` to `into_peeled_id()`
* rename `Head::into_fully_peeled_id()` to `try_peel_into_id()`
* rename `Head::peel_to_id_in_place()` to `Head::try_peel_to_id_in_place()`

### Bug Fixes (BREAKING)

 - <csr-id-2189cee47f99350b368390eaa2a01961bb77c250/> rename `GITOXIDE_*` environment variables to `GIX_#`
 - <csr-id-88f8b342ab317696bcab8a0fe75c042e7290a75c/> Remove unsafe transmute of should_interrupt
   Adds a lifetime to the ExtendedBufRead trait to specify how long the
   callback provided must live.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 69 commits contributed to the release.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1061](https://github.com/GitoxideLabs/gitoxide/issues/1061), [#1076](https://github.com/GitoxideLabs/gitoxide/issues/1076), [#1090](https://github.com/GitoxideLabs/gitoxide/issues/1090), [#1125](https://github.com/GitoxideLabs/gitoxide/issues/1125), [#1129](https://github.com/GitoxideLabs/gitoxide/issues/1129), [#972](https://github.com/GitoxideLabs/gitoxide/issues/972)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1061](https://github.com/GitoxideLabs/gitoxide/issues/1061)**
    - V1 negotiation won't hang anymore ([`6295dec`](https://github.com/GitoxideLabs/gitoxide/commit/6295dec2bdd6c3bb35e45db7a486651ebfe50369))
 * **[#1076](https://github.com/GitoxideLabs/gitoxide/issues/1076)**
    - Don't use `trust-dns` by default when using request. ([`8d9296f`](https://github.com/GitoxideLabs/gitoxide/commit/8d9296ff150a887cb887ee6b6a9c4a9cb550cae0))
 * **[#1090](https://github.com/GitoxideLabs/gitoxide/issues/1090)**
    - Add the `gitoxide.credentials.terminalPrompt` key to represent the GIT_TERMINAL_PROMPT ([`e95bb9f`](https://github.com/GitoxideLabs/gitoxide/commit/e95bb9fa3a69bec039ebf932b672496de753fe97))
 * **[#1125](https://github.com/GitoxideLabs/gitoxide/issues/1125)**
    - Fix; `SnapshotMut::set_value()` now sets values for keys in subsections as well. ([`d8452a0`](https://github.com/GitoxideLabs/gitoxide/commit/d8452a074f9f371c09ab3b06ae1870c90cf90475))
 * **[#1129](https://github.com/GitoxideLabs/gitoxide/issues/1129)**
    - Assure the correct repository is used for checkouts after clone. ([`0b3eb14`](https://github.com/GitoxideLabs/gitoxide/commit/0b3eb141bee59ffc17c973a8d126efaa52edb9b3))
 * **[#972](https://github.com/GitoxideLabs/gitoxide/issues/972)**
    - Allow multiple packs to be received one after another. ([`3ff1827`](https://github.com/GitoxideLabs/gitoxide/commit/3ff1827a12557a601da22d138beb97e8647d5d6e))
 * **Uncategorized**
    - Release gix v0.56.0 ([`476d5ef`](https://github.com/GitoxideLabs/gitoxide/commit/476d5ef2309ed0de20f3a46f0aad803579ab9ed4))
    - Release gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`c8568b9`](https://github.com/GitoxideLabs/gitoxide/commit/c8568b9c9bf883f77e81a9a98b1fc2cbe726df79))
    - Release gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`d3fd11e`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd11ec3783843d4e49081e1d14359ed9714b5f))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - Merge branch 'adjustments-for-cargo' ([`8156340`](https://github.com/GitoxideLabs/gitoxide/commit/8156340724b1b7cb15824f88c75f6ddd7302cff5))
    - Add `gitoxide.core.externalCommandStderr` to allow enabling `stderr` to the enclosing terminal. ([`2762724`](https://github.com/GitoxideLabs/gitoxide/commit/27627248a019d85a904ecd8a57e395f34c1b16a4))
    - Use `gitoxide.credentials.helperStderr` key to control how stderr is handled with helpers. ([`6cf73a4`](https://github.com/GitoxideLabs/gitoxide/commit/6cf73a44cbcd8bdca6a353cfd02d6237b1883b8c))
    - Rename `GITOXIDE_*` environment variables to `GIX_#` ([`2189cee`](https://github.com/GitoxideLabs/gitoxide/commit/2189cee47f99350b368390eaa2a01961bb77c250))
    - Merge branch 'gix-status' ([`5fdc9df`](https://github.com/GitoxideLabs/gitoxide/commit/5fdc9df069f3d9a4bd88e4e0ca5d67916e2908c9))
    - Merge branch 'remove-unsafe' ([`d2ba97c`](https://github.com/GitoxideLabs/gitoxide/commit/d2ba97c057de62022d4b8b720750c3a706ac0f9c))
    - Remove unsafe transmute of should_interrupt ([`88f8b34`](https://github.com/GitoxideLabs/gitoxide/commit/88f8b342ab317696bcab8a0fe75c042e7290a75c))
    - `revision::Spec::path_and_mode()` ([`77686db`](https://github.com/GitoxideLabs/gitoxide/commit/77686db3f91e16fa6657dbae2182ec72e88d3fd0))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'gix-status' ([`dfb3f18`](https://github.com/GitoxideLabs/gitoxide/commit/dfb3f1821428f294f1832543ad0cf2fc883b03fb))
    - Adapt to changes in `gix-diff` ([`1706e23`](https://github.com/GitoxideLabs/gitoxide/commit/1706e2394380c35cd98d0e106eb0985ae1912da0))
    - `object::blob::diff::Platform` now performs all necessary conversions. ([`4743212`](https://github.com/GitoxideLabs/gitoxide/commit/4743212269c6fab69f6306fba88ee38b669a7dc3))
    - Add key for `diff.external`. ([`6f4bbc3`](https://github.com/GitoxideLabs/gitoxide/commit/6f4bbc31411cd3528cc6dd3db54a333ff861ec95))
    - Add the`diff::resource_cache()` low-level utility for rapid in-memory diffing of combinations of resources. ([`4aea9b0`](https://github.com/GitoxideLabs/gitoxide/commit/4aea9b097fb08e504cdfc4a7c3b7511a308dc074))
    - Merge branch 'support_ssl_verify' ([`5ce9784`](https://github.com/GitoxideLabs/gitoxide/commit/5ce978432231e257ef625fc401895b34f963bf6d))
    - Refactor ([`ead00e9`](https://github.com/GitoxideLabs/gitoxide/commit/ead00e9c8864eca804b8ba0dbf6792e28da85ecc))
    - Add config value gitoxide.http.sslNoVerify ([`dd575cd`](https://github.com/GitoxideLabs/gitoxide/commit/dd575cd0e3749ff7f59c1582cec0524ff231667d))
    - In gix read http.sslVerify config value and pass it to gix-transport. ([`c6e83cf`](https://github.com/GitoxideLabs/gitoxide/commit/c6e83cf69f1a17e9ba3010bcce3a4ddd3305424c))
    - Merge pull request #1140 from bittrance/fix-pr1127 ([`698caaa`](https://github.com/GitoxideLabs/gitoxide/commit/698caaa53447c3d87a39749d6bc7526c6acbfe14))
    - Connect new gitoxide.credentials subsection into section tree. ([`8b8704f`](https://github.com/GitoxideLabs/gitoxide/commit/8b8704f8cd4382d0955fbf34c8f653b6ec3ff159))
    - Adapt to changes in `gix-filter` ([`1763862`](https://github.com/GitoxideLabs/gitoxide/commit/17638628586900d43d730e6ed2a0862d8e408f29))
    - Merge branch 'improve-filters' ([`f09ea13`](https://github.com/GitoxideLabs/gitoxide/commit/f09ea13b94a8dad695e4d26533fcd5c739043574))
    - Add `gitoxide.core.refsNamespace` key and respect the `GIT_NAMESPACE` environment variable. ([`8434aab`](https://github.com/GitoxideLabs/gitoxide/commit/8434aab5fb6ce32be2bf3b20e38c28c780bd5db9))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'sh-on-windows' ([`2b80d84`](https://github.com/GitoxideLabs/gitoxide/commit/2b80d8424196088d4ccc36914c87e320e4416ea1))
    - Remove special handling in favor of allowing shell-avoidance. ([`a0cc80d`](https://github.com/GitoxideLabs/gitoxide/commit/a0cc80d21e74a43d5770cf08a221ef92f39920bb))
    - Merge branch 'fix-1103' ([`d75159c`](https://github.com/GitoxideLabs/gitoxide/commit/d75159c6d49c01c24c97777c718a76261b88e5d3))
    - Adapt to changes in `gix-credentials` ([`c712850`](https://github.com/GitoxideLabs/gitoxide/commit/c7128502d2f8a97b5f730920c056bbda7f4509a5))
    - Merge branch 'gix-status' ([`c87f2cc`](https://github.com/GitoxideLabs/gitoxide/commit/c87f2cc7a499cbd354c03c40f9923c80845fc56c))
    - Generalize rename-tracking engine for later use with status. ([`089c4dc`](https://github.com/GitoxideLabs/gitoxide/commit/089c4dc8b7d323637e5f9a9f7446f2a8e9f51ce1))
    - Merge branch 'error' ([`c372321`](https://github.com/GitoxideLabs/gitoxide/commit/c372321dd6ea66a41c135d28c7319ab05a6d0942))
    - Make `verbose-object-parsing-errors` available in `gix`. ([`0ed0a89`](https://github.com/GitoxideLabs/gitoxide/commit/0ed0a8936eaf73407721fe0e06da5d345a54956b))
    - Merge branch 'fix-1096' ([`ff99a18`](https://github.com/GitoxideLabs/gitoxide/commit/ff99a18e9f9388542a9cbf17d61b413f34b1d533))
    - Adapt to changes in `gix-object` ([`203d69c`](https://github.com/GitoxideLabs/gitoxide/commit/203d69c8890acc716bd4f7a7b1b2b91a8c828bde))
    - Merge branch 'caio/main' ([`7227410`](https://github.com/GitoxideLabs/gitoxide/commit/72274107fdb8c8faa93a4abbe1382ca3301003c9))
    - Count removed bytes correctly ([`267b13d`](https://github.com/GitoxideLabs/gitoxide/commit/267b13d9ebd089d4ffb788e7cb94895914a1fd1d))
    - Merge branch 'gix-object-find' ([`c8bd660`](https://github.com/GitoxideLabs/gitoxide/commit/c8bd66065316176dfbbfe7ecaa092a25cad1854b))
    - Thanks clippy ([`82b01c2`](https://github.com/GitoxideLabs/gitoxide/commit/82b01c28bbbcd3b8ce346d1977fe7d8587273be6))
    - Adapt to changes related to usage of `gix-object::Find` trait where necessary ([`5761a4d`](https://github.com/GitoxideLabs/gitoxide/commit/5761a4daf80e5febe469e32220b71dc3063fb4a6))
    - Adapt to changes in `gix_object` and `gix_odb`. ([`24e319e`](https://github.com/GitoxideLabs/gitoxide/commit/24e319e996b4822782521430a2d0e8ce3710f123))
    - Merge branch 'BloopAI/main' ([`c197cbf`](https://github.com/GitoxideLabs/gitoxide/commit/c197cbfedf1405ac9b95b4f9d6630b98b1bac89f))
    - Add feature to allow using rustls without trust-dns ([`ea8cd0e`](https://github.com/GitoxideLabs/gitoxide/commit/ea8cd0e45793c374cfc6ebbacd09b09ebfbecfe4))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
    - Merge branch 'fix-v1-negotiation' ([`eb23338`](https://github.com/GitoxideLabs/gitoxide/commit/eb23338b847af2b26c797e6e903969a569deb0a7))
    - Merge branch 'trace-packetlines' ([`e7de4c7`](https://github.com/GitoxideLabs/gitoxide/commit/e7de4c702a223ad9eb19b407391028dcb08d80c4))
    - Make it possible to trace incoming and outgoing packetlines. ([`c3edef1`](https://github.com/GitoxideLabs/gitoxide/commit/c3edef1c0c49accbb037bdf086dade3ed0e5e507))
    - Merge branch 'discover-split-worktree' ([`16170d9`](https://github.com/GitoxideLabs/gitoxide/commit/16170d9c2e4de6a2e639ff99b75e65bbd0e782d7))
    - Improve the error message around incorrect worktree paths ([`dd57957`](https://github.com/GitoxideLabs/gitoxide/commit/dd57957b34425ea1a61304222f42ccaa667224bd))
    - Allow to open split worktree repositories ([`20f962e`](https://github.com/GitoxideLabs/gitoxide/commit/20f962e5d6a7c19ca097ccd3f06434f6c9501262))
    - Merge branch 'fuzz' ([`c5a7e66`](https://github.com/GitoxideLabs/gitoxide/commit/c5a7e66d901868237ef5a4f86534b9878cc397ff))
    - Add `http-client-curl-rustls` (CLI) and `blocking-http-transport-curl-rustls` (lib) features to avoid openssl. ([`f34f46a`](https://github.com/GitoxideLabs/gitoxide/commit/f34f46a3895a157036b099d6663d8953567119e7))
    - Make it easier to see what's happening during negotiation with `tracing enabled` ([`4e48558`](https://github.com/GitoxideLabs/gitoxide/commit/4e485585ccdc1f98d7627eab9b58729dc526c73c))
    - Release gix-url v0.25.1 ([`47a1241`](https://github.com/GitoxideLabs/gitoxide/commit/47a1241484fdb424184ca37f85a8b287d374d2a1))
    - Merge branch 'head-conversions' ([`c2cf20c`](https://github.com/GitoxideLabs/gitoxide/commit/c2cf20cd2d685c2c24527729fff35fd0a7903742))
    - Add `Head::try_into_peeled_object()` and `Head::peel_to_object_in_place()` ([`117357e`](https://github.com/GitoxideLabs/gitoxide/commit/117357e7bbfcb1bfe887f85173e88db9436814b1))
    - Improve `head()` peeling API ([`4e6a4e6`](https://github.com/GitoxideLabs/gitoxide/commit/4e6a4e6ef440c72f61513ba82b439b9dca298e73))
</details>

## 0.55.2 (2023-10-13)

### Bug Fixes

 - <csr-id-8011c73ee401bfca03811a249c46a4dd468af1b8/> bump `gix-transport` version to prevent it from being picked up.
   `gix-transport` v0.37.1 could accidentally be picked up by older, incompatible,
   `gix` versions which now fail to build.
   
   Thus v0.37.1 is now yanked and replaced with v0.38.0 along with a new
   release of `gix` to go with it.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.38.0, gix-protocol v0.41.1, gix v0.55.2, gitoxide-core v0.33.1, gitoxide v0.31.1 ([`1955a57`](https://github.com/GitoxideLabs/gitoxide/commit/1955a57f003f7d731d04e582e70ea86f15e8e7d9))
    - Prepare changelogs prior to release ([`12b5caf`](https://github.com/GitoxideLabs/gitoxide/commit/12b5cafc49baf07d00313de468970a2db33ac1f8))
    - Bump `gix-transport` version to prevent it from being picked up. ([`8011c73`](https://github.com/GitoxideLabs/gitoxide/commit/8011c73ee401bfca03811a249c46a4dd468af1b8))
</details>

## 0.55.1 (2023-10-12)

### New Features

 - <csr-id-5732303180d26374016b70bdd7fa0278dd84cff3/> Add `take_data()` to all primitive object types.
   That is the new, most direct way to obtain its data which otherwise
   is immovable.
 - <csr-id-88f2e6c4c540b9c8032e6eee9c5da65a9bcfeef8/> Add `detach()` and `detached()` too all object types.
   That way, the detachment API is symmetric.
   It's required to overcome the `Drop` implementation of each of these types
   which prevents moving data out of the object (easily).

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.55.1 ([`4642c0c`](https://github.com/GitoxideLabs/gitoxide/commit/4642c0c78f45b1956837bc874f6757fc302bee4a))
    - Add `take_data()` to all primitive object types. ([`5732303`](https://github.com/GitoxideLabs/gitoxide/commit/5732303180d26374016b70bdd7fa0278dd84cff3))
    - Add `detach()` and `detached()` too all object types. ([`88f2e6c`](https://github.com/GitoxideLabs/gitoxide/commit/88f2e6c4c540b9c8032e6eee9c5da65a9bcfeef8))
</details>

## 0.55.0 (2023-10-12)

<csr-id-f478a3722f0be35c109ea60b79cd4ac6e607480b/>

This release contains a complete rewrite of the internal url parsing logic, the public interface stays mostly the same however. Gitoxide will now be
more correct, interpreting more urls the same way Git does. Improvements include the added support for ssh aliases (`github:byron/gitoxide` has previously
been parsed as local path), adjustments around the interpretation of colons in file names (previously we disallowed colons that were not followed up
with a slash character) and some smaller changes that bring the interpretation of file urls more in line with Git's implementation. Additionally, the
error types have been adjusted to print a more comprehensive message by default, making sure they stay helpful even when bubbled up through multiple abstraction
layers.

There are still many (edge) cases in Git's url parsing implementation which are not handled correctly by Gitoxide. If you notice any such deviation please
open a new issue to help us making Gitoxide even more correct.

### Other

 - <csr-id-f478a3722f0be35c109ea60b79cd4ac6e607480b/> inform about the absence of strict hash verification and strict object creation.
   Those are present in `git2` and enabled by default, and `gitoxde` definitely
   wants to do the same at some point.

### New Features

 - <csr-id-c79a7daa30fe90d14d8e3387ec48116b37faf460/> add `Repository::head_tree()` to more easily obtain the current tree.
 - <csr-id-787a9aa91c1abaa7572f5d19f8a2acbb7ecc0732/> Add `Repository::has_object()` as a high-level alternative.
   Previously, one would have to call `repo.objects.contains()`, which
   is fine, but this method is necessary for symmetry of the API
   and one shouldn't have to drop down a level to do this.
   
   This method also knows empty trees as special case.
 - <csr-id-3cec935e692eeb33ffcac98988e34a390f755bf3/> add `Object::try_into_blob()` and `into_blob()` and `Repository::empty_blob()`
   This way it's easier to assert that an object is actually a blob.
 - <csr-id-7d9ecdd1c230204468a965f703d5efd00fa7fb79/> add `Repository::index_or_empty()`.
   This is useful if a missing index should mean it's empty.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 16 calendar days.
 - 17 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.37.1, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0 ([`14ddbd4`](https://github.com/GitoxideLabs/gitoxide/commit/14ddbd4c15128b1d5631a2388a00e024842b7b83))
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
    - Merge branch 'improvements' ([`429e7b2`](https://github.com/GitoxideLabs/gitoxide/commit/429e7b25f93c8a7947db19bafa74babf199a1aa6))
    - Inform about the absence of strict hash verification and strict object creation. ([`f478a37`](https://github.com/GitoxideLabs/gitoxide/commit/f478a3722f0be35c109ea60b79cd4ac6e607480b))
    - Add `Repository::head_tree()` to more easily obtain the current tree. ([`c79a7da`](https://github.com/GitoxideLabs/gitoxide/commit/c79a7daa30fe90d14d8e3387ec48116b37faf460))
    - Add `Repository::has_object()` as a high-level alternative. ([`787a9aa`](https://github.com/GitoxideLabs/gitoxide/commit/787a9aa91c1abaa7572f5d19f8a2acbb7ecc0732))
    - Add `Object::try_into_blob()` and `into_blob()` and `Repository::empty_blob()` ([`3cec935`](https://github.com/GitoxideLabs/gitoxide/commit/3cec935e692eeb33ffcac98988e34a390f755bf3))
    - Thanks clippy ([`345712d`](https://github.com/GitoxideLabs/gitoxide/commit/345712dcdfddcccc630bbfef2ed4f461b21550d3))
    - Merge branch 'reset' ([`b842691`](https://github.com/GitoxideLabs/gitoxide/commit/b8426919a491dc3a7df01ee3f258fc0d8a3a327c))
    - Trust Ctime again ([`f929d42`](https://github.com/GitoxideLabs/gitoxide/commit/f929d420cb768f2df1d7886564ca03b3c3254a82))
    - Add `Repository::index_or_empty()`. ([`7d9ecdd`](https://github.com/GitoxideLabs/gitoxide/commit/7d9ecdd1c230204468a965f703d5efd00fa7fb79))
    - Adapt to changes in `gix-status` ([`54fb7c2`](https://github.com/GitoxideLabs/gitoxide/commit/54fb7c24a97cb2339a67ad269344ce65166a545d))
    - Merge branch 'gix-url-parse-rewrite' ([`a12e4a8`](https://github.com/GitoxideLabs/gitoxide/commit/a12e4a88d5f5636cd694c72ce45a8b75aa754d28))
    - Update changelogs ([`4349353`](https://github.com/GitoxideLabs/gitoxide/commit/43493531bbf3049bee3d7b14b7a6dbe874e37ebc))
</details>

## 0.54.1 (2023-09-25)

### Bug Fixes

 - <csr-id-300a83821358f2a43649515606ebb84741e82780/> local refs created during fetching will now always be valid.
   Previously it could create symbolic refs that were effectively unborn, i.e.
   point to a reference which doesn't exist.
   
   Now these will always point to the peeled object instead.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.54.1 ([`f603fd7`](https://github.com/GitoxideLabs/gitoxide/commit/f603fd7a68206a6989a9f959216eba6cca0a6733))
    - Local refs created during fetching will now always be valid. ([`300a838`](https://github.com/GitoxideLabs/gitoxide/commit/300a83821358f2a43649515606ebb84741e82780))
</details>

## 0.54.0 (2023-09-24)

<csr-id-e022096aa495f55a05f83860243f49552be501f7/>
<csr-id-79e47a512507c7fd7acbdff624a5249e24505e0d/>

### New Features

 - <csr-id-f9d14d86a6578cf0f9a0c4a2256ad227b9264340/> Add `PathspecDetached` as pathspec that can more easily be used across threads.
 - <csr-id-f066f9889b57a4ffaebc0ed1442d77999498db42/> `PathSpec` implements `gix_status::PathSpec` to allow it to be used there.
   The reason we need a trait and can't do with simply a function is that multiple calls
   are needed to test for inclusion *and* allow the common-prefix optimization.
 - <csr-id-a8333f1137df51d237f6debf056ac075b0a2cd94/> add `Repository::stat_options()` to learn how an index would compare filesystem stats.
 - <csr-id-2734e84b74b761bff27fc1eb27f57d9d839c9240/> add `parallel` feature toggle
   Make certain data structure threadsafe (or `Sync`) to facilitate multithreading.
   Further, many algorithms will now use multiple threads by default.
   If unset, most of `gix` can only be used in a single thread
   as data structures won't be `Send` anymore.

### Bug Fixes

 - <csr-id-e22893c1c95a76d9a5f3b2f2a4e2a30f815ee7e5/> do not trust ctime by default.
   On MacOS it seems to be off by two seconds right from the source, which
   seems to be an issue `stat` isn't having.
 - <csr-id-334281c8771790df7a022daa4a700c96b99acbc0/> ignore empty `core.askpass` settings
   This is the same as what `git` does, it's explicit per value, which
   means that other paths might be flagged as empty automatically.

### Other

 - <csr-id-e022096aa495f55a05f83860243f49552be501f7/> add note about the trust-model.
   It should explain why `gix` is happy to open repositories that won't
   be handled by `git` unless overrides are set.

### Test

 - <csr-id-79e47a512507c7fd7acbdff624a5249e24505e0d/> add assertion to assure `ThreadSafeRepository` is sync.
   If it doesn't appear to be sync, be sure to use the `max-performance-safe` feature.

### Bug Fixes (BREAKING)

 - <csr-id-ee9276f2a7789c20d88d40624ad648e44b604a27/> `PrepareCheckout::main_worktree()` now takes `Progress` as geric argument.
   This makes it more flexible and convenient, but is technically a breaking change.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 31 commits contributed to the release.
 - 15 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
    - Merge branch 'reset' ([`54a8495`](https://github.com/GitoxideLabs/gitoxide/commit/54a849545140f7f1c0c7564c418071c0a76a34e7))
    - Add `PathspecDetached` as pathspec that can more easily be used across threads. ([`f9d14d8`](https://github.com/GitoxideLabs/gitoxide/commit/f9d14d86a6578cf0f9a0c4a2256ad227b9264340))
    - Do not trust ctime by default. ([`e22893c`](https://github.com/GitoxideLabs/gitoxide/commit/e22893c1c95a76d9a5f3b2f2a4e2a30f815ee7e5))
    - `PathSpec` implements `gix_status::PathSpec` to allow it to be used there. ([`f066f98`](https://github.com/GitoxideLabs/gitoxide/commit/f066f9889b57a4ffaebc0ed1442d77999498db42))
    - Add `Repository::stat_options()` to learn how an index would compare filesystem stats. ([`a8333f1`](https://github.com/GitoxideLabs/gitoxide/commit/a8333f1137df51d237f6debf056ac075b0a2cd94))
    - Fix compile time warning ([`4ce7f7c`](https://github.com/GitoxideLabs/gitoxide/commit/4ce7f7c2c8bb66f0c093bf6e4d20f5568ca04f6a))
    - Merge branch 'parallel-feature' ([`c270f78`](https://github.com/GitoxideLabs/gitoxide/commit/c270f7883e1ea8156d521b12d161a47b2144425c))
    - Add `parallel` feature toggle ([`2734e84`](https://github.com/GitoxideLabs/gitoxide/commit/2734e84b74b761bff27fc1eb27f57d9d839c9240))
    - Add assertion to assure `ThreadSafeRepository` is sync. ([`79e47a5`](https://github.com/GitoxideLabs/gitoxide/commit/79e47a512507c7fd7acbdff624a5249e24505e0d))
    - Merge pull request #1015 from NobodyXu/optimize/prepare-checkout ([`14312b6`](https://github.com/GitoxideLabs/gitoxide/commit/14312b6c21d6382f7e536db7a1d8b519b97b8300))
    - Merge branch 'path-config' ([`9c528dc`](https://github.com/GitoxideLabs/gitoxide/commit/9c528dc8282c8b2f3a023e523dccdd0f7a711e61))
    - Merge pull request #1012 from NobodyXu/optimization/try-into-de-momo ([`afb1960`](https://github.com/GitoxideLabs/gitoxide/commit/afb1960e3e13bc9fc7cc5f3a3a244945f00966ad))
    - Ignore empty `core.askpass` settings ([`334281c`](https://github.com/GitoxideLabs/gitoxide/commit/334281c8771790df7a022daa4a700c96b99acbc0))
    - Merge branch 'optimize/progress-use' ([`1f2ffb6`](https://github.com/GitoxideLabs/gitoxide/commit/1f2ffb6d86ef073caf43a2f7a77fe712a1aa495e))
    - `PrepareCheckout::main_worktree()` now takes `Progress` as geric argument. ([`ee9276f`](https://github.com/GitoxideLabs/gitoxide/commit/ee9276f2a7789c20d88d40624ad648e44b604a27))
    - Add note about the trust-model. ([`e022096`](https://github.com/GitoxideLabs/gitoxide/commit/e022096aa495f55a05f83860243f49552be501f7))
    - Optimize `clone::PrepareCheckout::main_worktree`` ([`938f518`](https://github.com/GitoxideLabs/gitoxide/commit/938f5187f0ff51561971ca463584ec0db93f3455))
    - Fix `maybe_async` ([`c80e809`](https://github.com/GitoxideLabs/gitoxide/commit/c80e809c2655d15d7f22170782dacd64fe2e01bd))
    - Rm unused clippy lint ([`d82f84b`](https://github.com/GitoxideLabs/gitoxide/commit/d82f84b23a3def8e237e2b2511874c6045032c04))
    - Fixed error by also using trait object in `remote::fetch::Prepare::receive` ([`44faa01`](https://github.com/GitoxideLabs/gitoxide/commit/44faa01cf0612df5685922710d4a0adf6715ef77))
    - Revert changes to binary files ([`3eb8653`](https://github.com/GitoxideLabs/gitoxide/commit/3eb8653b78f2a0ca654fbebb185f3d6416d779d5))
    - Rm binary files ([`6a33594`](https://github.com/GitoxideLabs/gitoxide/commit/6a335940332b8d5069cb0c310d2d8e43fbeee01e))
    - Use trait object for `progress` in `PrepareFetch::fetch_only` ([`70989b3`](https://github.com/GitoxideLabs/gitoxide/commit/70989b3965077ae00ec6cf344f31627a804a8225))
    - Fix clippy warnings ([`d5aa2ba`](https://github.com/GitoxideLabs/gitoxide/commit/d5aa2ba030f57d65021d84efa99c0abc9d61f575))
    - Optimize `Repository::write_blob_stream`: Avoid dup codegen ([`ca8a373`](https://github.com/GitoxideLabs/gitoxide/commit/ca8a373b1a3de44d2bef3e4908d6f5269b6cdd1f))
    - Apply `gix_macros::momo` to `Repository::write_blob` ([`bae928d`](https://github.com/GitoxideLabs/gitoxide/commit/bae928d9668bbb4ba0dadb4605d77fc773362e3f))
    - Optimize `Repository::write_object`: Avoid dup momo ([`32f1c7d`](https://github.com/GitoxideLabs/gitoxide/commit/32f1c7d2bc2e91cb346c8b379dce41293f88b222))
    - Rm unnecessary lifetime annotation in `Repository::commit_as_inner` ([`cf70a2e`](https://github.com/GitoxideLabs/gitoxide/commit/cf70a2e0f08dd323c0713b4e23b21f54668a99a2))
    - Optimize `gix`: de-momo `impl TryInto` by hand ([`b19c140`](https://github.com/GitoxideLabs/gitoxide/commit/b19c140ce3a6e5d9ddf65684361223a2f9fa7e73))
</details>

## 0.53.1 (2023-09-08)

### Bug Fixes

 - <csr-id-902639b9b72ead72b5355e0a1a4da5afd7fed46d/> `interrupt` feature only gates signal-handling, but leaves the `interrupt` module alone.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.53.1 ([`1b1fc25`](https://github.com/GitoxideLabs/gitoxide/commit/1b1fc257d5748c7c41e899bf2d1447ffd9f22d19))
    - `interrupt` feature only gates signal-handling, but leaves the `interrupt` module alone. ([`902639b`](https://github.com/GitoxideLabs/gitoxide/commit/902639b9b72ead72b5355e0a1a4da5afd7fed46d))
</details>

## 0.53.0 (2023-09-08)

<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

This release adds feature toggles which help to reduce compile time. Please [see the library](https://docs.rs/gix/0.53.0/gix/) documentation for all the details.

### New Features

 - <csr-id-2b8d09f785f471aa12fc6793f0ea40c1f8d9ea4a/> remove `log` dependency in favor of `gix-trace`
 - <csr-id-36d34bd7e8cd944c009cb7acbe39c1dc445b4adc/> add `interrupt` feature to reduce dependencies
 - <csr-id-721c37722ca8b12a5f9c060061040c79f9da6aa9/> Allow index access to be toggled with the `index` feature.
 - <csr-id-92dd18154b526b4c5132d770960a36ccf739dec8/> add `excludes` feature to make exclude-checks possible.
 - <csr-id-c4ffde013a62a38a6f63c4a160bc3cdb6aafd65a/> add `mailmap` feature
 - <csr-id-c42064d8ca382513c944f3df5c08e4ff66d5f804/> add `revision` component behind a feature toggle.
 - <csr-id-147528ff647dc74473ef8dd4ceac6fedebc0b15c/> `gix` without connection support includes less code
 - <csr-id-fea044e5d09282a5772c8fe9a534d9ebf7f11bbc/> allow disabling the `blob-diff` capability
   This also removes all diff capabilities.
 - <csr-id-c5ec244979b7e6baf9a8237e4f12cb87809131ae/> improve feature documentation.
   This should make optimizing compile time and performance easier, while
   assuring these options aren't pre-determined by library providers.
 - <csr-id-c79991cde8216271ab854b7574e7d97efd79d07c/> `Clone` for `ThreadSafeRepository`
   It is `Sync` and can easily be passed by reference, but sometimes it's nice
   to be cloning it as well.
 - <csr-id-d22b7fb1304cce3b2aabac42cc58fe7c5911f276/> provide `Repository::find_fetch_remote()` to obtain a remote just like git would.

### Bug Fixes

 - <csr-id-a957478e0f623803bc6358d08b9ffaa2305e24d4/> put `gix-credentials` and `gix-prompt` behind the 'credentials' feature toggle.
   They are also available when using https transports.
 - <csr-id-4971a4837ff5ac6654aa75214bdd2243d4d864a5/> handle submodules whose entry in the index is a file.

### Chore (BREAKING)

 - <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
   It makes proper usage of `Progress` types easier and allows them to be used
   as `dyn` traits as well.

### New Features (BREAKING)

 - <csr-id-58b0e6f860d4d3b5548c18d3eae97141bc6dc377/> Use `stack` abstraction in `Repository::excludes()`.
   This makes it easier to use.
 - <csr-id-24dd870919ba444aa8099c63a78ea120d47ec28e/> use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress`
 - <csr-id-54291fdfc62c7d8a31bc5564713c23eab3865dc5/> Provide a wrapper for `gix_worktree::Stack` for simpler attribute queries.

### Bug Fixes (BREAKING)

 - <csr-id-741b41e6e6c6f283c1632a7de0da44a5e7842817/> remove `regex` feature in favor of `revparse-regex`.
   `revparse-regex` is only used when parsing revspecs that use a special syntax.
   This feature is also enabled by default.
 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 57 commits contributed to the release over the course of 17 calendar days.
 - 17 days passed between releases.
 - 19 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 2 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0 ([`1ff3064`](https://github.com/GitoxideLabs/gitoxide/commit/1ff30641b8724efd6699d8bef5c71d28454e98b9))
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch 'optimizations' ([`6135a5e`](https://github.com/GitoxideLabs/gitoxide/commit/6135a5ea8709646f01da62939a59dd3a9750e007))
    - Adapt to changes in `gix` ([`805b8aa`](https://github.com/GitoxideLabs/gitoxide/commit/805b8aa74b064b7aa08d87094a994bb8c7aae6ed))
    - Remove `log` dependency in favor of `gix-trace` ([`2b8d09f`](https://github.com/GitoxideLabs/gitoxide/commit/2b8d09f785f471aa12fc6793f0ea40c1f8d9ea4a))
    - Add `interrupt` feature to reduce dependencies ([`36d34bd`](https://github.com/GitoxideLabs/gitoxide/commit/36d34bd7e8cd944c009cb7acbe39c1dc445b4adc))
    - Allow index access to be toggled with the `index` feature. ([`721c377`](https://github.com/GitoxideLabs/gitoxide/commit/721c37722ca8b12a5f9c060061040c79f9da6aa9))
    - Put `gix-credentials` and `gix-prompt` behind the 'credentials' feature toggle. ([`a957478`](https://github.com/GitoxideLabs/gitoxide/commit/a957478e0f623803bc6358d08b9ffaa2305e24d4))
    - Add `excludes` feature to make exclude-checks possible. ([`92dd181`](https://github.com/GitoxideLabs/gitoxide/commit/92dd18154b526b4c5132d770960a36ccf739dec8))
    - Use `stack` abstraction in `Repository::excludes()`. ([`58b0e6f`](https://github.com/GitoxideLabs/gitoxide/commit/58b0e6f860d4d3b5548c18d3eae97141bc6dc377))
    - Add `mailmap` feature ([`c4ffde0`](https://github.com/GitoxideLabs/gitoxide/commit/c4ffde013a62a38a6f63c4a160bc3cdb6aafd65a))
    - Simplify test-suite ([`799a515`](https://github.com/GitoxideLabs/gitoxide/commit/799a5152c7ca444a8240a022e049c14b0b61d22d))
    - Remove `regex` feature in favor of `revparse-regex`. ([`741b41e`](https://github.com/GitoxideLabs/gitoxide/commit/741b41e6e6c6f283c1632a7de0da44a5e7842817))
    - Add `revision` component behind a feature toggle. ([`c42064d`](https://github.com/GitoxideLabs/gitoxide/commit/c42064d8ca382513c944f3df5c08e4ff66d5f804))
    - `gix` without connection support includes less code ([`147528f`](https://github.com/GitoxideLabs/gitoxide/commit/147528ff647dc74473ef8dd4ceac6fedebc0b15c))
    - Allow disabling the `blob-diff` capability ([`fea044e`](https://github.com/GitoxideLabs/gitoxide/commit/fea044e5d09282a5772c8fe9a534d9ebf7f11bbc))
    - Improve feature documentation. ([`c5ec244`](https://github.com/GitoxideLabs/gitoxide/commit/c5ec244979b7e6baf9a8237e4f12cb87809131ae))
    - Merge branch 'feat/gix-momo' ([`a1ed6a1`](https://github.com/GitoxideLabs/gitoxide/commit/a1ed6a1aacae02a167b7ec44e1a47411a2194ff7))
    - Handle submodules whose entry in the index is a file. ([`4971a48`](https://github.com/GitoxideLabs/gitoxide/commit/4971a4837ff5ac6654aa75214bdd2243d4d864a5))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress` ([`24dd870`](https://github.com/GitoxideLabs/gitoxide/commit/24dd870919ba444aa8099c63a78ea120d47ec28e))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Merge branch 'improvements' ([`8a7c2af`](https://github.com/GitoxideLabs/gitoxide/commit/8a7c2af0d302d5acc87ef2d432bd6870017af63e))
    - Provide a wrapper for `gix_worktree::Stack` for simpler attribute queries. ([`54291fd`](https://github.com/GitoxideLabs/gitoxide/commit/54291fdfc62c7d8a31bc5564713c23eab3865dc5))
    - `Clone` for `ThreadSafeRepository` ([`c79991c`](https://github.com/GitoxideLabs/gitoxide/commit/c79991cde8216271ab854b7574e7d97efd79d07c))
    - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
    - Adapt to changes in `gix-submodule` ([`f8471b1`](https://github.com/GitoxideLabs/gitoxide/commit/f8471b11e0d65fdb2617b927a8a207659a161439))
    - Release gix-index v0.23.1 ([`11b9c71`](https://github.com/GitoxideLabs/gitoxide/commit/11b9c71311df978ebb20cca0d765cf249c8eedcf))
    - Release gix-date v0.7.4, gix-index v0.23.0, safety bump 5 crates ([`3be2b1c`](https://github.com/GitoxideLabs/gitoxide/commit/3be2b1ccfe30eeae45711c64b88efc522a2b51b7))
    - Apply `momo` to fn `gix::Remote::save_as_to` ([`875c287`](https://github.com/GitoxideLabs/gitoxide/commit/875c28757e4a91cf314ec59dd1a0dde779698e53))
    - Apply `momo` to fn `gix::revision::Spec::from_bstr` ([`1d90301`](https://github.com/GitoxideLabs/gitoxide/commit/1d9030112b54699db9bd8d1125116e46c4a6f71e))
    - Apply `momo` to mod `config::snapshot::access` ([`25912fe`](https://github.com/GitoxideLabs/gitoxide/commit/25912fe1e5d60765458a2e90c0fa487657b0831c))
    - Apply `momo` to mod `gix::create::into` ([`cd3c289`](https://github.com/GitoxideLabs/gitoxide/commit/cd3c2893b095d38d36f0c969549f0aaadfcef2ee))
    - Rm unnecessary `#[allow(unused_mut)]` put on `momo`ed functions ([`89ae797`](https://github.com/GitoxideLabs/gitoxide/commit/89ae797c7f1f4d26c48ed54c5d8b31f39599f063))
    - Remove unnecessary change in `repository/config/transport.rs` ([`86b8e50`](https://github.com/GitoxideLabs/gitoxide/commit/86b8e50fafa7e5d57989acb9e8b848fd95d271a9))
    - Remove unnecessary `#[allow(clippy::needless_lifetimes)]` ([`e1b9d51`](https://github.com/GitoxideLabs/gitoxide/commit/e1b9d51acd137cdea7680584451702e52aab775f))
    - Dramatically simplify `gix_macros::momo` ([`c72eaa0`](https://github.com/GitoxideLabs/gitoxide/commit/c72eaa05697a3e34adaa3ee90584dce4b5c00120))
    - Manually de-`momo` `Repository::try_find_remote_{without_url_rewrite}` ([`e760225`](https://github.com/GitoxideLabs/gitoxide/commit/e7602257662cd9ddb4cfe41ef26cdf28cc007be7))
    - Merge branch 'fixes' ([`4bfd1cc`](https://github.com/GitoxideLabs/gitoxide/commit/4bfd1cc8f7922a8c4de6b9d078d54b93e78f51ff))
    - Thanks clippy ([`0d6d4ec`](https://github.com/GitoxideLabs/gitoxide/commit/0d6d4ec8030d2e8f4c7a9d6f421d54776c4b67fb))
    - Adapt to changes in `gix-index` and pass skip-hash through for performance.. ([`713cd59`](https://github.com/GitoxideLabs/gitoxide/commit/713cd59f0b1eff6397b80f1e1fceec278532fd59))
    - Use new `gix` method to obtain the fetch remote (instead of implementing it by hand) ([`e2c0912`](https://github.com/GitoxideLabs/gitoxide/commit/e2c0912cfede044431c17ae81ddae02746650ae4))
    - Provide `Repository::find_fetch_remote()` to obtain a remote just like git would. ([`d22b7fb`](https://github.com/GitoxideLabs/gitoxide/commit/d22b7fb1304cce3b2aabac42cc58fe7c5911f276))
    - Fix clippy lints in `gix/src/repository/remote.rs` ([`ff210d8`](https://github.com/GitoxideLabs/gitoxide/commit/ff210d82573cebfdf4edbfb39beaef08979c058f))
    - Apply `momo` to mod `gix::repository` ([`5a50537`](https://github.com/GitoxideLabs/gitoxide/commit/5a505377199730354c2a6b6b7b060184558bb9c4))
    - Apply `momo` to mod `remote::connection::fetch::receive_pack` ([`ea5c2db`](https://github.com/GitoxideLabs/gitoxide/commit/ea5c2dbabe9d3c1eb1ab5f15a578ec9f9c36a5d8))
    - Apply `momo` to `gix::reference` ([`3c205ab`](https://github.com/GitoxideLabs/gitoxide/commit/3c205abbdc0a80090b9f0f5681ce0949497e770f))
    - Apply `momo` to `gix::pathspec` ([`767ec2d`](https://github.com/GitoxideLabs/gitoxide/commit/767ec2dcfd1fadaca93390770604494d03f88ab3))
    - Apply `momo` to mod `gix::open::repository` ([`3ce0144`](https://github.com/GitoxideLabs/gitoxide/commit/3ce014499a86e4e8bb57ffe7caa540792c1c0a47))
    - Apply `momo` to `gix::object::tree` ([`d835526`](https://github.com/GitoxideLabs/gitoxide/commit/d8355267fd64dbcf22a01a11cb29d93e75f0fb4c))
    - Apply `momo` to mod `gix::init` ([`46a9dfe`](https://github.com/GitoxideLabs/gitoxide/commit/46a9dfe12dedc1cbf997ea408d1f1d2c5d673ba5))
    - Apply `momo` to mod `gix::discover` ([`58fbb08`](https://github.com/GitoxideLabs/gitoxide/commit/58fbb08461064d96dd9816e2fb6911cf76b6badc))
    - Thanks clippy ([`5044c3b`](https://github.com/GitoxideLabs/gitoxide/commit/5044c3b87456cf58ebfbbd00f23c9ba671cb290c))
    - Imrpove git2 mapping by using aliases. ([`6194ebe`](https://github.com/GitoxideLabs/gitoxide/commit/6194ebe1fb10dedc22f1937b91df858dffc50db3))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.52.0 (2023-08-22)

### New Features

 - <csr-id-28249bda58b56af340c7d6af883496c3bb2d6804/> add `Worktree::pathspec()` to easily get worktree-scoped pathspec searches.
 - <csr-id-59bb3c4109c4e7f1977cea602293be85b7d14a8a/> add `Submodule` type to represent a declared submodule.
 - <csr-id-a7d0e441b2326520ae467e83e045302792e6bcd0/> `pathspec_search([specs])` to instantiate a search using pathspecs.
   It can be used to for filtering input paths.
   This type also makes filtering index entries easy.
 - <csr-id-77da01456118227a59b654f32c15eeb1e5e19cd9/> make `gix-pathspec` crate available
 - <csr-id-5c13459721eabb9d0746899a2498a104ddbdae59/> add `Commit::signature()` to yield the PGP sigature of a commit, if present.

### Bug Fixes

 - <csr-id-c51c8daee1ab54130ae3ed83ce67d08f01c4881a/> fix incorrect s/git-config/gix-config/
   3a861c8f049f6502d3bcbdac752659aa1aeda46a just blindly replaced any
   occurence of "git-config" or "git_config" with "gix-config"/"gix_config".
   
   There is no such thing as a gix-config file.
   gix-config is a git-config file parser.

### New Features (BREAKING)

 - <csr-id-b1e55d6f9e4c0d78f0cdeb7b85e09c2eb7032ced/> `Repository::prefix()` turns `Option<Result` into `Result<Option`.
   This makes it easier for the caller as they won't have to call transpose anymore.
 - <csr-id-46225c2bc399e6db5a56b522c978e0d1fac163df/> improve `interrupt::init_handler()` to be usable from multiple threads
   Previously it was geared towards applications which would initialize handlers
   only from the main thread.
   
   Now the API supports multiple threads.

### Bug Fixes (BREAKING)

 - <csr-id-430e58cd1efef0044fc36b23de019156a21f947c/> `Repository::prefix()` is now side-effect free and won't error if CWD is outside of working tree dir.
   This makes it more usable, especially in contexts where many repositories are held, possibly with
   changing current working dirs.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 32 commits contributed to the release over the course of 18 calendar days.
 - 19 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0 ([`6c62e74`](https://github.com/GitoxideLabs/gitoxide/commit/6c62e748240ac0980fc23fdf30f8477dea8b9bc3))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Merge branch 'gix-submodule' ([`8f3f358`](https://github.com/GitoxideLabs/gitoxide/commit/8f3f358800f1fe77d7ba7ebd396a90b692d3c0c1))
    - Make sure that submodule hashes aren't attached as the parent repo is the wrong one here. ([`c96f26b`](https://github.com/GitoxideLabs/gitoxide/commit/c96f26b5c13581812753638a24d261c3f75dddcf))
    - Properly isolate environment variable based tests into their own binary ([`c35ddab`](https://github.com/GitoxideLabs/gitoxide/commit/c35ddab41ff6f18ad9cd11df44cfffee91563433))
    - Just fmt ([`0d258f4`](https://github.com/GitoxideLabs/gitoxide/commit/0d258f40afcd848509e2b0c7c264e9f346ed1726))
    - Merge branch 'submodule-in-gix' ([`36f7b78`](https://github.com/GitoxideLabs/gitoxide/commit/36f7b783c67b8a087076a130f5ee9b90b23bc3cc))
    - Adapt to changes in `gix` ([`9fe3052`](https://github.com/GitoxideLabs/gitoxide/commit/9fe305291cf8ba908eaf38235f54abfa1d0ddeed))
    - Add `Worktree::pathspec()` to easily get worktree-scoped pathspec searches. ([`28249bd`](https://github.com/GitoxideLabs/gitoxide/commit/28249bda58b56af340c7d6af883496c3bb2d6804))
    - Add `Submodule` type to represent a declared submodule. ([`59bb3c4`](https://github.com/GitoxideLabs/gitoxide/commit/59bb3c4109c4e7f1977cea602293be85b7d14a8a))
    - Merge branch 'worktree-organization' ([`8d0d8e0`](https://github.com/GitoxideLabs/gitoxide/commit/8d0d8e005d7f11924a6717954d892aae5cec45e7))
    - Adapt to changes in `gix-worktree` ([`e5717e1`](https://github.com/GitoxideLabs/gitoxide/commit/e5717e1d12c49285d31a90b03b7f8e9cbc6c1108))
    - Merge pull request #988 from not-my-profile/fix-gix-config-sub ([`7735047`](https://github.com/GitoxideLabs/gitoxide/commit/7735047198bd7cc5059ca338f5c2147dd273f711))
    - Fix incorrect s/git-config/gix-config/ ([`c51c8da`](https://github.com/GitoxideLabs/gitoxide/commit/c51c8daee1ab54130ae3ed83ce67d08f01c4881a))
    - Merge branch 'submodule-active' ([`a3afaa4`](https://github.com/GitoxideLabs/gitoxide/commit/a3afaa42741616a0f1abeef9b54557e7c2b800cb))
    - Adapt to changes in `gix-url` ([`f8fc662`](https://github.com/GitoxideLabs/gitoxide/commit/f8fc6625d8c22f43e7ab5f1cdf1e8eb9a6ea34de))
    - `pathspec_search([specs])` to instantiate a search using pathspecs. ([`a7d0e44`](https://github.com/GitoxideLabs/gitoxide/commit/a7d0e441b2326520ae467e83e045302792e6bcd0))
    - `Repository::prefix()` is now side-effect free and won't error if CWD is outside of working tree dir. ([`430e58c`](https://github.com/GitoxideLabs/gitoxide/commit/430e58cd1efef0044fc36b23de019156a21f947c))
    - Merge branch 'pathspec-matching' ([`9f4dfe0`](https://github.com/GitoxideLabs/gitoxide/commit/9f4dfe0f0b948280692916b596923959ea2fd9da))
    - `Repository::prefix()` turns `Option<Result` into `Result<Option`. ([`b1e55d6`](https://github.com/GitoxideLabs/gitoxide/commit/b1e55d6f9e4c0d78f0cdeb7b85e09c2eb7032ced))
    - Make `gix-pathspec` crate available ([`77da014`](https://github.com/GitoxideLabs/gitoxide/commit/77da01456118227a59b654f32c15eeb1e5e19cd9))
    - Merge branch 'handlers-mt' ([`f584d76`](https://github.com/GitoxideLabs/gitoxide/commit/f584d7698d93836daef2000fd369034de46037f0))
    - Improve `interrupt::init_handler()` to be usable from multiple threads ([`46225c2`](https://github.com/GitoxideLabs/gitoxide/commit/46225c2bc399e6db5a56b522c978e0d1fac163df))
    - Merge branch 'extract-signatures' ([`b37affe`](https://github.com/GitoxideLabs/gitoxide/commit/b37affefecfb30a94431cd21dae6659004ca6244))
    - Add `Commit::signature()` to yield the PGP sigature of a commit, if present. ([`5c13459`](https://github.com/GitoxideLabs/gitoxide/commit/5c13459721eabb9d0746899a2498a104ddbdae59))
    - Release gix-glob v0.10.2, gix-date v0.7.2, gix-validate v0.8.0, gix-object v0.34.0, gix-ref v0.34.0, gix-config v0.27.0, gix-commitgraph v0.18.2, gix-revwalk v0.5.0, gix-revision v0.19.0, gix-refspec v0.15.0, gix-submodule v0.1.0, safety bump 18 crates ([`4604f83`](https://github.com/GitoxideLabs/gitoxide/commit/4604f83ef238dc07c85aaeae097399b67f3cfd0c))
    - Merge branch 'submodules' ([`b629f8a`](https://github.com/GitoxideLabs/gitoxide/commit/b629f8a774931d58c0a9b124fa75f85807c6c5d1))
    - More idiomatic use of `config.section_by_name()` ([`0a584ee`](https://github.com/GitoxideLabs/gitoxide/commit/0a584eeb5c756ec4b0d54c4fd9ea3cc1497f4ba9))
    - Adjust to changes in `gix-validate` ([`a8bc0de`](https://github.com/GitoxideLabs/gitoxide/commit/a8bc0de6d071be82364434b6e27afecc02f3be51))
    - Merge branch 'dev-on-linux' ([`6b4a303`](https://github.com/GitoxideLabs/gitoxide/commit/6b4a30330fe49fc97daa73f55bf56580cc0597aa))
    - Fix various tests to run properly on linux ([`ef8ccd9`](https://github.com/GitoxideLabs/gitoxide/commit/ef8ccd9d16143d37155d063747c69cade80f162d))
</details>

## 0.51.0 (2023-08-02)

This is mostly a bug-fix release with many improvements for fetching, along with more forgiving commit parsing.

### New Features

 - <csr-id-d9e551b44aa3e84109660328de7637d465d59578/> Add `Reference::follow()` as a way to peel symbolic refs step by step.

### Bug Fixes (BREAKING)

 - <csr-id-74ce8639e88db5107691e9279df2bbfd38d26de3/> handle symbolic ref updates far more gracefully and with more logical consistency.
   Previously, refspecs couldn't be used to update sybolic references locally, particularly because the logic
   to do so correctly isn't trivial and `git` itself also seems to cover only the most common cases.
   
   However, the logic now changed so that remote updates will only be rejected if
   
   * fast-forward rules are violated
* the local ref is currently checked out
* existing refs would not become 'unborn', i.e. point to a reference that doesn't exist and won't be created due to ref-specs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 2 calendar days.
 - 9 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-actor v0.24.2, gix-object v0.33.2, gix-ref v0.33.3, gix-config v0.26.2, gix-prompt v0.5.5, gix-odb v0.50.2, gix-transport v0.34.2, gix-protocol v0.37.0, gix-worktree v0.23.1, gix v0.51.0, safety bump 3 crates ([`231ac1c`](https://github.com/GitoxideLabs/gitoxide/commit/231ac1c6ad5ca9a84dbeb0dee14bfbf2fef1ae1e))
    - Prepare additional changelogs ([`db63815`](https://github.com/GitoxideLabs/gitoxide/commit/db6381522395a0de047118e81df5cd3cbeb862b9))
    - Prepare changelogs ([`e4d2890`](https://github.com/GitoxideLabs/gitoxide/commit/e4d2890a85bf60e9cdb4016dddfab3c4dccbe75e))
    - Merge branch 'fixes-and-improvements' ([`f8b1f55`](https://github.com/GitoxideLabs/gitoxide/commit/f8b1f553371f25b1bea6bce7cbb2ff1f01194856))
    - Handle symbolic ref updates far more gracefully and with more logical consistency. ([`74ce863`](https://github.com/GitoxideLabs/gitoxide/commit/74ce8639e88db5107691e9279df2bbfd38d26de3))
    - Adapt to changes in `gix-protocol` ([`df81076`](https://github.com/GitoxideLabs/gitoxide/commit/df810766dfeaaad7474339358a3d844b2c3368cd))
    - Add `Reference::follow()` as a way to peel symbolic refs step by step. ([`d9e551b`](https://github.com/GitoxideLabs/gitoxide/commit/d9e551b44aa3e84109660328de7637d465d59578))
</details>

## 0.50.1 (2023-07-24)

### Bug Fixes

 - <csr-id-145f8658a32c46db1f54d3098cf9371fe6eeec5e/> `Tree::lookup_entry(_by_path)()` now actually works
   Previously it was lacking a test and that showed.

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
    - Release gix-archive v0.2.1, gix-ref v0.33.2, gix-pack v0.40.2, gix v0.50.1 ([`13883e5`](https://github.com/GitoxideLabs/gitoxide/commit/13883e5528385f892ee402e911298121e0c297c0))
    - Prepare changelogs ([`735c206`](https://github.com/GitoxideLabs/gitoxide/commit/735c2062625aaeffbdbca3c1395dbcf075661e3a))
    - `Tree::lookup_entry(_by_path)()` now actually works ([`145f865`](https://github.com/GitoxideLabs/gitoxide/commit/145f8658a32c46db1f54d3098cf9371fe6eeec5e))
</details>

## 0.50.0 (2023-07-22)

### New Features

 - <csr-id-caa8fb9502906fa47546c26bbeb3c546664ad944/> `TreeEntryRefExt` and `TreeEntryExt` to be able to easily attach a repo to it.
   Also, add `detach()` to types that were missing it.
 - <csr-id-62cacd4b7a9fc0c0e4c5049f6d0aa7011c8ef923/> `Tree::find_entry()` to easily find an entry in a tree's entries.
 - <csr-id-c4a1fb1ba461c28ac3ea2482adf5f75721d14706/> add `Repository::archive()` as extra
   It implements a high-level interface to achieve `git archive` like functionality.
 - <csr-id-4ee285741e6e1cde3a967980fbf48bab20ddbf68/> optionally make `gix-workspace-stream` available via `Repository::worktree_stream()`
   That way it's easy to obtain a representation of the worktree
   in a fully streaming fashion, which is also the basis for
   `archive`-like functionality.

### New Features (BREAKING)

 - <csr-id-d5e4ee0e6e26ff3feeed1f5aee5bdd0cdc03d1f8/> unify API between `object::tree::Entry` and `object::tree::EntryRef<'_>`

### Bug Fixes (BREAKING)

 - <csr-id-8cad009eafe8e1054a715dc99bb9a884325d5ea5/> `Tree::lookup_entry(_by_path))()` are not mutating anymore; add `Tree::peel_to_entry()` and `peel_to_entry_by_path()`
   The previous implementation was a crutch that could now be circumvented.
   
   The new methods allow to reuse a buffer in case the object isn't used or needed further,
   possibly saving allocations.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 1 calendar day.
 - 3 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-config v0.26.1, gix v0.50.0 ([`d34a4ea`](https://github.com/GitoxideLabs/gitoxide/commit/d34a4ea27cd83b916c84cf15e1c05da35576db5e))
    - Release gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0 ([`0062971`](https://github.com/GitoxideLabs/gitoxide/commit/00629710dffeb10fda340665530353703cf5d129))
    - Release gix-tempfile v7.0.2, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0 ([`107a64e`](https://github.com/GitoxideLabs/gitoxide/commit/107a64e734580ad9e2c4142db96394529d8072df))
    - Release gix-features v0.32.1, gix-actor v0.24.1, gix-validate v0.7.7, gix-object v0.33.1, gix-path v0.8.4, gix-glob v0.10.1, gix-quote v0.4.6, gix-attributes v0.16.0, gix-command v0.2.8, gix-packetline-blocking v0.16.4, gix-filter v0.2.0, gix-fs v0.4.1, gix-chunk v0.4.4, gix-commitgraph v0.18.1, gix-hashtable v0.2.4, gix-revwalk v0.4.1, gix-traverse v0.30.1, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.5, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.1, gix-sec v0.8.4, gix-prompt v0.5.4, gix-url v0.21.1, gix-credentials v0.17.1, gix-diff v0.33.1, gix-discover v0.22.1, gix-ignore v0.5.1, gix-bitmap v0.2.6, gix-index v0.21.1, gix-mailmap v0.16.1, gix-negotiate v0.5.1, gix-pack v0.40.1, gix-odb v0.50.1, gix-packetline v0.16.4, gix-transport v0.34.1, gix-protocol v0.36.1, gix-revision v0.18.1, gix-refspec v0.14.1, gix-worktree v0.23.0, gix v0.50.0, safety bump 5 crates ([`16295b5`](https://github.com/GitoxideLabs/gitoxide/commit/16295b58e2581d2e8b8b762816f52baabe871c75))
    - Prepare more changelogs ([`c4cc5f2`](https://github.com/GitoxideLabs/gitoxide/commit/c4cc5f261d29f712a101033a18293a97a9d4ae85))
    - Release gix-date v0.7.1, gix-hash v0.11.4, gix-trace v0.1.3, gix-features v0.32.0, gix-actor v0.24.0, gix-validate v0.7.7, gix-object v0.33.0, gix-path v0.8.4, gix-glob v0.10.0, gix-quote v0.4.6, gix-attributes v0.15.0, gix-command v0.2.7, gix-packetline-blocking v0.16.3, gix-filter v0.1.0, gix-fs v0.4.0, gix-chunk v0.4.4, gix-commitgraph v0.18.0, gix-hashtable v0.2.4, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-worktree-stream v0.2.0, gix-archive v0.2.0, gix-config-value v0.12.4, gix-tempfile v7.0.1, gix-utils v0.1.5, gix-lock v7.0.2, gix-ref v0.33.0, gix-sec v0.8.4, gix-prompt v0.5.3, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-ignore v0.5.0, gix-bitmap v0.2.6, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-packetline v0.16.4, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.1 ([`5cb3589`](https://github.com/GitoxideLabs/gitoxide/commit/5cb3589b74fc5376e02cbfe151e71344e1c417fe))
    - Update changelogs prior to release ([`2fc66b5`](https://github.com/GitoxideLabs/gitoxide/commit/2fc66b55097ed494b72d1af939ba5561f71fde97))
    - Merge branch 'improvements-for-crates-index' ([`3f914e8`](https://github.com/GitoxideLabs/gitoxide/commit/3f914e8840afc59441c3c463bdc89c53136d583e))
    - `TreeEntryRefExt` and `TreeEntryExt` to be able to easily attach a repo to it. ([`caa8fb9`](https://github.com/GitoxideLabs/gitoxide/commit/caa8fb9502906fa47546c26bbeb3c546664ad944))
    - `Tree::find_entry()` to easily find an entry in a tree's entries. ([`62cacd4`](https://github.com/GitoxideLabs/gitoxide/commit/62cacd4b7a9fc0c0e4c5049f6d0aa7011c8ef923))
    - `Tree::lookup_entry(_by_path))()` are not mutating anymore; add `Tree::peel_to_entry()` and `peel_to_entry_by_path()` ([`8cad009`](https://github.com/GitoxideLabs/gitoxide/commit/8cad009eafe8e1054a715dc99bb9a884325d5ea5))
    - Unify API between `object::tree::Entry` and `object::tree::EntryRef<'_>` ([`d5e4ee0`](https://github.com/GitoxideLabs/gitoxide/commit/d5e4ee0e6e26ff3feeed1f5aee5bdd0cdc03d1f8))
    - J fmt ([`57cab40`](https://github.com/GitoxideLabs/gitoxide/commit/57cab40f5cb437cc5b0a2c1fc5ae0f91f98bbbcc))
    - Merge branch 'gix-archive' ([`1dda48b`](https://github.com/GitoxideLabs/gitoxide/commit/1dda48ba2fccb93ebac00fe3460e923af43c86ce))
    - Change archive implementation to require the seek bound. ([`61aed0e`](https://github.com/GitoxideLabs/gitoxide/commit/61aed0e955974f65f4fea042cbae68ea8a2cc2f5))
    - Add `Repository::archive()` as extra ([`c4a1fb1`](https://github.com/GitoxideLabs/gitoxide/commit/c4a1fb1ba461c28ac3ea2482adf5f75721d14706))
    - Optionally make `gix-workspace-stream` available via `Repository::worktree_stream()` ([`4ee2857`](https://github.com/GitoxideLabs/gitoxide/commit/4ee285741e6e1cde3a967980fbf48bab20ddbf68))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
</details>

## 0.49.1 (2023-07-19)

A maintenance release without user-facing changes.

### Bug Fixes (BREAKING)

 - <csr-id-8cad009eafe8e1054a715dc99bb9a884325d5ea5/> `Tree::lookup_entry(_by_path))()` are not mutating anymore; add `Tree::peel_to_entry()` and `peel_to_entry_by_path()`
   The previous implementation was a crutch that could now be circumvented.
   
   The new methods allow to reuse a buffer in case the object isn't used or needed further,
   possibly saving allocations.

### New Features (BREAKING)

 - <csr-id-d5e4ee0e6e26ff3feeed1f5aee5bdd0cdc03d1f8/> unify API between `object::tree::Entry` and `object::tree::EntryRef<'_>`

### New Features

 - <csr-id-caa8fb9502906fa47546c26bbeb3c546664ad944/> `TreeEntryRefExt` and `TreeEntryExt` to be able to easily attach a repo to it.
   Also, add `detach()` to types that were missing it.
 - <csr-id-62cacd4b7a9fc0c0e4c5049f6d0aa7011c8ef923/> `Tree::find_entry()` to easily find an entry in a tree's entries.
 - <csr-id-c4a1fb1ba461c28ac3ea2482adf5f75721d14706/> add `Repository::archive()` as extra
   It implements a high-level interface to achieve `git archive` like functionality.
 - <csr-id-4ee285741e6e1cde3a967980fbf48bab20ddbf68/> optionally make `gix-workspace-stream` available via `Repository::worktree_stream()`
   That way it's easy to obtain a representation of the worktree
   in a fully streaming fashion, which is also the basis for
   `archive`-like functionality.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-prompt v0.5.3, gix v0.49.1, cargo-smart-release v0.20.0 ([`f069852`](https://github.com/GitoxideLabs/gitoxide/commit/f0698522940c9ba4d45db5a44dce9f21ca29cb4e))
    - Prepare changelogs prior to release ([`849f508`](https://github.com/GitoxideLabs/gitoxide/commit/849f5081313c4a44bdaef6848758d0d9a5d1598b))
    - Merge branch 'smart-release-stability' ([`8629f56`](https://github.com/GitoxideLabs/gitoxide/commit/8629f569cd5917b6c0c3fd928fde021e7976ee85))
    - Update git2 API mapping and be clear what stability means as well. ([`64cd396`](https://github.com/GitoxideLabs/gitoxide/commit/64cd396ab05959e1f843f7ccd53ac5d4585584ad))
</details>

## 0.49.0 (2023-07-19)

<csr-id-c548780e6ea49453ecdb45b11bf4c5781b105e6b/>

### New Features

 - <csr-id-980c2ba591dce7fc787c418aed85078c19e2d6d4/> Make `EntryMode` available from `gix::object::tree`.
   Previously one had to go through `gix::objs::tree` which wasn't symmetric
   with `gix::object::Kind`.
 - <csr-id-d4a8f8cf6d8b059978719ea314fc8a4bfe26c60d/> Add `Id::header()` and `Id::try_header()` as syblings to `::object()` and `::try_object()`.
   With the new header related functions one can obtain information about an object more quickly.
 - <csr-id-b73435b3bf334d5be2931c2ea6a597a9dd51b783/> `Repository::header()` and `::try_header()` to learn about objects, quickly
   Accessing just the headers of an object is much faster than accessing the entire
   object. Previously, this method was only available on the `objects` field, now it's
   available through `Repository` directly.
 - <csr-id-c05eb2204620a5ff5e04b766009c873a14ae0f9e/> top-level examples that represent fully-fledged command-line applications.
   Please note that these are just examples, which aren't necessarily
   production ready in terms of quality or performance.
 - <csr-id-8cc106aa430d39ac9967dcfb3d293725fc76cb79/> checkouts when cloning now respect attributes and use filters.
 - <csr-id-8993b777cd0331e7260d7d7d1f820afc79a34b19/> add `Repository::filter_pipeline()` to obtain a primitive to handle data conversions.
   It's fully configured as git would, and can be used to convert data from git or to git.

### Bug Fixes

 - <csr-id-47ca8465e04bdd13fe2ebfc6f012f8191e3f7896/> properly re-initialize object caches after their configuration changes.

### Refactor (BREAKING)

 - <csr-id-c548780e6ea49453ecdb45b11bf4c5781b105e6b/> move error structs into `repository` module where appropriate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release over the course of 11 calendar days.
 - 19 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#925](https://github.com/GitoxideLabs/gitoxide/issues/925)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#925](https://github.com/GitoxideLabs/gitoxide/issues/925)**
    - Remove all copies of repo-initialization files and rework them to be our own. ([`5ac2269`](https://github.com/GitoxideLabs/gitoxide/commit/5ac22699936dbc5c09c5eefd28b75d48271b286b))
 * **Uncategorized**
    - Release gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`4aca8c2`](https://github.com/GitoxideLabs/gitoxide/commit/4aca8c2ae2ec588fb65ec4faa0c07c19d219569f))
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
    - Just fmt ([`a063c62`](https://github.com/GitoxideLabs/gitoxide/commit/a063c62e3a30006d837b267e2ce74e70e48b4fb6))
    - Merge branch 'adjustments-for-crates-index' ([`b82868d`](https://github.com/GitoxideLabs/gitoxide/commit/b82868d5688d8d4849c47ed3d209a96ee59e69b3))
    - Make `EntryMode` available from `gix::object::tree`. ([`980c2ba`](https://github.com/GitoxideLabs/gitoxide/commit/980c2ba591dce7fc787c418aed85078c19e2d6d4))
    - Add `Id::header()` and `Id::try_header()` as syblings to `::object()` and `::try_object()`. ([`d4a8f8c`](https://github.com/GitoxideLabs/gitoxide/commit/d4a8f8cf6d8b059978719ea314fc8a4bfe26c60d))
    - `Repository::header()` and `::try_header()` to learn about objects, quickly ([`b73435b`](https://github.com/GitoxideLabs/gitoxide/commit/b73435b3bf334d5be2931c2ea6a597a9dd51b783))
    - Properly re-initialize object caches after their configuration changes. ([`47ca846`](https://github.com/GitoxideLabs/gitoxide/commit/47ca8465e04bdd13fe2ebfc6f012f8191e3f7896))
    - Top-level examples that represent fully-fledged command-line applications. ([`c05eb22`](https://github.com/GitoxideLabs/gitoxide/commit/c05eb2204620a5ff5e04b766009c873a14ae0f9e))
    - Cargo fmt ([`6121b8f`](https://github.com/GitoxideLabs/gitoxide/commit/6121b8f6a7da7f263c6e066155f053a1d7c81477))
    - `git log` example include empty parents and paths ([`bd59bbe`](https://github.com/GitoxideLabs/gitoxide/commit/bd59bbebddf804a4dd0872127dcc31b5c3b29c2f))
    - `git log` example now accepts multiple paths. ([`0df9f70`](https://github.com/GitoxideLabs/gitoxide/commit/0df9f707987c8001c4ca81faf69033c679a75fd5))
    - `git log` example filter for min/max parents ([`01e9c29`](https://github.com/GitoxideLabs/gitoxide/commit/01e9c29bf7106b30d8e3e8c71b37eff77bcc38b5))
    - `git log` example iterator now properly lazy ([`8a6f1e8`](https://github.com/GitoxideLabs/gitoxide/commit/8a6f1e89fa4d736a2c902be55413887e14885957))
    - `git log` example now shows merge parents ([`5cbb6a7`](https://github.com/GitoxideLabs/gitoxide/commit/5cbb6a72c34d926a2782569d8370e54d4c63ab34))
    - A `git log` example ([`03b3423`](https://github.com/GitoxideLabs/gitoxide/commit/03b342306c5effac5e8aa92a349385e59785c0b7))
    - A `git ls-tree` example ([`6f4b431`](https://github.com/GitoxideLabs/gitoxide/commit/6f4b43101f7b46e38c2f61c2f859347085d8214f))
    - Thanks clippy ([`3ef32af`](https://github.com/GitoxideLabs/gitoxide/commit/3ef32af9bf477cbc60d24da8bb3f15d20976e9e0))
    - Merge branch 'unique-templates' ([`cbb0db8`](https://github.com/GitoxideLabs/gitoxide/commit/cbb0db80ccc5c29c92f7abd8af2a03c67d86fc2b))
    - Adapt journey tests to changes in init templates ([`6297d22`](https://github.com/GitoxideLabs/gitoxide/commit/6297d2201abb97ec999986a7a19b9ddb02114e24))
    - Merge branch 'integrate-filtering' ([`b19a56d`](https://github.com/GitoxideLabs/gitoxide/commit/b19a56dcfa9bea86332a84aa4e8fad445e7d1724))
    - Checkouts when cloning now respect attributes and use filters. ([`8cc106a`](https://github.com/GitoxideLabs/gitoxide/commit/8cc106aa430d39ac9967dcfb3d293725fc76cb79))
    - Add `Repository::filter_pipeline()` to obtain a primitive to handle data conversions. ([`8993b77`](https://github.com/GitoxideLabs/gitoxide/commit/8993b777cd0331e7260d7d7d1f820afc79a34b19))
    - Move error structs into `repository` module where appropriate. ([`c548780`](https://github.com/GitoxideLabs/gitoxide/commit/c548780e6ea49453ecdb45b11bf4c5781b105e6b))
    - Add keys required to deal with worktree conversions and filters. ([`3fbd7b0`](https://github.com/GitoxideLabs/gitoxide/commit/3fbd7b0c864cf2f1a38ae24e85d47b0b26b271a7))
</details>

## 0.48.0 (2023-06-29)

<csr-id-fb63f3f07f0f9545be5942bcb66b06040fbc7fe9/>
<csr-id-3c8e3c1e88d36657d4e6eeaf0819be7fd9341ae1/>

The main feature of this release is support dates prior to the UNIX epoch. Note that this is a feature that isn't supported by `git`, but only by `libgit2`.

### Bug Fixes

 - <csr-id-9cfc4aa318bc44c9e4310db7d3764b015472e1af/> use type for time consistently.
   This will allow it to be changed more easily later.

### Other

 - <csr-id-fb63f3f07f0f9545be5942bcb66b06040fbc7fe9/> Add incomplete mapping of typical `git2` functions and their counterpart in `gix`.
   That way the ground-work is laid for making the usage of `gix` easier for those who used
   `git2` before.
 - <csr-id-3c8e3c1e88d36657d4e6eeaf0819be7fd9341ae1/> make clear what can happen if rewrite-tracking isn't disabled if it is not desired.
   Triggered by this `onefetch` PR: https://github.com/o2sh/onefetch/pull/1093

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 6 calendar days.
 - 6 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.33.1, gix v0.48.0 ([`f27ca12`](https://github.com/GitoxideLabs/gitoxide/commit/f27ca128c5f109ad02e4e1a12dc14e93b07bbfcf))
    - Release gix-lock v7.0.1, gix v0.48.0 ([`5ce81ef`](https://github.com/GitoxideLabs/gitoxide/commit/5ce81ef16210f0b0b72dfd5710a064ccda96ac1c))
    - Release gix-glob v0.9.1, gix-attributes v0.14.1, gix-config-value v0.12.3, gix-ref v0.32.1, gix-sec v0.8.3, gix-config v0.25.1, gix-url v0.20.1, gix-credentials v0.16.1, gix-discover v0.21.1, gix-ignore v0.4.1, gix-pack v0.39.1, gix-odb v0.49.1, gix-worktree v0.21.1, gix v0.48.0 ([`69c6a36`](https://github.com/GitoxideLabs/gitoxide/commit/69c6a36ba14cbef129deebda9fd8870005fefa17))
    - Release gix-features v0.31.1, gix-path v0.8.3, gix v0.48.0 ([`9ca3464`](https://github.com/GitoxideLabs/gitoxide/commit/9ca346462806671fbc49643a87cea25ab0da3be8))
    - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
    - Prepare changelogs prior to release ([`00f96fb`](https://github.com/GitoxideLabs/gitoxide/commit/00f96fb3110a8f81a1bd0d74c757c15b8773c6f6))
    - Merge branch 'i64-times' ([`b407461`](https://github.com/GitoxideLabs/gitoxide/commit/b407461d8991db67a5bdb2ab13f518f78a85ed40))
    - Add incomplete mapping of typical `git2` functions and their counterpart in `gix`. ([`fb63f3f`](https://github.com/GitoxideLabs/gitoxide/commit/fb63f3f07f0f9545be5942bcb66b06040fbc7fe9))
    - Adapt to changes in `gix-date` ([`fba45c6`](https://github.com/GitoxideLabs/gitoxide/commit/fba45c68d57d5f73070a6949556a04187d42e427))
    - Use type for time consistently. ([`9cfc4aa`](https://github.com/GitoxideLabs/gitoxide/commit/9cfc4aa318bc44c9e4310db7d3764b015472e1af))
    - Add a test to see what happens if negative dates are used in commits ([`57a5cd1`](https://github.com/GitoxideLabs/gitoxide/commit/57a5cd1ca2f8153568c366cd1709be7d4ebec972))
    - Make clear what can happen if rewrite-tracking isn't disabled if it is not desired. ([`3c8e3c1`](https://github.com/GitoxideLabs/gitoxide/commit/3c8e3c1e88d36657d4e6eeaf0819be7fd9341ae1))
    - More tracing information when updating refs ([`6906e0d`](https://github.com/GitoxideLabs/gitoxide/commit/6906e0d717bb04efb0744eb20afb4c53ebe360c9))
    - Add more details for negotation phases ([`8341d08`](https://github.com/GitoxideLabs/gitoxide/commit/8341d0882697b02275ec2bb0f05229ba3b60df3b))
    - Add a span for each negotiation round ([`ec73479`](https://github.com/GitoxideLabs/gitoxide/commit/ec7347971163e838e0fdedb0bc6bc88d32d30d8e))
</details>

## 0.47.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

 - <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### New Features

 - <csr-id-3cffa268460eb2d41bd6a30d45778b88db4ec602/> provide basic `tracing` spans for common operations.
   This is just the beginning and more crates will integrate with it over time.
 - <csr-id-47c7b0dff9ca82e7c0c60b1dcf1d120f7bec7955/> expose `hashtable` in root for access to optimized-for-object-ids sets and maps.

### Bug Fixes

 - <csr-id-67c06d99dc8387ce566c2fe436c28cdaa041bf07/> make sure empty packs in shallow clones are working as well.
 - <csr-id-db69e31d8bb73aba886a9a323bfa154a23deacf8/> no-want detection for negotiation phase is now consistent.
   It being inconsistent was a reason for 'failing to parse server response' which
   was empty as we didn't provide any wants to the server, but didn't detect that case
   in the initial negotiation-preparation phase.
   
   Turns out we didn't detect it as our special handling of implicit tags was not done
   in the negotiation-preparation phase.
   
   The fix consists of unifying the filtering phase to all places that needed, so
   the preparation phase outcome is now consistent with what would have come later.

### New Features (BREAKING)

 - <csr-id-682def03ce6ec93d7bcd2f79eedea52021b77f03/> provide `fetch::outcome::Negotiate` for details on what happened during negotiation.
   We also remove the `negotiation_rounds` field in favor of a far more detailed `fetch::outcome::Negotiate` struct.
 - <csr-id-574e0f4786719bd56da2fa218772f879fda282bf/> respect the `core.commitGraph` option.
   Previously, we would always use the commitgraph when available, but now we only do so
   if the `core.commitGraph` option is set.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release over the course of 11 calendar days.
 - 12 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.6.0, gix-hash v0.11.3, gix-trace v0.1.1, gix-features v0.31.0, gix-actor v0.22.0, gix-path v0.8.2, gix-glob v0.9.0, gix-quote v0.4.5, gix-attributes v0.14.0, gix-chunk v0.4.3, gix-commitgraph v0.17.0, gix-config-value v0.12.2, gix-fs v0.3.0, gix-tempfile v7.0.0, gix-utils v0.1.3, gix-lock v7.0.0, gix-validate v0.7.6, gix-object v0.31.0, gix-ref v0.31.0, gix-sec v0.8.2, gix-config v0.24.0, gix-command v0.2.6, gix-prompt v0.5.2, gix-url v0.20.0, gix-credentials v0.16.0, gix-diff v0.31.0, gix-discover v0.20.0, gix-hashtable v0.2.2, gix-ignore v0.4.0, gix-bitmap v0.2.5, gix-revwalk v0.2.0, gix-traverse v0.28.0, gix-index v0.19.0, gix-mailmap v0.14.0, gix-negotiate v0.3.0, gix-pack v0.38.0, gix-odb v0.48.0, gix-packetline v0.16.3, gix-transport v0.33.0, gix-protocol v0.34.0, gix-revision v0.16.0, gix-refspec v0.12.0, gix-worktree v0.20.0, gix v0.47.0, gitoxide-core v0.29.0, gitoxide v0.27.0, safety bump 30 crates ([`ea9f942`](https://github.com/GitoxideLabs/gitoxide/commit/ea9f9424e777f10da0e33bb9ffbbefd01c4c5a74))
    - Prepare changelogs prior to release ([`18b0a37`](https://github.com/GitoxideLabs/gitoxide/commit/18b0a371941aa2d4d62512437d5daa351ba99ffd))
    - Add a span for another potentially expensive portion of the negotiation ([`f2e7ec4`](https://github.com/GitoxideLabs/gitoxide/commit/f2e7ec4d80299933327ebdc932a758d5d9c7f218))
    - `just fmt` ([`871dd0b`](https://github.com/GitoxideLabs/gitoxide/commit/871dd0b977caf17159092a4739ba5408403cdb2c))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Provide basic `tracing` spans for common operations. ([`3cffa26`](https://github.com/GitoxideLabs/gitoxide/commit/3cffa268460eb2d41bd6a30d45778b88db4ec602))
    - Merge branch 'gix-revision-graph' ([`036e60a`](https://github.com/GitoxideLabs/gitoxide/commit/036e60a3ad39ba9b018c0b56454f12fad455c7bb))
    - Expose `hashtable` in root for access to optimized-for-object-ids sets and maps. ([`47c7b0d`](https://github.com/GitoxideLabs/gitoxide/commit/47c7b0dff9ca82e7c0c60b1dcf1d120f7bec7955))
    - Provide `fetch::outcome::Negotiate` for details on what happened during negotiation. ([`682def0`](https://github.com/GitoxideLabs/gitoxide/commit/682def03ce6ec93d7bcd2f79eedea52021b77f03))
    - Merge branch 'fix-no-want-detection' ([`71efcbb`](https://github.com/GitoxideLabs/gitoxide/commit/71efcbba112376b4acaf37d662cdb38d369462be))
    - Make sure empty packs in shallow clones are working as well. ([`67c06d9`](https://github.com/GitoxideLabs/gitoxide/commit/67c06d99dc8387ce566c2fe436c28cdaa041bf07))
    - No-want detection for negotiation phase is now consistent. ([`db69e31`](https://github.com/GitoxideLabs/gitoxide/commit/db69e31d8bb73aba886a9a323bfa154a23deacf8))
    - Merge branch 'help-874-redundant-closures' ([`fe59956`](https://github.com/GitoxideLabs/gitoxide/commit/fe59956ad667303a923d7cfd9ffd72283df41d78))
    - Add `clippy::redundant-closure-for-method-calls` lint ([`bcad5c2`](https://github.com/GitoxideLabs/gitoxide/commit/bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d))
    - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
    - Respect the `core.commitGraph` option. ([`574e0f4`](https://github.com/GitoxideLabs/gitoxide/commit/574e0f4786719bd56da2fa218772f879fda282bf))
    - Adapt to changes in `gix-revision`/`gix-revwalk` ([`1fdaf71`](https://github.com/GitoxideLabs/gitoxide/commit/1fdaf71d32eb60ad056376d27837ff37d4d314cd))
    - Adapt to changes in `gix-protocol` ([`b785e81`](https://github.com/GitoxideLabs/gitoxide/commit/b785e811232d645ad72bfb87459efbd80cb0a399))
    - Adapt to changes in `gix-traverse` ([`b447f47`](https://github.com/GitoxideLabs/gitoxide/commit/b447f478b8a5a28c659cef178b2a06b666f89ec3))
    - Adapt to changes in `gix-actor` ([`4a80e86`](https://github.com/GitoxideLabs/gitoxide/commit/4a80e868f9530896616e649838e9be64b6d10036))
    - Adapt to changes in `gix-date` ([`d575336`](https://github.com/GitoxideLabs/gitoxide/commit/d575336c26e6026e463cd06d88266bb2bdd3e162))
</details>

## 0.46.0 (2023-06-10)

<csr-id-f0ddc3b9c5a34b7930b965dfb1438f95279a8bde/>

### New Features (BREAKING)

 - <csr-id-7e9f202746d4376332f9779c6e4bd67933d618c7/> `revision::Walk` yields `revision::Info` structs instead of `Id`s.
   This enables re-use of information that was already obtained, like the parents of
   a commit and possibly its commit-time.

### Changed (BREAKING)

 - <csr-id-068603a4b7a52eaa397d61212e7aec5a0195ac29/> rename `Repository::commit_graph()` to `::revision_graph()`.
   THat's a better fix given its locaion in `gix-revision`, while differentiating
   it further from the lower-level `commit-graph`.
   
   Also rename `Repository::commit_cache()` to `::commit_graph()` now that the name is free.

### Other

 - <csr-id-f0ddc3b9c5a34b7930b965dfb1438f95279a8bde/> `gix::revision::walk::Platform` now informas about the commitgraph.
   In short, one should use the `Graph` to obtain the tools necessary for potentially
   accelerated, custom commit walks.

### New Features

 - <csr-id-cc72e497868636b0e7c943f675bda82860c2b53e/> make it possible to use `config::tree::Key` to more conveniently set values via `config::SnapshotMut::set()`
 - <csr-id-b2b88dc11d1b745e787596e9b94122238ccaf34c/> use the `commitgraph` if possible and allow its usage to be controlled via `revision::walk::Platform::use_commit_graph(toggle)`.
   The commitgraph is a data structure to greatly accelerate commit walks. It is now supported and
   used by default, but can be deactivated if desired.
   
   Further, add `Repository::commit_cache()` for direct access to just the commit-graph datastructure,
   without the extras provided by `gix_revision::Graph`.
 - <csr-id-5d320121533d60fc594792da7838a4f9c661dea0/> add `Repository::index_or_load_from_head()`.
   That way it's possible to either open the existing worktree index, or create one
   in-memory by turning our HEAD tree into an index on the fly.
 - <csr-id-2a698fab7323fd5befd14926bcb9cebf09afc312/> make it possible to use `config::tree::Key` to more conveniently set values via `config::SnapshotMut::set()`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-attributes v0.13.1, gix-diff v0.30.1, gix-revwalk v0.1.0, gix-traverse v0.27.0, gix-index v0.18.0, gix-revision v0.15.2, gix-negotiate v0.2.1, gix-pack v0.37.0, gix-odb v0.47.0, gix-protocol v0.33.2, gix-worktree v0.19.0, gix v0.46.0, safety bump 7 crates ([`2560a2c`](https://github.com/GitoxideLabs/gitoxide/commit/2560a2cc3e1d8c60cd812e15696fa4761d036e19))
    - Prepare changelogs prior to release ([`298f3d7`](https://github.com/GitoxideLabs/gitoxide/commit/298f3d7359c5b183314d8c584e45dcdd559d88b3))
    - Improve tests related to the handling of shallow repos ([`d50bfa9`](https://github.com/GitoxideLabs/gitoxide/commit/d50bfa97f528141e0183558f21a364d969911ef4))
    - Merge branch 'walk-with-commitgraph' ([`fdee9a2`](https://github.com/GitoxideLabs/gitoxide/commit/fdee9a22873a13ae644d3dc92f8fe93f8f0266c0))
    - Adapt to changes in `gix` ([`20f73c8`](https://github.com/GitoxideLabs/gitoxide/commit/20f73c8224ead1b423a1b6331c9cab65f769d46a))
    - `revision::Walk` yields `revision::Info` structs instead of `Id`s. ([`7e9f202`](https://github.com/GitoxideLabs/gitoxide/commit/7e9f202746d4376332f9779c6e4bd67933d618c7))
    - Rename `Repository::commit_graph()` to `::revision_graph()`. ([`068603a`](https://github.com/GitoxideLabs/gitoxide/commit/068603a4b7a52eaa397d61212e7aec5a0195ac29))
    - Use the `commitgraph` if possible and allow its usage to be controlled via `revision::walk::Platform::use_commit_graph(toggle)`. ([`b2b88dc`](https://github.com/GitoxideLabs/gitoxide/commit/b2b88dc11d1b745e787596e9b94122238ccaf34c))
    - Adapt to changes in `gix-traverse` ([`1f682fd`](https://github.com/GitoxideLabs/gitoxide/commit/1f682fd991b9b76a8d37e6852567ff239c0ac0db))
    - Adapt to changes in `gix-revwalk` ([`f7d95d1`](https://github.com/GitoxideLabs/gitoxide/commit/f7d95d189af1422a7ba48db1857452e32e1d9db9))
    - Add `Repository::index_or_load_from_head()`. ([`5d32012`](https://github.com/GitoxideLabs/gitoxide/commit/5d320121533d60fc594792da7838a4f9c661dea0))
    - `gix::revision::walk::Platform` now informas about the commitgraph. ([`f0ddc3b`](https://github.com/GitoxideLabs/gitoxide/commit/f0ddc3b9c5a34b7930b965dfb1438f95279a8bde))
    - Update changelog with information for the `gix` CLI. ([`4e081f2`](https://github.com/GitoxideLabs/gitoxide/commit/4e081f2141dcb9919597c53dfd5706cc9439d541))
    - Make it possible to use `config::tree::Key` to more conveniently set values via `config::SnapshotMut::set()` ([`2a698fa`](https://github.com/GitoxideLabs/gitoxide/commit/2a698fab7323fd5befd14926bcb9cebf09afc312))
    - Release gix-protocol v0.33.1 ([`9c99ed3`](https://github.com/GitoxideLabs/gitoxide/commit/9c99ed30162081a7f26d72e0ed26966ff62d2b1c))
</details>

## 0.45.1 (2023-06-06)

### Bug Fixes

 - <csr-id-9010f586ac46fcea5b8abba8f30a5639ed6b9225/> `gix::env::fetch::collate::Error` now considers negotiation errors a sign of corrupt git repos.
   Indeed, all of these negotiation errors are due to failures reading references or objects that ought
   to be there.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-revision v0.15.1, gix v0.45.1 ([`11766a0`](https://github.com/GitoxideLabs/gitoxide/commit/11766a0a82754fee9918ccdb8eaf92af6d2561ba))
    - Merge branch 'adjustments-for-cargo' ([`04f011c`](https://github.com/GitoxideLabs/gitoxide/commit/04f011c3c3e49e87a3b868d4bf6e77a361b96da8))
    - `gix::env::fetch::collate::Error` now considers negotiation errors a sign of corrupt git repos. ([`9010f58`](https://github.com/GitoxideLabs/gitoxide/commit/9010f586ac46fcea5b8abba8f30a5639ed6b9225))
</details>

## 0.45.0 (2023-06-06)

<csr-id-dbc6cbb4363c2532f81b0bd6e351c4577bb9e9a3/>
<csr-id-9689a08d00e9d54f6bb581660ee99077bd214cb4/>

The reason for this release is the ability to properly negotiate packs, also across multiple rounds, and with `protocol.version` 1 or 2, across
stateless or stateful transports.

### Chore

 - <csr-id-dbc6cbb4363c2532f81b0bd6e351c4577bb9e9a3/> inline format args

### New Features

 - <csr-id-af0ef2f36736e3805f769d8cd59c9fa7eb6a22a0/> use `gix-negotiate` in fetch machinery.
   Thanks to it we are finally able to do pack negotiations just like git can,
   as many rounds as it takes and with all available algorithms.
   
   Works for V1 and V2 and for stateless and stateful transports.
 - <csr-id-020ff4e383fc76a255eabf099bb9cf5116a95afa/> Add `gitoxide.core.defaultPackCacheMemoryLimit` to control memory limits.
   Previously the 64 slot LRU cache didn't have any limit, now one is implemented that
   defaults to about 96MB.

### New Features (BREAKING)

 - <csr-id-e011e360fb2db0288f718cb3bb2b28b4652bc8ae/> respect `core.useReplaceRefs` and remove `gitoxide.objects.noReplace`.
   The gitoxide specific variable wasn't needed in the first place.

### Refactor (BREAKING)

 - <csr-id-9689a08d00e9d54f6bb581660ee99077bd214cb4/> Move `Kind` into `repository::Kind`.
   This type was from old times where `gix` was called `gix-repository`.
   Also remote `ThreadSafeRepository::kind()` in favor of leaving only
   `Repository::kind()`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 33 commits contributed to the release over the course of 40 calendar days.
 - 40 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#851](https://github.com/GitoxideLabs/gitoxide/issues/851)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#851](https://github.com/GitoxideLabs/gitoxide/issues/851)**
    - Add `gitoxide.core.defaultPackCacheMemoryLimit` to control memory limits. ([`020ff4e`](https://github.com/GitoxideLabs/gitoxide/commit/020ff4e383fc76a255eabf099bb9cf5116a95afa))
 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - `just fmt` ([`ffc1276`](https://github.com/GitoxideLabs/gitoxide/commit/ffc1276e0c991ac33ce842f5dca0b45ac69680c0))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Add test to validate alternates in the context of fetching ([`ae1bc41`](https://github.com/GitoxideLabs/gitoxide/commit/ae1bc41817bec3b83fe65104e7e3efe4bd798a78))
    - Use `gix-negotiate` in fetch machinery. ([`af0ef2f`](https://github.com/GitoxideLabs/gitoxide/commit/af0ef2f36736e3805f769d8cd59c9fa7eb6a22a0))
    - Respect `core.useReplaceRefs` and remove `gitoxide.objects.noReplace`. ([`e011e36`](https://github.com/GitoxideLabs/gitoxide/commit/e011e360fb2db0288f718cb3bb2b28b4652bc8ae))
    - Thanks clippy ([`9525ac8`](https://github.com/GitoxideLabs/gitoxide/commit/9525ac822aa902f5325f17e7b08ffb60b683e0e7))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Minor fixes ([`89a8cfe`](https://github.com/GitoxideLabs/gitoxide/commit/89a8cfe40e5c3a9d4a4181fa055e3f4a529a8081))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Move `Kind` into `repository::Kind`. ([`9689a08`](https://github.com/GitoxideLabs/gitoxide/commit/9689a08d00e9d54f6bb581660ee99077bd214cb4))
    - Merge branch 'auto-clippy' ([`dbf8aa1`](https://github.com/GitoxideLabs/gitoxide/commit/dbf8aa19d19109195d0274928eae4b94f248cd88))
    - Autofix map-or-unwrap clippy lint (and manual fix what was left) ([`2087032`](https://github.com/GitoxideLabs/gitoxide/commit/2087032b5956dcd82bce6ac57e530e8724b57f17))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge pull request #864 from nyurik/lint-fmt ([`279dc09`](https://github.com/GitoxideLabs/gitoxide/commit/279dc09446f41d7f1d76350fbfafb444e53cd7da))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Inline format args ([`dbc6cbb`](https://github.com/GitoxideLabs/gitoxide/commit/dbc6cbb4363c2532f81b0bd6e351c4577bb9e9a3))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Merge branch 'consecutive-negotiation' ([`97b3f7e`](https://github.com/GitoxideLabs/gitoxide/commit/97b3f7e2eaddea20c98f2f7ab6a0d2e2117b0793))
    - Release gix-commitgraph v0.15.0, gix-revision v0.14.0, gix-negotiate v0.1.0, safety bump 7 crates ([`92832ca`](https://github.com/GitoxideLabs/gitoxide/commit/92832ca2899cd2f222f4c7b1cc9e766178f55806))
    - Merge branch 'consecutive-negotiation' ([`4507f94`](https://github.com/GitoxideLabs/gitoxide/commit/4507f94984c811ea098e43472e5f54ec4dbb90c1))
    - Adapt to changes in `gix-revision` ([`56f4d30`](https://github.com/GitoxideLabs/gitoxide/commit/56f4d30960de4afc8c53136af45149cf880547c5))
    - Refactor ([`f4245f4`](https://github.com/GitoxideLabs/gitoxide/commit/f4245f4bb0921610456dde2c56068e7c5e4f1d27))
    - Merge branch 'fix-851' ([`2f275d5`](https://github.com/GitoxideLabs/gitoxide/commit/2f275d5d3cb49b3b8ba53b30e4b4386fac32662b))
    - Adjust to changes in `gix-pack` ([`215889c`](https://github.com/GitoxideLabs/gitoxide/commit/215889ceb976a59368c132aabfffb71a6a2ac9f8))
    - Support reading the fetch algorithm from configuration ([`33b7770`](https://github.com/GitoxideLabs/gitoxide/commit/33b777074db21db8cd060ecf8cfdac0409a7e10c))
    - Release gix-object v0.29.2 ([`4f879bf`](https://github.com/GitoxideLabs/gitoxide/commit/4f879bf35653bdc8f9729d524c6e8e1fb3c6886b))
    - Release gix-discover v0.18.1, gix-worktree v0.17.1, gix-testtools v0.12.0 ([`f7b6c6f`](https://github.com/GitoxideLabs/gitoxide/commit/f7b6c6f27c090cbc584fbd3f5403da5ac1a9ff02))
    - Release gix-index v0.16.1 ([`08c6f9d`](https://github.com/GitoxideLabs/gitoxide/commit/08c6f9de95c65ff05db4ce6a5593127c4280b2ef))
    - Release gix-ref v0.29.1 ([`13e01f5`](https://github.com/GitoxideLabs/gitoxide/commit/13e01f5742ed2121f00f4b16c1df0cce5e7708ef))
    - Improve docs for `Shallow` ([`3d95bb7`](https://github.com/GitoxideLabs/gitoxide/commit/3d95bb76746a56b6e9060245f6c190c3836a0102))
</details>

## 0.44.1 (2023-04-27)

A maintenance release without user-facing changes. It's meant to fix breakage that occurred when publishing a breaking change in `gix-path` by accident.

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

### New Features

 - <csr-id-08e8fc2152794652ba1c986df493c2ac915af9e7/> `gix index entries` also prints attributes.
 - <csr-id-bc28443e452c4de81368739a11a2482ae0a93485/> add `Repository::attributes()` and `Worktree::attributes()`.
 - <csr-id-40a1b7444ba9d9b61a1c22a7f25662eec3c25a1b/> add `index.threads` configuration to `gix::config::tree`
 - <csr-id-afe7faa14afb2ec4934f204e01ed12bcd0b3e786/> Before writing new objects, check if they exist.
   That way we safe expensive IO at the cost of some CPU.
 - <csr-id-037f52d4099e239c28210476ad7ab57d22aa3626/> add `Object::into_tag()` and `Tag::decode()` methods.
   This makes the API more symmetric as similar methods exist for commits
   and trees.
 - <csr-id-35cb6b42bd8071e5e5c16ed6d37884deea524330/> Allow `USE_NSEC` and `USE_STDEV` compile time flags to configured at runtime.
   Right now git may be compiled without these capabilities, even though on some platforms
   it might make perfect sense to enable them by default or enable them on a per repository
   basis. This is now possible thanks to added gitoxide specific functions.
 - <csr-id-358500f0efaec7c67b307a6a1aa27ecad7502eb7/> `open::Options` now allow controlling where gitattributes files are loaded from.
   That way it's possible to, for example, isolate all operations that rely on the `gitattribute`
   system, like checkouts or additions to the index.
 - <csr-id-ec93f75cfdf6cbd617c4a92eefae97f2c7736d65/> `revision::walk::Platform::selected(filter)` to selectively prune parts of the commit graph.

### Bug Fixes

 - <csr-id-2cd5054b0a1994571a25a49193449904cfd30b50/> When removing all shallow commits from shallow file, delete it.
   Previously it would leave an empty file, which will be ignored by the implementation
   but might be confusing to users.
 - <csr-id-43f695a9607f1f85f859f2ef944b785b5b6dd238/> `gix::open()` can handle bare repositories with index.
   These are mis-classified as non-bare repository, which previosuly
   caused it to get off-track.

### New Features (BREAKING)

 - <csr-id-26e6a661ed5827151708b9fcc3d7468aa60cf4e3/> add `Repository::excludes()` and simplify signature of `Worktree::excludes()`.
   Further, this change removes the `permission` module without replacement,
   and moves `permissions` into `open`.
   
   This corrects an artifact of this crate previously being name `gix-repository` and brings
   these types semantically closer to where they are actually used.
 - <csr-id-cb3437632fe7ff0ce4efd11c08a8d684d7e7e430/> support configuring the connection (i.e. for auth) during clone.
   This change also removes the generic type for Progress from `Connection`
   which forces it to be passed to every potentially long-running method.
 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.
 - <csr-id-b645d28f9641c6b4022e1e37ad9fe528922ec747/> remove types that are now available in `gix-os`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 48 commits contributed to the release over the course of 26 calendar days.
 - 27 days passed between releases.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#801](https://github.com/GitoxideLabs/gitoxide/issues/801), [#814](https://github.com/GitoxideLabs/gitoxide/issues/814)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#801](https://github.com/GitoxideLabs/gitoxide/issues/801)**
    - `revision::walk::Platform::selected(filter)` to selectively prune parts of the commit graph. ([`ec93f75`](https://github.com/GitoxideLabs/gitoxide/commit/ec93f75cfdf6cbd617c4a92eefae97f2c7736d65))
 * **[#814](https://github.com/GitoxideLabs/gitoxide/issues/814)**
    - Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability. ([`b83ee36`](https://github.com/GitoxideLabs/gitoxide/commit/b83ee366a3c65c717beb587ad809268f1c54b8ad))
 * **Uncategorized**
    - Release gix-worktree v0.16.0, gix v0.44.0 ([`4527fb8`](https://github.com/GitoxideLabs/gitoxide/commit/4527fb8e0fe0786a70ccfd8c3f5c5e79e8867944))
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`d7173b2`](https://github.com/GitoxideLabs/gitoxide/commit/d7173b2d2cb79685fdf7f618c31c576db24fa648))
    - Release gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0 ([`e4df557`](https://github.com/GitoxideLabs/gitoxide/commit/e4df5574c0813a0236319fa6e8b3b41bab179fc8))
    - Release gix-hash v0.11.1, gix-path v0.7.4, gix-glob v0.6.0, gix-attributes v0.11.0, gix-config-value v0.11.0, gix-fs v0.1.1, gix-tempfile v5.0.3, gix-utils v0.1.1, gix-lock v5.0.1, gix-object v0.29.1, gix-ref v0.28.0, gix-sec v0.7.0, gix-config v0.21.0, gix-prompt v0.4.0, gix-url v0.17.0, gix-credentials v0.13.0, gix-diff v0.29.0, gix-discover v0.17.0, gix-hashtable v0.2.0, gix-ignore v0.1.0, gix-bitmap v0.2.3, gix-traverse v0.25.0, gix-index v0.16.0, gix-mailmap v0.12.0, gix-pack v0.34.0, gix-odb v0.44.0, gix-packetline v0.16.0, gix-transport v0.30.0, gix-protocol v0.31.0, gix-revision v0.13.0, gix-refspec v0.10.0, gix-worktree v0.16.0, gix v0.44.0, safety bump 7 crates ([`91134a1`](https://github.com/GitoxideLabs/gitoxide/commit/91134a11c8ba0e942f692488ec9bce9fa1086324))
    - Prepare changelogs prior to release ([`30a1a71`](https://github.com/GitoxideLabs/gitoxide/commit/30a1a71f36f24faac0e0b362ffdfedea7f9cdbf1))
    - When removing all shallow commits from shallow file, delete it. ([`2cd5054`](https://github.com/GitoxideLabs/gitoxide/commit/2cd5054b0a1994571a25a49193449904cfd30b50))
    - Merge branch 'index-entries-attrs' ([`f37a930`](https://github.com/GitoxideLabs/gitoxide/commit/f37a930aefa27e67f0b693ba9669cc26d49044fa))
    - `gix index entries` also prints attributes. ([`08e8fc2`](https://github.com/GitoxideLabs/gitoxide/commit/08e8fc2152794652ba1c986df493c2ac915af9e7))
    - Adjust to changes in `gix-worktree` ([`27a39ca`](https://github.com/GitoxideLabs/gitoxide/commit/27a39cad498ca8b2c9cba05790284e2b68ba7636))
    - Add `Repository::attributes()` and `Worktree::attributes()`. ([`bc28443`](https://github.com/GitoxideLabs/gitoxide/commit/bc28443e452c4de81368739a11a2482ae0a93485))
    - Add `Repository::excludes()` and simplify signature of `Worktree::excludes()`. ([`26e6a66`](https://github.com/GitoxideLabs/gitoxide/commit/26e6a661ed5827151708b9fcc3d7468aa60cf4e3))
    - Add `index.threads` configuration to `gix::config::tree` ([`40a1b74`](https://github.com/GitoxideLabs/gitoxide/commit/40a1b7444ba9d9b61a1c22a7f25662eec3c25a1b))
    - Adjust to changes in `gix-worktree` ([`f722d6b`](https://github.com/GitoxideLabs/gitoxide/commit/f722d6bebcd215b6e270261a3ed032a5f7e7b72f))
    - Merge branch 'attributes-cache' ([`3456c84`](https://github.com/GitoxideLabs/gitoxide/commit/3456c845dfeedd2fa96b4313b1a84c8cbe9433c5))
    - Adjust to changes in `gix-worktree` ([`13a070f`](https://github.com/GitoxideLabs/gitoxide/commit/13a070f405230d52e4377e18f6bdc5c673b718a0))
    - Merge branch 'fix-823' ([`6ebd61e`](https://github.com/GitoxideLabs/gitoxide/commit/6ebd61e548a36a04e413ac725a03e607a3588334))
    - `gix::open()` can handle bare repositories with index. ([`43f695a`](https://github.com/GitoxideLabs/gitoxide/commit/43f695a9607f1f85f859f2ef944b785b5b6dd238))
    - Thanks clippy ([`14e64e7`](https://github.com/GitoxideLabs/gitoxide/commit/14e64e74649cfb1f2f99da87015939af98fae5c8))
    - Merge branch 'clone-auth' ([`1a65308`](https://github.com/GitoxideLabs/gitoxide/commit/1a653083bf0a3a01ee116535e65202392a2c676c))
    - Support configuring the connection (i.e. for auth) during clone. ([`cb34376`](https://github.com/GitoxideLabs/gitoxide/commit/cb3437632fe7ff0ce4efd11c08a8d684d7e7e430))
    - Merge branch 'fix-819' ([`69faad0`](https://github.com/GitoxideLabs/gitoxide/commit/69faad0d7cc100de54d757d42acc152a22edc022))
    - Before writing new objects, check if they exist. ([`afe7faa`](https://github.com/GitoxideLabs/gitoxide/commit/afe7faa14afb2ec4934f204e01ed12bcd0b3e786))
    - Add `Object::into_tag()` and `Tag::decode()` methods. ([`037f52d`](https://github.com/GitoxideLabs/gitoxide/commit/037f52d4099e239c28210476ad7ab57d22aa3626))
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Support native zlib-ng via flate2's zlib-ng feature ([`9a6e0d7`](https://github.com/GitoxideLabs/gitoxide/commit/9a6e0d7b418ea721da6a7e4bc48c47b47d4dfa79))
    - Make fmt ([`5d2b5d0`](https://github.com/GitoxideLabs/gitoxide/commit/5d2b5d02c3869e07dc2507a8f2519ee1df633df7))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Minor adjustments to the worktree structure. ([`8920229`](https://github.com/GitoxideLabs/gitoxide/commit/89202296f63dacedfd396aefe25e686b4d426b2a))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Create new `gix-fs` crate to consolidate all filesystem utilities ([`f8cc33c`](https://github.com/GitoxideLabs/gitoxide/commit/f8cc33cb372dd2b4bbe4a09cf4f64916681ab1dd))
    - Allow `USE_NSEC` and `USE_STDEV` compile time flags to configured at runtime. ([`35cb6b4`](https://github.com/GitoxideLabs/gitoxide/commit/35cb6b42bd8071e5e5c16ed6d37884deea524330))
    - Merge branch 'main' into dev ([`23ee47f`](https://github.com/GitoxideLabs/gitoxide/commit/23ee47fb24c197f8437bd426544b2aa74e005bdc))
    - Merge branch 'worktree-stack' ([`3d47919`](https://github.com/GitoxideLabs/gitoxide/commit/3d47919c1a2f83fc7c1fd7ae590d098057a22626))
    - `open::Options` now allow controlling where gitattributes files are loaded from. ([`358500f`](https://github.com/GitoxideLabs/gitoxide/commit/358500f0efaec7c67b307a6a1aa27ecad7502eb7))
    - Adjust to changes in `gix-attributes` ([`1755c81`](https://github.com/GitoxideLabs/gitoxide/commit/1755c81f64ce8a68807c2026eeae13dc46021db1))
    - Remove types that are now available in `gix-os` ([`b645d28`](https://github.com/GitoxideLabs/gitoxide/commit/b645d28f9641c6b4022e1e37ad9fe528922ec747))
    - Refactor ([`0677406`](https://github.com/GitoxideLabs/gitoxide/commit/067740636b3ca24ce90db91923dfd4ee592fa7f6))
    - Centralize index entry Stat creation/comparison ([`870bdb2`](https://github.com/GitoxideLabs/gitoxide/commit/870bdb2f3957e0f5690679e2aeb6752cd0b8d93e))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
    - Merge branch 'patch-1' ([`b02bf24`](https://github.com/GitoxideLabs/gitoxide/commit/b02bf247890c873184e58f734e0912eac6c6bbae))
    - Add test to run tests on 32 bit systems ([`fb31ee8`](https://github.com/GitoxideLabs/gitoxide/commit/fb31ee8bbcfc72fa0e7e38bc84d02f6f7d2f0fff))
    - Merge branch 'patch-1' ([`d0052c1`](https://github.com/GitoxideLabs/gitoxide/commit/d0052c13cabcde8058177d2439053b50ea5adbfc))
    - Upgrade serial-test to v2 ([`6932017`](https://github.com/GitoxideLabs/gitoxide/commit/69320174685e72940cd0fe600c94abb948a62bdd))
    - Release gix-revision v0.12.2 ([`ec64a88`](https://github.com/GitoxideLabs/gitoxide/commit/ec64a88690243a210efee6d5ae5164723e13f734))
    - Merge branch 'fix-801' ([`a884121`](https://github.com/GitoxideLabs/gitoxide/commit/a88412194ff8960cd69a3794042d9c6c29428ea6))
    - Prevent env-altering tests to affect shallow tests ([`61eec5a`](https://github.com/GitoxideLabs/gitoxide/commit/61eec5ae48006b4f0a6ac5c7b9549811dfa9431d))
</details>

## 0.43.1 (2023-03-30)

### Documentation

 - <csr-id-02c4659984fa6423bc76cc4980a143edaba8ace0/> fix minor typos
 - <csr-id-cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5/> fix minor typos

### New Features

 - <csr-id-7c2e5c8d08e4dd1ec115ae06f20f9c8f93d6d616/> add `Tree::decode()` and `TryFrom<Tree> for gix::objs::Tree`.
   This makes it possible to obtain mutable trees for creating trees by hand
   for the purpose of making commits.

### Bug Fixes

 - <csr-id-d1bd513f27e17787eb223f7b0521f954c518153e/> $HOME detection on windows

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 3 calendar days.
 - 4 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.7.3, gix-config-value v0.10.2, gix-config v0.20.1, gix-discover v0.16.2, gix-index v0.15.1, gix-odb v0.43.1, gix-packetline v0.15.1, gix-protocol v0.30.2, gix-worktree v0.15.2, gix v0.43.1 ([`38eed1d`](https://github.com/GitoxideLabs/gitoxide/commit/38eed1d06e7cbb8fbcd54b2cad3163ca45e0baf1))
    - Merge branch 'pascalkuthe/main' ([`d47cebe`](https://github.com/GitoxideLabs/gitoxide/commit/d47cebe3b23080c45829cb307b867220e3af20db))
    - Refactor ([`d1e5e12`](https://github.com/GitoxideLabs/gitoxide/commit/d1e5e12d54f79c030325860838c1cfadac1a7ac5))
    - $HOME detection on windows ([`d1bd513`](https://github.com/GitoxideLabs/gitoxide/commit/d1bd513f27e17787eb223f7b0521f954c518153e))
    - Fix minor typos ([`02c4659`](https://github.com/GitoxideLabs/gitoxide/commit/02c4659984fa6423bc76cc4980a143edaba8ace0))
    - Fix minor typos ([`cc48c35`](https://github.com/GitoxideLabs/gitoxide/commit/cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5))
    - Add `Tree::decode()` and `TryFrom<Tree> for gix::objs::Tree`. ([`7c2e5c8`](https://github.com/GitoxideLabs/gitoxide/commit/7c2e5c8d08e4dd1ec115ae06f20f9c8f93d6d616))
    - Release gix-ref v0.27.2 ([`e965b18`](https://github.com/GitoxideLabs/gitoxide/commit/e965b18aedcf13ec4538bc7bc700269a56ca615e))
    - Be sure to clear the buffer after an intermediate read error happened and we ignore it. ([`877951a`](https://github.com/GitoxideLabs/gitoxide/commit/877951aa0009ab5e2a814c95f4c5d3662305cb27))
</details>

## 0.43.0 (2023-03-26)

<csr-id-87f5621d941b5af40abd59a26164a09d0dde2649/>

### Bug Fixes

 - <csr-id-7bd8823ab4241d6d0401f03aec8c0d34f68c347c/> opening repositories without 'strict' mode also ignores IO errors.
   These will instead be logged, but won't make it impossible to open an
   otherwise fine repository.

### Other

 - <csr-id-87f5621d941b5af40abd59a26164a09d0dde2649/> make clear that `gix::discover()` isn't suited for authentication remote operations.
   We also provide information on how to accomplish this.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#787](https://github.com/GitoxideLabs/gitoxide/issues/787), [#790](https://github.com/GitoxideLabs/gitoxide/issues/790)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#787](https://github.com/GitoxideLabs/gitoxide/issues/787)**
    - Make clear that `gix::discover()` isn't suited for authentication remote operations. ([`87f5621`](https://github.com/GitoxideLabs/gitoxide/commit/87f5621d941b5af40abd59a26164a09d0dde2649))
 * **[#790](https://github.com/GitoxideLabs/gitoxide/issues/790)**
    - Opening repositories without 'strict' mode also ignores IO errors. ([`7bd8823`](https://github.com/GitoxideLabs/gitoxide/commit/7bd8823ab4241d6d0401f03aec8c0d34f68c347c))
 * **Uncategorized**
    - Release gix-tempfile v5.0.2, gix-validate v0.7.4, gix-config v0.20.0, gix-prompt v0.3.3, gix-diff v0.28.1, gix-discover v0.16.1, gix-pack v0.33.2, gix-transport v0.29.1, gix-protocol v0.30.1, gix-revision v0.12.1, gix-worktree v0.15.1, gix v0.43.0, safety bump gix v0.43.0 ([`5dc1f9f`](https://github.com/GitoxideLabs/gitoxide/commit/5dc1f9f2bcb8b3e147115fcb6f76558e8f48ffef))
    - Prepare changelogs prior to release ([`3016a28`](https://github.com/GitoxideLabs/gitoxide/commit/3016a285f566bdfe7de2774fa6f2254c1b1a2c51))
    - Merge branch 'fix-790' ([`ee36e5b`](https://github.com/GitoxideLabs/gitoxide/commit/ee36e5bb985e9ad90bc382cdd051a8b5295ca18c))
    - Less dependencies for tests (via `serial_test` no default features) ([`8f2accd`](https://github.com/GitoxideLabs/gitoxide/commit/8f2accdf738def9aa4abdf08fc299d0e9807bc3e))
</details>

## 0.42.0 (2023-03-14)

### New Features

 - <csr-id-93d412c54833d822e5369644226c6fd3b888c89c/> shallow support for `fetch` operations.
 - <csr-id-4e89c19d7656a96bd512dafbc9669011487671f5/> shallow support for `clone` operations.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.28.1, gix-tempfile v5.0.1, gix-ref v0.27.1, gix-pack v0.33.1, gix-packetline v0.15.0, gix-transport v0.29.0, gix-protocol v0.30.0, gix v0.42.0, safety bump 3 crates ([`c1f1bfb`](https://github.com/GitoxideLabs/gitoxide/commit/c1f1bfb8dc0e73993678353e4492d0614b642ed1))
    - Prepare changelogs prior to release ([`c66e298`](https://github.com/GitoxideLabs/gitoxide/commit/c66e2982577e4cd9faef63798986b8cf8ece93a2))
    - Make fmt ([`3836cc0`](https://github.com/GitoxideLabs/gitoxide/commit/3836cc0c9c3e1158b56142b924483c8a77217d53))
    - Merge branch 'various-fixes' ([`cc0f506`](https://github.com/GitoxideLabs/gitoxide/commit/cc0f5061fba27d57022dc616c941034b98fd4875))
    - Improve fetchspec handling to be closer to what git does. ([`a22621d`](https://github.com/GitoxideLabs/gitoxide/commit/a22621d1b92a6155b83a09e68ed1de3a4860e766))
    - Assure that --deepen 0 (despite allowed) doesn't actually confuse the server. ([`b43ea6b`](https://github.com/GitoxideLabs/gitoxide/commit/b43ea6bdc873da2facdb0fe8369ab1644a6702ef))
    - Adjust to changes in `gix-packetline` ([`4f45814`](https://github.com/GitoxideLabs/gitoxide/commit/4f45814eea9c20b449effd9b29d31623943ff853))
    - Merge branch 'shallow-protocol' ([`531dd19`](https://github.com/GitoxideLabs/gitoxide/commit/531dd19502b8b635fb1a60f747eb381fd12e00ca))
    - Shallow support for `fetch` operations. ([`93d412c`](https://github.com/GitoxideLabs/gitoxide/commit/93d412c54833d822e5369644226c6fd3b888c89c))
    - Shallow support for `clone` operations. ([`4e89c19`](https://github.com/GitoxideLabs/gitoxide/commit/4e89c19d7656a96bd512dafbc9669011487671f5))
    - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
</details>

## 0.41.0 (2023-03-10)

A maintenance release without user-facing changes, but with some fixes in the dependency chain, namely:

- `gix-credentials` allows credential helpers to ignore `stdin`, making it robust when facing helpers that don't read from `stdin`.
- `gix-tempfile` refers to the most recent version of `tempfile` without pinning it, which removes a security vulnerability.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 day passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v5.0.0, gix-lock v5.0.0, gix-ref v0.27.0, gix-config v0.19.0, gix-url v0.16.0, gix-credentials v0.12.0, gix-discover v0.16.0, gix-index v0.15.0, gix-pack v0.33.0, gix-odb v0.43.0, gix-transport v0.28.0, gix-protocol v0.29.0, gix-worktree v0.15.0, gix v0.41.0, safety bump 12 crates ([`29a0870`](https://github.com/GitoxideLabs/gitoxide/commit/29a087043d1feb2f127b065341c8028d0bd0301e))
    - Prepare changelogs prior to release ([`e06f5f5`](https://github.com/GitoxideLabs/gitoxide/commit/e06f5f523e83f4da390eddbebcb9a2d58674587b))
    - Merge branch 'password-in-urls' ([`85f8b28`](https://github.com/GitoxideLabs/gitoxide/commit/85f8b283a1671e2631cda437ca8da93f9a2a4ebd))
    - Adjust to changes in `gix-url` ([`66602bb`](https://github.com/GitoxideLabs/gitoxide/commit/66602bbb7fe62f7425c8289902a1d2fce121e87c))
</details>

## 0.40.0 (2023-03-09)

### New Features

 - <csr-id-5bfbb9a32f8edb8bfb71ae00167277b9109de35a/> `Repository::shallow_commits()` returns an uptodate list of shallow boundary commits.
 - <csr-id-3e69535630714205904fe64f511da28a3f2d7fb6/> `Repository::is_shallow()` to test if a repository is shallow.

### Bug Fixes (BREAKING)

 - <csr-id-1046ea2b3312838169aa08f30b598bf4ce2338d9/> allow to traverse the entire commit graph of shallow repos
   Previously, when traversing commits, we would assume to be in a
   shallow repository if a commit's parent could not be found in the
   repository.
   
   Now we validate that assumption by reading the 'shallow' file to
   check if the last seen commit is on the commit boundary.
   
   This removes `is_shallow` and `error_on_missing_commit()` on the
   `revision::walk::Platform` as shallow commits are now known and handled
   without any guesswork.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 3 calendar days.
 - 4 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.40.0 ([`18e72c9`](https://github.com/GitoxideLabs/gitoxide/commit/18e72c988a58415080d4555bc869ae04df8d04fa))
    - Merge branch 'shallow-support' ([`6d88fd2`](https://github.com/GitoxideLabs/gitoxide/commit/6d88fd208bcdec0603d57785bdbfe2f286a65384))
    - Allow to traverse the entire commit graph of shallow repos ([`1046ea2`](https://github.com/GitoxideLabs/gitoxide/commit/1046ea2b3312838169aa08f30b598bf4ce2338d9))
    - `Repository::shallow_commits()` returns an uptodate list of shallow boundary commits. ([`5bfbb9a`](https://github.com/GitoxideLabs/gitoxide/commit/5bfbb9a32f8edb8bfb71ae00167277b9109de35a))
    - `Repository::is_shallow()` to test if a repository is shallow. ([`3e69535`](https://github.com/GitoxideLabs/gitoxide/commit/3e69535630714205904fe64f511da28a3f2d7fb6))
</details>

## 0.39.0 (2023-03-04)

A maintenance release without user-facing changes, primarily for getting the progress-bar updates into `cargo`.

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

## 0.38.0 (2023-03-01)

### New Features

 - <csr-id-256f7d46ed88067aa96f47be2a97a6f9f5b98075/> the `hp-tempfile-registry` feature toggle to control the `dashmap` dependency.
   And also, probably provide a better performance in certain cases.
 - <csr-id-fd7eebcd922f98c1aed9e3177b9a48ff1415ffd8/> make `gix-pack` feature toggles related to pack caches available.
   Previously they would have to be configured by pulling in `gix-pack`, which
   isn't desirable as the only crate we want to expose like that is `gix-features`.
 - <csr-id-5b0ebd272c3d98e26c9249ed27b4ea9a8ad80746/> Add `comfort` feature toggle (default enabled) to make better progress units available.
   This could be a breaking change for those who turned default-features off, as you may now
   have to re-add the `comfort` feature to get nicer progress messages.

### Bug Fixes

 - <csr-id-b2375e3dbe1f87ee3ac6e814fc8f4898143c438d/> `gix-tempfile` is now configured to not use the high-performance hashmap anymore.
   It was hard to justify as tests actually seemed to be faster without it.

### New Features (BREAKING)

 - <csr-id-fea8c56089e5b354669396853c5bd0f31bdf0d33/> Put `progress::tree` behind the `progress-tree` feature toggle.
   It's a convenience export that implies pulling in more dependencies, so it
   should be gated.
 - <csr-id-441f5ac4dd2f0636ec07065f8095e8bae5ce6985/> gate all signal handling behind the `signals` feature toggle.
   This change also consolidates all signal handling into its own module called
   `signal` to provide reusable handlers and as well as well as signal initialization.
   
   Note that the functions to cleanup tempfiles don't interact with the signal registry,
   hence they still can be called without the `signals` feature enabled.
   
   Note that this change sneakily fixes a bug that could have caused a `write_all()`
   on a tempfile that was removed by a signal to enter an infinite loop.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 4 calendar days.
 - 4 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#339](https://github.com/GitoxideLabs/gitoxide/issues/339)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#339](https://github.com/GitoxideLabs/gitoxide/issues/339)**
    - Gate all signal handling behind the `signals` feature toggle. ([`441f5ac`](https://github.com/GitoxideLabs/gitoxide/commit/441f5ac4dd2f0636ec07065f8095e8bae5ce6985))
 * **Uncategorized**
    - Release gix-tempfile v4.1.0, gix-lock v4.0.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0, safety bump 6 crates ([`ea9fd1d`](https://github.com/GitoxideLabs/gitoxide/commit/ea9fd1d9b60e1e9e17042e9e37c06525823c40a5))
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
    - Merge branch 'adjustments-for-cargo' ([`d686d94`](https://github.com/GitoxideLabs/gitoxide/commit/d686d94e1030a8591ba074757d56927a346c8351))
    - `gix-tempfile` is now configured to not use the high-performance hashmap anymore. ([`b2375e3`](https://github.com/GitoxideLabs/gitoxide/commit/b2375e3dbe1f87ee3ac6e814fc8f4898143c438d))
    - Depend on latest version of `prodash` for performance improvements. ([`5d00324`](https://github.com/GitoxideLabs/gitoxide/commit/5d003242abe82b1604e2188d49dec9690ebb2a6a))
    - The `hp-tempfile-registry` feature toggle to control the `dashmap` dependency. ([`256f7d4`](https://github.com/GitoxideLabs/gitoxide/commit/256f7d46ed88067aa96f47be2a97a6f9f5b98075))
    - Make `gix-pack` feature toggles related to pack caches available. ([`fd7eebc`](https://github.com/GitoxideLabs/gitoxide/commit/fd7eebcd922f98c1aed9e3177b9a48ff1415ffd8))
    - Put `progress::tree` behind the `progress-tree` feature toggle. ([`fea8c56`](https://github.com/GitoxideLabs/gitoxide/commit/fea8c56089e5b354669396853c5bd0f31bdf0d33))
    - Add `comfort` feature toggle (default enabled) to make better progress units available. ([`5b0ebd2`](https://github.com/GitoxideLabs/gitoxide/commit/5b0ebd272c3d98e26c9249ed27b4ea9a8ad80746))
    - Prepare for git-tempfile release ([`56c005b`](https://github.com/GitoxideLabs/gitoxide/commit/56c005b13c44376f71e61781e73c0bf93416d0e4))
    - Merge branch 'tempfile-upgrades' ([`3522cba`](https://github.com/GitoxideLabs/gitoxide/commit/3522cbaac721c8079605be51b9053014bc5e863a))
    - Adjust to changes in `gix-tempfile` ([`c6785fc`](https://github.com/GitoxideLabs/gitoxide/commit/c6785fc7082b90c8a27cef6a0f5cc5acd8cb8951))
    - Make fmt ([`8ef1cb2`](https://github.com/GitoxideLabs/gitoxide/commit/8ef1cb293434c7b9e1fda4a6963368e0435920a9))
    - Fix diff-tests on windows ([`441a64b`](https://github.com/GitoxideLabs/gitoxide/commit/441a64b6b703f7f97cfcefe4d3db31bc7427b48c))
</details>

## 0.37.2 (2023-02-24)

### Bug Fixes

 - <csr-id-1d3d22d45e70222c12fcf5a82063ceb9321a0129/> reproduce a diff issue and fix it
   Diffs could be quite wrong and this is a small repro along with the fix.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-object v0.26.4, gix-diff v0.26.3, gix v0.37.2, gix-commitgraph v0.13.1, gitoxide-core v0.25.0, gitoxide v0.23.0 ([`9982949`](https://github.com/GitoxideLabs/gitoxide/commit/9982949cab401501d5ce3cba4e2ba900bc249c53))
    - Fix new diff tests on windows ([`b1ec1b7`](https://github.com/GitoxideLabs/gitoxide/commit/b1ec1b776696b4b1d73e3dd26cbaf33260367855))
    - Prepare changelog for release ([`13a1ec1`](https://github.com/GitoxideLabs/gitoxide/commit/13a1ec1803d677c2e94f3ea0461118c2426f8071))
    - Merge branch 'rename-tracking' ([`550144a`](https://github.com/GitoxideLabs/gitoxide/commit/550144a5fd37d501d86f4b1c4db2948d951d1c93))
    - Reproduce a diff issue and fix it ([`1d3d22d`](https://github.com/GitoxideLabs/gitoxide/commit/1d3d22d45e70222c12fcf5a82063ceb9321a0129))
</details>

## 0.37.1 (2023-02-21)

A maintenance release to restore MSRV (1.64) support.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-config v0.16.3, gix v0.37.1 ([`a3c283f`](https://github.com/GitoxideLabs/gitoxide/commit/a3c283ff0e3f21cedb3ba7cd464fdfa0f5133af0))
    - Prepare changelogs prior to release ([`362d659`](https://github.com/GitoxideLabs/gitoxide/commit/362d659f946ca1ff2cbf915766113a34a9df97b3))
    - Restore msrv compatibility by removing sole `if let ... else` ([`9160659`](https://github.com/GitoxideLabs/gitoxide/commit/91606597b714a6e9b3a2c071bdb08baeacd6056b))
</details>

## 0.37.0 (2023-02-20)

### Bug Fixes

 - <csr-id-d3b974000133caa0ea107cb4724b950eda91d69b/> `Repository::object_cache_size()` now unsets the cache if `Some(0)` is passed.
   Previously it would fail.

### New Features (BREAKING)

 - <csr-id-ed87f4c7c2799625bc6c7109368687908f0fb6f0/> `object::tree::diff::Platform::track_rewrites(...)`
   The invocation of `object::tree::diff::Platform::track_rewrites(Rewrites { percentage: None, ..Default::default() })`
   is now able to explicitly configure perfect rename tracking without percentage of equivalence.
   
   By setting `percentage = Some(<fraction>)` one can set how similar both files should be to be considered related.
   
   The same can be configured for copy-tracking, which also includes something like `--find-copies-harder`.
   
   Note that by default, renames are considered if a file looks 50% similar, and copies tracking is
   using the same convention.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-object v0.26.3, gix-diff v0.26.2, gix-traverse v0.22.2, gix v0.37.0, safety bump 3 crates ([`8b3e42f`](https://github.com/GitoxideLabs/gitoxide/commit/8b3e42f69fe97fe5083eb845c323f10d7ac087b2))
    - `Repository::object_cache_size()` now unsets the cache if `Some(0)` is passed. ([`d3b9740`](https://github.com/GitoxideLabs/gitoxide/commit/d3b974000133caa0ea107cb4724b950eda91d69b))
    - Merge branch 'rename-tracking' ([`35415c5`](https://github.com/GitoxideLabs/gitoxide/commit/35415c5061bf5ea90a04db80d06ac3622d0b0f1a))
    - `object::tree::diff::Platform::track_rewrites(...)` ([`ed87f4c`](https://github.com/GitoxideLabs/gitoxide/commit/ed87f4c7c2799625bc6c7109368687908f0fb6f0))
</details>

## 0.36.1 (2023-02-20)

### Bug Fixes

 - <csr-id-135d317065aae87af302beb6c26bb6ca8e30b6aa/> compatibility with `bstr` v1.3, use `*.as_bytes()` instead of `.as_ref()`.
   `as_ref()` relies on a known target type which isn't always present. However, once
   there is only one implementation, that's no problem, but when that changes compilation
   fails due to ambiguity.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.36.1 ([`fac6bce`](https://github.com/GitoxideLabs/gitoxide/commit/fac6bce2f9942d7f333f66a92374d5400a00b0a5))
    - Release gix-date v0.4.3, gix-hash v0.10.3, gix-features v0.26.5, gix-actor v0.17.2, gix-glob v0.5.5, gix-path v0.7.2, gix-quote v0.4.2, gix-attributes v0.8.3, gix-validate v0.7.3, gix-object v0.26.2, gix-ref v0.24.1, gix-config v0.16.2, gix-command v0.2.4, gix-url v0.13.3, gix-credentials v0.9.2, gix-discover v0.13.1, gix-index v0.12.4, gix-mailmap v0.9.3, gix-pack v0.30.3, gix-packetline v0.14.3, gix-transport v0.25.6, gix-protocol v0.26.4, gix-revision v0.10.4, gix-refspec v0.7.3, gix-worktree v0.12.3, gix v0.36.1 ([`9604783`](https://github.com/GitoxideLabs/gitoxide/commit/96047839a20a657a559376b0b14c65aeab96acbd))
    - Compatibility with `bstr` v1.3, use `*.as_bytes()` instead of `.as_ref()`. ([`135d317`](https://github.com/GitoxideLabs/gitoxide/commit/135d317065aae87af302beb6c26bb6ca8e30b6aa))
    - Release gix-glob v0.5.4 ([`c56d336`](https://github.com/GitoxideLabs/gitoxide/commit/c56d3365fde21120cf6101cf34f8b5669804977c))
    - Release gix-transport v0.25.5 ([`f872ba8`](https://github.com/GitoxideLabs/gitoxide/commit/f872ba8271a5d632acc071e7a857ef19f7cf5610))
</details>

## 0.36.0 (2023-02-17)

### New Features

 - <csr-id-4f49992fae2bc60b22644e86808d61afe557f192/> cloning repositories doesn't require a committer anymore.
   This is similar to what git does and probably a decent convenience to have.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 45 commits contributed to the release over the course of 3 calendar days.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#737](https://github.com/GitoxideLabs/gitoxide/issues/737)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#737](https://github.com/GitoxideLabs/gitoxide/issues/737)**
    - Cloning repositories doesn't require a committer anymore. ([`4f49992`](https://github.com/GitoxideLabs/gitoxide/commit/4f49992fae2bc60b22644e86808d61afe557f192))
 * **Uncategorized**
    - Release gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`7fc00f8`](https://github.com/GitoxideLabs/gitoxide/commit/7fc00f87d74aedf631ce4032be1cdfe1804c7e7d))
    - Release gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`59e9fac`](https://github.com/GitoxideLabs/gitoxide/commit/59e9fac67d1b353e124300435b55f6b5468d7deb))
    - Release gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`48f5bd2`](https://github.com/GitoxideLabs/gitoxide/commit/48f5bd2014fa3dda6fbd60d091065c5537f69453))
    - Release gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`a5869e0`](https://github.com/GitoxideLabs/gitoxide/commit/a5869e0b223406820bca836e3e3a7fae2bfd9b04))
    - Release gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`41d57b9`](https://github.com/GitoxideLabs/gitoxide/commit/41d57b98964094fc1528adb09f69ca824229bf25))
    - Release gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`e313112`](https://github.com/GitoxideLabs/gitoxide/commit/e31311257bd138b52042dea5fc40c3abab7f269b))
    - Release gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6efd0d3`](https://github.com/GitoxideLabs/gitoxide/commit/6efd0d31fbeca31ab7319aa2ac97bb31dc4ce055))
    - Release gix-date v0.4.2, gix-hash v0.10.2, gix-features v0.26.4, gix-actor v0.17.1, gix-glob v0.5.3, gix-path v0.7.1, gix-quote v0.4.1, gix-attributes v0.8.2, gix-config-value v0.10.1, gix-tempfile v3.0.2, gix-lock v3.0.2, gix-validate v0.7.2, gix-object v0.26.1, gix-ref v0.24.0, gix-sec v0.6.2, gix-config v0.16.1, gix-command v0.2.3, gix-prompt v0.3.2, gix-url v0.13.2, gix-credentials v0.9.1, gix-diff v0.26.1, gix-discover v0.13.0, gix-hashtable v0.1.1, gix-bitmap v0.2.1, gix-traverse v0.22.1, gix-index v0.12.3, gix-mailmap v0.9.2, gix-chunk v0.4.1, gix-pack v0.30.2, gix-odb v0.40.2, gix-packetline v0.14.2, gix-transport v0.25.4, gix-protocol v0.26.3, gix-revision v0.10.3, gix-refspec v0.7.2, gix-worktree v0.12.2, gix v0.36.0 ([`6ccc88a`](https://github.com/GitoxideLabs/gitoxide/commit/6ccc88a8e4a56973b1a358cf72dc012ee3c75d56))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Release git-date v0.4.3, git-hash v0.10.3, git-features v0.26.5, git-actor v0.17.2, git-glob v0.5.4, git-path v0.7.2, git-quote v0.4.2, git-attributes v0.8.3, git-bitmap v0.2.2, git-chunk v0.4.2, git-command v0.2.4, git-commitgraph v0.13.1, git-config-value v0.10.2, git-tempfile v3.0.3, git-lock v3.0.3, git-validate v0.7.3, git-object v0.26.2, git-ref v0.24.1, git-sec v0.6.3, git-config v0.16.2, git-prompt v0.3.3, git-url v0.13.3, git-credentials v0.9.2, git-diff v0.26.2, git-discover v0.13.1, git-fetchhead v0.1.0, git-filter v0.1.0, git-hashtable v0.1.2, git-traverse v0.22.2, git-index v0.12.4, git-lfs v0.1.0, git-mailmap v0.9.3, git-note v0.1.0, git-pack v0.31.0, git-odb v0.41.0, git-packetline v0.14.3, git-pathspec v0.1.0, git-transport v0.25.5, git-protocol v0.26.4, git-rebase v0.1.0, git-revision v0.10.4, git-refspec v0.7.3, git-sequencer v0.1.0, git-submodule v0.1.0, git-tix v0.1.0, git-tui v0.1.0, git-worktree v0.12.3, safety bump 2 crates ([`90035a3`](https://github.com/GitoxideLabs/gitoxide/commit/90035a332d0ba67584558db3605500fbcb424ddd))
    - Rename `git-testtools` to `gix-testtools` ([`b65c33d`](https://github.com/GitoxideLabs/gitoxide/commit/b65c33d256cfed65d11adeff41132e3e58754089))
    - Adjust to renaming of `git-pack` to `gix-pack` ([`1ee81ad`](https://github.com/GitoxideLabs/gitoxide/commit/1ee81ad310285ee4aa118118a2be3810dbace574))
    - Adjust to renaming of `git-odb` to `gix-odb` ([`476e2ad`](https://github.com/GitoxideLabs/gitoxide/commit/476e2ad1a64e9e3f0d7c8651d5bcbee36cd78241))
    - Adjust to renaming of `git-index` to `gix-index` ([`86db5e0`](https://github.com/GitoxideLabs/gitoxide/commit/86db5e09fc58ce66b252dc13b8d7e2c48e4d5062))
    - Adjust to renaming of `git-diff` to `gix-diff` ([`49a163e`](https://github.com/GitoxideLabs/gitoxide/commit/49a163ec8b18f0e5fcd05a315de16d5d8be7650e))
    - Adjust to renaming of `git-mailmap` to `gix-mailmap` ([`2e28c56`](https://github.com/GitoxideLabs/gitoxide/commit/2e28c56bb9f70de6f97439818118d3a25859698f))
    - Adjust to renaming of `git-discover` to `gix-discover` ([`53adfe1`](https://github.com/GitoxideLabs/gitoxide/commit/53adfe1c34e9ea3b27067a97b5e7ac80b351c441))
    - Adjust to renaming for `git-protocol` to `gix-protocol` ([`823795a`](https://github.com/GitoxideLabs/gitoxide/commit/823795addea3810243cab7936cd8ec0137cbc224))
    - Adjust to renaming of `git-refspec` to `gix-refspec` ([`c958802`](https://github.com/GitoxideLabs/gitoxide/commit/c9588020561577736faa065e7e5b5bb486ca8fe1))
    - Adjust to renaming of `git-revision` to `gix-revision` ([`ee0ee84`](https://github.com/GitoxideLabs/gitoxide/commit/ee0ee84607c2ffe11ee75f27a31903db68afed02))
    - Adjust to renaming of `git-transport` to `gix-transport` ([`b2ccf71`](https://github.com/GitoxideLabs/gitoxide/commit/b2ccf716dc4425bb96651d4d58806a3cc2da219e))
    - Adjust to renaming of `git-credentials` to `gix-credentials` ([`6b18abc`](https://github.com/GitoxideLabs/gitoxide/commit/6b18abcf2856f02ab938d535a65e51ac282bf94a))
    - Adjust to renaming of `git-prompt` to `gix-prompt` ([`6a4654e`](https://github.com/GitoxideLabs/gitoxide/commit/6a4654e0d10ab773dd219cb4b731c0fc1471c36d))
    - Adjust to renaming of `git-worktree` to `gix-worktree` ([`73a1282`](https://github.com/GitoxideLabs/gitoxide/commit/73a12821b3d9b66ec1714d07dd27eb7a73e3a544))
    - Adjust to renamining of `git-hashtable` to `gix-hashtable` ([`26a0c98`](https://github.com/GitoxideLabs/gitoxide/commit/26a0c98d0a389b03e3dc7bfc758b37155e285244))
    - Adjust to renamining of `git-worktree` to `gix-worktree` ([`108bb1a`](https://github.com/GitoxideLabs/gitoxide/commit/108bb1a634f4828853fb590e9fc125f79441dd38))
    - Adjust to renaming of `git-url` to `gix-url` ([`b50817a`](https://github.com/GitoxideLabs/gitoxide/commit/b50817aadb143e19f61f64e19b19ec1107d980c6))
    - Adjust to renaming of `git-date` to `gix-date` ([`9a79ff2`](https://github.com/GitoxideLabs/gitoxide/commit/9a79ff2d5cc74c1efad9f41e21095ae498cce00b))
    - Adjust to renamining of `git-attributes` to `gix-attributes` ([`4a8b3b8`](https://github.com/GitoxideLabs/gitoxide/commit/4a8b3b812ac26f2a2aee8ce8ca81591273383c84))
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
    - Fix `gix` changelog (find-replace issue) to indicate renaming from `git-repository` ([`f86b780`](https://github.com/GitoxideLabs/gitoxide/commit/f86b7803e85839450ed2eeef57fe738da1e3ec87))
    - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
    - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
</details>

## 0.35.0 (2023-02-13)

This is the last release under this name and merely a notice to inform that `git-repository` from now on is `gix`.

Furthermore, all `git-*` crates belonging to the `gitoxide` project will be renamed to `gix-*`.

### Changed (BREAKING)

 - <csr-id-1408482fd21be7487b46753bb54a018c7a164f34/> a note of the pending rename of `git-repository` to `gix`

### New Features

 - <csr-id-069eb6c3f0844b43873ae1bd536e2bca53ff5c8a/> tree diffs with simple rename and copy tracking in cases where there is no additional modification.
   As the fastest way of rename tracking, we now offer support for tracking renames and copies,
   that is a file was renamed or copied without modification.
 - <csr-id-f6ed34aa254d34e596ad027c33f78404a630ff76/> Add `diff.renames` and `diff.renameLimit` keys to config tree.
   In preparation for the implementation.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release git-config v0.16.1, git-revision v0.10.3, gix v0.35.0 ([`74390ba`](https://github.com/GitoxideLabs/gitoxide/commit/74390baf9d177a1abe3c7c35f1d9bc67faba1e97))
    - Show more debugging information if unreachable code is reached. ([`66f5341`](https://github.com/GitoxideLabs/gitoxide/commit/66f53414efef6cfd6d03f830520964c9bdd23634))
    - Prepare changelogs prior to release ([`446f866`](https://github.com/GitoxideLabs/gitoxide/commit/446f866d146e255ab8302b89f87bf28f2c5f3733))
    - Merge branch 'rename-crates' ([`6461c3d`](https://github.com/GitoxideLabs/gitoxide/commit/6461c3da4d6daee857606d94294c3f87fc36965a))
    - Rename `git-repository` to `gix` ([`7bed2a9`](https://github.com/GitoxideLabs/gitoxide/commit/7bed2a96604397fa990f427b1a970ddeb6f09f1c))
</details>

## 0.0.0 (2023-02-10)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix v0.0.0 ([`8bce6d5`](https://github.com/GitoxideLabs/gitoxide/commit/8bce6d5cba12630bf4d12ed92f572a379d945329))
    - Add `gix` crate to reserve name ([`5104a78`](https://github.com/GitoxideLabs/gitoxide/commit/5104a787127bf0b1f9b65f371b7c5b79f491e396))
</details>

## 0.34.0 (2023-02-09)

<csr-id-a01f5d72346c36fdcb77af095273da6f4ab86e21/>

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### New Features

 - <csr-id-297d59e8396fbe2e5a2224a8524fa0778e786773/> add `env::collate::fetch::Error` - a combined error type with its own API.
   This error API allows to look at all the steps it takes to perform an operation and
   gather insights from it which require understanding a lot about the semantics of
   the contained errors.
 - <csr-id-d792ea543246632bf1ca8d0e1d239bbe7f07e219/> use enumerations to advertise progress ids publicly.
   Previously these were an implementation detail which also means they
   couldn't be relied upon.
   
   Thanks to an intermediate enumeration, they become part of the public API
   and their actual value is not exposed.
 - <csr-id-5dc408f726d6f0f480438348eb5d713776329710/> read shared indices by dissolving them into the current one.
   This allows the 'link' extension to be processed correctly, even though it
   won't be maintained. When written back, the 'link' extension will be removed
   automatically.

### Bug Fixes

 - <csr-id-5d3a3a245968d5ad8c29ea11a99b4896d1b41191/> don't panic, but error when parsing the rev-specs `^`, `..`, `...`.

### Chore (BREAKING)

 - <csr-id-a01f5d72346c36fdcb77af095273da6f4ab86e21/> adjust to changes in `gitoxide` for clap upgrade to 4.1

### New Features (BREAKING)

 - <csr-id-2faad43d11283ff06381c51d2466307cfb8736ff/> transfer knowledge about configuration and its usage into the type system.
   That way it's possible to use configuration overrides, even though ultimately being strings,
   in a type-safe manner and leverage code-completion while at it.
   
   In that process, we also change `Repository::(committer|Author)()` to return
   `Option<Result<...>>` to be able to account for date parse errors.

## 0.33.0 (2023-01-10)

<csr-id-dd7f3bf19cce0d214924fa86aeb4c5823f5bcc02/>

### Chore (BREAKING)

 - <csr-id-dd7f3bf19cce0d214924fa86aeb4c5823f5bcc02/> upgrade MSRV to v1.64 (possible due to `windows` upgrade)

## 0.32.0 (2023-01-09)

<csr-id-80dcb406c5f588122531da115398094de3c3af79/>

### Bug Fixes

 - <csr-id-a05b1c4d82bc6c7758989a3bbe326ea610903820/> default author and committer time
   When needing to fallback to a default author or committer signature, the
   time from GIT_AUTHOR_DATE should only be used for the author and
   GIT_COMMITTER_DATE should only be used for the committer and not
   intermixed. This change enforces that constraint.
 - <csr-id-ec7bf71b60f8c1e7529d610557c0305d624c1253/> signature name and email resolution
   The name and email for the author and/or committer may come from different
   config files. For example, user.name may be set in the global config and
   user.email may come from the repository local config.
   
   This case was broken due to Personas.from_config_and_env() only looking in
   the last config section containing, for example, a "user" section. Thus if
   the user.name and user.email are split across multiple sections (i.e.
   originating from separate config files), the fallback name and email
   ("gitoxide" and "gitoxide@localhost") would be used.
   
   The solution is to use gix_config::File::string() to lookup the name and
   email separately. The string() method correctly resolves the value by
   looking through all sections from all files in the correct order.

### Other

 - <csr-id-80dcb406c5f588122531da115398094de3c3af79/> name and email from different config sections
   The user.name, user.email, author.name, author.email, committer.name, and
   committer.email configuration may come from different sections from
   different config files. This new test exercises a couple of scenarios that
   are currently broken.

### Reverted (BREAKING)

 - <csr-id-87abb51596bd0a5a6b552a5de98a920d6c797e3c/> `committer_or_default()`, `author_or_default()` and `user_default()`.
   This means that all methods that previously succeeded by adding a default
   will now fail.
   
   This is preferable over 'doing something' and also admits that gits
   guesswork that tries to find user information by querying the system
   is nothing we want to repeat.

## 0.31.0 (2022-12-30)

<csr-id-9fabfc50007603f9c1f7e70b5bb79a39726b12af/>
<csr-id-91720798889ee7eb26da03a9e732caedda83b3e3/>

### New Features

 - <csr-id-d48b9a7ae2d51676c7549377bcb0b9d3baa83681/> fetching `ssh` urls can ask for a different username.
   If authentication fails, the user will be queried for a different username
   to try authentication via ssh mechanisms again.
 - <csr-id-61d89f586a0ad913fc2f502520282520a5e1fd15/> collect ssh-specific options to control how the ssh program is invoked.
   These are passed through when creating the ssh transport.

### Other

 - <csr-id-9fabfc50007603f9c1f7e70b5bb79a39726b12af/> explain how it's possible to deal with the first commit when comparing trees
   The reason the other tree isn't an option is that it's a special case that can more easily be handled
   with an `.unwrap_or_else(|| repo.empty_tree())` (or similar) for those who need it.
   
   Making the empty tree explicit also helps to deal with diffs from the empty tree (which can't be `Option<Tree>`)
   to the first tree of the first commit.

### Chore (BREAKING)

 - <csr-id-91720798889ee7eb26da03a9e732caedda83b3e3/> upgrade to prodash v23

## 0.30.2 (2022-12-26)

<csr-id-114f184855b6177aa1f0dbf6e6589f23deb5ffe6/>

### New Features

 - <csr-id-38ae61a805bd8cca5df8d1c1dcf3a8a0f9c85f5a/> make more HTTP options available
   - `http.schannelCheckRevoke`

### Other

 - <csr-id-114f184855b6177aa1f0dbf6e6589f23deb5ffe6/> provide a repository clone example

## 0.30.1 (2022-12-22)

### New Features

 - <csr-id-ca84c87734804cbfc65e311b89ff6ccfc236149c/> `open::Options::open_path_as_is()` allows to avoid 'smart opening' to try the path verbatim.
   The path to git repositories is well-known as they either end in `.git` or `.../.git`.
   If this is not the case, by default we append `/.git` to the path.
   
   With this new option enabled, no path transformations apply to open the given path as is,
   which is preferable if you know it's a non-standard git repository folder name.

## 0.30.0 (2022-12-19)

<csr-id-fceee748c114b2d0760074e911e533cd020f6996/>

### Changed

 - <csr-id-a4ac9cf3e667a3059e33aac8188150529578622d/> represent `GIT_(COMMITTER|AUTHOR)_(NAME|EMAIL|DATE)` with git configuration.
   That way it becomes more obvious where values are coming from.

### New Features

 - <csr-id-1683a848459cae2b9182b365e3e22b0e8ba73534/> expose `gix-features` crate at root under `features`.
   That way application developers can use more of the utilities
   that power most of the `gitoxide` plumbing crates.
 - <csr-id-90ef6fc36b440cc4baf3fde4a30060f1b4a0c8cf/> `Remote` knows about its `tagOpt` configuration.
   That way it's clear if it should or shouldn't fetch included/reachable
   tags automatically.
   
   The default setting for this is to include tags, similar to `git`.
   
   The `fetch_tags()` accessor allows to query this information, and the
   `with_fetch_tags()` builder method allows to set the value comfortably
   right after creating the `Remote` instance.
   
   The `tagOpt` key will also be written as part of the remote's git
   configuration.
   
   Clone operations can set the `Tags` setting when configuring the
   remote in a callback.
   
   This also comes with a fix to assure that ref-updates aren't skipped
   just because there was no pack to receive. That way, locally missing
   refs or tags will automatically be put back.
 - <csr-id-28e48083052216ddf1fd1f187cc22d506d3d9f86/> network related Error type support `is_spurious()` method.
   That way the caller can determine more easily if it makes sense
   to try again.
 - <csr-id-457c2e081b1aa5dfaab3f663b9aba66c16369939/> Make `prodash::tree` available as `progress::tree`.
 - <csr-id-d1b7ec605f8016c52c088477b6b0c5adf7ea0ab2/> read worktree specific configuration to override the one from the shared repository.
   This is intensively used when space checkouts are created, along with
   Cone mode. Thus it's the basis for properly interpreting sparse checkout
   options which are set on a per-worktree basis.
 - <csr-id-fc64693d5af0fda402c560d10d15652c24d14219/> add `permissions::Environment::http_transport`.
   That way it's possible to deny using environment variables that affect
   the HTTP transport, like setting the proxy.
 - <csr-id-0ce29a965cf16273cf74bd22e40f464e322e2f62/> `open::Options::modify()` as general pattern to allow builder methods usage in `&mut self`.
   That way it's easier to configure both the `full` and the `partial` trust instances
   of discovery options.
 - <csr-id-8482f90d0a2b61259cd51ca4f40ce322e388cb34/> Add `Repository::commit_as(committer, author, …)` convenience method.
   That way it's, very much beyond convenience, possible to set the time
   of a commit.
   
   Many thanks to @epage for the suggestion.
 - <csr-id-c8835c6edae784c9ffcb69a674c0a6545dbb2af3/> upgrade to `prodash 21.1` and add `Ids` to all progress instances.
   That way callers can identify progress they are interested in, say, for
   selective visualizations.

### Bug Fixes

 - <csr-id-d659bda2e1b0fcab529df7af6467f063ae5d0dd7/> provide a clearer error message when trying to open a git repository that isn't one.
 - <csr-id-ff0332e815c228cc5cdfe58c3598ad261bb2879e/> http transports can now reuse a connection.
   This makes connections more efficient generally and `cargo` relies
   on that behaviour in their tests as well.
 - <csr-id-9079b9d2e5f7cc133c6f2b2c2e64245b150c7d74/> allow to open a `Repository` from if 'config' file is missing.
   In this case, treat it similar to having an empty repository configuration
   file and assume defaults everywhere.
 - <csr-id-40f7379b7a89f7fe6f916801384e9e65e5b85c57/> improve error verbosity when fetching and cloning
 - <csr-id-b77fc86ab580dd81b08022996f07cc7925104e77/> `tree::diff::Platform::for_each_to_obtain_tree()` now properly surfaces user provided errors.
   Previously it would squelch them unintentionally.
   
   First discovered via https://github.com/Byron/crates-index-diff-rs/issues/35.
 - <csr-id-5386eed6a13a32a850c59706b15d8988c67733ce/> when fetching from file://, don't upset windows by trying `d:/foo`, use `d:\\foo` instead.
 - <csr-id-363ac7a74ec841505b5fc7cc1b8fae11c0a63ea9/> `config::CommitAutoRollback` now implements `DerefMut`.

### Changed (BREAKING)

 - <csr-id-3c84cebc5997356ff5f531c6cc9567bdd9b83eb5/> default features are set to `max-performance-safe` to assure compatibility.
   Previously the `max-performance` setting might have caused issues during compilation
   or issues at runtime if libraries like `git2` are used in the same binary, and the
   new default feature settings maximizes compatibility so this won't happen.
   
   For best performance, however, one will have to activate the `max-performance`
   feature on top of that.
 - <csr-id-5fe6aa3f3f2f81d84f0d96e874e68a8bf4de1db1/> environment variable permissions are per topic.
   Now `Permissions` for environment variables are so that they
   are by topic instead of by prefix, by default. That way
   it's easier to allow or deny particular sets of related
   environment variables.
   
   The catch-all permissions by prefix are still present for all
   other variables that aren't contained in one particular topic.
 - <csr-id-49f39d6bb487c0254176a5082f2c7851b83952a1/> `open::ReplacementObjects` is removed in favor of two custom git-configuration flags.
   Now it's possible to map the environment variables `GIT_REPLACE_REF_BASE` and `GIT_NO_REPLACE_OBJECTS`
   to custom git configuration keys which can also be set, namely `gitoxide.odb.replaceObjectsRefBase`
   and `gitoxide.odb.noReplaceObjects`.
   
   Along with the possibility of disabling the usage of `GIT_` prefixed environment variables one
   reaches the previous level of control without making object replacement a special case.

### New Features (BREAKING)

 - <csr-id-f8a2bfb93dadbc64393135e0a447f3d76628509c/> `interrupts::init_handler()` can now be undone.
   This can be done by calling `deregister()` or `auto_deregister()` on the return value
   of `interrupts::init_handler(…)`.
   
   That way it's possible to temporarily do interrupt handling only while some methods
   that require it are running.
 - <csr-id-becbd8d896a1663f1607be4e86e632773e926f1f/> represent object cache configuration like `GITOXIDE_PACK_CACHE_MEMORY` in git-configuration.
   That way there is a unified system for how to set values, which may be overridable by configuration
   variables or not.
   
   With this changes, the explicit application of environment variables for setting the cache
   isn't required anymore as everything happens using git-configuration, and automatically,
   while providing full control like before.
 - <csr-id-f16e36168cc93768ba5d53c9848ff2e8432d06b1/> remove `SnapshotMut::apply_cli_overrides()` in favor of `open::Options::cli_overrides()`.
 - <csr-id-84d594caf3f04f1ce337e455343278a6f4674957/> more type-safety for remote names by making clear they can be named after URLs.

### Other (BREAKING)

 - <csr-id-fceee748c114b2d0760074e911e533cd020f6996/> `Remote::with_refspec()` to `Remote::with_refspecs()` to allow adding more than one refspec as part of the builder.

## 0.29.0 (2022-11-21)

<csr-id-f302fc1bcd06fadccd126f4f5f9c0165afabedda/>

### New Features

<csr-id-ff9e1571b558475e727dc6ba11dab24ef15fb6f4/>

 - <csr-id-3ddbd2de369b521fa3f21935f10fe9c248840893/> Make `reqwest` TLS backend configuration easy.
   We provide the choice of `native-tls` or `rust-tls`. If none is
   provided, the user can configure on their on similar to how it's done
   in `gix`.
   
   Please note that a choice now has to be made or HTTPS will not be
   available, so use one of…
   
   * blocking-http-transport-reqwest-rust-tls
* blocking-http-transport-reqwest-native-tls

### Bug Fixes

 - <csr-id-c6a690219915b2b401d2d11f61db35b2931e5b3a/> `gix::Commit::describe()` chooses tag names (more) correctly.
   Previously, if there were multiple choices for tags on the same commit,
   `git describe` would disagree with `gitoxide` due to different
   prioritization of names.
   
   This has now been fixed.
 - <csr-id-84ed89c3bf6692f18c4bb97173527de1bcba7ac6/> also sort entries lexicographically

### Other

 - <csr-id-f302fc1bcd06fadccd126f4f5f9c0165afabedda/> Set GIT_EDITOR in make_rebase_i_repo.sh
   If the user has core.editor set in their global git config, then that value
   takes precedence over the EDITOR environment variable. The GIT_EDITOR
   environment variable, however, has higher precedence than core.editor. For
   this test, using GIT_EDITOR ensures that the desired sed command line is
   used.

### New Features (BREAKING)

 - <csr-id-bc2a399f2fbb69d23b0b05e8dfb95f3c64ff93b9/> rename `blocking-http-transport` feature to `blocking-http-transport-curl`; add `blocking-http-transport-reqwest`.
   With the new and relatively immature second tier http backend we pave
   the way to support builds without the use of open-ssl and probably many
   other C libraries.
   
   Note that it's early and not `http` configuration option is implemented
   yet.
 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `gix-features` and `gix-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.

## 0.28.0 (2022-11-17)

<csr-id-6beb6f263fd40884b440092f39034dd43d3a95de/>

### New Features

 - <csr-id-58e14884b1d025651f874d899cb2d627c4a2afbf/> `Id` implements `std::fmt::Display`
 - <csr-id-25f7aabe38267b6b6c0547806028b2becb806416/> `Remote::repo()` to obtain the underlying repository.
   For convenience.
 - <csr-id-709a73229b7cde56ddffa099158661632c606468/> Support for user-customizable user agent strings.
   Doable by setting the `gitoxide.userAgent` variable.
 - <csr-id-e60d07997989993216c2bd93efeb6f1b48da0a87/> add `env::agent()` for obtaining the default client agent string.

### Other

 - <csr-id-6beb6f263fd40884b440092f39034dd43d3a95de/> try to apply maybe-async in a place where it's probably not possible.
   The goal is to re-use the existing tests, but right now they only
   compile in async mode as the `maybe-async` crates needs
   a feature to be set. Doing so is hard(er) if it's not already used
   in the main crate, which we do not and will do our best to avoid.

### New Features (BREAKING)

 - <csr-id-db9040f0bb3a16879c8da0252a77df80bd417387/> add `remote::Connection::with_transport_config()`, change the way `*::transport_mut()` is used.
   Previously `transport_mut()` was supposed to be used for calling
   `configure()`, but that doesn't work anymore as `configure()` can
   only effectively be called once the initialization of the Connection
   is complete, as it may depend on the Remote name AND the credential
   provider for proxy auth credential acquisition.
   
   Thus we allow callers to set the transport options they need in advance
   for it to be used when needed.

## 0.27.0 (2022-11-08)

### Changed (BREAKING)

 - <csr-id-c50868c7ed7309515b4f0a188213d332d57dd146/> Move `object::tree::diff::change::DiffPlatform` to `object::blob::diff::Platform`.
 - <csr-id-4ee32713093c2e41a12d148c6030add1df6aa966/> new `DiffPlatform::counts()`, open `DiffPlatform` for use of `gix-diff::blob::*`.

## 0.26.0 (2022-11-06)

<csr-id-c6f92c15529ddff7539667b74bafa2348f3040e3/>

### New Features

 - <csr-id-b1edb9e3537df86669714f03666f4a88e0ac3709/> diff algorithm is controlled by git configuration `diff.algorithm`
 - <csr-id-072f5bc9c91c4c09bd6a73f9d7ac672805cae533/> Query of `core.logAllRefUpdates` is now fallible.
   This is the same behaviour as shown by `git`, which requires valid
   values or aborts.
 - <csr-id-2eaf69e5f8f8da10e5af85cb9f0c39577ad1707f/> automatically handle `.keep` files after writing a pack bundle to disk.
   The logic implemented here tries to do the right thing, that is when
   we have reason to believe that the objects in the pack are linked up
   with a ref, we delete the keep file automatically.
   
   However, if there was no local ref edit as the ref specs didn't contain
   local destinations, or if the pack was empty, then keep the .keep file
   and leave it to the caller to handle.
 - <csr-id-8b9fbd4e9ed7be37976c7203cd9a89c6116a6d3d/> Use `core.askpass` when building the credential helper.
   Previously it would only consider the environment variable, which can
   still override the provided git-configuration at core.askpass .
 - <csr-id-a9d14492322785a14f4ecb5b0d3dbdc87e56f8c5/> `remote::fetch::Prepare::handshake_outcome()` to obtain server information right after listing refs.
 - <csr-id-0b5c53ec43bdb58b2b7cf46e453ddf858770a95a/> `open::Options::config_overrides()` for early configuration; support for `init.defaultBranch`.

### Bug Fixes

 - <csr-id-f869b224170b0c49a0e4d89e88bfbf5caedaa725/> don't allow non-bare repositories to be initialized into non-empty directories.
 - <csr-id-91baefad02a0d52c745106359da3693d06aace46/> `init_bare()` now creates the destination directory if it doesn't exist.
 - <csr-id-5c11b84f4e74e3eefdd0f5804976ebfc505e0f2f/> build correct path for `$HOME/.config/…` files.
   The special per-user `ignore` and `attributes` files can also be
   defaulted if some environment variables are set and may be accessed.
   
   Previously the default for `$HOME` was incorrect, as it was missing the
   intermediate `.config/` directory. This is now present to build paths
   exactly like git.
 - <csr-id-275e80f3d602b63ef91efe31a92b4aafb2eeca44/> ref-map filtering now uses correct prefixes.
   Previously specs could get filtered out server-side as a matching prefix
   was entirely missing.

### Changed (BREAKING)

 - <csr-id-449ff066d2b5dd423c639618193dd9e54d03c1f8/> `Repository::branch_remote_name()` returns `reference::remote::Name`.
   That way it's made clear the remote can also be a URL, while rejecting
   illformed UTF8. The latter isn't valid for remote names anyway as these
   only support a very limited character set.
   
   Note that this error currently is degenerated, making it appear if the
   remote name doesn't exists if illformed UTF-8 is found in what appears
   to be a symbolic ref.
 - <csr-id-71f15fc46fbaea455cf84a2b4cfe3e680047d790/> be specific about the kind of `diff::Error`, moving it to `diff::for_each::Error`.

### New Features (BREAKING)

 - <csr-id-7413a284eb7754e63ba45d0f526347b9f79b557d/> `Tree::lookup_entry*()` returns attached `Entry` type.
   That way chaining gets even easier.

### Bug Fixes (BREAKING)

 - <csr-id-2bece79285e244a7029f9393dafc990e39420e2d/> `create::into(…)` takes `create::Kind` to determine if it's bare or not.
   First of all, `bare` is not an option anymore, but a parameter as
   it can't be defaulted.
   Other public signatures change as well to accommodate for it.

### Other (BREAKING)

 - <csr-id-c6f92c15529ddff7539667b74bafa2348f3040e3/> `DiffPlatform::text()` to `*::lines()`.
   This is more specific as one could also do character changes in a single
   line, and it adapts the signature to the new `imra-diff` powered
   mechanism, for a 2x speed boost.

## 0.25.0 (2022-10-10)

<csr-id-5bef0a00e8d01110c054a517f6d9696f981a7efc/>

### New Features

 - <csr-id-22d3b37ea6239170a478b859361a7d1d7ba01a9a/> `Url::try_from(path: &std::path::Path)` for more convenient instantiation.
 - <csr-id-31a7089f2583832727e2175ada6fb5c30c3beebe/> make some private methods public to give callers more flexibility.
   This allows to implement the fetch-negotiation part oneself and break
   free from constraints of the delegate.
 - <csr-id-4367994a8a7476eb44e1309e833e345fdb78f246/> add `config::SnapshotMut::commit()` to make clear it's transactional.
 - <csr-id-d2bea003230078ffb4e6cd80d1b01c3995435a34/> add `config::SnapshotMut::forget()` to forget all changes before applying them.
   The documentation was update to make clear when the changes are applied.
 - <csr-id-4b1e3b3d91c51da3dbea9191e60f959a1266cf47/> add `Repository::find_default_remote()` which works on detached heads as well.
 - <csr-id-25f06400c49ddd1688fd76f9285542b121b223b4/> `Remote::rem_map()` now specifies ref-prefixes to the remote.
   This can greatly reduce the amount of refs sent.

### Other

 - <csr-id-5bef0a00e8d01110c054a517f6d9696f981a7efc/> try to make the transport configurable after being boxed, but…
   …that would force it to be 'static, which is something we explicitly
   cannot have. We need references to be contained within, if I remember
   correctly.

### Changed (BREAKING)

 - <csr-id-e88de0f948325773db1925b07aa878e1dbb76bad/> All methods editing references don't take the author as parameter anymore.
   Instead, these are taken from the git configuration and can be
   configured on the fly with temporarily altered configuration.
 - <csr-id-3a0fb1b45c757add49677450836c0aaf6179a2b5/> remote `lock_mode` from all methods dealing with reference edits.
   It is now read from `core.filesRefLockTimeout` accordingly.

### New Features (BREAKING)

 - <csr-id-3b29fc18672c0176684c797a0f16f85d09369bf8/> make jwalk fully optional
 - <csr-id-78ad3df64f2c016ba17b158bd9ab1d2341aab399/> add `fetch::Transport::configure` to generically configure any transport.

## 0.24.0 (2022-09-20)

<csr-id-f5959edc1477573278afcfe23e9e52ddaacb37db/>
<csr-id-79c22557ce0aea1ee8f3a58192c2c76087ccd3d8/>

### New Features

 - <csr-id-0871a96b9cc84d7a496d39393e081999c0a3fe17/> `Object::peel_to_tree()` as convenience method.
   It's very common to try to work with trees, so let's make that easier.
 - <csr-id-1027be960852618915014f9ba6e6632bd4999b0e/> `interrupt::Iter` now allows accessing the inner iterator without consumption.
   This is useful if these provide additional out-of-band information.
 - <csr-id-8c2e5c60f9f5f8d0859ecd84c17af20e88812512/> Once a change is obtained, it's easy to obtain changes line by line.
 - <csr-id-7e96d1841989b37133cddf334724a2d6df557e69/> obtain a refmap after listing refs via `remote::Connection::list_refs_to_map()`.
   With it it's possible to establish a relationship between what's about
   to be fetched to local tracking branches as established by refspecs for
   fetching.
 - <csr-id-d51e7c901fe5ed60d5dd56006c5faedb71cad537/> Add `permissions::Config::git_binary` field
   When true, default false, inject the git installation configuration file
   if present at the cost of one `git config` invocation.
   
   Note that we rely on the underlying `gix-config` crate to not load
   duplicate files.
   
   We also currently lie about the scope which is actually unclear - have
   seen 'unknown' or normal scopes like `system`.
 - <csr-id-1c13f1125664fbcc276a1ca440d168d07d0bf493/> add `prompt` to top level forwarding #450)

### Bug Fixes

 - <csr-id-ae3866065c9c3c6d01709f8dde1cea1ae1345779/> rev-spec parsing can now handle the empty tree as full hex hash.
   Even though the empty-tree object can be found when searched via
   `Repository::find_object()`, previously it was not locatable when
   used during rev-spec parsing.
 - <csr-id-74ede2031d1beedf11f1cdf006fff71e597a2cb5/> `Reference::remote()` can produce remotes for URLs

### Refactor

 - <csr-id-f5959edc1477573278afcfe23e9e52ddaacb37db/> use specific error type for `rev_parse_single()`

### Changed (BREAKING)

 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

### New Features (BREAKING)

 - <csr-id-2992b1ba4e7bbeab26f41175cd31fd664abf2a11/> Add reference remote name type to make usage of `remote_name()` result clear

### Other (BREAKING)

 - <csr-id-79c22557ce0aea1ee8f3a58192c2c76087ccd3d8/> `Tree::lookup_path()` -> `Tree::lookup_entry()`.

## 0.23.1 (2022-09-01)

### Changed (BREAKING)

 - <csr-id-36d8c57824a2b921012439105e49261fac66c955/> Remove 'unstable' feature.
   It's not worth maintaining it especially since everything is in
   pre-release mode right now.
   
   It might be something to re-introduce after go-live.

### Bug Fixes

 - <csr-id-d18e76cfb512ef7fe5bfee6e87726372c6a4a8b6/> `max-performance-safe` mode does not include zlib-ng adjustments anymore.
   git2 cannot handle this and fails to fetch packs after a couple of
   seconds.
   
   It's unclear what is causing this except that git2 doesn't like libz
   with zlibng support enabled, which happens if git2 in the
   same tree is with us.
 - Transitively through a kindly contributed fix in the `gix-discover` crate, `Repository` can now be opened on `exFat` volumes.

## 0.23.0 (2022-08-28)

### New Features

 - <csr-id-70aa850591de268488ae9bf2d3839a5c9c543c35/> The empty tree can always be returned by `Repository::(try_)find_object()`
   This matches the behaviour of git and libgit2.
   We consciously chose to only do this on the highest level, allowing lower
   levels to determine if the object exists or not.
 - <csr-id-8d0786646e17a82d20ca6b2799b54f6349d389f4/> Make `find::object::*::Error` publicly available.
 - <csr-id-2d0b63997b276a53b3cf8f09fac51f8e3f044bcd/> Add `Reference::delete()` for simple reference deletion
 - <csr-id-9170562059c3eaa529850025ef35ac5ffffc0fdf/> `Reference::set_target_id()` to easily set the target id of a reference
 - <csr-id-950da602925e6376b08640ed3ebfdf407394db34/> `Reference::head_ref()` to quickly access the reference the head points to.

### Bug Fixes

 - <csr-id-2834311b4f262c57e76627addaa4932196fd26b3/> `Commit::tree_id()` now returns a connected id

### New Features (BREAKING)

 - <csr-id-e090f843f5cffc8e8e47a8cac2e6fb98e4c47771/> `gix-diff` is now included by default as part of core functionality

## 0.22.1 (2022-08-24)

A maintenance release without user facing changes.

## 0.22.0 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

 - <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-c28bcec19b5526acf888f928e6ddc4671873368e/> support avoiding usage of `fast-sha1` in gix-features separately.
   That way one has an angle on compile failures in client libraries,
   see https://github.com/o2sh/onefetch/pull/752 for motivation.
 - <csr-id-4f87a0672f7288486a9b6403c0bb07a6279d2cfe/> `Repository::write_blob[_stream]()` to more easily write blobs.
   That way, one won't have to use the underlying `objects` database but
   can remain in the land of `Repository` enabled types for longer.
 - <csr-id-d35cd2a12c6db3d655ce10cad5c027bde99e19b4/> `SnapshotMut::apply_cli_overrides()` to make it easy to support things like `-c`
 - <csr-id-2a839f3209f3bd35e0c0f7edff664cc953059f65/> `Repository::config_snapshot_mut()` to mutate configuration values in memory.
   It's a first step towards writing changes back to disk, which can work
   already, but probably wouldn't as we currently don't localize changes
   to only one section type, i.e. Api, but instead may change values
   from other sections.
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs

### Bug Fixes

 - <csr-id-ff71730b4e3635533d9969d9dd44c0f3c75c6648/> Don't fail worktree tests if the system's git version is not supporting required features

## 0.21.1 (2022-08-19)

A maintenance release that speeds up `commit.describe()` performance if `max_candidates()` is 0.

## 0.21.0 (2022-08-17)

<csr-id-b38a212459e2646ab97ad7b5c24e54d962aae960/>

### Changed

 - <csr-id-0235111a4fcc40c7b57d973bfce27a66eddea901/> Invert behaviour to `open::Options::strict_config()`, with lenient being the default.
   This means API users will get libgit2 behaviour but commands like `gix` can
   change options to emulate `git` behaviour.

### New Features

 - <csr-id-a01525d159a33d6ad60a5324f2e9abbbe17fcfad/> `Kind` can now represent submodules.
   This should complete the list of git repository types and flavors.
 - <csr-id-5dac021bbbc5621167e7f49d62b11f68f76e42b6/> `open()` and `discover()` support opening submodules.
   This includes submodule checkouts as well as their original module git
   directories.
 - <csr-id-067c3342f3564dd7f152a720e93e3aa590ae6524/> `open::Options::lenient_config(…)` to default otherwise invalid configuration values where possible
   Originally required by https://github.com/starship/starship/issues/4266 .
 - <csr-id-0bf8371706d319681c3f794af5cd13f2f50a27d0/> support core.worktree option
 - <csr-id-b47bbb787ef2e31dd2612a56f9e7759ef8a188b8/> display for `object::tree::EntryRef`
 - <csr-id-727768a49c41165b03ddcdbc71ca88b66c330f32/> `Head::prior_checked_out_branches()`
 - <csr-id-ffe72918baf5c4c9f0f3709f75f068a663778588/> `Repository::index()` and `Worktree::index()`.
   These methods provide a possibly updated shared index.
 - <csr-id-47619f7c06a49dcf60a30e1a43a5352914183092/> add `Repository::object_cache_size_if_unset()`
 - <csr-id-d2611cee61841bc7bd978bef5af7dc66154248a2/> `Commit::message_raw_sloppy()` to provide yet another way to obtain a commit message.
 - <csr-id-906c95845fa4aa2d4390c522bb566a188b8c0e78/> add `rev_spec::parse::ObjectKindHint` to support `core.disambiguate`.
   The latter is seemingly undocumented in the typical place, gix-config.
 - <csr-id-ef187f0180d89544d9015cbc2bc03d8cb51f4615/> `Remote::with_refspec()` to add new unique refspecs
 - <csr-id-d51ba42c643d8ee03a3c6b648f8524ff04827170/> `Remote::push_url()` to set it after the fact
 - <csr-id-9b07b91ad065836e7473df6635025659af2865ee/> `Repository::remote_at(…)` to create an unnamed remote
 - <csr-id-a67fc26b80e5d1183ddc5c6598396214f3e19945/> more conversions for `TryFrom`: `String` and `&str`
 - <csr-id-7a512ecdf236afc0b3d562d60fa81ab62c00cd9d/> `Head::into_remote()` to try really hard to find the correct remote
 - <csr-id-f392f26bec6069ac43ecd461b4f50e0def8b8972/> `Repository::remote_default_name()` to obtain the repo-wide remote for a a direction.
 - <csr-id-f47464f64f7c21500a24f563b25a8fc070c41778/> `Repository::branch_names()` to obtain branch names for which configuration exists.

### Bug Fixes

 - <csr-id-be6114e7c4ac48467db6acb2180b443dc9f59f32/> assure permissions per trust level are properly inherited into `open::Options`.
 - <csr-id-270242c707bd086b7746ee45b55791587f1484b1/> provide additional explanation about when to use `open::Options::with()`

### Refactor

 - <csr-id-b38a212459e2646ab97ad7b5c24e54d962aae960/> embrace `revision` module and move `rev_walk` there.
   Let's embrace the idea of structured modules and platforms in the right
   spot in the module hierarchy instead of forcing known names on it that
   over-simplify.

### Changed (BREAKING)

 - <csr-id-0deda0df55c11647f51374ed5a8bf11c932e2bae/> remove `permissions::Config::strict()` as they were unused internally.
   Furthermore, they were allowing everything as before so better not to
   have it.
 - <csr-id-1c12d49eefa6d79ef50b2960f41b29de680ac8eb/> rename `Repository::load_mailmap*` to `Repository::open_mailmap*`.
   For consistency with other similar methods.
 - <csr-id-ea35183b53f2ff71bdf2270ac4f7470a85d7756f/> remove `Repository::load_index()` in favor of `repo.worktree().open_index()`.
 - <csr-id-4fd096840ba27da6ce86678a85ede33e3be974ff/> `gix_revision` is now available in `revision::plumbing`.
   That way it won't clash with the higher-level constructs on top of it
   which use the same names.
 - <csr-id-2424957cff75daacf6f6f14f74b9639f6875c4fb/> Turn `id::Ancestors` into general-purpose `RevWalk`.
 - <csr-id-1df379ab0046887a330c0a670ad0414e79cfae7b/> remove `Permissions::git_dir` field entirely.
   It was meant to help dealing with bailing out if the git dir isn't
   fully trusted, but the way this was done was over-engineered especially
   since the read-only permission level wasn't implemented at all.
   
   That function is now performed by a new flag, the `bail_on_untrusted`
   which is off by default.
 - <csr-id-5ab81ece15ec802ad4328ce31304233bd25b2929/> rename `Repository::remote_ref()` to `::branch_remote_ref()`

### New Features (BREAKING)

 - <csr-id-e2aff28e818951785d933f4b55b2f1b882729cb6/> `Repository::rev_parse()` returns a `RevSpec`.
   This lays the foundation for actually handling rev-specs faithfully.
   Previous users should use `rev_parse().single()` to obtain a single
   object id which was the only supported usecase previously.

### Bug Fixes (BREAKING)

 - <csr-id-c68b125a46f666700cdbda6f8cd39a044f4feb1b/> Don't panic for `@{1}` in new repos; rename `Head::into_referent()` to `::try_into_referent()`
   The signature change will prevent such issues in the future as one
   cannot simply ignore new repositories.

## 0.20.0 (2022-07-22)

### New Features

 - <csr-id-1b765ec6ae70d1f4cc5a885b3c68d6f3335ba827/> respect `safe.directory`.
   In practice, this code will rarely be hit as it would require very
   strict settings that forbid any operation within a non-owned git
   directory.
 - <csr-id-840d9a3018d11146bb8e80fc92693c65eb534d91/> permissions for configuration.
   It provides fine-grained control over what sources to load.
 - <csr-id-657080829867d9dcb0c9b9cb6c1c8126c4df3783/> `gix-config` is now accessible in `git-repository::config`.
 - <csr-id-d99453ebeb970ed493be236def299d1e82b01f83/> `gix config` lists all entries of all configuration files git considers.
   Filters allow to narrow down the output.
 - <csr-id-ebedd03e119aa5d46da07e577bfccad621eaecb5/> repository now initializes global configuration files and resolves includes
 - <csr-id-de8572ff2ced9422832e1ba433955c33f0994675/> resolve includes in local repository configuration
 - <csr-id-d5a48b82230b047434610550aacd2dc741b4b5f0/> `config::Snapshot::trusted_path()` to obtain trustworthy paths.
   We also apply trust-based config query during initialization to assure
   we don't use paths which aren't owned by the current user.
 - <csr-id-5f9bfa89ceb61f484be80575b0379bbf9d7a36b3/> `Repository::config_snapshot()` to access configuration values.
 - <csr-id-7f67b23b9462b805591b1fe5a8406f8d7404f372/> Use `gix-config` to write config file on initialization, including `logallrefupdates` and `precomposeunicode`.
 - <csr-id-e263e13d312e41aa1481d104fa79ede509fbe1c5/> respect `core.logallrefupdates` configuration setting.

### Changed (BREAKING)

 - <csr-id-68f4bc2570d455c762da7e3d675b9b507cec69bb/> Make `SignatureRef<'_>` mandatory for editing reference changelogs.
   If defaults are desired, these can be set by the caller.
 - <csr-id-f932cea68ece997f711add3368db53aeb8cdf064/> `Repository::committer()` now returns an `Option`, see `::committer_or_default()` for a method that doesn't.
 - <csr-id-89a41bf2b37db29b9983b4e5492cfd67ed490b23/> remove local-time-support feature toggle.
   We treat local time as default feature without a lot of fuzz, and
   will eventually document that definitive support needs a compile
   time switch in the compiler (`--cfg unsound_local_offset` or something).
   
   One day it will perish. Failure is possible anyway and we will write
   code to deal with it while minimizing the amount of system time
   fetches when asking for the current local time.
 - <csr-id-6f4eea936d64fb9827277c160f989168e7b1dba2/> Associate `file::Metadata` with each `File`.
   This is the first step towards knowing more about the source of each
   value to filter them based on some properties.
   
   This breaks various methods handling the instantiation of configuration
   files as `file::Metadata` typically has to be provided by the caller
   now or be associated with each path to read configuration from.

### New Features (BREAKING)

 - <csr-id-d003c0f139d61e3bd998a0283a9c7af25a60db02/> Support for `lossy` load mode.
   There is a lot of breaking changes as `file::from_paths::Options` now
   became `file::init::Options`, and the same goes for the error type.
 - <csr-id-311d4b447daf8d4364670382a20901468748d34d/> change mostly internal uses of [u8] to BString/BStr

## 0.19.0 (2022-06-13)

### New Features (BREAKING)

 - <csr-id-266d4379e9132fd7dd21e6c8fccb36e125069d6e/> Make `realpath()` easier to use by introducing `realpath_opt()`.
   That way there is consistency about how many symlinks to follow.

## 0.18.1 (2022-05-23)

### New Features

 - <csr-id-c78baecbb37fd92a0a86231810c9e35e9a4c21cd/> `Debug` for `Reference`.

## 0.18.0 (2022-05-21)

<csr-id-e63e722791a7795cd99048bed834459595c60abc/>

### Other

 - <csr-id-e63e722791a7795cd99048bed834459595c60abc/> add ceiling_dirs option to upwards discovery

## 0.17.0 (2022-05-18)

<csr-id-53c06c7e6a3003b34edaab10db1f158e2fb57403/>
<csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/>
<csr-id-da8059ce26343c8cd275f43c879d98c92f77fa51/>

### New Features

 - <csr-id-45920da7c8c5618c6e7258de08dbd633a637d017/> Add `Repository::head_name()`.
   A convenient way to obtain the name of a head, if not detached.

### Bug Fixes

 - <csr-id-a1680b44ef568317465d2971da6e0930f9885530/> `Commit::describe()` now returns annotated tags before lightweight ones and prefers more recent ones as well
 - <csr-id-99365f221065ebc315ac80940ad72cae253743bc/> Support for in truncated history in git-describe
   This allows `describe()` to work on shallow clones.

### Other

 - <csr-id-53c06c7e6a3003b34edaab10db1f158e2fb57403/> allow reading information about remote branch
 - <csr-id-e4f4c4b2c75a63a40a174e3a006ea64ef8d78809/> `path::discover()` now returns the shortest path.
   If and only if it canonicalized the source path. That way, users will
   still get a familiar path. This is due to `parent()` not operating
   in the file system, which otherwise would be equivalent to `..`,
   but that's not how we work.
   
   Maybe we should overhaul the way this works to use `../` instead
   and just 'absolutize' the path later (std::path::absolute()) is
   on the way for that.
 - <csr-id-da8059ce26343c8cd275f43c879d98c92f77fa51/> remove unused variant

### Changed (BREAKING)

 - <csr-id-80e8fd4a5944890f43f3d888b7a73bb26351b195/> integrate trust model into repository discovery
   That way it's possible to ignore repositories which effectively
   aren't owned by the current user, or to not ignore them (default)
   but assign tighter permissions to the repository.
 - <csr-id-2e39b0ede98826e6f85c56fef77ac65a5b7e7ac2/> `path::discover::existing()` -> `path::discover()`
 - <csr-id-38dfdcf80f9b7368ccaa10f4b78b2129849848d0/> remove `values::*Error` in favor of `value::parse::Error`.
   This makes it easier to work with errors in practice, we are either
   interested in the value that failed to parse to try something else
   or want a nice user message.
   
   Having one decode error type facilitates that.

### New Features (BREAKING)

 - <csr-id-32dc1829a5661f66396d109c8d0a8eaae6b1f532/> use `gix-credentials` in `gix-protocol`

## 0.16.0 (2022-04-05)

### New Features

 - <csr-id-47556f6815148ed960a727fd122f7162345544c3/> auto-calculation of a good hex-len, like what git does
   If the `core.abbrev` value isn't set or is set to `auto`.
 - <csr-id-654f4afb794a370b7cd9d9502ff6d0c3378ec417/> `Commit::describe()`
   A way to fluidly configure a `git describe` operation and run it.
   
   Along that, a new `Tag` top-level object was added as well to provide
   convenient access to otherwise lower-level objects. It's not strictly
   required for our implementation here but it's needed for a symmetric
   API.

## 0.15.0 (2022-04-03)

<csr-id-5f7595305efc85d6ca3c541e9f9adac3915cbd84/>
<csr-id-c10f07c50f6dde4b39bf1e3ff26c239c5f202912/>
<csr-id-bbc6efeceb26050973e1425e68a52e51b9df4572/>

### New Features

 - <csr-id-1322dbf6827ea5cc1d71175afb669e01fb1242ef/> support for object replacement
   The Repository now respects replacement refs created by `git replace`
   and picks up environment variables for its configuration as well.
   
   Which environment variables are used is fully configurable.
 - <csr-id-a39bf71531ee0a6c8db082758d3212c805ce2bf0/> support for trimming of whitespace around name and email
   It's separated from parsing to assure we can round-trip, but it's
   made easy to obtain trimmed results using new methods.
   
   This high-level git-repository will also trim by default now.
 - <csr-id-00578040a699e1939b3d3813616d3cc4e1d8669e/> `Repository::head_commit()`
   A shortcut to get to the commit much faster.
 - <csr-id-def80df2e165b74f4b053e4030f563902b7d34a4/> `ein tool estimate-hours` now supports mailmaps
 - <csr-id-f0d8a49587c08713350252e1701a45bb308b6f9d/> `Repository::head_id()`
   A long-needed shortcut.
 - <csr-id-d2388d8d80f379eccc9ee84ebe07acd67d154630/> `gix repository mailmap entries`
 - <csr-id-e3bc1b410409a9e27894a5cac48b06d8c3295e36/> unstable mailmap module
 - <csr-id-1be00cf9e00ce9428ffddb2c79b2373926069b13/> `Commit::short_id()`
 - <csr-id-c7dff9e8b695d298a3fb21f19f51752a885a5ce3/> in-manifest and in-lib documentation of feature toggles
 - <csr-id-9f5663ed83d83c7335b346313837d4cada9cd846/> `easy::Commit::time()` to access the committers time conveniently.
 - <csr-id-7c88b62e439af7a60ddb68fb6737cb3b1cebf00d/> easy::Head::name() to learn about the name of the HEAD ref
   It's mainly for completeness to provide people with with a `FullNameRef`
   of HEAD.
 - <csr-id-3b0913a2e6695e4e9e94341ef48d2ba3b4a518e6/> `easy::Head::peel_to_commit_in_place()`
   It allows to quickly get a commit from the head, something most people
   want when getting started with any kind of tool.
 - <csr-id-1c22d76c26464db4a185e19bb6c1f9a17fa19bc9/> `Repository::load_index()`
   This method makes the index of the default workspace available.

### Bug Fixes

 - <csr-id-c329dd75420f82d506fd415cd377f7df6c6ccbad/> Properly classify worktrees as non-bare, helps with `ein t find`
   They use git-files which point to the actual repository data.

### Changed (BREAKING)

 - <csr-id-a8b6589a7c645f323f95da6cb94321fc967e9b06/> Easier access to local and remote branches

### New Features (BREAKING)

 - <csr-id-8945d95f7fa88562d37ff67ac6e38bead73dd2df/> `interrupt::Iter`, rename `interrupt::Iter` -> `interrupt::IterWithError`
 - <csr-id-813a3bea88cdbe1fd9b0a8070efeee2a44f7823e/> Let 'easy::Object::try_into_…()` return `try_into::Error`.
   That way, the typical usage of `try_into_commit()?` will not result
   in a strange error about `Object` not being convertible into some
   error. We think having a real error there is the least surprising.

### Bug Fixes (BREAKING)

 - <csr-id-c863ea5b34fa9ee3dac21c1f85587da16045f8d8/> do not install signal handlers by default
   The previous behaviour is meant to be convenient for the casual
   user even though it
   ends up being surprising when used in applications that install
   their own signal handlers and need more control over how the program
   shuts down.
   
   This is now fixed by **requiring an explicit `setup()`** call before
   the first tempfile is created, which makes it a breaking change.

### Other (BREAKING)

 - <csr-id-5f7595305efc85d6ca3c541e9f9adac3915cbd84/> `Id::prefix` -> `Id::shorten()`
   It's definitely more intuitive that way.

### Refactor (BREAKING)

 - <csr-id-c10f07c50f6dde4b39bf1e3ff26c239c5f202912/> dissolve 'easy' module by moving everything one level up
 - <csr-id-bbc6efeceb26050973e1425e68a52e51b9df4572/> clarify different repository types much better

## 0.14.0 (2022-01-23)

<csr-id-7a91212631219e94b9454d2874b53f3ecc1db77e/>
<csr-id-b2cc0c63570d45de032d63e62d94c3344783440e/>
<csr-id-ebc7f47708a63c3df4415ba0e702660d976dfb3e/>
<csr-id-2290d006705ff47ad780b009fe58ee422b3285af/>

### New Features

 - <csr-id-667485e133ca29fcc6914a7142cf953564b5fce3/> Add `easy::Tree::traverse()` platform
 - <csr-id-8f650c089c88698483f778aa5c0070f606b94f09/> Add `easy::Commit` object
   It allows to more conveniently access commit information.
 - <csr-id-0ae2a8da010d848d98bef47ac923ae1d770091ff/> `easy::Oid::ancestors()` now supports `sorting()` and iteration by first commit only
   Especially the sorting is useful to avoid having to sort commits by
   hand after collecting them.
 - <csr-id-bc77534f9c385046f6c9adb994b1443307afda46/> Use GITOXIDE_OBJECT_CACHE_MEMORY to control how much object cache is used
   Note that this is mostly for debugging or quickly seeing if object
   caches help with certain operations.
   
   Ideally the implementation knows themselves and sets up caches
   accordingly, probably after trying it with these environment variables.

### Changed (BREAKING)

 - <csr-id-6e3a745dfada66a2fcac256dae0ac63959e74d08/> rename `easy::Object` methods returning `Ref` objects to have `ref` in their name
   That way, it's more clear that the `Ref` versions are low-level ones
   whereas the `into_` ones are higher-level ones that are part of the
   `easy` suite.
 - <csr-id-b6730979808ce28b98c65888a349f1e3d0ea1b9a/> Rename `OwnedObject` to `DetachedObject`
   The latter more clearly indicates what the difference is to
   `Object` (which is attached and carries a lifetime)
 - <csr-id-c4184f3c31ffc4597bd089e8140653906a6594d8/> Remove easy::borrow::Error entirely; support for multiple objects per handle
   This massive simplification finally allows any amounts of objects to be
   created while adding support for reusing their data buffers thanks
   to a simple free-list stored with the handle.
 - <csr-id-880b56426859306aa30038ff35e2ad14607e9e90/> rename `easy::Object` to `OwnedObject`; remove `Ref` suffix from `ObjectRef` and `TreeRef`
 - <csr-id-f9c0493460ab7c664aaa231ffcf7dfd56076c920/> use `gix_odb::Find*` traits in prelude, instead of `gix_pack::Find*`
   These are higher-level and generally more desirable.
   The Find traits in `gix-pack` are more useful internally when packs
   have to be handled directly, for example when generating packs.
 - <csr-id-83d7b31e7dd6d09eea79fc3c68620d099459132f/> rename easy::State to easy::Handle
   As the first step to remove the 'Easy' abstraction.
 - <csr-id-5e7aa1689f5d7ea5b510611a3ca0868828226291/> fully rely on OdbHandle in repository State
 - <csr-id-57de915886b76f80b3641def0ccf4fd79e334fc8/> Rename `Repository::odb` to` Repository::objects`
   This way it's more inline with `Repository::refs`.
 - <csr-id-93db4a5e70456d2c33ea010e3c86e5f26eb1bcc0/> remove Repository::refresh_object_database()
   With the linked DB this is simply not possible anymore and we expect
   these updates to happen automatically in future for greater convenience.
   
   For now, in order to refresh a repository, one has to reopen it.
 - <csr-id-580e96c1b2d9782a2e8cf9d1123f6d53a5376a3d/> Rename `Handle` to `Cache`
   Because this is exactly what it is effectively.
   Also add some basic instantiation for the new object store.
 - remove borrowing Repo as possible failure cause
   The `easy::Handle` is now a full (but shared) clone of the original
   Repository with additional thread-local state, hence there is no more
   need for a way to access the original repository.
 - remove Easy… abstraction in favor of Handle
   This great reduction of complexity allows for being multi-threading
   capable by default with the option to turn that off at compile time.
   
   All `to|into_easy…()` methods are removed in favor of `to_easy()`
   along with the removal of all `Easy` types in favor of the single
 - remove pack-cache from `Find::try_find(…)`
   With the new architecture this can be an implementation detail without
   forcing it to be Sync.
 - move gix_pack::data::Object to gix_object::Data, massively alter gix_odb::Find trait
   This will break a lot, but has to happen to prepare these traits for the
   next generation of object databases.

## 0.13.0 (2021-11-29)

<csr-id-951c050ecbb70c9de216603e55c7cfbc89a067e3/>
<csr-id-0e1875363fea09452789d7a90fc6860a7996d6d3/>

With changes to `gix-ref`, what follows is all the adjustments made to simplify the `gix` implementation.

### Changed (BREAKING)

 - <csr-id-5d498a33236391d8e456f267b1bf6af24de66f11/> file::Store::iter() is now a platform, with `.all()` and `.prefixed(…)` respectively
   This way, it's possible to keep shared ownership of the packed buffer
   while allowing the exact same iterator machinery to work as before.
 - <csr-id-15d429bb50602363292453606902bdce5042d9a5/> file::Store::(try_)find(…, packed) was removed
   The packed buffer is now handled internally while loading it on demand.
   When compiled with `gix-features/parallel` the `file::Store` remains
   send and sync.
   
   The packed refs buffer is shared across clones and it's recommended
   to clone one `file::Store` instance per thread, each of which can
   use its own namespace.
 - <csr-id-95247322a8191edfa7fac9c5aa72b40239f3aa88/> move `gix_ref::file::WriteRefLog` to `gix_ref::store::WriteRefLog`

### Bug Fixes (BREAKING)

 - <csr-id-fc8e85cd71d4f16bc8daad0b790d875045faefff/> ref namespaces are now thread-local
   Previously these were shared in the shared Repo instance, which makes
   threaded applications impossible to remain deterministic across multiple
   connections.
   
   Now they are local to the thread, which allowed some methods to remove
   their Result<> as they cannot fail anymore, the reason for this being
   a breaking change.

### Other (BREAKING)

 - <csr-id-951c050ecbb70c9de216603e55c7cfbc89a067e3/> Reference::logs() -> Reference::log_iter()
   The latter now returns a standard Platform to iterate over all
   reflog entries from oldest to newest or vice versa.

### Refactor (BREAKING)

 - <csr-id-0e1875363fea09452789d7a90fc6860a7996d6d3/> `file::Store::base` is now `file::Store::base()` and read-only
   That way, file databases can't be repositioned anymore, it's recommended
   to recreate it if that's desired.

## 0.12.0 (2021-11-16)

### New Features

 - <csr-id-b7aab9efd42975e8f2dcb5c97e51495996175702/> Allow `PartialNameRef` to be created from owned items

### Changed (BREAKING)

 - <csr-id-e8b091943f0c9a26317da0003f7fcdf5a56ef21a/> Rename gix->ein and gixp->gix

## v0.11.0 (2021-10-19)

A maintenance release to properly dealing with previously breaking changes in `gix-hash`.

## v0.10.0 (2021-10-15)

<csr-id-1cb41f81cffe19c75aadf49a5cc7ec390ec6cae7/>
<csr-id-2f2d856efe733d3cf81110c0e0607d2e7c40d968/>
<csr-id-a19567eceab0dd7f5478b83c2ff9ce79754db308/>
<csr-id-61793ff42f5c2f9ddf302901adea2dac6149eac8/>
<csr-id-0cd585e20a5abd323a34ec32d92fbd48531b3b18/>
<csr-id-89f15051763a03627f332c46beedfc53b8b9b15b/>
<csr-id-f644d0ede7a2e8d344a81c7003c3877eed64a6b0/>
<csr-id-ac3b9efb7b90958274ce55800959d930f8641115/>
<csr-id-03fe8a7ebd34608d725d4585da5c1630123762ec/>
<csr-id-8fe461281842b58aa11437445637c6e587bedd63/>
<csr-id-b209da29f361512ba757febf56bc1aca039f2a41/>
<csr-id-741558dd8194590c5cc8566aa22f96e73df38edf/>
<csr-id-e16603b15b5488b81563c583cd8f5292ab9d24a2/>
<csr-id-54a64a588ff72515451a3d0343306ac4abe1cb35/>
<csr-id-1f4e45a26a3d2727f00c3f248452dd41fc8a95be/>
<csr-id-1958e8aa65eb97f9755f065d713f0a48c5e41b1b/>
<csr-id-066f59b23a125b1ce9a015437a3f4468e5791da0/>
<csr-id-329d183ad4e256a4f9cdeb34589b5f3432495f79/>
<csr-id-1a1959f487d69ffdd5394775b707139c44dbd11d/>
<csr-id-5e091fb2b4fd33879c176e6dadd3c9805d99af50/>
<csr-id-e3760679547e0dc1bf31761acdb6e63b04a50919/>
<csr-id-de004b318fdc6923711dd001bff5f4bcbba4270e/>
<csr-id-41afad3386461b658ee859225785b6de86d13cfb/>
<csr-id-f582439a3efe5c234f54c488792395e9de09a032/>
<csr-id-42080aefe3b286afb58235c1c22491579ab73919/>
<csr-id-d422b9a31a37a03551bec4382039aaf3a7e49902/>
<csr-id-e7c061b10c263001eb4abf03098d6694b770f828/>
<csr-id-66292fd1076c2c9db4694c5ded09799a0be11a03/>
<csr-id-5aadf75a0d93d1a990ad0305c38366c5c22bdcb2/>
<csr-id-d79a1b75304e397c16b5af7055906591a187ddfd/>
<csr-id-7d2b6b66e09ff39727fccd68d190679b52d90126/>
<csr-id-06996e032b1e451a674395ebaca94434fac46f05/>
<csr-id-daec7167df524b329daad7dabb1b9920b6ef8936/>
<csr-id-4fe4786797d240a59d29dbf2c6310490a381c8b6/>
<csr-id-debe0094826f83839f907523715def929133fd58/>
<csr-id-56e39fac54bfa3871c42bbf76a9f7c49486b85be/>
<csr-id-293bfc0278c5983c0beaec93253fb51f00d81156/>
<csr-id-650241251a420602f74037babfc24c9f64df78d8/>
<csr-id-2b4a61589a7cba3f7600710e21304e731ae3b36a/>
<csr-id-8b82f7d44c7eb63b7922ddc31ada9cefdce776b0/>

### New Features

 - <csr-id-11b64fce4630371633b6415f227eecdc6b42b20b/> Make `gix_url::Url` available under `gix::Url`
 - <csr-id-80b8331092f4856f52afa1d85fa375ae688bdd28/> add easy::ext::ObjectAccessExt::tag(…) to create tag objects
   It's a quick sketch on how tag object creation could work.
   
   Note the duplication the method name using traits, which seems like a good solution
   to the problem of differentiating tag objects and tag references while
   keeping the method name short.
   
   Most will only ever need one, right?
   
   Even in my example that's not the case, so maybe we have to rename it.
 - <csr-id-0ebfeb614264ca06ab763189e55e6c016c9997af/> Make `gix_url::Url` available under `gix::Url`

### BREAKING Changes

 - Use 'to_*' when converting `easy::Object` to specific object kind
   This also makes the API more consistent while being more idiomatic.
 - Avoid duplicate module paths in 'tree' and 'commit'
 - rename ObjectIdExt::ancestors_iter() to *::ancestors()
 - rename `easy::Object::to_(commit|tag)_iter()`…
   …to  `easy::Object::try_to_(commit|tag)_iter()` for consistency.
 - rename `*::State` into `*::Platform`
 - various small API changes
 - move easy::head::peel::Error -> easy::head::peel::to_id::Error
 - rename path::is_git to path::is
 - rename easy::reference::log::State to easy::reference::Logs

### Other

 - <csr-id-293bfc0278c5983c0beaec93253fb51f00d81156/> loose reference iteration with non-dir prefixes…
   Previously it was expected for the prefix `Path` to always exist for
   the prefix to be valid. This, however, is not similar to packed
   prefixes, which allow non-dir prefixes as well.
   
   Now we will check if the prefix is actually a directory, and if not
   split it into its parent directory and the filename portion. The latter
   is then used for prefix matching file names within that directory.
 - <csr-id-650241251a420602f74037babfc24c9f64df78d8/> Add 'references().all().peeled().'…
   …to not only make typical usage of iterated references more convenient
   but also work around a double-borrow error one would see otherwise.
 - <csr-id-2b4a61589a7cba3f7600710e21304e731ae3b36a/> filter refs correctly, but…
   …it needs a way to peel references right away without trying
   to double-borrow. This means the Iterator needs to implement this.
 - <csr-id-8b82f7d44c7eb63b7922ddc31ada9cefdce776b0/> improved changelog…
   …akin to 'Keep a changelog'.

### Refactor

 - <csr-id-8fe461281842b58aa11437445637c6e587bedd63/> split data::output::count::objects into files
 - <csr-id-b209da29f361512ba757febf56bc1aca039f2a41/> use new gix_pack::cache::Object trait
 - <csr-id-741558dd8194590c5cc8566aa22f96e73df38edf/> remove object cache impl which now lives in gix-pack

### Other

 - <csr-id-e16603b15b5488b81563c583cd8f5292ab9d24a2/> :remote_url() is now optional
   Otherwise it wouldn't work on repos that don't have a remote set yet.
   Instead of failing, we don't create links.
 - <csr-id-54a64a588ff72515451a3d0343306ac4abe1cb35/> try to create persistent Easy iterator, but can't make it Send…
   …which is fair as it contains borrowed RefCells, which really would have
   to be owned to work for this, which would in turn require the Ancestor's
   struct to be kind of self-referential
 - <csr-id-1f4e45a26a3d2727f00c3f248452dd41fc8a95be/> path::is
 - <csr-id-1958e8aa65eb97f9755f065d713f0a48c5e41b1b/> path::discover
 - <csr-id-066f59b23a125b1ce9a015437a3f4468e5791da0/> top-level of 'path' module
 - <csr-id-329d183ad4e256a4f9cdeb34589b5f3432495f79/> object_id
 - <csr-id-1a1959f487d69ffdd5394775b707139c44dbd11d/> repository
 - <csr-id-5e091fb2b4fd33879c176e6dadd3c9805d99af50/> ext::tree
 - <csr-id-e3760679547e0dc1bf31761acdb6e63b04a50919/> easy::object::peel
 - <csr-id-de004b318fdc6923711dd001bff5f4bcbba4270e/> easy::object::errors
 - <csr-id-41afad3386461b658ee859225785b6de86d13cfb/> a seemingly slow version of path lookup, but…
   …in debug mode it's faster than the fast path, despite doing more
   and being the same when it comes to searching path components.
 - <csr-id-f582439a3efe5c234f54c488792395e9de09a032/> easy::object, sans a few child-modules
 - <csr-id-42080aefe3b286afb58235c1c22491579ab73919/> update 'platform' information to reflect the current usage
 - <csr-id-d422b9a31a37a03551bec4382039aaf3a7e49902/> configure caches with env vars using `apply_environment()`
 - <csr-id-e7c061b10c263001eb4abf03098d6694b770f828/> refactor
 - <csr-id-66292fd1076c2c9db4694c5ded09799a0be11a03/> set package cache via RepositoryAccessExt
 - <csr-id-5aadf75a0d93d1a990ad0305c38366c5c22bdcb2/> Add GITOXIDE_PACK_CACHE_MEMORY_IN_BYTES=536870912 to control pack-cache size…
   …which can mean another considerable speed-up for many workloads, but
   usually needs some knowledge about the application, repos, and should
   thus be with the user.
 - <csr-id-d79a1b75304e397c16b5af7055906591a187ddfd/> allow disabling the pack cache with GITOXIDE_DISABLE_PACK_CACHE
 - <csr-id-7d2b6b66e09ff39727fccd68d190679b52d90126/> prepare for configurable pack cache
 - <csr-id-06996e032b1e451a674395ebaca94434fac46f05/> object-cache to allow for a speed boost…
   …by avoiding duplicate accesses to hit the object database.
   However, the cost for the cache are relatively high and involve some
   memory copying, so hit rates of about 50% is certainly what is needed
   to get any speed boost at all.
 - <csr-id-daec7167df524b329daad7dabb1b9920b6ef8936/> build commit history for later use in changelog generation
 - <csr-id-4fe4786797d240a59d29dbf2c6310490a381c8b6/> Allow object access during commit ancestor traversal…
   …by getting only a temporary handle to the pack-cache. The cost of this
   should be negligible compared to the cost of object decoding.
 - <csr-id-debe0094826f83839f907523715def929133fd58/> sketch history acquisition
 - <csr-id-56e39fac54bfa3871c42bbf76a9f7c49486b85be/> add 'Head::peeled()' method

### Changed (BREAKING)

 - <csr-id-c3385cd144298eb9f06d7751d180e26da7b4d338/> `easy::Object::try_to_commit()` now returns `Result<CommitRef>`…
   …without the nested `Option`, folding the type mismatch into a specific
   `conversion::Error` instead.
 - <csr-id-e59f901f47fb0180211494a1591aed62b856406a/> rename `ObjectAccessExt::tag(…)` to `*::tag_reference(…)`, add `easy::Object::try_to_tag()`
   This one also contains the first and probably only test for tag object
   creation.

## v0.9.1 (2021-09-10)

<csr-id-293bfc0278c5983c0beaec93253fb51f00d81156/>
<csr-id-650241251a420602f74037babfc24c9f64df78d8/>
<csr-id-2b4a61589a7cba3f7600710e21304e731ae3b36a/>
<csr-id-8b82f7d44c7eb63b7922ddc31ada9cefdce776b0/>

- Remove `max-performance` feature from default set until the `msvc` build issue is fixed. Otherwise it will surprisingly break windows builds.

### Other

 - <csr-id-293bfc0278c5983c0beaec93253fb51f00d81156/> loose reference iteration with non-dir prefixes…
   Previously it was expected for the prefix `Path` to always exist for
   the prefix to be valid. This, however, is not similar to packed
   prefixes, which allow non-dir prefixes as well.
   
   Now we will check if the prefix is actually a directory, and if not
   split it into its parent directory and the filename portion. The latter
   is then used for prefix matching file names within that directory.
 - <csr-id-650241251a420602f74037babfc24c9f64df78d8/> Add 'references().all().peeled().'…
   …to not only make typical usage of iterated references more convenient
   but also work around a double-borrow error one would see otherwise.
 - <csr-id-2b4a61589a7cba3f7600710e21304e731ae3b36a/> filter refs correctly, but…
   …it needs a way to peel references right away without trying
   to double-borrow. This means the Iterator needs to implement this.
 - <csr-id-8b82f7d44c7eb63b7922ddc31ada9cefdce776b0/> improved changelog…
   …akin to 'Keep a changelog'.

## v0.9.0 (2021-09-08)

- rename `prelude::ConfigAccessExt` to `prelude::RepositoryAccessExt`
- `prelude::ObjectAccessExt::commit()` signature change
- cargo feature changed in incompatible ways. `network` was replaced by more finegrained options for _blocking_ and _async_ networking, as well as optional http transport

### New

- `init()`
- `init_bare()`
- `Repository::init(Kind)`
- `open()`
- `Repository::open()`
- `easy::Head`
- `easy::ext::ReferenceAccessExt::head()`
- `ext::ReferenceExt` trait

### Breaking
- **renames / moves / Signature Changes**
    - `path::Path` to `Path`
    - `init::repository(dir)` -> `path::create::into(dir, **Kind**)`

## v0.8.2 (2021-09-07)

## v0.8.1 (2021-08-28)

- Introduce `EasyArcExclusive` type, now available thanks to `parking_lot` 0.11.2

## v0.8.0 (2021-08-27)

- Rename `object` to `objs` to be equivalent to `refs` and make space for the new `object` module
- various minor version updates of pre-release dependencies

## v0.7.2 (2021-08-17)

## v0.7.1 (2021-08-13)

## v0.7.0 (2021-08-10)

## v0.6.0 (2021-05-28)

## v0.5.0 (2021-04-08)

## v0.4.0 (2020-09-12)

## v0.3.0 (2020-08-12)

## v0.1.0 (2020-07-12)

