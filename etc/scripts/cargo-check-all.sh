#!/usr/bin/env bash
set -euo pipefail
exec cargo run --locked -p internal-tools --no-default-features --bin ci -- run --mode exhaustive --stage check "$@"
