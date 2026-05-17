#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Base commit.
echo "original content" > file.txt
git add file.txt
git commit -q -m "initial commit"

# Make a change and stash it — leaves WT clean.
echo "stashed modification" > file.txt
git stash push -m "stash: stashed modification"
