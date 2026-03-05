#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name committer
git config user.email committer@example.com
git config commitGraph.changedPathsVersion 2

mkdir -p dir/subdir
echo one > dir/subdir/file
git add dir/subdir/file
git commit -q -m c1

echo two > dir/subdir/file
echo hello > other
git add dir/subdir/file other
git commit -q -m c2

git commit-graph write --no-progress --reachable --changed-paths
