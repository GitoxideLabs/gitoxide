#!/usr/bin/env bash
set -eu -o pipefail

# HEAD inference must distinguish its symbolic target from branch aliases at
# the same commit, while detached HEAD can identify several candidate branches.
git init -q -b main source
(
    cd source
    printf 'content\n' >file
    git add file
    git commit -qm initial
    git branch duplicate
)
git clone -q --bare source symbolic
git clone -q --bare source ambiguous
git clone -q --bare source unique
for name in ambiguous unique; do
    commit_id=$(git -C "$name" rev-parse HEAD)
    git -C "$name" update-ref --no-deref HEAD "$commit_id"
done
git -C unique branch -D duplicate
git init -q --bare -b unborn unborn
git init -q --bare local
for name in symbolic ambiguous unique unborn; do
    git -C local remote add "$name" "$PWD/$name"
    git -C local remote show "$name" >"$name-remote-show"
done
