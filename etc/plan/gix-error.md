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
- [ ] Make `cargo nextest --workflow` run without `--exclude gix-error`.
  Evidence: `.github/workflows/ci.yml` still excludes `gix-error`.
- [ ] Replace `thiserror` with `gix-error` everywhere.
  Evidence: 33 crates in this branch still carry a `thiserror` dependency and/or `thiserror::Error` usage.
- [x] Keep `NotARepository` distinct from generic open failures.
  Evidence: `gix::open::Error::NotARepository` exists and is asserted in tests.
- [ ] Use `gix_error::Error` in tests when that simplifies `Exn`-heavy paths.
  Evidence: partially adopted, but not clearly finished as a repo-wide sweep.
- [x] Make `gix-validate` failures identifiable as `gix_error::ValidationError`.
  Evidence: `gix-error` exports `ValidationError`, and downstream crates already use it directly.

## Current Snapshot

Workspace scan basis:

- `thiserror` dependency present in `Cargo.toml`
- `thiserror::Error` mentions under `src/**/*.rs`

Result on 2026-04-22:

- 32 crates are done
- 33 crates are still pending

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

- Replace `thiserror` in `Cargo.toml` with `gix-error`.
- Prefer `pub type Error = gix_error::Exn<gix_error::Message>;` unless the crate needs a more specific concrete error.
- Convert validation/parsing-only paths to `gix_error::ValidationError`.
- Replace `#[from]` / `#[source]` propagation with `.or_raise(...)` or `.ok_or_raise(...)`.
- Keep `gix_error::Error` as the erased boundary type, mainly at `gix` and in tests that benefit from downcasting or frame inspection.
- When migrating a crate, run its local checks and at least one downstream compile pass.

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
- [ ] `gix-fs` (still uses `thiserror`)
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
- [ ] `gix-config` (deferred: the conversion predates the lifetime-free config refactor; kept at `main` pending re-integration)
- [x] `gix-credentials`
- [x] `gix-discover`

### Batch 5: transport and index-adjacent

- [x] `gix-index`
- [x] `gix-transport`
- [x] `gix-worktree-stream`
- [ ] `gix-submodule` (deferred: entangled with the lifetime-free config refactor; kept at `main` pending re-integration)

### Batch 6: diff / protocol tier

- [x] `gix-diff`
- [ ] `gix-protocol` - 8
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

- [ ] `gix` - 138

## Already Done Outside The Active Queue

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

- [ ] Finish Batch 1 in this branch before assuming the upstream batch-1 PR history is present locally.
- [ ] Remove the `gix-error` special-case from `.github/workflows/ci.yml`.
- [ ] Re-scan counts after each crate or mini-batch instead of trusting the original issue numbers.
- [ ] Only move `gix` itself after all plumbing crates beneath it are clean.

## Exit Criteria

- [ ] No crate in this workspace depends on `thiserror`.
- [ ] No `src/**/*.rs` file in this workspace mentions `thiserror::Error`.
- [ ] `cargo nextest --workflow` no longer excludes `gix-error`.
- [ ] The `gix` boundary still returns `gix_error::Error` where type erasure is desired.
- [ ] Validation-heavy crates still expose typed validation failures where callers need them.
