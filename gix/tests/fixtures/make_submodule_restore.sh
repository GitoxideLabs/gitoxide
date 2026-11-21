#!/usr/bin/env bash
set -eu -o pipefail

# The local clone survives deletion of its worktree. Its nested submodule must
# stay unpopulated when restoring the outer checkout without recursion.
git init -q nested-source
(
    cd nested-source
    echo nested >file
    git add file
    git commit -qm nested
)
git init -q module-source
(
    cd module-source
    echo tracked >file
    git add file
    git commit -qm initial
    git submodule add -q ../nested-source nested
    git commit -qm nested
)
git init -q parent
(
    cd parent
    git submodule add -q ../module-source dir/module
    git commit -qm module
)
