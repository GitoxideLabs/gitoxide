#!/usr/bin/env bash
set -eu -o pipefail

# A staged rename within the visible cone must work with unrelated sparse directories.
git init -q
mkdir included excluded
printf 'first\nsecond\nthird\nfourth\n' > included/old.txt
printf 'hidden\n' > excluded/other.txt
git add .
git commit -qm root
git sparse-checkout init --cone --sparse-index
git sparse-checkout set included
git mv included/old.txt included/new.txt
printf 'first\nSECOND\nthird\nfourth\n' > included/new.txt
