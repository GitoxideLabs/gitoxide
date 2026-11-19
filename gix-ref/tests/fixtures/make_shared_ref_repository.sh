#!/usr/bin/env bash
set -eu -o pipefail

# Two commits seed both reflogs so a second user can rewind and restore HEAD.
# Ownership and deliberately mismatched reflog modes are set only in writable copies.
git init --shared=0666
git commit --allow-empty -m first
git commit --allow-empty -m second
