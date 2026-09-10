#!/usr/bin/env bash
set -eu -o pipefail

# Leave a real bisect or conflicting rebase with detached HEAD in either the main
# or linked worktree. Its original branch must remain unavailable in both worktrees.
operation="$1"
worktree="$2"
mkdir main
(
  cd main
  git init -q
  git checkout -b main
  for value in base one two three; do
    echo "$value" >file
    git add file
    git commit -qm "$value"
  done
  git checkout -b onto HEAD~3
  echo onto >file
  git commit -qam onto
  git checkout main
  git branch available
  git worktree add -b topic ../linked
)

cd "$worktree"
case "$operation" in
  bisect)
    git bisect start HEAD HEAD~3
    ;;
  rebase-merge|rebase-apply)
    # Both backends stop on the first commit because 'onto' changed the same line.
    if git rebase "--${operation#rebase-}" --onto onto HEAD~3; then
      echo 'expected the rebase to stop with a conflict' >&2
      exit 1
    fi
    ;;
  rebase-interactive)
    if git -c sequence.editor=true rebase -i --onto onto HEAD~3; then
      echo 'expected the interactive rebase to stop with a conflict' >&2
      exit 1
    fi
    ;;
esac
