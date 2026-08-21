---
name: tix-qa-sweep
description: "Visit every visible commit in a clean Tix-managed stack or tree, oldest first, and repair failures from a user-selected fast or thorough QA profile. Use when asked to validate, sweep, or fix every Tix commit so each one is independently QA-clean."
---

# Tix QA Sweep

Repair every visible non-base commit without squashing, reordering, or skipping commits. Use stable Tix change IDs because amendments rewrite commit hashes.

## Choose the QA Profile

Before running repository commands, determine which profile the user wants:

- **Fast** runs formatting checks, `cargo machete`, Clippy, and `cargo deny`.
- **Thorough** runs the complete local QA script, including the fast checks, tests, documentation tests, journey tests, worktree checks, check enrichment, and cleanup.

If the request explicitly says fast or thorough, infer the corresponding profile. Otherwise, use a questionnaire when available to ask the user to choose **Fast** or **Thorough**. If no questionnaire is available, ask for the choice in chat and do not start the sweep until the user answers. Do not infer a profile from a generic request to validate or sweep.

Use `--fast` or `--thorough`, respectively, for every copied-script invocation in the sweep.

## Prepare

1. Run `git status --porcelain=v1 --branch`. Require a clean index and worktree, including no untracked files. Do not stash, discard, or absorb pre-existing work.
2. Require `tix`, `cargo`, `just`, `cargo-machete`, and `cargo-deny` to be available. For a thorough sweep, also require `cargo-nextest`.
3. Create a private directory with `mktemp -d "${TMPDIR:-/tmp}/tix-qa-sweep.XXXXXX"`.
4. Write `tix show` unchanged to `<temp-dir>/show.txt` and copy `etc/scripts/ci-check-local.sh` to `<temp-dir>/ci-check-local.sh`. Run the copy from the repository root throughout the sweep because both repository files disappear when visiting commits older than their introduction.
5. Record the starting checkout and its stable change ID. From `show.txt`, collect every visible commit row except base separators, ordered oldest first. Preserve topological order; for independent commits at the same depth, use their bottom-to-top display order. Stop if a displayed change-ID prefix is ambiguous or duplicated.

## Sweep

Visit the recorded change IDs once in oldest-first order. A fast sweep keeps Cargo's build cache between change IDs; a successful thorough check cleans before moving to the next change ID.

For each recorded change ID:

1. Run `tix travel <change-id>` directly and verify that `HEAD` is the intended change.
2. From the repository root, run `<temp-dir>/ci-check-local.sh <mode-flag>` with the selected `--fast` or `--thorough` flag.
3. When it passes, continue to the next change ID without amending.
4. If `cargo fmt --all -- --check` fails, run `cargo fmt --all`, stage and amend all resulting formatting changes with signing disabled, require a clean worktree, and rerun the selected profile. Formatting changes are always wanted and do not need the repair loop.
5. For any other failure, use the repair loop below and rerun the selected profile until it passes.

### Repair Loop

1. Reproduce the printed failing command without output suppression. Inspect the failure, relevant callers, tests, and nearby history. Distinguish a repository defect from a missing tool, unsupported host behavior, network failure, or flake.
2. Fix the repository defect with the smallest change that makes the current commit self-contained. Preserve the commit's intent and do not pull unrelated later changes backward.
3. Run the focused failing check while the worktree is dirty. Inspect the complete diff and stage only intended paths; do not absorb generated residue blindly.
4. Amend the staged fix with signing disabled:

   ```bash
   GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgSign GIT_CONFIG_VALUE_0=false tix amend --index
   ```

5. Require a clean worktree, then rerun the selected profile. Repeat the diagnose, fix, focused-check, amend, and profile-check cycle until it passes.

## Handle Travel Conflicts

Treat a failed `tix travel` as an expected replay conflict only when it says time travel would conflict and the worktree remains unchanged.

1. If the semantic resolution is clear, rerun `tix travel --materialize-conflicts <change-id>`.
2. Require an unmerged index, then inspect `git diff --cc`, index stages `:1:`, `:2:`, and `:3:`, nearby code, tests, and relevant history.
3. Resolve while preserving both the amended ancestor and replayed commit intents. Stage only the resolution and run the signing-disabled `tix amend --index` command above.
4. Require a clean worktree and retry `tix travel <change-id>`.

Stop instead of guessing when resolution requires API, compatibility, or product judgment.

## Stop and Complete

- Stop on an unexpected Tix failure, unresolved environmental failure, repeatable flake, ambiguous change ID, or failure whose correct fix is unclear. Report the current change ID, command, output, and worktree state. Do not reset, switch with Git, create substitute commits, or push.
- After all commits pass the selected profile, return with `tix travel <starting-change-id>`. Verify the original checkout is restored and `git status --porcelain=v1` is empty.
- Report the selected profile, tested change IDs, amended changes, resolved travel conflicts, and any QA jobs outside the selected profile's scope. Remove the temporary directory only after successful completion.
