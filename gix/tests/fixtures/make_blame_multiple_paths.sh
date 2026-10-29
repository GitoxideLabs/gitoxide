#!/usr/bin/env bash
set -eu -o pipefail

# Both parents rename a different file from the same root commit to merged.txt.
# The resolution takes adjacent source ranges from these distinct original files.
git init -q
printf 'a1\na2\nx3\nx4\n' > a.txt
printf 'y1\ny2\nb3\nb4\n' > b.txt
git add .
git commit -qm root
git tag root
git checkout -qb ours
git mv a.txt merged.txt
git rm -q b.txt
git commit -qm ours
git checkout -qb theirs root
git mv b.txt merged.txt
git rm -q a.txt
git commit -qm theirs
git checkout -q ours
if git merge --no-commit theirs; then
    echo 'expected a conflict' >&2
    exit 1
fi
test -f .git/MERGE_HEAD
printf 'a1\na2\nb3\nb4\n' > merged.txt
