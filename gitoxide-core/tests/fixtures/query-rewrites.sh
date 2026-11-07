#!/usr/bin/env bash
set -eu -o pipefail

# Query needs line counts for edited text renames, while exact rewrites and binary
# renames have none. Large text is still recognized as a rename, but its line diff
# respects core.bigFileThreshold. Distinct content keeps each pair unambiguous.
# Copies stay exact-only even though rename detection accepts edited content.
git init -q
git config core.bigFileThreshold 1k
for i in {1..20}; do
  printf 'text line %02d\n' "$i" >>text
  printf 'copy source %02d\n' "$i" >>source
  printf 'binary record %02d\0\n' "$i" >>binary
  printf 'large text record %02d with enough content to exceed the diff size limit\n' "$i" >>large
done
printf 'exact rename\n' >exact
git add .
git commit -qm before

git mv text renamed-text
printf 'additional text line\n' >>renamed-text
git mv binary renamed-binary
printf 'additional binary record\0\n' >>renamed-binary
git mv exact renamed-exact
git mv large renamed-large
printf 'additional large text line\n' >>renamed-large
cp source copied-exact
cp source copied-edited
printf 'additional copy line\n' >>copied-edited
printf 'replacement source\n' >source
git add .
git commit -qm after
