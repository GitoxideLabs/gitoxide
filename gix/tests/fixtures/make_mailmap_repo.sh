#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git checkout -b main

git config user.name "Committer"
git config user.email "committer@example.com"
git config commit.gpgsign false

# A .mailmap that remaps an author's display name by email.
# Format: <Proper Name> <commit@email>
cat >.mailmap <<'EOF'
Proper Name <proper@example.com>
EOF

touch a
git add a .mailmap
git commit -q -m "initial with mailmap"
