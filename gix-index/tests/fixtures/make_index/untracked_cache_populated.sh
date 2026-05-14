#!/usr/bin/env bash
set -eu -o pipefail

# Reuse the empty UNTR fixture and run status so Git fills the cache with the
# descriptive tracked/untracked path layout and seeded mtimes from that script.
. "$(dirname -- "${BASH_SOURCE[0]}")/untracked_cache_empty.sh"
# This triggers the untracked cache to be refreshed.
git status > /dev/null
