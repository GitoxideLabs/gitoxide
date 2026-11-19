#!/usr/bin/env bash
set -eu -o pipefail

# The name is a byte-oriented config key but contains NUL, which cannot appear in a path.
git init -q
printf '%b' '[submodule "library\0name"]
  path = deps/library
  url = ../library.git
' >.gitmodules
