#!/usr/bin/env bash
set -eu -o pipefail

git init -q
# Legacy subsection names are folded to lowercase, unlike quoted names.
cat >>.git/config <<'EOF'
[remote.Origin]
    url = https://example.com/legacy.git
[branch.Main]
    remote = origin
EOF
git remote >remote-names
git remote get-url origin >remote-url
