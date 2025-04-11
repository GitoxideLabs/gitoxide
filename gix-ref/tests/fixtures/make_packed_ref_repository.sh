#!/usr/bin/env bash
set -eu -o pipefail

git init -q

git checkout -q -b main
git commit -q --allow-empty -m c1
git branch dt1
git branch d1
git branch A

mkdir -p .git/refs/remotes/origin
mkdir -p .git/refs/prefix/feature/sub/dir

cp .git/refs/heads/main .git/refs/remotes/origin/
cp .git/refs/heads/main .git/refs/d1
cp .git/refs/heads/main .git/refs/prefix/feature-suffix
cp .git/refs/heads/main .git/refs/prefix/feature/sub/dir/algo

echo "ref: refs/remotes/origin/main" > .git/refs/remotes/origin/HEAD
echo "notahexsha" > .git/refs/broken

echo "ref: refs/heads/multi-link-target1" > .git/refs/multi-link
echo "ref: refs/tags/multi-link-target2" > .git/refs/heads/multi-link-target1
echo "ref: refs/remotes/origin/multi-link-target3" > .git/refs/tags/multi-link-target2
git rev-parse HEAD > .git/refs/remotes/origin/multi-link-target3


echo "ref: refs/loop-b" > .git/refs/loop-a
echo "ref: refs/loop-a" > .git/refs/loop-b

git tag t1
git tag -m "tag object" dt1

git pack-refs --all --prune
