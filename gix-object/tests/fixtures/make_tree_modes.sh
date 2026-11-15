#!/usr/bin/env bash
set -eu -o pipefail

# Git accepts noncanonical tree modes with a badFilemode warning.
# In particular, 140000 is a gitlink, while 040000 is a zero-padded directory.
git init -q
subtree_id=$(git mktree </dev/null)
escaped_id=$(printf '%s' "$subtree_id" | sed 's/../\\x&/g')
for mode in 100700 100100 100600 100010 100664 120777 140000 140777 170000 060000 40000 040000 40777 040777; do
    { printf '%s entry\0' "$mode"; printf '%b' "$escaped_id"; } >"$mode.tree"
    # mktree normalizes modes, so store the original bytes literally instead.
    tree_id=$(git hash-object -w -t tree --literally --stdin <"$mode.tree")
    git ls-tree "$tree_id" >"$mode.baseline"
done
