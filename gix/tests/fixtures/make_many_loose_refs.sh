#!/usr/bin/env bash
set -eu -o pipefail

git init -q -b main
git commit -q --allow-empty -m initial
commit_id=$(git rev-parse HEAD)

# Equally sized directories exercise reuse of sorting buffers across siblings.
for ((group = 0; group < 10; group++)); do
  printf -v dir '.git/refs/heads/loose/%03d' "$group"
  mkdir -p "$dir"
  for ((index = 0; index < 100; index++)); do
    printf -v name '%03d' "$index"
    printf '%s\n' "$commit_id" > "$dir/$name"
  done
done

# Symbolic refs exercise reuse of the separate target-name buffer.
mkdir -p .git/refs/heads/symbolic
for ((index = 0; index < 100; index++)); do
  printf -v name '%04d' "$index"
  printf 'ref: refs/heads/main\n' > ".git/refs/heads/symbolic/$name"
done
