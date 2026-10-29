#!/usr/bin/env bash
set -eu -o pipefail

# Keep reference Git command execution outside the developer's repositories.
git init -q repo
