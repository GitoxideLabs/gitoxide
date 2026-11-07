#!/usr/bin/env bash
set -eu -o pipefail

# Git primes both FSMN and UNTR. Mixed directories prevent a cached listing
# which forgets tracked names from incorrectly collapsing to `mixed/`. The
# tracked-only branch has complete type information without per-file stat calls.
git init -q
mkdir -p mixed/deep stable untracked tracked-only/deep tracked-only/second
for path in mixed/tracked mixed/deep/tracked stable/tracked tracked-only/tracked tracked-only/deep/tracked tracked-only/second/tracked; do
  echo tracked >"$path"
done
echo '*.ignored' >.gitignore
git add .
git commit -qm initial
for path in mixed/visible mixed/deep/visible stable/visible untracked/visible; do
  echo untracked >"$path"
done
echo ignored >mixed/hidden.ignored
echo global >.git/global-excludes
echo ignored >global
git config core.excludesFile "$(pwd)/.git/global-excludes"
cat >.git/fsmonitor-hook <<'EOF'
#!/bin/sh
test "$1" = 2 || exit 1
if test -f .git/fsmonitor-token; then
  cat .git/fsmonitor-token
  printf '\000'
else
  printf 'next-token\000'
fi
cat .git/fsmonitor-paths
EOF
chmod +x .git/fsmonitor-hook
: >.git/fsmonitor-paths
git config core.fsmonitor .git/fsmonitor-hook
git config core.fsmonitorHookVersion 2
git config core.untrackedCache true
git status --porcelain >.git/expected-status
git status --porcelain >.git/expected-status
