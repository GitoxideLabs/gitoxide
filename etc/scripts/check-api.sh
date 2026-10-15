#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
exec cargo run --quiet --manifest-path "$repo_dir/Cargo.toml" \
    -p gix-testtools --bin gix-check-api --no-default-features --features sha1,api-check -- "$@"
