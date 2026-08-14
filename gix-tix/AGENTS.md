# gix-tix invariants

## Compatibility

- Tix is experimental. Implement requested command-line and behavioral changes
  without compatibility aliases or migration shims unless compatibility is
  explicitly requested.

## Behavioral specification

- Keep `spec.md` synchronized with every user-visible, lifecycle, performance,
  or resource-ownership change to tix. Update the specification and its
  regression coverage in the same semantic patch as the implementation.

## Commit authorship

- A commit created or materially rewritten by an AI agent must use that agent's own name and email as its author. Do not silently inherit the repository owner's configured identity.
- Preserve the author of an existing commit when the agent is not responsible for its contents.
- Use another author's identity only when the user explicitly requests it for that particular commit.
- Keep this provenance in commit metadata so reviewers can distinguish agent-authored changes without relying on commit-message trailers.

## Repository lifetime

- Treat the event-loop head as a repository-lifecycle boundary. After any wait,
  the original worktree directory and process CWD may no longer exist; recover
  to the normalized common repository before processing watchers, loading view
  data, or drawing.
- Do not retain a `gix::Repository`, or a platform/object that owns one, in application or event-loop state while tix is idle. The sole exception is an unresolved in-memory rebase result awaiting the user's immediate accept-or-cancel key; dropping it must leave no observable repository state.
- Open a fresh, non-isolated repository for bounded view population so configuration such as mailmap and diff filters is honored, then retain only detached display data.
- The fill repository may be reused only while continuous navigation is active and must be dropped when its idle timer expires.
- Filesystem watchers retain paths and native watcher handles, never repositories.

## Filesystem emphasis snapshots

- Transition tests use `insta` snapshots containing every distinct frame; unchanged hold frames are intentionally omitted.
- Run them with `cargo insta test -p gix-tix -F sha1`; review updates with `cargo insta review`. Never edit `.snap` files manually.
