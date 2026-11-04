#!/usr/bin/env bash
set -eu -o pipefail

# Reflog expiry must precede reachability collection, otherwise the old commit
# stays alive for an additional GC. Exercise reference packing in bare and
# non-bare repositories without relying on physical pack layout.
for kind in bare worktree; do
    if test "$kind" = bare; then git init -q --bare "$kind"; else git init -q "$kind"; fi
    (
        cd "$kind"
        git symbolic-ref HEAD refs/heads/main
        tree=$(git mktree </dev/null)
        old=$(printf old | git commit-tree "$tree")
        live=$(printf live | git commit-tree "$tree")
        GIT_COMMITTER_DATE='2000-01-01T00:00:00 +0000' git update-ref --create-reflog refs/heads/main "$old"
        GIT_COMMITTER_DATE='2001-01-01T00:00:00 +0000' git update-ref refs/heads/main "$live"
        printf '%s\n' "$old" >old-id
        git config gc.reflogExpire all
        git config gc.reflogExpireUnreachable all
    )
done
