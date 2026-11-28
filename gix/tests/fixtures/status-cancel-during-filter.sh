#!/usr/bin/env bash
set -eu -o pipefail

git init -q
printf 'file filter=gate\n' >.gitattributes
printf 'old\n' >file
git add .gitattributes file
git commit -q -m baseline

# Status must compare the contents through the clean filter configured by the test.
printf 'new\n' >file
