#!/usr/bin/env bash
set -eu -o pipefail

git init -q module1
(cd module1
  touch this
  mkdir subdir
  touch subdir/that
  git add .
  git commit -q -m c1
  echo hello >> this
  git commit -q -am c2
  touch untracked
)

mkdir symlinked-git-dir
(cd symlinked-git-dir
  git init -q r1
  (cd r1
    git commit -q --allow-empty -m "init"
  )

  git config -f r1/.git/config core.worktree "$(pwd)"
  ln -s r1/.git .git

  git -c protocol.file.allow=always submodule add ../module1 m1
  git commit -m "add module 1"
)
