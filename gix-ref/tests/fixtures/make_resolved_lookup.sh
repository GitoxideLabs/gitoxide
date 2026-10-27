#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git commit -q --allow-empty -m initial
# A dangling tag hides the live branch for raw lookup, but not resolved lookup.
git symbolic-ref refs/tags/main refs/tags/missing
git symbolic-ref refs/tags/cycle refs/tags/cycle
git branch cycle
git symbolic-ref refs/heads/alias refs/heads/main
git pack-refs --all
