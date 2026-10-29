#!/usr/bin/env bash
set -eu -o pipefail

# Two malformed indices precede a healthy one in the ODB's descending-size order.
# Keep the second index's valid bytes so recovery can be tested while the first stays corrupt.
git init -q --bare
for label in healthy recoverable; do
    blob_id=$(printf '%s blob\n' "$label" | git hash-object -w --stdin)
    pack_id=$(printf '%s\n' "$blob_id" | git pack-objects objects/pack/pack)
    printf '%s\n' "$blob_id" >"$label-object-id"
    printf 'pack-%s.idx\n' "$pack_id" >"$label-index-name"
done
git prune-packed
recoverable_index=$(cat recoverable-index-name)
cp "objects/pack/$recoverable_index" recoverable.idx
cp "objects/pack/${recoverable_index%.idx}.pack" objects/pack/pack-broken.pack
rm "objects/pack/$recoverable_index"
dd if=/dev/zero of="objects/pack/$recoverable_index" bs=2048 count=1 2>/dev/null
dd if=/dev/zero of=objects/pack/pack-broken.idx bs=4096 count=1 2>/dev/null

# Git still reads the healthy pack after diagnosing the unrelated corrupt indices.
healthy_blob_id=$(cat healthy-object-id)
test "$(git cat-file blob "$healthy_blob_id")" = "healthy blob"
