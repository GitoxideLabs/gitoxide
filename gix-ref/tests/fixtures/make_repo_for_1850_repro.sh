#!/usr/bin/env bash
set -eu -o pipefail

git init -q

cat <<EOF >.git/packed-refs
# pack-refs with: peeled fully-peeled sorted
17dad46c0ce3be4d4b6d45def031437ab2e40666 refs/heads/ig-branch-remote
83a70366fcc1255d35a00102138293bac673b331 refs/heads/ig-inttest
21b57230833a1733f6685e14eabe936a09689a1b refs/heads/ig-pr4021
d773228d0ee0012fcca53fffe581b0fce0b1dc56 refs/heads/ig/aliases
ba37abe04f91fec76a6b9a817d40ee2daec47207 refs/heads/ig/cifail
EOF

mkdir -p .git/refs/heads/ig/pr
echo d22f46f3d7d2504d56c573b5fe54919bd16be48a >.git/refs/heads/ig/push-name
echo 4dec145966c546402c5a9e28b932e7c8c939e01e >.git/refs/heads/ig-pr4021
