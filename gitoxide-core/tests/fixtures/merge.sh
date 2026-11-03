#!/usr/bin/env bash
set -eu -o pipefail

# Merge tests rebuild this committed tree and index while updating HEAD.
git init -q
git config user.name "merge test"
git config user.email merge@example.com
printf 'tracked\n' >file
mkdir stable
printf 'unchanged\n' >stable/kept
# Keep the retained stat safely older than the index timestamp.
touch -t 200001010000 stable/kept
git add file stable
git commit -qm initial
git checkout -qb incoming
printf 'added\n' >a
printf 'changed\n' >file
git add a file
git commit -qm incoming
git checkout -q -
# Give Git a reusable sibling listing, including an untracked name.
printf 'untracked\n' >stable/visible
git update-index --untracked-cache
git status --porcelain >/dev/null
