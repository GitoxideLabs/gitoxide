#!/usr/bin/env bash
set -eu -o pipefail

# Cargo, rather than the QA planner, resolves optional aliases and self-dev features.
mkdir -p app/src changed/src platform/src
cat >Cargo.toml <<'EOF'
[workspace]
members = ["app", "changed", "platform"]
resolver = "2"
EOF
cat >app/Cargo.toml <<'EOF'
[package]
name = "app"
version = "0.0.0"
edition = "2021"
[features]
archive = ["dep:renamed"]
weak = ["renamed?/weak"]
[dependencies]
renamed = { package = "changed", path = "../changed", optional = true }
[dev-dependencies]
app = { path = ".", default-features = false, features = ["archive"] }
EOF
cat >changed/Cargo.toml <<'EOF'
[package]
name = "changed"
version = "0.0.0"
edition = "2021"
[features]
weak = []
EOF

# All-target queries must retain Windows dependencies even when planning a WASM build.
cat >platform/Cargo.toml <<'EOF'
[package]
name = "platform"
version = "0.0.0"
edition = "2021"
[target.'cfg(windows)'.dependencies]
changed = { path = "../changed" }
EOF
touch app/src/lib.rs changed/src/lib.rs platform/src/lib.rs

# Generate once so read-only tests can use --locked without registry access or source writes.
cargo +stable generate-lockfile --offline
