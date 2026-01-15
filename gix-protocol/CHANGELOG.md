# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
    - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
    - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
    - Merge pull request #2333 from GitoxideLabs/improvements ([`9e1ba5e`](https://github.com/GitoxideLabs/gitoxide/commit/9e1ba5ee6b7e0e032f682097d48664a68891762d))
    - Release gix-command v0.6.4, gix-credentials v0.34.1, gix-transport v0.52.1 ([`3bc0c47`](https://github.com/GitoxideLabs/gitoxide/commit/3bc0c472a8395e9634dd602839e208501f841c11))
    - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))
</details>

## 0.55.0 (2025-12-31)

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

## 0.54.0 (2025-12-22)

<csr-id-37a42bde69d1a80080f91a78514565508b278876/>
<csr-id-0f58b0184b34df3b4e530ce4f7ab26369f882f99/>
<csr-id-e5498415a704859fe8385b09e46539372df04d9f/>
<csr-id-7e08219644fb84550b82e4c36cdec2d08a06136d/>
<csr-id-ab3cce472efc4f6469d99a0d3a425d58ac2140b9/>
<csr-id-c26efb9e733cb33a38ef885eded915e17e61bd9c/>
<csr-id-0021f9daec773f381c6f10bd310cd0f8c5077d54/>
<csr-id-7a60e7846a0e077a1c7f6d560461ee81ae0d4e85/>
<csr-id-3b4e06ac9f6b600f58c459a034e6cfaac7221514/>
<csr-id-3bc580173ca8f6e95f0e8996a05a451dc199dfa3/>
<csr-id-c6d76885bdddc2d6a230618b8ba3f4ebae715987/>
<csr-id-149bcfca6a35ebfd7493ee8517905a6b6ca58dd5/>

### Other

 - <csr-id-37a42bde69d1a80080f91a78514565508b278876/> attach push_prefix_arguments() to LsRefsCommand
 - <csr-id-0f58b0184b34df3b4e530ce4f7ab26369f882f99/> extract condition from helper function
 - <csr-id-e5498415a704859fe8385b09e46539372df04d9f/> import RefsAction directly
 - <csr-id-7e08219644fb84550b82e4c36cdec2d08a06136d/> drop unnecessary argument from DelegateBlocking::prepare_ls_refs()

### Refactor (BREAKING)

 - <csr-id-ab3cce472efc4f6469d99a0d3a425d58ac2140b9/> split async and blocking implementations for Handshake::fetch_or_extract_refmap()
 - <csr-id-c26efb9e733cb33a38ef885eded915e17e61bd9c/> split Handshake::fetch_or_extract_refmap()
 - <csr-id-0021f9daec773f381c6f10bd310cd0f8c5077d54/> inline RefMap::fetch() into Handshake::fetch_or_extract_refmap()
 - <csr-id-7a60e7846a0e077a1c7f6d560461ee81ae0d4e85/> move prefix argument handling into LsRefsCommand::new()
 - <csr-id-3b4e06ac9f6b600f58c459a034e6cfaac7221514/> extract LsRefsCommand type to split preparation
 - <csr-id-3bc580173ca8f6e95f0e8996a05a451dc199dfa3/> stop taking a callback for test-only uses
 - <csr-id-c6d76885bdddc2d6a230618b8ba3f4ebae715987/> rename ls_refs::Action to RefsAction
 - <csr-id-149bcfca6a35ebfd7493ee8517905a6b6ca58dd5/> take extra arguments as a separate argument

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 26 calendar days.
 - 29 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
    - Merge pull request #2274 from djc/simplify-ls-refs ([`eab774c`](https://github.com/GitoxideLabs/gitoxide/commit/eab774c3bbc5fc0b4b81877892f12d4491f17565))
    - Refactor ([`9d936fb`](https://github.com/GitoxideLabs/gitoxide/commit/9d936fbb357877c13c0737bd84715e189a3d9dea))
    - Split async and blocking implementations for Handshake::fetch_or_extract_refmap() ([`ab3cce4`](https://github.com/GitoxideLabs/gitoxide/commit/ab3cce472efc4f6469d99a0d3a425d58ac2140b9))
    - Split Handshake::fetch_or_extract_refmap() ([`c26efb9`](https://github.com/GitoxideLabs/gitoxide/commit/c26efb9e733cb33a38ef885eded915e17e61bd9c))
    - Inline RefMap::fetch() into Handshake::fetch_or_extract_refmap() ([`0021f9d`](https://github.com/GitoxideLabs/gitoxide/commit/0021f9daec773f381c6f10bd310cd0f8c5077d54))
    - Move prefix argument handling into LsRefsCommand::new() ([`7a60e78`](https://github.com/GitoxideLabs/gitoxide/commit/7a60e7846a0e077a1c7f6d560461ee81ae0d4e85))
    - Attach push_prefix_arguments() to LsRefsCommand ([`37a42bd`](https://github.com/GitoxideLabs/gitoxide/commit/37a42bde69d1a80080f91a78514565508b278876))
    - Extract condition from helper function ([`0f58b01`](https://github.com/GitoxideLabs/gitoxide/commit/0f58b0184b34df3b4e530ce4f7ab26369f882f99))
    - Extract LsRefsCommand type to split preparation ([`3b4e06a`](https://github.com/GitoxideLabs/gitoxide/commit/3b4e06ac9f6b600f58c459a034e6cfaac7221514))
    - Stop taking a callback for test-only uses ([`3bc5801`](https://github.com/GitoxideLabs/gitoxide/commit/3bc580173ca8f6e95f0e8996a05a451dc199dfa3))
    - Import RefsAction directly ([`e549841`](https://github.com/GitoxideLabs/gitoxide/commit/e5498415a704859fe8385b09e46539372df04d9f))
    - Rename ls_refs::Action to RefsAction ([`c6d7688`](https://github.com/GitoxideLabs/gitoxide/commit/c6d76885bdddc2d6a230618b8ba3f4ebae715987))
    - Take extra arguments as a separate argument ([`149bcfc`](https://github.com/GitoxideLabs/gitoxide/commit/149bcfca6a35ebfd7493ee8517905a6b6ca58dd5))
    - Merge pull request #2275 from GitoxideLabs/dependabot/cargo/cargo-92eaa62a2e ([`93dd630`](https://github.com/GitoxideLabs/gitoxide/commit/93dd630ca6a2a4622ca74d7eaff42ece2750b6c5))
    - Bump the cargo group across 1 directory with 14 updates ([`703644c`](https://github.com/GitoxideLabs/gitoxide/commit/703644c8821aae161592d19495d3b3162133324f))
    - Drop unnecessary argument from DelegateBlocking::prepare_ls_refs() ([`7e08219`](https://github.com/GitoxideLabs/gitoxide/commit/7e08219644fb84550b82e4c36cdec2d08a06136d))
</details>

## 0.53.0 (2025-11-22)

<csr-id-4c40484391c32b4d42b5904a1a0c42639369673e/>
<csr-id-36c2be80e61a7a0ef637a5385e1960de13261984/>
<csr-id-541a7a6aab6a56fa2949a5577acbacf6dc59a9dd/>
<csr-id-19a005135b91641a19a3a70fd714985ce1203333/>
<csr-id-50120de92dab3d8347672df7a43df02291162c9a/>
<csr-id-37ab03687a940c89131141b882e5ef1a7fef85be/>
<csr-id-f05dfab881481de5de1d23e3266cad9c069f59c5/>
<csr-id-b05eba5cca4294b9baeafd0c15885cefd21d0d6c/>

### New Features

 - <csr-id-aeee98234002f0cf79bd3629ada5d3ec3d7b86cd/> add `Handshake::ref_map()` to produce a ref-map from a V1 or V2 handshake.

### Refactor

 - <csr-id-4c40484391c32b4d42b5904a1a0c42639369673e/> keep Options intact through fetching

### New Features (BREAKING)

 - <csr-id-bce8468443dd490d3e2955b623aa3e97e6fe7bfd/> large portions of the API for greater ease of use

### Bug Fixes (BREAKING)

 - <csr-id-87d5419a35298d6627a5695790f91ead698b6fb0/> enable async I/O code independent of blocking_io
 - <csr-id-c2050d0ec74e2c2d1d70b68e57bbb79e196796b9/> make gix-transport I/O mode features additive
 - <csr-id-b6d58225f6760db7a4b3c47c8d72e346d6295ec9/> move TransportWithoutIO::request() method to I/O-specific traits
 - <csr-id-f2c6a19f676f1de9e0b4e0ff77f4e26d817fac7e/> make `gix-packeline` features additive.
   For good measure, everything that was touched is marked as breaking change.

### Refactor (BREAKING)

 - <csr-id-36c2be80e61a7a0ef637a5385e1960de13261984/> simplify gix-protocol handshake API by avoiding duplicates that remove the `server` parameter.
 - <csr-id-541a7a6aab6a56fa2949a5577acbacf6dc59a9dd/> turn `handshake::Outcome` into `Handshake` type.
 - <csr-id-19a005135b91641a19a3a70fd714985ce1203333/> hoist handshake ref handling out of transport code
 - <csr-id-50120de92dab3d8347672df7a43df02291162c9a/> store all_refspecs in Options
 - <csr-id-37ab03687a940c89131141b882e5ef1a7fef85be/> store fetch refspecs in Options
 - <csr-id-f05dfab881481de5de1d23e3266cad9c069f59c5/> flatten RefMap::new() arguments
 - <csr-id-b05eba5cca4294b9baeafd0c15885cefd21d0d6c/> pass user agent into ls_refs() separately

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
    - Merge pull request #2250 from djc/rm-handshake-layers ([`3c2b422`](https://github.com/GitoxideLabs/gitoxide/commit/3c2b422db647b91b584913b8109b1a6c225d1dd7))
    - Bring the `service` parameter back to not hardcode the handshake for fetches. ([`c9a97db`](https://github.com/GitoxideLabs/gitoxide/commit/c9a97dbe35d1447319fd953062c465cd1c45d9b6))
    - Simplify gix-protocol handshake API by avoiding duplicates that remove the `server` parameter. ([`36c2be8`](https://github.com/GitoxideLabs/gitoxide/commit/36c2be80e61a7a0ef637a5385e1960de13261984))
    - Merge pull request #2242 from djc/ls-refs ([`0ac3080`](https://github.com/GitoxideLabs/gitoxide/commit/0ac3080d00d753429845e994c19b6baa9bccae21))
    - Add `Handshake::ref_map()` to produce a ref-map from a V1 or V2 handshake. ([`aeee982`](https://github.com/GitoxideLabs/gitoxide/commit/aeee98234002f0cf79bd3629ada5d3ec3d7b86cd))
    - Turn `handshake::Outcome` into `Handshake` type. ([`541a7a6`](https://github.com/GitoxideLabs/gitoxide/commit/541a7a6aab6a56fa2949a5577acbacf6dc59a9dd))
    - Refactor ([`48fdf5d`](https://github.com/GitoxideLabs/gitoxide/commit/48fdf5d9d32f45811eba9600f2a3f154c69ce5a9))
    - Hoist handshake ref handling out of transport code ([`19a0051`](https://github.com/GitoxideLabs/gitoxide/commit/19a005135b91641a19a3a70fd714985ce1203333))
    - Keep Options intact through fetching ([`4c40484`](https://github.com/GitoxideLabs/gitoxide/commit/4c40484391c32b4d42b5904a1a0c42639369673e))
    - Store all_refspecs in Options ([`50120de`](https://github.com/GitoxideLabs/gitoxide/commit/50120de92dab3d8347672df7a43df02291162c9a))
    - Store fetch refspecs in Options ([`37ab036`](https://github.com/GitoxideLabs/gitoxide/commit/37ab03687a940c89131141b882e5ef1a7fef85be))
    - Flatten RefMap::new() arguments ([`f05dfab`](https://github.com/GitoxideLabs/gitoxide/commit/f05dfab881481de5de1d23e3266cad9c069f59c5))
    - Pass user agent into ls_refs() separately ([`b05eba5`](https://github.com/GitoxideLabs/gitoxide/commit/b05eba5cca4294b9baeafd0c15885cefd21d0d6c))
    - Extract helper utility from RefMap::new() ([`102077b`](https://github.com/GitoxideLabs/gitoxide/commit/102077b3207ee24ff971ff543880158c04324b98))
    - Merge pull request #2241 from djc/transport-all ([`7ce40ef`](https://github.com/GitoxideLabs/gitoxide/commit/7ce40ef1b9c5d2fc94c12dfeaf7e5793342769a5))
    - Enable async I/O code independent of blocking_io ([`87d5419`](https://github.com/GitoxideLabs/gitoxide/commit/87d5419a35298d6627a5695790f91ead698b6fb0))
    - Merge pull request #2236 from djc/additive-transport ([`6e89afa`](https://github.com/GitoxideLabs/gitoxide/commit/6e89afacbf1fc8392ce52d5b87f7e85c9bab17a1))
    - Make gix-transport I/O mode features additive ([`c2050d0`](https://github.com/GitoxideLabs/gitoxide/commit/c2050d0ec74e2c2d1d70b68e57bbb79e196796b9))
    - Merge pull request #2231 from djc/move-request ([`6566ec9`](https://github.com/GitoxideLabs/gitoxide/commit/6566ec9b7a1073fecc3a80a36584977a8129f318))
    - Move TransportWithoutIO::request() method to I/O-specific traits ([`b6d5822`](https://github.com/GitoxideLabs/gitoxide/commit/b6d58225f6760db7a4b3c47c8d72e346d6295ec9))
    - Merge pull request #2220 from djc/dedup-packetline ([`cbf7445`](https://github.com/GitoxideLabs/gitoxide/commit/cbf7445055357c7bac8b5946fb6e55b04b1a862a))
    - Refactor ([`7289a04`](https://github.com/GitoxideLabs/gitoxide/commit/7289a040aa1c848e13dff4b15ca6eba9e8dc0a30))
    - Large portions of the API for greater ease of use ([`bce8468`](https://github.com/GitoxideLabs/gitoxide/commit/bce8468443dd490d3e2955b623aa3e97e6fe7bfd))
    - Make `gix-packeline` features additive. ([`f2c6a19`](https://github.com/GitoxideLabs/gitoxide/commit/f2c6a19f676f1de9e0b4e0ff77f4e26d817fac7e))
    - Merge pull request #2230 from yuki0iq/doc_auto_cfg ([`fbf9c39`](https://github.com/GitoxideLabs/gitoxide/commit/fbf9c39c3ccd5e7879a2d7918aa157f7923cb8a5))
</details>

## 0.52.1 (2025-10-23)

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

## 0.52.0 (2025-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 99 calendar days.
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
    - Merge pull request #2163 from cruessler/deprecate-in-place-methods ([`42f8db5`](https://github.com/GitoxideLabs/gitoxide/commit/42f8db5bc9096cdedba497c850c86285ecbacc69))
    - Adapt to changes in `gix-ref` ([`44922d0`](https://github.com/GitoxideLabs/gitoxide/commit/44922d0a71b6cd3595dec8187fa3842097eacd5b))
    - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
    - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
    - Merge pull request #2110 from jpgrayson/fix/gix-date-parse-raw ([`651f9fa`](https://github.com/GitoxideLabs/gitoxide/commit/651f9fa560d5df7260a45068b8440f72820a6ffd))
    - Release gix-date v0.10.5 ([`4289ae6`](https://github.com/GitoxideLabs/gitoxide/commit/4289ae635d94d713d247eaf6f87d0ba91a1a3826))
    - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
    - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
    - Merge pull request #2097 from GitoxideLabs/fix-gix-date ([`589d63e`](https://github.com/GitoxideLabs/gitoxide/commit/589d63ed21e5f2cd53ad2cac96fc387df3ea26e9))
    - Release gix-date v0.10.4 ([`007e3f6`](https://github.com/GitoxideLabs/gitoxide/commit/007e3f66246aaafc2374b85cbf77f89ec0b09512))
    - Merge pull request #2090 from GitoxideLabs/dependabot/cargo/cargo-f147714000 ([`473fe52`](https://github.com/GitoxideLabs/gitoxide/commit/473fe522e84569f77bf38294a412f0d13fa54d63))
    - Bump the cargo group with 41 updates ([`428412c`](https://github.com/GitoxideLabs/gitoxide/commit/428412c9ff05caabb4f8714d5de769603e18a8f9))
    - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))
</details>

## 0.51.0 (2025-07-15)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 79 calendar days.
 - 79 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
    - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
    - Merge pull request #2062 from rickprice/minor_documentation_fixups ([`c2eb0c1`](https://github.com/GitoxideLabs/gitoxide/commit/c2eb0c144dd21cac87fd08829f4a5ca02f85008d))
    - Small documentation fixes ([`bfb1c34`](https://github.com/GitoxideLabs/gitoxide/commit/bfb1c34f75997a603b8f85fca75bf9e1ca310be0))
    - Merge pull request #2033 from GitoxideLabs/dependabot/cargo/cargo-b72232998d ([`f8d7c0a`](https://github.com/GitoxideLabs/gitoxide/commit/f8d7c0ad8fa7745c973c6b87e7eee70831300207))
    - Bump the cargo group with 56 updates ([`151e3a5`](https://github.com/GitoxideLabs/gitoxide/commit/151e3a5cca06444eea4c6a362649e66c831673d6))
    - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
    - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
    - Merge pull request #1975 from GitoxideLabs/improvements ([`28935a5`](https://github.com/GitoxideLabs/gitoxide/commit/28935a56ff91f1fc2c17a7d23b057cf7119144e9))
    - Thanks clippy ([`dbf65c9`](https://github.com/GitoxideLabs/gitoxide/commit/dbf65c95644e6a134e7f9b75e7871479720b4deb))
    - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))
</details>

## 0.50.1 (2025-04-26)

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

## 0.50.0 (2025-04-25)

### Bug Fixes

 - <csr-id-11bd59f794811e11e48ef8de4db59035434f9da6/> cargo-auditable build error
   Use `futures-lite` instead of `dep:futures-lite` in gix-packetline and
   gix-protocol.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
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
    - Merge pull request #1968 from GitoxideLabs/dependabot/cargo/cargo-bd18780e40 ([`46227e6`](https://github.com/GitoxideLabs/gitoxide/commit/46227e6d1ddc0879662730e5bb21a8597716b1ca))
    - Bump the cargo group with 40 updates ([`06bf1e1`](https://github.com/GitoxideLabs/gitoxide/commit/06bf1e1552de65ce692911bdc4c501d487bbc3d7))
    - Merge pull request #1957 from EliahKagan/run-ci/versioning ([`5823b22`](https://github.com/GitoxideLabs/gitoxide/commit/5823b22bfcd30123b6859ec9dc62c62ce0737f72))
    - Adapt `Cargo.toml` files in workspace to `gix-features` bump ([`6315536`](https://github.com/GitoxideLabs/gitoxide/commit/63155368cc5074328314f1b3f565e5813df725cf))
    - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
    - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
    - Merge pull request #1933 from GitoxideLabs/release-gix-features ([`1612c73`](https://github.com/GitoxideLabs/gitoxide/commit/1612c73a16c8d900e1b6ef35b25bd6b3e3f6652a))
    - Release gix-features v0.41.1 ([`fc5faf2`](https://github.com/GitoxideLabs/gitoxide/commit/fc5faf24dfc6d6e1580308ec5e7c12e96e0ccb41))
    - Merge pull request #1929 from Prince213/fix-cargo-auditable ([`19f50d0`](https://github.com/GitoxideLabs/gitoxide/commit/19f50d0ff30291fe57927f19b215ef1c9096217c))
    - Cargo-auditable build error ([`11bd59f`](https://github.com/GitoxideLabs/gitoxide/commit/11bd59f794811e11e48ef8de4db59035434f9da6))
    - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))
</details>

## 0.49.0 (2025-04-04)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge pull request #1909 from cruessler/take-to-components-in-fs-stack ([`5cb5337`](https://github.com/GitoxideLabs/gitoxide/commit/5cb5337efd7679d8a2ab4bd5e6a5da8c366f7f1a))
    - Use `gix_fs::stack::ToNormalPathComponents` everywhere. ([`1f98edb`](https://github.com/GitoxideLabs/gitoxide/commit/1f98edbaa51caaf152eda289b769388676259a06))
    - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
    - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
    - Merge pull request #1822 from epage/w7 ([`11ac79c`](https://github.com/GitoxideLabs/gitoxide/commit/11ac79c068181d4ed9f6a404e4875ad7c206520c))
    - Upgrade to Winnow 0.7 ([`fdc57e7`](https://github.com/GitoxideLabs/gitoxide/commit/fdc57e79af6f7922d91ad8d7796943821f637124))
    - Resolve Winnow deprecations ([`3cd3e2a`](https://github.com/GitoxideLabs/gitoxide/commit/3cd3e2a71beb01591afe732ab4ae914ed62a4ecf))
    - Upgrade to Winnow 0.6.26 ([`783c4e6`](https://github.com/GitoxideLabs/gitoxide/commit/783c4e698234b8afaf8fbd25057aca11c5c66e75))
    - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))
</details>

## 0.48.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Chore

 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 27 calendar days.
 - 27 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
    - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
    - Merge pull request #1763 from GitoxideLabs/better-refspec-primitives ([`af8f201`](https://github.com/GitoxideLabs/gitoxide/commit/af8f2019723dd9ee3ac46a935910946fcc15e8bb))
    - Adapt to changes in `gix-refspec` ([`6d7dd9b`](https://github.com/GitoxideLabs/gitoxide/commit/6d7dd9bced4a1a0e8175e047be838746a95aa596))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))
</details>

## 0.47.0 (2024-12-22)

### Bug Fixes

 - <csr-id-5c21ebc3f523bbe64cb84bbcdf39a2c284ba1df1/> respect interrupt-requests during negotiation
   It is possible for the negotiation to go on forever if implemented incorrectly
   on the client side. Hence it's useful to gracefully interrupt it.

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

 - <csr-id-a5f0d295cfc7183e5c0b35b72610e933f91059d9/> assure that empty fetches due to no objects wanted are terminated early
 - <csr-id-595092629115cd7d2b48140b1c0b211263146028/> remove `ref_map` from `fetch()` function.
   It's only uses for validation of the ref-maps own state, which
   is something that can be separate.
 - <csr-id-212ea9e8d6bd972d2a73f7ff4f9f35487d1f80f9/> move `fetch()` to tests so they are the only ones with delegates.
   This is a first step towards creating a multi-step `fetch()` which can
   also be used in the `gix` crate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release over the course of 28 calendar days.
 - 28 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0 ([`beb0ea8`](https://github.com/GitoxideLabs/gitoxide/commit/beb0ea8c4ff94c64b7773772a9d388ccb403f3c1))
    - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
    - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
    - Merge pull request #1731 from GitoxideLabs/fix-pack-receive ([`ca54b8c`](https://github.com/GitoxideLabs/gitoxide/commit/ca54b8c67eb6c81b7175f62ee74a0d5aab6f52cc))
    - Respect interrupt-requests during negotiation ([`5c21ebc`](https://github.com/GitoxideLabs/gitoxide/commit/5c21ebc3f523bbe64cb84bbcdf39a2c284ba1df1))
    - Assure that empty fetches due to no objects wanted are terminated early ([`a5f0d29`](https://github.com/GitoxideLabs/gitoxide/commit/a5f0d295cfc7183e5c0b35b72610e933f91059d9))
    - Merge pull request #1726 from GitoxideLabs/radicle-tuning ([`a542775`](https://github.com/GitoxideLabs/gitoxide/commit/a54277561a62cd560a9a072c6052eaf182ad4ace))
    - Remove `ref_map` from `fetch()` function. ([`5950926`](https://github.com/GitoxideLabs/gitoxide/commit/595092629115cd7d2b48140b1c0b211263146028))
    - Merge pull request #1634 from GitoxideLabs/remove-delegates ([`ddeb97f`](https://github.com/GitoxideLabs/gitoxide/commit/ddeb97f550bb95835648841b476d7647dd7c1dc0))
    - Move all possible code from `gix` to `gix-protocol`. ([`e59fc09`](https://github.com/GitoxideLabs/gitoxide/commit/e59fc09f12a1c6b27d878525ff6074ac846aa87e))
    - Add `gix-shallow` crate and use it from `gix` and `gix-protocol` ([`6367c7d`](https://github.com/GitoxideLabs/gitoxide/commit/6367c7d0a796aff8ee8778916c1a1ddae68b654d))
    - Move `fetch()` to tests so they are the only ones with delegates. ([`212ea9e`](https://github.com/GitoxideLabs/gitoxide/commit/212ea9e8d6bd972d2a73f7ff4f9f35487d1f80f9))
    - Refactor tests to conform to 'new style' ([`d7b1cce`](https://github.com/GitoxideLabs/gitoxide/commit/d7b1cce0514c3fe8eefb75ae8bea9cd2e9e58b08))
    - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))
</details>

## 0.46.1 (2024-11-24)

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
    - Merge pull request #1690 from EliahKagan/gitattributes ([`700cfa5`](https://github.com/GitoxideLabs/gitoxide/commit/700cfa52e7f3008036881a99fbdeb04c9ab1f2f5))
    - Fix marking `gix-packetline-blocking` copy auto-generated ([`325588e`](https://github.com/GitoxideLabs/gitoxide/commit/325588e700ec159e5139beb149127c947164fb98))
    - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
    - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
    - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
</details>

## 0.46.0 (2024-10-22)

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

### Bug Fixes (BREAKING)

 - <csr-id-24da857dbd236ae1cc75a4efd0816e717b885c7f/> make V1 shallow announcements possible
 - <csr-id-17573779688e755a786546d5e42ab533088cd726/> remove all workspace dependencies
   The problem is that with them, we don't notice anymore if the crate changes,
   because a dependency changes. That also means that older versions of the dependency
   may stay even though some other crates might pick up a newer version.
   
   Ultimately, this will lead to drift and subtle incompatibilities.
   
   We declare this breaking to enforce a proper re-release.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 60 calendar days.
 - 61 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1604](https://github.com/GitoxideLabs/gitoxide/issues/1604)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1604](https://github.com/GitoxideLabs/gitoxide/issues/1604)**
    - Make V1 shallow announcements possible ([`24da857`](https://github.com/GitoxideLabs/gitoxide/commit/24da857dbd236ae1cc75a4efd0816e717b885c7f))
 * **Uncategorized**
    - Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0 ([`f1364dc`](https://github.com/GitoxideLabs/gitoxide/commit/f1364dcb8aa66e3d8730e38445b045c5b63c56e6))
    - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
    - Thanks clippy ([`af03832`](https://github.com/GitoxideLabs/gitoxide/commit/af0383254422b70d53f27572c415eea2e4154447))
    - Merge pull request #1604 from Byron/protocol-shallow-v1 ([`612896d`](https://github.com/GitoxideLabs/gitoxide/commit/612896d7ec15c153d3d48012c75aaf85d10a5abe))
    - Reproduce #1598 ([`6c82f31`](https://github.com/GitoxideLabs/gitoxide/commit/6c82f31fd4009e9806a928de0bc2b21f043ca1d6))
    - Merge pull request #1557 from Byron/merge-base ([`649f588`](https://github.com/GitoxideLabs/gitoxide/commit/649f5882cbebadf1133fa5f310e09b4aab77217e))
    - Allow empty-docs ([`beba720`](https://github.com/GitoxideLabs/gitoxide/commit/beba7204a50a84b30e3eb81413d968920599e226))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - A few more missing semicolons ([`fc45c93`](https://github.com/GitoxideLabs/gitoxide/commit/fc45c931c132ac8b6ea4f2e4c3d5f0d19727f46f))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge pull request #1546 from nyurik/semilocons ([`f992fb7`](https://github.com/GitoxideLabs/gitoxide/commit/f992fb773b443454015bd14658cfaa2f3ac07997))
    - Add missing semicolons ([`ec69c88`](https://github.com/GitoxideLabs/gitoxide/commit/ec69c88fc119f3aa1967a7e7f5fca30e3ce97595))
    - Merge branch 'fixes' ([`46cd1ae`](https://github.com/GitoxideLabs/gitoxide/commit/46cd1aed7815d27cdc818edb87641b20b82ba048))
    - Remove all workspace dependencies ([`1757377`](https://github.com/GitoxideLabs/gitoxide/commit/17573779688e755a786546d5e42ab533088cd726))
</details>

## 0.45.3 (2024-08-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 28 calendar days.
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
    - Merge branch 'dependabot/github_actions/github-actions-c4bcf5a8e2' ([`2e00b5e`](https://github.com/GitoxideLabs/gitoxide/commit/2e00b5ef6e8a15e7f0a34d54739a5cd1c986b322))
    - Make `winnow` a workspace dependency ([`78a7e32`](https://github.com/GitoxideLabs/gitoxide/commit/78a7e32c34150dece4065e513cd177356619419f))
    - Merge branch 'ag/jiff' ([`5871fb1`](https://github.com/GitoxideLabs/gitoxide/commit/5871fb130b1a603c1e768f4b2371ac9d7cc56330))
    - Assure the next release is breaking ([`9fd1090`](https://github.com/GitoxideLabs/gitoxide/commit/9fd10905449a41cdda5eb2764e4d45d314de9c04))
    - Release gix-credentials v0.24.4 ([`f6a4eb9`](https://github.com/GitoxideLabs/gitoxide/commit/f6a4eb98740fe4151e293d53a578233119307a58))
</details>

## 0.45.2 (2024-07-23)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 32 calendar days.
 - 62 days passed between releases.
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
    - Release gix-date v0.8.7, gix-mailmap v0.23.2 ([`c1d7c02`](https://github.com/GitoxideLabs/gitoxide/commit/c1d7c023d595eb04891b65295f001d85c9ba8074))
    - Merge branch 'main' into config-key-take-2 ([`9fa1054`](https://github.com/GitoxideLabs/gitoxide/commit/9fa1054a01071180d7b08c8c2b5bd61e9d0d32da))
</details>

## 0.45.1 (2024-05-22)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 8 calendar days.
 - 38 days passed between releases.
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

## 0.45.0 (2024-04-13)

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
    - Release gix-trace v0.1.9, gix-utils v0.1.12, gix-packetline-blocking v0.17.4, gix-filter v0.11.1, gix-fs v0.10.2, gix-traverse v0.39.0, gix-worktree-stream v0.12.0, gix-archive v0.12.0, gix-config v0.36.1, gix-url v0.27.3, gix-index v0.32.0, gix-worktree v0.33.0, gix-diff v0.43.0, gix-pathspec v0.7.3, gix-dir v0.4.0, gix-pack v0.50.0, gix-odb v0.60.0, gix-transport v0.42.0, gix-protocol v0.45.0, gix-status v0.9.0, gix-worktree-state v0.10.0, gix v0.62.0, gix-fsck v0.4.0, gitoxide-core v0.37.0, gitoxide v0.35.0, safety bump 14 crates ([`095c673`](https://github.com/GitoxideLabs/gitoxide/commit/095c6739b2722a8b9af90776b435ef2da454c0e6))
    - Prepare changelogs prior to release ([`5755271`](https://github.com/GitoxideLabs/gitoxide/commit/57552717f46f96c35ba4ddc0a64434354ef845e9))
</details>

## 0.44.2 (2024-03-14)

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

## 0.44.1 (2024-02-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 36 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.4, gix-utils v0.1.10, gix-actor v0.30.1, gix-object v0.41.1, gix-path v0.10.6, gix-glob v0.16.1, gix-quote v0.4.11, gix-attributes v0.22.1, gix-command v0.3.5, gix-filter v0.10.0, gix-commitgraph v0.24.1, gix-worktree-stream v0.10.0, gix-archive v0.10.0, gix-config-value v0.14.5, gix-ref v0.42.0, gix-sec v0.10.5, gix-config v0.35.0, gix-prompt v0.8.3, gix-url v0.27.1, gix-credentials v0.24.1, gix-ignore v0.11.1, gix-index v0.30.0, gix-worktree v0.31.0, gix-diff v0.41.0, gix-discover v0.30.0, gix-pathspec v0.7.0, gix-dir v0.1.0, gix-pack v0.48.0, gix-odb v0.58.0, gix-transport v0.41.1, gix-protocol v0.44.1, gix-revision v0.26.1, gix-refspec v0.22.1, gix-status v0.6.0, gix-submodule v0.9.0, gix-worktree-state v0.8.0, gix v0.59.0, gix-fsck v0.3.0, gitoxide-core v0.36.0, gitoxide v0.34.0, safety bump 10 crates ([`45b4470`](https://github.com/GitoxideLabs/gitoxide/commit/45b447045bc826f252129c300c531acde2652c64))
    - Prepare changelogs prior to release ([`f2e111f`](https://github.com/GitoxideLabs/gitoxide/commit/f2e111f768fc1bc6182355261c20b63610cffec7))
    - Merge branch 'btoi' ([`5fc379d`](https://github.com/GitoxideLabs/gitoxide/commit/5fc379d1dc867d15a50cb086e30beefde2b42d86))
    - Refactor ([`c5c69bd`](https://github.com/GitoxideLabs/gitoxide/commit/c5c69bd355771a6fb3e4f6db0c5f49aa2bf7f42f))
    - Inline btoi code to reduce compile times ([`f26f298`](https://github.com/GitoxideLabs/gitoxide/commit/f26f2988f51f6c419ec7eff4ae6f4df0f4011663))
    - Merge pull request #1290 from epage/winnow ([`a663e9f`](https://github.com/GitoxideLabs/gitoxide/commit/a663e9fcdb5a3aedc9200da77ebae17d5c3e7135))
    - Update winnow to 0.6 ([`e175b20`](https://github.com/GitoxideLabs/gitoxide/commit/e175b20d431faa6859fbcc52f78400e50f91cad1))
    - Update winnow to 0.5.40 ([`516e105`](https://github.com/GitoxideLabs/gitoxide/commit/516e105db5f22e1483b4b8a886cc4f3929ad7f6a))
    - Merge pull request #1267 from epage/winnow ([`69cb78b`](https://github.com/GitoxideLabs/gitoxide/commit/69cb78bd865a372c580b386766d7b61e5ca9303a))
    - Update from winnow 0.5.31 to 0.5.36 ([`9470554`](https://github.com/GitoxideLabs/gitoxide/commit/94705546cf0e4c8e38bcc96999cfa79cd8ee1acd))
</details>

## 0.44.0 (2024-01-20)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 4 calendar days.
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
</details>

## 0.43.1 (2023-12-30)

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

## 0.43.0 (2023-12-29)

<csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/>

### Chore

- <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
  Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 21 calendar days.
 - 22 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.8.2, gix-hash v0.14.0, gix-trace v0.1.5, gix-features v0.37.0, gix-actor v0.29.0, gix-validate v0.8.2, gix-object v0.40.0, gix-path v0.10.2, gix-glob v0.15.0, gix-quote v0.4.9, gix-attributes v0.21.0, gix-command v0.3.1, gix-packetline-blocking v0.17.1, gix-utils v0.1.7, gix-filter v0.8.0, gix-fs v0.9.0, gix-chunk v0.4.6, gix-commitgraph v0.23.0, gix-hashtable v0.5.0, gix-revwalk v0.11.0, gix-traverse v0.36.0, gix-worktree-stream v0.8.0, gix-archive v0.8.0, gix-config-value v0.14.2, gix-tempfile v12.0.0, gix-lock v12.0.0, gix-ref v0.40.0, gix-sec v0.10.2, gix-config v0.33.0, gix-prompt v0.8.1, gix-url v0.26.0, gix-credentials v0.23.0, gix-ignore v0.10.0, gix-bitmap v0.2.9, gix-index v0.28.0, gix-worktree v0.29.0, gix-diff v0.39.0, gix-discover v0.28.0, gix-macros v0.1.2, gix-mailmap v0.21.0, gix-negotiate v0.11.0, gix-pack v0.46.0, gix-odb v0.56.0, gix-pathspec v0.5.0, gix-packetline v0.17.1, gix-transport v0.40.0, gix-protocol v0.43.0, gix-revision v0.25.0, gix-refspec v0.21.0, gix-status v0.4.0, gix-submodule v0.7.0, gix-worktree-state v0.6.0, gix v0.57.0, gix-fsck v0.2.0, gitoxide-core v0.35.0, gitoxide v0.33.0, safety bump 40 crates ([`e1aae19`](https://github.com/GitoxideLabs/gitoxide/commit/e1aae191d7421c748913c92e2c5883274331dd20))
    - Prepare changelogs of next release ([`e78a92b`](https://github.com/GitoxideLabs/gitoxide/commit/e78a92bfeda168b2f35bb7ba9a94175cdece12f2))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
    - Thanks clippy ([`d38d1cc`](https://github.com/GitoxideLabs/gitoxide/commit/d38d1cc1aa3402629a0f182324e3310e730ce3f2))
    - Merge branch 'main' into fix-1183 ([`1691ba6`](https://github.com/GitoxideLabs/gitoxide/commit/1691ba669537f4a39ebb0891747dc509a6aedbef))
    - Release gix-hash v0.13.3, gix-index v0.27.1 ([`98b08f4`](https://github.com/GitoxideLabs/gitoxide/commit/98b08f4d0d9237be0e0c2caa9bf5c13ae8bbf9d8))
    - Merge branch '32bit' ([`ff1542c`](https://github.com/GitoxideLabs/gitoxide/commit/ff1542cedf3072a8c7c493d454aef5cc61de6d4c))
    - Update `prodash` to latest version ([`97f62cb`](https://github.com/GitoxideLabs/gitoxide/commit/97f62cb183cc36d6045c9320697bb0b4755d6f6b))
</details>

## 0.42.0 (2023-12-06)

### Bug Fixes

 - <csr-id-6295dec2bdd6c3bb35e45db7a486651ebfe50369/> V1 negotiation won't hang anymore
   The logic previously tried to estimate when a pack can be expected,
   and when a NAK is the end of a block, or the beginning of a pack.
   
   This can be known because a pack (with our settings) needs two things:
   
   * the server thinks it's ready
* a `done` sent by the client

### New Features (BREAKING)

 - <csr-id-c3edef1c0c49accbb037bdf086dade3ed0e5e507/> make it possible to trace incoming and outgoing packetlines.
   Due to the way this is (and has to be) setup, unfortunately one
   has to integrate that with two crates, instead of just one.
   
   This changes touches multiple crates, most of which receive a single
   boolean as last argument to indicate whether the tracing should
   happen in the first place.

### Bug Fixes (BREAKING)

 - <csr-id-88f8b342ab317696bcab8a0fe75c042e7290a75c/> Remove unsafe transmute of should_interrupt
   Adds a lifetime to the ExtendedBufRead trait to specify how long the
   callback provided must live.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1061](https://github.com/GitoxideLabs/gitoxide/issues/1061)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1061](https://github.com/GitoxideLabs/gitoxide/issues/1061)**
    - V1 negotiation won't hang anymore ([`6295dec`](https://github.com/GitoxideLabs/gitoxide/commit/6295dec2bdd6c3bb35e45db7a486651ebfe50369))
 * **Uncategorized**
    - Release gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0 ([`d3fd11e`](https://github.com/GitoxideLabs/gitoxide/commit/d3fd11ec3783843d4e49081e1d14359ed9714b5f))
    - Release gix-date v0.8.1, gix-hash v0.13.2, gix-trace v0.1.4, gix-features v0.36.1, gix-actor v0.28.1, gix-validate v0.8.1, gix-object v0.39.0, gix-path v0.10.1, gix-glob v0.14.1, gix-quote v0.4.8, gix-attributes v0.20.1, gix-command v0.3.0, gix-packetline-blocking v0.17.0, gix-utils v0.1.6, gix-filter v0.7.0, gix-fs v0.8.1, gix-chunk v0.4.5, gix-commitgraph v0.22.1, gix-hashtable v0.4.1, gix-revwalk v0.10.0, gix-traverse v0.35.0, gix-worktree-stream v0.7.0, gix-archive v0.7.0, gix-config-value v0.14.1, gix-tempfile v11.0.1, gix-lock v11.0.1, gix-ref v0.39.0, gix-sec v0.10.1, gix-config v0.32.0, gix-prompt v0.8.0, gix-url v0.25.2, gix-credentials v0.22.0, gix-ignore v0.9.1, gix-bitmap v0.2.8, gix-index v0.27.0, gix-worktree v0.28.0, gix-diff v0.38.0, gix-discover v0.27.0, gix-macros v0.1.1, gix-mailmap v0.20.1, gix-negotiate v0.10.0, gix-pack v0.45.0, gix-odb v0.55.0, gix-pathspec v0.4.1, gix-packetline v0.17.0, gix-transport v0.39.0, gix-protocol v0.42.0, gix-revision v0.24.0, gix-refspec v0.20.0, gix-status v0.3.0, gix-submodule v0.6.0, gix-worktree-state v0.5.0, gix v0.56.0, gix-fsck v0.1.0, gitoxide-core v0.34.0, gitoxide v0.32.0, safety bump 27 crates ([`55d386a`](https://github.com/GitoxideLabs/gitoxide/commit/55d386a2448aba1dd22c73fb63b3fd5b3a8401c9))
    - Prepare changelogs prior to release ([`d3dcbe5`](https://github.com/GitoxideLabs/gitoxide/commit/d3dcbe5c4e3a004360d02fbfb74a8fad52f19b5e))
    - Merge branch 'remove-unsafe' ([`d2ba97c`](https://github.com/GitoxideLabs/gitoxide/commit/d2ba97c057de62022d4b8b720750c3a706ac0f9c))
    - Remove unsafe transmute of should_interrupt ([`88f8b34`](https://github.com/GitoxideLabs/gitoxide/commit/88f8b342ab317696bcab8a0fe75c042e7290a75c))
    - Upgrade to `winnow` 0.5.24 ([`abcfb65`](https://github.com/GitoxideLabs/gitoxide/commit/abcfb659786425ec09eff6b644cd2ad36b7d6bc4))
    - J fmt ([`51c7abc`](https://github.com/GitoxideLabs/gitoxide/commit/51c7abc65f368b1b2bd3d82473793d3cd4fcbad5))
    - Merge branch 'check-cfg' ([`5a0d93e`](https://github.com/GitoxideLabs/gitoxide/commit/5a0d93e7522564d126c34ce5d569f9a385698513))
    - Replace all docsrs config by the document-features feature ([`bb3224c`](https://github.com/GitoxideLabs/gitoxide/commit/bb3224c25abf6df50286b3bbdf2cdef01e9eeca1))
    - Merge branch 'size-optimization' ([`c0e72fb`](https://github.com/GitoxideLabs/gitoxide/commit/c0e72fbadc0a494f47a110aebb46462d7b9f5664))
    - Remove CHANGELOG.md from all packages ([`b65a80b`](https://github.com/GitoxideLabs/gitoxide/commit/b65a80b05c9372e752e7e67fcc5c073f71da164a))
    - Merge branch 'fix-v1-negotiation' ([`eb23338`](https://github.com/GitoxideLabs/gitoxide/commit/eb23338b847af2b26c797e6e903969a569deb0a7))
    - Merge branch 'trace-packetlines' ([`e7de4c7`](https://github.com/GitoxideLabs/gitoxide/commit/e7de4c702a223ad9eb19b407391028dcb08d80c4))
    - Make it possible to trace incoming and outgoing packetlines. ([`c3edef1`](https://github.com/GitoxideLabs/gitoxide/commit/c3edef1c0c49accbb037bdf086dade3ed0e5e507))
</details>

## 0.41.1 (2023-10-13)

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

## 0.41.0 (2023-10-12)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 17 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-transport v0.37.1, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0 ([`14ddbd4`](https://github.com/GitoxideLabs/gitoxide/commit/14ddbd4c15128b1d5631a2388a00e024842b7b83))
    - Release gix-hash v0.13.1, gix-features v0.36.0, gix-actor v0.28.0, gix-object v0.38.0, gix-glob v0.14.0, gix-attributes v0.20.0, gix-command v0.2.10, gix-filter v0.6.0, gix-fs v0.8.0, gix-commitgraph v0.22.0, gix-revwalk v0.9.0, gix-traverse v0.34.0, gix-worktree-stream v0.6.0, gix-archive v0.6.0, gix-tempfile v11.0.0, gix-lock v11.0.0, gix-ref v0.38.0, gix-config v0.31.0, gix-url v0.25.0, gix-credentials v0.21.0, gix-diff v0.37.0, gix-discover v0.26.0, gix-ignore v0.9.0, gix-index v0.26.0, gix-mailmap v0.20.0, gix-negotiate v0.9.0, gix-pack v0.44.0, gix-odb v0.54.0, gix-pathspec v0.4.0, gix-packetline v0.16.7, gix-transport v0.37.0, gix-protocol v0.41.0, gix-revision v0.23.0, gix-refspec v0.19.0, gix-worktree v0.27.0, gix-status v0.2.0, gix-submodule v0.5.0, gix-worktree-state v0.4.0, gix v0.55.0, safety bump 37 crates ([`68e5432`](https://github.com/GitoxideLabs/gitoxide/commit/68e54326e527a55dd5b5079921fc251615833040))
    - Prepare changelogs prior to release ([`1347a54`](https://github.com/GitoxideLabs/gitoxide/commit/1347a54f84599d8f0aa935d6e64b16c2298d25cf))
</details>

## 0.40.0 (2023-09-24)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 15 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-features v0.35.0, gix-actor v0.27.0, gix-object v0.37.0, gix-glob v0.13.0, gix-attributes v0.19.0, gix-filter v0.5.0, gix-fs v0.7.0, gix-commitgraph v0.21.0, gix-revwalk v0.8.0, gix-traverse v0.33.0, gix-worktree-stream v0.5.0, gix-archive v0.5.0, gix-tempfile v10.0.0, gix-lock v10.0.0, gix-ref v0.37.0, gix-config v0.30.0, gix-url v0.24.0, gix-credentials v0.20.0, gix-diff v0.36.0, gix-discover v0.25.0, gix-ignore v0.8.0, gix-index v0.25.0, gix-mailmap v0.19.0, gix-negotiate v0.8.0, gix-pack v0.43.0, gix-odb v0.53.0, gix-pathspec v0.3.0, gix-transport v0.37.0, gix-protocol v0.40.0, gix-revision v0.22.0, gix-refspec v0.18.0, gix-status v0.1.0, gix-submodule v0.4.0, gix-worktree v0.26.0, gix-worktree-state v0.3.0, gix v0.54.0, gitoxide-core v0.32.0, gitoxide v0.30.0, safety bump 37 crates ([`7891fb1`](https://github.com/GitoxideLabs/gitoxide/commit/7891fb17348ec2f4c997665f9a25be36e2713da4))
    - Prepare changelogs prior to release ([`8a60d5b`](https://github.com/GitoxideLabs/gitoxide/commit/8a60d5b80877c213c3b646d3061e8a33e0e433ec))
</details>

## 0.39.0 (2023-09-08)

<csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/>

### Chore (BREAKING)

- <csr-id-ed327f6163f54756e58c20f86a563a97efb256ca/> update to the latest `prodash`
  It makes proper usage of `Progress` types easier and allows them to be used
  as `dyn` traits as well.

### New Features (BREAKING)

 - <csr-id-24dd870919ba444aa8099c63a78ea120d47ec28e/> use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress`

### Bug Fixes (BREAKING)

 - <csr-id-072ee32f693a31161cd6a843da6582d13efbb20b/> use `dyn` trait where possible.
   This reduces compile time due to avoiding duplication.

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
    - Release gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0 ([`1ff3064`](https://github.com/GitoxideLabs/gitoxide/commit/1ff30641b8724efd6699d8bef5c71d28454e98b9))
    - Release gix-date v0.8.0, gix-hash v0.13.0, gix-features v0.34.0, gix-actor v0.26.0, gix-object v0.36.0, gix-path v0.10.0, gix-glob v0.12.0, gix-attributes v0.18.0, gix-packetline-blocking v0.16.6, gix-filter v0.4.0, gix-fs v0.6.0, gix-commitgraph v0.20.0, gix-hashtable v0.4.0, gix-revwalk v0.7.0, gix-traverse v0.32.0, gix-worktree-stream v0.4.0, gix-archive v0.4.0, gix-config-value v0.14.0, gix-tempfile v9.0.0, gix-lock v9.0.0, gix-ref v0.36.0, gix-sec v0.10.0, gix-config v0.29.0, gix-prompt v0.7.0, gix-url v0.23.0, gix-credentials v0.19.0, gix-diff v0.35.0, gix-discover v0.24.0, gix-ignore v0.7.0, gix-index v0.24.0, gix-macros v0.1.0, gix-mailmap v0.18.0, gix-negotiate v0.7.0, gix-pack v0.42.0, gix-odb v0.52.0, gix-pathspec v0.2.0, gix-packetline v0.16.6, gix-transport v0.36.0, gix-protocol v0.39.0, gix-revision v0.21.0, gix-refspec v0.17.0, gix-submodule v0.3.0, gix-worktree v0.25.0, gix-worktree-state v0.2.0, gix v0.53.0, safety bump 39 crates ([`8bd0456`](https://github.com/GitoxideLabs/gitoxide/commit/8bd045676bb2cdc02624ab93e73ff8518064ca38))
    - Prepare changelogs for release ([`375db06`](https://github.com/GitoxideLabs/gitoxide/commit/375db06a8442378c3f7a922fae38e2a6694d9d04))
    - Merge branch `dyn`ification ([`f658fcc`](https://github.com/GitoxideLabs/gitoxide/commit/f658fcc52dc2200ae34ca53dc10be97fb9012057))
    - Use `dyn` trait where possible. ([`072ee32`](https://github.com/GitoxideLabs/gitoxide/commit/072ee32f693a31161cd6a843da6582d13efbb20b))
    - Use `prodash::Count` to indicate that nothing more than counting is performed, in place of `prodash::Progress` ([`24dd870`](https://github.com/GitoxideLabs/gitoxide/commit/24dd870919ba444aa8099c63a78ea120d47ec28e))
    - Merge pull request #1009 from epage/proto ([`16cac47`](https://github.com/GitoxideLabs/gitoxide/commit/16cac4724347401d7dde2c8567b2f26420bf7216))
    - Don't bother with error tracking that is ignored ([`444f970`](https://github.com/GitoxideLabs/gitoxide/commit/444f970e70e8d24c6593293fec70b3a6f7f2d5bf))
    - Upgrade gix-protocol to winnow 0.5 ([`237657e`](https://github.com/GitoxideLabs/gitoxide/commit/237657ee59a093a65b3157e637f28282290212bb))
    - Prep for winnow 0.5 upgrade for gix-protocol ([`8c80dba`](https://github.com/GitoxideLabs/gitoxide/commit/8c80dba1836394b4d1995580f60e1443a386bdac))
    - Resolve winnow deprecations in gix-protocol ([`b213889`](https://github.com/GitoxideLabs/gitoxide/commit/b213889db2697d27d70b110c863bdfdf71bc0f4f))
    - Upgrade gix-protocol to winnow 0.4 ([`659ebe9`](https://github.com/GitoxideLabs/gitoxide/commit/659ebe9cf3569f40c0d81eb02b802516c428287a))
    - Prep for winnow 0.4 upgrade for gix-protocol ([`8609aec`](https://github.com/GitoxideLabs/gitoxide/commit/8609aec804a7191b9c5a074595ef16a08e7d0be4))
    - Resolve winnow deprecations in gix-protocol ([`b46fe9c`](https://github.com/GitoxideLabs/gitoxide/commit/b46fe9c15f6af4351c6ea7a443f0c79bd0c77969))
    - Migrate gix-protocol to winnow ([`0e8edef`](https://github.com/GitoxideLabs/gitoxide/commit/0e8edef9882eea72a70b8782d8cf6edc8185b097))
    - Update to the latest `prodash` ([`ed327f6`](https://github.com/GitoxideLabs/gitoxide/commit/ed327f6163f54756e58c20f86a563a97efb256ca))
    - Merge branch 'adjustments-for-cargo' ([`b7560a2`](https://github.com/GitoxideLabs/gitoxide/commit/b7560a2445b62f888bf5aa2ba4c5a47ae037cb23))
    - Release gix-date v0.7.4, gix-index v0.23.0, safety bump 5 crates ([`3be2b1c`](https://github.com/GitoxideLabs/gitoxide/commit/3be2b1ccfe30eeae45711c64b88efc522a2b51b7))
    - Merge branch 'gix-submodule' ([`363ee77`](https://github.com/GitoxideLabs/gitoxide/commit/363ee77400805f473c9ad66eadad9214e7ab66f4))
</details>

## 0.38.0 (2023-08-22)

<csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/>

### Chore

- <csr-id-229bd4899213f749a7cc124aa2b82a1368fba40f/> don't call crate 'WIP' in manifest anymore.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 15 calendar days.
 - 19 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0 ([`6c62e74`](https://github.com/GitoxideLabs/gitoxide/commit/6c62e748240ac0980fc23fdf30f8477dea8b9bc3))
    - Release gix-date v0.7.3, gix-hash v0.12.0, gix-features v0.33.0, gix-actor v0.25.0, gix-object v0.35.0, gix-path v0.9.0, gix-glob v0.11.0, gix-quote v0.4.7, gix-attributes v0.17.0, gix-command v0.2.9, gix-packetline-blocking v0.16.5, gix-filter v0.3.0, gix-fs v0.5.0, gix-commitgraph v0.19.0, gix-hashtable v0.3.0, gix-revwalk v0.6.0, gix-traverse v0.31.0, gix-worktree-stream v0.3.0, gix-archive v0.3.0, gix-config-value v0.13.0, gix-tempfile v8.0.0, gix-lock v8.0.0, gix-ref v0.35.0, gix-sec v0.9.0, gix-config v0.28.0, gix-prompt v0.6.0, gix-url v0.22.0, gix-credentials v0.18.0, gix-diff v0.34.0, gix-discover v0.23.0, gix-ignore v0.6.0, gix-bitmap v0.2.7, gix-index v0.22.0, gix-mailmap v0.17.0, gix-negotiate v0.6.0, gix-pack v0.41.0, gix-odb v0.51.0, gix-pathspec v0.1.0, gix-packetline v0.16.5, gix-transport v0.35.0, gix-protocol v0.38.0, gix-revision v0.20.0, gix-refspec v0.16.0, gix-submodule v0.2.0, gix-worktree v0.24.0, gix-worktree-state v0.1.0, gix v0.52.0, gitoxide-core v0.31.0, gitoxide v0.29.0, safety bump 41 crates ([`30b2761`](https://github.com/GitoxideLabs/gitoxide/commit/30b27615047692d3ced1b2d9c2ac15a80f79fbee))
    - Update changelogs prior to release ([`f23ea88`](https://github.com/GitoxideLabs/gitoxide/commit/f23ea8828f2d9ba7559973daca388c9591bcc5fc))
    - Don't call crate 'WIP' in manifest anymore. ([`229bd48`](https://github.com/GitoxideLabs/gitoxide/commit/229bd4899213f749a7cc124aa2b82a1368fba40f))
    - Release gix-glob v0.10.2, gix-date v0.7.2, gix-validate v0.8.0, gix-object v0.34.0, gix-ref v0.34.0, gix-config v0.27.0, gix-commitgraph v0.18.2, gix-revwalk v0.5.0, gix-revision v0.19.0, gix-refspec v0.15.0, gix-submodule v0.1.0, safety bump 18 crates ([`4604f83`](https://github.com/GitoxideLabs/gitoxide/commit/4604f83ef238dc07c85aaeae097399b67f3cfd0c))
</details>

## 0.37.0 (2023-08-02)

### New Features

 - <csr-id-d1f9c63f8a79dcb00766d94edda80ed9add724ea/> add `fetch::Arguments::add_feature()` for unconditional additions of arguments that are supposed to be features.
   This differentiation is important to support V1 and V2 correctly, which have a different notion of features with V1
   special-casing them, and V2 just sees everything as arguments.

### Bug Fixes (BREAKING)

 - <csr-id-0de7117492fb64c4f2b4f5ee7482813d57b57d40/> `handshake::Ref::Symbolic` now has a `tag` field.
   This allows symbolic refs to be parsed which are in fact pointing
   to an annotated tag as in-between object.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 1 calendar day.
 - 10 days passed between releases.
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
    - Add `fetch::Arguments::add_feature()` for unconditional additions of arguments that are supposed to be features. ([`d1f9c63`](https://github.com/GitoxideLabs/gitoxide/commit/d1f9c63f8a79dcb00766d94edda80ed9add724ea))
    - `handshake::Ref::Symbolic` now has a `tag` field. ([`0de7117`](https://github.com/GitoxideLabs/gitoxide/commit/0de7117492fb64c4f2b4f5ee7482813d57b57d40))
</details>

## 0.36.1 (2023-07-22)

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

## 0.36.0 (2023-07-19)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 19 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`4aca8c2`](https://github.com/GitoxideLabs/gitoxide/commit/4aca8c2ae2ec588fb65ec4faa0c07c19d219569f))
    - Release gix-features v0.32.0, gix-actor v0.24.0, gix-glob v0.10.0, gix-attributes v0.15.0, gix-commitgraph v0.18.0, gix-config-value v0.12.4, gix-fs v0.4.0, gix-object v0.33.0, gix-ref v0.33.0, gix-config v0.26.0, gix-command v0.2.7, gix-url v0.21.0, gix-credentials v0.17.0, gix-diff v0.33.0, gix-discover v0.22.0, gix-filter v0.1.0, gix-ignore v0.5.0, gix-revwalk v0.4.0, gix-traverse v0.30.0, gix-index v0.21.0, gix-mailmap v0.16.0, gix-negotiate v0.5.0, gix-pack v0.40.0, gix-odb v0.50.0, gix-transport v0.34.0, gix-protocol v0.36.0, gix-revision v0.18.0, gix-refspec v0.14.0, gix-worktree v0.22.0, gix v0.49.0 ([`68ae3ff`](https://github.com/GitoxideLabs/gitoxide/commit/68ae3ff9d642ec56f088a6a682a073dc16f4e8ca))
    - Adjust package versions (by cargo-smart-release) ([`c70e54f`](https://github.com/GitoxideLabs/gitoxide/commit/c70e54f163c312c87753a506eeaad462e8579bfb))
    - Prepare changelogs prior to release ([`e4dded0`](https://github.com/GitoxideLabs/gitoxide/commit/e4dded05138562f9737a7dcfb60570c55769486d))
</details>

## 0.35.0 (2023-06-29)

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
    - Release gix-date v0.7.0, gix-trace v0.1.2, gix-actor v0.23.0, gix-commitgraph v0.17.1, gix-utils v0.1.4, gix-object v0.32.0, gix-ref v0.32.0, gix-config v0.25.0, gix-diff v0.32.0, gix-discover v0.21.0, gix-hashtable v0.2.3, gix-revwalk v0.3.0, gix-traverse v0.29.0, gix-index v0.20.0, gix-mailmap v0.15.0, gix-negotiate v0.4.0, gix-pack v0.39.0, gix-odb v0.49.0, gix-protocol v0.35.0, gix-revision v0.17.0, gix-refspec v0.13.0, gix-worktree v0.21.0, gix v0.48.0, safety bump 20 crates ([`27e8c18`](https://github.com/GitoxideLabs/gitoxide/commit/27e8c18db5a9a21843381c116a8ed6d9f681b3f8))
    - Prepare changelogs prior to release ([`00f96fb`](https://github.com/GitoxideLabs/gitoxide/commit/00f96fb3110a8f81a1bd0d74c757c15b8773c6f6))
</details>

## 0.34.0 (2023-06-22)

<csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/>

### Chore

- <csr-id-bcad5c22049d56a25ef69d6c7a3344e78f9a1d4d/> Add `clippy::redundant-closure-for-method-calls` lint

### Changed (BREAKING)

 - <csr-id-9f8fa221b0e15923de5fc015cf6abb3dd578167e/> use `SecondsSinceUnixEpoch` type where needed.
   Previously we used `usize` which isn't correct on 32 bit systems.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 11 calendar days.
 - 12 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Merge branch 'future-dates' ([`8d2e6a9`](https://github.com/GitoxideLabs/gitoxide/commit/8d2e6a91ac92a033e9e3daad5cffa90263075536))
    - Use `SecondsSinceUnixEpoch` type where needed. ([`9f8fa22`](https://github.com/GitoxideLabs/gitoxide/commit/9f8fa221b0e15923de5fc015cf6abb3dd578167e))
</details>

## 0.33.2 (2023-06-10)

### New Features

 - <csr-id-c8e8b9fc55486215bda4c380d3c84a6c6ae5f602/> support `capabilities^{}` syntax to announce caps in empty repos under V1
   This is supposed to work for existing code, but doesn't in our case as we really
   want to associate a branch. In this case, we can ignore the ref as it's a null
   hash anyway.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-attributes v0.13.1, gix-diff v0.30.1, gix-revwalk v0.1.0, gix-traverse v0.27.0, gix-index v0.18.0, gix-revision v0.15.2, gix-negotiate v0.2.1, gix-pack v0.37.0, gix-odb v0.47.0, gix-protocol v0.33.2, gix-worktree v0.19.0, gix v0.46.0, safety bump 7 crates ([`2560a2c`](https://github.com/GitoxideLabs/gitoxide/commit/2560a2cc3e1d8c60cd812e15696fa4761d036e19))
    - Prepare changelogs prior to release ([`298f3d7`](https://github.com/GitoxideLabs/gitoxide/commit/298f3d7359c5b183314d8c584e45dcdd559d88b3))
    - Support `capabilities^{}` syntax to announce caps in empty repos under V1 ([`c8e8b9f`](https://github.com/GitoxideLabs/gitoxide/commit/c8e8b9fc55486215bda4c380d3c84a6c6ae5f602))
</details>

## 0.33.1 (2023-06-07)

### Bug Fixes

 - <csr-id-88670c2ff995e4478f4a40b2733c6c8ca811fa15/> assure we don't stop early on NAK if READY was sent to handle V1 specialty.
   Seeing READY means a pack will follow, which is exactly the kind of knowledge we want.
   Thus we now either listen to the client OR to what the server says.
   
   Note that this remedy relies on `multi-ack-detailed`, which some very old servers might
   not support. For those we would be out-of-luck, but that seems acceptable.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#882](https://github.com/GitoxideLabs/gitoxide/issues/882)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#882](https://github.com/GitoxideLabs/gitoxide/issues/882)**
    - Assure we don't stop early on NAK if READY was sent to handle V1 specialty. ([`88670c2`](https://github.com/GitoxideLabs/gitoxide/commit/88670c2ff995e4478f4a40b2733c6c8ca811fa15))
 * **Uncategorized**
    - Release gix-protocol v0.33.1 ([`9c99ed3`](https://github.com/GitoxideLabs/gitoxide/commit/9c99ed30162081a7f26d72e0ed26966ff62d2b1c))
</details>

## 0.33.0 (2023-06-06)

### New Features

 - <csr-id-877aa2921d8491d301945f86d5a69382e40fb081/> Add `fetch::Arguments::is_stateless()` to aid proper use of arguments.
   When arguments are used, haves are reset every round in stateless protocols, while
   everything else is repeated. However, this also means that previously confirmed common
   commits aren't repeated unless this is specifically implemented by the user of `Arguments`.
   
   That caller can now easily determine if negotiations have to be compensated for.
   
   Please note that `Arguments` explicitly doesn't implement repeating of all prior arguments, which
   would also repeat a lot of *in-vain* haves.

### Bug Fixes

 - <csr-id-c5dc7b4c43f07c04cdfb218de03e6725ff3fdb64/> `include-tag` is now properly handled in V1 fetch arguments
   Previously it would be added like it's V2 arguments, which makes using it
   in V1 impossible.

### Bug Fixes (BREAKING)

 - <csr-id-6a3c02131c9ebca911be5f751e5a6c67fbdbf609/> make V1 stateless negotations work.
   This is done by working around another V1 negotiation oddity
   by exploiting client-side knowledge. For very old servers, we probably
   wouldn't be able to do multi-rounds without dead-locking, but with
   recent-enough (probably 10 years or so) old git servers all should
   work fine.
   
   All this to not actually have to implement the V1 strangeness, allowing
   our code to work smoothly with all permutations of stateless/stateful connections
   and V1/V2 interactions, with a single high-level implementation essentially.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 12 calendar days.
 - 40 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.5.1, gix-hash v0.11.2, gix-features v0.30.0, gix-actor v0.21.0, gix-path v0.8.1, gix-glob v0.8.0, gix-quote v0.4.4, gix-attributes v0.13.0, gix-chunk v0.4.2, gix-commitgraph v0.16.0, gix-config-value v0.12.1, gix-fs v0.2.0, gix-tempfile v6.0.0, gix-utils v0.1.2, gix-lock v6.0.0, gix-validate v0.7.5, gix-object v0.30.0, gix-ref v0.30.0, gix-sec v0.8.1, gix-config v0.23.0, gix-command v0.2.5, gix-prompt v0.5.1, gix-url v0.19.0, gix-credentials v0.15.0, gix-diff v0.30.0, gix-discover v0.19.0, gix-hashtable v0.2.1, gix-ignore v0.3.0, gix-bitmap v0.2.4, gix-traverse v0.26.0, gix-index v0.17.0, gix-mailmap v0.13.0, gix-revision v0.15.0, gix-negotiate v0.2.0, gix-pack v0.36.0, gix-odb v0.46.0, gix-packetline v0.16.2, gix-transport v0.32.0, gix-protocol v0.33.0, gix-refspec v0.11.0, gix-worktree v0.18.0, gix v0.45.0, safety bump 29 crates ([`9a9fa96`](https://github.com/GitoxideLabs/gitoxide/commit/9a9fa96fa8a722bddc5c3b2270b0edf8f6615141))
    - Prepare changelogs prior to release ([`8f15cec`](https://github.com/GitoxideLabs/gitoxide/commit/8f15cec1ec7d5a9d56bb158f155011ef2bb3539b))
    - Merge branch 'integrate-gix-negotiate' ([`ae845de`](https://github.com/GitoxideLabs/gitoxide/commit/ae845dea6cee6523c88a23d7a14293589cf8092f))
    - Make V1 stateless negotations work. ([`6a3c021`](https://github.com/GitoxideLabs/gitoxide/commit/6a3c02131c9ebca911be5f751e5a6c67fbdbf609))
    - `include-tag` is now properly handled in V1 fetch arguments ([`c5dc7b4`](https://github.com/GitoxideLabs/gitoxide/commit/c5dc7b4c43f07c04cdfb218de03e6725ff3fdb64))
    - Add `fetch::Arguments::is_stateless()` to aid proper use of arguments. ([`877aa29`](https://github.com/GitoxideLabs/gitoxide/commit/877aa2921d8491d301945f86d5a69382e40fb081))
    - Adapt to changes in `gix-transport` ([`60eaceb`](https://github.com/GitoxideLabs/gitoxide/commit/60eaceb5cd4b9206e9262cf05d9036c31a7b7d11))
    - Merge branch 'fix-docs' ([`420553a`](https://github.com/GitoxideLabs/gitoxide/commit/420553a10d780e0b2dc466cac120989298a5f187))
    - Cleaning up documentation ([`2578e57`](https://github.com/GitoxideLabs/gitoxide/commit/2578e576bfa365d194a23a1fb0bf09be230873de))
    - Apply -W clippy::cloned-instead-of-copied ([`150463c`](https://github.com/GitoxideLabs/gitoxide/commit/150463c26f0d2e1c2b5facba731ccba29cf23228))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
</details>

## 0.32.0 (2023-04-27)

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

## 0.31.0 (2023-04-26)

### New Features (BREAKING)

 - <csr-id-b83ee366a3c65c717beb587ad809268f1c54b8ad/> Rename `serde1` cargo feature to `serde` and use the weak-deps cargo capability.
   With it it's possible to not automatically declare all optional dependencies externally visible
   features, and thus re-use feature names that oterwise are also a crate name.
   
   Previously I thought that `serde1` is for future-proofing and supporting multiple serde versions
   at the same time. However, it's most definitely a burden I wouldn't want anyway, so using
   `serde` seems to be the way to go into the future.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 14 calendar days.
 - 27 days passed between releases.
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
    - Release gix-utils v0.1.0, gix-hash v0.11.0, gix-date v0.5.0, gix-features v0.29.0, gix-actor v0.20.0, gix-object v0.29.0, gix-archive v0.1.0, gix-fs v0.1.0, safety bump 25 crates ([`8dbd0a6`](https://github.com/GitoxideLabs/gitoxide/commit/8dbd0a60557a85acfa231800a058cbac0271a8cf))
    - Merge branch 'main' into dev ([`cdef398`](https://github.com/GitoxideLabs/gitoxide/commit/cdef398c4a3bd01baf0be2c27a3f77a400172b0d))
    - Rename the serde1 feature to serde ([`19338d9`](https://github.com/GitoxideLabs/gitoxide/commit/19338d934b6712b7d6bd3fa3b2e4189bf7e6c8a1))
    - Release gix-hash v0.10.4, gix-hashtable v0.1.3 ([`b574a39`](https://github.com/GitoxideLabs/gitoxide/commit/b574a3904203762a6b9e475e16a7c358d7616599))
</details>

## 0.30.2 (2023-03-30)

### Documentation

 - <csr-id-cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5/> fix minor typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 1 calendar day.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-path v0.7.3, gix-config-value v0.10.2, gix-config v0.20.1, gix-discover v0.16.2, gix-index v0.15.1, gix-odb v0.43.1, gix-packetline v0.15.1, gix-protocol v0.30.2, gix-worktree v0.15.2, gix v0.43.1 ([`38eed1d`](https://github.com/GitoxideLabs/gitoxide/commit/38eed1d06e7cbb8fbcd54b2cad3163ca45e0baf1))
    - Fix minor typos ([`cc48c35`](https://github.com/GitoxideLabs/gitoxide/commit/cc48c35d0ecf35824910c5b6ecc62fe9b2aff1b5))
</details>

## 0.30.1 (2023-03-26)

### Bug Fixes

 - <csr-id-d4c82e7e7139ae251300d4b71bcd8ed0255a163a/> don't assume that credential providers always return credentials or error during handshake.
   The type-system allows this even though when getting credentials this should never happen and won't
   in the default implementation. However, users can of course return nothing and provide no error which
   is when we have to create one, instead of panicking.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v5.0.2, gix-validate v0.7.4, gix-config v0.20.0, gix-prompt v0.3.3, gix-diff v0.28.1, gix-discover v0.16.1, gix-pack v0.33.2, gix-transport v0.29.1, gix-protocol v0.30.1, gix-revision v0.12.1, gix-worktree v0.15.1, gix v0.43.0, safety bump gix v0.43.0 ([`5dc1f9f`](https://github.com/GitoxideLabs/gitoxide/commit/5dc1f9f2bcb8b3e147115fcb6f76558e8f48ffef))
    - Prepare changelogs prior to release ([`3016a28`](https://github.com/GitoxideLabs/gitoxide/commit/3016a285f566bdfe7de2774fa6f2254c1b1a2c51))
    - Don't assume that credential providers always return credentials or error during handshake. ([`d4c82e7`](https://github.com/GitoxideLabs/gitoxide/commit/d4c82e7e7139ae251300d4b71bcd8ed0255a163a))
</details>

## 0.30.0 (2023-03-14)

A maintenance release without any user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 3 calendar days.
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
    - Merge branch 'various-fixes' ([`cc0f506`](https://github.com/GitoxideLabs/gitoxide/commit/cc0f5061fba27d57022dc616c941034b98fd4875))
    - Adjust to changes in `gix-packetline` ([`4f45814`](https://github.com/GitoxideLabs/gitoxide/commit/4f45814eea9c20b449effd9b29d31623943ff853))
    - Merge branch 'shallow-protocol' ([`531dd19`](https://github.com/GitoxideLabs/gitoxide/commit/531dd19502b8b635fb1a60f747eb381fd12e00ca))
    - Merge branch 'fix-cred-helper' ([`01277a6`](https://github.com/GitoxideLabs/gitoxide/commit/01277a681e4997896e04567490c572b5af606f35))
</details>

## 0.29.0 (2023-03-10)

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
    - Release gix-tempfile v5.0.0, gix-lock v5.0.0, gix-ref v0.27.0, gix-config v0.19.0, gix-url v0.16.0, gix-credentials v0.12.0, gix-discover v0.16.0, gix-index v0.15.0, gix-pack v0.33.0, gix-odb v0.43.0, gix-transport v0.28.0, gix-protocol v0.29.0, gix-worktree v0.15.0, gix v0.41.0, safety bump 12 crates ([`29a0870`](https://github.com/GitoxideLabs/gitoxide/commit/29a087043d1feb2f127b065341c8028d0bd0301e))
    - Prepare changelogs prior to release ([`e06f5f5`](https://github.com/GitoxideLabs/gitoxide/commit/e06f5f523e83f4da390eddbebcb9a2d58674587b))
    - Add more fetch::Arguments tests ([`ac34d2e`](https://github.com/GitoxideLabs/gitoxide/commit/ac34d2e96d7904fdf0c654a64260be0ec106ae0f))
    - Adapt to changes in `gix-transport` and add V2 test for unshallow fetching ([`0eecfab`](https://github.com/GitoxideLabs/gitoxide/commit/0eecfab26972fe784b7945b299e06624c922fe33))
</details>

## 0.28.0 (2023-03-04)

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

## 0.27.0 (2023-03-01)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 8 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-tempfile v4.1.0, gix-lock v4.0.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0, safety bump 6 crates ([`ea9fd1d`](https://github.com/GitoxideLabs/gitoxide/commit/ea9fd1d9b60e1e9e17042e9e37c06525823c40a5))
    - Release gix-features v0.27.0, gix-actor v0.18.0, gix-quote v0.4.3, gix-attributes v0.9.0, gix-object v0.27.0, gix-ref v0.25.0, gix-config v0.17.0, gix-url v0.14.0, gix-credentials v0.10.0, gix-diff v0.27.0, gix-discover v0.14.0, gix-hashtable v0.1.2, gix-bitmap v0.2.2, gix-traverse v0.23.0, gix-index v0.13.0, gix-mailmap v0.10.0, gix-pack v0.31.0, gix-odb v0.41.0, gix-transport v0.26.0, gix-protocol v0.27.0, gix-revision v0.11.0, gix-refspec v0.8.0, gix-worktree v0.13.0, gix v0.38.0 ([`e6cc618`](https://github.com/GitoxideLabs/gitoxide/commit/e6cc6184a7a49dbc2503c1c1bdd3688ca5cec5fe))
    - Adjust manifests prior to release ([`addd789`](https://github.com/GitoxideLabs/gitoxide/commit/addd78958fdd1e54eb702854e96079539d01965a))
    - Prepare changelogs prior to release ([`94c99c7`](https://github.com/GitoxideLabs/gitoxide/commit/94c99c71520f33269cc8dbc26f82a74747cc7e16))
</details>

## 0.26.4 (2023-02-20)

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

 - 3 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-date v0.4.3, gix-hash v0.10.3, gix-features v0.26.5, gix-actor v0.17.2, gix-glob v0.5.5, gix-path v0.7.2, gix-quote v0.4.2, gix-attributes v0.8.3, gix-validate v0.7.3, gix-object v0.26.2, gix-ref v0.24.1, gix-config v0.16.2, gix-command v0.2.4, gix-url v0.13.3, gix-credentials v0.9.2, gix-discover v0.13.1, gix-index v0.12.4, gix-mailmap v0.9.3, gix-pack v0.30.3, gix-packetline v0.14.3, gix-transport v0.25.6, gix-protocol v0.26.4, gix-revision v0.10.4, gix-refspec v0.7.3, gix-worktree v0.12.3, gix v0.36.1 ([`9604783`](https://github.com/GitoxideLabs/gitoxide/commit/96047839a20a657a559376b0b14c65aeab96acbd))
    - Compatibility with `bstr` v1.3, use `*.as_bytes()` instead of `.as_ref()`. ([`135d317`](https://github.com/GitoxideLabs/gitoxide/commit/135d317065aae87af302beb6c26bb6ca8e30b6aa))
    - Release gix-transport v0.25.5 ([`f872ba8`](https://github.com/GitoxideLabs/gitoxide/commit/f872ba8271a5d632acc071e7a857ef19f7cf5610))
</details>

## 0.26.3 (2023-02-17)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Bug Fixes (BREAKING)

 - <csr-id-da68bfb8104ecf58e73e3f99d87f81c90712a2ca/> fetch::Ref::Symbolic::target is now an option…
   …to represent the possibility that the server may not be able to
   strip its namespace if it is not the currently set namespace.
   
   Note that the object it points to might still be resolvable and
   pointing to another namespace.

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
 - <csr-id-02e37f04d7dfeab7067c8273bcd76408cb1f4852/> Support for `unborn` ls-refs capability if server supports it.
   We can also parse it, adding yet another variant to `fetch::Refs`.
 - <csr-id-32dc1829a5661f66396d109c8d0a8eaae6b1f532/> use `git-credentials` in `git-protocol`

### Changed (BREAKING)

 - <csr-id-09070a7c17f39383730c3a2b809eec677f79f386/> move `fetch::refs(…, protocol, …)` to the crate root as `ls_refs(…)`.
   This move also removes the `protocol` parameter as ls-refs is only available
   in V2 and the caller is forced to know that after running a
   `handshake(…)` anyway which yields the actual protocol used by the
   server.
   
   Note that the client can suggest the version to use when connecting,
   which is specific to the transport at hand.
 - <csr-id-35f7b4df164c130fb50fcffcf5de99816c2ca872/> move `fetch::Command` into the crate root.
   This represents much better on how it is actually used, which is for
   validation of any Command invoked through the protocol independenty.
 - <csr-id-a03f8f6cce34618883e8448dd1c31b41c54d9448/> move `fetch::agent|indicate_end_of_interaction` to crate root
   These are generally useful in all interactions, including push.
 - <csr-id-a3bcf82ae50defa4439862943008647d03d09792/> `handshake(…)` is now generalized to support `git-receive-pack` as well.
   Note that `fetch::handshake()` is still present, selecting the correct
   service like before, but most of the `fetch::Ref` parsing is now handled
   by `handshake::Ref` in it's more general location.
   
   There is still `fetch::refs(…)` which just invokes a V2 ls-refs command
   (without that being generalized) as `git-receive-pack` just supports
   V1 at the moment, and making the `fetch::LsRefs` Command more general
   would mean that every command has to be general, at is doesn't matter
   anymore if it's used for Fetch or Push.
   
   As long as git won't support V2 for pushes, which it never might,
   there is no need to introduce this breakage even though it
   should probably be done before going 1.0.
 - <csr-id-d05b0b800a553e1e380801fb141e9aa054a6cbd0/> `fetch::agent()` returns agent string only; `fetch()` takes agent name as parameter.
 - <csr-id-aab278f2a3e07edb6d8109e7ffba003b5d5d7857/> don't auto-insert user agent, instead prepare to insert it later.
 - <csr-id-f957d9a1833af079b01ba6bd6941eb4af6c9e436/> `fetch::agent()` now returns the agent name, `fetch::agent_tuple()` returns specific key-value pair.
   This is in preparation for allowing user-defined agent strings.
 - <csr-id-cd867ade55eaec138029c0cf17fa83e0679ef7fc/> `fetch::Ref::unpack()` returns `Option<oid>`.
   That way the caller has to be aware of the possibility of an unborn
   branch (probably the only unborn branch) on the remote.
 - <csr-id-91f193f73ae37314319f7d055893b95431fd018e/> `fetch::Ref::unpack()` returns `&BStr` instead of `&BString` as ref name.
 - <csr-id-d2c772e28619f7602ab01285b25e79a7040c9aab/> `fetch::Ref::unpack()` now returns the peeled object as well.
 - <csr-id-bab586099a6e21e54932218f7a5c14f8bfc6cabd/> rename `fetch::Ref::path` to `fetch::Ref::full_ref_name`.
   Previously it was inaccurately named and described due to an
   insufficient understanding of the ref database.
 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`
 - <csr-id-e91c301342d44cff35ebe12fba4ec10afb4a1922/> replace `quick-error` with `this-error`
 - <csr-id-12589cc6f08e4d7aabae30bcdadaa0c2b4850229/> adapt to changes in `git-url` and use `BString` to represent URLs.
   They can contain paths, which is why `String` can't repsent a URL
   losslessly.
   
   For HTTP urls these are ultimately UTF-8 strings though.

### Reverted

 - <csr-id-f87b7eb4dc05b73f33ac554b57d50f3eee7f84c1/> `client::fetch_pack()` isn't worth it after all.
   The overhead required to make it work isn't worth the logic we are
   enabling. Instead, it's OK to re-implement custom logic on the consumer
   side based on this one.

### Bug Fixes

 - <csr-id-5f2276b63129163096be3cb229864fc589348da8/> don't enforce V2 as protocol, but smoothly downgrade like git does.
   For backward compatibility the shared handshake implementation allows the
   transport to control which protocol versions it wants to support
   to allow optiomizing for one special case, namely to prevent it to
   read all V1 refs on old servers but abort instead, closing the connection
   without delay.
   
   Now we leave this feature for custom transports (who usually come with custom
   servers) and instead support fallbacks to other protocols if the server
   demands it.
 - <csr-id-a8727824a4c38b27d6ad7c73ccaf7a45839c0aa9/> don't pass 'include-tag' as argument by default.
   This flag always has to be added by the caller based on configuration.
 - <csr-id-24000fa7ad4607c23f6a4fb4d84d2c85d96ee01f/> don't post `packfile-uris` capability unless we are able to make use of it.
   Turns out that this option can be rejected by a server that advertises
   it, so definitely needs more work to make it work.
 - <csr-id-806b8c2ef392137f3a6ebd0f28da2a3a07a9f3eb/> Parse refs from bytes, not from String.
   The latter can cause issues around illformed UTF-8 which wouldn't
   bother git either.
   
   This comes at the expense of not parsing line by line anymore, but
   instead reading as fast as possible and parsing afterwards.
   
   Performance wise I think it doesn't matter, but it will cause
   more memory to be used. If this ever becomes a problem,
   for example during pushes where we are stuck with V1, we can consider
   implementing our own streaming appreach that works with packet lines
   instead - they are just not exposed here even though they could.
 - <csr-id-d7f62b441700c6d3526517c8c4f369cb9a72c102/> support keepalive packets.
   Keepalive packets are side-band only empty datalines that should
   just be ignored. This is now happening, allowing longer git
   operations to work as they will send keepalive packets every 5 seconds,
   and previously we would choke on it.
   
   Note that empty datalines are never send otherwise, making it a
   previously unused marker that can safely be skipped.
 - <csr-id-266395e18d7cf754f51355098c84fd1c969fc972/> Link up Io error as source for error chaining; add `Debug` to `client::fetch::Response`
 - <csr-id-c77bd7a01820110154f2c66cd954c1ccfff173c1/> '(null)' symref targets are turned into direct refs instead…
   …because that's what 'git' would do if it would care enough.
   On the client side this means one has to deal with detached heads
   and it's obvious that these are detached. It's not clear anymore
   why this is the case, and probably it's good to hide it as the
   current git behaviour is accidental and may change in future, and
   do so in a way we predict.
   
   There is the possibility that git would abort the entire
   fetch/upload-pack operation, but that's already handled correctly
   by our implementation as well as we understand ERR messages in packet
   lines.

### New Features

 - <csr-id-4b8abb0e8a5326662f36b88f04681bdd0eaa8079/> display the underlying permission denied error as source when failng handshake due to authentication.
 - <csr-id-5a50d9599f820237866928df6d24a196a9b86fe3/> `fetch::Arguments::use_include_tag()` allows to signal `include-tag`.
   One can also check for its availability using `fetch::Arguments::can_use_include_tag()`.
   
   Further there was a bugfix that assues V1 capabilities are correctly interpreted to support
   include-tag.
 - <csr-id-1c0a6c0edaca4063129c3b06638629b7af17a64c/> due to more specific errors one can differentiate if remote authentication failed repeatedly.
 - <csr-id-2363bcbd3207d1d16f2f95fbd20a7d7715b92240/> some errors support `is_spurious()` to tell if retrying is possible.
 - <csr-id-68c65a9f8b3927d454164a6f72167b47c0facee5/> expose `futures_lite` for convenience in downstream crates.
 - <csr-id-aed93d22d7a7faf4d60f1f603830c882175cbcb6/> Allow defaulting `fetch::handshake::Outcome`.
   This is useful in conjunction with `std::mem::take()`.
 - <csr-id-0bcb2fd4d5d8692a0e91c70bcbcbff9dff60442f/> `fetch::Arguments::is_empty()` to help decide if arguments should be sent at all.
   Furthermore, the `Debug` trait was added.
 - <csr-id-31a7089f2583832727e2175ada6fb5c30c3beebe/> make some private methods public to give callers more flexibility.
   This allows to implement the fetch-negotiation part oneself and break
   free from constraints of the delegate.
 - <csr-id-744ed03cf6648e258affab62ef39ffad75c65397/> add `client::fetch_pack()` allowing to fetch a pack after a handshake.
 - <csr-id-eaff67c14366f149ccca346acb46af12531a24e6/> `helper::main` to easily create credential helper implementations
 - <csr-id-5ba9e1d20a8d523571a153f9f4e3e1a5285335b5/> provide a function to gracefully shut donw a fetch transport
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-d6b4878d9832a0279e0dd19c1ca520a282289e69/> re-export maybe_async
   That way, API users can also provide dynamically sync or async APIs.
 - <csr-id-d2de51d65f9b4ab895e19cc1b307e42a9bb4bbd8/> add `fetch::refs()` method to invoke `ls-refs` for V2 and parse the result
 - <csr-id-35d17b4ea80be611607faa774d1ce0ee9c5000f2/> factor fetch handshake into `fetch::handshake()` function
 - <csr-id-0826cc92717faceff62df48a6e86fbb7ca6e90f5/> in-manifest and in-lib feature documentation

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### Changed

 - <csr-id-3a38d1bc4910aab98c9c0b4a309be4a449db92fc/> turn `prepare_ls_refs` in `fetch::refs()` into `FnOnce`.

### Documentation

 - <csr-id-39ed9eda62b7718d5109135e5ad406fb1fe2978c/> fix typos

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 672 commits contributed to the release.
 - 46 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 15 unique issues were worked on: [#198](https://github.com/GitoxideLabs/gitoxide/issues/198), [#200](https://github.com/GitoxideLabs/gitoxide/issues/200), [#205](https://github.com/GitoxideLabs/gitoxide/issues/205), [#222](https://github.com/GitoxideLabs/gitoxide/issues/222), [#254](https://github.com/GitoxideLabs/gitoxide/issues/254), [#279](https://github.com/GitoxideLabs/gitoxide/issues/279), [#301](https://github.com/GitoxideLabs/gitoxide/issues/301), [#329](https://github.com/GitoxideLabs/gitoxide/issues/329), [#386](https://github.com/GitoxideLabs/gitoxide/issues/386), [#450](https://github.com/GitoxideLabs/gitoxide/issues/450), [#470](https://github.com/GitoxideLabs/gitoxide/issues/470), [#63](https://github.com/GitoxideLabs/gitoxide/issues/63), [#639](https://github.com/GitoxideLabs/gitoxide/issues/639), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691), [#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 29 times to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#198](https://github.com/GitoxideLabs/gitoxide/issues/198)**
    - Deduplicate conventional message ids ([`e695eda`](https://github.com/GitoxideLabs/gitoxide/commit/e695eda8cd183f703d9a3e59b7c3c7fa496ea1d2))
    - Regenerate all changelogs to get links ([`0c81769`](https://github.com/GitoxideLabs/gitoxide/commit/0c817690bd444f52bed2936b2b451cafd87dde92))
    - Mention actual issues that where worked on ([`a517e39`](https://github.com/GitoxideLabs/gitoxide/commit/a517e39a81145b331f6c7a6cc2fc22e25daf42e2))
    - Support writing whole bodies in conventional messages… ([`c1f3c9d`](https://github.com/GitoxideLabs/gitoxide/commit/c1f3c9d2bd5a8e123ac9b376c257e3d5630876a0))
    - Support for paragraphs in conventional items ([`7f52823`](https://github.com/GitoxideLabs/gitoxide/commit/7f528239089788f4dd1f75a85bee1d0492285d60))
    - Respect release-wide ignore list to allow removing entire conventional headlines ([`145103d`](https://github.com/GitoxideLabs/gitoxide/commit/145103d4aa715386da9d4953f7f85fadc49fff9a))
    - Rebuild all changelogs to assure properly ordered headlines ([`4a9a05f`](https://github.com/GitoxideLabs/gitoxide/commit/4a9a05f95930bad5938d4ce9c517ebf0e0b990f1))
    - Sort all commits by time, descending… ([`f536bad`](https://github.com/GitoxideLabs/gitoxide/commit/f536bad20ffbac4dc353dfeb1a917bb88becbb78))
    - Greatly reduce changelog size now that the traversal fix is applied ([`a0bc98c`](https://github.com/GitoxideLabs/gitoxide/commit/a0bc98c06c349de2fd6e0d4593606e68b98def72))
    - Fixup remaining changelogs… ([`2f75db2`](https://github.com/GitoxideLabs/gitoxide/commit/2f75db294fcf20c325555822f65629611be52971))
    - Generate changelogs with details ([`e1861ca`](https://github.com/GitoxideLabs/gitoxide/commit/e1861caa435d312953a9fea7ceff6d2e07b03443))
    - Update all changelogs with details ([`58ab2ae`](https://github.com/GitoxideLabs/gitoxide/commit/58ab2aee23ba70a536e9487b44fb04c610374d1a))
    - Update changelogs ([`c857d61`](https://github.com/GitoxideLabs/gitoxide/commit/c857d61ce3ce342012a2c4ba10a8327822aa530e))
    - Fix docs ([`f1fa6dd`](https://github.com/GitoxideLabs/gitoxide/commit/f1fa6dd98c3bc0380a126adbb059a2dc1e8a0924))
    - Avoid adding newlines which make writing unstable ([`6b5c394`](https://github.com/GitoxideLabs/gitoxide/commit/6b5c394f49282a8d09c2a9ffece840e4683572db))
    - Fix section headline level ([`9d6f263`](https://github.com/GitoxideLabs/gitoxide/commit/9d6f263beef289d227dec1acc2d4240087cb9be6))
    - Write first version of changlogs thus far… ([`719b6bd`](https://github.com/GitoxideLabs/gitoxide/commit/719b6bdf543b8269ccafad9ad6b46e0c55efaa38))
    - Parse more user generated section content, adapt existing changelogs to work correctly ([`2f43a54`](https://github.com/GitoxideLabs/gitoxide/commit/2f43a54298e7ecfff2334627df149fe0882b5d1d))
 * **[#200](https://github.com/GitoxideLabs/gitoxide/issues/200)**
    - Feat: Lift io::Errors to response::Error::UploadPack(…)… ([`f293b63`](https://github.com/GitoxideLabs/gitoxide/commit/f293b633d16c0f7393d0ede64e12f14e47d0296b))
 * **[#205](https://github.com/GitoxideLabs/gitoxide/issues/205)**
    - '(null)' symref targets are turned into direct refs instead… ([`c77bd7a`](https://github.com/GitoxideLabs/gitoxide/commit/c77bd7a01820110154f2c66cd954c1ccfff173c1))
    - Fetch::Ref::Symbolic::target is now an option… ([`da68bfb`](https://github.com/GitoxideLabs/gitoxide/commit/da68bfb8104ecf58e73e3f99d87f81c90712a2ca))
 * **[#222](https://github.com/GitoxideLabs/gitoxide/issues/222)**
    - Update changelogs prior to release ([`9a493d0`](https://github.com/GitoxideLabs/gitoxide/commit/9a493d0651b0b6d71cf230dc510a658be7f8cb19))
    - Stabilize changelogs ([`920e832`](https://github.com/GitoxideLabs/gitoxide/commit/920e83219911df1c440d3fe42fd5ec3a295b0bb8))
    - Update changelogs prior to release ([`b3e2252`](https://github.com/GitoxideLabs/gitoxide/commit/b3e2252f7461a003d9a4612da60ba931dd8c0bef))
 * **[#254](https://github.com/GitoxideLabs/gitoxide/issues/254)**
    - Adjust changelogs prior to git-pack release ([`6776a3f`](https://github.com/GitoxideLabs/gitoxide/commit/6776a3ff9fa5a283da06c9ec5723d13023a0b267))
 * **[#279](https://github.com/GitoxideLabs/gitoxide/issues/279)**
    - Adjust to changes in git-hash ([`07aa1bc`](https://github.com/GitoxideLabs/gitoxide/commit/07aa1bca225c30b168a597f920bda392b2cb2713))
 * **[#301](https://github.com/GitoxideLabs/gitoxide/issues/301)**
    - Update changelogs prior to release ([`84cb256`](https://github.com/GitoxideLabs/gitoxide/commit/84cb25614a5fcddff297c1713eba4efbb6ff1596))
 * **[#329](https://github.com/GitoxideLabs/gitoxide/issues/329)**
    - In-manifest and in-lib feature documentation ([`0826cc9`](https://github.com/GitoxideLabs/gitoxide/commit/0826cc92717faceff62df48a6e86fbb7ca6e90f5))
    - Document all features related to serde1 ([`72b97f2`](https://github.com/GitoxideLabs/gitoxide/commit/72b97f2ae4dc7642b160f183c6d5df4502dc186f))
 * **[#386](https://github.com/GitoxideLabs/gitoxide/issues/386)**
    - Adapt to changes in git-sec ([`c5e2346`](https://github.com/GitoxideLabs/gitoxide/commit/c5e2346cee53019b1b321e45cf080b210e60bb7a))
    - Use `git-credentials` in `git-protocol` ([`32dc182`](https://github.com/GitoxideLabs/gitoxide/commit/32dc1829a5661f66396d109c8d0a8eaae6b1f532))
    - Reduce API surface ([`6df2881`](https://github.com/GitoxideLabs/gitoxide/commit/6df2881a9155a097e98b239167ad249b5d4cc086))
 * **[#450](https://github.com/GitoxideLabs/gitoxide/issues/450)**
    - Don't degenerate information about the unborn fetch ref's path. ([`42c977f`](https://github.com/GitoxideLabs/gitoxide/commit/42c977f66e93a0768acd24f268a91a2c8067d558))
    - `fetch::Ref::unpack()` returns `Option<oid>`. ([`cd867ad`](https://github.com/GitoxideLabs/gitoxide/commit/cd867ade55eaec138029c0cf17fa83e0679ef7fc))
    - Support for `unborn` ls-refs capability if server supports it. ([`02e37f0`](https://github.com/GitoxideLabs/gitoxide/commit/02e37f04d7dfeab7067c8273bcd76408cb1f4852))
    - Less noisy way of writing trait bounds ([`b593806`](https://github.com/GitoxideLabs/gitoxide/commit/b593806ca3571d680801130ad528f266d3eab83e))
    - Upgrade to `prodash` v21 ([`a0655dc`](https://github.com/GitoxideLabs/gitoxide/commit/a0655dc7bc5dff388bc69a648e7f16b44fd1abd9))
    - Improve docs slightly ([`306ebef`](https://github.com/GitoxideLabs/gitoxide/commit/306ebefdc0fea1e1926e6df11c617cc12087c3b5))
    - Support keepalive packets. ([`d7f62b4`](https://github.com/GitoxideLabs/gitoxide/commit/d7f62b441700c6d3526517c8c4f369cb9a72c102))
    - Properly implement `fetch::Arguments::is_empty()` ([`df36ede`](https://github.com/GitoxideLabs/gitoxide/commit/df36ede64501fab7d6161ac1961da9d4db9e5ecc))
    - Link up Io error as source for error chaining; add `Debug` to `client::fetch::Response` ([`266395e`](https://github.com/GitoxideLabs/gitoxide/commit/266395e18d7cf754f51355098c84fd1c969fc972))
    - Allow defaulting `fetch::handshake::Outcome`. ([`aed93d2`](https://github.com/GitoxideLabs/gitoxide/commit/aed93d22d7a7faf4d60f1f603830c882175cbcb6))
    - `fetch::Arguments::is_empty()` to help decide if arguments should be sent at all. ([`0bcb2fd`](https://github.com/GitoxideLabs/gitoxide/commit/0bcb2fd4d5d8692a0e91c70bcbcbff9dff60442f))
    - Make some private methods public to give callers more flexibility. ([`31a7089`](https://github.com/GitoxideLabs/gitoxide/commit/31a7089f2583832727e2175ada6fb5c30c3beebe))
    - Use `&dyn Any` instead of unspecified serialization format, as it's the right way. ([`779eefe`](https://github.com/GitoxideLabs/gitoxide/commit/779eefed97685300f4cd7b09957d3442c96e5b1f))
    - Adapt to changes in `git-transport` ([`e842843`](https://github.com/GitoxideLabs/gitoxide/commit/e8428438ff662a9222858d7b9703102eb1bd3cf1))
    - `client::fetch_pack()` isn't worth it after all. ([`f87b7eb`](https://github.com/GitoxideLabs/gitoxide/commit/f87b7eb4dc05b73f33ac554b57d50f3eee7f84c1))
    - Add `client::fetch_pack()` allowing to fetch a pack after a handshake. ([`744ed03`](https://github.com/GitoxideLabs/gitoxide/commit/744ed03cf6648e258affab62ef39ffad75c65397))
    - Refactor ([`c3e4b2a`](https://github.com/GitoxideLabs/gitoxide/commit/c3e4b2a9e7dd27d53426e30607ef7a158fe67cc3))
    - Update fetch feature list with `wait-for-done` ([`02245a6`](https://github.com/GitoxideLabs/gitoxide/commit/02245a619df3a88214b2fb23c3049eedd3ece332))
    - Turn `prepare_ls_refs` in `fetch::refs()` into `FnOnce`. ([`3a38d1b`](https://github.com/GitoxideLabs/gitoxide/commit/3a38d1bc4910aab98c9c0b4a309be4a449db92fc))
    - Improve docs a little ([`45394d5`](https://github.com/GitoxideLabs/gitoxide/commit/45394d580506722136906426078b333c7ace92eb))
    - Option to print server information about the connection ([`4720666`](https://github.com/GitoxideLabs/gitoxide/commit/4720666c8bfdaa3acc5c832b44755d4b4f86e16e))
    - Adjust to changes in `git-refspec` ([`ffa24a1`](https://github.com/GitoxideLabs/gitoxide/commit/ffa24a1365480523197b5247bded6a7a4772bdfc))
    - `fetch::Ref::unpack()` returns `&BStr` instead of `&BString` as ref name. ([`91f193f`](https://github.com/GitoxideLabs/gitoxide/commit/91f193f73ae37314319f7d055893b95431fd018e))
    - `fetch::Ref::unpack()` now returns the peeled object as well. ([`d2c772e`](https://github.com/GitoxideLabs/gitoxide/commit/d2c772e28619f7602ab01285b25e79a7040c9aab))
    - Rename `fetch::Ref::path` to `fetch::Ref::full_ref_name`. ([`bab5860`](https://github.com/GitoxideLabs/gitoxide/commit/bab586099a6e21e54932218f7a5c14f8bfc6cabd))
    - Upgrade `bstr` to `1.0.1` ([`99905ba`](https://github.com/GitoxideLabs/gitoxide/commit/99905bacace8aed42b16d43f0f04cae996cb971c))
    - Adjust to changes in git-credentials ([`32758e8`](https://github.com/GitoxideLabs/gitoxide/commit/32758e81bb96f7d83ff5f8ea0c226dea50fbeeec))
    - Adapt to changes in `git-credentials` ([`27c9da5`](https://github.com/GitoxideLabs/gitoxide/commit/27c9da50932ccba041ce137ff0bd93e06e2d4e7b))
    - Improved error when identity could not be obtained ([`08c1287`](https://github.com/GitoxideLabs/gitoxide/commit/08c12876d763a4ade3d4013ce1be66d9594e4ff1))
    - `helper::main` to easily create credential helper implementations ([`eaff67c`](https://github.com/GitoxideLabs/gitoxide/commit/eaff67c14366f149ccca346acb46af12531a24e6))
    - Adjust to changes in `git-credentials` ([`33f1347`](https://github.com/GitoxideLabs/gitoxide/commit/33f1347f8c91abf4c226e66dd18e2299dd965995))
    - Adjust to changes in `git-credentials` ([`fe1a06d`](https://github.com/GitoxideLabs/gitoxide/commit/fe1a06da4f935c565505e6f004ce927ecb466998))
    - Adjust to changes in `git-credentials` ([`40cc1dc`](https://github.com/GitoxideLabs/gitoxide/commit/40cc1dc09bf8639b53e0ef70e78dcd56518d92ea))
    - Refactor ([`077ab57`](https://github.com/GitoxideLabs/gitoxide/commit/077ab57425e9becba646c4651653aae256b8a1c9))
    - Provide a function to gracefully shut donw a fetch transport ([`5ba9e1d`](https://github.com/GitoxideLabs/gitoxide/commit/5ba9e1d20a8d523571a153f9f4e3e1a5285335b5))
    - Re-export maybe_async ([`d6b4878`](https://github.com/GitoxideLabs/gitoxide/commit/d6b4878d9832a0279e0dd19c1ca520a282289e69))
    - Fix docs ([`a6105cb`](https://github.com/GitoxideLabs/gitoxide/commit/a6105cb30dd5f1ff3126320984d3b05b4b939587))
    - Use `fetch::refs()` function in `fetch()` ([`37504e6`](https://github.com/GitoxideLabs/gitoxide/commit/37504e64076e6e32730d21bae6ee2044a5dc155f))
    - Add `fetch::refs()` method to invoke `ls-refs` for V2 and parse the result ([`d2de51d`](https://github.com/GitoxideLabs/gitoxide/commit/d2de51d65f9b4ab895e19cc1b307e42a9bb4bbd8))
    - Refactor ([`41a7391`](https://github.com/GitoxideLabs/gitoxide/commit/41a7391d86efff39a2fce126041c50429bda224c))
    - Use handshake() function in `fetch()` method ([`aaa1680`](https://github.com/GitoxideLabs/gitoxide/commit/aaa1680f9e19b64a0d380ced9559c7325b79dd04))
    - Replace `quick-error` with `this-error` ([`e91c301`](https://github.com/GitoxideLabs/gitoxide/commit/e91c301342d44cff35ebe12fba4ec10afb4a1922))
    - Factor fetch handshake into `fetch::handshake()` function ([`35d17b4`](https://github.com/GitoxideLabs/gitoxide/commit/35d17b4ea80be611607faa774d1ce0ee9c5000f2))
    - Sketch of simple delegate to collect listed refs ([`1c5f561`](https://github.com/GitoxideLabs/gitoxide/commit/1c5f5617940efe818a5e2aca5afe2cbd7f4ad940))
    - A first sketch on how connections could be working ([`e55b43e`](https://github.com/GitoxideLabs/gitoxide/commit/e55b43ef72bb3f23655c7e0884b8efcf2496f944))
    - Adjust to changes in `git-transport` ([`2141585`](https://github.com/GitoxideLabs/gitoxide/commit/2141585b1752a15a933a42e3f977142b4dea80fd))
    - Adapt to changes in `git-url` and use `BString` to represent URLs. ([`12589cc`](https://github.com/GitoxideLabs/gitoxide/commit/12589cc6f08e4d7aabae30bcdadaa0c2b4850229))
 * **[#470](https://github.com/GitoxideLabs/gitoxide/issues/470)**
    - Update changelogs prior to release ([`caa7a1b`](https://github.com/GitoxideLabs/gitoxide/commit/caa7a1bdef74d7d3166a7e38127a59f5ab3cfbdd))
 * **[#63](https://github.com/GitoxideLabs/gitoxide/issues/63)**
    - Git-protocol uses `oid` type ([`3930a6f`](https://github.com/GitoxideLabs/gitoxide/commit/3930a6ff508f5bb2249fb2c2f21e00b74fecda22))
    - Refactor; better errors for invalid hash sizes ([`be84b36`](https://github.com/GitoxideLabs/gitoxide/commit/be84b36129694a2e89d1b81d932f2eba23aedf54))
    - Make ObjectId/oid happen! ([`ca78d15`](https://github.com/GitoxideLabs/gitoxide/commit/ca78d15373ec988d909be8f240baefe75555e077))
    - Remove all public exports of git-hash types in git-object ([`accf89d`](https://github.com/GitoxideLabs/gitoxide/commit/accf89d25560e5ded6f44a1c4a898ee65d14f8f6))
    - Remove re-export of git_object::borrowed::Id ([`a3f2816`](https://github.com/GitoxideLabs/gitoxide/commit/a3f28169c1268c1129852f279631d5a7f7540cdf))
    - Make git-hash Error usage explicit (it's for decoding only) ([`4805cfc`](https://github.com/GitoxideLabs/gitoxide/commit/4805cfc8d837bb111424b5e32f46d0fb9b12365a))
 * **[#639](https://github.com/GitoxideLabs/gitoxide/issues/639)**
    - Don't post `packfile-uris` capability unless we are able to make use of it. ([`24000fa`](https://github.com/GitoxideLabs/gitoxide/commit/24000fa7ad4607c23f6a4fb4d84d2c85d96ee01f))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **[#XXX](https://github.com/GitoxideLabs/gitoxide/issues/XXX)**
    - Prepare changelogs prior to release ([`8c0bca3`](https://github.com/GitoxideLabs/gitoxide/commit/8c0bca37ff9fbaadbe55561fb2b0d649980c95b1))
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
    - Rename `git-protocol` to `gix-protocol` ([`9a35a90`](https://github.com/GitoxideLabs/gitoxide/commit/9a35a90457fe80384cee8efdd3f688c199fbaf42))
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
    - Release git-features v0.26.4 ([`109f434`](https://github.com/GitoxideLabs/gitoxide/commit/109f434e66559a791d541f86876ded8df10766f1))
    - Release git-features v0.26.3 ([`1ecfb7f`](https://github.com/GitoxideLabs/gitoxide/commit/1ecfb7f8bfb24432690d8f31367488f2e59a642a))
    - Release git-date v0.4.2, git-hash v0.10.2, git-features v0.26.2, git-actor v0.17.1, git-glob v0.5.3, git-path v0.7.1, git-quote v0.4.1, git-attributes v0.8.2, git-config-value v0.10.1, git-tempfile v3.0.2, git-lock v3.0.2, git-validate v0.7.2, git-object v0.26.1, git-ref v0.24.0, git-sec v0.6.2, git-config v0.16.0, git-command v0.2.3, git-prompt v0.3.2, git-url v0.13.2, git-credentials v0.9.1, git-diff v0.26.1, git-discover v0.13.0, git-hashtable v0.1.1, git-bitmap v0.2.1, git-traverse v0.22.1, git-index v0.12.3, git-mailmap v0.9.2, git-chunk v0.4.1, git-pack v0.30.2, git-odb v0.40.2, git-packetline v0.14.2, git-transport v0.25.4, git-protocol v0.26.3, git-revision v0.10.2, git-refspec v0.7.2, git-worktree v0.12.2, git-repository v0.34.0, safety bump 3 crates ([`c196d20`](https://github.com/GitoxideLabs/gitoxide/commit/c196d206d57a310b1ce974a1cf0e7e6d6db5c4d6))
    - Prepare changelogs prior to release ([`7c846d2`](https://github.com/GitoxideLabs/gitoxide/commit/7c846d2102dc767366771925212712ef8cc9bf07))
    - Merge branch 'Lioness100/main' ([`1e544e8`](https://github.com/GitoxideLabs/gitoxide/commit/1e544e82455bf9ecb5e3c2146280eaf7ecd81f16))
    - Fix typos ([`39ed9ed`](https://github.com/GitoxideLabs/gitoxide/commit/39ed9eda62b7718d5109135e5ad406fb1fe2978c))
    - Thanks clippy ([`bac57dd`](https://github.com/GitoxideLabs/gitoxide/commit/bac57dd05ea2d5a4ee45ef9350fa3f2e19474bc0))
    - Break cyclical dev dependencies ([`1fea18f`](https://github.com/GitoxideLabs/gitoxide/commit/1fea18f5f8b4189a23dc4fa3f041a672f6fbcfb3))
    - Release git-date v0.4.1, git-features v0.26.1, git-glob v0.5.2, git-attributes v0.8.1, git-tempfile v3.0.1, git-ref v0.23.1, git-sec v0.6.1, git-config v0.15.1, git-prompt v0.3.1, git-url v0.13.1, git-discover v0.12.1, git-index v0.12.2, git-mailmap v0.9.1, git-pack v0.30.1, git-odb v0.40.1, git-transport v0.25.3, git-protocol v0.26.2, git-revision v0.10.1, git-refspec v0.7.1, git-worktree v0.12.1, git-repository v0.33.0 ([`5b5b380`](https://github.com/GitoxideLabs/gitoxide/commit/5b5b3809faa71c658db38b40dfc410224d08a367))
    - Prepare changelogs prior to release ([`93bef97`](https://github.com/GitoxideLabs/gitoxide/commit/93bef97b3c0c75d4bf7119fdd787516e1efc77bf))
    - Merge branch 'patch-1' ([`b93f0c4`](https://github.com/GitoxideLabs/gitoxide/commit/b93f0c49fc677b6c19aea332cbfc1445ce475375))
    - Thanks clippy ([`b34c9fe`](https://github.com/GitoxideLabs/gitoxide/commit/b34c9fe58223862712eacc1cb7353e497a4b9778))
    - Release git-ref v0.23.0, git-config v0.15.0, git-command v0.2.2, git-diff v0.26.0, git-discover v0.12.0, git-mailmap v0.9.0, git-pack v0.30.0, git-odb v0.40.0, git-transport v0.25.2, git-protocol v0.26.1, git-revision v0.10.0, git-refspec v0.7.0, git-worktree v0.12.0, git-repository v0.32.0 ([`ffb5b6a`](https://github.com/GitoxideLabs/gitoxide/commit/ffb5b6a21cb415315db6fd5294940c7c6deb4538))
    - Prepare changelogs prior to release ([`4381a03`](https://github.com/GitoxideLabs/gitoxide/commit/4381a03a34c305f31713cce234c2afbf8ac60f01))
    - Release git-transport v0.25.1 ([`e0b12fe`](https://github.com/GitoxideLabs/gitoxide/commit/e0b12fe64b50a1b614111924b55ce02f1c39ac00))
    - Release git-features v0.26.0, git-actor v0.16.0, git-attributes v0.8.0, git-object v0.25.0, git-ref v0.22.0, git-config v0.14.0, git-command v0.2.1, git-url v0.13.0, git-credentials v0.9.0, git-diff v0.25.0, git-discover v0.11.0, git-traverse v0.21.0, git-index v0.11.0, git-mailmap v0.8.0, git-pack v0.29.0, git-odb v0.39.0, git-transport v0.25.0, git-protocol v0.26.0, git-revision v0.9.0, git-refspec v0.6.0, git-worktree v0.11.0, git-repository v0.31.0, safety bump 24 crates ([`5ac9fbe`](https://github.com/GitoxideLabs/gitoxide/commit/5ac9fbe265a5b61c533a2a6b3abfed2bdf7f89ad))
    - Prepare changelogs prior to release ([`30d8ca1`](https://github.com/GitoxideLabs/gitoxide/commit/30d8ca19284049dcfbb0de2698cafae1d1a16b0c))
    - Display the underlying permission denied error as source when failng handshake due to authentication. ([`4b8abb0`](https://github.com/GitoxideLabs/gitoxide/commit/4b8abb0e8a5326662f36b88f04681bdd0eaa8079))
    - Release git-url v0.12.1, git-transport v0.24.1, git-protocol v0.25.1, git-repository v0.30.1, git-commitgraph v0.12.0, gitoxide-core v0.22.0, gitoxide v0.20.0 ([`08ec3a9`](https://github.com/GitoxideLabs/gitoxide/commit/08ec3a93d77a1018439a5c41c23729ffed27c5a5))
    - Prepare changelogs prior to release ([`68ce15d`](https://github.com/GitoxideLabs/gitoxide/commit/68ce15d07b50cfacdac0d1e42fe7f5e6330ba523))
    - Merge branch 'adjustments-for-cargo' ([`5afa7f5`](https://github.com/GitoxideLabs/gitoxide/commit/5afa7f51342deaf0938e7fb2ebe6a578e83ab645))
    - Release git-date v0.3.1, git-features v0.25.0, git-actor v0.15.0, git-glob v0.5.1, git-path v0.7.0, git-attributes v0.7.0, git-config-value v0.10.0, git-lock v3.0.1, git-validate v0.7.1, git-object v0.24.0, git-ref v0.21.0, git-sec v0.6.0, git-config v0.13.0, git-prompt v0.3.0, git-url v0.12.0, git-credentials v0.8.0, git-diff v0.24.0, git-discover v0.10.0, git-traverse v0.20.0, git-index v0.10.0, git-mailmap v0.7.0, git-pack v0.28.0, git-odb v0.38.0, git-packetline v0.14.1, git-transport v0.24.0, git-protocol v0.25.0, git-revision v0.8.0, git-refspec v0.5.0, git-worktree v0.10.0, git-repository v0.30.0, safety bump 26 crates ([`e6b9906`](https://github.com/GitoxideLabs/gitoxide/commit/e6b9906c486b11057936da16ed6e0ec450a0fb83))
    - Prepare chnagelogs prior to git-repository release ([`7114bbb`](https://github.com/GitoxideLabs/gitoxide/commit/7114bbb6732aa8571d4ab74f28ed3e26e9fbe4d0))
    - Don't enforce V2 as protocol, but smoothly downgrade like git does. ([`5f2276b`](https://github.com/GitoxideLabs/gitoxide/commit/5f2276b63129163096be3cb229864fc589348da8))
    - Merge branch 'main' into read-split-index ([`c57bdde`](https://github.com/GitoxideLabs/gitoxide/commit/c57bdde6de37eca9672ea715962bbd02aa3eb055))
    - Merge branch 'adjustments-for-cargo' ([`083909b`](https://github.com/GitoxideLabs/gitoxide/commit/083909bc7eb902eeee2002034fdb6ed88280dc5c))
    - `fetch::Arguments::use_include_tag()` allows to signal `include-tag`. ([`5a50d95`](https://github.com/GitoxideLabs/gitoxide/commit/5a50d9599f820237866928df6d24a196a9b86fe3))
    - Don't pass 'include-tag' as argument by default. ([`a872782`](https://github.com/GitoxideLabs/gitoxide/commit/a8727824a4c38b27d6ad7c73ccaf7a45839c0aa9))
    - Merge branch 'adjustments-for-cargo' ([`94750e1`](https://github.com/GitoxideLabs/gitoxide/commit/94750e15831969059551af35d31c21009462084d))
    - Due to more specific errors one can differentiate if remote authentication failed repeatedly. ([`1c0a6c0`](https://github.com/GitoxideLabs/gitoxide/commit/1c0a6c0edaca4063129c3b06638629b7af17a64c))
    - Merge branch 'adjustments-for-cargo' ([`70ccbb2`](https://github.com/GitoxideLabs/gitoxide/commit/70ccbb21b1113bdeb20b52d274141a9fdb75f579))
    - Some errors support `is_spurious()` to tell if retrying is possible. ([`2363bcb`](https://github.com/GitoxideLabs/gitoxide/commit/2363bcbd3207d1d16f2f95fbd20a7d7715b92240))
    - Adapt to changes in `git-transport` ([`527c62e`](https://github.com/GitoxideLabs/gitoxide/commit/527c62ef034a961a7e2b1dd1868cf8f81cc2eedc))
    - Merge branch 'fix-638' ([`eb4c5f0`](https://github.com/GitoxideLabs/gitoxide/commit/eb4c5f051ae2a4eb7178289cfc1437417f265608))
    - Merge branch 'fixture-async' ([`eca6705`](https://github.com/GitoxideLabs/gitoxide/commit/eca670585db212985d653cb2c6ec3636ec560905))
    - Async version of ref-line parsing now reads line by line. ([`dadd896`](https://github.com/GitoxideLabs/gitoxide/commit/dadd8964ec551702908055476df10624b266a79f))
    - Add `Fixture` implementation for `async`. ([`b583c2a`](https://github.com/GitoxideLabs/gitoxide/commit/b583c2a631a68429aa84f33212586c7c2d1165e5))
    - Release git-hash v0.10.1, git-hashtable v0.1.0 ([`7717170`](https://github.com/GitoxideLabs/gitoxide/commit/771717095d9a67b0625021eb0928828ab686e772))
    - Merge branch 'remove-lines-parsing' ([`9d8e32d`](https://github.com/GitoxideLabs/gitoxide/commit/9d8e32d3c276fec34e3fce0feb29de0d24a8d1d2))
    - Parse refs from bytes, not from String. ([`806b8c2`](https://github.com/GitoxideLabs/gitoxide/commit/806b8c2ef392137f3a6ebd0f28da2a3a07a9f3eb))
    - Adapt to changes in `git-transport` ([`49461b1`](https://github.com/GitoxideLabs/gitoxide/commit/49461b1ca92e301734c090220936941fca57db0e))
    - Merge branch 'main' into http-config ([`6b9632e`](https://github.com/GitoxideLabs/gitoxide/commit/6b9632e16c416841ffff1b767ee7a6c89b421220))
    - Release git-features v0.24.1, git-actor v0.14.1, git-index v0.9.1 ([`7893502`](https://github.com/GitoxideLabs/gitoxide/commit/789350208efc9d5fc6f9bc4f113f77f9cb445156))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Make fmt ([`0abab7d`](https://github.com/GitoxideLabs/gitoxide/commit/0abab7da2ec1b8560e6c1eb009f534c9fc7814fe))
    - Release git-hash v0.10.0, git-features v0.24.0, git-date v0.3.0, git-actor v0.14.0, git-glob v0.5.0, git-path v0.6.0, git-quote v0.4.0, git-attributes v0.6.0, git-config-value v0.9.0, git-tempfile v3.0.0, git-lock v3.0.0, git-validate v0.7.0, git-object v0.23.0, git-ref v0.20.0, git-sec v0.5.0, git-config v0.12.0, git-command v0.2.0, git-prompt v0.2.0, git-url v0.11.0, git-credentials v0.7.0, git-diff v0.23.0, git-discover v0.9.0, git-bitmap v0.2.0, git-traverse v0.19.0, git-index v0.9.0, git-mailmap v0.6.0, git-chunk v0.4.0, git-pack v0.27.0, git-odb v0.37.0, git-packetline v0.14.0, git-transport v0.23.0, git-protocol v0.24.0, git-revision v0.7.0, git-refspec v0.4.0, git-worktree v0.9.0, git-repository v0.29.0, git-commitgraph v0.11.0, gitoxide-core v0.21.0, gitoxide v0.19.0, safety bump 28 crates ([`b2c301e`](https://github.com/GitoxideLabs/gitoxide/commit/b2c301ef131ffe1871314e19f387cf10a8d2ac16))
    - Prepare changelogs prior to release ([`e4648f8`](https://github.com/GitoxideLabs/gitoxide/commit/e4648f827c97e9d13636d1bbdc83dd63436e6e5c))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Release git-glob v0.4.2, git-config-value v0.8.2, git-lock v2.2.0, git-ref v0.19.0, git-config v0.11.0, git-discover v0.8.0, git-index v0.8.0, git-transport v0.22.0, git-protocol v0.23.0, git-worktree v0.8.0, git-repository v0.28.0, gitoxide-core v0.20.0, gitoxide v0.18.0, safety bump 9 crates ([`0c253b1`](https://github.com/GitoxideLabs/gitoxide/commit/0c253b15143dcedfe4c66d64ab1ea6e097030651))
    - Prepare changelogs prior to release ([`fe5721f`](https://github.com/GitoxideLabs/gitoxide/commit/fe5721f888c64c79fe9a734a9e33b94a282f8d97))
    - Merge branch 'http-config' ([`665b53e`](https://github.com/GitoxideLabs/gitoxide/commit/665b53e1c2e1de65fafa28b669f58977868bbc81))
    - Fix docs ([`b5c316e`](https://github.com/GitoxideLabs/gitoxide/commit/b5c316e285369a84e57ec6f7425b92fec2978a49))
    - Move `fetch::refs(…, protocol, …)` to the crate root as `ls_refs(…)`. ([`09070a7`](https://github.com/GitoxideLabs/gitoxide/commit/09070a7c17f39383730c3a2b809eec677f79f386))
    - Move `fetch::Command` into the crate root. ([`35f7b4d`](https://github.com/GitoxideLabs/gitoxide/commit/35f7b4df164c130fb50fcffcf5de99816c2ca872))
    - Move `fetch::agent|indicate_end_of_interaction` to crate root ([`a03f8f6`](https://github.com/GitoxideLabs/gitoxide/commit/a03f8f6cce34618883e8448dd1c31b41c54d9448))
    - `handshake(…)` is now generalized to support `git-receive-pack` as well. ([`a3bcf82`](https://github.com/GitoxideLabs/gitoxide/commit/a3bcf82ae50defa4439862943008647d03d09792))
    - Migrate independent parts of ref parsing to generalized handshake. ([`611a139`](https://github.com/GitoxideLabs/gitoxide/commit/611a1394b1a7470b9247474ea0c43ef59560f6fe))
    - Prepare for handshake generalization by copying it into position; parameterize service ([`7691353`](https://github.com/GitoxideLabs/gitoxide/commit/7691353111f3c61cf9c3ddc26c518016c0b45c4c))
    - Merge branch 'main' into http-config ([`f4ff821`](https://github.com/GitoxideLabs/gitoxide/commit/f4ff821fd4233dd1dc1a449af4d4600becf3b4ac))
    - Merge branch 'async-fetch' ([`0c9c48b`](https://github.com/GitoxideLabs/gitoxide/commit/0c9c48b3b91a1396eb1796f288a2cb10380d1f14))
    - Expose `futures_lite` for convenience in downstream crates. ([`68c65a9`](https://github.com/GitoxideLabs/gitoxide/commit/68c65a9f8b3927d454164a6f72167b47c0facee5))
    - Set exact patch level for `git-transport` ([`b46e3bb`](https://github.com/GitoxideLabs/gitoxide/commit/b46e3bb062cdfb9ed4c4ea2b4d764e36f255e2a5))
    - Adjust for changes in `git-transport` ([`f88569b`](https://github.com/GitoxideLabs/gitoxide/commit/f88569b686c65bde3c330ee591e032bf3b1abc61))
    - Adapt to changes in `git-transport` ([`226f33a`](https://github.com/GitoxideLabs/gitoxide/commit/226f33ac38cf5197c41f0787f1bee91a584914f0))
    - Make trait parameter more open, no need for `'static` ([`68dc86f`](https://github.com/GitoxideLabs/gitoxide/commit/68dc86fdf1a386d4e535c6bc8cb594237395a861))
    - `fetch::agent()` returns agent string only; `fetch()` takes agent name as parameter. ([`d05b0b8`](https://github.com/GitoxideLabs/gitoxide/commit/d05b0b800a553e1e380801fb141e9aa054a6cbd0))
    - Don't auto-insert user agent, instead prepare to insert it later. ([`aab278f`](https://github.com/GitoxideLabs/gitoxide/commit/aab278f2a3e07edb6d8109e7ffba003b5d5d7857))
    - `fetch::agent()` now returns the agent name, `fetch::agent_tuple()` returns specific key-value pair. ([`f957d9a`](https://github.com/GitoxideLabs/gitoxide/commit/f957d9a1833af079b01ba6bd6941eb4af6c9e436))
    - Release git-features v0.23.1, git-glob v0.4.1, git-config-value v0.8.1, git-tempfile v2.0.6, git-object v0.22.1, git-ref v0.18.0, git-sec v0.4.2, git-config v0.10.0, git-prompt v0.1.1, git-url v0.10.1, git-credentials v0.6.1, git-diff v0.21.0, git-discover v0.7.0, git-index v0.7.0, git-pack v0.25.0, git-odb v0.35.0, git-transport v0.21.1, git-protocol v0.22.0, git-refspec v0.3.1, git-worktree v0.7.0, git-repository v0.26.0, git-commitgraph v0.10.0, gitoxide-core v0.19.0, gitoxide v0.17.0, safety bump 9 crates ([`d071583`](https://github.com/GitoxideLabs/gitoxide/commit/d071583c5576fdf5f7717765ffed5681792aa81f))
    - Prepare changelogs prior to release ([`423af90`](https://github.com/GitoxideLabs/gitoxide/commit/423af90c8202d62dc1ea4a76a0df6421d1f0aa06))
    - Merge branch 'main' into write-sparse-index (upgrade to Rust 1.65) ([`5406630`](https://github.com/GitoxideLabs/gitoxide/commit/5406630466145990b5adbdadb59151036993060d))
    - Thanks clippy ([`04cfa63`](https://github.com/GitoxideLabs/gitoxide/commit/04cfa635a65ae34ad6d22391f2febd2ca7eabca9))
    - Merge branch 'main' into write-sparse-index ([`c4e6849`](https://github.com/GitoxideLabs/gitoxide/commit/c4e68496c368611ebe17c6693d06c8147c28c717))
    - Merge branch 'gix-clone' ([`def53b3`](https://github.com/GitoxideLabs/gitoxide/commit/def53b36c3dec26fa78939ab0584fe4ff930909c))
    - Thanks clippy ([`bc9c780`](https://github.com/GitoxideLabs/gitoxide/commit/bc9c780793140efd5fced9fe57cfe151f6a67eba))
    - Release git-hash v0.9.11, git-features v0.23.0, git-actor v0.13.0, git-attributes v0.5.0, git-object v0.22.0, git-ref v0.17.0, git-sec v0.4.1, git-config v0.9.0, git-url v0.10.0, git-credentials v0.6.0, git-diff v0.20.0, git-discover v0.6.0, git-traverse v0.18.0, git-index v0.6.0, git-mailmap v0.5.0, git-pack v0.24.0, git-odb v0.34.0, git-packetline v0.13.1, git-transport v0.21.0, git-protocol v0.21.0, git-revision v0.6.0, git-refspec v0.3.0, git-worktree v0.6.0, git-repository v0.25.0, safety bump 24 crates ([`104d922`](https://github.com/GitoxideLabs/gitoxide/commit/104d922add61ab21c534c24ce8ed37cddf3e275a))
    - Prepare changelogs for release ([`d232567`](https://github.com/GitoxideLabs/gitoxide/commit/d23256701a95284857dc8d1cb37c7c94cada973c))
    - Merge branch 'main' into new-http-impl ([`702a161`](https://github.com/GitoxideLabs/gitoxide/commit/702a161ef11fc959611bf44b70e9ffe04561c7ad))
    - Merge branch 'fetch-pack' ([`f47c891`](https://github.com/GitoxideLabs/gitoxide/commit/f47c89129732bcb06fe76a4696fe38ab1151fb0c))
    - Merge branch 'fetch-pack' ([`3c49400`](https://github.com/GitoxideLabs/gitoxide/commit/3c49400809c7c2120f4ce704c19a0421545b5acd))
    - Merge branch 'fix-git-features' ([`82fd251`](https://github.com/GitoxideLabs/gitoxide/commit/82fd251ac80d07bc9da8a4d36e517aa35580d188))
    - Merge branch 'diff' ([`25a7726`](https://github.com/GitoxideLabs/gitoxide/commit/25a7726377fbe400ea3c4927d04e9dec99802b7b))
    - Release git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0 ([`f5c36d8`](https://github.com/GitoxideLabs/gitoxide/commit/f5c36d85755d1f0f503b77d9a565fad6aecf6728))
    - Release git-hash v0.9.10, git-features v0.22.5, git-date v0.2.0, git-actor v0.12.0, git-glob v0.4.0, git-path v0.5.0, git-quote v0.3.0, git-attributes v0.4.0, git-config-value v0.8.0, git-tempfile v2.0.5, git-validate v0.6.0, git-object v0.21.0, git-ref v0.16.0, git-sec v0.4.0, git-config v0.8.0, git-discover v0.5.0, git-traverse v0.17.0, git-index v0.5.0, git-worktree v0.5.0, git-testtools v0.9.0, git-command v0.1.0, git-prompt v0.1.0, git-url v0.9.0, git-credentials v0.5.0, git-diff v0.19.0, git-mailmap v0.4.0, git-chunk v0.3.2, git-pack v0.23.0, git-odb v0.33.0, git-packetline v0.13.0, git-transport v0.20.0, git-protocol v0.20.0, git-revision v0.5.0, git-refspec v0.2.0, git-repository v0.24.0, git-commitgraph v0.9.0, gitoxide-core v0.18.0, gitoxide v0.16.0, safety bump 28 crates ([`29a043b`](https://github.com/GitoxideLabs/gitoxide/commit/29a043be6808a3e9199a9b26bd076fe843afe4f4))
    - Merge branch 'filter-refs' ([`3773b92`](https://github.com/GitoxideLabs/gitoxide/commit/3773b92b8372c9a40a74d281149ca65b057a7da9))
    - Merge branch 'filter-refs' ([`fd14489`](https://github.com/GitoxideLabs/gitoxide/commit/fd14489f729172d615d0fa1e8dbd605e9eacf69d))
    - Release git-features v0.22.6 ([`c9eda72`](https://github.com/GitoxideLabs/gitoxide/commit/c9eda729d8f8bc266c7516c613d38acfb83a4743))
    - Make fmt ([`535e967`](https://github.com/GitoxideLabs/gitoxide/commit/535e967666c6da657ff1b7eff7c64ab27cafb182))
    - Merge branch 'filter-refs-by-spec' ([`5c05198`](https://github.com/GitoxideLabs/gitoxide/commit/5c051986bd89590a9287d85d84c713d83dfab83a))
    - Merge branch 'main' into filter-refs-by-spec ([`9aa1d3d`](https://github.com/GitoxideLabs/gitoxide/commit/9aa1d3dc46d4b1c76af257f573aff3aeef2d3fa8))
    - Release git-features v0.22.4, git-url v0.8.0, safety bump 4 crates ([`1d4600a`](https://github.com/GitoxideLabs/gitoxide/commit/1d4600ae51475c2e225f96c16c41e2c4a2b3f2aa))
    - Merge branch 'main' into filter-refs-by-spec ([`1f6e5ab`](https://github.com/GitoxideLabs/gitoxide/commit/1f6e5ab15f5fd8d23719b13e6aea59cd231ac0fe))
    - Merge branch 'fix-522' ([`5869e9f`](https://github.com/GitoxideLabs/gitoxide/commit/5869e9ff2508d5a93c07635277af8764fcb57713))
    - Release git-hash v0.9.9 ([`da0716f`](https://github.com/GitoxideLabs/gitoxide/commit/da0716f8c27b4f29cfff0e5ce7fcb3d7240f4aeb))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into filter-refs-by-spec ([`cef0b51`](https://github.com/GitoxideLabs/gitoxide/commit/cef0b51ade2a3301fa09ede7a425aa1fe3527e78))
    - Release git-object v0.20.3, git-ref v0.15.4, git-config v0.7.1, git-diff v0.18.0, git-traverse v0.16.3, git-pack v0.22.0, git-odb v0.32.0, git-url v0.7.3, git-transport v0.19.3, git-protocol v0.19.1, git-refspec v0.1.1, git-repository v0.23.0, safety bump 6 crates ([`85a3bed`](https://github.com/GitoxideLabs/gitoxide/commit/85a3bedd68d2e5f36592a2f691c977dc55298279))
    - Release git-features v0.22.3, git-revision v0.4.4 ([`c2660e2`](https://github.com/GitoxideLabs/gitoxide/commit/c2660e2503323531ba02519eaa51124ee22fec51))
    - Merge branch 'main' into filter-refs-by-spec ([`cfa1440`](https://github.com/GitoxideLabs/gitoxide/commit/cfa144031dbcac2707ab0cec012bc35e78f9c475))
    - Release git-date v0.0.5, git-hash v0.9.8, git-features v0.22.2, git-actor v0.11.3, git-glob v0.3.2, git-quote v0.2.1, git-attributes v0.3.2, git-tempfile v2.0.4, git-lock v2.1.1, git-validate v0.5.5, git-object v0.20.2, git-ref v0.15.2, git-sec v0.3.1, git-config v0.7.0, git-credentials v0.4.0, git-diff v0.17.2, git-discover v0.4.1, git-bitmap v0.1.2, git-index v0.4.2, git-mailmap v0.3.2, git-chunk v0.3.1, git-traverse v0.16.2, git-pack v0.21.2, git-odb v0.31.2, git-packetline v0.12.7, git-url v0.7.2, git-transport v0.19.2, git-protocol v0.19.0, git-revision v0.4.2, git-refspec v0.1.0, git-worktree v0.4.2, git-repository v0.22.0, safety bump 4 crates ([`4974eca`](https://github.com/GitoxideLabs/gitoxide/commit/4974eca96d525d1ee4f8cad79bb713af7a18bf9d))
    - Merge branch 'remote-ls-refs' ([`39d585d`](https://github.com/GitoxideLabs/gitoxide/commit/39d585d9f9ac6f3ecf51359c8e37f0a50e21ed45))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Use docsrs feature in code to show what is feature-gated automatically on docs.rs ([`b1c40b0`](https://github.com/GitoxideLabs/gitoxide/commit/b1c40b0364ef092cd52d03b34f491b254816b18d))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Pass --cfg docsrs when compiling for https://docs.rs ([`5176771`](https://github.com/GitoxideLabs/gitoxide/commit/517677147f1c17304c62cf97a1dd09f232ebf5db))
    - Merge branch 'main' into remote-ls-refs ([`bd5f3e8`](https://github.com/GitoxideLabs/gitoxide/commit/bd5f3e8db7e0bb4abfb7b0f79f585ab82c3a14ab))
    - Release git-date v0.0.3, git-actor v0.11.1, git-attributes v0.3.1, git-tempfile v2.0.3, git-object v0.20.1, git-ref v0.15.1, git-config v0.6.1, git-diff v0.17.1, git-discover v0.4.0, git-bitmap v0.1.1, git-index v0.4.1, git-mailmap v0.3.1, git-traverse v0.16.1, git-pack v0.21.1, git-odb v0.31.1, git-packetline v0.12.6, git-url v0.7.1, git-transport v0.19.1, git-protocol v0.18.1, git-revision v0.4.0, git-worktree v0.4.1, git-repository v0.21.0, safety bump 5 crates ([`c96473d`](https://github.com/GitoxideLabs/gitoxide/commit/c96473dce21c3464aacbc0a62d520c1a33172611))
    - Prepare changelogs prior to reelase ([`c06ae1c`](https://github.com/GitoxideLabs/gitoxide/commit/c06ae1c606b6af9c2a12021103d99c2810750d60))
    - Thanks clippy ([`f48ac49`](https://github.com/GitoxideLabs/gitoxide/commit/f48ac49e167c93b4d7937b4c538b0d9389e7bbe8))
    - Release git-hash v0.9.7, git-features v0.22.1 ([`232784a`](https://github.com/GitoxideLabs/gitoxide/commit/232784a59ded3e8016e4257c7e146ad385cdd64a))
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
    - Merge branch 'main' into pathspec ([`89ea12b`](https://github.com/GitoxideLabs/gitoxide/commit/89ea12b558bcc056b892193ee8fb44b8664b5da4))
    - Merge branch 'main' into cont_include_if ([`41ea8ba`](https://github.com/GitoxideLabs/gitoxide/commit/41ea8ba78e74f5c988148367386a1f4f304cb951))
    - Release git-path v0.3.0, safety bump 14 crates ([`400c9be`](https://github.com/GitoxideLabs/gitoxide/commit/400c9bec49e4ec5351dc9357b246e7677a63ea35))
    - Release git-date v0.0.1, git-hash v0.9.5, git-features v0.21.1, git-actor v0.10.1, git-path v0.2.0, git-attributes v0.2.0, git-ref v0.14.0, git-sec v0.2.0, git-config v0.5.0, git-credentials v0.2.0, git-discover v0.2.0, git-pack v0.20.0, git-odb v0.30.0, git-url v0.6.0, git-transport v0.18.0, git-protocol v0.17.0, git-revision v0.2.1, git-worktree v0.3.0, git-repository v0.19.0, safety bump 13 crates ([`a417177`](https://github.com/GitoxideLabs/gitoxide/commit/a41717712578f590f04a33d27adaa63171f25267))
    - Update changelogs prior to release ([`bb424f5`](https://github.com/GitoxideLabs/gitoxide/commit/bb424f51068b8a8e762696890a55ab48900ab980))
    - Merge branch 'main' into SidneyDouw-pathspec ([`a22b1d8`](https://github.com/GitoxideLabs/gitoxide/commit/a22b1d88a21311d44509018729c3ef1936cf052a))
    - Merge branch 'main' into git_includeif ([`598c853`](https://github.com/GitoxideLabs/gitoxide/commit/598c853087fcf8f77299aa5b9803bcec705c0cd0))
    - Release git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0 ([`349c590`](https://github.com/GitoxideLabs/gitoxide/commit/349c5904b0dac350838a896759d51576b66880a7))
    - Release git-hash v0.9.4, git-features v0.21.0, git-actor v0.10.0, git-glob v0.3.0, git-path v0.1.1, git-attributes v0.1.0, git-sec v0.1.0, git-config v0.3.0, git-credentials v0.1.0, git-validate v0.5.4, git-object v0.19.0, git-diff v0.16.0, git-lock v2.1.0, git-ref v0.13.0, git-discover v0.1.0, git-index v0.3.0, git-mailmap v0.2.0, git-traverse v0.15.0, git-pack v0.19.0, git-odb v0.29.0, git-packetline v0.12.5, git-url v0.5.0, git-transport v0.17.0, git-protocol v0.16.0, git-revision v0.2.0, git-worktree v0.2.0, git-repository v0.17.0, safety bump 20 crates ([`654cf39`](https://github.com/GitoxideLabs/gitoxide/commit/654cf39c92d5aa4c8d542a6cadf13d4acef6a78e))
    - Merge branch 'main' into repo-status ([`9679d6b`](https://github.com/GitoxideLabs/gitoxide/commit/9679d6b0e68c28438e22cb65c554d0b31dfaf159))
    - Merge branch 'git-sec' ([`cd723b5`](https://github.com/GitoxideLabs/gitoxide/commit/cd723b5ae11148e7e9fd07daf28bc04455d5c46f))
    - Release git-diff v0.14.0, git-bitmap v0.1.0, git-index v0.2.0, git-tempfile v2.0.1, git-lock v2.0.0, git-mailmap v0.1.0, git-traverse v0.13.0, git-pack v0.17.0, git-quote v0.2.0, git-odb v0.27.0, git-packetline v0.12.4, git-url v0.4.0, git-transport v0.16.0, git-protocol v0.15.0, git-ref v0.12.0, git-worktree v0.1.0, git-repository v0.15.0, cargo-smart-release v0.9.0, safety bump 5 crates ([`e58dc30`](https://github.com/GitoxideLabs/gitoxide/commit/e58dc3084cf17a9f618ae3a6554a7323e44428bf))
    - Release git-hash v0.9.3, git-features v0.20.0, git-config v0.2.0, safety bump 12 crates ([`f0cbb24`](https://github.com/GitoxideLabs/gitoxide/commit/f0cbb24b2e3d8f028be0e773f9da530da2656257))
    - Merge branch 'short-id' ([`5849d5b`](https://github.com/GitoxideLabs/gitoxide/commit/5849d5b326b83f98a16cf1d956c720c7f0fd4445))
    - Thanks clippy ([`4618f8a`](https://github.com/GitoxideLabs/gitoxide/commit/4618f8aa7648c0553a8e1b023fceb6738654e38b))
    - Upgrade document-features ([`c35e62e`](https://github.com/GitoxideLabs/gitoxide/commit/c35e62e0da9ac1f7dcb863f5f9c69108c728d32e))
    - Release git-hash v0.9.2, git-object v0.17.1, git-pack v0.16.1 ([`0db19b8`](https://github.com/GitoxideLabs/gitoxide/commit/0db19b8deaf11a4d4cbc03fa3ae40eea104bc302))
    - Release git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`b286b24`](https://github.com/GitoxideLabs/gitoxide/commit/b286b24a51878be7d2e0fd77ff0c5c99b439a6a0))
    - Release git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`42ebb53`](https://github.com/GitoxideLabs/gitoxide/commit/42ebb536cd6086f096b8422291776c9720fa0948))
    - Release git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`1b76119`](https://github.com/GitoxideLabs/gitoxide/commit/1b76119259b8168aeb99cbbec233f7ddaa2d7d2c))
    - Release git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`8f57c29`](https://github.com/GitoxideLabs/gitoxide/commit/8f57c297d7d6ed68cf51415ea7ede4bf9263326e))
    - Release git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0 ([`d78aab7`](https://github.com/GitoxideLabs/gitoxide/commit/d78aab7b9c4b431d437ac70a0ef96263acb64e46))
    - Release git-hash v0.9.1, git-features v0.19.1, git-actor v0.8.0, git-config v0.1.10, git-object v0.17.0, git-diff v0.13.0, git-tempfile v1.0.4, git-chunk v0.3.0, git-traverse v0.12.0, git-pack v0.16.0, git-odb v0.26.0, git-packetline v0.12.3, git-url v0.3.5, git-transport v0.15.0, git-protocol v0.14.0, git-ref v0.11.0, git-repository v0.14.0, cargo-smart-release v0.8.0, safety bump 4 crates ([`373cbc8`](https://github.com/GitoxideLabs/gitoxide/commit/373cbc877f7ad60dac682e57c52a7b90f108ebe3))
    - Prepare changelogs for release ([`674ec73`](https://github.com/GitoxideLabs/gitoxide/commit/674ec73b0816baa2c63b4ef1b40b7a41849c5e95))
    - Prepar changelogs for cargo-smart-release release ([`8900d69`](https://github.com/GitoxideLabs/gitoxide/commit/8900d699226eb0995be70d66249827ce348261df))
    - Release git-bitmap v0.0.1, git-hash v0.9.0, git-features v0.19.0, git-index v0.1.0, safety bump 9 crates ([`4624725`](https://github.com/GitoxideLabs/gitoxide/commit/4624725f54a34dd6b35d3632fb3516965922f60a))
    - Merge branch 'sync-db-draft' ([`7d2e20c`](https://github.com/GitoxideLabs/gitoxide/commit/7d2e20c6fedc2c7e71a307d8d072412fa847a4aa))
    - Thanks clippy ([`4ca9e07`](https://github.com/GitoxideLabs/gitoxide/commit/4ca9e07c7ac062d48d64ad7b516274e32dbc51c6))
    - Thanks clippy ([`7dd2313`](https://github.com/GitoxideLabs/gitoxide/commit/7dd2313d980fe7c058319ae66d313b3097e3ae5f))
    - Release git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0 ([`d3f9227`](https://github.com/GitoxideLabs/gitoxide/commit/d3f922781a81e8fbb81aa47afdbe9afeb06d666b))
    - Release git-features v0.18.0, git-actor v0.7.0, git-config v0.1.9, git-object v0.16.0, git-diff v0.12.0, git-traverse v0.11.0, git-pack v0.15.0, git-odb v0.25.0, git-packetline v0.12.2, git-transport v0.14.0, git-protocol v0.13.0, git-ref v0.10.0, git-repository v0.13.0, cargo-smart-release v0.7.0, safety bump 12 crates ([`acd3737`](https://github.com/GitoxideLabs/gitoxide/commit/acd37371dcd92ebac3d1f039224d02f2b4e9fa0b))
    - Adjust changelogs prior to release ([`ec38950`](https://github.com/GitoxideLabs/gitoxide/commit/ec3895005d141abe79764eaff7c0f04153e38d73))
    - Release git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0 ([`f606fa9`](https://github.com/GitoxideLabs/gitoxide/commit/f606fa9a0ca338534252df8921cd5e9d3875bf94))
    - Better changelog descriptions. ([`f69b2d6`](https://github.com/GitoxideLabs/gitoxide/commit/f69b2d627099639bc144fd94fde678d84a10d6f7))
    - Adjusting changelogs prior to release of git-config v0.1.8, git-object v0.15.1, git-diff v0.11.1, git-traverse v0.10.1, git-pack v0.14.0, git-odb v0.24.0, git-packetline v0.12.1, git-transport v0.13.1, git-protocol v0.12.1, git-ref v0.9.1, git-repository v0.12.0, cargo-smart-release v0.6.0, safety bump 5 crates ([`39b40c8`](https://github.com/GitoxideLabs/gitoxide/commit/39b40c8c3691029cc146b893fa0d8d25d56d0819))
    - Release git-hash v0.8.0, git-features v0.17.0, git-actor v0.6.0, git-object v0.15.0, git-diff v0.11.0, git-traverse v0.10.0, git-pack v0.13.0, git-odb v0.23.0, git-packetline v0.12.0, git-transport v0.13.0, git-protocol v0.12.0, git-ref v0.9.0, git-repository v0.11.0, git-commitgraph v0.6.0, gitoxide-core v0.12.0, gitoxide v0.10.0, cargo-smart-release v0.5.0, safety bump 16 crates ([`0e02953`](https://github.com/GitoxideLabs/gitoxide/commit/0e029537a7f6242d02ccf7e63d8d92f5246e6c5e))
    - Release git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-config v0.1.7, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0 ([`59ffbd9`](https://github.com/GitoxideLabs/gitoxide/commit/59ffbd9f15583c8248b7f48b3f55ec6faffe7cfe))
    - Adjusting changelogs prior to release of git-hash v0.7.0, git-features v0.16.5, git-actor v0.5.3, git-validate v0.5.3, git-object v0.14.1, git-diff v0.10.0, git-tempfile v1.0.3, git-lock v1.0.1, git-traverse v0.9.0, git-pack v0.12.0, git-odb v0.22.0, git-packetline v0.11.0, git-url v0.3.4, git-transport v0.12.0, git-protocol v0.11.0, git-ref v0.8.0, git-repository v0.10.0, cargo-smart-release v0.4.0, safety bump 3 crates ([`a474395`](https://github.com/GitoxideLabs/gitoxide/commit/a47439590e36b1cb8b516b6053fd5cbfc42efed7))
    - Make fmt, but now it picked up some parts that usually don't get altered… ([`01f7b72`](https://github.com/GitoxideLabs/gitoxide/commit/01f7b729337bd2c99498321c479a9a13b1858e3e))
    - Update changelogs just for fun ([`21541b3`](https://github.com/GitoxideLabs/gitoxide/commit/21541b3301de1e053fc0e84373be60d2162fbaae))
    - Merge branch 'changelog-generation' ([`bf0106e`](https://github.com/GitoxideLabs/gitoxide/commit/bf0106ea21734d4e59d190b424c22743c22da966))
    - Merge branch 'main' into changelog-generation ([`c956f33`](https://github.com/GitoxideLabs/gitoxide/commit/c956f3351d766c748faf0460780e32ac8dfe8165))
    - Bump git-traverse v0.9.0, safety bump 8 crates ([`d39fabb`](https://github.com/GitoxideLabs/gitoxide/commit/d39fabb8757369aa19452a457f610fe21dc13a14))
    - Release git-protocol v0.10.4 ([`898ee08`](https://github.com/GitoxideLabs/gitoxide/commit/898ee08befa1eb7dd22980063c7633f83d0a8958))
    - Thanks clippy ([`4701296`](https://github.com/GitoxideLabs/gitoxide/commit/4701296bd5e2c4ad2f80f4e1de498db49f93385a))
    - Release git-protocol v0.10.3 ([`aa90f98`](https://github.com/GitoxideLabs/gitoxide/commit/aa90f98eb45e93b629462b629660e38b1824c405))
    - Merge branch 'repository-integration' ([`49f5453`](https://github.com/GitoxideLabs/gitoxide/commit/49f5453629646ac24d752f53c532e5f67eb09374))
    - Bump git-hash v0.6.0 ([`6efd90d`](https://github.com/GitoxideLabs/gitoxide/commit/6efd90db54f7f7441b76159dba3be80c15657a3d))
    - Release git-protocol v0.10.2 ([`54a4400`](https://github.com/GitoxideLabs/gitoxide/commit/54a44009e3507ee1c53a51a5f3b6735b6115a887))
    - [various #184] configure docs.rs build features ([`cc50249`](https://github.com/GitoxideLabs/gitoxide/commit/cc502492c512293e93e95610ca80a71896076ded))
    - Release git-protocol v0.10.1 ([`cec8ee3`](https://github.com/GitoxideLabs/gitoxide/commit/cec8ee3709ed401303cdd412a53e73f91eced619))
    - [protocol #174] fix tests… ([`cdc16fc`](https://github.com/GitoxideLabs/gitoxide/commit/cdc16fc0ef42df4a17ec4fde4be4511ee2cdaed6))
    - Bump git-protocol v0.10.0 ([`82d5a0b`](https://github.com/GitoxideLabs/gitoxide/commit/82d5a0bb38903a8389e43cd5416e02e5496e661a))
    - Bump git-transport v0.11.0 ([`1149f1b`](https://github.com/GitoxideLabs/gitoxide/commit/1149f1b716624f8f4fdaed20c803530aebc45599))
    - Bump git-packetline v0.10.0 ([`b09f391`](https://github.com/GitoxideLabs/gitoxide/commit/b09f3912e0addd7b4b0ef22bc3a24869d5011646))
    - [packetline #178] rename PacketLine to PacketLineRef… ([`d4c16a9`](https://github.com/GitoxideLabs/gitoxide/commit/d4c16a93946244177606b58cc702b81a16424ad4))
    - [stability #171] Prime git-tempfile and git-lock for release ([`01278fe`](https://github.com/GitoxideLabs/gitoxide/commit/01278fe4e28bf97ce6a2b8947198683646e361ee))
    - Merge branch 'Byron:main' into main ([`dc58eca`](https://github.com/GitoxideLabs/gitoxide/commit/dc58eca510e5a067acdeaad4b595a34b4598a0cd))
    - Upgrade to nom-7 ([`f0aa3e1`](https://github.com/GitoxideLabs/gitoxide/commit/f0aa3e1b5b407b2afd187c9cb622676fcddaf706))
    - [protocol] prepare release to fix crates-io instalations ([`83d7423`](https://github.com/GitoxideLabs/gitoxide/commit/83d74239108df420bd464c340762c1dfcb6ae78a))
    - Bump git-protocol to v0.9.0 as there are breaking changes ([`b4e3340`](https://github.com/GitoxideLabs/gitoxide/commit/b4e33408b8eb12c9418704f663322385fd1dfb25))
    - Apply nightly rustfmt rules. ([`5e0edba`](https://github.com/GitoxideLabs/gitoxide/commit/5e0edbadb39673d4de640f112fa306349fb11814))
    - Release git-protocol v0.8.1 ([`b57c339`](https://github.com/GitoxideLabs/gitoxide/commit/b57c3397706940354c493cadf1ab93916b79f917))
    - Release git-transport v0.10.0 ([`b944278`](https://github.com/GitoxideLabs/gitoxide/commit/b94427835bf922aa9388cdd78200c79a3c31da43))
    - Release git-packetline v0.9.0 ([`7ffbd60`](https://github.com/GitoxideLabs/gitoxide/commit/7ffbd602c08605026b0bb97ab85216907badaf09))
    - Remove dev-dependency cycles by removing their version ([`c40faca`](https://github.com/GitoxideLabs/gitoxide/commit/c40faca41632cd2a226daf4ddf5293b65d1fdc82))
    - Bump transport version to 0.10 ([`f26a3d3`](https://github.com/GitoxideLabs/gitoxide/commit/f26a3d3a2745f3eb69d76e0cfd718a90cf74f003))
    - (cargo-release) version 0.8.0 ([`ad6d7f9`](https://github.com/GitoxideLabs/gitoxide/commit/ad6d7f9c2b4f8879d466e758fc9b51ece6879e96))
    - (cargo-release) version 0.7.0 ([`2ef3106`](https://github.com/GitoxideLabs/gitoxide/commit/2ef3106eb84981e2dabd84f81362b4e44f938ea6))
    - [transport] A much better name for 'is_stateful()` ([`f15f1e8`](https://github.com/GitoxideLabs/gitoxide/commit/f15f1e85fda76eef72c3754d625cf51e3c454eea))
    - [protocol] Make fetch-connection usage explicit ([`29696f9`](https://github.com/GitoxideLabs/gitoxide/commit/29696f9b8e3ba3a72af1b099dac1c0866194d5ce))
    - (cargo-release) version 0.5.0 ([`ae02dab`](https://github.com/GitoxideLabs/gitoxide/commit/ae02dabae961089a92a21e6a60a7006de4b56dad))
    - (cargo-release) version 0.16.0 ([`1231dbd`](https://github.com/GitoxideLabs/gitoxide/commit/1231dbd16dacefb39adec8e067c312d313a82e3c))
    - [protocol RL-#741] Respect delegate configuration when running only ls-refs ([`65ce8e1`](https://github.com/GitoxideLabs/gitoxide/commit/65ce8e1812ce820e3a0c40e39170339bf73234e5))
    - [protocol #145] Unify the `previous` and `previous_result` parameters… ([`96f77c7`](https://github.com/GitoxideLabs/gitoxide/commit/96f77c78a08e975d367ca25ac5d07eb2253cf4e5))
    - Merge pull request #145 from kim/ref-in-want-corrections ([`8dfc943`](https://github.com/GitoxideLabs/gitoxide/commit/8dfc943382909b45959b49381f6c21431d343a63))
    - [protocol] remove misleading documentation about ref-in-want ([`9a8f6b5`](https://github.com/GitoxideLabs/gitoxide/commit/9a8f6b5480bf55f52315ddf86ac28771147a4664))
    - Clippy on tests and thanks clippy ([`a77a71c`](https://github.com/GitoxideLabs/gitoxide/commit/a77a71cf02d328a2a964388928d6b2a235a0aa85))
    - Thanks clippy ([`e1964e4`](https://github.com/GitoxideLabs/gitoxide/commit/e1964e43979b3e32a5d4bfbe377a842d2c0b10ea))
    - Bump async-trait from 0.1.50 to 0.1.51 ([`ce0b81e`](https://github.com/GitoxideLabs/gitoxide/commit/ce0b81e8f5c652d389ff876844bc42bcfa687921))
    - Bump futures-io from 0.3.15 to 0.3.16 ([`3c23820`](https://github.com/GitoxideLabs/gitoxide/commit/3c23820d3f0d3567f44215cdb0ad13ab675a201f))
    - [protocol] Delegate will indicate end-of-operation when fetch is done ([`928f75a`](https://github.com/GitoxideLabs/gitoxide/commit/928f75ad939e35a159d3d2751d5d0f9d00d796af))
    - [protocol] Let 'fetch()' only be used via `git_protocol::fetch` ([`4bae2f9`](https://github.com/GitoxideLabs/gitoxide/commit/4bae2f959bdf3f4a12b378f5734d9abdc25af36d))
    - Thanks clippy ([`eccbecb`](https://github.com/GitoxideLabs/gitoxide/commit/eccbecb938f0c84b63ad7e1ee17fb8113ce89c2e))
    - [protocol] fix build ([`38aca40`](https://github.com/GitoxideLabs/gitoxide/commit/38aca4076037a6f8288c2cf483f134ea16c328d5))
    - [protocol] Allow both preparation delegate methods to fail ([`d89393b`](https://github.com/GitoxideLabs/gitoxide/commit/d89393bbd5fce130a50855316ef364083c62eccd))
    - [protocol] start trying LsRefsAction::Abort(Box<dyn Error>)… ([`660b9dc`](https://github.com/GitoxideLabs/gitoxide/commit/660b9dcc4e5249506a7656b038333f64b109261d))
    - Merge branch 'negotiate-fallible' ([`27c8abe`](https://github.com/GitoxideLabs/gitoxide/commit/27c8abe1948bc10c779efa33d4bc0b92741f6444))
    - [protocol] adjust description of fetch::Error to match io::Error sources ([`23dafc6`](https://github.com/GitoxideLabs/gitoxide/commit/23dafc6e24377ad00b70c0235fd7a8ff107eee0a))
    - [protocol] fallible negotiation ([`e269a2c`](https://github.com/GitoxideLabs/gitoxide/commit/e269a2cde18f604a36b33efb7e53f31ea5c45e2d))
    - Revert "[ref] Try using BorrowMut to avoid blanket trait impls, but…" ([`8212536`](https://github.com/GitoxideLabs/gitoxide/commit/8212536376341e673a6ef05221d20815659d92d3))
    - [ref] Try using BorrowMut to avoid blanket trait impls, but… ([`4bb9bba`](https://github.com/GitoxideLabs/gitoxide/commit/4bb9bbad5b4e0c2e64a48a8e4a70a1b3af1ca3e3))
    - [protocol] only send flush packets in stateful connections ([`0995c22`](https://github.com/GitoxideLabs/gitoxide/commit/0995c225c92a0dcccd2514b53abcf8400d9342e1))
    - [transport] remove Transport::close()… ([`4268a9b`](https://github.com/GitoxideLabs/gitoxide/commit/4268a9bcf733413f7326be7af487a8fcdec1f71c))
    - [ref] rename Action::Close to Action::Cancel… ([`cac1f6c`](https://github.com/GitoxideLabs/gitoxide/commit/cac1f6c757709797d193c6bca30e99fe40466ddc))
    - [transport] impl Delegate for &mut T: Delegate; refactor fetch() signature ([`2ded7f9`](https://github.com/GitoxideLabs/gitoxide/commit/2ded7f9b2659ab8705ad6b896aaf6ca5afb12a6c))
    - [transport] implement Transport for &mut T: Transport as well ([`372fb81`](https://github.com/GitoxideLabs/gitoxide/commit/372fb8183aff19bd0f2d17ea74409b2ca3a08511))
    - Merge branch 'ref-in-want' ([`f248557`](https://github.com/GitoxideLabs/gitoxide/commit/f248557186384501e705473e0adab03d3fa10519))
    - [protocol] refactor ([`11b2fd1`](https://github.com/GitoxideLabs/gitoxide/commit/11b2fd1250ff902dd084d8664f06732d4b69b4b3))
    - [protocol] refactor ([`967946a`](https://github.com/GitoxideLabs/gitoxide/commit/967946a65f67cb1fc5d7bf6944a7e900ff3521c7))
    - [protocol] refactor ([`8dc425f`](https://github.com/GitoxideLabs/gitoxide/commit/8dc425ff91d00d3315903f429d4009df6410ba77))
    - [protocol] assure we don't coerce refs into UTF-8 representation ([`5ceb64d`](https://github.com/GitoxideLabs/gitoxide/commit/5ceb64dfed67b942100e2e36715903492d870c71))
    - [protocol] support ref-in-want ([`b6df400`](https://github.com/GitoxideLabs/gitoxide/commit/b6df400dccd66ad2f01c80d2fa05b8f9bb130b23))
    - [transport] tests for extra parameters ([`fffd926`](https://github.com/GitoxideLabs/gitoxide/commit/fffd926a3d5c6abfa732aa2305a4a05fdd06254d))
    - [protocol] extra_parameters are forwarded from delegate to handshake ([`03e3db3`](https://github.com/GitoxideLabs/gitoxide/commit/03e3db3809bd031d7d0c151ada2542214d7e32c0))
    - [transport] unsupported protocol versions now abort the fetch operation ([`812aa3b`](https://github.com/GitoxideLabs/gitoxide/commit/812aa3bc02a823cb9277847db905e76a50ee7413))
    - [transport] flexible version of version support check doesn't actually work :D ([`2b220f0`](https://github.com/GitoxideLabs/gitoxide/commit/2b220f0758cb7a96a66b256552f13a020cdee3fc))
    - [protocol] make refs parsing functionality public ([`d6da891`](https://github.com/GitoxideLabs/gitoxide/commit/d6da891419f66208a8820185dd165e62b7a01a6e))
    - [protocol] async-io path handles improved refs parsing ([`328ab9c`](https://github.com/GitoxideLabs/gitoxide/commit/328ab9c4ce739fe79f12ae539ea37e50c541b786))
    - [protocol] first step towards keeping InternalRef internal in blocking-io ([`6c4ed2d`](https://github.com/GitoxideLabs/gitoxide/commit/6c4ed2d4dd352b4218419a1a79269a49cc91a992))
    - Refactor ([`24697bc`](https://github.com/GitoxideLabs/gitoxide/commit/24697bc66363f8e8b1ff14a59fdf303ffdab132d))
    - [async-client] cleanup Send bounds! ([`c7dee44`](https://github.com/GitoxideLabs/gitoxide/commit/c7dee44267462d5ece491b8a45cf35afa904ce81))
    - [async-client] refactor ([`b252932`](https://github.com/GitoxideLabs/gitoxide/commit/b252932ee3eb26bb26560a849a9b13aca11cf00f))
    - [async-client] unblock the async delegate in the cheapest possible way… ([`a3b5d75`](https://github.com/GitoxideLabs/gitoxide/commit/a3b5d75d387dc5d6c44f695f63df8803613637a2))
    - Revert "[async-client] a taste of what it means to unblock the delegate" ([`2ba452f`](https://github.com/GitoxideLabs/gitoxide/commit/2ba452ff1c9659f7433328b12732d792e7871102))
    - [async-client] a taste of what it means to unblock the delegate ([`4d6c10a`](https://github.com/GitoxideLabs/gitoxide/commit/4d6c10a6956bb9a81144a61ebb6bcab3aedb840e))
    - [async-client] prepare for unblocking the protocol delegate ([`796c7d5`](https://github.com/GitoxideLabs/gitoxide/commit/796c7d54a20ef32a581be572e1d681f9727482de))
    - [async-client] refactor ([`0d5b911`](https://github.com/GitoxideLabs/gitoxide/commit/0d5b911ad5f47ab8f044d6bbe660a6d1dfeecb5f))
    - Revert "[async-client] Try to bring 'Send' back but…" ([`52eb953`](https://github.com/GitoxideLabs/gitoxide/commit/52eb953fcc44cce19604b1df6a600237b8c81392))
    - [async-client] Try to bring 'Send' back but… ([`3a06adb`](https://github.com/GitoxideLabs/gitoxide/commit/3a06adb41f6b2946f78044e4ab1385e6441fc40f))
    - [git-protocol] fix test ([`e30ea36`](https://github.com/GitoxideLabs/gitoxide/commit/e30ea363311aa82486828c59755a012cc76751b1))
    - [git-protocol] no warnings when building without client ([`2f30666`](https://github.com/GitoxideLabs/gitoxide/commit/2f3066659280f7b43ca39d285166f11192ac7fa9))
    - [git-protocol] remove compile warnings if no client type is specified… ([`478a980`](https://github.com/GitoxideLabs/gitoxide/commit/478a98056afd2504050391262dabc921b59425c5))
    - Thanks clippy ([`57106e2`](https://github.com/GitoxideLabs/gitoxide/commit/57106e21089ae3c3a529295bceb8c0a515e2c2b6))
    - [git-protocol] builds without features work ([`a1945ff`](https://github.com/GitoxideLabs/gitoxide/commit/a1945ff22f3412be1fbfac76236d487896ec4685))
    - [git-protocol] async fetch tests work ([`fe434a5`](https://github.com/GitoxideLabs/gitoxide/commit/fe434a58d321b3ac12644827e65eb4db11cfe5fb))
    - [git-protocol] fetch tests nearly compile in async ([`97fb186`](https://github.com/GitoxideLabs/gitoxide/commit/97fb186df5661fb297c2c9485186dbfe0ed1d504))
    - [git-protocol] fetch in sync and async… ([`4776039`](https://github.com/GitoxideLabs/gitoxide/commit/47760399bffd030c848e0ef6df52a4765d8fb566))
    - [git-protocol] refactor ([`80379fd`](https://github.com/GitoxideLabs/gitoxide/commit/80379fd32aae02f2975d8637326188655f85b474))
    - [git-protocol] build should fail if mutually exclusiive features are set ([`72cf940`](https://github.com/GitoxideLabs/gitoxide/commit/72cf9401dda6e1bb465cce8d65ce66a7cc6a03fd))
    - Bump maybe-async from 0.2.4 to 0.2.6 ([`d99a1a8`](https://github.com/GitoxideLabs/gitoxide/commit/d99a1a815809d22c7384c6ecb1275e39fb911d91))
    - [git-protocol] fix build ([`4cce648`](https://github.com/GitoxideLabs/gitoxide/commit/4cce6487d6d514541afee1a9aa92043f186136d3))
    - [git-protocol] async Delegate ([`1aa6781`](https://github.com/GitoxideLabs/gitoxide/commit/1aa678172f0eb75af76017addd3dff4d7e62ff41))
    - Thanks clippy ([`0759ade`](https://github.com/GitoxideLabs/gitoxide/commit/0759ade3e8e97927f452eabd11e249bb93aa54e2))
    - [git-protocol] refactor ([`94d7be4`](https://github.com/GitoxideLabs/gitoxide/commit/94d7be4a16f2c2e68a9dacf120eef7a417a8a6b9))
    - [git-protocol] refactor ([`990099b`](https://github.com/GitoxideLabs/gitoxide/commit/990099b01bfd54b926f0f4e7ecf727c423a23b8e))
    - [git-protocol] refactor ([`d623cf7`](https://github.com/GitoxideLabs/gitoxide/commit/d623cf7db4488815ad5a2afd2d1bcbbbda275d2c))
    - [git-protocol] async response ([`c498557`](https://github.com/GitoxideLabs/gitoxide/commit/c49855738bc164f65130cb307ba612b71c3fa83e))
    - [git-protocol] refactor ([`a8dc078`](https://github.com/GitoxideLabs/gitoxide/commit/a8dc078e00d8a5689ba0d8070732421d35df50c8))
    - Refactor ([`2eefe17`](https://github.com/GitoxideLabs/gitoxide/commit/2eefe1712131a69298be02e94df8b6ba844afcd9))
    - [git-protocol] prepare response module for async ([`08b891b`](https://github.com/GitoxideLabs/gitoxide/commit/08b891b089081a3ec3c44ed27b1aca316391d0de))
    - [git-protocol] fix tests without any feature toggles ([`1da0b1a`](https://github.com/GitoxideLabs/gitoxide/commit/1da0b1ab9e22040d5b273a5604954859990e0334))
    - Thanks clippy ([`91fdfba`](https://github.com/GitoxideLabs/gitoxide/commit/91fdfba7cabc7331598903106a1dd7cea3b49eeb))
    - [git-protocol] refs now available in async ([`3a5b2cf`](https://github.com/GitoxideLabs/gitoxide/commit/3a5b2cfcc50a48e09a6495c4c15af69596f966df))
    - [git-protocol] refactor ([`abf0b9d`](https://github.com/GitoxideLabs/gitoxide/commit/abf0b9d41a2509d35102970602e77fb45e898d52))
    - [git-protocol] prepare to translate refs ([`bf79c91`](https://github.com/GitoxideLabs/gitoxide/commit/bf79c91b30be61135dd33122bb93b3cf3a49f586))
    - [git-protocol] no warnings if there is no client feature set ([`335e831`](https://github.com/GitoxideLabs/gitoxide/commit/335e83136efa7cb0913bc5e317bb49d616ee0290))
    - [git-protocol] fix tests in case there is no client feature set ([`1ee5518`](https://github.com/GitoxideLabs/gitoxide/commit/1ee551878ef21d20925fab00b3eef044ada97065))
    - [git-protocol] refactor ([`0b4ff16`](https://github.com/GitoxideLabs/gitoxide/commit/0b4ff166175dd51cded5131bcebf1edd80335abe))
    - [git-protocol] refactor ([`e99a03b`](https://github.com/GitoxideLabs/gitoxide/commit/e99a03b360e4bb757904a03834297f14df67838f))
    - [git-protocol] async capabilities and arguments abstractions ([`aa3eacb`](https://github.com/GitoxideLabs/gitoxide/commit/aa3eacbd53665d6b76bd9706d801d1189a970261))
    - [git-protocol] now just a dummy async transport impl and… ([`c7f0b80`](https://github.com/GitoxideLabs/gitoxide/commit/c7f0b80182c08430a3720474eda41519b6814f17))
    - [git-protocol] a big step towards getting 'Arguments' test into async ([`5d1c30f`](https://github.com/GitoxideLabs/gitoxide/commit/5d1c30f3ceae6fe26a0d9961d135b44f371d9cd7))
    - [git-protocol] move everything into `blocking_io` for later translation… ([`fa03374`](https://github.com/GitoxideLabs/gitoxide/commit/fa03374fd42e127f5be7fb4da2bac85ea38c8afa))
    - [git-protocol] all blocking fetch tests ([`0d39b5d`](https://github.com/GitoxideLabs/gitoxide/commit/0d39b5d23659d29a9f0e33428db401a3a887c007))
    - [git-protocol] re-introduce credentials helper code ([`6a5575f`](https://github.com/GitoxideLabs/gitoxide/commit/6a5575fa7dbfa2a835fabf6746494097c3af23c2))
    - [git-protocol] separate test configuration for async mode ([`62a117c`](https://github.com/GitoxideLabs/gitoxide/commit/62a117c4e6bd205c4bb1d224db7d8e80ba46322f))
    - [git-transport] fix git-protocol ([`0cc9537`](https://github.com/GitoxideLabs/gitoxide/commit/0cc9537036003c86584223aa61f9c207a2c5c2df))
    - [git-protocol] simplify test setup ([`189ed2c`](https://github.com/GitoxideLabs/gitoxide/commit/189ed2c32636ef59975dd15ec0ef61e8a62b98c0))
    - Refactor ([`2ba9f91`](https://github.com/GitoxideLabs/gitoxide/commit/2ba9f915035a518bef3eb8b0ed1c9972c4a47cfa))
    - (cargo-release) version 0.4.0 ([`866f86f`](https://github.com/GitoxideLabs/gitoxide/commit/866f86f59e66652968dcafc1a57912f9849cb21d))
    - Switch to latest nom ([`859e57e`](https://github.com/GitoxideLabs/gitoxide/commit/859e57eae93c3490523b7ed98f7a606acbd87a2f))
    - (cargo-release) version 0.15.0 ([`d69d9fb`](https://github.com/GitoxideLabs/gitoxide/commit/d69d9fb0931f8257cef96ef14a89da9340ad9738))
    - Put prodash behind a feature toggle, too ([`966058d`](https://github.com/GitoxideLabs/gitoxide/commit/966058d611c548e90c050462de52e36f1925e775))
    - [git-packetline] refactor ([`1328c5b`](https://github.com/GitoxideLabs/gitoxide/commit/1328c5b4001f380936beff73e1f822f14e41e98b))
    - (cargo-release) version 0.6.0 ([`ec5a54e`](https://github.com/GitoxideLabs/gitoxide/commit/ec5a54e9f3543afddc9f972f16135edc6ef6ff5b))
    - [git-packetline] refactor ([`e5769d1`](https://github.com/GitoxideLabs/gitoxide/commit/e5769d1e7668ae54c667d2593c0c22e7723710c0))
    - (cargo-release) version 0.8.0 ([`ccea4b6`](https://github.com/GitoxideLabs/gitoxide/commit/ccea4b6bcdaba0ee6c6a6236d225ea1276d2547c))
    - (cargo-release) version 0.9.0 ([`18f6d01`](https://github.com/GitoxideLabs/gitoxide/commit/18f6d011043203523f1d0dacf657704ed3f9cf89))
    - [git-transport] simplify parsing capabilities from lines ([`401af09`](https://github.com/GitoxideLabs/gitoxide/commit/401af0974742f10c8b9b3c9752e9d30205e96c16))
    - [git-protocol] separate tests those who need feature toggles ([`4a49d64`](https://github.com/GitoxideLabs/gitoxide/commit/4a49d6406c9c39d75ab5021b6e213fd2c9d63adb))
    - [git-transport] remove default features to force being explicit everywhere ([`d1b39f8`](https://github.com/GitoxideLabs/gitoxide/commit/d1b39f8093c032a172237a584c9208479611a866))
    - Fix git-protocol ([`284f8af`](https://github.com/GitoxideLabs/gitoxide/commit/284f8af0599bee4e3de0e385b69a389713cee9f7))
    - Refactor ([`1412282`](https://github.com/GitoxideLabs/gitoxide/commit/141228219d33e8056489514f91221d803888edd8))
    - (cargo-release) version 0.7.0 ([`069184e`](https://github.com/GitoxideLabs/gitoxide/commit/069184e55057a1655d2754cb1fd68a4424beff34))
    - (cargo-release) version 0.8.0 ([`411a05e`](https://github.com/GitoxideLabs/gitoxide/commit/411a05ead1546c76fe51f359fbcb961a1140535e))
    - (cargo-release) version 0.5.0 ([`8c4cc3f`](https://github.com/GitoxideLabs/gitoxide/commit/8c4cc3fb5922d1a761463bbbad65e59f91cce4cb))
    - Thanks clippy ([`17258cc`](https://github.com/GitoxideLabs/gitoxide/commit/17258cc58767caa6e71227898decd160ad0cdf13))
    - (cargo-release) version 0.14.0 ([`a760f8c`](https://github.com/GitoxideLabs/gitoxide/commit/a760f8c013e13ba82daa1acf1a4a57e0818a008d))
    - (cargo-release) version 0.3.0 ([`e9665c7`](https://github.com/GitoxideLabs/gitoxide/commit/e9665c784ae7e5cdaf662151395ee2355e9b57b6))
    - (cargo-release) version 0.13.0 ([`ac2eddb`](https://github.com/GitoxideLabs/gitoxide/commit/ac2eddb06eb3d8a9a3dcdcd796eb54a7e45ab935))
    - (cargo-release) version 0.6.0 ([`8513f0f`](https://github.com/GitoxideLabs/gitoxide/commit/8513f0fafbf8ae61d86df2d8b0aefa52d3eb1680))
    - (cargo-release) version 0.7.0 ([`334b7e1`](https://github.com/GitoxideLabs/gitoxide/commit/334b7e1b838b5201f2484be42dee3c4d2fd789d7))
    - (cargo-release) version 0.12.0 ([`3b71e7e`](https://github.com/GitoxideLabs/gitoxide/commit/3b71e7e8416e550b47e5aed2259c1181497ac9e8))
    - (cargo-release) version 0.2.0 ([`4ec09f4`](https://github.com/GitoxideLabs/gitoxide/commit/4ec09f4d2239ea1d44f7145027e64191bf2c158c))
    - (cargo-release) version 0.5.0 ([`3cc4a57`](https://github.com/GitoxideLabs/gitoxide/commit/3cc4a5799fa1f487452b5c346b57fea97e45b47e))
    - (cargo-release) version 0.6.0 ([`50fb6f2`](https://github.com/GitoxideLabs/gitoxide/commit/50fb6f25e9afa900ac1c3cfb88d7ca0d5a9a95f7))
    - Thanks clippy ([`0fc239c`](https://github.com/GitoxideLabs/gitoxide/commit/0fc239cf9b773f72928b7c42344b578c6ff5d19f))
    - Thanks clippy ([`749ceba`](https://github.com/GitoxideLabs/gitoxide/commit/749ceba246fb8a4cb8d48fa86184619fef500108))
    - (cargo-release) version 0.11.0 ([`1aa1f5e`](https://github.com/GitoxideLabs/gitoxide/commit/1aa1f5e84a07427d5d7f3231735fe9c1923f506f))
    - (cargo-release) version 0.4.1 ([`6244fb4`](https://github.com/GitoxideLabs/gitoxide/commit/6244fb4cfbc40d35f46f4d1942519414a04ac355))
    - Finish docs for `git-protocol` crate ([`598f700`](https://github.com/GitoxideLabs/gitoxide/commit/598f700ce2a273a6f430c8d2442dbd71e21a2704))
    - Revise trait documentation of git-protocol ([`5271128`](https://github.com/GitoxideLabs/gitoxide/commit/52711283d456eefcbdc37ac8f8da36149afc1322))
    - Docs for response in git-protocol ([`487de13`](https://github.com/GitoxideLabs/gitoxide/commit/487de1383d801fb442abc8101666a0d9a050af15))
    - More docs for git-protocol ([`bca0cbd`](https://github.com/GitoxideLabs/gitoxide/commit/bca0cbd98ab02b63ac24b1d15baea602b02e1623))
    - Docs for fetch::refs ([`6a97a3e`](https://github.com/GitoxideLabs/gitoxide/commit/6a97a3e5883d9a6c0011a68b16966d1f8be589d7))
    - Docs for git credentials helper utilities ([`eb6bb6e`](https://github.com/GitoxideLabs/gitoxide/commit/eb6bb6ee2fe22ad0621f7e1743a7e56adbc54bd1))
    - First pieces of docs for git-protocol ([`12d8a83`](https://github.com/GitoxideLabs/gitoxide/commit/12d8a83fbc1b70bd2612ad62aa1a69e87914fe39))
    - Thanks clippy ([`343ab9a`](https://github.com/GitoxideLabs/gitoxide/commit/343ab9adb62da1dde495fc209c179137bbe59a10))
    - (cargo-release) version 0.5.0 ([`28df5e9`](https://github.com/GitoxideLabs/gitoxide/commit/28df5e9131aec3efb2b68db204662b92b232b33c))
    - All crates use git-hash::Kind and its types, sometimes through git-object ([`124c171`](https://github.com/GitoxideLabs/gitoxide/commit/124c171aaf546d8977e9913ff84e65383a80ee98))
    - Use git-hash in git-features ([`5b307e0`](https://github.com/GitoxideLabs/gitoxide/commit/5b307e076f6f5975592c8b177c122c91c1d809c6))
    - (cargo-release) version 0.3.0 ([`e60dbe6`](https://github.com/GitoxideLabs/gitoxide/commit/e60dbe6c21843eab44d6f05fe70927252453cb41))
    - (cargo-release) version 0.4.0 ([`32aefc0`](https://github.com/GitoxideLabs/gitoxide/commit/32aefc051c7ad9d1a160f77db070df7fa4843dbc))
    - (cargo-release) version 0.4.0 ([`72eaece`](https://github.com/GitoxideLabs/gitoxide/commit/72eaeceed135e4cc5c943685f4c902d03597c4d2))
    - (cargo-release) version 0.9.0 ([`a89fdb9`](https://github.com/GitoxideLabs/gitoxide/commit/a89fdb98f64bb0ca070fa79a1f58f1232bb14090))
    - (cargo-release) version 0.5.0 ([`fc7d600`](https://github.com/GitoxideLabs/gitoxide/commit/fc7d600ac2c438c8b6b91f67cb69b0ac5ec37675))
    - (cargo-release) version 0.2.0 ([`a476a46`](https://github.com/GitoxideLabs/gitoxide/commit/a476a46b7b933a3c2fa4aa8c285beec1777a3f2d))
    - (cargo-release) version 0.3.0 ([`d19ee35`](https://github.com/GitoxideLabs/gitoxide/commit/d19ee35cc6683c63e0eabd717e4758075faeaa71))
    - (cargo-release) version 0.3.0 ([`eade7d1`](https://github.com/GitoxideLabs/gitoxide/commit/eade7d101e071153055b07d9c6ae3c1452493a21))
    - (cargo-release) version 0.8.0 ([`47c00c2`](https://github.com/GitoxideLabs/gitoxide/commit/47c00c2228cf25c79e1fa3eb4229c7ab24de91e5))
    - Cargo clippy Rust 1.48 ([`475a68c`](https://github.com/GitoxideLabs/gitoxide/commit/475a68ce33b895de911939c51afa159df534f7b8))
    - (cargo-release) version 0.7.0 ([`7fa7bae`](https://github.com/GitoxideLabs/gitoxide/commit/7fa7baeb3e7d008a25e4d714eff908e2516c828b))
    - Thanks clippy ([`b9e0a87`](https://github.com/GitoxideLabs/gitoxide/commit/b9e0a87996b8f3c4531a392607c353a1f0824ce6))
    - Remove dash in all repository links ([`98c1360`](https://github.com/GitoxideLabs/gitoxide/commit/98c1360ba4d2fb3443602b7da8775906224feb1d))
    - Merge from main. ([`b59bd5e`](https://github.com/GitoxideLabs/gitoxide/commit/b59bd5e0b0895c7d1d585816cec8be4dea78c278))
    - Refactor ([`7c3c80a`](https://github.com/GitoxideLabs/gitoxide/commit/7c3c80acf487296014ae9f2f9b88865c6aa6d98e))
    - (cargo-release) version 0.6.0 ([`9ef184e`](https://github.com/GitoxideLabs/gitoxide/commit/9ef184e35712f938fb4f9f6da7390a8777a9284e))
    - (cargo-release) version 0.1.1 ([`bb38c6b`](https://github.com/GitoxideLabs/gitoxide/commit/bb38c6b66e8de2b6743bb873c94afb187c8c8dd3))
    - Support V2 shallow-info section ([`6679c91`](https://github.com/GitoxideLabs/gitoxide/commit/6679c918628979efc73e68c60e0968058cd220db))
    - Tests for V2 shallow section parsing ([`5bf58ab`](https://github.com/GitoxideLabs/gitoxide/commit/5bf58ab344cb6b670ae535c7f7bca8a7f99a726c))
    - Support for the 'deepen-relative' argument ([`b86fed6`](https://github.com/GitoxideLabs/gitoxide/commit/b86fed6e415183f52bb34c68d8b503566740f671))
    - V1 parsing of shallow and unshallow lines… ([`8bcf535`](https://github.com/GitoxideLabs/gitoxide/commit/8bcf535a8b07d9b1d53fb84c73ba55c76a318daf))
    - Remove unused fixtures ([`6ae69f5`](https://github.com/GitoxideLabs/gitoxide/commit/6ae69f5f57ab371684e8c35cc77803aea05edd7b))
    - Fix wants/haves separator handling for stateful V1 ([`1629575`](https://github.com/GitoxideLabs/gitoxide/commit/16295757a33cdbdb8c69ba6c487ae8b298f612cd))
    - Make really clear that V2 is stateless no matter what the transport supports :D ([`c296845`](https://github.com/GitoxideLabs/gitoxide/commit/c296845201b379273ff8077489ace9ed33f416b7))
    - Assure the first 'want' in V1 is always first ([`e729ec8`](https://github.com/GitoxideLabs/gitoxide/commit/e729ec8f075a6c3122b42e367486a15c5367960f))
    - Properly handle statelessness in V2 protocol ([`1b49f1e`](https://github.com/GitoxideLabs/gitoxide/commit/1b49f1ef6d7a40e2dec07f9c08036b1b1d460f6b))
    - Add some samples for deepen clones ([`61bc41a`](https://github.com/GitoxideLabs/gitoxide/commit/61bc41a6f97decd3bdd96f874001ffb45251aca4))
    - Switch to prodash 10 and safe a lot of trait bounds in the process ([`e2fb1d9`](https://github.com/GitoxideLabs/gitoxide/commit/e2fb1d944b4d803a11c91f868b831d406fb5e35f))
    - (cargo-release) version 0.4.0 ([`0d7b60e`](https://github.com/GitoxideLabs/gitoxide/commit/0d7b60e856325009431172e1df742a1cd2165575))
    - (cargo-release) version 0.2.0 ([`779e9d0`](https://github.com/GitoxideLabs/gitoxide/commit/779e9d0ad67c20fa9cec14359e87774ca2d74ee4))
    - (cargo-release) version 0.2.0 ([`da830de`](https://github.com/GitoxideLabs/gitoxide/commit/da830defc9cfa81ce159f6d908da828227760845))
    - (cargo-release) version 0.5.0 ([`82b7313`](https://github.com/GitoxideLabs/gitoxide/commit/82b73131b79ec3c42a712dad1c0766a72209d737))
    - [clone] Assure we don't hang due to unprocessed headers when peeking lines! ([`d9ced27`](https://github.com/GitoxideLabs/gitoxide/commit/d9ced2711dba702d73b28f0e1b9399cd7eab5183))
    - [clone] more correct handling of 'no-done'/done when sending wants/haves… ([`50f4516`](https://github.com/GitoxideLabs/gitoxide/commit/50f4516adfa458f4b16e301340a39b3c34ddbef0))
    - [clone] Don't try to explicitly close the connection… ([`17200b3`](https://github.com/GitoxideLabs/gitoxide/commit/17200b3c494a24de19b7c6ec3191e61551a54380))
    - [clone] Fix encoding of V1 capabilities in first want ([`b68a5c5`](https://github.com/GitoxideLabs/gitoxide/commit/b68a5c57a6bd35391d8efb6436bb36e032851b49))
    - [clone] When unpacking peeled refs, use the object that refers to the tag… ([`fe8bb39`](https://github.com/GitoxideLabs/gitoxide/commit/fe8bb3985bd5529a36c71fa170ca48df91060491))
    - [clone] none the wiser - it really looks like everything is alright… ([`3b8d613`](https://github.com/GitoxideLabs/gitoxide/commit/3b8d613c6de349defce9af06d56f73ac2c0d0d25))
    - [clone] it looks like in order to figure out the issue, it needs tests higher up… ([`edf1540`](https://github.com/GitoxideLabs/gitoxide/commit/edf1540d2014eb26cd5b98aa1baaa1e0c99662bd))
    - [clone] Don't send V2 capabilities that don't have a value… ([`9c9a4ee`](https://github.com/GitoxideLabs/gitoxide/commit/9c9a4ee2a9c93612fd80844e8d2338461ee82ccc))
    - [clone] Handle remote progress name prefixing (more) correctly ([`51d4d15`](https://github.com/GitoxideLabs/gitoxide/commit/51d4d15028a4162fae2d4e68a8fbb34c6ba93cc7))
    - [clone] This actually works: first MVP of retrieving packs via clone ([`c06d819`](https://github.com/GitoxideLabs/gitoxide/commit/c06d8194173f9ec468ddd0faf72dd6d8dbf7d35d))
    - Use git attributes to prevent crlf conversion of fixtures on windows ([`80ca8b2`](https://github.com/GitoxideLabs/gitoxide/commit/80ca8b24b5565d82bc1f8e7d92c942f985e6ea3b))
    - [clone] Support for reading multi-step negoritaions, but… ([`507d342`](https://github.com/GitoxideLabs/gitoxide/commit/507d342dfe2a714a4dd0bc100d96ed9e64a58243))
    - [clone] refactor ([`ded46fd`](https://github.com/GitoxideLabs/gitoxide/commit/ded46fd5eafcb1fa1ef99dcbdd933ee8631ed7dc))
    - [clone] support for progress that can handle writing pack files ([`46e0055`](https://github.com/GitoxideLabs/gitoxide/commit/46e0055eab47e402807b15c63b6a4577f5c0b7bb))
    - [clone] leave aborting the negotiation loop in the hands of the delegate ([`ea83ce7`](https://github.com/GitoxideLabs/gitoxide/commit/ea83ce73b16b24409dec4009f09a0cbf203a89f7))
    - [clone] sideband-all support ([`ecc8e09`](https://github.com/GitoxideLabs/gitoxide/commit/ecc8e091fb97a5d44828cd56412358b7043e47ba))
    - [clone] Actually pass pack file to the delegate ([`94c5e62`](https://github.com/GitoxideLabs/gitoxide/commit/94c5e62b274b0fc39f64ee5b04273db5ead4a470))
    - [clone] Response parsing up to (optional) pack ([`24064c7`](https://github.com/GitoxideLabs/gitoxide/commit/24064c77f2969380fb92ea66109df86e84060324))
    - [clone] FAIL: try to model pack reading using ownership… ([`4ee14e3`](https://github.com/GitoxideLabs/gitoxide/commit/4ee14e322d904cafa297ad989a0d653e7f8e5d2f))
    - [clone] properly handle V2 response parsing ([`0d7d768`](https://github.com/GitoxideLabs/gitoxide/commit/0d7d768278234824e03c5e74dacaafca3ee65713))
    - Refactor ([`f2c31ec`](https://github.com/GitoxideLabs/gitoxide/commit/f2c31ec4f245ce4e42e1371c4c9095fc4124cf16))
    - Refactor ([`fab9f99`](https://github.com/GitoxideLabs/gitoxide/commit/fab9f99a1f73378747b07f2f27f69492da899cba))
    - [clone] Provide a handle to the packfile, if it is present in the response ([`fcb4cc1`](https://github.com/GitoxideLabs/gitoxide/commit/fcb4cc1b011edb2597686fcf24ad383819a52389))
    - [ref-ls] A way to abort on multiple delimiters; first tests work ([`8d44912`](https://github.com/GitoxideLabs/gitoxide/commit/8d44912e7215b85c6931b7b829bd73ac38584424))
    - Refactor ([`feec5be`](https://github.com/GitoxideLabs/gitoxide/commit/feec5be335a99a4c47ba98f93803863044575838))
    - [ref-ls] Allow multiple delimiters at the same time ([`cfae63a`](https://github.com/GitoxideLabs/gitoxide/commit/cfae63a5f7d2d99560dd857f7220980d70c4c4d8))
    - [ref-ls] basic V2 acknowledgement and packfile parsing, but… ([`549f404`](https://github.com/GitoxideLabs/gitoxide/commit/549f404378535390195dea4d6c5b6485db34b81e))
    - Thanks clippy ([`ac88eef`](https://github.com/GitoxideLabs/gitoxide/commit/ac88eefd56095995841f60f0cfdca78295006584))
    - [ref-ls] parse all V1 acknowledgements, without duplication ([`f7c1580`](https://github.com/GitoxideLabs/gitoxide/commit/f7c15809d74729b92e4c64a71543a4850765a8f8))
    - [ref-ls] first stab at V1 acknowledgement parsing ([`1d21cd4`](https://github.com/GitoxideLabs/gitoxide/commit/1d21cd4a59c28fe5c631a12a10f332f4cc8fd3f3))
    - [ref-ls] It would be practical to simply have access to the line provider… ([`5fba787`](https://github.com/GitoxideLabs/gitoxide/commit/5fba78796d3bcc16f812dc3202d521ee057e86f9))
    - Thanks clippy ([`27f30df`](https://github.com/GitoxideLabs/gitoxide/commit/27f30df9a8046fe4e872837e36dd497096660282))
    - [ref-ls] support for line peeking in packet line readers ([`0c0c575`](https://github.com/GitoxideLabs/gitoxide/commit/0c0c57522972f2a49ed5261474114da062e6ab15))
    - [ref-ls] Let's make Acks copy, because owned::Id is as well ([`1f9cc44`](https://github.com/GitoxideLabs/gitoxide/commit/1f9cc44275d226a7e80e24ed592f6d6bd98de31a))
    - Refactor ([`935d5fe`](https://github.com/GitoxideLabs/gitoxide/commit/935d5fea48b0d8710be822e2d64c77a7008143c4))
    - [ref-ls] first sketch of V1 tests for result parsing (ack + pack) ([`fd16a5f`](https://github.com/GitoxideLabs/gitoxide/commit/fd16a5f265764ae9f18b9b9fc0f713ccfaaf2944))
    - [ref-ls] tests for stateless V1/V2 ([`d34afc6`](https://github.com/GitoxideLabs/gitoxide/commit/d34afc6fcbcdc175c09b12d6697b01611dcd02ed))
    - [ref-ls] first step towards parsing negotiation result ([`51ecf7e`](https://github.com/GitoxideLabs/gitoxide/commit/51ecf7e248724cd0b499e7a8662df4511f24d6ee))
    - Refactor ([`61e9812`](https://github.com/GitoxideLabs/gitoxide/commit/61e98128ddd85cde1a352b70f83870fdea0c6bac))
    - Thanks clippy ([`6b1294a`](https://github.com/GitoxideLabs/gitoxide/commit/6b1294a7046af84a13e34c3c43f8ddd2b3b1cb97))
    - [ref-ls] Argument tests for fetches ([`50cd260`](https://github.com/GitoxideLabs/gitoxide/commit/50cd260866b7dbc44653d8c193e6517e770f44eb))
    - [ref-ls] first argument tests for clone ([`83490ef`](https://github.com/GitoxideLabs/gitoxide/commit/83490ef764c2625ac34e42c27de7364d5445cdd6))
    - [ref-ls] Also add 'haves' in V2; some more assertions ([`3e6bfb1`](https://github.com/GitoxideLabs/gitoxide/commit/3e6bfb1d144f9de4d45502b8257ea0f278d49376))
    - [ref-ls] Do feature assertions to not have to support old servers ([`9980ff9`](https://github.com/GitoxideLabs/gitoxide/commit/9980ff9e52b466a56418857ca15fbcdc0d17b6b8))
    - [ref-ls] don't do anything on drop ([`9f18d9b`](https://github.com/GitoxideLabs/gitoxide/commit/9f18d9b9062d61d6da6e2bb7564fe5edbb1528c4))
    - [ref-ls] A step towards getting the negotiation right, really need tests ([`abb56d8`](https://github.com/GitoxideLabs/gitoxide/commit/abb56d855d49f232d25e0326fdef13732605df5b))
    - [ref-ls] Transport layer knows whether it's stateful or not ([`22c3640`](https://github.com/GitoxideLabs/gitoxide/commit/22c3640b70bb6925d72794eeaeda48b0687f2047))
    - [ref-ls] Also re-send V1 features in each request, independently of statefulness for now ([`f8669d6`](https://github.com/GitoxideLabs/gitoxide/commit/f8669d60cb349b6217227eea0d76664e8da9a458))
    - [ref-ls] potentially fix 'is-done' logic ([`f9e338f`](https://github.com/GitoxideLabs/gitoxide/commit/f9e338f244806aa9f0e24352912091cb7d8e0e80))
    - [ref-ls] Sketch of sending arguments in V1 & V2 ([`e1d27b6`](https://github.com/GitoxideLabs/gitoxide/commit/e1d27b6693adca053bfb42d841c03ef16a256d88))
    - [ref-ls] first step towards supporting negotiation ([`27b6d2d`](https://github.com/GitoxideLabs/gitoxide/commit/27b6d2d24a92c1ffc1579a116a044cece50d9d20))
    - [ref-ls] probably all it takes to handle all capabilities of fetch arguments ([`d956ecc`](https://github.com/GitoxideLabs/gitoxide/commit/d956ecc7d66544157d9233c4803b27fdc3fee1c4))
    - [ref-ls] first sketch of argument utility to help creating wants/haves ([`b0b0166`](https://github.com/GitoxideLabs/gitoxide/commit/b0b0166c8dcc1094d7294ddf63e20c0ced2c85e7))
    - [ref-ls] fix feature validation in V2 ([`eb387d2`](https://github.com/GitoxideLabs/gitoxide/commit/eb387d24267d90e731b41897c7e4071131508ce2))
    - Update tasks ([`079fc02`](https://github.com/GitoxideLabs/gitoxide/commit/079fc02608432fb6c5539759813e336c3c9f6c58))
    - [ref-ls] Always send a flush before closing the connection ([`918f19f`](https://github.com/GitoxideLabs/gitoxide/commit/918f19f0c2dc202ed2014e30b7247e63a0f6a51e))
    - [ref-ls] Make credentials helper truly work ([`7f3c3a7`](https://github.com/GitoxideLabs/gitoxide/commit/7f3c3a71db7eeba1d37481ba1b522d5ded654237))
    - [ref-ls] And it even doesn't work if it is the very same transport ([`4ba50fe`](https://github.com/GitoxideLabs/gitoxide/commit/4ba50fe06f7423c31f4cd78079d51ef3ffd51920))
    - [clone] support automatic downgrade to protocol version 1 ([`4cf3643`](https://github.com/GitoxideLabs/gitoxide/commit/4cf36436f11eb95d420c1147a1ec8adb618ea5fb))
    - [clone] basic progress for fetch in protocol ([`1925d02`](https://github.com/GitoxideLabs/gitoxide/commit/1925d020b1ab922465f9555515f691b06aaba46a))
    - Refactor ([`aa7e8b1`](https://github.com/GitoxideLabs/gitoxide/commit/aa7e8b1eeccaa1182cfdc668592f61d8b28867d7))
    - Refactor ([`b97507e`](https://github.com/GitoxideLabs/gitoxide/commit/b97507ec5cd041d0433977d78006fc0d9a35e88e))
    - [clone] update README, improve delegate docs ([`dc7908f`](https://github.com/GitoxideLabs/gitoxide/commit/dc7908f1546239ade71f4147a389a001769311f5))
    - [clone] test ls-remote V2 ([`0907771`](https://github.com/GitoxideLabs/gitoxide/commit/09077710fb489b7a6dfa2bace4fda47609a97e78))
    - Thanks clippy ([`baf0b2c`](https://github.com/GitoxideLabs/gitoxide/commit/baf0b2c253b005e64762226dcf628b401b1684d4))
    - [clone] more tests for fetch features and arguments ([`a946861`](https://github.com/GitoxideLabs/gitoxide/commit/a9468614e1f40de4e7442b3915c6ce09d58f8c01))
    - [clone] features for V1 fetch ([`5b24a55`](https://github.com/GitoxideLabs/gitoxide/commit/5b24a559dfb03c99ee360e9997650c443fd30077))
    - [clone] assert on ref-prefix for ls-refs command ([`70347a5`](https://github.com/GitoxideLabs/gitoxide/commit/70347a5406e66a77c490010cd695ceffd80fb7e2))
    - Thanks clippy ([`d55cd56`](https://github.com/GitoxideLabs/gitoxide/commit/d55cd56c6721ab591157f9add4ba44507373398c))
    - Refactor ([`f02232d`](https://github.com/GitoxideLabs/gitoxide/commit/f02232d6698a217e7ff87164bda2869777c54e33))
    - [clone] Getting there with feature handling for ls-refs ([`27c5adc`](https://github.com/GitoxideLabs/gitoxide/commit/27c5adca6c428343492de69bdf2e4bd3ac9c89f3))
    - [clone] Remove intermediary mutable Capabilities implementation ([`f59344a`](https://github.com/GitoxideLabs/gitoxide/commit/f59344a6e39dac579624fa2a9db64cb10afcdb75))
    - Refactor ([`5ea42ba`](https://github.com/GitoxideLabs/gitoxide/commit/5ea42ba9eece2f7d9557456fe6adda5058d0ae1a))
    - [clone] first step towards better organizing features/capabilities/argument names ([`7d45f3a`](https://github.com/GitoxideLabs/gitoxide/commit/7d45f3abad100e8fe5691430ea3e3b95c7ae068a))
    - Dependency update ([`dea0028`](https://github.com/GitoxideLabs/gitoxide/commit/dea002855ef949a58851b1a3f853a59c57e4d164))
    - [clone] first sign of somethign working: ls-remote ([`df58fa1`](https://github.com/GitoxideLabs/gitoxide/commit/df58fa15bc01cb047115577da58fec867f118cf9))
    - Refactor; thanks clippy ([`03c3d17`](https://github.com/GitoxideLabs/gitoxide/commit/03c3d176fc4c534798df9a6faf80d0722dcf0b33))
    - Refactor ([`25122f2`](https://github.com/GitoxideLabs/gitoxide/commit/25122f2acc95c363ee573fa875d8573ad0ee7586))
    - [clone] V2 ref parsing ([`455fa0f`](https://github.com/GitoxideLabs/gitoxide/commit/455fa0f3a607cdbf24f0833e05a8a4e75ddca0c2))
    - [clone] A better way to set the agent in V2 invocations ([`325d3a2`](https://github.com/GitoxideLabs/gitoxide/commit/325d3a26e45c78aa953400229d131f2119f06f75))
    - [clone] Make the actual ls-refs call ([`898cb8b`](https://github.com/GitoxideLabs/gitoxide/commit/898cb8b0d672420536387926f8c6b26fba698b81))
    - [clone] sketch of delegating simple commands along with arg/feature verification ([`c2ebc48`](https://github.com/GitoxideLabs/gitoxide/commit/c2ebc4875587db0936648d59440e07cc941f9503))
    - Refactor ([`a6bcdc4`](https://github.com/GitoxideLabs/gitoxide/commit/a6bcdc42a82b63d544b6ca6fd32d123f5ea0f4ae))
    - Ignore keep-alive packages in case of 'sideband-all' ([`2e77b86`](https://github.com/GitoxideLabs/gitoxide/commit/2e77b862896c5070246184290c138a68cefbe313))
    - Refactor ([`ad0b2e9`](https://github.com/GitoxideLabs/gitoxide/commit/ad0b2e9df98ad8f5a687849af32cb4593be9ae53))
    - Thanks clippy ([`8b1ea29`](https://github.com/GitoxideLabs/gitoxide/commit/8b1ea290f8f132e5a3b11828acfe4859c3d19bc1))
    - [clone] apply another mild workaround to be able to use 'transport.close()' ([`ea636ae`](https://github.com/GitoxideLabs/gitoxide/commit/ea636aea6d4486edee79280c33770961a422e6bf))
    - [clone] remove workaround ([`55cf167`](https://github.com/GitoxideLabs/gitoxide/commit/55cf16744126137ee70b06513c2daba116645aa9))
    - [clone] more safety checks ([`6f5a9f3`](https://github.com/GitoxideLabs/gitoxide/commit/6f5a9f370542fd1d79a318e57fba65263f05028b))
    - Thanks clippy ([`423458e`](https://github.com/GitoxideLabs/gitoxide/commit/423458e8013b69a901a127c954281b8cb323fb26))
    - Refactor ([`f29ea65`](https://github.com/GitoxideLabs/gitoxide/commit/f29ea65de4693a6096d979531add42d1e0f3d04f))
    - [clone] proper parsing of V1 refs ([`d262307`](https://github.com/GitoxideLabs/gitoxide/commit/d26230727ef795a819852bc82d6c2e9956809d8c))
    - [clone] A little more ref V1 parsing ([`4bc7842`](https://github.com/GitoxideLabs/gitoxide/commit/4bc78425aba304b4e4967fb7599460366322ef41))
    - [clone] preparation of test for proper ref parsing (V1) ([`85cd580`](https://github.com/GitoxideLabs/gitoxide/commit/85cd5806299a2fd92e786e242f946fe9e29853c1))
    - Refactor ([`99247f4`](https://github.com/GitoxideLabs/gitoxide/commit/99247f46673ff6772796bf55662e920200ba0c38))
    - Refactor ([`c985370`](https://github.com/GitoxideLabs/gitoxide/commit/c9853702e4b63dc217e94a838de8c5ee5c877a4d))
    - [clone] symref parsing from capabilities ([`8c2ff64`](https://github.com/GitoxideLabs/gitoxide/commit/8c2ff640cce4f5f42a3424405efc15b18f4aa7f4))
    - [clone] A step closer to parsing symrefs correctly ([`250a340`](https://github.com/GitoxideLabs/gitoxide/commit/250a34045c26ae0f5c2e06b1943479887edfe412))
    - [clone] attempt to make refs more accessible… ([`fa1112c`](https://github.com/GitoxideLabs/gitoxide/commit/fa1112c69911b4cee8b2d768f907114b910832ac))
    - Refactor ([`c138059`](https://github.com/GitoxideLabs/gitoxide/commit/c138059434885536984996cd8fec002aba3d5fe1))
    - [clone] Prevent accidental leakage by transforming back to the 'right' type ([`2d469c6`](https://github.com/GitoxideLabs/gitoxide/commit/2d469c66ec47be2e1bc3e0b1f3d17dfea5050970))
    - Thanks clippy ([`9afa7f9`](https://github.com/GitoxideLabs/gitoxide/commit/9afa7f9c95635559426395f61f670dfcd6f6154d))
    - [clone] a better workaround for the 'drop scope' issue ([`3ccf32b`](https://github.com/GitoxideLabs/gitoxide/commit/3ccf32be15efea134bd72bbcc59c3f79252eeb3b))
    - [clone] First step of workarounding rusts drop rules ([`6b47923`](https://github.com/GitoxideLabs/gitoxide/commit/6b479239cd2a60ebfe7a4b11f9e2df0a8ea4a096))
    - [clone] update tracking ticket information ([`650c452`](https://github.com/GitoxideLabs/gitoxide/commit/650c4520ffc12b3c3861d406a7b8ffa2df5b5c04))
    - [clone] add Rustc issue to see if this is just my bad ([`ccb9b53`](https://github.com/GitoxideLabs/gitoxide/commit/ccb9b53bfecd0e6adcccfd6dc155e8c3033cf16e))
    - Thanks clippy ([`fd6f9e5`](https://github.com/GitoxideLabs/gitoxide/commit/fd6f9e5c9c2ac8f68ab885d9bbf2d5f7a77a732a))
    - [clone] Workaround for the drop-issue ([`43c6159`](https://github.com/GitoxideLabs/gitoxide/commit/43c61597b8907eba572eecf39b90bdca438ef7c3))
    - [clone] first attempt at adding authentication logic, but… ([`a36d14a`](https://github.com/GitoxideLabs/gitoxide/commit/a36d14a6b916f6aafc2c5757acda7c32415370c5))
    - [clone] first rough sketch of (mutable) capabailities in the protocol side ([`13f7ecb`](https://github.com/GitoxideLabs/gitoxide/commit/13f7ecbf493d4de633fd872f9b75292378449165))
    - Refactor ([`a567b24`](https://github.com/GitoxideLabs/gitoxide/commit/a567b24cb9e040d92c49364e6c4e45ff77895629))
    - Refactor ([`88ecda1`](https://github.com/GitoxideLabs/gitoxide/commit/88ecda11dc1d97a7460a449350945dcac2f13752))
    - [clone] frame for first 'fetch' tests ([`2da70f6`](https://github.com/GitoxideLabs/gitoxide/commit/2da70f688da95434e256ba1f355dbb809100604a))
    - Refactor ([`89aabde`](https://github.com/GitoxideLabs/gitoxide/commit/89aabde074b26a3d36579227912eec0b74ca5a91))
    - Refactor ([`51f6142`](https://github.com/GitoxideLabs/gitoxide/commit/51f6142913ce520329f9829976ee364e226a41a7))
    - [clone] support for git-credentials helper ([`a6546da`](https://github.com/GitoxideLabs/gitoxide/commit/a6546dab8d6d0dc4453052b77278cf5bb96aaade))
    - Refactor ([`cf0e45a`](https://github.com/GitoxideLabs/gitoxide/commit/cf0e45a7f129e91d377d15558378724ac0c1aca8))
    - [clone] decoding of credential message replies ([`1c2f56d`](https://github.com/GitoxideLabs/gitoxide/commit/1c2f56d0fd10d3592d0a6de298360b136b34467a))
    - [clone] encode message for git credentials helper ([`143549e`](https://github.com/GitoxideLabs/gitoxide/commit/143549e0757d4fa7a8347aa1b8b4734e9b62bf04))
    - [clone] sketch for identity handling ([`b23f470`](https://github.com/GitoxideLabs/gitoxide/commit/b23f47029fba50c7bba23a6ebe135e129ee9392a))
    - [clone] put remaining remote progress parsing code into protocol ([`e03e0e5`](https://github.com/GitoxideLabs/gitoxide/commit/e03e0e58191c71220ea1f8b9207bab96b3f9b303))
    - Refactor - decouple protocol from packetline ([`dc98db2`](https://github.com/GitoxideLabs/gitoxide/commit/dc98db28b77cc6a0bff2248167942224e58cdd2e))
    - [clone] move packet-line code into own crate ([`879af67`](https://github.com/GitoxideLabs/gitoxide/commit/879af671fcde405d3d08ddbc07ea70d0bee23ef1))
    - [clone] move packet-lint into transport layer ([`c0dd831`](https://github.com/GitoxideLabs/gitoxide/commit/c0dd8315089243164d82c444499a459756a0337b))
    - [clone] link up lean plumbing command with gitoxide-core: pack-receive ([`5ea49c8`](https://github.com/GitoxideLabs/gitoxide/commit/5ea49c8aa0d449bed98ce0147ad222ff25c27c32))
    - [url] basic frame and first failing test ([`60aacf0`](https://github.com/GitoxideLabs/gitoxide/commit/60aacf0c279d277c4abf13e62697a51feeee26fd))
    - [protocol] properly implement remote progress reporting ([`a81954a`](https://github.com/GitoxideLabs/gitoxide/commit/a81954a6a37afacd51add6661a656b8fb663ca54))
    - Refactor ([`66e9cd1`](https://github.com/GitoxideLabs/gitoxide/commit/66e9cd1fa1d17cfaac1235b573ba0230230e549c))
    - Thanks clippy ([`7f6e290`](https://github.com/GitoxideLabs/gitoxide/commit/7f6e29033ae05285afad846157f9c44b8c8710a5))
    - [protocol] prepare passing most of remote progress on to prodash… ([`b8a34e5`](https://github.com/GitoxideLabs/gitoxide/commit/b8a34e5cf26c469ff69f29fd5d02c61605887929))
    - Refactor ([`df8ebdc`](https://github.com/GitoxideLabs/gitoxide/commit/df8ebdc443458fa95f9fc7fbb43ca2b6d874d972))
    - Refactor ([`2ea3288`](https://github.com/GitoxideLabs/gitoxide/commit/2ea3288e57ddd5204821fd6efee6cbb05231e311))
    - Refactor ([`2102cab`](https://github.com/GitoxideLabs/gitoxide/commit/2102cabc9860900e2b5d9391cdfde6e59ad4a119))
    - [protocol] remote::Progress can now parse the usual progress ([`b0e5601`](https://github.com/GitoxideLabs/gitoxide/commit/b0e5601ae2d96b96b267b36b68ff7426c75ee3a8))
    - [protocol] first steps towards parsing remote progress ([`c3d0e7a`](https://github.com/GitoxideLabs/gitoxide/commit/c3d0e7a490cfa4d114bf8c13b5b3803eb6187290))
    - [protocol] even starting to parse remote progress by hand is painful… ([`d68db3c`](https://github.com/GitoxideLabs/gitoxide/commit/d68db3ca8a187d6e9b7e341dae3058ea210197fd))
    - Less ambiguous name for 'index-from-pack': 'pack-index-from-data' ([`386673c`](https://github.com/GitoxideLabs/gitoxide/commit/386673ccc99d18d023c7df3fcd40e86d71960b25))
    - [protocol] handle errors as well; transmit progress (first part) ([`c484398`](https://github.com/GitoxideLabs/gitoxide/commit/c48439818dbde32007a4ec350bc0599c5cbb0cf2))
    - [protocol] first successful test with pack reading ([`ad1e8bf`](https://github.com/GitoxideLabs/gitoxide/commit/ad1e8bf7668a935733b0ba6a0f1573de2250eced))
    - [protocol] first stab at decoding sidebands in Read ([`51fe596`](https://github.com/GitoxideLabs/gitoxide/commit/51fe5960a84e48e41544ee6d8523b7bb1e2c6a82))
    - [protocol] allow Reader delimiter to be configured ([`5a01596`](https://github.com/GitoxideLabs/gitoxide/commit/5a01596ba4c9fc50beaa99260ff2b263f64e99a0))
    - Refactor ([`78f27d8`](https://github.com/GitoxideLabs/gitoxide/commit/78f27d8bd0dada168bf2502937cc82ee9b6cfcfe))
    - Revert "[protocol] an alternative version with external buffer" ([`157d810`](https://github.com/GitoxideLabs/gitoxide/commit/157d810e50f3cc8dd12586ccd128be1d7c8a331a))
    - Revert "[protocol] But external buffers also don't help at all" ([`579a697`](https://github.com/GitoxideLabs/gitoxide/commit/579a697536ff7de9727f5a7e517b83a3feb75540))
    - [protocol] But external buffers also don't help at all ([`8e711df`](https://github.com/GitoxideLabs/gitoxide/commit/8e711df01b812aac9e4197a196582cad47ee6bbe))
    - [protocol] an alternative version with external buffer ([`a862d22`](https://github.com/GitoxideLabs/gitoxide/commit/a862d22aaadbd1f096400d4bcd06bc5c1ce17425))
    - [protocol] a struggle - putting buffers in Read adapters = bad idea ([`e257426`](https://github.com/GitoxideLabs/gitoxide/commit/e257426f3583b079120ed75e0bda2f035e70d94b))
    - [protocol] FAIL: keep referenced PacketLine for minimal copy ([`7e4d1f3`](https://github.com/GitoxideLabs/gitoxide/commit/7e4d1f304b6821118f38a6cdab599cc02e6e949c))
    - [protocol] sketch of Read impl for pack line iterator ([`fe3b050`](https://github.com/GitoxideLabs/gitoxide/commit/fe3b050ca7218aa7b4adf99e702534f5a6eaa70c))
    - Refactor ([`c81caa3`](https://github.com/GitoxideLabs/gitoxide/commit/c81caa3d178671c447846f346d08b60f59b313c4))
    - Revert "[protocol] FAIL: attempt to add an actual Iterator impl for packet lines" ([`2989781`](https://github.com/GitoxideLabs/gitoxide/commit/2989781250e85042a5e26632df4b3471abe8adee))
    - [protocol] FAIL: attempt to add an actual Iterator impl for packet lines ([`a6e4cb1`](https://github.com/GitoxideLabs/gitoxide/commit/a6e4cb13be7a3157d08fb899a7b9137a4f81c5b7))
    - Refactor ([`20b10c5`](https://github.com/GitoxideLabs/gitoxide/commit/20b10c5a52ed408a4d45e1f361dfa6faeb952850))
    - [protocol] thanks clippy ([`10b9017`](https://github.com/GitoxideLabs/gitoxide/commit/10b9017f1ced471a612713ab364e7c702078e756))
    - [protocol] tests for the reader ([`86d1a40`](https://github.com/GitoxideLabs/gitoxide/commit/86d1a40d735d88b4da4b654fa573e53c67c5f3c4))
    - [protocol] A chance for the reader to actually work ([`d6aebed`](https://github.com/GitoxideLabs/gitoxide/commit/d6aebed49320fc52dd1f11a42ec6dc54b2de8824))
    - Refactor ([`8ebdcbd`](https://github.com/GitoxideLabs/gitoxide/commit/8ebdcbd7e6ae9ecb874dabf689c8a4f7a2bc4f67))
    - [protocol] FAIL: finally the reader compiles with the 'slice split technique'… ([`58543cb`](https://github.com/GitoxideLabs/gitoxide/commit/58543cb13d88201f27ba015786d4916ee854ce67))
    - [protocol] FAIL3: giving up - it's quite impossible to do that without 'bytes' ([`047d67c`](https://github.com/GitoxideLabs/gitoxide/commit/047d67c9ed4d329718494076f1b741da16343906))
    - [protocol] reader FAIL: wherever the loop moves, it will not borrowcheck ([`cb154f2`](https://github.com/GitoxideLabs/gitoxide/commit/cb154f25d0ca6431ea3be278b573d80fa43fc66d))
    - [protocol] FAIL2: lifetime issues with loop ([`c2ff070`](https://github.com/GitoxideLabs/gitoxide/commit/c2ff0700a2ea7088cdfd1c66d140bc393b7a85ce))
    - [protocol] decode-band can fail on malformed input ([`0f468f9`](https://github.com/GitoxideLabs/gitoxide/commit/0f468f983efe082900689b900a10ae81ffab0157))
    - Refactor ([`ed1f364`](https://github.com/GitoxideLabs/gitoxide/commit/ed1f3649a89cdb224efa0ce62a63372fd973cc3b))
    - [protocol] better handling of text-lines ([`7ad1db0`](https://github.com/GitoxideLabs/gitoxide/commit/7ad1db0cc1efd486b4ce9ecfef6f6a763f8d6aac))
    - [protocol] attempt to implement a streaming pack line reader (FAIL :D) ([`cc45cec`](https://github.com/GitoxideLabs/gitoxide/commit/cc45cec34c43e93348fed7149c4ad5abd81dd775))
    - [protocol] add cargo-diet assertions ([`831b758`](https://github.com/GitoxideLabs/gitoxide/commit/831b7587828b819844341ff451baf54694e7641c))
    - Refactor ([`73e24c9`](https://github.com/GitoxideLabs/gitoxide/commit/73e24c9c1966206125bea0bfa627b50ef339ce11))
    - [protocol] side-band channel encoding and decoding ([`9b4fb3e`](https://github.com/GitoxideLabs/gitoxide/commit/9b4fb3eeecc7c383c7c9b9d890e7adf771ddc80a))
    - [protocol] suppot for V2 special lines ([`4e46719`](https://github.com/GitoxideLabs/gitoxide/commit/4e467194d19c2804b49f5f1c445f62a5d2dc7c44))
    - Encode and decode errors ([`3f4fd90`](https://github.com/GitoxideLabs/gitoxide/commit/3f4fd90333f80fc4a6b395dfb476d4ae0be921c7))
    - Decode ERR lines as actual errors ([`1f58568`](https://github.com/GitoxideLabs/gitoxide/commit/1f58568f670b8b3dfc996b6e7dbd2d5ef59f0f28))
    - More tests ([`c34d88b`](https://github.com/GitoxideLabs/gitoxide/commit/c34d88b18a23ee499b0df8e499bd772d41a9b8e1))
    - The first succeeding tests for streaming decoding :D ([`7ea25c5`](https://github.com/GitoxideLabs/gitoxide/commit/7ea25c5e94967d4480dd81bb2f3e4ad18a9d226e))
    - First stab at implementing streaming decoding of packet line… ([`843c6fb`](https://github.com/GitoxideLabs/gitoxide/commit/843c6fb51e001fe9384e0f1c2cde8ec906250ee5))
    - Cargo fmt ([`60cd21b`](https://github.com/GitoxideLabs/gitoxide/commit/60cd21b7a2df78dbf57efbb51ab6e7a507b4f187))
    - Allow dual-licensing with Apache 2.0 ([`ea353eb`](https://github.com/GitoxideLabs/gitoxide/commit/ea353eb02fd4f75508600cc5676107bc7e627f1e))
    - Refactor ([`7e3f67d`](https://github.com/GitoxideLabs/gitoxide/commit/7e3f67dbb8bd17cc2ee0888db08c716d7c81539a))
    - Packet line encoding with flush support ([`e924a59`](https://github.com/GitoxideLabs/gitoxide/commit/e924a595a4d9c9bd8647a72fd728f1bcb3f0db1a))
    - First bunch of tasks I see after studying parts of the protocol docs ([`9bd97ba`](https://github.com/GitoxideLabs/gitoxide/commit/9bd97bafd299efefd063dde73cef53fde9d36670))
</details>

## 0.26.2 (2023-01-10)

A maintenance release without user-facing changes.

## 0.26.1 (2023-01-09)

A maintenance release without user-facing changes.

## 0.26.0 (2022-12-30)

### New Features

 - <csr-id-4b8abb0e8a5326662f36b88f04681bdd0eaa8079/> display the underlying permission denied error as source when failing handshake due to authentication.

## 0.25.1 (2022-12-22)

A maintenance release without user-facing changes.

## 0.25.0 (2022-12-19)

### New Features

 - <csr-id-5a50d9599f820237866928df6d24a196a9b86fe3/> `fetch::Arguments::use_include_tag()` allows to signal `include-tag`.
   One can also check for its availability using `fetch::Arguments::can_use_include_tag()`.
   
   Further there was a bugfix that assumes V1 capabilities are correctly interpreted to support
   include-tag.
 - <csr-id-1c0a6c0edaca4063129c3b06638629b7af17a64c/> due to more specific errors one can differentiate if remote authentication failed repeatedly.
 - <csr-id-2363bcbd3207d1d16f2f95fbd20a7d7715b92240/> some errors support `is_spurious()` to tell if retrying is possible.

### Bug Fixes

 - <csr-id-a8727824a4c38b27d6ad7c73ccaf7a45839c0aa9/> don't pass 'include-tag' as argument by default.
   This flag always has to be added by the caller based on configuration.
 - <csr-id-24000fa7ad4607c23f6a4fb4d84d2c85d96ee01f/> don't post `packfile-uris` capability unless we are able to make use of it.
   Turns out that this option can be rejected by a server that advertises
   it, so definitely needs more work to make it work.
 - <csr-id-806b8c2ef392137f3a6ebd0f28da2a3a07a9f3eb/> Parse refs from bytes, not from String.
   The latter can cause issues around illformed UTF-8 which wouldn't
   bother git either.
   
   This comes at the expense of not parsing line by line anymore, but
   instead reading as fast as possible and parsing afterwards.
   
   Performance wise I think it doesn't matter, but it will cause
   more memory to be used. If this ever becomes a problem,
   for example during pushes where we are stuck with V1, we can consider
   implementing our own streaming approach that works with packet lines
   instead - they are just not exposed here even though they could.
 - <csr-id-5f2276b63129163096be3cb229864fc589348da8/> don't enforce V2 as protocol, but smoothly downgrade like git does.
   For backward compatibility the shared handshake implementation allows the
   transport to control which protocol versions it wants to support
   to allow optimizing for one special case, namely to prevent it to
   read all V1 refs on old servers but abort instead, closing the connection
   without delay.
   
   Now we leave this feature for custom transports (who usually come with custom
   servers) and instead support fallbacks to other protocols if the server
   demands it.

## 0.24.0 (2022-11-21)

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

## 0.23.0 (2022-11-17)

### New Features

 - <csr-id-68c65a9f8b3927d454164a6f72167b47c0facee5/> expose `futures_lite` for convenience in downstream crates.

### Changed (BREAKING)

 - <csr-id-09070a7c17f39383730c3a2b809eec677f79f386/> move `fetch::refs(…, protocol, …)` to the crate root as `ls_refs(…)`.
   This move also removes the `protocol` parameter as ls-refs is only available
   in V2 and the caller is forced to know that after running a
   `handshake(…)` anyway which yields the actual protocol used by the
   server.
   
   Note that the client can suggest the version to use when connecting,
   which is specific to the transport at hand.
 - <csr-id-35f7b4df164c130fb50fcffcf5de99816c2ca872/> move `fetch::Command` into the crate root.
   This represents much better on how it is actually used, which is for
   validation of any Command invoked through the protocol independently.
 - <csr-id-a03f8f6cce34618883e8448dd1c31b41c54d9448/> move `fetch::agent|indicate_end_of_interaction` to crate root
   These are generally useful in all interactions, including push.
 - <csr-id-a3bcf82ae50defa4439862943008647d03d09792/> `handshake(…)` is now generalized to support `git-receive-pack` as well.
   Note that `fetch::handshake()` is still present, selecting the correct
   service like before, but most of the `fetch::Ref` parsing is now handled
   by `handshake::Ref` in it's more general location.
   
   There is still `fetch::refs(…)` which just invokes a V2 ls-refs command
   (without that being generalized) as `git-receive-pack` just supports
   V1 at the moment, and making the `fetch::LsRefs` Command more general
   would mean that every command has to be general, at is doesn't matter
   anymore if it's used for Fetch or Push.
   
   As long as git won't support V2 for pushes, which it never might,
   there is no need to introduce this breakage even though it
   should probably be done before going 1.0.
 - <csr-id-d05b0b800a553e1e380801fb141e9aa054a6cbd0/> `fetch::agent()` returns agent string only; `fetch()` takes agent name as parameter.
 - <csr-id-aab278f2a3e07edb6d8109e7ffba003b5d5d7857/> don't auto-insert user agent, instead prepare to insert it later.
 - <csr-id-f957d9a1833af079b01ba6bd6941eb4af6c9e436/> `fetch::agent()` now returns the agent name, `fetch::agent_tuple()` returns specific key-value pair.
   This is in preparation for allowing user-defined agent strings.

## 0.22.0 (2022-11-06)

### Changed (BREAKING)

 - <csr-id-cd867ade55eaec138029c0cf17fa83e0679ef7fc/> `fetch::Ref::unpack()` returns `Option<oid>`.
   That way the caller has to be aware of the possibility of an unborn
   branch (probably the only unborn branch) on the remote.

### New Features (BREAKING)

 - <csr-id-02e37f04d7dfeab7067c8273bcd76408cb1f4852/> Support for `unborn` ls-refs capability if server supports it.
   We can also parse it, adding yet another variant to `fetch::Refs`.

## 0.21.0 (2022-10-10)

### Changed

 - <csr-id-3a38d1bc4910aab98c9c0b4a309be4a449db92fc/> turn `prepare_ls_refs` in `fetch::refs()` into `FnOnce`.

### New Features

 - <csr-id-aed93d22d7a7faf4d60f1f603830c882175cbcb6/> Allow defaulting `fetch::handshake::Outcome`.
   This is useful in conjunction with `std::mem::take()`.
 - <csr-id-0bcb2fd4d5d8692a0e91c70bcbcbff9dff60442f/> `fetch::Arguments::is_empty()` to help decide if arguments should be sent at all.
   Furthermore, the `Debug` trait was added.
 - <csr-id-31a7089f2583832727e2175ada6fb5c30c3beebe/> make some private methods public to give callers more flexibility.
   This allows to implement the fetch-negotiation part oneself and break
   free from constraints of the delegate.
 - <csr-id-744ed03cf6648e258affab62ef39ffad75c65397/> add `client::fetch_pack()` allowing to fetch a pack after a handshake.

### Bug Fixes

 - <csr-id-d7f62b441700c6d3526517c8c4f369cb9a72c102/> support keepalive packets.
   Keepalive packets are side-band only empty datalines that should
   just be ignored. This is now happening, allowing longer git
   operations to work as they will send keepalive packets every 5 seconds,
   and previously we would choke on it.
   
   Note that empty datalines are never send otherwise, making it a
   previously unused marker that can safely be skipped.
 - <csr-id-266395e18d7cf754f51355098c84fd1c969fc972/> Link up Io error as source for error chaining; add `Debug` to `client::fetch::Response`

### Reverted

 - <csr-id-f87b7eb4dc05b73f33ac554b57d50f3eee7f84c1/> `client::fetch_pack()` isn't worth it after all.
   The overhead required to make it work isn't worth the logic we are
   enabling. Instead, it's OK to re-implement custom logic on the consumer
   side based on this one.

## 0.20.0 (2022-09-20)

### Changed (BREAKING)

 - <csr-id-91f193f73ae37314319f7d055893b95431fd018e/> `fetch::Ref::unpack()` returns `&BStr` instead of `&BString` as ref name.
 - <csr-id-d2c772e28619f7602ab01285b25e79a7040c9aab/> `fetch::Ref::unpack()` now returns the peeled object as well.
 - <csr-id-bab586099a6e21e54932218f7a5c14f8bfc6cabd/> rename `fetch::Ref::path` to `fetch::Ref::full_ref_name`.
   Previously it was inaccurately named and described due to an
   insufficient understanding of the ref database.
 - <csr-id-99905bacace8aed42b16d43f0f04cae996cb971c/> upgrade `bstr` to `1.0.1`

## 0.19.1 (2022-08-28)

Maintenance release without user-facing changes.

### New Features

 - <csr-id-eaff67c14366f149ccca346acb46af12531a24e6/> `helper::main` to easily create credential helper implementations

## 0.19.0 (2022-08-24)

<csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/>

### Chore

- <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes

### New Features

 - <csr-id-5ba9e1d20a8d523571a153f9f4e3e1a5285335b5/> provide a function to gracefully shut down a fetch transport
 - <csr-id-b1c40b0364ef092cd52d03b34f491b254816b18d/> use docsrs feature in code to show what is feature-gated automatically on docs.rs
 - <csr-id-517677147f1c17304c62cf97a1dd09f232ebf5db/> pass --cfg docsrs when compiling for https://docs.rs
 - <csr-id-d6b4878d9832a0279e0dd19c1ca520a282289e69/> re-export maybe_async
   That way, API users can also provide dynamically sync or async APIs.

## 0.18.1 (2022-08-17)

A maintenance release without user facing changes.

### Changed (BREAKING)

 - <csr-id-e91c301342d44cff35ebe12fba4ec10afb4a1922/> replace `quick-error` with `this-error`
 - <csr-id-12589cc6f08e4d7aabae30bcdadaa0c2b4850229/> adapt to changes in `gix-url` and use `BString` to represent URLs.
   They can contain paths, which is why `String` can't represent a URL
   losslessly.
   
   For HTTP urls these are ultimately UTF-8 strings though.

### New Features

 - <csr-id-d2de51d65f9b4ab895e19cc1b307e42a9bb4bbd8/> add `fetch::refs()` method to invoke `ls-refs` for V2 and parse the result
 - <csr-id-35d17b4ea80be611607faa774d1ce0ee9c5000f2/> factor fetch handshake into `fetch::handshake()` function

## 0.18.0 (2022-07-22)

This is a maintenance release with no functional changes.

## 0.17.0 (2022-06-13)

A maintenance release without user-facing changes.

## 0.16.0 (2022-05-18)

### New Features (BREAKING)

 - <csr-id-32dc1829a5661f66396d109c8d0a8eaae6b1f532/> use `gix-credentials` in `gix-protocol`

## 0.15.0 (2022-04-03)

### New Features

 - <csr-id-0826cc92717faceff62df48a6e86fbb7ca6e90f5/> in-manifest and in-lib feature documentation

## 0.14.0 (2022-01-23)

A maintenance release with no relevant changes.

## 0.13.0 (2021-11-29)

A maintenance release, triggered by putting too many adjustments into a single commit.

## 0.12.1 (2021-11-16)

A maintenance release triggered by changes to gix-pack and changelog rewrites.

## v0.12.0 (2021-10-19)

A maintenance release to properly dealing with previously breaking changes in `gix-hash`.

## v0.11.0 (2021-10-15)

<csr-id-da68bfb8104ecf58e73e3f99d87f81c90712a2ca/>
<csr-id-c77bd7a01820110154f2c66cd954c1ccfff173c1/>

### Bug Fixes

 - <csr-id-c77bd7a01820110154f2c66cd954c1ccfff173c1/> '(null)' symref targets are turned into direct refs instead
   
   because that's what 'git' would do if it would care enough.
   On the client side this means one has to deal with detached heads
   and it's obvious that these are detached. It's not clear anymore
   why this is the case, and probably it's good to hide it as the
   current git behaviour is accidental and may change in future, and
   do so in a way we predict.
   
   There is the possibility that git would abort the entire
   fetch/upload-pack operation, but that's already handled correctly
   by our implementation as well as we understand ERR messages in packet
   lines.

## v0.10.4 (2021-09-10)

## v0.10.3 (2021-09-07)

## v0.10.2 (2021-08-29)

## v0.10.1 (2021-08-27)

- instruct docs.rs which features to use for more useful documentation

## v0.10.0 (2021-08-27)

- Various minor updates of pre-release dependencies

## v0.9.0 (2021-08-17)

### BREAKING

- Add fifth argument to `fetch(…)`

## v0.8.1 (2021-08-13)

## v0.8.0 (2021-08-10)

## v0.7.0 (2021-05-09)

## v0.6.0 (2021-04-08)

## v0.5.0 (2021-03-26)

## v0.4.1 (2021-01-05)

## v0.4.0 (2020-12-16)

## v0.3.0 (2020-12-15)

## v0.2.0 (2020-12-15)

## v0.1.1 (2020-09-14)

## v0.1.0 (2020-09-12)

## v0.0.0 (2020-08-13)

