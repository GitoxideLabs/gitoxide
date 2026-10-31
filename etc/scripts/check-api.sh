#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
exec cargo run --quiet --manifest-path "$repo_dir/Cargo.toml" \
    -p internal-tools --bin it -- check-api "$@"
