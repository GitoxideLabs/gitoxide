#!/usr/bin/env bash
set -eu

# Scoop writes absolute Windows paths into its UTF-8 shim sidecars. Derive them
# during local fixture creation with cygpath; this Windows-only fixture must not
# be moved or restored from an archive created at another location.
scoop_fixture_root=$(cygpath -am "$PWD")

# Empty shims and target executables cannot run. Successful public path discovery
# must use the sidecar and the target's modern Git for Windows installation layout.
# `current` is a plain directory: shim resolution does not require a junction.
for installation in scoop no-junction; do
    version=current
    if test "$installation" = no-junction; then
        version=2.56.0
    fi
    root="$installation/apps/git/$version"
    mkdir -p "$installation/shims" "$root/cmd" "$root/ucrt64/libexec/git-core" "$root/etc"
    touch "$installation/shims/git.exe" "$root/cmd/git.exe"
    printf '[core]\n\tautocrlf = true\n' >"$root/etc/gitconfig"
    printf 'path = "%s/%s/cmd/git.exe"\n' "$scoop_fixture_root" "$root" >"$installation/shims/git.shim"
done

# A valid target with additional arguments may change Git's runtime selection.
# Stale and absent sidecars likewise require querying the selected executable.
for installation in args stale absent; do
    mkdir -p "$installation/shims"
    touch "$installation/shims/git.exe"
done
printf 'path = "%s/scoop/apps/git/current/cmd/git.exe"\nargs = --exec-path=elsewhere\n' \
    "$scoop_fixture_root" >args/shims/git.shim
printf 'path = "%s/missing/cmd/git.exe"\n' "$scoop_fixture_root" >stale/shims/git.shim
