#!/usr/bin/env bash
set -eu -o pipefail

# Create a repository whose blob was stored without clean filters. On checkout,
# the unavailable encoding must be skipped while the CRLF conversion still runs.
git init -q remote
(
  cd remote
  printf 'file.txt text eol=crlf working-tree-encoding=definitely-not-an-encoding\n' >.gitattributes
  printf 'one\ntwo\n' >file.txt
  git add .gitattributes
  blob_id=$(git hash-object --no-filters -w file.txt)
  git update-index --add --cacheinfo 100644 "$blob_id" file.txt
  git commit -qm 'unknown worktree encoding'
)
