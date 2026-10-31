#!/usr/bin/env bash

set -euo pipefail

case "${1:-}" in
    --fast) mode=fast ;;
    --thorough) mode=exhaustive ;;
    *) printf 'usage: %s (--fast|--thorough) [qa run options...]\n' "${0##*/}" >&2; exit 2 ;;
esac
shift
cd -- "$(git rev-parse --show-toplevel)"
# Keep the historical entry point used by Tix QA profiles.
exec cargo run --profile qa --locked -p internal-tools --no-default-features --bin qa -- run --mode "$mode" "$@"
