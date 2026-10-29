#!/usr/bin/env bash
set -eu -o pipefail

# 2,001 packed blobs exercise two full 1,000-object chunks and the partial tail.
# Keep every object packed so the cold statistics pass must load pack headers.
git init -q --bare repo.git
{
    for n in $(seq 1 2001); do
        content="packed statistics blob $n"
        printf 'blob\ndata %s\n%s\n' "${#content}" "$content"
    done
} | git --git-dir=repo.git -c fastimport.unpackLimit=0 fast-import --quiet
