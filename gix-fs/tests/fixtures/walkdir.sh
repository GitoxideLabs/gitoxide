#!/usr/bin/env bash
set -eu -o pipefail

# Independent layouts keep one traversal scenario from affecting another.
# Hidden files must be included, but the depth limit must exclude nested entries.
mkdir -p hidden/directory
touch hidden/.hidden hidden/directory/nested

# Git orders these siblings as common., common/, common0, unlike lexical sorting.
mkdir -p sorted/common
touch sorted/common. sorted/common0 sorted/common/child

# Both components deliberately contain U+0061 U+0308, not precomposed U+00E4.
# This checks normalization of the full path as well as the entry name.
mkdir -p "unicode/ä"
touch "unicode/ä/ä"

# Leave "missing" absent so traversal can preserve its underlying I/O error.
