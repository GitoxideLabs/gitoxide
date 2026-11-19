#!/usr/bin/env bash
set -eu -o pipefail

# Keep protection and sharing settings in each repository-local source so opening can assign
# reduced trust without changing ownership or writing to the cached fixture.
for source in local worktree; do
  for value in false true invalid; do
    repo="$source-$value"
    git init -q "$repo"
    config="$repo/.git/config"
    if test "$source" = worktree; then
      git -C "$repo" config extensions.worktreeConfig true
      config="$repo/.git/config.worktree"
    fi
    cat >>"$config" <<EOF
[core]
  symlinks = $value
  protectHFS = $value
  protectNTFS = $value
  sharedRepository = group
[gitoxide "core"]
  protectWindows = $value
EOF
  done
done

# A file as the checkout path makes it impossible to create a link there on any platform.
# Protected configuration selects it while the local core.symlinks setting stays ineligible.
git init -q unavailable-checkout
git -C unavailable-checkout config core.symlinks true
touch unavailable-checkout/.git/checkout-file
git config --file unavailable-checkout/.git/checkout.config core.worktree checkout-file

# Provide a protected configuration source that tests can opt into while forcing
# reduced initial trust. The wildcard promotes every dynamically located fixture
# repository to full trust, making its local or worktree protection settings
# eligible before reference protection is initialized.
cat >trust-all-repositories.config <<EOF
[safe]
  directory = *
EOF
