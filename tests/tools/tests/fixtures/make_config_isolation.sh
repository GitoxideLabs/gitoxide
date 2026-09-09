#!/usr/bin/env bash
set -eu -o pipefail

# A script's COUNT entries coexist with isolation, but PARAMETERS takes precedence for shared keys.
export GIT_CONFIG_COUNT=2 GIT_CONFIG_KEY_0=init.defaultBranch GIT_CONFIG_VALUE_0=other
export GIT_CONFIG_KEY_1=fixture.custom GIT_CONFIG_VALUE_1=present
git init -q repo
git -C repo config --get maintenance.auto >maintenance-auto
git -C repo config --get fixture.custom >custom
git -C repo symbolic-ref HEAD >head
# Explicit command-line configuration can still override isolation.
git -c init.defaultBranch=other init -q override
git -C override symbolic-ref HEAD >override-head
