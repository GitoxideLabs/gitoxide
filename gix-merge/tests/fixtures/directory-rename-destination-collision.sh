#!/usr/bin/env bash
set -eu

# A file rename and an inferred directory rename both target an existing
# directory. Other structural conflicts can consume that destination first.
git init repo
cd repo
mkdir -p a d/d
printf anchor >a/g
printf file >b
printf nested >d/d/d
git add .
git commit -m base
git branch base
git checkout -b ours
mkdir -p a/d
git mv d/d/d a/d/d
rmdir d/d d
printf anchor >a/a
printf anchor >d
git add .
git commit -m 'move directory into existing destination and replace its source'
git checkout -b theirs base
git rm a/g
git mv b a
mkdir -p b/a
printf fresh >b/a/f
printf anchor >d/a
chmod +x b/a/f d/a
git add .
git commit -m 'rename file to the shared destination and add descendants'
git rev-parse base^{tree} ours^{tree} theirs^{tree} >trees
