#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Base commit: file.txt = "content A"
echo "content A" > file.txt
git add file.txt
git commit -q -m "base commit"

# Modify to "content B" and stash — stash records WIP=B on base=A.
echo "content B" > file.txt
git stash push -m "stash: content B"

# Now modify HEAD to "content C" and commit.
# When we pop, base=A, ours=C, theirs=B → conflict.
echo "content C" > file.txt
git add file.txt
git commit -q -m "commit with content C"
