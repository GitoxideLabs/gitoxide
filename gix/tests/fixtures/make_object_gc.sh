#!/usr/bin/env bash
set -eu -o pipefail
export TZ=UTC

# A recent unreachable commit protects a much older tree/blob. An unrelated old
# blob can expire. Repacking these into cruft must preserve those original ages.
for storage in loose cruft; do
    git init -q --bare "$storage"
    (
        cd "$storage"
        git config core.logAllRefUpdates false
        live_blob=$(printf live | git hash-object -w --stdin)
        live_tree=$(printf '100644 blob %s\tlive\n' "$live_blob" | git mktree)
        live_commit=$(printf live | git commit-tree "$live_tree")
        git update-ref refs/heads/main "$live_commit"
        old_blob=$(printf expired | git hash-object -w --stdin)
        recent_blob=$(printf 'recent dependency' | git hash-object -w --stdin)
        recent_tree=$(printf '100644 blob %s\tdependency\n' "$recent_blob" | git mktree)
        recent_commit=$(printf recent | git commit-tree "$recent_tree")
        printf '%s\n' "$live_blob" "$live_tree" "$live_commit" >live-ids
        printf '%s\n' "$recent_blob" "$recent_tree" "$recent_commit" >recent-ids
        printf '%s\n' "$old_blob" >expired-id
        for object_id in "$live_blob" "$live_tree" "$live_commit" "$old_blob" "$recent_blob" "$recent_tree" "$recent_commit"; do
            touch -t 200101010000.00 "objects/${object_id:0:2}/${object_id:2}"
        done
        touch -t 200301010000.00 "objects/${recent_commit:0:2}/${recent_commit:2}"
        if test "$storage" = cruft; then
            git repack --cruft -d
        fi
    )
done
