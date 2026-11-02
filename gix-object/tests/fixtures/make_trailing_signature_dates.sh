#!/usr/bin/env bash
set -eu -o pipefail

# GIX-34: fsck rejects extra date fields, but ordinary reads use the first date
# and timezone after the last `>`. Preserve those malformed bytes in both objects.
git init -q
tree_id=$(git mktree </dev/null)
for offset in +0200 -0530; do
    cat >"commit$offset" <<EOF
tree $tree_id
author A <a@example.com> 1112591163 $offset 1700000000 -0500
committer C <c@example.com> 1112591164 $offset 1700000000 -0500

Trailing signature dates
EOF
    commit_id=$(git hash-object --literally -t commit -w --stdin <"commit$offset")
    git show -s --format='%ad%n%cd' --date=raw "$commit_id" >"commit$offset.dates"
    cat >"tag$offset" <<EOF
object $commit_id
type commit
tag trailing$offset
tagger T <t@example.com> 1112591165 $offset 1700000000 -0500

Trailing tagger date
EOF
    tag_id=$(git hash-object --literally -t tag -w --stdin <"tag$offset")
    git update-ref "refs/tags/trailing$offset" "$tag_id"
    git for-each-ref --format='%(taggerdate:raw)' "refs/tags/trailing$offset" >"tag$offset.date"
done
