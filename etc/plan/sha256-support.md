# SHA256 / Object-Hash Transition Plan

Source issue: [GitoxideLabs/gitoxide#281](https://github.com/GitoxideLabs/gitoxide/issues/281)  
Imported on: 2026-04-22  
Last reconciled: 2026-05-08
Working assumption: checkboxes in this file reflect current checkout, not only historical issue state.

## Mission

Make object-hash kind first-class across config, protocol, storage, tests, and clone flow so SHA1 and SHA256 are both deliberate runtime choices instead of SHA1 being hidden fallback everywhere.

## Constraints

- Use current workspace as source of truth.
- Treat old issue checkmarks as historical context only.
- Prefer end-to-end correctness over isolated enum or parser support.
- Keep scope on actual supported transition path, not abandoned pack-index-v3 speculation.

## Reconciled Status

- [ ] Remove hash-type specific methods from `gix-hash` and lean on `gix_hash::Kind`-parametric usage.
  Evidence: `gix-hash` still contains `new_sha1`, `new_sha256`, `from_20_bytes`, `from_32_bytes`, `null_sha1`, `null_sha256`.
- [ ] Remove len-20 assumptions from all relevant code paths.
  Evidence: big progress exists, but 79 non-plan/non-changelog `Kind::Sha1.null()`/`ObjectId::null(...Sha1)` call sites still remain, plus a few explicit 20-byte comments and helpers.
- [x] Provide visible CLI path for choosing object hash kind.
  Evidence: `src/plumbing/options/mod.rs` and `src/plumbing/options/free.rs` expose clap `object_hash` fields, which produce `--object-hash`.
- [x] Propagate `sha256` feature support through crates that participate in object traversal, object parsing, and object-id storage.
  Evidence: 31 workspace packages now define a `sha256` feature, including `gix-object`, `gix-index`, `gix-protocol`, `gix-ref`, `gix-refspec`, `gix-traverse`, and top-level `gix`.
- [ ] Make SHA256-only builds compile where supported by feature flags.
  Evidence: `cargo check -p gix --no-default-features --features sha256` fails because `gix/src/config/cache/incubate.rs` and `gix/src/config/tree/sections/core.rs` still name `gix_hash::Kind::Sha1` unconditionally.
- [x] Remove default `sha1` feature from `gix-hash` and deal with fallout.
  Evidence: `gix-hash` has `default = []`, docs.rs explicitly enables `sha1`, root `gitoxide` chooses SHA1 via features, and `justfile` contains compile-guard checks for missing hash selection.
- [x] Remove SHA1 mention from `gix-features` feature toggles.
  Evidence: `gix-features/Cargo.toml` has no SHA1/SHA256 feature toggles anymore.
- [x] Parameterize hash length when decoding non-blob objects.
  Evidence: `gix-object` tree decoding hotspot uses `hash_kind.len_in_bytes()`.
- [x] Add `Sha256` enum variant and hasher support.
  Evidence: `gix_hash::Kind::Sha256`, `ObjectId::Sha256`, and `Hasher::Sha256` exist.
- [x] Add broad dual-hash test hooks for reading refs and objects of different lengths.
  Evidence: `justfile` runs SHA256 fixture suites for `gix-object`, `gix-ref-tests`, `gix-traverse-tests`, and top-level `gix`.
- [ ] Add write/read roundtrip coverage for different hash lengths, ideally with stronger repo-level verification.
  Evidence: targeted tests exist, but no obvious repo-conversion or full transition verifier is present.
- [ ] Handle remote object-hash mismatch during clone by configuring repository accordingly.
  Evidence: `gix/src/clone/fetch/mod.rs` still has `unimplemented!("configure repository to expect a different object hash as advertised by the server")`.

## Deferred / Out Of Scope

- [x] Pack index v3 transition work stays deferred unless Git actually relies on it.
  Rationale: issue itself already demoted this from active task to decision point.

## Current Snapshot

Workspace signals on 2026-05-08:

- `gix-hash` default hash feature: removed
- compile-guard checks for missing hash selection in `justfile`: 8
- crates/packages with explicit `sha256 = ...` feature declarations: 31
- `gix-traverse` hash feature declarations: `sha1` and `sha256`
- `Kind::Sha1.null()`/`ObjectId::null(...Sha1)` occurrences outside this plan and changelogs: 79
- `object-format=sha1` fixture occurrences outside this plan: 10
- `object-format=sha256` fixture occurrences outside this plan: 0
- clone path `unimplemented!()` for hash mismatch: 1
- visible object-hash CLI fields: present in repo and no-repo plumbing options
- `cargo check -p gix --no-default-features --features sha256`: fails on unconditional `Kind::Sha1` references in `gix`

Dual-hash test hooks already exist in `justfile` for at least:

- `gix`
- `gix-filter`
- `gix-diff`
- `gix-status-tests`
- `gix-commitgraph`
- `gix-object`
- `gix-ref-tests`
- `gix-pack`
- `gix-diff-tests`
- `gix-traverse-tests`
- `gix-blame`
- `gix-refspec`
- `gix-worktree-stream`
- `gix-hash`

## Confirmed Done

- [x] `gix-commitgraph`
  Evidence: issue marked it complete, dev-dependencies enable `gix-hash` with `sha1` and `sha256`, and `justfile` runs it with `GIX_TEST_FIXTURE_HASH=sha1` and `sha256`.
- [x] `gix-traverse` feature declaration
  Evidence: `gix-traverse/Cargo.toml` now defines both `sha1` and `sha256`.
- [x] `extensions.objectFormat=sha256` parsing
  Evidence: `gix/src/config/tree/sections/extensions.rs` accepts `sha256` behind the feature, and `gix/tests/gix/config/tree.rs` covers lowercase and uppercase SHA256.

## Remaining Hotspots

- `gix/src/config/cache/incubate.rs`
  Repository object hash still falls back to SHA1 when config does not say otherwise, and the unconditional `Kind::Sha1` references break SHA256-only `gix` builds.
- `gix/src/config/tree/sections/core.rs`
  `core.abbrev` validation still passes `Kind::Sha1` unconditionally for a context that may compile without SHA1.
- `gix-protocol/src/fetch/refmap/init.rs`
  `object-format` capability parsing still rejects anything except `sha1`.
- `gix/src/clone/fetch/mod.rs`
  Clone still aborts on remote hash mismatch instead of configuring repo state.
- `gix/Cargo.toml`
  Top-level `sha256` currently forwards to `gix-hash`, `gix-pack`, and optional `gix-worktree-stream`, but not to all direct dependencies that define their own hash features; confirm whether this is intentional feature unification or an under-forwarding gap.
- `gix-worktree-stream/Cargo.toml`
  Its `sha256` feature forwards only `gix-hash/sha256`. This may be acceptable because dependencies currently avoid SHA-specific cfgs, but it should be verified against feature-isolated builds.

## Execution Order

### Batch 1: hash API, features, and explicit selection

- [ ] `gix-hash`
  Remove remaining SHA1/SHA256-shaped helper APIs where `Kind`-based forms can replace them.
- [x] feature propagation
  Add and forward `sha256` features where crates already have hash-sensitive APIs or compile guards, including `gix-traverse`.
- [x] `gitoxide` CLI surface
  Keep the clap-derived `--object-hash` selection on plumbing commands.
- [ ] SHA256-only compile
  Replace unconditional `Kind::Sha1` fallbacks in `gix` config/cache paths with feature-aware defaults or required configuration.
- [ ] `gix-refspec`
  Keep object-hash-looking refspec parsing honest under SHA256-heavy inputs.

### Batch 2: config and object parsing

- [x] `gix`
  Teach `extensions.objectFormat` config parsing to accept `sha256`.
- [ ] `gix`
  Decide the correct default/fallback behavior when no `extensions.objectFormat` exists and SHA1 is not compiled in.
- [ ] `gix-object`
  Keep object parsers hash-length aware and extend tests around non-SHA1 trees and related decode paths.
- [ ] `gix-ref`
  Expand refs and reflog read/write coverage to both hash lengths.
- [ ] `gix-index`
  Extend checksum and extension tests to SHA256-sized object ids.
- [x] `gix-traverse`
  Add `sha256` feature support and a SHA256 fixture run in `justfile`.

### Batch 3: protocol and transport

- [ ] `gix-transport`
  Add negotiation fixtures that advertise `object-format=sha256`.
- [ ] `gix-protocol`
  Accept and preserve SHA256 object-format negotiation end to end.

### Batch 4: storage layer

- [ ] `gix-odb`
  Strengthen loose/packed lookup and prefix behavior under SHA256.
- [ ] `gix-pack`
  Finish pack data, index, multi-index, and verification assumptions that still lean on SHA1-shaped fixtures or sentinels.

### Batch 5: porcelain behavior and sentinel cleanup

- [ ] `gix`
  Replace clone hash-mismatch `unimplemented!()` with real repo initialization/configuration.
- [ ] `gix-diff`
  Remove SHA1-only sentinel assumptions where caller hash kind should drive impossible ids.
- [ ] `gix-blame`
  Same sentinel cleanup where SHA1 null ids are only placeholders.
- [ ] `gix-traverse`
  Replace remaining SHA1 defaults in traversal state with caller/repository hash kind where traversal starts from generic object ids.
- [ ] broad repo sweep
  Review remaining `Kind::Sha1.null()` occurrences one by one and separate acceptable sentinels from real SHA1 assumptions.

### Batch 6: acceptance coverage

- [ ] dual-hash refs/object read suite
- [ ] dual-hash write/read suite
- [ ] repo-level transition or conversion verification
- [ ] CI gating for SHA1 and SHA256 critical paths

## Immediate Next Moves

- [ ] Fix SHA256-only `gix` compilation by removing unconditional `Kind::Sha1` references from config/cache code paths.
- [ ] Make fetch negotiation accept `object-format=sha256`.
- [ ] Remove clone-time `unimplemented!()` for remote hash mismatch.
- [ ] Turn current scattered dual-hash tests into named acceptance criteria.

## Exit Criteria

- [x] SHA256 repository format parses through config without immediate rejection.
- [ ] SHA256-only top-level `gix` build compiles when requested by features.
- [ ] protocol negotiation can roundtrip `object-format=sha256`.
- [ ] clone/fetch can initialize repo state for non-SHA1 remotes without panicking or aborting.
- [ ] no important code path relies on implicit SHA1-only object-id shape.
- [ ] CI and local test entrypoints exercise both SHA1 and SHA256 where behavior differs.
