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

# A split directory follows the destination receiving most finalized file renames,
# even when another destination sorts first and a winning file also changes name.
git rm -q -rf .
mkdir old
printf 'one\n' >old/one
printf 'two\n' >old/two
printf 'three\n' >old/three
record_before directory-split
mkdir other winner
mv old/one winner/renamed
mv old/two winner/two
mv old/three other/three
rmdir old
record_after directory-split -M100%

# Equal support for two destinations does not establish a directory rename.
git rm -q -rf .
mkdir old
printf 'one\n' >old/one
printf 'two\n' >old/two
record_before directory-tie
mkdir first second
mv old/one first/one
mv old/two second/two
rmdir old
record_after directory-tie -M100%

# With empty-file tracking disabled, equal tree IDs cannot manufacture leaf renames.
git rm -q -rf .
mkdir old
touch old/empty
record_before directory-empty
mv old new
record_after directory-empty -M100%

# A leaf moving between different nested directories identifies their rename,
# but does not establish a rename of the roots above them.
git rm -q -rf .
mkdir -p old/sub
printf 'one\n' >old/sub/file
record_before directory-different-suffix
mkdir -p new/other
mv old/sub/file new/other/file
rmdir old/sub old
record_after directory-different-suffix -M100%

# An independently renamed exact subtree remains visible alongside its renamed
# root. The root-level child supports a/ -> b/ despite sub/ -> renamed/ below it.
git rm -q -rf .
mkdir -p a/sub
printf 'nested\n' >a/sub/file
printf 'root\n' >a/root
record_before directory-nested
mkdir b
mv a/sub b/renamed
mv a/root b/root
rmdir a
record_after directory-nested -M100%

# A removed root may move beneath a newly added root; parent/child relation
# labels must not restrict the inferred directory source and destination.
git rm -q -rf .
mkdir old
printf 'deeper\n' >old/file
record_before directory-deeper
mkdir new
mv old new/sub
record_after directory-deeper -M100%
