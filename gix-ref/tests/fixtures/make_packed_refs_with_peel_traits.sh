#!/usr/bin/env bash
set -eu -o pipefail

# Refs packed by `git pack-refs`, whose header says `fully-peeled`: every annotated
# tag has its peeled object recorded on a `^` line below it, so a ref without one
# isn't an annotated tag and is its own peeled object. The copies differ only in
# their header, as older versions of git wrote it: `peeled` promises that for
# `refs/tags/` alone, and without a header nothing is promised.
#
# There are no objects for the tests to look up: they peel with none at hand.

git init -q fully-peeled
(cd fully-peeled
  git checkout -q -b main
  git commit -q --allow-empty -m c1
  git update-ref refs/remotes/origin/main HEAD
  git tag light
  git tag -m "annotated" annotated
  git pack-refs --all --prune
  head -n 1 .git/packed-refs | grep -q ' fully-peeled '
)

cp -R fully-peeled peeled
{ echo '# pack-refs with: peeled sorted '; tail -n +2 fully-peeled/.git/packed-refs; } > peeled/.git/packed-refs

cp -R fully-peeled no-header
tail -n +2 fully-peeled/.git/packed-refs > no-header/.git/packed-refs
