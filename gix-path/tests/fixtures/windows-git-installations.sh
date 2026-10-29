#!/usr/bin/env bash
set -eu

# Discovery only checks for git.exe, so empty files model installed executables.
# Keep the layout read-only after creation, with distinct roots to test precedence.
for directory in user/scoop/apps/git/current/cmd custom/apps/git/current/cmd global/scoop/apps/git/current/cmd explicit/cmd path chocolatey/git/cmd conda/Library/bin winget/Microsoft/WinGet/Links machine/WinGet/Links msys/usr/bin; do
    mkdir -p "$directory"
    touch "$directory/git.exe"
done

# A relocated MSYS runtime lets Windows tests exercise real path conversion without requiring
# pacman or downloading packages. Git for Windows provides these same runtime tools.
if test -f /usr/bin/cygpath.exe; then
    cp /usr/bin/cygpath.exe /usr/bin/msys-2.0.dll msys/usr/bin/
fi
touch msys/usr/bin/bash.exe msys/usr/bin/sh.exe
