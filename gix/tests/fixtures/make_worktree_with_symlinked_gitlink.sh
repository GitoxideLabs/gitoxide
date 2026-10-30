#!/usr/bin/env bash
set -eu -o pipefail

# Worktree removal must reject a symlink at an unconfigured gitlink instead of
# inspecting its target as a submodule. The target is the main repository, whose
# HEAD supplies a real commit for the gitlink; there is deliberately no .gitmodules.
# Use Creation::Execute because Git records absolute worktree registration paths.
git init -q main
git -C main commit -q --allow-empty -m init
git -C main worktree add -q --detach ../linked HEAD
ln -s ../main linked/module

# Stage a gitlink without replacing the symlink with a submodule checkout.
git -C main rev-parse HEAD |
  sed 's/.*/160000 &	module/' |
  git -C linked update-index --index-info
