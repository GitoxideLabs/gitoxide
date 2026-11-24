#!/usr/bin/env bash
set -eu -o pipefail

input="$(dirname "${BASH_SOURCE[0]}")/$1"
cp "$input" large-offsets.pack
# The first object starts at byte 12. Encode all subsequent offsets in the 64-bit table.
git index-pack --object-format=sha1 --index-version=2,12 large-offsets.pack
git verify-pack --object-format=sha1 large-offsets.idx
