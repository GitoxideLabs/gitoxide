#!/usr/bin/env bash
set -eu -o pipefail

# Explicit HEAD and main selectors must use the current branch tracking configuration.
# Record Git results here so the read-only test never needs to invoke Git.
git init -q
GIT_COMMITTER_DATE='2000-01-01 00:00:00 +0000' git commit -q --allow-empty -m initial
git remote add origin .
git config branch.main.remote origin
git config branch.main.merge refs/heads/main
GIT_COMMITTER_DATE='2000-01-01 00:00:00 +0000' git fetch -q origin
# Keep two tracking-ref values so chained entry and date selectors can distinguish them.
GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git commit -q --allow-empty -m second
GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git fetch -q origin

for op in upstream u push; do
  for branch in '' HEAD main; do
    revspec="$branch@{$op}"
    printf '%s\n' "$revspec" >>baseline.git
    git rev-parse --verify "$revspec" >>baseline.git
    for query in 0 1 '2000-01-01 12:00:00 +0000' -1; do
      spec="$revspec@{$query}"
      printf '%s\n' "$spec" >>baseline.git
      git rev-parse -q --verify "$spec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
    done
  done
done

# Local tracking cycles make a repeated tracking selector resolvable, but Git
# rejects that syntax. Pushes stay on the same local branch for the same check.
git branch local-a HEAD
git branch local-b HEAD~1
git config remote...url .
git config remote...fetch '+refs/heads/*:refs/heads/*'
git config branch.local-a.remote .
git config branch.local-a.merge refs/heads/local-b
git config branch.local-b.remote .
git config branch.local-b.merge refs/heads/local-a
git config push.default current
for op in upstream u push; do
  for branch in local-a local-b; do
    for query in 0 u upstream push; do
      spec="$branch@{$op}@{$query}"
      printf '%s\n' "$spec" >>baseline.git
      git rev-parse -q --verify "$spec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
    done
  done
done
