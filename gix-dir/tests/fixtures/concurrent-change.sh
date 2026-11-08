#!/usr/bin/env bash
set -eu -o pipefail

# The walk delegate changes `parent/gone` after classification, before reading it.
# A surviving sibling verifies traversal continues without collapsing the parent.
git init
mkdir -p parent/gone
touch parent/gone/file parent/keep
