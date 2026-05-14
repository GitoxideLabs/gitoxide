#!/usr/bin/env bash
#
# Check every gix-* crate that advertises hash features with sha1, sha256, and
# sha1+sha256, limited to combinations the crate actually exposes.
#
# This keeps feature forwarding honest: a crate with a public hash feature must
# compile with that feature, and sha256 support must not silently regress while
# sha1 remains covered by older checks.

set -euo pipefail

metadata="$(cargo metadata --format-version 1 --no-deps)"

for features in sha1 sha256 sha1,sha256; do
    crates="$(
        printf '%s\n' "$metadata" |
            jq --arg features "$features" --raw-output '
                ($features | split(",")) as $required_features
                | .workspace_members as $members
                | .packages[] as $pkg
                | select($members | index($pkg.id))
                | select($pkg.name | startswith("gix-"))
                | select(all($required_features[]; . as $feature | $pkg.features | has($feature)))
                | $pkg.name
            '
    )"

    while IFS= read -r crate; do
        [[ -n "$crate" ]] || continue
        printf 'Checking %s with %s: cargo check -p %s --features %s\n' "$crate" "$features" "$crate" "$features"
        cargo check -p "$crate" --features "$features"
    done <<<"$crates"
done
