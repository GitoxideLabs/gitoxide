#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git config diff.algorithm histogram

# Git 2.54 finds a shared empty line by re-diffing the change group after it slides.
printf '%s\n' L '' P Q '' >file
git add file
git commit -qm before
printf '%s\n' R Q '' Q '' >file
git commit -qam after

git diff --numstat HEAD^ HEAD -- file >.git/histogram.numstat
