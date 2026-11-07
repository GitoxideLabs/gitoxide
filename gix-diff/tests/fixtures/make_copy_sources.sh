#!/usr/bin/env bash
set -eu -o pipefail

git init -q

# The modified source moves from index 1 to index 2 and loses its executable bit.
# Only its old content is eligible for copies; its new content is also added to
# expose implementations that accidentally match the destination-side version.
echo unrelated > aa-removed
echo original > z-source
chmod +x z-source
git add .
git write-tree > .git/modified-before.tree
cp z-source a-copy
chmod +x a-copy
echo replacement > z-source
chmod -x z-source
cp z-source b-new-copy
rm aa-removed
git add -A
git write-tree > .git/modified-after.tree
git diff-tree --no-commit-id -r --name-status -M100% -C100% \
  "$(cat .git/modified-before.tree)" "$(cat .git/modified-after.tree)" > .git/modified.git

git rm -q -rf .
# The two indices have different path backing. The unchanged symlink is an
# exact copy source under --find-copies-harder, just like a regular file.
echo unrelated > aa-removed
ln -s missing z-source
git add .
git write-tree > .git/harder-before.tree
rm aa-removed
ln -s missing a-copy
git add -A
git write-tree > .git/harder-after.tree
git diff-tree --no-commit-id -r --name-status -M100% -C100% --find-copies-harder \
  "$(cat .git/harder-before.tree)" "$(cat .git/harder-after.tree)" > .git/harder.git

record_before() {
  git add -A
  git write-tree > ".git/$1-before.tree"
}

record_after() {
  local case_name="$1"
  shift
  git add -A
  git write-tree > ".git/$case_name-after.tree"
  git diff-tree --no-commit-id -r --raw -z --no-abbrev --no-ext-diff --no-textconv \
    --no-rename-empty -l0 "$@" \
    "$(cat ".git/$case_name-before.tree")" "$(cat ".git/$case_name-after.tree")" > ".git/$case_name.raw"
}

# The identical old/ and a/ trees must not reserve their child as a rename:
# Git labels the last destination of the deleted source R100 and the first C100.
git rm -q -rf .
mkdir old
printf 'same content\n' >old/file
record_before directory-copy-exact
mkdir a b
cp old/file a/file
cp old/file b/other
git rm -q -rf old
record_after directory-copy-exact -M100% -C100%

# The last destination remains the rename when it is only an inexact match.
git rm -q -rf .
mkdir old
for ((i = 0; i < 16; i++)); do printf 'line-%02d\n' "$i"; done >old/file
record_before directory-copy-edited
mkdir a b
cp old/file a/file
{ head -n 14 old/file; printf 'edit-14\nedit-15\n'; } >b/other
git rm -q -rf old
record_after directory-copy-edited -M50% -C50%

# Duplicate leaves compete across identical directories. Git pairs old-a with
# the first destination, leaving old-b/file for the later basename match.
# Directory inference must follow that leaf choice instead of reserving old-a.
git rm -q -rf .
mkdir old-a old-b old-c
printf 'same content\n' >old-a/file
cp old-a/file old-b/file
cp old-a/file old-c/other
record_before directory-duplicate-sources
mkdir new-a new-b
cp old-a/file new-a/different
cp old-a/file new-b/file
git rm -q -rf old-a old-b old-c
record_after directory-duplicate-sources -M100%
