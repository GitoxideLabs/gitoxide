#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Commit a tracked file at HEAD.
echo "original content" > tracked.txt
git add tracked.txt
git commit -q -m "initial commit"
