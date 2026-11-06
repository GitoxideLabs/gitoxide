#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
generator=$(mktemp "${TMPDIR:-/tmp}/gix-encoding-generate.XXXXXX")
trap 'rm -f "$generator"' EXIT
"${CC:-cc}" -std=c99 -O2 -Wall -Wextra -Werror generate.c -o "$generator"
"$generator"
