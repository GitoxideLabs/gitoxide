#!/bin/sh
set -eu

# Several refs at one commit distinguish exact symbolic pin targets from their aliases.
git init -q -b main .
git commit -q --allow-empty -m root
git branch unpinned
git tag -a annotated -m annotated
git update-ref refs/remotes/origin/main HEAD
git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
git symbolic-ref refs/worktree/tix/pins/local refs/heads/main
git symbolic-ref refs/worktree/tix/pins/again refs/heads/main
git symbolic-ref refs/worktree/tix/pins/alias refs/remotes/origin/HEAD
git symbolic-ref refs/worktree/tix/pins/tagged refs/tags/annotated
