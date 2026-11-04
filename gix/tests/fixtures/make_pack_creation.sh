#!/usr/bin/env bash
set -eu -o pipefail

# Pack related blobs as deltas, leave one loose object, and install a replacement.
# Selecting only a delta must still produce a self-contained pack with original data.
git init -q
for version in 1 2 3; do
  for number in $(seq 1 200); do
    printf 'common content line %s\n' "$number"
  done >file
  printf 'version %s\n' "$version" >>file
  git add file
  git commit -qm "version $version"
done
git repack -adf --window=10 --depth=10
git rev-list --objects --all >objects
printf 'loose content\n' | git hash-object -w --stdin >loose-id
original=$(git rev-parse HEAD:file)
replacement=$(cat loose-id)
git replace "$original" "$replacement"
printf '%s\n' "$original" >original-id
