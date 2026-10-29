#!/usr/bin/env bash
set -eu -o pipefail

# Tracking refs cover stale, excluded, symbolic, and overlapping fetch mappings.
# Tags fetched implicitly must survive pruning of branches.
git init -q server
git -C server commit --allow-empty -m initial
for branch in keep stale excluded; do
  git -C server branch "$branch"
done
git -C server tag stale-tag
git clone -q server client
git -C client config --add remote.origin.fetch '^refs/heads/excluded'
git -C client config --add remote.origin.fetch '+refs/heads/keep:refs/remotes/origin/shared'
git -C client config --add remote.origin.fetch '+refs/heads/gone:refs/remotes/origin/shared'
git -C client update-ref refs/remotes/origin/shared HEAD
git -C client symbolic-ref refs/remotes/origin/symbolic refs/remotes/origin/stale
git -C server branch -D stale excluded
git -C server tag -d stale-tag
