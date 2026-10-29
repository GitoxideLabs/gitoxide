#!/usr/bin/env bash
set -eu -o pipefail

# Reflog expiry uses entry timestamps, independently of object reachability.
# Keep a record exactly on the cutoff to exercise Git's strict comparison.
git init -q
for year in 2000 2001 2002; do
    GIT_COMMITTER_DATE="$year-01-01T00:00:00 +0000" git commit -q --allow-empty -m "$year"
done
