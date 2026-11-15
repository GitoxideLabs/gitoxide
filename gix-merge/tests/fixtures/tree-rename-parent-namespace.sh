#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# A directory relocation returns one explicit rename to its original source.
# Its identity must survive when the other side renames that source elsewhere.
# Identical contents also exercise the interaction with executable mode changes.
git init repo
cd repo
blob_id=$(printf 'identical\n' | git hash-object -w --stdin)
tree () {
  git read-tree --empty
  for filepath in "$@"; do
    git update-index --add --cacheinfo "100755,$blob_id,$filepath"
  done
  git write-tree
}
base_tree_id=$(tree f/c/h h/a)
our_tree_id=$(tree f/c/h f/a)
their_tree_id=$(tree a h/c/h)
base_commit_id=$(git commit-tree "$base_tree_id" -m base)
git update-ref refs/heads/A "$(git commit-tree "$our_tree_id" -p "$base_commit_id" -m ours)"
git update-ref refs/heads/B "$(git commit-tree "$their_tree_id" -p "$base_commit_id" -m theirs)"
printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
for direction in A-B B-A; do
  current=${direction%-*}
  other=${direction#*-}
  status=0
  git merge-tree -z --write-tree "$current" "$other" >"$direction.merge-info" || status=$?
  test "$status" -le 1
  echo "$status" >"$direction.status"
done

cd ..
git init file-directory
(
  cd file-directory
  # A separately added descendant must survive choosing its side's directory.
  # The rename returning to h/a also changes executable mode, and the opposite
  # rename targets a, the ancestor of that independently added file.
  blob_id=$(printf 'binary payload 0\0\n' | git hash-object -w --stdin)
  added_blob_id=$(printf 'binary payload 4\0\n' | git hash-object -w --stdin)
  tree () {
    git read-tree --empty
    while test "$#" -gt 0; do
      git update-index --add --cacheinfo "$1,$2,$3"
      shift 3
    done
    git write-tree
  }
  base_tree_id=$(tree 100644 "$blob_id" f/c/h 100755 "$blob_id" h/a)
  our_tree_id=$(tree 100644 "$blob_id" f/c/h 100644 "$blob_id" f/a 100644 "$added_blob_id" a/a/a)
  their_tree_id=$(tree 100755 "$blob_id" a 100644 "$blob_id" h/c/h)
  printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
)

git init single-return
(
  cd single-return
  # An unchanged sibling prevents inferring new->old on A. B's old->new move
  # then returns A's explicit new/file->old/file rename to its deleted source.
  anchor_blob_id=$(printf 'anchor\n' | git hash-object -w --stdin)
  file_blob_id=$(printf 'file\n' | git hash-object -w --stdin)
  keep_blob_id=$(printf 'keep\n' | git hash-object -w --stdin)
  tree () {
    git read-tree --empty
    while test "$#" -gt 0; do
      git update-index --add --cacheinfo "100644,$2,$1"
      shift 2
    done
    git write-tree
  }
  base_tree_id=$(tree old/anchor "$anchor_blob_id" new/file "$file_blob_id" new/keep "$keep_blob_id")
  our_tree_id=$(tree old/anchor "$anchor_blob_id" old/file "$file_blob_id" new/keep "$keep_blob_id")
  their_tree_id=$(tree new/anchor "$anchor_blob_id" new/keep "$keep_blob_id")
  base_commit_id=$(git commit-tree "$base_tree_id" -m base)
  git update-ref refs/heads/A "$(git commit-tree "$our_tree_id" -p "$base_commit_id" -m ours)"
  git update-ref refs/heads/B "$(git commit-tree "$their_tree_id" -p "$base_commit_id" -m theirs)"
  printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
  for policy in true conflict; do
    for direction in A-B B-A; do
      current=${direction%-*}
      other=${direction#*-}
      status=0
      git -c merge.directoryRenames="$policy" merge-tree -z --write-tree "$current" "$other" >"$direction.$policy.merge-info" || status=$?
      test "$status" -le 1
      echo "$status" >"$direction.$policy.status"
    done
  done
)

git init distinct
(
  cd distinct
  # This control removes exact-rename ties entirely: f/c/h and h/c/h use
  # different content from h/a, f/a and a. Git still differs by merge direction.
  original_blob_id=$(printf 'original\n' | git hash-object -w --stdin)
  moved_blob_id=$(printf 'moved\n' | git hash-object -w --stdin)
  tree () {
    git read-tree --empty
    while test "$#" -gt 0; do
      git update-index --add --cacheinfo "100755,$2,$1"
      shift 2
    done
    git write-tree
  }
  base_tree_id=$(tree f/c/h "$moved_blob_id" h/a "$original_blob_id")
  our_tree_id=$(tree f/c/h "$moved_blob_id" f/a "$original_blob_id")
  their_tree_id=$(tree a "$original_blob_id" h/c/h "$moved_blob_id")
  base_commit_id=$(git commit-tree "$base_tree_id" -m base)
  git update-ref refs/heads/A "$(git commit-tree "$our_tree_id" -p "$base_commit_id" -m ours)"
  git update-ref refs/heads/B "$(git commit-tree "$their_tree_id" -p "$base_commit_id" -m theirs)"
  printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
  for direction in A-B B-A; do
    current=${direction%-*}
    other=${direction#*-}
    status=0
    git merge-tree -z --write-tree "$current" "$other" >"$direction.merge-info" || status=$?
    test "$status" -le 1
    echo "$status" >"$direction.status"
    git -c merge.directoryRenames=false merge-tree -z --write-tree "$current" "$other" >"$direction.no-directories.merge-info" || status=$?
    test "$status" -le 1
    git diff-tree -r -M100% "$base_commit_id" "$current" >"$current.exact-renames"
  done
)
