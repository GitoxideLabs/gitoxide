#!/usr/bin/env bash
set -eu -o pipefail

# These objects have valid graph edges but historical malformed identities.
# Fetching raw objects must not depend on interpreting author or tagger fields.
git init -q -b main source
blob_id=$(printf content | git -C source hash-object -w --stdin)
tree_id=$(printf '100644 blob %s\tfile\n' "$blob_id" | git -C source mktree)
commit_id=$(printf 'tree %s\nauthor legacy\ncommitter legacy\n\nold metadata\n' "$tree_id" |
    git -C source hash-object -w --literally -t commit --stdin)
tag_id=$(printf 'object %s\ntype commit\ntag legacy\ntagger legacy\n\nold tag metadata\n' "$commit_id" |
    git -C source hash-object -w --literally -t tag --stdin)
git -C source update-ref refs/heads/main "$commit_id"
git -C source update-ref refs/tags/legacy "$tag_id"
printf '%s\n' "$commit_id" >commit-id
printf '%s\n' "$tag_id" >tag-id
