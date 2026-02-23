#!/usr/bin/env bash
set -eu -o pipefail

# These fixtures use symlinks. See `many.sh` for some that don't.

git init breakout-symlink
(cd breakout-symlink
  mkdir hide
  ln -s ../.. hide/breakout
  touch file
)

ln -s breakout-symlink symlink-to-breakout-symlink

git init immediate-breakout-symlink
(cd immediate-breakout-symlink
  ln -s .. breakout
)

git init excluded-symlinks-to-dir
(cd excluded-symlinks-to-dir
  cat <<'EOF' >.gitignore
src1
src2/
file1
file2/
ignored
ignored-must-be-dir/
EOF
  git add .gitignore && git commit -m "init"

  mkdir src
  >src/file

  mkdir ignored-must-be-dir ignored
  touch ignored-must-be-dir/file ignored/file

  ln -s src src1
  ln -s src src2
  ln -s src/file file1
  ln -s src/file file2
)

ln -s excluded-symlinks-to-dir worktree-root-is-symlink

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

git init -q submodule-symlink
(cd submodule-symlink
  git submodule add ../module sub
  git commit -q -m "add submodule"
  rm -Rf sub
  ln -s ../module sub
)
