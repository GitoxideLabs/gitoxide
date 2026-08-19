#!/bin/sh
set -eu

# Exercise every repository surface captured by repository::snapshot(): detached commit
# topology, multiple refs, nested paths, a conflicted index with an ordinary staged entry, and untracked worktree data.
git init -q -b main .
echo base >tracked
mkdir -p nested/deep
echo nested >nested/deep/tracked
git add tracked nested/deep/tracked
GIT_AUTHOR_DATE='2000-01-01T00:00:00 +0000' GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git commit -q -m base
git branch side
echo staged >staged
git add staged
base=$(printf 'conflict base\n' | git hash-object -w --stdin)
ours=$(printf 'conflict ours\n' | git hash-object -w --stdin)
theirs=$(printf 'conflict theirs\n' | git hash-object -w --stdin)
printf '100644 %s 1\tconflicted\n100644 %s 2\tconflicted\n100644 %s 3\tconflicted\n' "$base" "$ours" "$theirs" |
  git update-index --index-info
echo untracked >untracked
commit=$(git commit-tree HEAD^{tree} -p HEAD -m detached)
git update-ref refs/heads/side "$commit"
