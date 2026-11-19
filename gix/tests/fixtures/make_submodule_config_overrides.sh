#!/usr/bin/env bash
set -eu -o pipefail

# Repository-local overrides conflict with every supported .gitmodules setting,
# including a command update that must only be available to trusted sections.
# The second module uses the superproject's active pathspec and fetch fallback.
git init worktree
(cd worktree
  cat >.gitmodules <<EOF
[submodule "s"]
  path = s
  url = https://safe.example/s
  update = checkout
  ignore = none
  branch = main
  fetchRecurseSubmodules = true
[submodule "pattern"]
  path = pattern
  url = https://safe.example/pattern
EOF
  cat >>.git/config <<EOF

[fetch]
  recurseSubmodules = true
[submodule]
  active = pattern
[submodule "s"]
  active = true
  url = https://override.example/s
  update = !override
  ignore = all
  branch = override
  fetchRecurseSubmodules = false
EOF
  git add .gitmodules
  git commit -m modules
)

# Force each fallback source without mutating the shared fixture during tests.
cp -R worktree index
rm index/.gitmodules
cp -R index tree
rm tree/.git/index
