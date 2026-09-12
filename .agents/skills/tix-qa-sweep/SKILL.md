---
name: tix-qa-sweep
description: "Visit every visible commit without a checks-pass mark in a clean Tix-managed stack or tree, oldest first, and repair failures from a user-selected fast or thorough QA profile. Use when asked to validate, sweep, or fix every unchecked Tix commit with the repository's QA runner or usual project checks, preserving reviewed patches through the shared Tix workflow."
---

# Tix QA Sweep

Repair every visible non-base commit whose tree is not already marked `✔️`, without squashing or reordering commits. Use stable Tix change IDs because edits rewrite commit hashes.

Read [tix](../tix/SKILL.md) and follow its repair policy and history mutation commands, including amendment versus fixup, review preservation, authorship, signing, and checkout restoration. This policy also applies to formatting repairs.

## Choose the QA Profile

Before running repository commands, determine which profile the user wants:

- **Fast** runs the project's formatting, lint, type, and configured static or dependency checks.
- **Thorough** adds the project's standard build, tests, and any configured documentation or integration checks.

If the request explicitly says fast or thorough, infer the corresponding profile. Otherwise, use a questionnaire when available to ask the user to choose **Fast** or **Thorough**. If no questionnaire is available, ask for the choice in chat and do not start the sweep until the user answers. Do not infer a profile from a generic request to validate or sweep.

## Select QA Commands

Resolve the repository being swept with `git rev-parse --show-toplevel`. Select QA from that repository, even when this skill is installed globally through a symlink.

1. Prefer its `etc/scripts/ci-check-local.sh` when present, inspecting its usage before choosing arguments. Gitoxide's runner takes `--fast` or `--thorough`: fast runs formatting, `cargo machete`, Clippy, and `cargo deny`; thorough also runs tests, documentation and journey tests, worktree checks, enrichment, and cache monitoring.
2. Otherwise, use the repository's documented QA commands from applicable `AGENTS.md` files, contributor docs, CI configuration, and task definitions such as `justfile`, `Makefile`, or package scripts. Preserve its toolchain, feature flags, package manager, and required working directories.
3. If no QA entrypoint is documented, use the usual checks supported by its manifests and configured tools. For an otherwise undocumented Cargo workspace, a reasonable fast profile is `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`; thorough adds `cargo test --workspace`. Other projects use their own configured formatters, linters, type checkers, build commands, and test runners.

State the selected commands and any exclusions before traversal. If meaningful checks cannot be identified or a required tool is unavailable, report the gap without marking checks passed.

## Prepare

1. Run `git status --porcelain=v1 --branch`. Require a clean index and worktree, including no untracked files. Do not stash, discard, or absorb pre-existing work.
2. Require `tix` and only the tools needed by the selected QA commands.
3. Create a private directory with `mktemp -d "${TMPDIR:-/tmp}/tix-qa-sweep.XXXXXX"`.
4. Write `tix show` unchanged to `<temp-dir>/show.txt` and record the QA commands, working directories, environment, and exclusions in `<temp-dir>/qa-plan.md`. When using Gitoxide's runner, copy it to `<temp-dir>/ci-check-local.sh` and run the copy from the repository root so it survives travel to older commits. For other runners, check relative-path assumptions before copying; retain their intended working directory and required helpers.
5. Record the starting checkout and its stable change ID. From `show.txt`, collect every visible commit row except base separators, including rows marked `✔️`, ordered oldest first. Preserve topological order; for independent commits at the same depth, use their bottom-to-top display order. Stop if a displayed change-ID prefix is ambiguous or duplicated.

## Sweep

Visit all recorded change IDs once in oldest-first order. An ancestor repair can change a descendant's tree and invalidate its checks-pass mark, so decide whether to skip QA only after traveling to that descendant. If an older revision lacks a selected task or entrypoint, inspect that revision for equivalent checks and report any scope change; do not count an omitted check as passed.

Reuse build caches and honor repository- or user-specific storage limits. For Gitoxide only, expect a warm `target/` around 70 GB; its thorough runner reports the size with `dua`. Do not infer it from APFS free-space reporting or delete caches automatically. If growth materially exceeds the applicable expectation, stop and report the measured size.

When a repair creates a fixup, run the selected profile on that fixup before proceeding to the next recorded change ID. Mark only the passing tree; leave the reviewed source unchanged and report that it still fails in isolation but is covered by the passing fixup.

For each recorded change ID:

1. Run `tix travel <change-id>` directly, complete any travel conflict resolution below, and verify that `HEAD` is the intended change.
2. Run a fresh `tix show` and inspect the intended change's leading enrichment field. If its current tree is marked `✔️`, continue to the next change ID. Do not use the initial `show.txt` marker or a changed commit hash to make this decision. Otherwise, run the selected QA commands in their recorded working directories.
3. Once all selected checks pass at the current `HEAD`, require a clean worktree and run `tix enrich tree checks-pass`. This is idempotent if the runner already set the mark. Require enrichment to succeed, then continue to the next recorded change ID without further edits.
4. If a configured formatter check fails, run its corresponding formatting command (for example, `cargo fmt --all`), inspect and stage the resulting formatting changes, and record them using the `tix` policy. Require a clean worktree and rerun the selected profile at the resulting `HEAD`. Formatting changes are always wanted and do not need the diagnostic repair loop.
5. For any other failure, use the repair loop below and rerun the selected profile until it passes.

### Repair Loop

1. Reproduce the printed failing command without output suppression. Inspect the failure, relevant callers, tests, and nearby history. Distinguish a repository defect from a missing tool, unsupported host behavior, network failure, or flake.
2. Fix the repository defect with the smallest change that resolves the failure. Preserve the commit's intent and do not pull unrelated later changes backward.
3. Run the focused failing check while the worktree is dirty. Inspect the complete diff and stage only intended paths; do not absorb generated residue blindly.
4. Record the staged repair using the `tix` policy. If this creates a fixup, continue subsequent repairs at its new `HEAD`.
5. Require a clean worktree, then rerun the selected profile at the resulting `HEAD`. Repeat the diagnose, fix, focused-check, commit, and profile-check cycle until it passes.

## Handle Travel Conflicts

Follow the replay-conflict procedure in `tix`, preserving both commit intents. Require a clean worktree after completing the resolution and retry the originally requested change ID. Stop instead of guessing when resolution requires API, compatibility, or product judgment.

## Stop and Complete

- Stop on an unexpected Tix failure, unresolved environmental failure, repeatable flake, ambiguous change ID, or failure whose correct fix is unclear. Report the current change ID, command, output, and worktree state. Do not bypass failed operations with resets or substitute commits, switch with Git, or push.
- After every recorded change passes the selected profile or is covered by a passing fixup, restore the starting checkout using the return procedure in `tix`. Verify the checkout and that `git status --porcelain=v1` is empty.
- Report the selected profile, tested change IDs, amended changes, reviewed commits covered by fixups with both change IDs, resolved travel conflicts, and any QA jobs outside the selected profile's scope. Remove the temporary directory only after successful completion.
