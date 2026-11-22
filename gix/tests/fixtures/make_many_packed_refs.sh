#!/usr/bin/env bash
set -eu -o pipefail

git init -q -b main
git commit -q --allow-empty -m initial
commit_id=$(git rev-parse HEAD)

# Build a sorted packed-refs file directly to avoid creating 100,000 loose files.
# The packed-only prefix keeps directory traversal out of the allocation measurement.
{
  printf '# pack-refs with: sorted\n'
  for ((index = 0; index < 100000; index++)); do
    printf '%s refs/heads/packed/%05d\n' "$commit_id" "$index"
  done
} > .git/packed-refs
