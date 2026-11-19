#!/usr/bin/env bash
set -eu -o pipefail

# The umask is a fixture argument so cached Git modes still match this test run.
# Keep baselines read-only: copying would lose shared directory modes.
umask "$1"
source "$(dirname "${BASH_SOURCE[0]}")/make_basic_repo.sh"
for shared in group 0640 false; do
  # Older Git reads sharing during initialization, before clone's --config option.
  git -c "core.sharedRepository=$shared" clone --no-local . "git-$shared"
done
