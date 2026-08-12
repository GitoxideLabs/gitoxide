#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config --local diff.algorithm histogram

git checkout -q -b main

seq 1 4 > target.txt
git add target.txt
git commit -q -m c1-create-target

ln -s target.txt symlink
git add symlink
git commit -q -m c2-create-symlink

seq 1 6 > target.txt
git add target.txt
git commit -q -m c3-change-target

seq 2 5 > second-target.txt
ln -s second-target.txt symlink-changing-target
git add second-target.txt symlink-changing-target
git commit -q -m c4-create-second-target

rm symlink-changing-target
ln -s target.txt symlink-changing-target
git add symlink-changing-target
git commit -q -m c5-change-symlink-target

ln -s target.txt symlink-before-rename
git add symlink-before-rename
git commit -q -m c6-create-symlink-before-rename

rm symlink-before-rename
ln -s target.txt symlink-after-rename
git add symlink-after-rename symlink-before-rename
git commit -q -m c7-rename-symlink

seq 1 3 > file-then-symlink
git add file-then-symlink
git commit -q -m c8-create-file

rm file-then-symlink
ln -s target.txt file-then-symlink
git add file-then-symlink
git commit -q -m c9-change-file-to-symlink

git blame --porcelain symlink > .git/symlink.baseline
git blame --porcelain symlink-changing-target > .git/symlink-changing-target.baseline
git blame --porcelain symlink-after-rename > .git/symlink-renamed.baseline
git blame --porcelain file-then-symlink > .git/file-becomes-symlink.baseline
