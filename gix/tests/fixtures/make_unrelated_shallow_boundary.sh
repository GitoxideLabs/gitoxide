#!/usr/bin/env bash
set -eu -o pipefail

# A malformed shallow commit outside the queried history must not be decoded.
# Write the commit-graph first so queries can also receive a pre-shallow cache.
git init -q
git checkout -q -b main
git commit -q --allow-empty -m parent
git commit -q --allow-empty -m child
git commit-graph write --no-progress --reachable
echo malformed | git hash-object --literally -t commit -w --stdin >.git/shallow

# Store both query directions: tip ID, base ID, ancestry status, ahead, behind.
for pair in 'HEAD HEAD^' 'HEAD^ HEAD'; do
  read -r tip base <<<"$pair"
  tip=$(git rev-parse "$tip")
  base=$(git rev-parse "$base")
  if git merge-base --is-ancestor "$base" "$tip"; then
    status=0
  else
    status=$?
    test "$status" = 1
  fi
  printf '%s %s %s ' "$tip" "$base" "$status" >>.git/history.baseline
  git rev-list --left-right --count "$tip...$base" >>.git/history.baseline
done
