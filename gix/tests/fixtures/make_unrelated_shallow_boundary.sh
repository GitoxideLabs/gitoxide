#!/usr/bin/env bash
set -eu -o pipefail

# A malformed shallow commit outside the queried history must not be decoded.
# Write the commit-graph first so queries can also receive a pre-shallow cache.
git init -q
git checkout -q -b main
git commit -q --allow-empty -m parent
git commit -q --allow-empty -m child
git commit-graph write --no-progress --reachable
echo malformed | git hash-object --literally -t commit -w --stdin >.git/shallow
