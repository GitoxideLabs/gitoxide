#!/usr/bin/env bash
set -eu -o pipefail

# All objects are in a single pack, without a multi-pack index or loose objects,
# so the object database reaches them only through that pack's index.
git init -q
echo content >file
git add file
git commit -q -m "packed commit"
git repack -adq
