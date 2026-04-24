#!/usr/bin/env bash
# Run a single parity test file WITHOUT running the full journey suite.
# Why: tests/journey.sh runs all tests under `set -eu` and aborts on first
# failure, including pre-existing unrelated ones (e.g. panic-behaviour
# snapshot drift). The parity loop needs to exercise exactly one
# tests/journey/parity/<cmd>.sh file in isolation.
#
# Usage:
#   bash tests/parity.sh tests/journey/parity/<cmd>.sh
#
# Optional env:
#   EIN, GIX, JTT  — override binary paths (defaults: target/debug/{ein,gix,jtt})
#   KIND           — feature kind: small|lean|max-pure|max (default: max)
set -eu

target="${1:?usage: $0 tests/journey/parity/<cmd>.sh}"
[[ -f "$target" ]] || { echo "not a file: $target" >&2; exit 2; }

root="$(cd "${0%/*}" && pwd)"

exe="${EIN:-$root/../target/debug/ein}"
exe_plumbing="${GIX:-$root/../target/debug/gix}"
jtt="${JTT:-$root/../target/debug/jtt}"
kind="${KIND:-max}"

for bin in "$exe" "$exe_plumbing" "$jtt"; do
  [[ -x "$bin" ]] || { echo "missing binary: $bin — run 'cargo build --features http-client-curl-rustls && cargo build -p gix-testtools --bin jtt'" >&2; exit 2; }
done

# shellcheck disable=1091
source "$root/utilities.sh"
# shellcheck disable=1091
source "$root/helpers.sh"
snapshot="$root/snapshots/parity"
fixtures="$root/fixtures"

SUCCESSFULLY=0
WITH_FAILURE=1
WITH_CLAP_FAILURE=2

export LC_ALL=C
set-static-git-environment

# Kill switch: if <target>.stop exists, bail gracefully for the loop.
if [[ -f "${target}.stop" ]]; then
  echo "${YELLOW}parity.sh: ${target}.stop present — halting gracefully"
  exit 0
fi

# Run the target file under each hash kind. Per-row skipping (for rows
# marked `# hash=sha1-only`) is enforced inside the target via the
# only_for_hash helper.
for hash_kind in sha1 sha256; do
  echo "${WHITE}====================================================="
  echo "${GREEN}HASH = $hash_kind"
  echo "${WHITE}====================================================="
  export GIX_TEST_FIXTURE_HASH="$hash_kind"
  # shellcheck disable=1090
  source "$target"
done
unset GIX_TEST_FIXTURE_HASH
