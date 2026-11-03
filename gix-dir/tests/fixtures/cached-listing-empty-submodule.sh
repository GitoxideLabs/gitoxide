#!/usr/bin/env bash
set -eu

# An empty submodule-style worktree contains only a .git file. Partial listings
# may omit that pruned file, but must not classify the repository root as empty.
git init --separate-git-dir=gitdir submodule
