#!/bin/sh
set -eu

# Removing the middle commit asks the tip's same-line edit to apply to an incompatible base.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git config commit.gpgSign false
printf 'base\n' >file
git add file
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base
printf 'middle\n' >file
git commit -qam middle
printf 'tip\n' >file
git commit -qam tip
