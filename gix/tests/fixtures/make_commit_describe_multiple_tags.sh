#!/usr/bin/env bash
set -eu -o pipefail

git init -q
git commit --allow-empty -q -m c1
git commit --allow-empty -q -m c2
git commit --allow-empty -q -m c3

# Tag the first commit (with lightweight tags only)
git tag l0 ":/c1"
GIT_COMMITTER_DATE="2022-01-02 00:00:00 +0000" git tag l1 "HEAD~2"

# Tag the second commit (for tests involving tag priority)
# The date is not checked for lightweight tags, so date the annotated tag to 0
GIT_COMMITTER_DATE="1970-01-01 00:00:00 +0000" git tag v1 -m "tag object 0" :/c2
git tag v0 ":/c2"

# Tag the third (HEAD) commit (testing the combination of priority, date and lexicographical order)
git tag v2 -m "tag object 1"
git tag v2.5
GIT_COMMITTER_DATE="2022-01-02 00:00:00 +0000" git tag v4 -m "tag object 4"
GIT_COMMITTER_DATE="2022-01-02 00:00:00 +0000" git tag v5 -m "tag object 5"
GIT_COMMITTER_DATE="2022-01-03 00:00:00 +0000" git tag v3

# Nested annotated tags use the outer tag's date and peel all the way to a commit.
for case in nested-both nested-only; do
  git init -q "$case"
  (
    cd "$case"
    git commit -q --allow-empty -m initial
    GIT_COMMITTER_DATE='2000000000 +0000' git tag -a -m v1 v1
    GIT_COMMITTER_DATE='2000000001 +0000' git tag -a -m v2 v2 v1
    if test "$case" = nested-only; then
      git tag -d v1
      git pack-refs --all
    fi
    git describe >describe.git
    git describe --tags >>describe.git
  )
done

# A malformed inner tag must not panic while collecting otherwise valid names.
cp -R nested-both nested-malformed
(
  cd nested-malformed
  malformed_id=$(printf 'type commit\ntag invalid\n\nmissing object header\n' | git hash-object --literally -t tag -w --stdin)
  GIT_COMMITTER_DATE='2000000002 +0000' git tag -a -m broken v3 "$malformed_id"
)
