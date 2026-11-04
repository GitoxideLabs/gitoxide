#!/usr/bin/env bash
set -eu -o pipefail

# QA can certify only a clean, unchanged commit. Keep each state read-only in tests.
git init -q clean
echo original >clean/tracked
git -C clean add tracked
git -C clean commit -qm original

for state in untracked unstaged staged moved; do
    cp -R clean "$state"
done
echo untracked >untracked/new
echo changed >unstaged/tracked
echo staged >staged/tracked
git -C staged add tracked

# A checkout change still invalidates the run when the new commit has the same tree.
git -C moved commit --allow-empty -qm moved
git init -q unborn
