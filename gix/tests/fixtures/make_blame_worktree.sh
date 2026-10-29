#!/usr/bin/env bash
set -eu -o pipefail

# Blame compares the working file, not just its staged blob, when following a staged rename.
git init -q
printf 'first\nsecond\nthird\nfourth\n' > old.txt
git add old.txt
git commit -qm root
git clone -q . staged
(
    cd staged
    git mv old.txt new.txt
    printf 'first\nSECOND\nthird\nfourth\n' > new.txt
    git add new.txt
    printf 'first\nSECOND\nTHIRD\nfourth\n' > new.txt
    printf 'untracked\n' > untracked.txt
)
git clone -q . rewritten
(
    cd rewritten
    git mv old.txt new.txt
    printf 'everything changed\n' > new.txt
)
