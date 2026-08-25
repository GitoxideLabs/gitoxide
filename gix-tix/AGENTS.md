# gix-tix invariants

## Compatibility

- Tix is experimental. Implement requested command-line and behavioral changes
  without compatibility aliases or migration shims unless compatibility is
  explicitly requested.

## Behavioral specification

- Keep `spec.md` synchronized with every user-visible, lifecycle, performance,
  or resource-ownership change to tix. Update the specification and its
  regression coverage in the same semantic patch as the implementation.

## Command output

- Write warnings, diagnostics, and informational messages to stderr. Reserve
  stdout for a command's primary data output so it remains safe to redirect.

## Test repositories

- Open fixture repositories through `crate::test_repository::open()` or
  `open_with()`. They isolate configuration and provide deterministic author,
  committer, signing, editor, and date defaults; pass only behavior-specific
  overrides to `open_with()`, which take precedence over those defaults.

### Provisional Commits

- Before changing tracked worktree files for a task, start from a clean
  worktree and create an empty provisional commit authored as
  `🚧WIP🚧 <wip@invalid>`. Give it the purposeful subject the completed change
  is expected to use so an interrupted session remains identifiable; do not use
  a `WIP:` prefix.
- When the task is complete, stage the intended changes and amend that
  provisional commit. Replace the WIP author with the responsible agent's own
  name and email, and update the subject and descriptive body to match what was
  actually completed. The final commit must contain the complete task and leave
  the worktree clean; never leave the provisional author in finished history.

## Commit authorship

- Create one commit per semantically distinct change. Do not combine independent
  behavior changes merely because they were requested together.
- Write a descriptive commit body for every commit, not only a subject line.
  Explain the motivation and relevant prior behavior, the context needed to
  review the change, and any important constraints or decisions. The message
  should let a reviewer understand why the change exists without reconstructing
  the conversation that produced it.
- Keep each change's implementation, tests, snapshots, and corresponding
  `spec.md` updates in that change's commit.
- A commit created or materially rewritten by an AI agent must use that agent's
  own name and email as its author. Set it explicitly when necessary, for example
  with `tix reword --author "Agent Name <agent@example.com>"`; do not silently
  inherit the repository owner's configured identity.
- Never impersonate the repository owner, user, or another person when authoring
  an agent-created or materially agent-rewritten commit.
- The explicit exception is any tracked change outside `gix-tix/`: isolate such
  changes in a dedicated commit containing no `gix-tix/` files, author it as
  `Byron <sebastian.thiel@icloud.com>`, and immediately mark it for review with
  `tix enrich commit todo <commit>`. This makes the outstanding human review
  visible in tix even when the outside change is required by gix-tix work.
- Preserve the author of an existing commit when the agent is not responsible for its contents.
- Keep this provenance in commit metadata so reviewers can distinguish agent-authored changes without relying on commit-message trailers.
- If follow-up work semantically belongs to an existing local commit, amend it
  into that commit instead of adding a corrective commit at the tip. Use
  `tix travel <commit>` when suitable to edit it while retaining its descendants,
  then return through the saved branch after the descendants are rebased.

## Repository lifetime

- Treat the event-loop head as a repository-lifecycle boundary. After any wait,
  the original worktree directory and process CWD may no longer exist; recover
  to the normalized common repository before processing watchers, loading view
  data, or drawing.
- Do not retain a `gix::Repository`, or a platform/object that owns one, in application or event-loop state while tix is idle. Exceptions are an unresolved in-memory rebase result awaiting the user's immediate accept-or-cancel key, and line-diff worker repositories during their bounded ten-second reuse window. The event loop must wake at the line-diff deadline and join the entire pool; workers must rebuild diff platforms per batch, and dropping either exception must leave no observable repository state.
- Open a fresh, non-isolated repository for bounded view population so configuration such as mailmap and diff filters is honored, then retain only detached display data.
- The fill repository may be reused only while continuous navigation is active and must be dropped when its idle timer expires.
- Filesystem watchers retain paths and native watcher handles, never repositories.
