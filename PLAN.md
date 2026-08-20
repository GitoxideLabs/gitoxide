# Reconciled Plan: Reftable Port + Integration

## Branch Reality
As of 2026-03-18, branch `codex/reftable-port-sequence` does not match the original "one commit per step" execution plan.

- The branch contains one reftable-only squash commit: `94793bb6fb` from 2026-03-03.
- That commit sits on top of `e8bf096c07`, which was `main` on 2026-03-03.
- Current `origin/main` is `8e47e0f00b`, so `git diff origin/main..HEAD` mixes this branch's work with unrelated upstream changes.
- To inspect only this branch's payload, compare `HEAD^..HEAD`.

In other words, this branch currently implements the standalone `gix-reftable` port and tests, but it does not yet contain the planned `gix-ref`/`gix` backend integration work.

## Reconciled Scope
Implemented on this branch:
- workspace wiring for `gix-reftable`
- low-level reftable primitives
- record encoding/decoding
- block, blocksource, and single-table reader support
- merged iteration helpers
- writer support
- stack transactions, compaction, reload, and fsck support
- upstream-style `u-reftable-*` parity tests
- selected `t0610`/`t0613`/`t0614` behavior tests

Not implemented on this branch:
- backend-agnostic `gix-ref` store activation
- reftable-backed `gix-ref` adapter
- `gix` repository opening and runtime support for reftable refs
- cross-backend regression coverage for the integrated path
- user-facing documentation of landed support

## Planned Sequence With Current Status
1. **`workspace: add gix-reftable crate skeleton and wire it into Cargo workspace`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

2. **`gix-reftable: port basics/constants/error/varint primitives from git/reftable`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

3. **`gix-reftable: implement record model and encode/decode parity (ref/log/obj/index)`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

4. **`gix-reftable: implement block + blocksource + table reader`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

5. **`gix-reftable: implement merged table iterators, pq, and tree helpers`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

6. **`gix-reftable: implement writer with limits/index emission/write options`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

7. **`gix-reftable: implement stack transactions, auto-compaction, reload, and fsck`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

8. **`gix-reftable/tests: port upstream u-reftable-* unit suites with 1:1 case mapping`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

9. **`gix-reftable/tests: add selected t0610/t0613/t0614 behavior parity integration tests`**  
   Status: completed, but folded into squash commit `94793bb6fb`.

10. **`gix-ref: activate backend-agnostic store abstraction (files + reftable state)`**  
    Status: not implemented on this branch.

11. **`gix-ref: add reftable-backed store adapter and route find/iter/transaction operations`**  
    Status: not implemented on this branch.

12. **`gix: switch RefStore to backend-capable store and detect extensions.refStorage=reftable`**  
    Status: not implemented on this branch.

13. **`gix: make reference iteration/peeling/fetch update paths backend-agnostic`**  
    Status: not implemented on this branch.

14. **`tests: update reftable open/head expectations and add cross-backend regression coverage`**  
    Status: not implemented on this branch.

15. **`docs/status: document reftable support, sha256 boundary, and update crate-status`**  
    Status: not implemented on this branch.

## What Must Happen Next To Match The Original Plan
1. Recreate or rebase this branch on top of current `origin/main` instead of comparing it directly from the old 2026-03-03 base.
2. Decide whether steps 1 through 9 must be restored as nine reviewable commits or can remain as one squash commit with documented scope.
3. Implement steps 10 through 15 as follow-up commits.
4. Update the existing `gix` reftable-open test once end-to-end support is actually present.

## Validation Guidance
For the work already present here, the relevant validation is:
- `gix-reftable` unit and behavior parity suites
- targeted workspace build/test coverage for the new crate wiring

For the remaining planned work, validation should expand to:
- `gix-ref` targeted tests
- `gix` targeted repository/reference tests
- reftable fixture coverage in repository-open and reference workflows

## Commit Message Rule For Remaining Work
Every remaining commit should still include:
- **Why now**
- **What changed**
- **Why this order**
- **What it unlocks next**

## Assumptions
- Source parity target is Git's in-tree reftable C implementation and tests.
- `gix-reftable` supports SHA-1 and SHA-256 in isolation.
- End-to-end `gix` reftable support is still outstanding in this branch until steps 10 through 15 land.
