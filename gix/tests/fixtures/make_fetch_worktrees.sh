#!/usr/bin/env bash
set -eu -o pipefail

# Checked-out branch rejection must find nested, locked and deleted linked worktrees.
git clone --shared "${1:?First argument is the complex base repo from make_remote_repos.sh/base}" worktree-root
(cd worktree-root
  git worktree add ../wt-a
  git worktree add ../prev/wt-a-nested
  git worktree add ../wt-b
  git worktree add ../wt-a/nested-wt-b
  git worktree add --lock ../wt-c-locked
  git worktree add ../wt-deleted && rm -Rf ../wt-deleted
)
