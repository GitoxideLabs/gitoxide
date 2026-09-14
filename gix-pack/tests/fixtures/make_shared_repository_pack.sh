#!/usr/bin/env bash
set -eu -o pipefail

# Include the umask and sharing setting in the fixture arguments so cached modes match the test case.
umask "$1"
input="$(dirname "${BASH_SOURCE[0]}")/$3"
git -c "core.sharedRepository=$2" init --bare --object-format=sha1 git

# Receiving a pack must recreate a missing pack directory with the repository's sharing policy.
rmdir git/objects/pack
git -C git index-pack --stdin --keep='permissions baseline' < "$input"
