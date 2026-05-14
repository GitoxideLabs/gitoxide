#!/usr/bin/env bash
set -eu -o pipefail

mkdir non-empty
printf "Pre-existing user content.\n" > non-empty/existing.txt

mkdir non-empty-with-conflicting-file
printf "Pre-existing user content.\n" > non-empty-with-conflicting-file/file

cp -R non-empty non-empty-with-dot-git
mkdir non-empty-with-dot-git/.git
printf "ref: refs/heads/pre-existing\n" > non-empty-with-dot-git/.git/HEAD
