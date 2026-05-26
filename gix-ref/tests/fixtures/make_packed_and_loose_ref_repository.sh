#!/usr/bin/env bash
set -eu -o pipefail

git init -q

git checkout -q -b main
git commit -q --allow-empty -m c1   # C1

# Branches that get compacted into packed-refs while still at C1.
git branch both
git branch stable

git pack-refs --all                  # main, both, stable -> packed at C1, loose files removed

git commit -q --allow-empty -m c2    # C2; writes a fresh loose .git/refs/heads/main = C2

# 'both' is now packed at C1 *and* loose at C2 -> exercises loose-over-packed precedence.
git update-ref refs/heads/both HEAD  # loose .git/refs/heads/both = C2 (packed still C1)

# 'fresh' is loose-only (never packed).
git branch fresh                     # loose .git/refs/heads/fresh = C2

# 'stable' is left untouched: packed-only at C1, no loose file.
