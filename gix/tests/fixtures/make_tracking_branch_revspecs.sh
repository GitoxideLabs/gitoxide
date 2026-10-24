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

# A short or full object ID can also be a branch name; selectors require that ref.
git config --add remote.origin.push refs/heads/main:refs/heads/main
for branch in "$(git rev-parse --short=4 HEAD)" "$(git rev-parse HEAD)"; do
  printf '%s\n' "$branch" >>hex-branches
  GIT_COMMITTER_DATE='2000-01-01 00:00:00 +0000' git branch "$branch" HEAD~1
  GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git update-ref "refs/heads/$branch" HEAD
  git config "branch.$branch.remote" origin
  git config "branch.$branch.merge" refs/heads/main
  git config "branch.$branch.pushRemote" origin
  git config --add remote.origin.push "refs/heads/$branch:refs/heads/main"
  for suffix in '' '@{0}' '@{1}' '@{2000-01-01 12:00:00 +0000}' '@{upstream}' '@{push}' '@{u}@{1}'; do
    spec="$branch$suffix"
    printf '%s\n' "$spec" >>baseline.git
    git rev-parse -q --verify "$spec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
  done
done

tree_prefix=$(git rev-parse --short=4 'HEAD^{tree}')
printf '%s\n' "$tree_prefix" >unreferenced-prefix
printf '%s\n' "$tree_prefix@{0}" >>baseline.git
git rev-parse -q --verify "$tree_prefix@{0}" >>baseline.git 2>/dev/null || echo $? >>baseline.git
