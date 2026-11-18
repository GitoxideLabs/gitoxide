#!/usr/bin/env bash
set -eu -o pipefail

# Git applies core.bigFileThreshold to diffing, but still merges large text.
# Exercise both a clean merge and markers so a binary fallback cannot pass.
git init
git checkout -b main
seq 1 9 >file
git add file
git commit -m base
git checkout -b A
sed 's/^1$/ours/' file >new && mv new file
git commit -am ours
git checkout -b B main
sed 's/^9$/theirs/' file >new && mv new file
git commit -am theirs
git checkout -b C main
sed 's/^1$/conflict/' file >new && mv new file
git commit -am conflict
git config core.bigFileThreshold 1
git rev-parse 'main^{tree}' 'A^{tree}' 'B^{tree}' 'C^{tree}' >trees
for side in B C; do
  status=0
  git merge-tree -z --write-tree A "$side" >"$side.merge-info" || status=$?
  if test "$side" = B; then test "$status" -eq 0; else test "$status" -eq 1; fi
done
