#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config --local diff.algorithm histogram

git config merge.ff false

git checkout -q -b main

seq 1 4 > before-rename.txt
git add before-rename.txt
git commit -q -m c1

mv before-rename.txt after-rename.txt
git add before-rename.txt after-rename.txt
git commit -q -m c2

seq 1 5 > after-rename.txt
git add after-rename.txt
git commit -q -m c3

git checkout -b different-branch
git reset --hard HEAD~2

seq 0 4 > before-rename.txt
git add before-rename.txt
git commit -q -m c10

mv before-rename.txt after-rename.txt
git add before-rename.txt after-rename.txt
git commit -q -m c11

git checkout main
git merge different-branch || true

git blame --porcelain after-rename.txt > .git/after-rename.baseline

echo -e "1\n2\n3\n4\n5\n" > change-and-rename.txt
git add change-and-rename.txt
git commit -q -m c2.1.1

echo -e "1\ntwo\n3\n4\n5\n" > change-and-rename.txt
git add change-and-rename.txt
git commit -q -m c2.1.2

git checkout -b branch-that-renames-file
git reset --hard HEAD~1

echo -e "1\n2\n3\nfour\n5\n" > change-and-rename.txt
git add change-and-rename.txt
git commit -q -m c2.2.1

mv change-and-rename.txt change-and-renamed.txt
git add change-and-rename.txt change-and-renamed.txt
git commit -q -m c2.2.2

echo -e "1\n2\n3\nfour\nfive\n" > change-and-renamed.txt
git add change-and-renamed.txt
git commit -q -m c2.2.2

git checkout main
git merge branch-that-renames-file || true
git add change-and-rename.txt
git commit --no-edit

git blame --porcelain change-and-renamed.txt > .git/change-and-renamed.baseline
