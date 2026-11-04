#!/usr/bin/env bash
set -eu

# Two disconnected histories make a forced update cross a shallow boundary.
git init -b main source
(
    cd source
    echo one >file
    git add file
    git commit -m one
    echo two >file
    git commit -am two
    git branch base HEAD~1
    git checkout --orphan other
    git commit -am other-root
    echo other >file
    git commit -am other-tip
    git checkout main
    git checkout -b skew
    GIT_COMMITTER_DATE='2000-01-01 00:00:00 +0000' git commit --allow-empty -m skew-middle
    GIT_COMMITTER_DATE='2000-01-03 00:00:00 +0000' git commit --allow-empty -m skew-tip
    git checkout main
)
git init --bare client
(
    cd client
    git fetch --depth=1 --no-tags ../source +refs/heads/main:refs/remotes/origin/main refs/heads/base:refs/heads/selected
)

git init --bare client-skew
(
    cd client-skew
    git fetch --no-tags ../source refs/heads/base:refs/heads/selected
)
