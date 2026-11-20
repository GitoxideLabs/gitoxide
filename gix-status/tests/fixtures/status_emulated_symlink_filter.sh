#!/usr/bin/env bash
set -eu -o pipefail

# Emulated symlinks contain the link target verbatim, even when text attributes
# would convert an ordinary file at the same path.
git init -q
git config core.symlinks false
printf '*.txt text eol=lf\n' >.gitattributes
git add .gitattributes
crlf_id=$(printf 'destination\r\n' | git hash-object -w --stdin)
lf_id=$(printf 'destination\n' | git hash-object -w --stdin)
git update-index --add --cacheinfo "120000,$crlf_id,clean.txt"
git update-index --add --cacheinfo "120000,$lf_id,changed.txt"
git checkout-index --all --force
printf 'destination\r\n' >changed.txt
