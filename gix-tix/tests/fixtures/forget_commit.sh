#!/bin/sh
set -eu

# The top commit changes and adds tracked files; an unrelated untracked file must survive forgetting it.
git init -q -b main .
git config user.name author
git config user.email author@example.com
printf 'base\n' >tracked
git add tracked
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base
printf 'top\n' >tracked
printf 'added\n' >added
git add tracked added
GIT_AUTHOR_DATE='2000-01-02T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-02T00:00:00 +0000' git commit -q -m top
git update-ref refs/patches/forget HEAD
git update-ref refs/tags/keep HEAD
git update-ref refs/remotes/origin/keep HEAD
printf 'untracked\n' >untracked
