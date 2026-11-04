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

# The manually resolved worktree contains lines from both conflicting parents and a new line.
for scenario in merging renamed-merge; do
    git clone -q . "$scenario"
    (
        cd "$scenario"
        git checkout -qb ours
        path=old.txt
        if test "$scenario" = renamed-merge; then
            git mv old.txt renamed.txt
            path=renamed.txt
        fi
        printf 'first\nOURS\nthird\nfourth\n' > "$path"
        git commit -qam ours
        git checkout -qb incoming HEAD^
        printf 'first\nTHEIRS\nthird\nfourth\n' > old.txt
        git commit -qam theirs
        git checkout -q ours
        if git merge --no-commit incoming; then
            echo 'expected a conflict' >&2
            exit 1
        fi
        test -f .git/MERGE_HEAD
        printf 'first\nOURS\nTHEIRS\nfourth\nresolution\n' > "$path"
    )
done
git clone -q . invalid-merge
printf 'invalid commit\n' > invalid-merge/.git/MERGE_HEAD
