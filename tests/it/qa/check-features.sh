#!/usr/bin/env bash

set -eux

target_dir="$(cargo metadata --format-version 1 --no-deps | jq --exit-status --raw-output '.target_directory')"
# Reuse compatible artifacts while resolving each fuzz workspace separately.
for manifest in gix-*/fuzz/Cargo.toml; do
    cargo check --manifest-path "$manifest" --all-targets --target-dir "$target_dir"
done
etc/scripts/check-gix-crates-without-hash-features.sh
etc/scripts/check-gix-crates-require-hash-features.sh
etc/scripts/check-gix-crates-do-not-default-hash-features.sh
etc/scripts/check-gix-crate-hash-feature-combinations.sh
# Assert that incompatible top-level feature combinations still fail.
if cargo check --features lean-async 2>/dev/null; then
    printf '%s\n' 'Expected lean-async to conflict with the default blocking client features' >&2
    exit 1
fi
if cargo check -p gitoxide-core --all-features --features gix/sha1 2>/dev/null; then
    printf '%s\n' 'Expected gitoxide-core to reject enabling both client I/O modes' >&2
    exit 1
fi
tree="$(cargo --color=never tree -p gix --no-default-features -e normal --prefix none --format '{p}')"
if printf '%s\n' "$tree" | grep -Eq '^gix-imara-diff(-01)? v'; then
    printf '%s\n' 'gix must not depend on gix-imara-diff without default features' >&2
    exit 1
fi
cargo --color=never tree -p gix --no-default-features -e normal -i gix-submodule \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
cargo --color=never tree -p gix --no-default-features -e normal -i gix-pathspec \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
cargo --color=never tree -p gix --no-default-features -e normal -i gix-filter \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
if cargo tree -p gix --no-default-features -i gix-credentials 2>/dev/null; then
    printf '%s\n' 'Expected gix-credentials to be absent without default features' >&2
    exit 1
fi
cargo check -p gix-worktree --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-pack --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-index --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-revision --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-diff --no-default-features 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-protocol --features blocking-client 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix --no-default-features 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-odb --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
