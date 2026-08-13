#!/bin/sh
set -eu

# A three-commit linear stack makes parent-only rewrites and tree-transplanting observable.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git config commit.gpgSign false

printf 'base\n' >base
git add base
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base

printf 'middle\n' >middle
git add middle
GIT_AUTHOR_DATE='2000-01-02T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-02T00:00:00 +0000' git commit -q -m middle
git update-ref refs/patches/middle HEAD

printf 'tip\n' >tip
git add tip
GIT_AUTHOR_DATE='2000-01-03T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-03T00:00:00 +0000' git commit -q -m tip
git update-ref refs/patches/tip HEAD
