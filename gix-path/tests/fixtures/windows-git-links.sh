#!/usr/bin/env bash
set -eu

# Native file symlinks need privileges on some Windows hosts. Keep their fixtures
# separate so ordinary installation-layout tests still run when links are unavailable.
gix_testtools_require_symlinks

# Empty executables cannot run; successful discovery must follow the selected
# WinGet link to this known native Git layout, including its configuration file.
mkdir -p native/cmd native/ucrt64/libexec/git-core native/etc unknown
touch native/cmd/git.exe unknown/git.exe
printf '[core]\n\tautocrlf = true\n' >native/etc/gitconfig

# WinGet portable packages expose relative file symlinks at stable user/machine
# entry points. The machine link also exercises a chain to the selected package.
mkdir -p winget/Microsoft/WinGet/Links winget-machine/WinGet/Links
ln -s ../../../../native/cmd/git.exe winget/Microsoft/WinGet/Links/git.exe
ln -s ../../../winget/Microsoft/WinGet/Links/git.exe winget-machine/WinGet/Links/git.exe

# A familiar link location must not override an unfamiliar target's own layout.
mkdir -p symlinked/ucrt64/bin symlinked/ucrt64/libexec/git-core
ln -s ../../../unknown/git.exe symlinked/ucrt64/bin/git.exe
