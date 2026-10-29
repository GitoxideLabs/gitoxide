#!/usr/bin/env bash
set -eu

# Discovery only checks for git.exe, so empty files model installed executables.
# Keep the layout read-only after creation, with distinct roots to test precedence.
for directory in user/scoop/apps/git/current/cmd custom/apps/git/current/cmd global/scoop/apps/git/current/cmd explicit/cmd path chocolatey/git/cmd winget/Microsoft/WinGet/Links machine/WinGet/Links; do
    mkdir -p "$directory"
    touch "$directory/git.exe"
done

