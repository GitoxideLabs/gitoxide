#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git checkout -q -b main
git commit -q --allow-empty -m 'first commit'
git branch feature
git checkout -q -b dev
git commit -q --allow-empty -m 'dev commit'
git checkout -q main
