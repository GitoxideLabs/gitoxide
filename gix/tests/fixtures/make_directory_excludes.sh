#!/usr/bin/env bash
set -eu -o pipefail

# Directory-only rules must match absent paths when a trailing slash identifies a directory.
git init -q
echo target/ >.gitignore

# Only the trailing-slash query should match, even though neither path exists.
git check-ignore --no-index absent/target/ absent/target >.git/check-ignore.baseline
