#!/usr/bin/env bash
set -eu -o pipefail

# Keep tree pairs, raw input, and Git's rename/copy decisions together. Each case
# starts with an empty index, so candidates from other cases cannot affect it.
git init -q
git config core.autocrlf false
git config core.filemode true
mkdir -p .git/baseline

begin_case() {
  git rm -r -f -q --ignore-unmatch .
  git clean -fdq
  case_dir=".git/baseline/$1"
  mkdir -p "$case_dir"
}

before() {
  git add -A
  before_tree=$(git write-tree)
  git ls-tree -r -z "$before_tree" >"$case_dir/sources"
}

after() {
  git add -A
  after_tree=$(git write-tree)
  git diff-tree --no-commit-id --no-renames -r --raw -z "$before_tree" "$after_tree" >"$case_dir/input"
  baseline default --find-renames=50% --no-rename-empty -l0
}

baseline() {
  local name="$1"
  shift
  git diff-tree --no-commit-id -r --raw -z "$@" "$before_tree" "$after_tree" >"$case_dir/$name"
}

lines() {
  local prefix="$1" count="$2"
  local i
  for ((i = 1; i <= count; i++)); do
    printf '%s%02d\n' "$prefix" "$i"
  done
}

begin_case exact-basename
mkdir -p old/A old/B new/A new/B
printf 'same content\n' >old/A/file
cp old/A/file old/B/file
cp old/A/file old/other
before
mv old/A/file new/A/file
mv old/B/file new/B/file
rm old/other
after
baseline exact --find-renames=100% --no-rename-empty -l1

# t4001-diff-rename.sh: a unique basename is chosen before a higher-scoring
# differently named destination, provided the basename score exceeds 75%.
begin_case basename-prepass
mkdir old
lines line 10 >old/file.txt
before
lines line 8 >file.txt
lines line 9 >file.md
rm old/file.txt
after
baseline high --find-renames=85% --no-rename-empty -l0
baseline limited --find-renames=50% --no-rename-empty -l1

# Both destinations prefer the same source. Choosing pairs globally must reserve
# it for the stronger match instead of accepting the first viable destination.
begin_case global-best
lines shared 10 >old-a
{ lines shared 6; lines other 4; } >old-b
before
{ lines shared 8; lines extra 2; } >new-a
{ lines shared 9; lines third 1; } >new-b
rm old-a old-b
after
baseline limited --find-renames=50% --no-rename-empty -l1

begin_case reordered
lines line 20 >old
before
{ lines line 10; lines new 1; sed -n '11,20p' old | sort -r; } >new
rm old
after
baseline high --find-renames=95% --no-rename-empty -l0
baseline exact --find-renames=100% --no-rename-empty -l0

begin_case crlf
lines line 20 >old
before
awk '{ printf "%s\r\n", $0 }' old >new
rm old
after
baseline high --find-renames=90% --no-rename-empty -l0

begin_case long-lines
# Two full 64-byte spans survive an edit to the third span of one long line.
printf '%064d%064d%064d\n' 1 2 3 >old
before
printf '%064d%064d%064d\n' 1 2 4 >new
rm old
after

begin_case binary
# The NUL forces binary classification; common 64-byte spans still count.
printf '\0%063d%064d%064d\n' 1 2 3 >old
before
printf '\0%063d%064d%064d\n' 1 2 4 >new
rm old
after

begin_case partial-span
{ lines line 10; printf '%050d' 1; } >old
before
{ lines line 9; printf 'changed\n%050d' 1; } >new
rm old
after
# Git before 2.54 ignored an unterminated final span. Keep the runtime baseline
# for supported Git versions; fixed unit tests cover the new scoring semantics.
if git diff-tree --no-commit-id -r --name-status -M "$before_tree" "$after_tree" | grep -q '^R09'; then
  touch "$case_dir/counts-partial-span"
fi

begin_case modified-copy
lines original 20 >source
before
cp source exact-copy
{ lines original 19; lines changed 1; } >edited-copy
lines replacement 20 >source
cp source new-content
after
baseline copies --find-copies=50% --no-rename-empty -l0

begin_case deleted-copy
lines original 20 >source
before
cp source first
cp source second
{ lines original 19; lines changed 1; } >third
rm source
after
baseline copies --find-copies=50% --no-rename-empty -l0

begin_case harder-copy
lines original 20 >source
lines different 20 >modified
before
cp source exact-copy
{ lines original 19; lines changed 1; } >edited-copy
lines replacement 20 >modified
after
baseline copies --find-copies=50% --no-rename-empty -l0
baseline harder --find-copies=50% --find-copies-harder --no-rename-empty -l0

begin_case single-harder-copy
lines original 20 >source
before
cp source only-addition
after
baseline harder --find-copies=50% --find-copies-harder --no-rename-empty -l0

# Copy-harder falls back to modified sources when unchanged sources exceed -l.
begin_case harder-limit
lines original 20 >source
for i in 1 2 3 4 5; do lines "unmodified$i" 20 >"unchanged$i"; done
before
{ lines original 19; lines changed 1; } >copy
lines replacement 20 >source
after
baseline harder --find-copies=50% --find-copies-harder --no-rename-empty -l1

begin_case empty
: >old
before
mv old new
after
baseline tracked --find-renames=50% --rename-empty -l0

begin_case modes
lines executable 20 >old-executable
ln -s target old-link
printf target >old-regular
before
mv old-executable new-executable
chmod +x new-executable
mv old-link new-link
mv old-regular new-regular
ln -s changed-target changed-link
after

begin_case size-delta
lines large 20 >old
before
lines large 9 >new
rm old
after
baseline low --find-renames=40% --no-rename-empty -l0

# Unused sources and matching basenames each contribute one exact-match point.
# A modified basename match therefore ties an unused differently named source.
begin_case exact-copy-competition
mkdir -p old/a-copy old/b-delete old/c-delete new/a new/b new/c
lines original 20 >old/a-copy/file
cp old/a-copy/file old/b-delete/other
cp old/a-copy/file old/c-delete/file
before
cp old/a-copy/file new/a/file
cp old/a-copy/file new/b/file
cp old/a-copy/file new/c/else
lines replacement 20 >old/a-copy/file
rm old/b-delete/other old/c-delete/file
after
baseline copies --find-copies=100% --no-rename-empty -l0

# The matching basename is the 101st identical candidate: Git deliberately
# stops after 100 alternatives and takes the first differently named source.
for alternatives in 99 100; do
  begin_case "exact-candidate-cap-$alternatives"
  mkdir old new
  for ((i = 0; i < alternatives; i++)); do
    printf 'identical\n' >"old/$(printf '%03d' "$i")"
  done
  printf 'identical\n' >old/same
  before
  cp old/same new/same
  rm old/*
  after
  baseline exact --find-renames=100% --no-rename-empty -l1
done

# Every destination has six equally good sources. Only four candidates survive
# per destination, leaving two additions/deletions after those sources are used.
begin_case matrix-ties
for i in 1 2 3 4 5 6; do
  { lines common 9; printf 'old-%s\n' "$i"; } >"source$i"
done
before
for i in 1 2 3 4 5 6; do
  { lines common 9; printf 'new-%s\n' "$i"; } >"destination$i"
done
rm source*
after

# Later candidates must displace earlier, weaker candidates in the four slots.
# The final two sources tie for best; all six destinations compete for them.
begin_case matrix-replace-worst
for i in 1 2 3 4 5 6; do
  count=$((i + 4))
  if [ "$count" -gt 9 ]; then count=9; fi
  { lines common "$count"; lines "other$i" "$((10 - count))"; } >"source$i"
done
before
for i in 1 2 3 4 5 6; do lines common 10 >"destination$i"; done
rm source*
after

# Candidates below the threshold still occupy Git's four matrix slots. With
# descending low scores, source5 replaces slot3 and source6 replaces slot2, so
# stable sorting chooses source6 from the two equally good matches.
# A size-rejected first source ranks as zero instead, making source5 win.
for variant in below-threshold size-rejected; do
  begin_case "matrix-$variant"
  if [ "$variant" = below-threshold ]; then
    { lines common 4; lines other1 6; } >source1
  else
    printf 'x\n' >source1
  fi
  for i in 2 3 4 5 6; do
    count=$((5 - i))
    if [ "$i" -ge 5 ]; then count=9; fi
    { lines common "$count"; lines "other$i" "$((10 - count))"; } >"source$i"
  done
  before
  lines common 10 >destination
  rm source*
  after
done
