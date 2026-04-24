#!/usr/bin/env bash

set -eux -o pipefail

root="$(readlink -f -- "$1")"
output_corpus="$2"

source "$root/etc/fuzz-corpus-builder.sh"

build_fuzz_corpus "$root" "$output_corpus" "gix-pack" "data_file"
