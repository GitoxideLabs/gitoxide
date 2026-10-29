#!/usr/bin/env bash
set -eu -o pipefail

# Two unreachable blobs with different ages demonstrate that a cruft table is
# ordered by object ID, not time, and preserves mtimes across a repack.
export TZ=UTC
for format in sha1 sha256; do
    git init -q --bare --object-format="$format" "$format"
    (
        cd "$format"
        old=$(printf old | git hash-object -w --stdin)
        new=$(printf new | git hash-object -w --stdin)
        printf '%s\n' "$old" >old-id
        printf '%s\n' "$new" >new-id
        touch -t 200101010000.00 "objects/${old:0:2}/${old:2}"
        touch -t 200201010000.00 "objects/${new:0:2}/${new:2}"
        git repack --cruft -d
    )
done
