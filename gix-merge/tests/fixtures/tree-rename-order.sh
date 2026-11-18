#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# ORT initially defers directories added on only one side. A root destination
# is therefore matched before a lexically earlier destination inside such a
# directory. Repeated payloads expose that identity choice in conflict stages.
# Changed directories with an unchanged opposite input are deferred too. A new
# directory present on both sides and distinct payloads are control cases.
git init matrix
cd matrix
git config merge.directoryRenames true
git config merge.conflictStyle merge
: >../baseline.cases

tree () {
  git read-tree --empty
  while test "$#" -gt 0; do
    git update-index --add --cacheinfo "$2,$3,$1"
    shift 3
  done
  git write-tree
}

record_case () {
  base_tree_id=$(tree "${base[@]}")
  our_tree_id=$(tree "${ours[@]}")
  their_tree_id=$(tree "${theirs[@]}")
  base_commit_id=$(git commit-tree "$base_tree_id" -m "$name base")
  git update-ref "refs/heads/A-$name" "$(git commit-tree "$our_tree_id" -p "$base_commit_id" -m ours)"
  git update-ref "refs/heads/B-$name" "$(git commit-tree "$their_tree_id" -p "$base_commit_id" -m theirs)"
  for direction in A-B B-A; do
    current="${direction%-*}-$name"
    other="${direction#*-}-$name"
    status=0
    git merge-tree -z --write-tree "$current" "$other" >"$name-$direction.merge-info" || status=$?
    test "$status" -le 1
    echo "$status" >".git/$current--$other.status"
    printf 'matrix merge %s %s %s %s %s expected\n' \
      "$(git rev-parse "$current")" "$current" "$(git rev-parse "$other")" "$other" \
      "$name-$direction.merge-info" >>../baseline.cases
  done
}

first_blob_id=$(printf 'payload\n' | git hash-object -w --stdin)
distinct_blob_id=$(printf 'different\n' | git hash-object -w --stdin)
anchor_blob_id=$(printf 'anchor\n' | git hash-object -w --stdin)
extra_blob_id=$(printf 'extra\n' | git hash-object -w --stdin)
for content in repeated distinct; do
  second_blob_id=$first_blob_id
  if test "$content" = distinct; then second_blob_id=$distinct_blob_id; fi
  for mode in 100644 100755; do
    for directory in new unchanged shared relevant two-new two-punctuation two-colliding two-front-tree two-front-file two-front-parent-file two-front-common no-deferred-front common-blocked-front; do
      name="$content-$mode-$directory"
      base=(one/name "$mode" "$first_blob_id" two/name 100644 "$second_blob_id")
      ours=(dst/name 100644 "$second_blob_id" flat 100644 "$first_blob_id")
      theirs=(result/name 100644 "$first_blob_id")
      if test "$directory" = shared || test "$directory" = unchanged; then theirs+=(dst/anchor 100644 "$anchor_blob_id"); fi
      if test "$directory" = unchanged; then
        base+=(dst/anchor 100644 "$anchor_blob_id")
        ours+=(dst/anchor 100644 "$anchor_blob_id")
      fi
      if test "$directory" = relevant; then
        # A direct addition in a removed directory requires rename information
        # for all descendants, so its newly added nested directory is not deferred.
        base+=(dst/anchor 100644 "$anchor_blob_id")
        ours=(dst/nested/name 100644 "$second_blob_id" flat 100644 "$first_blob_id"
          dst/anchor 100644 "$anchor_blob_id" dst/extra 100644 "$extra_blob_id")
      fi
      if test "$directory" = two-new; then
        ours=(a/name 100644 "$second_blob_id" b/name 100644 "$first_blob_id")
      elif test "$directory" = two-punctuation; then
        ours=(a/name 100644 "$second_blob_id" a-/name 100644 "$first_blob_id")
      elif test "$directory" = two-colliding; then
        # dir9 and dir30 share bucket 39 in Git's initial 64-bucket hashmap.
        ours=(dir9/name 100644 "$second_blob_id" dir30/name 100644 "$first_blob_id")
      elif test "$directory" = two-front-tree; then
        # Base/theirs offer a before ours' a$: three-input sibling fronts
        # determine insertion order even when every entry is a directory.
        base+=(a/anchor 100644 "$anchor_blob_id")
        ours=(a/anchor 100644 "$anchor_blob_id" a/name 100644 "$second_blob_id" 'a$/name' 100644 "$first_blob_id")
        theirs+=(a/anchor 100644 "$anchor_blob_id")
      elif test "$directory" = two-front-file; then
        # a and a$ collide in bucket62; the base file a pulls its replacement
        # directory ahead of a$, reversing their usual directory order.
        base+=(a 100644 "$anchor_blob_id")
        ours=(a/name 100644 "$second_blob_id" 'a$/name' 100644 "$first_blob_id")
        theirs+=(a 100644 "$anchor_blob_id")
      elif test "$directory" = two-front-parent-file; then
        # The same sibling-front rule orders nested deferred roots: a/d38 and
        # a$ collide in bucket62, while the modified parent a is visited first.
        base+=(a 100644 "$anchor_blob_id")
        ours=(a/d38/name 100644 "$second_blob_id" 'a$/name' 100644 "$first_blob_id")
        theirs+=(a 100644 "$extra_blob_id")
      elif test "$directory" = two-front-common; then
        # An all-three-identical opaque tree interleaves the input fronts but
        # is omitted from merge's sparse state. a- and b0 collide in bucket55.
        base+=(a/anchor 100644 "$anchor_blob_id")
        ours=(a/anchor 100644 "$anchor_blob_id" a-/name 100644 "$second_blob_id" b0/name 100644 "$first_blob_id")
        theirs+=(a/anchor 100644 "$anchor_blob_id")
      elif test "$directory" = no-deferred-front; then
        # Neither A directory is deferred: a is shared, while a$ replaces a
        # base file deleted by B. B's a front still pulls it ahead of A's a$.
        base+=('a$' 100644 "$anchor_blob_id")
        ours=(a/name 100644 "$second_blob_id" 'a$/name' 100644 "$first_blob_id")
        theirs+=(a/anchor 100644 "$anchor_blob_id")
      elif test "$directory" = common-blocked-front; then
        # Common a- blocks A's a frontier until B's a$ is visited. Removing
        # that placeholder reverses B's ambiguous destination matching order.
        base+=('a$/anchor' 100644 "$extra_blob_id" a-/anchor 100644 "$anchor_blob_id")
        ours=(a-/anchor 100644 "$anchor_blob_id" a/anchor 100644 "$anchor_blob_id" result/name 100644 "$first_blob_id")
        theirs=(a-/anchor 100644 "$anchor_blob_id" a/name 100644 "$second_blob_id" 'a$' 100644 "$first_blob_id")
      fi
      record_case
    done
  done
done

# Git grows its pending-directory hashmap after the 52nd entry (80% of 64).
# The executable source makes its chosen destination's base stage observable.
for size in 51 52 53 source-only; do
  name="growth-$size"
  base=()
  ours=()
  theirs=(result/name 100644 "$first_blob_id")
  if test "$size" = source-only; then
    size=51
    # Even an unchanged directory deleted on A is queued, and counts towards
    # growth although it has no possible rename destinations on that side.
    base+=(unmoved/name 100644 "$anchor_blob_id")
    theirs+=(unmoved/name 100644 "$anchor_blob_id")
  fi
  for ((number=0; number<size; number++)); do
    printf -v suffix '%02d' "$number"
    mode=100644
    if test "$number" = 0; then mode=100755; fi
    base+=("source$suffix/name" "$mode" "$first_blob_id")
    ours+=("destination$suffix/name" 100644 "$first_blob_id")
  done
  record_case
done
