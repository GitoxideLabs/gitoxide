---
name: reintegrate-git
description: Cross-check unprocessed commits from local Git master and git-for-windows main clones against Gitoxide, port relevant changes in a pristine prepared Gitoxide worktree, and record source-only outcomes in ets/plan/cgr.md with agent-authored local commits. Use for upstream Git porting sweeps or explicitly requested baselines; creating or discussing the skill does not start a sweep.
---

# Reintegrate Git

Find upstream behavior that Gitoxide still needs, implement it in the appropriate Rust crates, and leave a reviewable local commit series plus a durable source-commit classification log. Each entry records an evidenced decision, not proof that every upstream change requires a port.

## Prepared Repositories

- Use the current, user-prepared Gitoxide worktree as the destination. Read its applicable `AGENTS.md` and contributor instructions, record its branch or detached state and starting `HEAD`, and require a pristine index and worktree including untracked files, with no unfinished Git or Tix operation. Do not stash, reset, switch branches, or create a worktree to satisfy this precondition.
- Obtain the two source paths from the request or established session context; ask for missing paths. Use generic placeholders such as `/path/to/git` and `/path/to/git-for-windows` in reusable instructions, never personal checkout paths or identities.
- Verify the source identities from their remotes and contents: normally `git/git` checked out on `master` and `git-for-windows/git` checked out on `main`; honor explicitly supplied mirrors. Record source status and pin the full hashes of `refs/heads/master` and `refs/heads/main` before enumeration, reading committed content rather than uncommitted source edits. Keep both clones unchanged, and make sure repository-redirecting `GIT_*` environment variables cannot redirect commands elsewhere.
- Work from those local snapshots. Do not fetch, pull, push, open a PR, or communicate through an account as part of this skill. Report the snapshot tips rather than claiming they are the latest remote revisions.

## Inventory and Resume

Use `ets/plan/cgr.md` (continuous git re-integration) in Gitoxide as the sole committed processing ledger. For a sweep, create it if absent; an empty or absent ledger establishes no prior coverage.

If explicitly asked to create a baseline, first check whether `ets/plan/cgr.md` exists. If present, refuse baseline creation and leave the document unchanged, even if it is empty or contains no baselines. Only when absent, create the document without overwriting an existing file, record each verified source repository name, branch ref, and full pinned tip in a baseline table outside classification bullets, and create a local commit without starting a sweep. State that historical commits remain unexamined; baselines are user-selected revision bounds, not completed analysis.

Later sweeps honor saved baselines unless the user supplies other bounds: rebuild each baseline's reachable set from its recorded source clone and exclude the union of those full IDs from both inventories, preserving topological order. Explicit bounds change only the current sweep's scope; preserve the recorded baselines. Newly merged side branches remain candidates when not reachable from either baseline, regardless of commit dates; report missing baseline objects or history instead of substituting a boundary.

Unless the user gives revision bounds or the ledger contains saved baselines, enumerate **all commits reachable from both pinned tips**, including merged side branches and merges, oldest first in topological order. For each source, with `source_path` and `source_tip` set to its verified path and full tip hash:

```bash
git -C "$source_path" rev-list --reverse --topo-order "$source_tip"
```

An explicit range limits coverage to that range; do not invent a date, release, first-parent cutoff, or starting point from Gitoxide's age. Check for shallow or missing history and report incomplete coverage instead of silently treating a truncated clone as complete. Keep the full candidate inventory, source membership, pinned tips, evidence, and unresolved work in task-owned scratch files outside the worktree so work can resume after interruption.

- Resolve the **leading bold source hash** of each existing ledger bullet against both clones. Compare full object IDs, not subjects or arbitrary hashes found in the prose. Report ambiguous or unresolvable entries and do not infer coverage from them.
- Deduplicate identical full hashes shared by the clones; one source commit gets one candidate per sweep and one ledger row. Preserve each history's parent-before-child order when combining inventories.
- Different hashes remain separate candidates even when a patch ID, cherry-pick trailer, or subject suggests equivalence. These are investigation aids, not sufficient evidence to classify a commit; confirm the actual behavior and context. An equivalent change already covered still gets its own reasoned `ALREADY-COVERED` row.
- Within the revision bounds, subtract only `PORTED`, `ALREADY-COVERED`, and `NOT-APPLICABLE` entries. Reassess `POSTPONED` entries on later sweeps; they do not establish completed coverage. A pending candidate or an old scan's maximum date/hash does not cover unexamined commits, including side branches merged later.

## Cross-Check Every Candidate

Read the upstream message, full patch, relevant tests, and surrounding implementation to identify the trigger, intended behavior, and dependencies. Search Gitoxide's implementation, public callers, tests, documentation, and history for the corresponding behavior; matching subjects or absent path names do not establish coverage. Keep concrete code/test evidence for each decision in the scratch notes.

Use one explicit class for each evidenced source decision:

- `PORTED`: relevant behavior has been implemented and validated in Gitoxide.
- `ALREADY-COVERED`: existing implementation and tests cover the behavior, or a merge adds no behavior beyond separately analyzed commits.
- `NOT-APPLICABLE`: the Git feature or change has no meaningful counterpart in Gitoxide; explain why.
- `POSTPONED`: relevant behavior cannot be supported because Gitoxide lacks required infrastructure; name the missing prerequisite and a concrete condition for revisiting the port.

Ground each decision in the codebase. Missing support alone does not establish `NOT-APPLICABLE`; use `POSTPONED` when that support is a relevant prerequisite. Windows-specific changes can affect Gitoxide's cross-platform filesystem, process, path, or protocol behavior; do not classify them as `NOT-APPLICABLE` just because they come from git-for-windows. Upstream test-only, performance, and documentation changes can reveal a relevant gap too.

For a merge, examine its parents and merge-specific changes or resolutions, using per-parent diffs when a combined diff hides changes. A merge that only brings in separately analyzed commits can be `ALREADY-COVERED` for that reason without changing their classifications; do not blanket-ignore merges. Check later fixes, reverts, and dependencies when they affect the intended final behavior, and group a coherent dependent range when porting an intermediate change would be misleading or broken.

If relevance, correctness, or validation remains unresolved, keep the candidate pending in scratch notes and report the blocker. Use `POSTPONED` only for an established infrastructure prerequisite, not uncertainty or an unavailable local check. Do not classify an unfinished or unvalidated port as `PORTED`, or record unsupported exclusions; continue independent candidates when possible.

## Implement and Validate

Port relevant behavior into the existing Gitoxide layers and helpers, updating affected callers, exports, feature configurations, tests, and directly related documentation. Translate behavior and test ideas into project-native Rust; do not copy upstream code or fixtures under incompatible licenses.

- Run relevant existing tests before edits. Add or extend a regression test demonstrating the missing behavior, observe the failure before fixing it, and check the same case against Git where feasible. Use the repository's isolated fixture and Git invocation helpers, including read-only fixtures for read-only APIs.
- Implement the smallest complete change and run focused tests plus checks appropriate to the affected crates, platforms, and feature flags. Each resulting commit must build and pass its applicable checks independently; run the repository's local QA profile when required by its instructions or the change's scope.
- If a required check cannot run, report the precise limitation and keep the candidate pending rather than claiming a validated port. Avoid unrelated refactors and never edit generated `CHANGELOG.md` files.

## Source Log and Local Commits

Keep one physical line per classified source commit, with an explicit class and one possibly long sentence. Append new decisions; update the existing row when a `POSTPONED` decision changes or resolves, preserving the source identity instead of adding a duplicate. Use a short hash of at least 12 hexadecimal characters that uniquely identifies the source commit across **both clones**; lengthen it if necessary. A repository-local `--short` result alone does not establish uniqueness across both histories. If an existing prefix becomes ambiguous, lengthen it only after verifying its original identity.

Every row must name its class; the following hashes are illustrative, not entries to add:

```markdown
- **0123456789ab** PORTED: Preserve the original path bytes when reporting a failed checkout.
- **abcdef012345** NOT-APPLICABLE: Correct an interactive shell prompt because Gitoxide does not provide that prompt integration.
- **fedcba987654** ALREADY-COVERED: Reject malformed object headers because the existing Gitoxide parser and regression tests already enforce this behavior.
- **987654abcdef** POSTPONED: Validate reftable-specific behavior once Gitoxide has a reftable backend capable of exercising the case.
```

Describe the change in your own words and explain the classification. Keep only the source hash in each row: no produced Gitoxide hashes, hash ranges, nested bullets, paragraphs, personal identities, or scratch notes. When one Gitoxide commit covers several source commits, give each source commit its own row explaining its contribution or decision. Never add fictitious records, duplicate rows, or unsupported decisions; unresolved analysis stays in scratch notes.

Create **one self-contained local Gitoxide commit per ported source commit or coherent dependent range**, including the code, tests, related documentation, and those `PORTED` ledger rows atomically. Persist `ALREADY-COVERED`, `NOT-APPLICABLE`, and evidenced `POSTPONED` decisions in a ledger-only commit per coherent batch of source commits, rather than losing decisions or attaching unrelated classifications to a port. An unchanged postponed row needs no new row or commit. Stage only task-owned paths/hunks and inspect the entire index, `git diff --cached`, and `git diff --check` before committing; recheck for concurrent work before each mutation.

Use the executing agent's own author identity, never the user or upstream author. For Codex, use the machine identity `Codex <codex@openai.com>`, with command-local identity overrides. Write complete Markdown messages to task-owned files outside the checkout, following the repository's scoped conventional-commit rules. Explain the upstream motivation, behavior, adaptations, and validation, and cite **full source hashes and source repository names in the commit body** so the correspondence remains reviewable without putting Gitoxide hashes in the log.

For a Tix-managed destination, read and follow the repository's [tix skill](../tix/SKILL.md), including review-state preservation, and create new commits with its supported command:

```bash
tix new --index --author='Codex <codex@openai.com>' --file "$message_file"
```

For a destination without Tix-managed history, use ordinary Git commits:

```bash
git -c user.name=Codex -c user.email=codex@openai.com \
  commit --author='Codex <codex@openai.com>' --file "$message_file"
```

Verify each new commit's author, contents, ledger rows, and checks. Preserve pre-existing commits and review marks; do not publish the series. On interruption, retain scratch evidence and unfinished task-owned changes, and report their state without recording them as `PORTED`.

## Handoff

Check the complete series and ledger for duplicate source IDs, explicit classes with supporting reasons, unique abbreviations, and source-only hashes. Confirm that `POSTPONED` entries name prerequisites and revisit conditions and remain open. Run appropriate cumulative validation and verify the final worktree state.

Report both source paths and pinned tips, any requested bounds or missing history, Gitoxide's starting and final revisions, decisions by class, postponed prerequisites, unclassified pending candidates, checks and limitations, and the scratch directory for resuming unfinished work. Give concrete local review commands such as `git log --reverse <start>..HEAD` and `git diff <start>..HEAD`. State that the commits are agent-authored and local, and do not claim complete integration while postponed or pending candidates or source coverage remain unresolved.

Recommend that the user study each local porting commit and its source classifications, especially `NOT-APPLICABLE` decisions, and consider changing them to `POSTPONED` when additional infrastructure would make a meaningful port possible.
