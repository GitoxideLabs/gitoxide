#!/usr/bin/env bash
set -eu -o pipefail

git init -q

# The target commit is deliberately newer than its child. Git name-rev uses the
# target date, minus a one-day slop, as a cutoff and does not let the older child
# tag name the newer parent.
echo base >f
git add f
GIT_AUTHOR_DATE="2001-01-01T00:00:00+0000" GIT_COMMITTER_DATE="2001-01-01T00:00:00+0000" git commit -qm base
git branch skew-target

echo skewed-child >>f
git add f
GIT_AUTHOR_DATE="2000-01-01T00:00:00+0000" GIT_COMMITTER_DATE="2000-01-01T00:00:00+0000" git commit -qm skewed-child
GIT_COMMITTER_DATE="2000-02-01T00:00:00+0000" git tag -a skewed-tag -m skewed-tag
