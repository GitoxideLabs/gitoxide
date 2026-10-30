#!/usr/bin/env bash
set -eu -o pipefail

git init embedded-repository
(cd embedded-repository
  echo content >file && git add file && git commit -m "init"
)

git init -q
echo content >file
ln -s file link
ln -s . directory-link

echo binary >exe && chmod +x exe
mkfifo fifo

git submodule add ./embedded-repository submodule

mkdir empty-dir
git init uninitialized-embedded-repository
