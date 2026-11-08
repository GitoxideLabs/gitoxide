#!/usr/bin/env bash
set -eu

# An inferred move into an existing directory puts a new descendant beneath a
# modified file, or directly onto one. Retrying at the original path must not
# restart that relocation.
git init repo
cd repo
mkdir old existing
printf anchor >old/anchor
printf base >existing/block
printf base-collision >existing/collision
git add .
git commit -m base
git branch base
git checkout -b ours
mkdir old/block
printf nested >old/block/fresh
printf direct >old/fresh
printf new-collision >old/collision
git add .
git commit -m additions
git checkout -b theirs base
git mv old/anchor existing/anchor
printf changed >existing/block
printf changed-collision >existing/collision
git add .
git commit -m 'rename directory and modify destination blocker'
git rev-parse base^{tree} ours^{tree} theirs^{tree} >trees
