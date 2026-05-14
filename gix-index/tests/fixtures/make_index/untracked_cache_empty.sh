#!/usr/bin/env bash
set -eu -o pipefail

. "$(dirname -- "${BASH_SOURCE[0]}")/shared.sh"

git init -q

mkdir tracked-dir untracked-dir-2 untracked-dir-3
touch tracked-root-one tracked-root-two untracked-root-file \
    tracked-dir/tracked-file \
    untracked-dir-2/untracked-file-two \
    untracked-dir-3/untracked-file-three
git add tracked-root-one tracked-root-two tracked-dir/tracked-file
: >.git/info/exclude
git update-index --untracked-cache
seed_untracked_cache_times \
    . \
    .git/info/exclude \
    tracked-dir \
    tracked-dir/tracked-file \
    untracked-dir-2 \
    untracked-dir-2/untracked-file-two \
    untracked-dir-3 \
    untracked-dir-3/untracked-file-three \
    tracked-root-one \
    tracked-root-two \
    untracked-root-file
