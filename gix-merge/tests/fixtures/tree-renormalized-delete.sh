#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# Raw object writes preserve the historical CRLF blob despite today's text
# attribute. Renormalization makes its LF replacement unchanged, so deletion
# should win. Mode changes, real edits, and rename/delete still conflict.
# D/F normalization uses the relocated filename, so wildcard attributes can
# resolve it while attributes applying only to the original filename cannot.
git init renormalize
cd renormalize
printf 'file* text\nrenamed text\nspecific text\n' >.gitattributes
attributes_blob_id=$(git hash-object -w .gitattributes)
base_blob_id=$(printf 'one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n' | git hash-object -w --stdin)
normalized_blob_id=$(printf 'one\ntwo\nthree\nfour\nfive\n' | git hash-object -w --stdin)
modified_blob_id=$(printf 'changed\ntwo\nthree\nfour\nfive\n' | git hash-object -w --stdin)
child_blob_id=$(printf 'independent child\n' | git hash-object -w --stdin)

tree () {
  git read-tree --empty
  git update-index --add --cacheinfo "100644,$attributes_blob_id,.gitattributes"
  while test "$#" -gt 0; do
    git update-index --add --cacheinfo "$2,$3,$1"
    shift 3
  done
  git write-tree
}

reference () {
  local ours=$1 theirs=$2 name=$3 renormalize=$4
  local status=0
  git -c merge.renormalize="$renormalize" merge-tree -z --write-tree "$ours" "$theirs" >"$name.merge-info" || status=$?
  test "$status" -le 1
  echo "$status" >".git/$ours--$theirs.status"
  printf 'renormalize merge %s %s %s %s %s expected\n' \
    "$(git rev-parse "$ours")" "$ours" "$(git rev-parse "$theirs")" "$theirs" \
    "$name.merge-info" >>../baseline.cases
}

: >../baseline.cases
for layout in newline mode content rename directory directory-specific; do
  path=file
  if test "$layout" = directory-specific; then path=specific; fi
  base_tree_id=$(tree "$path" 100644 "$base_blob_id")
  base_commit_id=$(git commit-tree "$base_tree_id" -m base)
  current_entries=("$path" 100644 "$normalized_blob_id")
  other_tree_id=$(tree)
  case "$layout" in
    mode) current_entries=(file 100755 "$normalized_blob_id") ;;
    content) current_entries=(file 100644 "$modified_blob_id") ;;
    rename) current_entries=(renamed 100644 "$normalized_blob_id") ;;
    directory|directory-specific) other_tree_id=$(tree "$path/child" 100644 "$child_blob_id") ;;
  esac
  current_tree_id=$(tree "${current_entries[@]}")
  for renormalize in false true; do
    ours="A-$layout-$renormalize"
    theirs="B-$layout-$renormalize"
    git update-ref "refs/heads/$ours" "$(git commit-tree "$current_tree_id" -p "$base_commit_id" -m "$ours")"
    git update-ref "refs/heads/$theirs" "$(git commit-tree "$other_tree_id" -p "$base_commit_id" -m "$theirs")"
    reference "$ours" "$theirs" "$layout-$renormalize-A-B" "$renormalize"
    reference "$theirs" "$ours" "$layout-$renormalize-B-A" "$renormalize"
  done
done
