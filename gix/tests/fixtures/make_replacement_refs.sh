#!/usr/bin/env bash
set -eu -o pipefail

# Two disconnected segments are joined by a replacement graft in both repository kinds.
# Write the commit-graph before grafting so it retains stale, disconnected parent links.
# Record Git's raw, replaced, and explicitly configured histories as test baselines.
# Keep ungrafted and graph-free copies so tests never need to mutate the fixture.
git init -q repo
(
  cd repo
  tree=$(git mktree </dev/null)
  a=$(git commit-tree "$tree" -m A)
  b=$(git commit-tree "$tree" -p "$a" -m B)
  s=$(git commit-tree "$tree" -m S)
  c=$(git commit-tree "$tree" -p "$s" -m C)
  git update-ref refs/heads/archive "$b"
  git update-ref refs/heads/squash "$s"
  git update-ref refs/heads/restored "$c"
  git symbolic-ref HEAD refs/heads/restored
  git commit-graph write --reachable
)
git clone -q --bare repo bare
(
  cd bare
  git commit-graph write --reachable
)

for name in repo bare; do
  (
    cd "$name"
    git --no-replace-objects rev-list HEAD >original.baseline
    cp -R . "../$name-original"
    git replace --graft squash archive
    git rev-list HEAD >connected.baseline
    git -c core.useReplaceRefs=false rev-list HEAD >configured-false.baseline
    git -c core.useReplaceRefs=true rev-list HEAD >configured-true.baseline
    cp -R . "../$name-without-graph"
    rm "../$name-without-graph/$(git rev-parse --git-path objects/info/commit-graph)"
  )
done
