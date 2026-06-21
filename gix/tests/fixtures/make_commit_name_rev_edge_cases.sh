#!/usr/bin/env bash
set -eu -o pipefail

git init -q

# One commit with far-future annotated tags checks that ref sorting keeps the full
# Git timestamp range instead of truncating to u32.
echo A >f
git add f
GIT_COMMITTER_DATE="@1 +0000" GIT_AUTHOR_DATE="@1 +0000" git commit -qm A
GIT_COMMITTER_DATE="@4294967296 +0000" git tag -a a -m a
GIT_COMMITTER_DATE="@4294967297 +0000" git tag -a z -m z

# A lightweight tag to a blob must be ignored by name-rev even when all tags are
# selected. Passing it to graph traversal would try to look up the blob as a commit.
git tag blob-tag HEAD:f
