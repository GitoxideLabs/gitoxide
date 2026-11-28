#!/usr/bin/env bash
set -eu -o pipefail

git init -q
printf 'candidate filter=reject-clean\n' >.gitattributes
git add .gitattributes
git commit -q -m baseline

# The dirwalk queues this file before the serial tracked-file scan starts.
# A cancelled scan must skip the required clean filter when it drains that queue.
printf 'untracked content\n' >candidate
git config filter.reject-clean.clean 'exit 7'
git config filter.reject-clean.required true
