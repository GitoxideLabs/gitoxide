#!/usr/bin/env bash
set -eu -o pipefail

# Record the three stages of a content conflict, including an executable mode,
# then resolve it so only REUC retains those original entries.
git init -q
git config core.fileMode false
printf 'base\n' >file
git add file
git commit -qm base
git checkout -qb other
printf 'other\n' >file
git add --chmod=+x file
git commit -qm other
git checkout -q main
printf 'main\n' >file
git commit -qam main
if git merge other; then
    echo 'expected a conflict' >&2
    exit 1
fi
printf 'resolved\n' >file
git add file
git ls-files --resolve-undo >expected
