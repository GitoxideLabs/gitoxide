#!/usr/bin/env bash
set -eu

# An inferred move into an existing directory puts a new descendant beneath a
# modified file, or directly onto one. Exact blockers keep additions at their
# original paths; modified parent blockers undergo ordinary file/directory resolution.
git init repo
cd repo
git config merge.directoryRenames true
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
for direction in ours-theirs theirs-ours; do
  status=0
  git merge-tree -z --write-tree "${direction%-*}" "${direction#*-}" >"$direction.merge-info" || status=$?
  test "$status" = 1
done

# Relocation must preserve a modified parent regardless of which input contains
# its edit. Git currently loses it on either side, including identical edits.
git checkout -b ours-modified ours
printf changed >existing/block
git add existing/block
git commit -m 'modify parent on the side adding its future child'
git checkout -b theirs-unmodified theirs
printf base >existing/block
git add existing/block
git commit -m 'leave the destination parent unchanged'
git checkout -b theirs-different theirs
printf other-change >existing/block
git add existing/block
git commit -m 'modify the parent differently on the directory-renaming side'
for layout in same both different; do
  other=theirs
  if test "$layout" = same; then other=theirs-unmodified; fi
  if test "$layout" = different; then other=theirs-different; fi
  git rev-parse base^{tree} ours-modified^{tree} "$other^{tree}" >"trees-$layout"
  for direction in ours-theirs theirs-ours; do
    current=ours-modified
    opposite=$other
    if test "$direction" = theirs-ours; then current=$other; opposite=ours-modified; fi
    status=0
    git merge-tree -z --write-tree "$current" "$opposite" >"$layout-$direction.merge-info" || status=$?
    test "$status" = 1
  done
done
