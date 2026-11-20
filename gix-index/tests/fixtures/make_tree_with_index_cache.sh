#!/usr/bin/env bash
set -eu -o pipefail

# Keep old index metadata beside Git's index-only reset result. Old file mtimes
# make unchanged cache entries non-racy, independent of the host's precision.
git init -q
for name in same changed mode removed skipped; do
  echo original >"$name"
done
git add .
git commit -qm base
echo changed >changed
echo added >added
git rm removed
git add changed added
git update-index --chmod=+x mode
git commit -qm target
git tag target
git reset --hard HEAD~1
touch -t 200001010000 same changed mode removed skipped
git update-index --refresh
git update-index --assume-unchanged same
git update-index --skip-worktree changed skipped
cp .git/index previous.index
git read-tree --reset target
cp .git/index expected.index
git rev-parse 'target^{tree}' >head.tree
