#!/usr/bin/env bash
set -eu -o pipefail

# A conflicting merge with an autostash and an older stash entry. Quitting must
# retain the conflict, index, and HEAD while appending the autostash to the stash.
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
git merge --autostash other-branch || :
test -f .git/MERGE_HEAD
test -f .git/MERGE_AUTOSTASH
