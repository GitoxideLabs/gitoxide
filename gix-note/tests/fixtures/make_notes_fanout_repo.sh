#!/usr/bin/env bash
set -eu -o pipefail

# Produce Git's notes-tree IDs at the count boundary where every leading-nibble bucket changes from one note to two.
# The first round remains flat. The last insertion of the second round causes one-byte fanout, and removing both notes
# from the first bucket collapses it again. The baseline lets the Rust test replay the same mutations without invoking
# Git or generating candidate objects at test time.
export GIT_AUTHOR_NAME="Gitoxide Fixture"
export GIT_AUTHOR_EMAIL="gitoxide@example.com"
export GIT_AUTHOR_DATE="2000-01-01T00:00:00Z"
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"

git init --bare -q repo
note=$(printf note | git -C repo hash-object -w --stdin)
printf 'note %s\n' "$note" >fanout.baseline

mkdir selected
for nibble in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
  : >"selected/$nibble"
done

counter=0
selected_count=0
while test "$selected_count" -lt 32; do
  annotated=$(printf 'annotated %s' "$counter" | git -C repo hash-object -w --stdin)
  nibble=${annotated:0:1}
  bucket="selected/$nibble"
  if test "$(wc -l <"$bucket")" -lt 2; then
    printf '%s\n' "$annotated" >>"$bucket"
    selected_count=$((selected_count + 1))
  fi
  counter=$((counter + 1))
done

for round in 1 2; do
  for nibble in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
    if test "$round" = 2 && test "$nibble" = f; then
      tree=$(git -C repo rev-parse 'refs/notes/fanout^{tree}')
      printf 'tree one-bucket-short %s\n' "$tree" >>fanout.baseline
    fi
    annotated=$(sed -n "${round}p" "selected/$nibble")
    git -C repo notes --ref=fanout add -C "$note" "$annotated"
    printf 'add %s\n' "$annotated" >>fanout.baseline
  done
  tree=$(git -C repo rev-parse 'refs/notes/fanout^{tree}')
  if test "$round" = 1; then
    printf 'tree one-per-bucket %s\n' "$tree" >>fanout.baseline
  else
    printf 'tree two-per-bucket %s\n' "$tree" >>fanout.baseline
  fi
done

removal=0
while IFS= read -r annotated; do
  git -C repo notes --ref=fanout remove "$annotated" >/dev/null
  printf 'remove %s\n' "$annotated" >>fanout.baseline
  removal=$((removal + 1))
  tree=$(git -C repo rev-parse 'refs/notes/fanout^{tree}')
  if test "$removal" = 1; then
    printf 'tree first-bucket-half-full %s\n' "$tree" >>fanout.baseline
  else
    printf 'tree first-bucket-empty %s\n' "$tree" >>fanout.baseline
  fi
done <selected/0

rm -rf selected
