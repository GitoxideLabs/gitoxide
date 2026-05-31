# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.0 (Unreleased)

The initial release.

### New Features

- `list(refs)` walks the `refs/stash` reflog and returns every stash entry
  newest-first. Returns an empty `Outcome` when `refs/stash` is unborn, matching
  `git stash list` output on a stash-free repo.

- `push(ctx, head_commit, head_tree, head_branch, options)` captures the current
  working tree as a new stash commit at `refs/stash`:
  - parent[0] is the commit `HEAD` points at, parent[1] is a commit whose tree
    matches the index at stash time, and parent[2] (optional, when
    `Options::include_untracked` is set) carries the untracked files.
  - The stash commit's own tree reflects the **working-tree** state for tracked
    files (not the index) so unstaged modifications are captured.
  - After the ref transaction the worktree is reset to `HEAD` via
    `gix_worktree_state::checkout`; untracked files captured into parent[2] are
    removed from disk.
  - Errors: `EmptyRepository`, `NoLocalChanges`, ODB write failures, ref
    transaction failures, worktree I/O.

- `pop(ctx, head_tree, options)` applies the latest stash to the working tree
  and drops the entry:
  - Performs a 3-way merge via `gix_merge::tree` with base = stash parent[0]
    tree, ours = current `head_tree`, theirs = stash WIP tree.
  - On a clean merge: writes the merged tree to the worktree via
    `gix_worktree_state::checkout`, restores parent[2] untracked files when
    present, then drops `refs/stash` (deletes the ref when the stack is
    exhausted, otherwise advances it to the next reflog entry).
  - On conflict: `Outcome::had_conflicts` is set, conflict markers are written
    to the worktree, and `refs/stash` is left untouched (matching
    `git stash pop` semantics).
  - Errors: `NoStash`, merge failures, ODB I/O, ref transaction failures.

### Known limitations

- The default `checkout_options::filters` and `checkout_options::attributes` are
  empty. Callers wiring this crate into porcelain (e.g. `gix` at the
  `Repository` level) must populate them so smudge/clean filters and
  gitattributes run during the worktree write.
- Tracked entries that have been **deleted from the worktree** are stored with
  their index OID rather than recorded as a deletion in the WIP tree. A pop of
  such a stash will not restore the deletion.
- The index produced by `pop` after a clean merge does not preserve stat data or
  timestamps from the merged tree.
- Operations that aren't `push` / `pop` / `list` (`apply`, `drop`, `show`,
  `branch`, autostash integration with rebase-like workflows) are deferred.
