#!/usr/bin/env bash
set -eu -o pipefail

# Validate parent symlinks separately from leaf links, retaining a symlinked root.

mkdir base;
(cd base
  touch file
  mkdir dir
  touch dir/file-in-dir
  (cd dir
    ln -s file-in-dir filelink
    mkdir subdir
    ln -s subdir dirlink
  )

  ln -s file root-filelink
  ln -s dir root-dirlink
)

ln -s base symlink-base

