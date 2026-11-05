---
name: tix-qa-sweep
description: "Visit unchecked visible commits in a clean Tix-managed stack or tree, oldest first, repair failures with fast QA, and validate every resulting terminal tip cumulatively. Default to fast; an explicit thorough request adds full QA at each tip. Use for requested Tix QA sweeps, preserving reviewed patches through the shared Tix workflow."
---

# Tix QA Sweep

Repair every visible non-base commit whose tree does not already carry a checks-pass artifact (`✔️` by default, or its message's leading emoji), without squashing or reordering commits, then validate every resulting terminal tip. Use stable Tix change IDs because edits rewrite commit hashes.

Read [tix](../tix/SKILL.md) and follow its repair policy and history mutation commands, including amendment versus fixup, review preservation, authorship, signing, and checkout restoration. This policy also applies to formatting repairs.

## Choose Coverage

Default to fast without a fast/thorough questionnaire. Honor explicit user commands, exclusions, and constraints. Intermediate commits always use the smallest documented fast profile, even when the user requests thorough. Final tips use cumulative fast selection by default; an explicit thorough request requires the documented exhaustive/full profile at every final tip. A runner's conservative fallback may expand fast selection to full coverage.

Keep automatic per-test coverage selection enabled for local fast runs unless the user explicitly disables it. Gitoxide defaults to `--coverage auto`; do not add `--coverage off` to change the resulting artifact. Checks-pass records that the selected QA profile passed; its message describes the actual coverage, including coverage-based test omissions. Preserve the runner's fallbacks and exhaustive mode's complete test execution.

## Select QA Commands

Resolve the active repository with `git rev-parse --show-toplevel` from the user's working directory. Never infer it from this skill's installation directory or symlink target. Inspect that repository's available QA entrypoints, documentation, and supported options before selecting commands.

1. Use `just qa` only when the repository defines that recipe. Inspect its help and task definition for fast/full profiles and dependency-based selection. For Gitoxide's current runner:
   - Intermediate commits: `just qa --base HEAD^` (default fast mode), comparing each commit with its parent.
   - Final tips: `just qa --base <recorded-base>` (default fast), covering cumulative stack changes.
   - Explicitly thorough final tips: `just qa --mode exhaustive --base <recorded-base>`.
   Preserve the runner's normal fallbacks for global inputs, unknown paths, and unavailable comparison bases; do not replace them with a hand-picked crate list or narrower stages.
2. Without a `qa` recipe, inspect `etc/scripts/ci-check-local.sh` when present, then applicable `AGENTS.md` files, contributor docs, CI configuration, and project task definitions such as `justfile`, `Makefile`, or package scripts. Use the documented fast and full profiles and only supported options. Gitoxide's compatibility script accepts `--fast`/`--thorough` and forwards current QA options; older or unrelated scripts may not support `--base`.
3. With dependency-based selection, use the same parent/cumulative comparisons through the runner's supported interface. Without it, use the smallest documented fast profile for intermediate commits and default tips, and the documented full profile for explicitly thorough tips. Do not invent dependency filtering.
4. If no QA entrypoint is documented, use the usual checks supported by its manifests and configured tools. For an otherwise undocumented Cargo workspace, a reasonable fast profile is `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`; thorough adds `cargo test --workspace`. Other projects use their own configured formatters, linters, type checkers, build commands, and test runners. Report the limits of this inferred coverage.

Preserve toolchains, environment, feature flags, package managers, working directories, and cache policies. State the effective intermediate and final commands, comparison bases, and exclusions before traversal. If meaningful checks cannot be identified or a required tool is unavailable, report the gap without marking checks passed.

## Prepare

1. Run `git status --porcelain=v1 --branch`. Require a clean index and worktree, including no untracked files. Do not stash, discard, or absorb pre-existing work.
2. Require `tix` and only the tools needed by the selected QA commands.
3. Create a private directory with `mktemp -d "${TMPDIR:-/tmp}/tix-qa-sweep.XXXXXX"`.
4. Write `tix show` unchanged to `<temp-dir>/show.txt` and record the QA commands, working directories, environment, and exclusions in `<temp-dir>/qa-plan.md`. When using Gitoxide's script, copy it to `<temp-dir>/ci-check-local.sh` and run the copy from the repository root so the entrypoint survives travel. Inspect relative-path assumptions and required helpers before copying any runner: a copied launcher alone does not preserve a compiled runner or its source dependencies at older commits.
5. Record the starting checkout and its stable change ID. From `show.txt`, collect every visible commit row except base separators, including rows with checks-pass artifacts, ordered oldest first. Preserve topological order; for independent commits at the same depth, use their bottom-to-top display order. Stop if a displayed change-ID prefix is ambiguous or duplicated.
6. Record each terminal branch's stable tip change ID and its stack/fork comparison base as a resolved commit hash before edits. Derive these from the visible topology, base separators, and Git ancestry, honoring any user-specified base. A branched tree needs a tip-to-base map, not just the starting branch. Include shared visible ancestors in cumulative coverage; do not choose a later fork point that omits included stack changes. Preserve these bases throughout repairs rather than recomputing them from rewritten tips. Stop if the topology or intended base is ambiguous.

## Sweep

Process all recorded change IDs once in oldest-first order. Skip intermediate travel and QA when the latest `tix show` records checks passing for a change's current tree and no pending replay or ancestor content edit can change it. One snapshot can cover consecutive unchanged, checked commits; refresh it after history mutations. If replay could change a tree, or its status is uncertain, travel and inspect the result before deciding to skip. Final coverage is a separate required phase. If an older revision lacks a selected task or entrypoint, inspect that revision for equivalent checks and report any scope change; do not count an omitted check as passed.

Reuse build caches and honor repository- or user-specific storage limits. For Gitoxide only, expect a warm `target/` around 70 GB; inspect the selected runner for cache-size reporting and otherwise measure with `dua` when needed. Do not infer it from APFS free-space reporting or delete caches automatically. If growth materially exceeds the applicable expectation, stop and report the measured size.

For each recorded change ID:

1. Apply the skip rule above before traveling. For changes that still need inspection or QA, run `tix travel <change-id>` unless already at that change, complete any travel conflict resolution below, and verify that `HEAD` is the intended change.
2. Run a fresh `tix show` and inspect the intended change's leading enrichment field. If its current tree has a checks-pass artifact (`✔️`, or a custom emoji such as QA's `⚡` or `🔬`), continue to the next change ID. After possible replay, use the refreshed checks-pass record rather than a pre-replay marker or commit hash alone. Otherwise, record the comparison parent with `git rev-parse HEAD^` before any repair and run the fast QA commands in their recorded working directories. A root commit has no parent: use the runner's documented safe fallback and report it.
3. Once all selected checks pass at the actual current `HEAD`, verify that `HEAD` still names the tested commit and require a clean worktree. Preserve the runner's checks-pass artifact, including its custom gutter emoji, coverage and timing message. If an older runner deliberately withheld the artifact after coverage-based test omissions, record the successful commands and coverage in `qa-plan.md` without disabling coverage or inventing an unqualified artifact. Otherwise, if the runner did not record one, run `tix enrich tree checks-pass "" --revision <tested-commit>` to set an empty artifact explicitly and require enrichment to succeed. The no-argument command toggles and can clear a passing result. Continue to the next recorded change ID without further edits.
4. If a configured formatter check fails, run its corresponding formatting command (for example, `cargo fmt --all`), inspect and stage the resulting formatting changes, and record them using the `tix` policy. Require a clean worktree and rerun fast QA at the resulting `HEAD`, retaining the recorded comparison parent. Formatting changes are always wanted and do not need the diagnostic repair loop.
5. For any other failure, use the repair loop below and rerun fast QA until it passes.

When a repair creates a fixup, run fast QA on its actual new `HEAD` before proceeding. Retain the original comparison parent: for Gitoxide use `just qa --base <recorded-parent>`, not the fixup's `HEAD^`, so selection includes both the original change and its correction. Apply this to subsequent fixups too. Mark only the passing tree; leave the reviewed source unchanged and report that it still fails in isolation but is covered by the passing fixup. Track new fixup change IDs and replace a terminal source in the tip map with its resulting fixup tip.

### Repair Loop

1. Reproduce the printed failing command without output suppression. Inspect the failure, relevant callers, tests, and nearby history. Distinguish a repository defect from a missing tool, unsupported host behavior, network failure, or flake.
2. Fix the repository defect with the smallest change that resolves the failure. Preserve the commit's intent and do not pull unrelated later changes backward.
3. Run the focused failing check while the worktree is dirty. Inspect the complete diff and stage only intended paths; do not absorb generated residue blindly.
4. Record the staged repair using the `tix` policy. If this creates a fixup, continue subsequent repairs at its new `HEAD`. If final diagnosis requires repairing an ancestor, return through Tix to the affected terminal tips and complete replay before recording their final success; QA at the repaired ancestor alone does not validate its descendants.
5. Require a clean worktree, then rerun the current phase's QA at the resulting `HEAD`, retaining its original comparison base: the recorded parent for intermediate QA, the recorded stack/fork base for final QA. Repeat the diagnose, fix, focused-check, commit, and profile-check cycle until it passes.

## Validate Final Tips

After intermediate traversal, refresh `tix show` and reconcile every recorded terminal branch with the resulting topology, including amendments, fixups, and pending descendant replay. Validate every actual terminal tip, not an obsolete source commit or just the starting checkout.

1. Travel to each resulting terminal change ID through Tix unless already there, resolve conflicts, and verify `HEAD`. Run cumulative fast QA against that branch's recorded stack/fork base, or exhaustive/full QA if explicitly requested. Never let an existing checks-pass artifact, including one from an earlier fast run, bypass this final run; it records an exact tree and its optional message does not substitute for the required cumulative coverage.
2. On failure, use the same formatting/repair policy and repair loop. Rerun the final profile against the unchanged recorded base at the actual repaired `HEAD`, including any new fixup. Intermediate fast success cannot substitute for explicitly thorough final QA.
3. After success, verify the tested `HEAD` and require a clean worktree. Follow the same artifact policy as intermediate runs. Record the resulting tip change ID, commit/tree hashes, base, effective commands, coverage mode, omitted-test count, exclusions, and unavailable checks in `qa-plan.md`.
4. After final repairs, reconcile all terminal branches and use Tix travel where needed to complete pending replay. Verify that each actual tip tree still matches its final validation record; do not travel again when it is already clear that the tip cannot have changed. Rerun final QA for changed or newly resulting tips, retaining their recorded bases, until every terminal branch has current final coverage. A changed base, command, or profile also invalidates that record.

## Handle Travel Conflicts

Follow the replay-conflict procedure in `tix`, preserving both commit intents. Require a clean worktree after completing the resolution and retry the originally requested change ID. Stop instead of guessing when resolution requires API, compatibility, or product judgment.

## Stop and Complete

- Stop on an unexpected Tix failure, unresolved environmental failure, repeatable flake, ambiguous change ID, or failure whose correct fix is unclear. Report the current change ID, command, output, and worktree state. Do not bypass failed operations with resets or substitute commits, switch with Git, or push.
- After every recorded change passes fast QA or is covered by a passing fixup and every resulting terminal branch has current final coverage, restore the starting checkout using the return procedure in `tix`. Verify the checkout and that `git status --porcelain=v1` is empty. If restoration changes a terminal tree through replay, validate that resulting tip again before completing.
- Report effective intermediate/final commands and comparison bases, tested or skipped change IDs, each terminal tip's cumulative or exhaustive coverage, amended changes, reviewed commits covered by fixups with both change IDs, resolved travel conflicts, exclusions, and unavailable or out-of-scope QA jobs. Do not count unavailable checks as passed. Remove the temporary directory only after successful completion.
