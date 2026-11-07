#!/usr/bin/env bash
set -eu

# Ignored files beside empty descendants keep their parent observable when
# untracked directories collapse. A pruned .git file must not obscure that case.
git init
mkdir -p parent/empty with-pruned/empty nested/parent/empty only-empty/empty leaf
printf '*.ignored\n' >.gitignore
touch parent/hidden.ignored with-pruned/hidden.ignored nested/parent/hidden.ignored
touch with-pruned/.git leaf/tracked leaf/untracked
git add .gitignore leaf/tracked
git commit -m initial
