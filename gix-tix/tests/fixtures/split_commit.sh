#!/bin/sh
set -eu

# A split must preserve disjoint staged and unstaged hunks, including changes to the same file.
git init -q -b main .
git config user.name author
git config user.email author@example.com
printf 'one\nbase staged\nmiddle\nbase worktree\nend\n' >both
printf 'base\n' >staged
printf 'base\n' >unstaged
git add both staged unstaged
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base

printf 'one\nstaged\nmiddle\nbase worktree\nend\n' >both
printf 'staged\n' >staged
git add both staged

printf 'one\nstaged\nmiddle\nworktree\nend\n' >both
printf 'worktree\n' >unstaged
printf 'untracked\n' >untracked
