#!/usr/bin/env bash
set -eu -o pipefail

# Record the Git resolved inside the same Bash used for fixture generation.
git --version >git-version
