#!/usr/bin/env bash
set -eu -o pipefail

# Merge tests rebuild this committed tree and index while updating HEAD.
git init -q
git config user.name "merge test"
git config user.email merge@example.com
printf 'tracked\n' >file
git add file
git commit -qm initial
