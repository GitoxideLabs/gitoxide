#!/usr/bin/env bash

set -eux

cargo check --workspace --all-targets
for manifest in gix-*/fuzz/Cargo.toml; do
    cargo check --manifest-path "$manifest" --all-targets
done
cargo check --no-default-features --features small
etc/scripts/check-gix-crates-without-hash-features.sh
etc/scripts/check-gix-crates-require-hash-features.sh
etc/scripts/check-gix-crates-do-not-default-hash-features.sh
etc/scripts/check-gix-crate-hash-feature-combinations.sh
cargo check -p gix-packetline --all-features 2>/dev/null
cargo check -p gix-transport --all-features 2>/dev/null
# Assure incompatible top-level feature combinations still fail, while gix-protocol supports both I/O modes together.
! cargo check --features lean-async 2>/dev/null
! cargo check -p gitoxide-core --all-features --features gix/sha1 2>/dev/null
cargo check -p gix-protocol --all-features
tree="$(cargo --color=never tree -p gix --no-default-features -e normal --prefix none --format '{p}')"
! printf '%s\n' "$tree" | rg -q '^gix-imara-diff(-01)? v'
cargo --color=never tree -p gix --no-default-features -e normal -i gix-submodule \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
cargo --color=never tree -p gix --no-default-features -e normal -i gix-pathspec \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
cargo --color=never tree -p gix --no-default-features -e normal -i gix-filter \
    2>&1 >/dev/null | grep '^warning: nothing to print\>'
! cargo tree -p gix --no-default-features -i gix-credentials 2>/dev/null
cargo check --no-default-features --features lean
cargo check --no-default-features --features lean-async
cargo check --no-default-features --features max
cargo check -p gitoxide-core --features gix/sha1,blocking-client
cargo check -p gitoxide-core --features gix/sha1,async-client
cargo check -p gix-pack --no-default-features --features sha1
cargo check -p gix-pack --no-default-features --features sha1,generate
cargo check -p gix-pack --no-default-features --features sha1,streaming-input
cargo check -p gix-hash --no-default-features --features sha1,bstr
cargo check -p gix-hash --all-features
cargo check -p gix-object --all-features
cargo check -p gix-attributes --features serde
cargo check -p gix-glob --features serde
cargo check -p gix-worktree --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-worktree --features sha1,serde
cargo check -p gix-worktree --no-default-features --features sha1
cargo check -p gix-actor --features serde
cargo check -p gix-date --features serde
cargo check -p gix-tempfile --features signals
cargo check -p gix-tempfile --features hp-hashmap
cargo check -p gix-pack --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-pack --features sha1,serde
cargo check -p gix-pack --features sha1,pack-cache-lru-static
cargo check -p gix-pack --features sha1,pack-cache-lru-dynamic
cargo check -p gix-pack --features sha1,object-cache-dynamic
cargo check -p gix-packetline --features blocking-io
cargo check -p gix-packetline --features async-io
cargo check -p gix-index --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-index --features sha1,serde
cargo check -p gix-credentials --features serde
cargo check -p gix-sec --features serde
cargo check -p gix-revision --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-revision --features sha1,serde
cargo check -p gix-revision --no-default-features --features sha1,describe
cargo check -p gix-mailmap --features serde
cargo check -p gix-url --all-features
cargo check -p gix-status --all-features
cargo check -p gix-features --all-features
cargo check -p gix-features --features parallel
cargo check -p gix-features --features fs-read-dir
cargo check -p gix-features --features progress
cargo check -p gix-features --features io-pipe
cargo check -p gix-features --features crc32
cargo check -p gix-features --features cache-efficiency-debug
cargo check -p gix-commitgraph --all-features
cargo check -p gix-config-value --all-features
cargo check -p gix-config --all-features
cargo check -p gix-diff --no-default-features 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-diff --no-default-features --features sha1
cargo check -p gix-transport --features blocking-client
cargo check -p gix-transport --features async-client
cargo check -p gix-transport --features async-client,async-std
cargo check -p gix-transport --features http-client
cargo check -p gix-transport --features http-client-curl
cargo check -p gix-transport --features http-client-reqwest
cargo check -p gix-protocol --features blocking-client 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-protocol --features sha1,blocking-client
cargo check -p gix-protocol --features sha1,async-client
cargo check -p gix --no-default-features --features sha1,async-network-client
cargo check -p gix --no-default-features --features sha1,async-network-client-async-std
cargo check -p gix --no-default-features --features sha1,blocking-network-client
cargo check -p gix --no-default-features --features sha1,blocking-http-transport-curl
cargo check -p gix --no-default-features --features sha1,blocking-http-transport-reqwest
cargo check -p gix --no-default-features --features max-performance --tests
cargo check -p gix --no-default-features --features max-performance-safe --tests
cargo check -p gix --no-default-features --features progress-tree --tests
cargo check -p gix --no-default-features --features blob-diff --tests
cargo check -p gix --no-default-features --features revision --tests
cargo check -p gix --no-default-features --features revparse-regex --tests
cargo check -p gix --no-default-features --features mailmap --tests
cargo check -p gix --no-default-features --features excludes --tests
cargo check -p gix --no-default-features --features attributes --tests
cargo check -p gix --no-default-features --features worktree-mutation --tests
cargo check -p gix --no-default-features --features credentials --tests
cargo check -p gix --no-default-features --features index --tests
cargo check -p gix --no-default-features --features interrupt --tests
cargo check -p gix --no-default-features --features blame --tests
cargo check -p gix --no-default-features --features sha1
cargo check -p gix --no-default-features --features sha1,sha256
cargo check -p gix --no-default-features --features sha256
cargo check -p gix --no-default-features 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-odb --features serde 2>&1 >/dev/null | grep 'Please set either the `sha1` or the `sha256` feature flag'
cargo check -p gix-odb --features sha1,serde
cargo check --no-default-features --features max-control,sha1
