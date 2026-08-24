#!/usr/bin/env bash
set -x
set -euo pipefail

# We override the global config with our own local one (see below)
export GIT_CONFIG_GLOBAL="$PWD/.gitconfig"

# We need to be able to do partial clones, so enable it
# - needs to be present in the persistent gitconfig, as a clone with `--no-local`
git config --global uploadpack.allowFilter true

# First build out a base repository
git init base
(
    cd base

    echo "blob 1" > blob-1
    git add -A
    git commit -m "commit 1"
    echo "blob-2" > blob-2
    mkdir tree-1
    echo "blob-3" > tree-1/blob-3
    git add -A
    git commit -m "commit 2"
    git rm blob-1 tree-1/blob-3
    git add -A
    git commit -m "commit 3"
)

# Blobless clone
git clone --no-local --no-hardlinks --filter=blob:none ./base blobless

# Treeless (and blobless) clone. Materialize only the tip worktree so the two
# ancestor root trees remain predictably absent instead of depending on clone behavior.
git clone --no-local --no-hardlinks --no-checkout --filter=tree:0 ./base treeless
git -C treeless checkout HEAD -- .
