#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git checkout -q -b main
git commit -q --allow-empty -m 'initial commit'
git tag lightweight-tag
git tag -a v1.0 -m 'version 1.0'
git pack-refs --all
