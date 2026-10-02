#!/bin/sh
set -eu

# `left` and `right` conflict on `shared`. The recorded diamond resolves that
# conflict and adds `merge-only`, which belongs to neither parent. `octopus`
# preserves this recorded resolution with an independent third parent. `common`
# lets replay update a shared ancestor once even though every parent inherits it.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git config commit.gpgSign false
export GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000'
export GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000'
printf 'base\n' >shared
printf 'initial\n' >common
git add shared common
git commit -qm base

git checkout -qb left
printf 'left\n' >shared
printf 'left contribution\n' >left
git add shared left
git commit -qm left

git checkout -qb right main
printf 'right\n' >shared
printf 'right contribution\n' >right
git add shared right
git commit -qm right

git checkout -qb extra main
printf 'extra contribution\n' >extra
git add extra
git commit -qm extra

git checkout -qb diamond left
if git merge -q --no-commit --no-ff right; then
  echo 'left and right must conflict before recording the resolution' >&2
  exit 1
fi
printf 'recorded resolution\n' >shared
printf 'only in the recorded merge\n' >merge-only
git add shared merge-only
git commit -qm 'Merge left and right'

# Git's octopus strategy refuses conflicts; write the explicitly resolved tree
# with all three ordered parents using Git's commit plumbing instead.
git checkout -qb octopus
git show extra:extra >extra
git add extra
merge_tree_id=$(git write-tree)
octopus_commit_id=$(git commit-tree "$merge_tree_id" -p left -p right -p extra -m 'Merge left, right and extra')
git reset -q --hard "$octopus_commit_id"
git checkout -q main
