---
name: tix-ci-sweep
description: "Visit every visible commit in a clean Tix-managed stack or tree, oldest first, and repair each commit until the repository's local CI check passes. Use when asked to validate, sweep, or fix every Tix commit so each one is independently CI-clean."
---

# Tix CI Sweep

Repair every visible non-base commit without squashing, reordering, or skipping commits. Use stable Tix change IDs because amendments rewrite commit hashes.

## Prepare

1. Run `git status --porcelain=v1 --branch`. Require a clean index and worktree, including no untracked files. Do not stash, discard, or absorb pre-existing work.
2. Require `tix`, `cargo`, `just`, `cargo-nextest`, `cargo-machete`, and `cargo-deny` to be available.
3. Create a private directory with `mktemp -d "${TMPDIR:-/tmp}/tix-ci-sweep.XXXXXX"`.
4. Write `tix show` unchanged to `<temp-dir>/show.txt` and copy `etc/scripts/ci-check-local.sh` to `<temp-dir>/ci-check-local.sh`. Run the copy from the repository root throughout the sweep because both repository files disappear when visiting commits older than their introduction.
5. Record the starting checkout and its stable change ID. From `show.txt`, collect every visible commit row except base separators, ordered oldest first. Preserve topological order; for independent commits at the same depth, use their bottom-to-top display order. Stop if a displayed change-ID prefix is ambiguous or duplicated.

## Sweep

For each recorded change ID:

1. Run `tix travel <change-id>` directly and verify that `HEAD` is the intended change.
2. From the repository root, run `<temp-dir>/ci-check-local.sh`.
3. When it passes, continue to the next change ID without amending.
4. When it fails, reproduce the printed command without output suppression. Inspect the failure, relevant callers, tests, and nearby history. Distinguish a repository defect from a missing tool, unsupported host behavior, network failure, or flake.
5. Fix the repository defect with the smallest change that makes the current commit self-contained. Preserve the commit's intent and do not pull unrelated later changes backward.
6. Run the focused failing check while the worktree is dirty. Inspect the complete diff and stage only intended paths; do not absorb generated residue blindly.
7. Amend the staged fix with signing disabled:

   ```bash
   GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgSign GIT_CONFIG_VALUE_0=false tix amend --index
   ```

8. Require a clean worktree, then rerun the complete local CI script. Repeat the diagnose, fix, focused-check, amend, and full-check cycle until it passes.

## Handle Travel Conflicts

Treat a failed `tix travel` as an expected replay conflict only when it says time travel would conflict and the worktree remains unchanged.

1. If the semantic resolution is clear, rerun `tix travel --materialize-conflicts <change-id>`.
2. Require an unmerged index, then inspect `git diff --cc`, index stages `:1:`, `:2:`, and `:3:`, nearby code, tests, and relevant history.
3. Resolve while preserving both the amended ancestor and replayed commit intents. Stage only the resolution and run the signing-disabled `tix amend --index` command above.
4. Require a clean worktree and retry `tix travel <change-id>`.

Stop instead of guessing when resolution requires API, compatibility, or product judgment.

## Stop and Complete

- Stop on an unexpected Tix failure, unresolved environmental failure, repeatable flake, ambiguous change ID, or failure whose correct fix is unclear. Report the current change ID, command, output, and worktree state. Do not reset, switch with Git, create substitute commits, or push.
- After all commits pass, return with `tix travel <starting-change-id>`. Verify the original checkout is restored and `git status --porcelain=v1` is empty.
- Report the tested change IDs, amended changes, resolved travel conflicts, and any CI jobs outside the local script's scope. Remove the temporary directory only after successful completion.
