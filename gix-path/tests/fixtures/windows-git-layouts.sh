#!/usr/bin/env bash
set -eu

# Empty executables cannot run. Successful detection must come from the layout,
# and selecting a runtime binary must disambiguate an installation with several runtimes.
for runtime in clangarm64 ucrt64 mingw64 mingw32; do
    mkdir -p "mixed/$runtime/bin"
    touch "mixed/$runtime/bin/git.exe"
done
mkdir -p mixed/etc
printf '[core]\n\tautocrlf = true\n' >mixed/etc/gitconfig
