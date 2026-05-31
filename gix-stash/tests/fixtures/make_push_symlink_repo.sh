#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Commit a tracked file at HEAD.
echo "original content" > tracked.txt
git add tracked.txt
git commit -q -m "initial commit"

# Create an untracked symlink (not staged) pointing at tracked.txt.
# We don't stage it — it should appear as an untracked entry when
# `include_untracked=true` is set.
ln -s tracked.txt mylink
