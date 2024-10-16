#!/usr/bin/env bash
set -eu -o pipefail

git config --local diff.algorithm histogram

git init -q
git config merge.ff false

git checkout -q -b main

echo "line 1" >> simple.txt
git add simple.txt
git commit -q -m c1

echo -e "line 1\nline 2\nline 3" >> multiline-hunks.txt
git add multiline-hunks.txt
git commit -q -m c1.1

echo -e "line 1\nline 2" > changed-lines.txt
echo -e "line 1\nline 2\nline 3\nline 4\nline 5\nline 6" >> changed-line-between-unchanged-lines.txt
git add changed-lines.txt
git add changed-line-between-unchanged-lines.txt
git commit -q -m c1.2

echo "line 2" >> added-lines.txt
echo "line 2" >> added-lines-around.txt
echo -e "line 1\nline 2" > coalesce-adjacent-hunks.txt
git add added-lines.txt
git add added-lines-around.txt
git add coalesce-adjacent-hunks.txt
git commit -q -m c1.3

echo "line 2" >> simple.txt
git add simple.txt
git commit -q -m c2

echo -e "line 4\nline 5\nline 6" >> multiline-hunks.txt
git add multiline-hunks.txt
git commit -q -m c2.1

echo -e "line 1\nline 2\nline 3\nline 4\nline 5\nline 6" >> deleted-lines.txt
echo -e "line 1\nline 2\nline 3\nline 4\nline 5\nline 6" >> deleted-lines-multiple-hunks.txt
git add deleted-lines.txt
git add deleted-lines-multiple-hunks.txt
git commit -q -m c2.2

echo -e "line 1\nline 2\nline 3" > added-line-before-changed-line.txt
git add added-line-before-changed-line.txt
git commit -q -m c2.3

echo -e "line 1\nline 2" > same-line-changed-twice.txt
echo -e "line 1\nline in between\nline 2" > coalesce-adjacent-hunks.txt
git add same-line-changed-twice.txt
git add coalesce-adjacent-hunks.txt
git commit -q -m c2.4

echo "line 3" >> simple.txt
git add simple.txt
git commit -q -m c3

echo -e "line 3\nline 4" > deleted-lines.txt
echo -e "line 2\nline 4" > deleted-lines-multiple-hunks.txt
git add deleted-lines.txt
git add deleted-lines-multiple-hunks.txt
git commit -q -m c3.1

echo -e "line 3\nline 4" > changed-lines.txt
echo -e "line 1\nline 2\nline 3 changed\nline 4\nline 5\nline 6" > changed-line-between-unchanged-lines.txt
git add changed-lines.txt
git add changed-line-between-unchanged-lines.txt
git commit -q -m c3.2

echo -e "line 2\nline 3" > added-line-before-changed-line.txt
echo -e "line 1\nline 2" > coalesce-adjacent-hunks.txt
git add added-line-before-changed-line.txt
git add coalesce-adjacent-hunks.txt
git commit -q -m c3.3

echo -e "line 1\nline 2 changed" > same-line-changed-twice.txt
git add same-line-changed-twice.txt
git commit -q -m c3.4

echo "line 4" >> simple.txt
git add simple.txt
git commit -q -m c4

echo -e "line 7\nline 8\nline 9" >> multiline-hunks.txt
git add multiline-hunks.txt
git commit -q -m c4.1

echo -e "line 1\nline 3\nline 2\nline 4" > switched-lines.txt
git add switched-lines.txt
git commit -q -m c4.2

echo -e "line 2 changed\nline 3" > added-line-before-changed-line.txt
git add added-line-before-changed-line.txt
git commit -q -m c4.3

echo -e "line 1\nline 2 changed a second time" > same-line-changed-twice.txt
git add same-line-changed-twice.txt
git commit -q -m c4.4

echo -e "  line 1\n\n  line 2\n\n  line 3" > empty-lines-histogram.txt
cp empty-lines-histogram.txt empty-lines-myers.txt
git add empty-lines-histogram.txt empty-lines-myers.txt
git commit -q -m c4.5

echo -e "line 0\nline 1\nline 2" > added-lines.txt
echo -e "line 0\nline 1\nline 2\nline 3" > added-lines-around.txt
git add added-lines.txt
git add added-lines-around.txt
git commit -q -m c5

echo -e "line 4" > deleted-lines.txt
git add deleted-lines.txt
git commit -q -m c5.1

echo -e "line 1\nline 2\nline 3\nline 4" > switched-lines.txt
git add switched-lines.txt
git commit -q -m c5.2

echo -e "line 1\nline 2 changed\nline 3" > added-line-before-changed-line.txt
git add added-line-before-changed-line.txt
git commit -q -m c5.3

echo -e "  line 1\n\n  line in between\n\n  line 2\n\n  line in between\n\n  line 3" > empty-lines-histogram.txt
cp empty-lines-histogram.txt empty-lines-myers.txt
git add empty-lines-histogram.txt empty-lines-myers.txt
git commit -q -m c5.4

# The commit history created by the commits above this line is linear, it only
# contains commits that have exactly one parent.
# Below this line, there’s also commits that have more than one parent.

echo -e "line 1 original\nline 2\n line 3" > resolved-conflict.txt
git add resolved-conflict.txt
git commit -q -m c6

echo -e "line 1 changed\nline 2\n line 3" > resolved-conflict.txt
git add resolved-conflict.txt
git commit -q -m c7

git checkout -b different-branch-to-create-a-conflict
git reset --hard HEAD~1

echo -e "line 1 changed in a different way\nline 2\n line 3" > resolved-conflict.txt
git add resolved-conflict.txt
git commit -q -m c8

git checkout main
git merge different-branch-to-create-a-conflict || true

echo -e "line 1 conflict resolved\nline 2\n line 3" > resolved-conflict.txt
git add resolved-conflict.txt
git commit -q -m c9

echo -e "line 1\nline 2\n line 3" > file-in-one-chain-of-ancestors.txt
git add file-in-one-chain-of-ancestors.txt
git commit -q -m c10

git checkout -b different-branch-that-does-not-contain-file
git reset --hard HEAD~1

echo -e "line 4\nline 5\n line 6" > different-file-in-another-chain-of-ancestors.txt
git add different-file-in-another-chain-of-ancestors.txt
git commit -q -m c11

git checkout main
git merge different-branch-that-does-not-contain-file || true

echo -e "line 1\nline 2\n line 3" > file-only-changed-in-branch.txt
git add file-only-changed-in-branch.txt
git commit -q -m c12

git checkout -b branch-that-has-one-commit

echo -e "line 1 changed\nline 2\n line 3" > file-only-changed-in-branch.txt
git add file-only-changed-in-branch.txt
git commit -q -m c13

git checkout main
git merge branch-that-has-one-commit || true

git blame --porcelain simple.txt > .git/simple.baseline
git blame --porcelain multiline-hunks.txt > .git/multiline-hunks.baseline
git blame --porcelain deleted-lines.txt > .git/deleted-lines.baseline
git blame --porcelain deleted-lines-multiple-hunks.txt > .git/deleted-lines-multiple-hunks.baseline
git blame --porcelain changed-lines.txt > .git/changed-lines.baseline
git blame --porcelain changed-line-between-unchanged-lines.txt > .git/changed-line-between-unchanged-lines.baseline
git blame --porcelain added-lines.txt > .git/added-lines.baseline
git blame --porcelain added-lines-around.txt > .git/added-lines-around.baseline
git blame --porcelain switched-lines.txt > .git/switched-lines.baseline
git blame --porcelain added-line-before-changed-line.txt > .git/added-line-before-changed-line.baseline
git blame --porcelain same-line-changed-twice.txt > .git/same-line-changed-twice.baseline
git blame --porcelain coalesce-adjacent-hunks.txt > .git/coalesce-adjacent-hunks.baseline

git blame --porcelain resolved-conflict.txt > .git/resolved-conflict.baseline
git blame --porcelain file-in-one-chain-of-ancestors.txt > .git/file-in-one-chain-of-ancestors.baseline
git blame --porcelain different-file-in-another-chain-of-ancestors.txt > .git/different-file-in-another-chain-of-ancestors.baseline
git blame --porcelain file-only-changed-in-branch.txt > .git/file-only-changed-in-branch.baseline

git blame --porcelain empty-lines-histogram.txt > .git/empty-lines-histogram.baseline

git config --local diff.algorithm myers

git blame --porcelain empty-lines-myers.txt > .git/empty-lines-myers.baseline
