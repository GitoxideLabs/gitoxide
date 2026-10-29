#!/usr/bin/env bash
set -eu

# Unequal sides of a merge test shortest-path depth, while an orphan branch
# leaves a client boundary outside the history selected for later deepening.
git init --initial-branch=main source
(
    cd source
    git commit --allow-empty -m root
    git branch side
    git commit --allow-empty -m main-1
    git commit --allow-empty -m main-2
    git switch side
    git commit --allow-empty -m side-1
    git switch main
    git merge --no-ff side -m merge
    git commit --allow-empty -m tip
    git switch --orphan unrelated
    git commit --allow-empty -m unrelated-root
    git commit --allow-empty -m unrelated-tip
    git switch main
)

# A boundary from a different remote must survive unshallowing when the source
# cannot supply its history.
git init --initial-branch=other foreign
git -C foreign commit --allow-empty -m foreign-root
git -C foreign commit --allow-empty -m foreign-tip
