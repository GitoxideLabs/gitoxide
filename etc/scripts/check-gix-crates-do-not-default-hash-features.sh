#!/usr/bin/env bash
#
# Check that gix-* crates with public sha1 or sha256 features do not enable
# either hash algorithm in their default feature set.
#
# Hash selection must be explicit for library crates. If a crate defaults to a
# hash algorithm, downstream users can accidentally compile with the wrong object
# hash support instead of making the choice at the application boundary.

set -euo pipefail

offenders="$(
    cargo metadata --format-version 1 --no-deps |
        jq --raw-output '
            .workspace_members as $members
            | .packages[] as $pkg
            | select($members | index($pkg.id))
            | select($pkg.name | startswith("gix-"))
            | select($pkg.features | has("sha1") or has("sha256"))
            | ($pkg.features.default // []) as $default_features
            | ($default_features | map(select(. == "sha1" or . == "sha256"))) as $default_hash_features
            | select($default_hash_features | length > 0)
            | "\($pkg.name): default includes \($default_hash_features | join(","))"
        '
)"

printf 'Checking gix-* crates do not default to sha1 or sha256\n'

if [[ -n "$offenders" ]]; then
    printf '%s: error: hash features must not be default features.\n' "$0" >&2
    printf '%s\n' "$offenders" >&2
    exit 1
fi
