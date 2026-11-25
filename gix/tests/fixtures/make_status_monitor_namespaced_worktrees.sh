#!/usr/bin/env bash
set -eu -o pipefail

# Reuse sibling checkout registrations. Only the namespace's branch has a newer
# tracked-file tree, so selecting that namespace must invalidate cached status.
source "$(dirname "$0")/make_notify_worktrees.sh"
ordinary_commit=$(git -C main rev-parse HEAD)
printf 'namespaced contents\n' >main/tracked/file
git -C main add tracked/file
git -C main commit -qm 'namespace status baseline'
git -C main update-ref refs/namespaces/status/refs/heads/main HEAD
git -C main symbolic-ref refs/namespaces/status/HEAD refs/heads/main
git -C main reset -q --hard "$ordinary_commit"
