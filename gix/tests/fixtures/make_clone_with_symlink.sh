#!/usr/bin/env bash
set -eu -o pipefail

# Build the symlink entry through Git's object plumbing so this fixture works even
# on systems where creating filesystem symlinks requires additional privileges.
git init --bare source.git
cd source.git
target_blob=$(printf 'target contents\n' | git hash-object -w --stdin)
link_blob=$(printf 'target' | git hash-object -w --stdin)
tree=$(printf '120000 blob %s\tlink\n100644 blob %s\ttarget\n' "$link_blob" "$target_blob" | git mktree)
commit=$(git commit-tree "$tree" -m 'Initial commit')
git update-ref refs/heads/main "$commit"
git symbolic-ref HEAD refs/heads/main
