#!/usr/bin/env bash
set -eu -o pipefail

git init -q

echo "line 1" >> simple.txt
git add simple.txt
git commit -q -m c1

echo "line 2" >> simple.txt
git add simple.txt
git commit -q -m c2

echo "line 3" >> simple.txt
git add simple.txt
git commit -q -m c3

echo "line 4" >> simple.txt
git add simple.txt
git commit -q -m c4
