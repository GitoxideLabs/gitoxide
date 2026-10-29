#!/usr/bin/env bash
set -eu -o pipefail

# Collect roots with no branch reachability: reflog-only history, staged blobs,
# cached trees, annotated tags, detached linked HEADs, and rebase state files.
git init -q repo
cd repo
printf 'base\n' >file
git add file
git commit -qm base
git rev-parse HEAD >../expected
git tag -a -m annotated annotated
git rev-parse refs/tags/annotated >>../expected
printf 'reflog only\n' >file
git commit -qam reflog-only
git rev-parse HEAD >>../expected
git reset --hard HEAD~1
printf 'index only\n' >staged
git add staged
git rev-parse :staged >>../expected
# A gitlink to a nonexistent commit must not introduce a root in this database.
missing=$(git rev-parse HEAD | sed 's/./1/g')
printf '%s\n' "$missing" >../missing-gitlink
git update-index --add --cacheinfo "160000,$missing,submodule"
# Resolving a synthetic conflict leaves its original stages only in REUC.
base=$(printf 'conflict base\n' | git hash-object -w --stdin)
ours=$(printf 'conflict ours\n' | git hash-object -w --stdin)
theirs=$(printf 'conflict theirs\n' | git hash-object -w --stdin)
printf '%s\n' "$base" "$ours" "$theirs" >>../expected
printf '100644 %s 1\tconflicted\n100644 %s 2\tconflicted\n100755 %s 3\tconflicted\n' "$base" "$ours" "$theirs" | git update-index --index-info
printf 'resolved\n' >conflicted
git add conflicted
git write-tree >>../expected
git worktree add --detach linked HEAD
(
    cd linked
    printf 'linked head only\n' >file
    git commit -qam linked
    git rev-parse HEAD >>../../expected
    printf 'linked index only\n' >staged
    git add staged
    git rev-parse :staged >>../../expected
)
# Remove linked reflogs, leaving the private detached HEAD as its sole root.
rm .git/worktrees/linked/logs/HEAD
# Save roots used only by a paused rebase, in both worktree administration dirs.
for state in .git/rebase-merge .git/worktrees/linked/rebase-apply; do
    mkdir "$state"
    for name in autostash orig-head; do
        object=$(printf '%s\n' "$state/$name" | git hash-object -w --stdin)
        printf '%s\n' "$object" >"$state/$name"
        printf '%s\n' "$object" >>../expected
    done
done
# Git's rev-list traverses refs, reflogs, and all worktree indices. Rebase roots
# are passed explicitly because rev-list does not use prune's root collection.
git rev-list --objects --no-object-names --all --reflog --indexed-objects >../git-reachable
# Root collection must still work after the linked checkout becomes unavailable.
mv linked detached-checkout
