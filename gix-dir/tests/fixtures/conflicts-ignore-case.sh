#!/usr/bin/env bash
set -eu -o pipefail

# The on-disk directories differ in case from both their indexed ancestor file
# and child. Case-insensitive cleanup must still descend to the tracked child.
git init -q
printf 'file-dir-*/\n' >.gitignore
git add .gitignore
blob_id=$(printf 'content\n' | git hash-object -w --stdin)
for stage in 1 2 3; do
  child_stage=3
  if [[ "$stage" == 3 ]]; then
    child_stage=2
  fi
  mkdir "file-dir-$stage"
  printf 'content\n' >"file-dir-$stage/bar"
  printf '100644 %s %s\tFILE-DIR-%s\n100644 %s %s\tFILE-DIR-%s/bar\n' \
    "$blob_id" "$stage" "$stage" "$blob_id" "$child_stage" "$stage"
done | git update-index --index-info
git -c core.ignoreCase=true clean -ndx >.git/baseline-clean
