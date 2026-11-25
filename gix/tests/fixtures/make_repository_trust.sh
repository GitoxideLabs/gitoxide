#!/usr/bin/env bash
set -eu -o pipefail

# A configured checkout differs from the inferred one. The missing checkout
# deterministically fails the late ownership check without needing chown.
mkdir checkout
for repo in promoted demoted declared-bare; do
  git init -q "$repo"
  git config --file trusted-git-dirs.config --add safe.directory "$(git -C "$repo" rev-parse --absolute-git-dir)"
  git config --file trusted-worktrees.config --add safe.directory "$(git -C "$repo" rev-parse --show-toplevel)"
  git -C "$repo" config core.sshCommand local-program
  git -C "$repo" config extensions.worktreeConfig true
  git -C "$repo" config --worktree gitoxide.core.refsNamespace local
done
mkdir promoted/subdir
# Discovery sees the checkout of declared-bare before core.bare suppresses it.
git config --file declared-bare/.git/config core.bare true
git init --bare -q bare.git
git config --file trusted-git-dirs.config --add safe.directory "$(git -C bare.git rev-parse --absolute-git-dir)"

# The candidate is recognizable, but discovery cannot check ownership of its common directory.
git init --bare -q missing-common.git
echo missing >missing-common.git/commondir

git config --file promoted/.git/config.worktree core.worktree ../../checkout
git config --file demoted/.git/config.worktree core.worktree ../../missing
git config --file demoted/.git/config.worktree gitoxide.objects.cacheLimit invalid

cat >fallback.config <<EOF
[core]
  sshCommand = system-program
[gitoxide "core"]
  refsNamespace = system
EOF
cat fallback.config >>trusted-worktrees.config
