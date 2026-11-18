#!/usr/bin/env bash
set -eu -o pipefail

# Both query tips exist, but deleting the intervening parent makes ancestry
# unknowable. Record Git's fatal result before declaring the tip shallow.
git init
tree_id=$(git mktree </dev/null)
base_commit_id=$(printf 'base\n' | git commit-tree "$tree_id")
parent_commit_id=$(printf 'parent\n' | git commit-tree "$tree_id" -p "$base_commit_id")
tip_commit_id=$(printf 'tip\n' | git commit-tree "$tree_id" -p "$parent_commit_id")
git update-ref refs/tags/base "$base_commit_id"
git update-ref refs/tags/parent "$parent_commit_id"
git update-ref refs/tags/tip "$tip_commit_id"
rm ".git/objects/${parent_commit_id:0:2}/${parent_commit_id:2}"

for direction in base-tip tip-base; do
  ancestor=${direction%-*}
  descendant=${direction#*-}
  status=0
  git -c core.commitGraph=false merge-base --is-ancestor "$ancestor" "$descendant" \
    >"$direction.stdout" 2>"$direction.stderr" || status=$?
  printf '%s\n' "$status" >"$direction.status"
done

# A real shallow boundary intentionally hides the missing parent and answers
# the same ancestry queries without an error. Plumbing callers synthesize it.
printf '%s\n' "$tip_commit_id" >.git/shallow
for direction in base-tip tip-base; do
  ancestor=${direction%-*}
  descendant=${direction#*-}
  status=0
  git -c core.commitGraph=false merge-base --is-ancestor "$ancestor" "$descendant" \
    >"shallow-$direction.stdout" 2>"shallow-$direction.stderr" || status=$?
  printf '%s\n' "$status" >"shallow-$direction.status"
done
