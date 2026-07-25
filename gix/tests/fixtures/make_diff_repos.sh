#!/usr/bin/env bash
set -eu -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

git init jj-trackcopy-1
(cd jj-trackcopy-1
  # These assets are named after their Git object IDs, which were computed with LF
  # line endings. Normalize them explicitly as Git may check them out with CRLF on
  # Windows, changing blob IDs and adding carriage returns to index paths.
  index=.git/index
  for blob in "$ROOT"/assets/jj-trackcopy-1/*.blob; do
    sed $'s/\r$//' "$blob" | git hash-object -w -t blob --stdin
  done
  rm -f "$index"
  sed $'s/\r$//' "$ROOT/assets/jj-trackcopy-1/2de73f57fc9599602e001fc6331034749b2eacb0.tree" |
    git update-index --index-info
  sed $'s/\r$//' "$ROOT/assets/jj-trackcopy-1/2de73f57fc9599602e001fc6331034749b2eacb0.msg" |
    git commit --allow-empty -F -
  rm -f "$index"
  sed $'s/\r$//' "$ROOT/assets/jj-trackcopy-1/47bd6f4aa4a7eeef8b01ce168c6c771bdfffcbd3.tree" |
    git update-index --index-info
  sed $'s/\r$//' "$ROOT/assets/jj-trackcopy-1/47bd6f4aa4a7eeef8b01ce168c6c771bdfffcbd3.msg" |
    git commit --allow-empty -F -

  git checkout -f HEAD
  git mv cli c
  git commit -m "renamed cli to c"

  rm -Rf c/
)
