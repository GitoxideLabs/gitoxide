#!/bin/sh
set -eu

# Textconv doubles text lines and extracts a line from binary data.
# Stats must describe the Git-form content while patches still show the conversion.
git init -q -b main .
git config diff.expanded.textconv 'sed p'
git config diff.decoded.textconv "tr '\\000' '\\n' <"
printf 'sample.txt diff=expanded\nsample.bin diff=decoded\n' >.gitattributes
printf 'before\n' >sample.txt
printf 'before\000' >sample.bin
git add .
git commit -qm before
printf 'after\n' >sample.txt
printf 'after\000' >sample.bin
git add sample.txt sample.bin
git commit -qm after

# Leave both staged and unstaged versions to exercise the Worktree stats too.
printf 'staged\n' >sample.txt
printf 'staged\000' >sample.bin
git add sample.txt sample.bin
printf 'worktree\n' >sample.txt
printf 'worktree\000' >sample.bin
