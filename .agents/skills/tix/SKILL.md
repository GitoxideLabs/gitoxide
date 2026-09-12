---
name: tix
description: "Edit Tix-managed commits and repair CI failures while preserving review state. Use for travel, amendments, Git-style fixups, new commits, and rewording in this repository."
---

# Tix

Use Tix for history mutations and Git for inspection and staging. User instructions control scope, signing, QA, and pushing; this skill grants no additional authorization.

## Inspect and Choose

Read `git status --porcelain=v1 --branch` and `tix show`. Record the starting branch, hash, and change ID. Refresh before mutations; other agents may be working here, so wait for their operations to settle when necessary.

Tix displays a Git hash followed by a change ID. Prefer the change ID for later travel because hashes change during edits and replay. Resolve ambiguous IDs instead of guessing. Check the installed `tix <command> --help` for supported arguments.

After an ancestor is edited, Tix may defer reapplying descendant commits' changes onto the edited history until `tix travel` reaches them. This is **pending replay**.

Determine the requested operation and target commit from the user's instructions and current history. For ancestor content edits, travel to the target and refresh `tix show` before choosing whether to amend or create a fixup: pending replay can temporarily hide review marks. At the intended `HEAD`, inspect there without unnecessary travel.

| Task | Default action |
| --- | --- |
| Add independent work | Create a new commit. |
| Change only a message | Reword the requested target directly. |
| Change a commit's contents without `✨` | Amend it directly; do not create a fixup, regardless of other enrichments. |
| Change a commit's contents with `✨` | Preserve it; insert a `fixup!` immediately above it. |

`✨` is patch review/refactoring approval within a Tix change (`tix enrich patch refackiewed`). `✔️` records checks passing for an exact tree. Neither implies the other. Do not fabricate marks after changes or treat a focused test as a complete QA profile. Explicit user instructions override the table; do not squash fixups unless requested.

## Edit Commands

For a bug or CI fix, locate the introducing commit from the failure, tested revision, diff, and history. CI can refer to an older PR head.

When changing checkout for content edits, travel from a clean index and worktree, confirm `HEAD`, edit, and run focused validation at that commit:

```bash
tix travel "$target_change"
```

At the correct `HEAD`, other agents' unstaged changes do not require travel or cleanup: stage only owned paths/hunks, including new files explicitly. Use `--index`: otherwise `amend` and `new` can fall back to tracked worktree changes when the index is unchanged. Untracked files are not implicitly included. `--index` consumes the entire index; coordinate if unrelated staged changes are present.

```bash
tix amend --index
```

For a fixup, generate its exact first line after traveling to the target:

```bash
git show -s --format='fixup! %s' HEAD > "$message_file"
```

Append a blank line and explain the failure, correction, and validation. Do not copy the subject from `tix show`, which renders Markdown and can omit literal backticks. There is no `tix new --fixup` flag.

For a fixup or independent commit, use a complete message file and the responsible agent's actual name/email in `agent_author`:

```bash
tix new --index --author "$agent_author" --file "$message_file"
```

After creating a fixup, record its new change ID as `fixup_change` and add a Tix note containing only the single-line title this fixup would have if it were a normal commit. Keep the explanation in the commit body and the exact `fixup! <original subject>` as the commit subject. Write only that title to `note_file` outside the checkout, then save it noninteractively through Git's editor:

```bash
TIX_FIXUP_NOTE_FILE="$note_file" GIT_EDITOR='cp "$TIX_FIXUP_NOTE_FILE"' \
  tix enrich commit note "$fixup_change"
```

The note command has no `--message` or `--file` flag. Require enrichment to succeed before continuing; if it fails, finish the note on the existing fixup instead of creating another commit.

For a message-only request, inspect the target's contents and supply its complete replacement message:

```bash
tix reword "$target_change" --file "$message_file"
```

Honor explicit targets even when `HEAD` moves. File-based messages preserve enrichments. Wording polish alone does not transfer authorship; use `--author` when responsibility for the contents changes. Follow scoped `AGENTS.md` rules: the provisional-commit rule under `gix-tix/` applies to edits there, not every Tix operation.

When signing is disabled, existing workflows use this command-local prefix, also applicable to `new` and `reword`:

```bash
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=commit.gpgSign GIT_CONFIG_VALUE_0=false tix amend --index
```

Preserve any existing configuration overrides instead of overwriting them or changing persistent Git configuration.

## Replay and Recovery

Edits can advance refs before descendant changes have been reapplied to the edited ancestry. After this task travels for an ancestor edit, use `tix travel "$return_branch"` to replay its ancestry and restore attachment. If initially detached, return through the saved change ID, not a stale hash. Honor subsequent user checkout changes: direct-target rewording does not require traveling back merely because an ancestor was rewritten. Leave pin management to Tix's normal commands.

If travel reports a replay conflict without changing files, use `tix travel --materialize-conflicts "$target_change"`. Its deliberate nonzero exit must correspond to an unmerged index. Inspect `git diff --cc` and stages `:1:`, `:2:`, `:3:`; preserve both commit intents. Stage the resolution, `tix amend --index`, then retry travel. Staging alone is incomplete.

Inspect state after other failures. Correct environmental problems before retrying, and never repeat an already successful mutation or substitute a reset for diagnosis.

Do not automatically stash or absorb another agent's work. Finish authorized task-owned changes before travel or retain verified patches/copies, including untracked files. `tix stash` and `travel --stash` are optional. After failed application, preserve the recovery commit and inspect what applied before restoring anything. Keep messages and recovery material outside the checkout: files can disappear at older commits.

## Validate and Finish

Keep commits self-contained. For CI fixes, run focused checks and distinguish cancellation fallout from actual failures. Use [tix-qa-sweep](../tix-qa-sweep/SKILL.md) for a requested sweep, retaining user exclusions, review-preservation rules, and QA level. Do not launch full QA automatically after each fix.

Reuse build caches; do not delete them, disable incremental compilation, or override authorized growth as incidental cleanup. Return to the requested checkout and verify status, placement, diff, and fresh `tix show`. Preserve untouched review marks without recreating invalidated ones. With concurrent work, verify only intended changes were committed rather than forcing everything clean.

Report final hashes/change IDs, validation, and push status, including when remote CI still tests an older head. Remove only task-owned scratch files after success; retain recovery material when interrupted.
