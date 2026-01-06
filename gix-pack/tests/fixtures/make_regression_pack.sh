#!/usr/bin/env bash
set -eu -o pipefail

# This script creates a pack file specifically designed to trigger the buffer overflow
# bug fixed in PR #2345.

cleanup() {
  cd ..
  rm -rf regression-pack-repo
}

trap cleanup EXIT

mkdir -p regression-pack-repo
cd regression-pack-repo
git init -q
git config user.email "test@example.com"
git config user.name "Test User"

# Create a large base blob with highly compressible repetitive content
{
  for i in {1..100}; do
    echo "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    echo "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
    echo "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
    echo "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"
    echo "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE"
    echo "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
    echo "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG"
    echo "HHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH"
  done
} > largefile.txt
git add largefile.txt
git commit -qm "Add large base file"

# Create first delta - small change
sed -i '1s/AAAA/XXXX/g' largefile.txt
git add largefile.txt
git commit -qm "Delta 1"

# Create second delta - more small changes
sed -i '2s/BBBB/YYYY/g' largefile.txt
git add largefile.txt
git commit -qm "Delta 2"

# Create third delta to make a longer chain
sed -i '3s/CCCC/ZZZZ/g' largefile.txt
git add largefile.txt
git commit -qm "Delta 3"

# Create fourth delta for even longer chain
sed -i '4s/DDDD/WWWW/g' largefile.txt
git add largefile.txt
git commit -qm "Delta 4"

# Repack aggressively to create delta chains
git repack -adf --window=250 --depth=250

# Copy the pack file to the fixtures directory
PACK_FILE=$(ls .git/objects/pack/*.pack)
PACK_IDX=$(ls .git/objects/pack/*.idx)
PACK_HASH=$(basename "$PACK_FILE" .pack | sed 's/pack-//')

cp "$PACK_FILE" ../objects/pack-regression-$PACK_HASH.pack
cp "$PACK_IDX" ../objects/pack-regression-$PACK_HASH.idx

echo "Created pack files:"
echo "  pack-regression-$PACK_HASH.pack"
echo "  pack-regression-$PACK_HASH.idx"
echo ""
echo "Pack statistics:"
git verify-pack -v "$PACK_FILE" | head -20
