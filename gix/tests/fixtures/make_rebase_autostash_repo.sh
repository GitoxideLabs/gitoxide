#!/usr/bin/env bash
set -eu -o pipefail

# Both rebase backends must retain dirty work in an autostash during conflicts.
# An older stash makes loss of reflog entries visible when the rebase is quit.
backend="$1"
git init -q
printf 'base\n' >conflict
printf 'original\n' >local
git add conflict local
git commit -m base
git checkout -b other-branch
printf 'other\n' >conflict
git commit -am other
git checkout main
printf 'main\n' >conflict
git commit -am main
printf 'older stash\n' >local
git stash push -m older
printf 'autostashed contents\n' >local
git -c rebase.backend="$backend" rebase --autostash other-branch || :
test -f ".git/rebase-$backend/autostash"
