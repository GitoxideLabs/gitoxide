#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git checkout -q -b main
git commit -q --allow-empty -m 'initial commit'
git branch feature
