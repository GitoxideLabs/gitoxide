#!/usr/bin/env bash
set -eu -o pipefail

. "$(dirname -- "${BASH_SOURCE[0]}")/shared.sh"

GIT_FORCE_UNTRACKED_CACHE=true
export GIT_FORCE_UNTRACKED_CACHE

git init -q
git config core.excludesFile ""

# This fixture extends the populated case with a tracked per-directory
# `.gitignore` and nested untracked directories. The path names are intentionally
# verbose because the test snapshots assert the decoded UNTR directory graph.
mkdir -p tracked-dir-with-ignore/nested-untracked-dir/deep-untracked-dir \
    untracked-dir-2 \
    untracked-dir-3
touch tracked-root-one tracked-root-two untracked-root-file \
    tracked-dir-with-ignore/tracked-file \
    tracked-dir-with-ignore/visible-untracked-file \
    tracked-dir-with-ignore/nested-untracked-dir/deep-untracked-dir/deep-untracked-file \
    untracked-dir-2/untracked-file-two \
    untracked-dir-3/untracked-file-three
printf "ignored-by-dir-ignore\nalso-ignored-by-dir-ignore\n" >tracked-dir-with-ignore/.gitignore
git add tracked-root-one tracked-root-two tracked-dir-with-ignore/tracked-file tracked-dir-with-ignore/.gitignore
mkdir -p .git/info
: >.git/info/exclude
git update-index --untracked-cache
seed_untracked_cache_times \
    . \
    .git/info/exclude \
    tracked-dir-with-ignore \
    tracked-dir-with-ignore/.gitignore \
    tracked-dir-with-ignore/tracked-file \
    tracked-dir-with-ignore/visible-untracked-file \
    tracked-dir-with-ignore/nested-untracked-dir \
    tracked-dir-with-ignore/nested-untracked-dir/deep-untracked-dir \
    tracked-dir-with-ignore/nested-untracked-dir/deep-untracked-dir/deep-untracked-file \
    untracked-dir-2 \
    untracked-dir-2/untracked-file-two \
    untracked-dir-3 \
    untracked-dir-3/untracked-file-three \
    tracked-root-one \
    tracked-root-two \
    untracked-root-file
git status --porcelain >/dev/null
