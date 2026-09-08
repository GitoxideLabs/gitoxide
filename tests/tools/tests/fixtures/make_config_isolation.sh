#!/usr/bin/env bash
set -eu -o pipefail

# A script's own command-scope configuration must add to the isolation, not replace it.
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=init.defaultBranch GIT_CONFIG_VALUE_0=other
git init -q repo
git -C repo config --get maintenance.auto >maintenance-auto
git -C repo symbolic-ref HEAD >head
