#!/bin/sh
set -eu

# HEAD adds its own path, while staged and unstaged hunks edit a shared parent
# file. Moving the staged hunk below HEAD must preserve the unstaged remainder.
# A separate unchanged path exposes staging preservation in another worktree.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git config commit.gpgSign false
printf 'base staged\none\ntwo\nthree\nbase unstaged\n' >shared
printf 'keep\n' >keep
git add shared keep
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base
printf 'head\n' >head
git add head
GIT_AUTHOR_DATE='2000-01-02T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-02T00:00:00 +0000' git commit -q -m head
printf 'staged\none\ntwo\nthree\nbase unstaged\n' >shared
git add shared
printf 'staged\none\ntwo\nthree\nunstaged\n' >shared
printf 'untracked\n' >untracked
