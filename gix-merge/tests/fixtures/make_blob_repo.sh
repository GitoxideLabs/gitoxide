#!/usr/bin/env bash
set -eu -o pipefail

git init -q

echo just-set > just-set
echo b > b
echo union > union
echo e > e-no-attr
echo unset > unset
echo unspecified > unspecified
# A zero-byte file must remain distinct from an absent worktree resource.
: > empty

cat <<EOF >.gitattributes
just-set merge conflict-marker-size=32
b merge=b
union merge=union
missing merge=missing
unset -merge
unspecified !merge
EOF

git add . && git commit -m "init"
