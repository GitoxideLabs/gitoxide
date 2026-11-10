#!/usr/bin/env bash
set -eu

# Prepare both system-config states so the test never mutates its read-only fixture.
# A missing prefix config must still trigger a Git query, unlike a missing system config.
mkdir -p missing/etc prefix/etc existing/etc
touch existing/etc/gitconfig
