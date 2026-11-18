#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# Directory relocation can bring different gitlinks together during rename
# normalization, before the ordinary path resolver sees them. They reference
# real commits, and must never be opened as blobs. No submodule is checked out.
git init matrix
cd matrix
: >../baseline.cases
empty_tree_id=$(git mktree </dev/null)
first_commit_id=$(git commit-tree "$empty_tree_id" -m first)
second_commit_id=$(git commit-tree "$empty_tree_id" -m second)
third_commit_id=$(git commit-tree "$empty_tree_id" -m third)
old_blob_id=$(printf 'old\n' | git hash-object -w --stdin)
tree () {
  git read-tree --empty
  while test "$#" -gt 0; do
    git update-index --add --cacheinfo "160000,$2,$1"
    shift 2
  done
  git write-tree
}
for layout in add-add modify directory nested-directory type-regular type-link; do
  case "$layout" in
    add-add)
      base_tree_id=$empty_tree_id
      our_tree_id=$(tree module "$first_commit_id")
      their_tree_id=$(tree module "$second_commit_id")
      ;;
    modify)
      base_tree_id=$(tree module "$first_commit_id")
      our_tree_id=$(tree module "$second_commit_id")
      their_tree_id=$(tree module "$third_commit_id")
      ;;
    type-regular|type-link)
      mode=100644
      if test "$layout" = type-link; then mode=120000; fi
      base_tree_id=$(printf '%s blob %s\tmodule\n' "$mode" "$old_blob_id" | git mktree)
      our_tree_id=$(tree module "$first_commit_id")
      their_tree_id=$(tree module "$second_commit_id")
      ;;
    directory)
      base_tree_id=$(tree src/one "$first_commit_id" dst "$second_commit_id")
      our_tree_id=$(tree new/two "$first_commit_id" new/one "$second_commit_id")
      their_tree_id=$(tree src/two "$second_commit_id")
      ;;
    nested-directory)
      # The original minimized fuzz history additionally nests both renames.
      base_tree_id=$(tree a "$first_commit_id" h/h "$second_commit_id")
      our_tree_id=$(tree h/a "$first_commit_id" h/h/h "$second_commit_id")
      their_tree_id=$(tree a/c/a "$second_commit_id")
      ;;
  esac
  printf '%s\n' "$base_tree_id" >"$layout.base.tree"
  printf '%s\n' "$our_tree_id" >"A-$layout.tree"
  printf '%s\n' "$their_tree_id" >"B-$layout.tree"
  # Favoring a tree entry preserves accepted directory relocation. Virtual
  # ancestors also retain intermediate rename destinations for later merges.
  case "$layout" in
    directory)
      tree dst "$second_commit_id" src/one "$first_commit_id" new/one "$second_commit_id" >directory.virtual-ancestor.tree
      tree new/two "$second_commit_id" >directory.B-relocated.tree
      tree new/one "$second_commit_id" new/two "$second_commit_id" >directory.B-virtual.tree
      ;;
    nested-directory)
      tree h/a "$first_commit_id" a/c/h/h "$second_commit_id" >nested-directory.A-relocated.tree
      tree a/c/a "$first_commit_id" >nested-directory.A-virtual.tree
      tree a/c/a "$second_commit_id" a/c/h/h "$second_commit_id" >nested-directory.B-virtual.tree
      ;;
  esac
  base_commit_id=$(git commit-tree "$base_tree_id" -m base)
  git update-ref "refs/heads/A-$layout" "$(git commit-tree "$our_tree_id" -p "$base_commit_id" -m ours)"
  git update-ref "refs/heads/B-$layout" "$(git commit-tree "$their_tree_id" -p "$base_commit_id" -m theirs)"
  for relocation in true conflict; do
    for direction in A-B B-A; do
      current="${direction%-*}-$layout"
      other="${direction#*-}-$layout"
      status=0
      git -c merge.directoryRenames="$relocation" merge-tree -z --write-tree "$current" "$other" >"$layout-$relocation-$direction.merge-info" || status=$?
      test "$status" -le 1
      echo "$status" >"$layout-$relocation-$direction.status"
      printf 'matrix merge %s %s %s %s %s expected\n' \
        "$(git rev-parse "$current")" "$current" "$(git rev-parse "$other")" "$other" \
        "$layout-$relocation-$direction.merge-info" >>../baseline.cases
    done
  done
done
