#!/usr/bin/env bash
set -euo pipefail
exec cargo run --profile qa --locked -p internal-tools --no-default-features --bin qa -- run --mode exhaustive --stage check "$@"
