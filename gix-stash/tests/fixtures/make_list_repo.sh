#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config user.name "Test User"
git config user.email "test@example.com"

# Initial commit so we have a HEAD.
echo "initial" > file.txt
git add file.txt
git commit -q -m "initial commit"

# First stash.
echo "change one" > file.txt
git stash push -m "first stash"

# Second stash.
echo "change two" > file.txt
git stash push -m "second stash"

# Third stash.
echo "change three" > file.txt
git stash push -m "third stash"
