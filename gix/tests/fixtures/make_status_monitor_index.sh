#!/usr/bin/env bash
set -eu -o pipefail

# The first and last entries expose index-position and path-backing changes when
# another path is staged. Old worktree mtimes keep unrelated entries non-racy.
git init -q --initial-branch=main
echo early >a
echo initial >tracked
echo last >z
git add a tracked z
git commit -qm initial
touch -t 200001010000 a tracked z
git update-index --refresh
