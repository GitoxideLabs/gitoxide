#!/usr/bin/env bash
set -eu -o pipefail

git init -q untracked-only
(cd untracked-only
  touch this
  mkdir subdir
  >subdir/that

  git add .
  git commit -q -m init

  mkdir new
  touch new/untracked subdir/untracked
)

git init git-mv
(cd git-mv
  echo hi > file
  git add file && git commit -m "init"

  git mv file renamed
)

git init racy-git
(cd racy-git
  echo hi >file
  git add file && git commit -m "init"

  echo ho >file && git add file
  echo ha >file
)

git init untracked-unborn
(cd untracked-unborn
  touch untracked
)

git init untracked-added
(cd untracked-added
  echo content >added
  git add added
)

git init symlink-replaces-tracked-dir
(cd symlink-replaces-tracked-dir
  mkdir tracked target
  echo content >tracked/file
  echo other >target/file
  git add tracked/file
  git commit -m init

  rm -rf tracked
  ln -s target tracked
)

git init -q module
(cd module
  touch this
  git add this
  git commit -q -m c1
)

git init -q submodule-assume-unchanged-symlink
(cd submodule-assume-unchanged-symlink
  git submodule add ../module sub
  git commit -q -m "add submodule"
  git update-index --assume-unchanged sub
  rm -Rf sub
  ln -s ../module sub
)
