#!/bin/sh
set -eu

# Neither destination wins the old directory's split, so replaying the
# addition needs a placement decision even though all files have stage zero.
git init -q -b main .
mkdir old
printf 'first\n' >old/first
printf 'second\n' >old/second
git add old
git commit -qm base
git branch topic
mkdir left right
git mv old/first left/
git mv old/second right/
git commit -qm split
git switch -q topic
printf 'added\n' >old/added
git add old/added
git commit -qm add
