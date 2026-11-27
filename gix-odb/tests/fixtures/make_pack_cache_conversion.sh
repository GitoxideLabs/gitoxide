#!/usr/bin/env bash
set -eu -o pipefail

# Both packs share their larger base blob, so the distinct delta blobs have matching offsets.
# Reversing pack-open order in a new store must not reuse cached bytes from the old store.
git init -q --bare
seq 1 1024 >common
base_id=$({ printf 'base\n'; cat common; printf 'padding to keep the shared base larger\n'; } | git hash-object -w --stdin)
for label in a b; do
    { printf '%s\n' "$label"; cat common; } >"$label"
    blob_id=$(git hash-object -w "$label")
    printf '%s\n' "$blob_id" >"$label.oid"
    printf '%s\n%s\n' "$base_id" "$blob_id" |
        git pack-objects --delta-base-offset --window=10 --depth=10 objects/pack/pack
done
git prune-packed
rm common
