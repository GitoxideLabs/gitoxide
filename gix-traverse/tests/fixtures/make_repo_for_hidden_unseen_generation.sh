#!/usr/bin/env bash
set -eu -o pipefail

# Short: H -> Z -> A, layers [A], [Z], [H].
# Long: H -> Z1 -> Z2 -> A, layers [A], [Z2, Z1], [H].
# A is newest, so seeing only A and H must not justify generation-based stopping
# before the unseen Z layer can order A behind its descendants or hide it.
# The long chain also defeats hidden painting that only looks two edges ahead.
# Rust post-processing sets every middle-layer generation to zero (a legacy
# writer) and H's to one (its parent's stored generation plus one), then repairs
# the checksums and chain. Each file is uniformly zero or nonzero, as Git v1
# verification requires.
for length in short long; do
  (
    mkdir "$length"
    cd "$length"
    git init -q -b main
    git config commitGraph.generationVersion 1

    GIT_AUTHOR_DATE='1700000300 +0000' GIT_COMMITTER_DATE='1700000300 +0000' \
      git commit --allow-empty -m A
    git tag A
    git commit-graph write --reachable --split=no-merge --no-progress

    middle=Z
    if test "$length" = long; then
      GIT_AUTHOR_DATE='1700000050 +0000' GIT_COMMITTER_DATE='1700000050 +0000' \
        git commit --allow-empty -m Z2
      git tag Z2
      middle=Z1
    fi
    GIT_AUTHOR_DATE='1700000100 +0000' GIT_COMMITTER_DATE='1700000100 +0000' \
      git commit --allow-empty -m "$middle"
    git tag "$middle"
    git commit-graph write --reachable --split=no-merge --no-progress

    GIT_AUTHOR_DATE='1700000200 +0000' GIT_COMMITTER_DATE='1700000200 +0000' \
      git commit --allow-empty -m H
    git tag H
    git commit-graph write --reachable --split=no-merge --no-progress

    git log --all --format='%H %s' >commit-names.baseline
  )
done
