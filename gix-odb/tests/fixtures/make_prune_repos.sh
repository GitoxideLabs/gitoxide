#!/usr/bin/env bash
set -eu -o pipefail

# Empty bare repositories let the Rust tests control every object's timestamp
# and compare loose-object expiry with Git for both supported hash formats.
for format in sha1 sha256; do
    git init -q --bare --object-format="$format" "$format"
done
