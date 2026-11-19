#!/usr/bin/env bash
set -eu -o pipefail

# Cache the source for shared worktree-add and status-index tests, which write
# only to disposable copies. Tracking a nested file exercises checkout directories.
source "$(dirname "${BASH_SOURCE[0]}")/make_basic_repo.sh"
git add some-with-file
git commit -m "track nested files for checkout permission checks"
case "$2" in
  common) git config core.sharedRepository "$1" ;;
  worktree)
    # The worktree-specific policy must override the common group policy.
    git config extensions.worktreeConfig true
    git config core.sharedRepository group
    git config --worktree core.sharedRepository "$1"
    ;;
  *)
    echo "unknown shared repository configuration scope: $2 (expected common or worktree)" >&2
    exit 1
    ;;
esac
