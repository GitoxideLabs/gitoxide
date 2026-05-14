#!/usr/bin/env bash
#
# Check that every gix-* crate with public hash features fails without selecting
# an explicit hash algorithm.
#
# This catches accidental default-hash behavior and empty feature gates. Crates
# that depend on gix-hash transitively must forward sha1/sha256 instead of
# compiling in an ambiguous hash configuration.

set -euo pipefail

hash_feature_error='Please set either the `sha1` or the `sha256` feature flag'
stderr_file="$(mktemp -t gix-hash-feature-check.XXXXXX)"
trap 'rm -f "$stderr_file"' EXIT

crates="$(
    cargo metadata --format-version 1 --no-deps |
        jq --raw-output '
            .workspace_members as $members
            | .packages[] as $pkg
            | select($members | index($pkg.id))
            | select($pkg.name | startswith("gix-"))
            | select($pkg.features | has("sha1") or has("sha256"))
            | $pkg.name
        '
)"

while IFS= read -r crate; do
    [[ -n "$crate" ]] || continue

    printf 'Checking %s requires explicit hash feature: cargo check -p %s\n' "$crate" "$crate"
    if cargo check -p "$crate" > /dev/null 2> "$stderr_file"; then
        printf '%s: error: expected %s to require an explicit hash feature.\n' "$0" "$crate" >&2
        printf 'Reproduce with: cargo check -p %s\n' "$crate" >&2
        exit 1
    fi

    if ! grep -Fq "$hash_feature_error" "$stderr_file"; then
        printf '%s: error: expected %s to fail with the hash feature message.\n' "$0" "$crate" >&2
        printf 'Reproduce with: cargo check -p %s\n' "$crate" >&2
        cat "$stderr_file" >&2
        exit 1
    fi
done <<<"$crates"
