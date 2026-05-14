#!/usr/bin/env bash
#
# Check every gix-* crate that does not advertise sha1 or sha256 without passing
# hash features.
#
# This protects hash-independent crates from accidentally acquiring a required
# hash configuration through dependencies. The crate set is derived from Cargo
# metadata so new crates are covered automatically.

set -euo pipefail

crates="$(
    cargo metadata --format-version 1 --no-deps |
        jq --raw-output '
            .workspace_members as $members
            | .packages[] as $pkg
            | select($members | index($pkg.id))
            | select($pkg.name | startswith("gix-"))
            | select(($pkg.features | has("sha1") or has("sha256")) | not)
            | $pkg.name
        '
)"

while IFS= read -r crate; do
    [[ -n "$crate" ]] || continue
    printf 'Checking %s: cargo check -p %s\n' "$crate" "$crate"
    cargo check -p "$crate"
done <<<"$crates"
