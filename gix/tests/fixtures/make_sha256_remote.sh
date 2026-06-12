#!/usr/bin/env bash
set -eu -o pipefail

# A SHA-256 remote, created explicitly with `--object-format=sha256` so it stays SHA-256 regardless of
# `GIX_TEST_FIXTURE_HASH`. It exists to exercise cloning a SHA-256 remote into a freshly initialized
# local repository, which defaults to SHA-1 and must adopt the remote's object format.
#
# The identity and dates are fixed so the generated archive is deterministic across hosts.
export GIT_AUTHOR_NAME='gitoxide' GIT_AUTHOR_EMAIL='gitoxide@example.com' GIT_AUTHOR_DATE='1112354055 +0200'
export GIT_COMMITTER_NAME='gitoxide' GIT_COMMITTER_EMAIL='gitoxide@example.com' GIT_COMMITTER_DATE='1112354055 +0200'

git -c init.defaultBranch=main init --object-format=sha256 -q remote
(
  cd remote
  echo hello >file
  git add file
  git commit -q -m "initial"
)
