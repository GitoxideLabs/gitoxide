#!/usr/bin/env bash
set -eu -o pipefail

# Git recognizes normalized refs/* symlink destinations as symbolic refs,
# including dangling refs. Other symlinks are read as ordinary reference files.
git init -q -b main
git commit -q --allow-empty -m first
git tag first
git commit -q --allow-empty -m second
git symbolic-ref refs/heads/alias refs/heads/main
rm .git/HEAD
ln -s refs/heads/alias .git/HEAD
ln -s refs/tags/first .git/ORIG_HEAD
ln -s refs/heads/missing .git/DANGLING
ln -s ./refs/heads/main .git/RELATIVE
# The target is interpreted as a repository refname even though OS traversal
# would resolve it relative to refs/heads and find no file there.
ln -s refs/heads/main .git/refs/heads/from-link
