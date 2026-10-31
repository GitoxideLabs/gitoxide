#!/usr/bin/env bash
set -eu -o pipefail

git init -q

git checkout -q -b main
git commit -q --allow-empty -m c1

current_commit="$(git rev-parse HEAD)"

git tag t1
git tag dt1

# Avoid creating and deleting 150,000 loose refs, which can time out on Windows.
for ((level = 1; level <= 1000; level++)); do
  for ((refname = 1; refname <= 150; refname++)); do
    printf '%s refs/heads/%04d/%d\n' "$current_commit" "$level" "$refname"
  done
done | LC_ALL=C sort -k2 > .git/packed-refs

# Let Git add the remaining refs and its canonical packed-refs header.
git pack-refs --all
