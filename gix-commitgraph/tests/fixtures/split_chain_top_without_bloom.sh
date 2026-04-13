#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name committer
git config user.email committer@example.com

echo one > tracked
git add tracked
git commit -q -m c1
git branch c1

echo two > tracked
git add tracked
git commit -q -m c2
git branch c2

# Keep this fixture distinct so cached outputs are regenerated with a Git that can emit v2 data.
git show-ref -s c1 | git -c commitGraph.changedPathsVersion=2 commit-graph write \
  --no-progress --changed-paths --split=no-merge --stdin-commits
git show-ref -s c2 | git commit-graph write \
  --no-progress --no-changed-paths --split=no-merge --stdin-commits
