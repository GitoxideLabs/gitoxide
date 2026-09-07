#!/bin/sh
set -eu

# A and B disagree on `shared`; B also adds a clean file which must disappear
# when B is muted. C is independent, so it must still merge after muted B.
# Every input has a name: remerges follow refs, not the original commit IDs.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git config commit.gpgSign false
printf 'base\n' >shared
git add shared
git commit -qm base
git checkout -qb A
printf 'A\n' >shared
git commit -qam A
git checkout -qb B main
printf 'B\n' >shared
printf 'B only\n' >b
git add shared b
git commit -qm B
git checkout -qb C main
printf 'C\n' >c
git add c
git commit -qm C
git checkout -q A
