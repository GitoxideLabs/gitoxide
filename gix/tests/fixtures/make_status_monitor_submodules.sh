#!/usr/bin/env bash
set -eu -o pipefail

# Only the innermost worktree is dirty. Status must recurse through two initialized
# submodules, with no changed gitlink or earlier dirty result to short-circuit it.
git init -q leaf
(cd leaf
  echo original >file
  git add file
  git commit -qm initial
)

git init -q middle
(cd middle
  git submodule add -q ../leaf inner
  git commit -qm 'add inner'
)

git init -q root
(cd root
  git submodule add -q ../middle outer
  git submodule update -q --init --recursive
  git commit -qm 'add outer'
  echo changed >>outer/inner/file
)
