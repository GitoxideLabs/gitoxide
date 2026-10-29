#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -gt 1 ]]; then
    printf 'usage: %s [BASELINE_REVISION]\n' "${0##*/}" >&2
    exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(git rev-parse --show-toplevel)
cd -- "$repo_root"
# Share the existing cache with the archived baseline, including when the caller supplied a relative path.
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$repo_root/target}
[[ "$CARGO_TARGET_DIR" == /* ]] || export CARGO_TARGET_DIR="$repo_root/$CARGO_TARGET_DIR"
args=(compare gix)
[[ -z "${1:-}" ]] || args+=(--baseline "$1")
exec "$script_dir/check-api.sh" "${args[@]}"
