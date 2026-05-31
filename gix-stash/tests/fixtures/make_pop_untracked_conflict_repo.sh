#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Base commit with a tracked file.
echo "tracked content" > tracked.txt
git add tracked.txt
git commit -q -m "initial commit"

# Create an untracked file and stash including untracked.
echo "stashed untracked content" > untracked.txt
git stash push --include-untracked -m "stash: with untracked file"

# Simulate the user creating a file at the same path after stashing.
echo "user's own content" > untracked.txt
