#!/usr/bin/env bash
set -eu

# Exercise complete directory listings and the reduced listings available from Git's
# untracked cache. The mixed directory must never collapse over its tracked file.
git init
mkdir mixed uncached generated empty
printf '*.ignored\ngenerated/\n' >.gitignore
printf 'tracked\n' >mixed/tracked
git add .gitignore mixed/tracked
git commit -m initial
touch mixed/new mixed/secret.ignored top-untracked uncached/file generated/data
git ls-files --others --exclude-standard >.git/untracked-files
