#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Base commit.
echo "original content" > file.txt
git add file.txt
git commit -q -m "initial commit"

# First stash (becomes stash@{1} after the second push).
echo "older stash" > file.txt
git stash push -m "stash: older modification"

# Second stash (newest, becomes stash@{0}).
echo "newer stash" > other.txt
git add other.txt
git stash push -m "stash: newer modification"
