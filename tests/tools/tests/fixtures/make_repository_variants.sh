#!/bin/sh
set -eu

# Exercise repository forms and nested Git repositories which need special treatment when captured:
# a plain gitlink without .gitmodules metadata, a configured submodule, an untracked embedded repository,
# a linked worktree, a bare clone, an unborn repository, a shallow boundary, replacement refs, and a Git
# directory whose worktree is configured through core.worktree, core.bare overriding a discovered worktree,
# legacy grafts and replacement refs which snapshots must ignore, and a sparse index containing a nested tree.
git init -q -b main source
echo submodule-source >source/tracked
git -C source add tracked
git -C source commit -q -m source

git init -q -b main main
echo superproject >main/tracked
git -C main add tracked
git -C main commit -q -m base
git -C main -c protocol.file.allow=always submodule add -q ../source submodule

git init -q -b main main/embedded
echo embedded >main/embedded/tracked
git -C main/embedded add tracked
git -C main/embedded commit -q -m embedded
git -C main add .gitmodules embedded submodule
git -C main commit -q -m gitlinks

git init -q -b main main/untracked-repository
echo untracked-repository >main/untracked-repository/tracked
git -C main/untracked-repository add tracked
git -C main/untracked-repository commit -q -m untracked

git -C main worktree add -q -b linked ../linked
git clone -q --bare main bare.git
git init -q -b main unborn

git clone -q main shallow
git -C shallow rev-parse HEAD >shallow/.git/shallow

git clone -q main replaced
git -C replaced replace HEAD HEAD~1
git -C replaced rev-parse HEAD >replaced/.git/info/grafts

git clone -q --separate-git-dir="$PWD/split.git" main split-worktree
git --git-dir=split.git config core.worktree ../split-worktree

git clone -q main configured-original
git clone -q main configured-worktree
echo configured-worktree >configured-worktree/configured-only
git -C configured-original config extensions.worktreeConfig true
git -C configured-original config --worktree core.worktree ../../configured-worktree

git clone -q main configured-bare
git -C configured-bare config core.bare true

git init -q -b main sparse
mkdir -p sparse/visible sparse/hidden/deep
echo visible >sparse/visible/tracked
echo hidden >sparse/hidden/deep/tracked
git -C sparse add .
git -C sparse commit -q -m sparse
git -C sparse sparse-checkout init --cone --sparse-index
git -C sparse sparse-checkout set visible
