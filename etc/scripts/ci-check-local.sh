#!/usr/bin/env bash

set -euo pipefail

case "${1:-}" in
    --fast) mode=fast ;;
    --thorough) mode=exhaustive ;;
    *) printf 'usage: %s (--fast|--thorough) [ci run options...]\n' "${0##*/}" >&2; exit 2 ;;
esac
shift
cd -- "$(git rev-parse --show-toplevel)"
exec cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode "$mode" "$@"
