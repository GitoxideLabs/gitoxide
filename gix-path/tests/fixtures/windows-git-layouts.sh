#!/usr/bin/env bash
set -eu

# Empty executables cannot run. Successful detection must come from the layout,
# and selecting a runtime binary must disambiguate an installation with several runtimes.
for runtime in clangarm64 ucrt64 mingw64 mingw32; do
    mkdir -p "mixed/$runtime/bin" "mixed/$runtime/libexec/git-core"
    touch "mixed/$runtime/bin/git.exe"
done
mkdir -p mixed/etc
printf '[core]\n\tautocrlf = true\n' >mixed/etc/gitconfig

# Chocolatey portable and Conda/Mamba expose the top-level bin launcher. The
# latter packages Git for Windows inside Library, including its cmd launcher.
for root in chocolatey/git conda/Library; do
    mkdir -p "$root/bin" "$root/cmd" "$root/ucrt64/libexec/git-core" "$root/etc"
    touch "$root/bin/git.exe" "$root/cmd/git.exe"
    printf '[core]\n\tautocrlf = true\n' >"$root/etc/gitconfig"
done

# An incomplete known layout and an unknown executable must still use the query fallback.
mkdir -p incomplete/ucrt64/bin unknown
touch incomplete/ucrt64/bin/git.exe unknown/git.exe

# WinGet portable packages expose file symlinks, including at custom package roots.
mkdir -p winget/Microsoft/WinGet/Links winget-machine/WinGet/Links symlinked/ucrt64/bin symlinked/ucrt64/libexec/git-core
ln -s ../../../../conda/Library/cmd/git.exe winget/Microsoft/WinGet/Links/git.exe
ln -s ../../../winget/Microsoft/WinGet/Links/git.exe winget-machine/WinGet/Links/git.exe
# A familiar link location must not override an unfamiliar target's own layout.
ln -s ../../../unknown/git.exe symlinked/ucrt64/bin/git.exe
