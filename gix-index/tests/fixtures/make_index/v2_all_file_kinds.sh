#!/usr/bin/env bash
set -eu -o pipefail

export GIT_INDEX_VERSION=2

git init -q sub
(cd sub
  touch a b c
  git add .
  git commit -m "init"
)

git init -q
git config index.threads 1

touch a b
chmod +x b
ln -s a c
mkdir d
(cd d && touch a b c)

git submodule add ./sub sub-worktree

git add .
git update-index --chmod=+x b  # For Windows.
git commit -m "init"

git rev-parse @^{tree} > head.tree
