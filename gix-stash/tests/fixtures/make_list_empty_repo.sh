#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Initial commit so the repo is valid but has no stashes.
echo "initial" > file.txt
git add file.txt
git commit -q -m "initial commit"
