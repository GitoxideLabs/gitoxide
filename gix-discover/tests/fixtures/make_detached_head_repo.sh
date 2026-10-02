#!/usr/bin/env bash
set -eu -o pipefail

# A detached HEAD contains a full commit hash rather than a symbolic reference,
# but discovery must still recognize it.
git init -q
git commit -q --allow-empty -m initial
git checkout -q --detach HEAD
