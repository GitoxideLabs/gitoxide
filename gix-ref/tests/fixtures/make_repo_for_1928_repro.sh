#!/usr/bin/env bash
set -eu -o pipefail

git init -q

mkdir -p .git/refs/heads/a
cat <<EOF >.git/packed-refs
# pack-refs with: peeled fully-peeled sorted
1111111111111111111111111111111111111111 refs/heads/a-
2222222222222222222222222222222222222222 refs/heads/a/b
3333333333333333333333333333333333333333 refs/heads/a0
EOF

mkdir -p .git/refs/heads/a
echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa >.git/refs/heads/a-
echo bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb >.git/refs/heads/a/b
echo cccccccccccccccccccccccccccccccccccccccc >.git/refs/heads/a0
