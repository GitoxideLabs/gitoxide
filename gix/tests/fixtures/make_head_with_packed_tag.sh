#!/usr/bin/env bash
set -eu -o pipefail

# HEAD points to an annotated tag with a peeled ID in packed-refs.
git init -q
git commit -qm initial --allow-empty
git tag -am annotated tagged
git pack-refs --all --prune
git symbolic-ref HEAD refs/tags/tagged
