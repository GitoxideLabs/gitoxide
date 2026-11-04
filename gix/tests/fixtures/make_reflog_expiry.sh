#!/usr/bin/env bash
set -eu -o pipefail

# Dates straddle total and unreachable expiry cutoffs. Resetting drops a commit
# from main while another ref keeps a different tip reachable from HEAD's log.
git init -q -b main
for year in 2000 2001 2002; do
    GIT_COMMITTER_DATE="$year-01-01T00:00:00 +0000" git commit -q --allow-empty -m "$year"
done
git branch protected HEAD~1
git branch keeps-dropped HEAD
GIT_COMMITTER_DATE='2003-01-01T00:00:00 +0000' git reset -q --hard HEAD~1
GIT_COMMITTER_DATE='2002-06-01T00:00:00 +0000' git update-ref --create-reflog refs/stash HEAD
GIT_COMMITTER_DATE='2002-06-01T00:00:00 +0000' git update-ref --create-reflog refs/heads/keep-main HEAD
GIT_COMMITTER_DATE='2002-06-01T00:00:00 +0000' git update-ref --create-reflog refs/heads/partial-main HEAD
blob=$(printf blob | git hash-object -w --stdin)
GIT_COMMITTER_DATE='2002-06-01T00:00:00 +0000' git update-ref --create-reflog refs/tags/blob "$blob"
git -c core.logAllRefUpdates=always worktree add -q linked -b linked
GIT_COMMITTER_DATE='2002-06-01T00:00:00 +0000' git -C linked commit -q --allow-empty -m linked

if test "${1-}" = defaults; then exit 0; fi

git config gc.reflogExpire '@1009843200'
git config gc.reflogExpireUnreachable '@1041379200'
git config 'gc.refs/heads/keep*.reflogExpire' never
git config 'gc.refs/heads/keep*.reflogExpireUnreachable' never
# Matching patterns use first-match order; unspecified fields mean never.
git config 'gc.refs/heads/partial*.reflogExpireUnreachable' never
git config 'gc.refs/heads/keep-main.reflogExpire' all
