#!/usr/bin/env bash
set -eu -o pipefail

# Three commits plus divergent and unrelated tips exercise ancestry and exact
# ahead/behind counts. Record Git's answers during setup, never during the tests.
# Each row contains tip ID, base ID, is-ancestor exit status, ahead, and behind.
baseline() {
  local pair tip base status
  >.git/history.baseline
  for pair in 'HEAD parent' 'parent HEAD' 'HEAD HEAD' \
    'HEAD divergent' 'divergent HEAD' 'HEAD unrelated' 'unrelated HEAD' \
    'HEAD root' 'root HEAD'; do
    read -r tip base <<<"$pair"
    tip=$(git rev-parse "$tip")
    base=$(git rev-parse "$base")
    if git merge-base --is-ancestor "$base" "$tip"; then
      status=0
    else
      status=$?
      test "$status" = 1
    fi
    printf '%s %s %s ' "$tip" "$base" "$status" >>.git/history.baseline
    git rev-list --left-right --count "$tip...$base" >>.git/history.baseline
  done
}

git init -q
git checkout -q -b main
git commit -q --allow-empty -m root
git tag root
git commit -q --allow-empty -m parent
git tag parent
git commit -q --allow-empty -m tip
git commit-graph write --no-progress --reachable

# These tips deliberately remain outside the commit-graph.
divergent=$(git commit-tree HEAD^{tree} -p parent -m divergent)
unrelated=$(git commit-tree HEAD^{tree} -m unrelated)
git update-ref refs/tags/divergent "$divergent"
git update-ref refs/tags/unrelated "$unrelated"
baseline

# All variants retain identical object IDs. Tests can reuse populated graphs
# across full and shallow views without modifying a shared read-only fixture.
for name in without-commit-graph missing-info shallow replacements; do
  mkdir "$name"
  cp -R .git "$name/.git"
done
rm without-commit-graph/.git/objects/info/commit-graph
rm -r missing-info/.git/objects/info

# Retain older objects and the pre-shallow cache behind the boundary.
git rev-parse parent >shallow/.git/shallow
(cd shallow; baseline)

# A parentless replacement changes topology without updating the stale cache.
(cd replacements
  cp .git/history.baseline .git/unmodified-history.baseline
  replacement=$(git commit-tree HEAD^{tree} -m replacement)
  git replace HEAD "$replacement"
  baseline
)
