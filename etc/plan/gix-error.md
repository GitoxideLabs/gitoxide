# gix-error / `Exn` Migration Plan

Source issue: [GitoxideLabs/gitoxide#2351](https://github.com/GitoxideLabs/gitoxide/issues/2351)  
Imported on: 2026-04-22  
Working assumption: the checkboxes in this file reflect the current `gix-error` branch in this checkout, not only the historical state of the upstream issue.

## Mission

Finish the migration from `thiserror`-based error enums to `gix-error` / `Exn`, while preserving three caller-facing properties:

- typed validation failures stay identifiable as `gix_error::ValidationError`
- repository-open failures keep a distinct `NotARepository` path
- crate-local plumbing errors stay cheap and composable until they are intentionally erased at the `gix` boundary

## Constraints

- Use the current workspace as the source of truth for completion.
- Treat upstream PR history as context only. Several linked PRs are merged upstream, but that work is not fully reflected in this branch.
- Keep migration leaf-first so downstream breakage stays local.

## Reconciled Status

- [x] Proof of concept completed in [#2352](https://github.com/GitoxideLabs/gitoxide/pull/2352), merged on January 12, 2026.
- [x] `anyhow` / source-chain integration completed in [#2383](https://github.com/GitoxideLabs/gitoxide/pull/2383), merged on January 19, 2026.
- [ ] Make `cargo nextest run --workspace` complete without `--exclude gix-error`.
  Evidence: `.github/workflows/ci.yml` still excludes `gix-error`, in three places (`ci.yml:304,364,450`). The adjacent comment claims `gix-error` "is tested individually," but no dedicated job for it was found in any `.github/workflows/*.yml` file at this commit — worth confirming with Byron whether the exclusion is a migration artifact or a deliberate, permanent split.
- [ ] Replace `thiserror` with `gix-error` everywhere.
  Evidence: no longer the actual target. Only `gix` still depends on `thiserror` (42 `thiserror::Error` derives across 27 files); of the other crates, 14 adopt `gix-error`'s types in their own public error surface (`gix-date` most completely — its entire public error is `pub use gix_error::ValidationError as Error;`, `gix-date/src/lib.rs:36`), while the plumbing crates covered by the maintainer's revert kept hand-written concrete `Display`/`Error` impls with no `gix-error` dependency at all. (A raw `gix-error` dependency-key count across the workspace runs to 16; that also counts `gix` itself and `gix-error`'s own self-referential dev-dependency, neither of which is an adopter.) See "Migration Rules" for the three-tier strategy this reflects.
- [x] Keep `NotARepository` distinct from generic open failures.
  Evidence: `gix::open::Error::NotARepository` exists (`gix/src/open/mod.rs`) and is constructed in `gix/src/open/repository.rs`.
- [ ] Use `gix_error::Error` in tests when that simplifies `Exn`-heavy paths.
  Evidence: only 3 files workspace-wide use `gix_error::Error` under a `tests/` path. Not moot: `Exn` is not rare in `gix` — it appears 33 times across 7 files, concentrated in the revision-spec parsing delegate layer (see "Migration Rules" for the breakdown) — so real `Exn`-heavy production surface exists; simplifying test paths this way remains open.
- [x] Make validation failures identifiable as `gix_error::ValidationError` in crates that adopt `gix-error`.
  Evidence: `gix-error` exports `ValidationError`; adopted directly by `gix-actor`, `gix-bitmap`, `gix-chunk`, `gix-date`, `gix-mailmap`, `gix-pack`, `gix-quote` and `gix-revision`, plus used at the `gix` boundary via `or_raise`/`message`. Note `gix-validate` itself is a plumbing crate with hand-written concrete errors (no `gix-error` dependency) — its own failures propagate as their own concrete types (e.g. `gix_validate::reference::name::Error`, re-exported verbatim by `gix-ref`), not literally as `ValidationError`.

## Current Snapshot

Workspace scan basis:

- `thiserror` dependency present in `Cargo.toml`
- `thiserror::Error` mentions under `src/**/*.rs`
- 68 top-level workspace member crates (the 70 entries in root `Cargo.toml`'s `members`, minus the two nested harness crates `tests/tools` and `tests/it`)

Result on 2026-07-27, on the `gix-error-batch1` branch:

- 67 crates are done
- 1 crate is still pending: `gix` — 42 `thiserror::Error` derives across 27 files, every one carrying a `TODO(review)` note explaining why it stays concrete (see "Migration Rules")

Diff size, measured against this branch's merge-base (the "Merge pull request #2738 from GitoxideLabs/improvements" merge, 2026-07-22): the branch adds roughly 6,000 lines net — the line count went up, not down. Restricted to `gix/src` and `gitoxide-core/src`, the two directories the maintainer's "redeemed on the caller side" question is actually about, it is net −318. The growth is concentrated in the plumbing crates, where dropping `thiserror` meant hand-writing the `Display` and `Error` impls it used to generate: `gix-pack` +770, `gix-ref` +669, `gix-filter` +494, `gix-odb` +435, `gix-config` +412 and `gix-protocol` +406 net. The 42 `TODO(review)` notes add roughly 260 comment lines to `gix/src`, about 4% of the branch total — moving them into this file would take the caller-side figure to about −580 but would not materially change the branch as a whole.

## Linked Upstream PRs

- [x] [#2352](https://github.com/GitoxideLabs/gitoxide/pull/2352) `gix-error` punch-through
- [x] [#2373](https://github.com/GitoxideLabs/gitoxide/pull/2373) Convert more crates to `gix-error`
- [x] [#2378](https://github.com/GitoxideLabs/gitoxide/pull/2378) `gix-commitgraph` to `gix-error`
- [x] [#2383](https://github.com/GitoxideLabs/gitoxide/pull/2383) `anyhow` integration for `gix-error`
- [x] [#2389](https://github.com/GitoxideLabs/gitoxide/pull/2389) custom error implementation follow-up
- [x] [#2390](https://github.com/GitoxideLabs/gitoxide/pull/2390) make validate errors non-exhaustive
- [x] [#2396](https://github.com/GitoxideLabs/gitoxide/pull/2396) `gix-actor`
- [x] [#2400](https://github.com/GitoxideLabs/gitoxide/pull/2400) more `gix-error`
- [x] [#2423](https://github.com/GitoxideLabs/gitoxide/pull/2423) batch 1, part 1

## Migration Rules

**Decision, acted on 2026-07-22:** the maintainer overruled the original "erase to `Exn` everywhere" approach, in review feedback on [GitoxideLabs/gitoxide#2716](https://github.com/GitoxideLabs/gitoxide/pull/2716) — summarized: in the plumbing crates, keep the original expanded, hand-implemented error types for now instead of bringing in `gix-error::Exn`, and focus the erasure effort on `gix` itself and its usage of `gix::Error` with direct error forwarding.

Acting on that, `gix-fs`, `gix-attributes`, `gix-pathspec`, `gix-lock`, `gix-shallow`, `gix-prompt`, `gix-url` and `gix-path` had their `Exn` conversions reverted back to hand-written error types (the `revert!: keep the plumbing crates' error types concrete` commit, 2026-07-22). The migration is now three-tier:

- **Plumbing crates** — drop `thiserror`, keep concrete enums with hand-written `Display`/`Error` impls. No `gix-error` dependency at all.
- **`gix-error` adopters** — take a `gix-error` dependency directly and use its types (e.g. `gix_error::ValidationError`) in their own public error surface, short of the full erasure below. 14 crates do this (`gix-date` most completely — its entire public error is `pub use gix_error::ValidationError as Error;`, `gix-date/src/lib.rs:36`): `gix-actor`, `gix-archive`, `gix-bitmap`, `gix-blame`, `gix-chunk`, `gix-commitgraph`, `gix-date`, `gix-mailmap`, `gix-pack`, `gix-quote`, `gix-refspec`, `gix-revision`, `gix-revwalk` and `gix-worktree-stream`. (A raw dependency-key count of `gix-error` across the workspace runs to 16 — the other two are `gix-error` itself, which defines these types rather than adopting them, and `gix`, which anchors the boundary tier next.)
- **The `gix` boundary** — erase to `pub type Error = gix_error::Error;` where callers don't need to match variants. `Exn` itself is not rare in `gix` — it appears 33 times across 7 files (`lib.rs`, `repository/reference.rs`, `config/tree/keys.rs`, `revision/spec/parse/mod.rs`, and `revision/spec/parse/delegate/{mod,navigate,revision}.rs`), with the revision-spec delegate layer alone holding roughly 14 function signatures returning `Result<_, Exn>` — core parsing plumbing, not config/test downcasting.

Rules:

- In plumbing crates: remove `thiserror` from `Cargo.toml`; replace `#[derive(thiserror::Error)]` enums with hand-written `Display` + `std::error::Error` impls on the same concrete enum shape. Do not add a `gix-error` dependency.
- In `gix`: replace `#[derive(thiserror::Error)]` types with `pub type Error = gix_error::Error;`, and convert call sites with `.map_err(gix_error::Error::from_error)`, `.or_raise(...)` or `.ok_or_raise(...)` — unless the type is blocked (see below).
- Convert validation/parsing-only paths to `gix_error::ValidationError` where a crate does adopt `gix-error` (unaffected by the plumbing-crate reversal above).
- When migrating a crate, run its local checks and at least one downstream compile pass.

### Why a type stays concrete in `gix`

All 42 types still concrete in `gix` (2026-07-27) are covered by four buckets, each recorded in that type's own `TODO(review)` comment; most cite exactly one, but a type can be blocked more than one independent way at once:

1. **Callers match variants.** Code matches on variants or reads fields directly; erasing breaks the call site.
2. **E0119, spent slot.** A parent enum already embeds a different erased type via one `#[from]`; erasing this type too would give the parent a second `From<gix_error::Error>` impl. Order-dependent, not permanent — erasing the *hub* enum that's holding the slot deletes it and frees everything it was pinning, so a blocker here can evaporate later in the campaign.
3. **Generic.** The type carries a type parameter (e.g. a caller-supplied source error `E`) that a `pub type` alias can't carry.
4. **E0117, orphan rule.** A local `impl <ForeignTrait> for Error` (e.g. `gix_transport::IsSpuriousError`, re-exported through `gix_protocol::transport`) becomes foreign-trait-on-foreign-type once `Error` is an alias to the equally-foreign `gix_error::Error`.

Thirteen of the 42 cite more than one bucket, and the reasons stack rather than replace each other. Five cite three: `gix/src/remote/connect.rs:16`, `gix/src/remote/connection/ref_map.rs:12`, `gix/src/remote/connection/fetch/error.rs:3` and `gix/src/remote/connection/fetch/mod.rs:104` all open "kept concrete, blocked three independent ways" over buckets 4, 2 and 1; `gix/src/env.rs:54` (`env::collate::fetch::Error<E>`) makes the same three-way case in prose rather than a numbered list, over buckets 3, 4 and 1. Eight more cite two, each signaled by a `Separately, …` clause except one argued in two unconnected sentences: `gix/src/config/mod.rs:62` and `gix/src/config/mod.rs:268` are two different types in the same file, over buckets 1+2 and 3+4 respectively; `gix/src/open/mod.rs:43`, `gix/src/reference/errors.rs:95`, `gix/src/remote/errors.rs:10` and `gix/src/status/iter/mod.rs:240` each cite buckets 1+2, as do `gix/src/submodule/errors.rs:27` and `gix/src/submodule/errors.rs:74`, two different types in the same file. The remaining 29 cite exactly one.

### The double-wrap trap

`.map_err(gix_error::Error::from_error)?` (equally `.or_raise(...)` / `.ok_or_raise(...)`) applied to a callee that *already* returns `gix_error::Error` nests one erased error inside another. It compiles and passes the full test suite — the compiler can't see it structurally, and tests don't catch it either, since the nested error still renders and downcasts fine one level down. Eight such sites are recorded fixed on this branch, in the `fix: collapse double-wrapped erased errors and correct a stale note` commit (`status::iter`; three in `repository::merge`; `repository::blame`; `commit`; `pathspec`; `filter`) — three of them inside `gix/src/repository/merge.rs`, which a bare `cargo check -p gix` does not compile at all, since `merge` is not a default feature (verified: absent from `default`, `basic`, `extras` and `comfort`; only pulled in by the non-default `need-more-recent-msrv` bundle). The campaign-wide count may be higher; eight is what this branch evidences.

Detection method: for each `from_error` / `or_raise` / `ok_or_raise` call site, resolve the callee's return type through its alias chain and check whether it's already `gix_error::Error`. A plain grep won't find this — the call site reads identically whether the callee's error is concrete or already erased. Watch the feature-gating blind spot specifically: any module behind a non-default feature is invisible to a bare `cargo check -p gix` — but not to `cargo check --workspace`, since `gitoxide-core` depends on `gix` non-optionally with `features = ["merge", ...]` (`gitoxide-core/Cargo.toml:52`), so workspace-wide checks and tests compile it via feature unification.

## Execution Order

### Batch 1: leaves

- [x] `gix-hash`
- [x] `gix-url`
- [x] `gix-packetline`
- [x] `gix-features` (already converted when its zlib module moved to `gix-zlib`)
- [x] `gix-path`
- [x] `gix-attributes`
- [x] `gix-quote`
- [x] `gix-lock`
- [x] `gix-fs` (`thiserror` removed; kept concrete per the 2026-07-22 plumbing-crate decision — see "Migration Rules")
- [x] `gix-bitmap`
- [x] `gix-mailmap`
- [x] `gix-zlib` (not originally listed; extracted from `gix-features` after this plan was written)

### Batch 2: simple dependents

- [x] `gix-object`
- [x] `gix-config-value`
- [x] `gix-shallow`
- [x] `gix-refspec`

### Batch 3: ref / filter layer

- [x] `gix-ref`
- [x] `gix-filter`
- [x] `gix-revwalk`
- [x] `gix-pathspec`
- [x] `gix-prompt`

### Batch 4: config and discovery

- [x] `gix-traverse`
- [x] `gix-config` (`thiserror` removed; hand-written concrete errors, no `gix-error` dependency)
- [x] `gix-credentials`
- [x] `gix-discover`

### Batch 5: transport and index-adjacent

- [x] `gix-index`
- [x] `gix-transport`
- [x] `gix-worktree-stream`
- [x] `gix-submodule` (`thiserror` removed; hand-written concrete errors, no `gix-error` dependency)

### Batch 6: diff / protocol tier

- [x] `gix-diff`
- [x] `gix-protocol` (`thiserror` removed; hand-written concrete errors, no `gix-error` dependency)
- [x] `gix-dir`
- [x] `gix-worktree-state`
- [x] `gix-archive`

### Batch 7: heavier consumers

- [x] `gix-pack`
- [x] `gix-merge`
- [x] `gix-status`
- [x] `gix-blame`

### Batch 8: object database

- [x] `gix-odb`

### Batch 9: top-level API

- [ ] `gix` — 42 `thiserror::Error` derives across 27 files remain, all documented (`TODO(review)`) against the four blockers in "Migration Rules"

## Already Done Outside The Active Queue

- [x] `gitoxide-core` (no `thiserror`; carries a configuration-only `gix-error` dependency to pin feature resolution workspace-wide, but no actual error-handling usage — was never tracked anywhere in this file until now)
- [x] `gix-actor`
- [x] `gix-chunk`
- [x] `gix-command`
- [x] `gix-commitgraph`
- [x] `gix-date`
- [x] `gix-error`
- [x] `gix-fetchhead`
- [x] `gix-fsck`
- [x] `gix-glob`
- [x] `gix-hashtable`
- [x] `gix-ignore`
- [x] `gix-imara-diff` (no `thiserror`, no `gix-error` — was never tracked anywhere in this file until now)
- [x] `gix-lfs`
- [x] `gix-macros`
- [x] `gix-negotiate`
- [x] `gix-note`
- [x] `gix-rebase`
- [x] `gix-revision`
- [x] `gix-sec`
- [x] `gix-sequencer`
- [x] `gix-tempfile`
- [x] `gix-tix`
- [x] `gix-trace`
- [x] `gix-tui`
- [x] `gix-utils`
- [x] `gix-validate`
- [x] `gix-worktree`

## Immediate Next Moves

- [ ] Convert the remaining 42 `thiserror::Error` types in `gix` to `pub type Error = gix_error::Error;` wherever none of the four blockers apply — re-check after each hub-enum erasure, since freeing an E0119 slot can unblock types that looked permanently stuck.
- [ ] For each blocked type, decide case-by-case whether the blocker is worth engineering around (e.g. restructuring a hub enum to free its one `From<gix_error::Error>` slot) or should stay concrete for good; record the call in its `TODO(review)` note.
- [ ] After every new erasure or `.or_raise`/`.ok_or_raise`/`from_error` call, check the callee isn't already returning `gix_error::Error` — see "The double-wrap trap." Remember non-default features (`merge`, and worth auditing similarly) are invisible to a bare `cargo check -p gix`.
- [ ] Confirm with Byron whether `.github/workflows/ci.yml`'s `--exclude gix-error` is a migration artifact or a deliberate permanent split — the adjacent comment claims individual testing that no workflow file in this repo currently shows.
- [ ] Once `gix` no longer depends on `thiserror`, drop it from `gix/Cargo.toml` and close out "Exit Criteria."

## Exit Criteria

- [ ] No crate in this workspace depends on `thiserror`.
  Still open: `gix` does (`gix/Cargo.toml:400`).
- [ ] No `src/**/*.rs` file in this workspace mentions `thiserror::Error`.
  Still open: 42 derives across 27 files in `gix/src`.
- [ ] `cargo nextest run --workspace` no longer excludes `gix-error`.
  Still open: three `--exclude gix-error` invocations remain in `.github/workflows/ci.yml` as of 2026-07-27, on the `gix-error-batch1` branch.
- [x] The `gix` boundary still returns `gix_error::Error` where type erasure is desired.
  101 `pub type Error = gix_error::Error;` aliases in `gix/src` at this commit.
- [x] Validation-heavy crates still expose typed validation failures where callers need them.
  Holds under the three-tier strategy: plumbing crates (e.g. `gix-validate`) keep their own typed concrete errors; crates that do adopt `gix-error` use `gix_error::ValidationError` for validation-only paths.
