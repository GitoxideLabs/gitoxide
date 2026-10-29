#!/usr/bin/env bash
set -eu -o pipefail

# Fetch one pack now. The test opens its reader before the second fetch adds a
# pack for the new tip and prunes the branch that the remote deleted.
git init -q -b main remote
git -C remote commit -q --allow-empty -m first
git -C remote branch to-delete
git init -q client
git -C client remote add origin ../remote
git -C client config fetch.unpackLimit 1
git -C client config gc.auto 0
git -C client fetch -q origin
git -C remote commit -q --allow-empty -m second
git -C remote branch -D to-delete
