#!/bin/sh
set -eu

# A staged version plus a different unstaged version makes the staged-wins rule observable.
git init -q -b main .
git config user.name author
git config user.email author@example.com
printf 'base\n' >tracked
git add tracked
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base
printf 'staged\n' >tracked
git add tracked
printf 'unstaged\n' >tracked
printf 'untracked\n' >untracked
