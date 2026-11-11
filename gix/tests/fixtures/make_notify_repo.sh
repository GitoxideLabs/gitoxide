#!/usr/bin/env bash
set -eu -o pipefail

# These repositories have identical directories and index entries. Only the ignore
# rules differ, so notification registration counts can prove that ignore-aware
# coverage skips both levels of an ignored subtree without timing absent events.
git init -q ignored
git init -q unignored
for repo in ignored unignored; do
  (
    cd "$repo"
    mkdir -p tracked ignored/deep
    echo tracked >tracked/file
    echo untracked >ignored/deep/file
    git add tracked/file
  )
done
printf 'ignored/\n' >ignored/.gitignore
printf '# No ignored directories.\n' >unignored/.gitignore
