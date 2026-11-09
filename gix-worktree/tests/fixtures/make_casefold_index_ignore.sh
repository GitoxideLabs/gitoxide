#!/usr/bin/env bash
set -eu -o pipefail

# Missing skip-worktree ignore files exercise index fallback independently of
# the host filesystem. Git folds pattern matching but still looks up the exact
# index spelling of both ignore filenames and their parent directories.
git init -q repo
cd repo
mkdir UPPER ANOTHER
printf 'root-pattern\n' >.GITIGNORE
printf 'nested-pattern\n' >UPPER/.gitignore
printf 'other-pattern\n' >ANOTHER/.GITIGNORE
git add .GITIGNORE UPPER/.gitignore ANOTHER/.GITIGNORE
git update-index --skip-worktree .GITIGNORE UPPER/.gitignore ANOTHER/.GITIGNORE
rm .GITIGNORE UPPER/.gitignore ANOTHER/.GITIGNORE

for case in false true; do
  git config core.ignoreCase "$case"
  for path in root-pattern UPPER/nested-pattern UPPER/NESTED-PATTERN upper/nested-pattern ANOTHER/other-pattern another/other-pattern; do
    status=0
    git check-ignore -q -- "$path" || status=$?
    printf '%s %s\n' "$status" "$path" >>"../baseline-$case"
  done
done
