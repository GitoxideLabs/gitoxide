#!/usr/bin/env bash
set -eu -o pipefail

# Build a compact catalogue of independent packs for dynamic ODB tests.
#
# The active object databases intentionally start empty. Tests copy individual
# catalogue components into them to reproduce pack publication, maintenance,
# removal, MIDX, and alternates transitions one filesystem step at a time.
git init -q --bare source.git
mkdir -p primary/objects/pack primary/objects/info
mkdir -p alternate/objects/pack alternate/objects/info
mkdir -p catalog/a catalog/b catalog/c

for n in $(seq 0 511); do
    printf 'odb scenario object %04d\n' "$n" |
        git --git-dir=source.git hash-object -w --stdin
done >all-ids

collision_prefix=$(
    cut -c1-4 all-ids |
        sort |
        uniq -d |
        sed -n '1p'
)
test -n "$collision_prefix"

grep "^$collision_prefix" all-ids | sed -n '1,2p' >collision-ids
collision_a=$(sed -n '1p' collision-ids)
collision_b=$(sed -n '2p' collision-ids)

grep -v -f collision-ids all-ids | sed -n '1,46p' >remaining-ids
{
    echo "$collision_a"
    sed -n '1,15p' remaining-ids
} >catalog/a/objects
{
    echo "$collision_b"
    sed -n '16,30p' remaining-ids
} >catalog/b/objects
sed -n '31,46p' remaining-ids >catalog/c/objects

for label in a b c; do
    pack_hash=$(
        git --git-dir=source.git pack-objects \
            --window=0 \
            --depth=0 \
            "catalog/$label/pack" <"catalog/$label/objects"
    )
    echo "pack $label pack-$pack_hash" >>manifest
    while read -r oid; do
        echo "object $label $oid" >>manifest
    done <"catalog/$label/objects"
done

echo "hash $(git --git-dir=source.git rev-parse --show-object-format)" >>manifest
echo "ambiguous $collision_prefix $collision_a $collision_b" >>manifest

rm -rf source.git all-ids collision-ids remaining-ids
