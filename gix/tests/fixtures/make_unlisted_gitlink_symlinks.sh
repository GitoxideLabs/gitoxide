#!/usr/bin/env bash
set -eu -o pipefail

# Exercise the status provider directly: the index gitlink has no .gitmodules entry.
gix_testtools_require_symlinks
git init -q
git init -q d
git -C d commit -q --allow-empty -m initial
git add d
git commit -qm gitlink
git -C d commit -q --allow-empty -m changed
ln -s . alias
ln -s d module-link
