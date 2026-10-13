#!/usr/bin/env bash
set -euo pipefail

# An independent workspace prevents application and dev-dependency features from
# silently enabling `gix/unstable` in the stable consumer.
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gix-api.XXXXXX")"
trap 'rm -rf -- "$probe_dir"' EXIT
manifest_root="${repo_root//\\/\\\\}"
manifest_root="${manifest_root//\"/\\\"}"
cp "$repo_root/gix/tests/stable-api/"*.rs "$probe_dir/"
cat >"$probe_dir/Cargo.toml" <<EOF
[package]
name = "gix-stable-api-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[features]
unstable = ["gix/unstable"]
parallel = ["gix/parallel"]
sha256 = ["gix/sha256", "gix-object/sha256"]
[dependencies]
gix = { path = "$manifest_root/gix", default-features = false, features = ["sha1"] }
gix-object = { path = "$manifest_root/gix-object", features = ["sha1"] }
[[bin]]
name = "stable"
path = "stable.rs"
[[bin]]
name = "plumbing"
path = "plumbing.rs"
EOF

check=(cargo check --quiet --manifest-path "$probe_dir/Cargo.toml" --target-dir "${CARGO_TARGET_DIR:-$repo_root/target}")
"${check[@]}" --bin stable
"${check[@]}" --bin stable --features parallel,sha256
if "${check[@]}" --bin plumbing >"$probe_dir/diagnostics" 2>&1; then
    printf '%s\n' 'Unstable plumbing unexpectedly compiled without opting in.' >&2
    exit 1
fi
for expected in OdbHandle objects head_id Find; do
    if ! grep -Fq "$expected" "$probe_dir/diagnostics"; then
        cat "$probe_dir/diagnostics" >&2
        printf 'Missing expected gate diagnostic: %s\n' "$expected" >&2
        exit 1
    fi
done
"${check[@]}" --bin plumbing --features unstable
"${check[@]}" --bin stable --features unstable,parallel,sha256
