#!/usr/bin/env bash
set -eu -o pipefail

# Model supported, ungated, and unreachable releases without touching the source checkout.
git init -q
mkdir -p gix/src
cat >Cargo.toml <<'EOF'
[workspace]
members = ["gix"]
resolver = "2"
EOF
cat >gix/Cargo.toml <<'EOF'
[package]
name = "gix"
version = "0.1.0"
edition = "2024"
EOF
cat >Cargo.lock <<'EOF'
version = 4
[[package]]
name = "gix"
version = "0.1.0"
EOF
echo 'pub fn read() -> bool { true }' >gix/src/lib.rs
git add Cargo.toml Cargo.lock gix
git commit -qm ungated
git tag gix-v0.1.0

# A trailing comment is valid TOML and must not hide a supported release.
cat >>gix/Cargo.toml <<'EOF'
[package.metadata.api-check] # supported API
profiles = [{ default-features = false, features = [] }]
EOF
if [[ "${1:-}" == before ]]; then
    exit 0
fi
for file in gix/Cargo.toml Cargo.lock; do
    sed 's/0.1.0/0.2.0/' "$file" >"$file.new"
    mv "$file.new" "$file"
done
git commit -qam supported
git tag gix-v0.2.0

# The newest reachable tag has no supported contract; skip it during automatic selection.
git show gix-v0.1.0:gix/Cargo.toml | sed 's/0.1.0/0.3.0/' >gix/Cargo.toml
sed 's/0.2.0/0.3.0/' Cargo.lock >Cargo.lock.new
mv Cargo.lock.new Cargo.lock
git commit -qam newer-ungated
git tag gix-v0.3.0

# An unreachable tag must be usable explicitly, but must not become the automatic baseline.
git show gix-v0.2.0:gix/Cargo.toml | sed 's/0.2.0/0.3.0/' >gix/Cargo.toml
echo 'pub fn read() -> u8 { 1 }' >gix/src/lib.rs
git commit -qam future
git tag gix-v99.0.0
git reset --hard HEAD^

# Local checks include uncommitted metadata and source changes.
git show gix-v0.2.0:gix/Cargo.toml | sed 's/0.2.0/0.3.0/' >gix/Cargo.toml
echo 'pub fn read() -> bool { false }' >gix/src/lib.rs
