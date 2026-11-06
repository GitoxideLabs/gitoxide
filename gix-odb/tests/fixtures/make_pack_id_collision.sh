#!/usr/bin/env bash
set -eu -o pipefail

# These equal-length pack names have the same CRC32 under any common path prefix.
# Distinct one-blob packs make selecting the wrong pack by its path CRC observable.
git init -q --bare
for name in pack-446fae5b7acbf932 pack-da48020c4b9ad19b; do
    blob_id=$(printf '%s\n' "$name" | git hash-object -w --stdin)
    pack_hash=$(printf '%s\n' "$blob_id" | git pack-objects objects/pack/temporary)
    for extension in pack idx; do
        mv "objects/pack/temporary-$pack_hash.$extension" "objects/pack/$name.$extension"
    done
done
git prune-packed
