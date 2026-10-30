#!/bin/sh
set -eu

# The view contains main and its base, but not root. Fetches use custom destinations.
git init -q -b main remote
cd remote
git commit -q --allow-empty -m root
git tag root
git commit -q --allow-empty -m base
git tag base
git commit -q --allow-empty -m visible
for branch in advanced forced pinned; do
  git branch "$branch"
done
cd ..
git init -q -b main local
cd local
git remote add origin ../remote
git config remote.origin.fetch '+refs/heads/*:refs/remotes/custom/*'
git config remote.origin.tagOpt --tags
git fetch -q origin
git reset -q --hard refs/remotes/custom/main
cd ../remote

# Existing refs advance or diverge; main stays unchanged. Two new refs share a tip.
git switch -q advanced
git commit -q --allow-empty -m advanced
git branch -f pinned
git branch new
git tag -a annotated -m annotated
git switch -q -C forced base
git commit -q --allow-empty -m forced
git branch same main

# This branch shares only hidden ancestry. A merge reaches the view by its second parent.
git switch -q -c unrelated root
git commit -q --allow-empty -m unrelated
git switch -q -c merged
git merge -q --no-ff --no-edit main

# Tags that do not peel to commits have no history to pin.
git tag blob "$(printf blob | git hash-object -w --stdin)"
