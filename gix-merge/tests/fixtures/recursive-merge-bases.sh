#!/usr/bin/env bash
set -eu -o pipefail

# Three independent bases expose sibling recursion depth. Unequal generations
# and reversed timestamps additionally expose merge-base order changes caused
# by loading a commit graph. Equal timestamps exercise traversal tie order.
# Diff3 makes the virtual ancestor's bytes observable.
tree () {
  local blob_id
  blob_id=$(printf '%s\n' "$1" | git hash-object -w --stdin)
  printf '100644 blob %s\tfile\n' "$blob_id" | git mktree
}

commit () {
  local tree_id=$1 date=$2
  shift 2
  GIT_AUTHOR_DATE="$date +0000" GIT_COMMITTER_DATE="$date +0000" \
    git commit-tree "$tree_id" "$@" -m fixture
}

for layout in sibling-depth skewed-order equal-times skewed-equal-times corrected-order; do
  git init "$layout"
  (
    cd "$layout"
    root_tree_id=$(tree root)
    root_commit_id=$(commit "$root_tree_id" 1300000000)
    y_parent_commit_id=$root_commit_id
    z_parent_commit_id=$root_commit_id
    if test "$layout" = skewed-order || test "$layout" = skewed-equal-times || test "$layout" = corrected-order; then
      y_parent_date=1300000001
      z_parent_date=1300000003
      if test "$layout" = corrected-order; then
        # These parents are newer than their children. Corrected dates order
        # the bases differently from raw dates, so v2 must affect traversal.
        y_parent_date=1300000080
        z_parent_date=1300000070
      fi
      y_parent_commit_id=$(commit "$root_tree_id" "$y_parent_date" -p "$y_parent_commit_id")
      y_parent_commit_id=$(commit "$root_tree_id" "$((y_parent_date + 1))" -p "$y_parent_commit_id")
      z_parent_commit_id=$(commit "$root_tree_id" "$z_parent_date" -p "$z_parent_commit_id")
    fi
    y_date=1300000020
    z_date=1300000040
    case "$layout" in
      *equal-times|corrected-order) y_date=1300000060; z_date=1300000060 ;;
    esac
    x_commit_id=$(commit "$(tree x)" 1300000060 -p "$root_commit_id")
    y_commit_id=$(commit "$(tree y)" "$y_date" -p "$y_parent_commit_id")
    z_commit_id=$(commit "$(tree z)" "$z_date" -p "$z_parent_commit_id")
    git update-ref refs/heads/A "$(commit "$(tree ours)" 1300000100 -p "$x_commit_id" -p "$y_commit_id" -p "$z_commit_id")"
    git update-ref refs/heads/B "$(commit "$(tree theirs)" 1300000120 -p "$z_commit_id" -p "$y_commit_id" -p "$x_commit_id")"
    git rev-parse A B >commits
    for cache in uncached v1 v2 partial-v2 mixed; do
      rm -rf .git/objects/info/commit-graph .git/objects/info/commit-graphs
      case "$cache" in
        v1) git -c commitGraph.generationVersion=1 commit-graph write --reachable ;;
        v2) git -c commitGraph.generationVersion=2 commit-graph write --reachable ;;
        partial-v2)
          printf '%s\n' "$x_commit_id" "$y_commit_id" "$z_commit_id" |
            git -c commitGraph.generationVersion=2 commit-graph write --stdin-commits
          ;;
        mixed)
          printf '%s\n' "$x_commit_id" "$y_commit_id" "$z_commit_id" |
            git -c commitGraph.generationVersion=1 commit-graph write --stdin-commits
          git -c commitGraph.generationVersion=2 commit-graph write --reachable --split=no-merge
          ;;
      esac
      if test "$cache" != uncached; then cp -R .git/objects/info "$cache"; fi
      for direction in A-B B-A; do
        ours=${direction%-*}
        theirs=${direction#*-}
        status=0
        git -c merge.conflictStyle=diff3 merge-tree -z --write-tree "$ours" "$theirs" >"$direction-$cache.merge-info" || status=$?
        test "$status" -eq 1
        git merge-base --all "$ours" "$theirs" >"$direction-$cache.bases"
      done
    done
  )
done
