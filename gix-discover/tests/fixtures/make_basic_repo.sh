#!/usr/bin/env bash
set -eu -o pipefail

git init -q

git checkout -b main
touch this
git add this
git commit -q -m c1
echo hello >> this
git commit -q -am c2

mkdir subdir
mkdir -p some/very/deeply/nested/subdir
# Starting discovery in this invalid `.git` directory must still check the repository above its parent.
mkdir -p non-repo/.git

git clone --bare --shared . bare.git

git init -q repo.git
(cd repo.git
  touch this
  git add this
  git commit -q -m "init non-bare repo with git suffix"
)

git clone --bare --shared . non-bare-without-worktree
(cd non-bare-without-worktree
  git config core.bare false
)

git clone --bare --shared . non-bare-without-worktree-with-index
(cd non-bare-without-worktree
  git config core.bare false
  cp ../.git/index .
)

git worktree add worktrees/a
git worktree add worktrees/b-private-dir-deleted
rm -R .git/worktrees/b-private-dir-deleted
git worktree add worktrees/c-worktree-deleted
rm -R worktrees/c-worktree-deleted

(cd bare.git
  git worktree add ../worktrees/from-bare/c
  git worktree add ../worktrees/from-bare/d-private-dir-deleted
  rm -R ./worktrees/d-private-dir-deleted
)

git clone --bare --shared . bare-no-config.git
(cd bare-no-config.git
  rm config
)

git init --bare bare-no-config-after-init.git
(cd bare-no-config-after-init.git
  rm config
)

git clone --shared . worktree-no-config
(cd worktree-no-config
  rm .git/config
)

git init worktree-no-config-after-init
(cd worktree-no-config-after-init
  rm .git/config
)

git init --bare bare-with-index.git
(cd bare-with-index.git
  touch index
)

git init --bare bare-with-index-bare
(cd bare-with-index-bare
  touch index
)

git init --bare bare-with-index-no-config-bare
(cd bare-with-index-no-config-bare
  touch index
  rm config
)

git init non-bare-without-index
(cd non-bare-without-index
  touch this
  git add this
  git commit -m "init"
  rm .git/index
)

git --git-dir=repo-with-worktree-in-config-unborn-no-worktreedir --work-tree=does-not-exist-yet init
worktree=repo-with-worktree-in-config-unborn-worktree
git --git-dir=repo-with-worktree-in-config-unborn --work-tree=$worktree init && mkdir $worktree

repo=repo-with-worktree-in-config-unborn-empty-worktreedir
git --git-dir=$repo --work-tree="." init
touch $repo/index
git -C $repo config core.worktree ''

repo=repo-with-worktree-in-config-unborn-worktreedir-missing-value
git --git-dir=$repo init
touch $repo/index
echo "    worktree" >> $repo/config

worktree=repo-with-worktree-in-config-worktree
git --git-dir=repo-with-worktree-in-config --work-tree=$worktree init
mkdir $worktree && touch $worktree/file
(cd repo-with-worktree-in-config
  git add file
  git commit -m "make sure na index exists"
)
