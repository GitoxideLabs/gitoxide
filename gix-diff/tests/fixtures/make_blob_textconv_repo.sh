#!/usr/bin/env bash
set -eu -o pipefail

# Exercise the `gix-diff` filter pipeline with a binary file and `textconv` filter.
git init -q

git config --local diff.bin.textconv "tr '\\000' '\\n' <"
git config --local diff.bin.binary true
git config --local diff.bin.algorithm histogram

git checkout -q -b main

cat >.gitattributes <<'EOF'
sample.bin diff=bin
EOF
git add .gitattributes

printf '%s\0' 3 4 > sample.bin
git add sample.bin
git commit -q -m c1-create-binary-file

git rev-parse HEAD:sample.bin > new-file.id

printf '%s\0' 1 2 3 4 5 6 > sample.bin
git add sample.bin
git commit -q -m c2-change-binary-file

git rev-parse HEAD:sample.bin > changed-file.id

git diff --textconv HEAD~1 HEAD -- sample.bin > baseline.diff
