#!/usr/bin/env bash
set -eu -o pipefail

# Every path changes between two commits, forcing blame to compare blobs rather
# than merely assigning an unchanged file to its introducing commit. Cover
# binary data on either or both sides, an existing empty blob, and plain text
# whose driver can be configured by the test as external or binary-to-text.
git init -q
git checkout -q -b main
printf 'text.txt diff=custom\n' > .gitattributes
printf 'stable\nold\n' > text.txt
printf '\000old\n' > old-binary.txt
printf 'old\n' > new-binary.txt
printf '\000old\n' > both-binary.txt
: > empty-before.txt
git add .
git commit -q -m 'Create comparison sources'

printf 'stable\nnew\n' > text.txt
printf 'new\n' > old-binary.txt
printf '\000new\n' > new-binary.txt
printf '\000new\n' > both-binary.txt
printf 'new\n' > empty-before.txt
git add .
git commit -q -m 'Modify all comparison resources'

git blame --porcelain text.txt > .git/text.baseline
git blame --porcelain empty-before.txt > .git/empty-before.baseline

# Tests point diff worktree roots here to exercise absent resources without
# corrupting the object database or mutating a shared read-only fixture.
mkdir missing
