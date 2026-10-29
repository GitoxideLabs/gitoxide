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

# Without junctions, Scoop selects the most recently installed version by marker mtime,
# not the highest version number. Ignore incomplete extractions and old backup directories.
for version in 2.99.0 2.50.0 2.100.0 _2.101.0.old; do
    mkdir -p "no-junction/apps/git/$version/cmd"
    touch "no-junction/apps/git/$version/cmd/git.exe"
done
touch -t 202001010000 no-junction/apps/git/2.99.0/install.json
touch -t 202101010000 no-junction/apps/git/2.50.0/scoop-install.json
touch -t 202201010000 no-junction/apps/git/_2.101.0.old/scoop-install.json
# A usable current junction/directory still wins over any retained version.
mkdir -p custom/apps/git/2.999.0/cmd
touch custom/apps/git/2.999.0/cmd/git.exe custom/apps/git/2.999.0/scoop-install.json
