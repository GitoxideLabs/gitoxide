#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# Directory destinations include the repository root. Ambiguous destinations,
# existing blockers, and multiple same-side additions retain their original
# paths, but still require attention even when the index has only stage zero.
git init matrix
cd matrix
git config merge.conflictStyle merge
: >../baseline.cases

tree () {
  git read-tree --empty
  while test "$#" -gt 0; do
    git update-index --add --cacheinfo "100644,$2,$1"
    shift 2
  done
  git write-tree
}

first_blob_id=$(printf 'first\n' | git hash-object -w --stdin)
second_blob_id=$(printf 'second\n' | git hash-object -w --stdin)
added_blob_id=$(printf 'added\n' | git hash-object -w --stdin)
blocked_blob_id=$(printf 'occupied\n' | git hash-object -w --stdin)

for layout in root nested-root tie blocked-file blocked-parent collision; do
  base=(old/a "$first_blob_id")
  ours=(a "$first_blob_id")
  theirs=(old/a "$first_blob_id" old/new "$added_blob_id")
  case "$layout" in
    nested-root)
      base=(old/sub/a "$first_blob_id")
      ours=(sub/a "$first_blob_id")
      theirs=(old/sub/a "$first_blob_id" old/new "$added_blob_id") ;;
    tie)
      base=(old/a "$first_blob_id" old/b "$second_blob_id")
      ours=(left/a "$first_blob_id" right/b "$second_blob_id")
      theirs=("${base[@]}" old/new "$added_blob_id") ;;
    blocked-file)
      base=(old/a "$first_blob_id" target/occupied "$blocked_blob_id")
      ours=(target/a "$first_blob_id" target/occupied "$blocked_blob_id")
      theirs=("${base[@]}" old/occupied "$added_blob_id") ;;
    blocked-parent)
      base=(old/a "$first_blob_id" target/block "$blocked_blob_id")
      ours=(target/a "$first_blob_id" target/block "$blocked_blob_id")
      # The direct addition makes old/ relevant to directory-rename detection.
      theirs=("${base[@]}" old/fresh "$first_blob_id" old/block/new "$added_blob_id") ;;
    collision)
      base=(one/a "$first_blob_id" two/b "$second_blob_id")
      ours=(target/a "$first_blob_id" target/b "$second_blob_id")
      theirs=("${base[@]}" one/new "$added_blob_id" two/new "$blocked_blob_id") ;;
  esac
  base_commit_id=$(git commit-tree "$(tree "${base[@]}")" -m "$layout base")
  git update-ref "refs/heads/A-$layout" "$(git commit-tree "$(tree "${ours[@]}")" -p "$base_commit_id" -m ours)"
  git update-ref "refs/heads/B-$layout" "$(git commit-tree "$(tree "${theirs[@]}")" -p "$base_commit_id" -m theirs)"
  for mode in true conflict false; do
    git config merge.directoryRenames "$mode"
    for direction in A-B B-A; do
      current="${direction%-*}-$layout"
      other="${direction#*-}-$layout"
      name="$layout-$mode-$direction"
      status=0
      git merge-tree -z --write-tree "$current" "$other" >"$name.merge-info" || status=$?
      test "$status" -le 1
      echo "$status" >".git/$current--$other-$mode.status"
      printf 'matrix merge %s %s %s %s %s expected\n' \
        "$(git rev-parse "$current")" "$current" "$(git rev-parse "$other")" "$other" \
        "$name.merge-info" >>../baseline.cases
    done
  done
done
