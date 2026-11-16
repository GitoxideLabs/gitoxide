#!/bin/sh
set -eu

# Empty roots, ordinary empty commits, and merges distinguish first-parent tree
# equality from ancestry or line counts. Pending commits retain a different
# display parent, and the shallow clone has an unavailable parent.
git init -q -b main .
git config user.name author
git config user.email author@example.com
git commit -q --allow-empty -m empty-root
git tag empty-root

echo content >file
git add file
git commit -q -m changed
git tag changed
changed_tree_id=$(git rev-parse HEAD^{tree})
nonempty_root_id=$(git commit-tree "$changed_tree_id" -m nonempty-root)
git tag nonempty-root "$nonempty_root_id"

git commit -q --allow-empty -m empty
git tag empty
empty_commit_id=$(git rev-parse HEAD)

git switch -q -c side
echo side >side
git add side
git commit -q -m side
side_commit_id=$(git rev-parse HEAD)
side_tree_id=$(git rev-parse HEAD^{tree})

git switch -q main
git merge -q --no-ff -s ours -m empty-merge side
git tag empty-merge
merge_commit_id=$(git commit-tree "$side_tree_id" -p HEAD -p side -m changed-merge)
git reset -q --hard "$merge_commit_id"
git tag changed-merge
git update-index --chmod=+x file
git commit -q -m mode-change
git tag mode-change

# Indexed commits defer metadata; pending commits below are newer than the graph.
git commit-graph write --reachable

pending_commit () {
    pending_commit_id=$({
        printf 'tree %s\nparent %s\n' "$1" "$side_commit_id"
        printf 'author author <author@example.com> 946684800 +0000\n'
        printf 'committer author <author@example.com> 946684800 +0000\n'
        printf 'tix-rebase-parent %s\n\n%s\n' "$empty_commit_id" "$2"
    } | git hash-object -t commit -w --stdin)
    git tag "$2" "$pending_commit_id"
}
pending_commit "$changed_tree_id" pending-empty
pending_commit "$side_tree_id" pending-changed

git clone -q --bare --depth 1 "file://$PWD" shallow
