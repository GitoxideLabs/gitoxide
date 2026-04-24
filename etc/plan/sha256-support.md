# SHA256 / Object-Hash Transition Plan

Source issue: [GitoxideLabs/gitoxide#281](https://github.com/GitoxideLabs/gitoxide/issues/281)  
Imported on: 2026-04-22  
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
  Evidence: big progress exists, but 73 `Kind::Sha1.null()` call sites still remain, plus a few explicit 20-byte comments and helpers.
- [ ] Provide visible CLI path for choosing object hash kind.
  Evidence: current tree has 0 `--object-hash` matches.
- [x] Remove default `sha1` feature from `gix-hash` and deal with fallout.
  Evidence: `gix-hash` has `default = []`, docs.rs explicitly enables `sha1`, root `gitoxide` chooses SHA1 via features, and `justfile` contains 31 compile-guard checks for missing hash selection.
- [x] Remove SHA1 mention from `gix-features` feature toggles.
  Evidence: `gix-features/Cargo.toml` has no SHA1/SHA256 feature toggles anymore.
- [x] Parameterize hash length when decoding non-blob objects.
  Evidence: `gix-object` tree decoding hotspot uses `hash_kind.len_in_bytes()`.
- [x] Add `Sha256` enum variant and hasher support.
  Evidence: `gix_hash::Kind::Sha256`, `ObjectId::Sha256`, and `Hasher::Sha256` exist.
- [ ] Add general tests for reading refs and objects of different lengths.
  Evidence: dual-hash coverage exists, but not yet as one clear acceptance layer.
- [ ] Add write/read roundtrip coverage for different hash lengths, ideally with stronger repo-level verification.
  Evidence: targeted tests exist, but no obvious repo-conversion or full transition verifier is present.
- [ ] Handle remote object-hash mismatch during clone by configuring repository accordingly.
  Evidence: `gix/src/clone/fetch/mod.rs` still has `unimplemented!("configure repository to expect a different object hash as advertised by the server")`.

## Deferred / Out Of Scope

- [x] Pack index v3 transition work stays deferred unless Git actually relies on it.
  Rationale: issue itself already demoted this from active task to decision point.

## Current Snapshot

Workspace signals on 2026-04-22:

- `gix-hash` default hash feature: removed
- compile-guard checks for missing hash selection in `justfile`: 31
- `Kind::Sha1.null()` occurrences: 73
- `object-format=sha1` fixture occurrences: 10
- clone path `unimplemented!()` for hash mismatch: 1
- `--object-hash` CLI flag matches: 0

Dual-hash test hooks already exist in `justfile` for at least:

- `gix-filter`
- `gix-commitgraph`
- `gix-object`
- `gix-pack`
- `gix-refspec`
- `gix-hash`

## Confirmed Done

- [x] `gix-commitgraph`
  Evidence: issue marked it complete, crate depends on `gix-hash` with `sha1` and `sha256`, and `justfile` runs it with `GIX_TEST_FIXTURE_HASH=sha1` and `sha256`.

## Remaining Hotspots

- `gix/src/config/tree/sections/extensions.rs`
  `extensions.objectFormat` still only accepts `sha1` and explicitly says SHA256 is not fully implemented.
- `gix-protocol/src/fetch/refmap/init.rs`
  `object-format` capability parsing still rejects anything except `sha1`.
- `gix/src/config/cache/incubate.rs`
  repository object hash still falls back to SHA1 when config does not say otherwise.
- `gix/src/clone/fetch/mod.rs`
  clone still aborts on remote hash mismatch instead of configuring repo state.

## Execution Order

### Batch 1: hash API and explicit selection

- [ ] `gix-hash`
  Remove remaining SHA1/SHA256-shaped helper APIs where `Kind`-based forms can replace them.
- [ ] `gitoxide` CLI surface
  Decide whether to restore a flag like `--object-hash` or bless config-only selection and document it clearly.
- [ ] `gix-refspec`
  Keep object-hash-looking refspec parsing honest under SHA256-heavy inputs.

### Batch 2: config and object parsing

- [ ] `gix`
  Teach `extensions.objectFormat` config parsing to accept `sha256`.
- [ ] `gix-object`
  Keep object parsers hash-length aware and extend tests around non-SHA1 trees and related decode paths.
- [ ] `gix-ref`
  Expand refs and reflog read/write coverage to both hash lengths.
- [ ] `gix-index`
  Extend checksum and extension tests to SHA256-sized object ids.

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
- [ ] broad repo sweep
  Review remaining `Kind::Sha1.null()` occurrences one by one and separate acceptable sentinels from real SHA1 assumptions.

### Batch 6: acceptance coverage

- [ ] dual-hash refs/object read suite
- [ ] dual-hash write/read suite
- [ ] repo-level transition or conversion verification
- [ ] CI gating for SHA1 and SHA256 critical paths

## Immediate Next Moves

- [ ] Decide whether `--object-hash` is still required as CLI UX, or whether config plus API is enough.
- [ ] Make `extensions.objectFormat=sha256` parse successfully.
- [ ] Make fetch negotiation accept `object-format=sha256`.
- [ ] Remove clone-time `unimplemented!()` for remote hash mismatch.
- [ ] Turn current scattered dual-hash tests into named acceptance criteria.

## Exit Criteria

- [ ] SHA256 repository format parses through config without immediate rejection.
- [ ] protocol negotiation can roundtrip `object-format=sha256`.
- [ ] clone/fetch can initialize repo state for non-SHA1 remotes without panicking or aborting.
- [ ] no important code path relies on implicit SHA1-only object-id shape.
- [ ] CI and local test entrypoints exercise both SHA1 and SHA256 where behavior differs.
