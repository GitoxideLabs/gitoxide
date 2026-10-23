#!/usr/bin/env bash
set -eu -o pipefail

git init -q untracked
(cd untracked
  touch file && git add file && git commit -m "just to get an index for the test-suite"

  mkfifo pipe
  git status
)

git init -q tracked-swapped
(cd tracked-swapped
  touch file && git add file && git commit -m "it starts out as trackable file"

  rm file && mkfifo file
  git status
)

# Executable files replaced by FIFOs remain modifications, not regular-file type changes.
git init -q executable-swapped
(cd executable-swapped
  echo content >file
  chmod +x file
  git add file && git commit -m "executable file before FIFO replacement"
  rm file && mkfifo file
  git status --porcelain=v1 >.git/status.baseline
)

