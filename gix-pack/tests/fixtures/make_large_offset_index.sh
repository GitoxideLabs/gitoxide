#!/usr/bin/env bash
set -eu -o pipefail

# `--index-version=2,0` makes Git store every offset of the pack `$1` in the 64-bit large-offset table,
# though the pack is far smaller than the 2GiB that needs one. Every lookup through `large-offsets.idx`
# therefore reads its offset from that table, as lookups in packs past 2GiB do.
input="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$1"
# Index within a repository of our own, so Git does not discover the checkout the fixture is created in.
git init -q --bare --object-format=sha1 git
git -C git index-pack --index-version=2,0 -o ../large-offsets.idx "$input" >/dev/null
